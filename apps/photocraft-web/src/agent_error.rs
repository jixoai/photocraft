use serde::Serialize;
use serde_json::{Value, json};

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Execution {
    NotStarted,
    NotApplied,
    Unknown,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    code: &'static str,
    message: String,
    execution: Execution,
    retry_safe: bool,
    hint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected: Option<Box<Value>>,
}

impl Failure {
    pub fn rejected(code: &'static str, message: impl Into<String>, hint: impl Into<String>) -> Self {
        Self { code, message: message.into(), execution: Execution::NotStarted, retry_safe: true, hint: hint.into(), path: None, expected: None }
    }

    pub fn invalid(path: impl Into<String>, expected: Value, message: impl Into<String>) -> Self {
        let mut failure = Self::rejected("INVALID_ARGUMENTS", message, "Correct the indicated field using the tool inputSchema, then retry.");
        failure.path = Some(path.into());
        failure.expected = Some(Box::new(expected));
        failure
    }

    pub fn execution(message: impl Into<String>) -> Self {
        Self {
            code: "EXECUTION_FAILED",
            message: message.into(),
            execution: Execution::Unknown,
            retry_safe: false,
            hint: "Inspect the document, undo history and jobs before retrying; changes may already have applied.".into(),
            path: None,
            expected: None,
        }
    }

    pub fn interrupted(message: impl Into<String>, not_started: bool) -> Self {
        if not_started {
            Self::rejected("REQUEST_INTERRUPTED", message, "No changes applied. Inspect readiness before retrying.")
        } else {
            Self { code: "EXECUTION_UNCERTAIN", ..Self::execution(message) }
        }
    }

    pub fn discarded(message: impl Into<String>) -> Self {
        Self {
            execution: Execution::NotApplied,
            ..Self::rejected("REQUEST_INTERRUPTED", message, "The worker result will not be applied. Inspect readiness before retrying.")
        }
    }

    pub fn reply(&self) -> Value {
        json!({"ok":false,"error":self.message,"errorInfo":self})
    }
}

impl From<String> for Failure {
    fn from(message: String) -> Self {
        Self::execution(message)
    }
}

impl From<&str> for Failure {
    fn from(message: &str) -> Self {
        Self::execution(message)
    }
}

pub fn enrich(mut reply: Value) -> Value {
    if reply.get("ok") == Some(&Value::Bool(false)) && reply.get("errorInfo").is_none() {
        let message = reply.get("error").and_then(Value::as_str).unwrap_or("Editor operation failed");
        let failure = Failure::execution(message);
        reply["errorInfo"] = json!(failure);
    }
    reply
}

fn pointer(parent: &str, field: &str) -> String {
    format!("{parent}/{}", field.replace('~', "~0").replace('/', "~1"))
}

pub fn validate(tool: &str, args: &Value) -> Result<(), Failure> {
    static TOOLS: std::sync::OnceLock<Vec<Value>> = std::sync::OnceLock::new();
    let tools = TOOLS.get_or_init(crate::agent::tools);
    let definition = tools
        .iter()
        .find(|definition| definition["name"] == tool)
        .ok_or_else(|| Failure::rejected("UNKNOWN_TOOL", "Unknown browser tool", "Discover tools and use an exact registered tool name."))?;
    validate_node(args, &definition["inputSchema"], "")
}

fn validate_node(value: &Value, schema: &Value, path: &str) -> Result<(), Failure> {
    let valid_type = match schema.get("type").and_then(Value::as_str) {
        Some("object") => value.is_object(),
        Some("string") => value.is_string(),
        Some("integer") => value.is_u64() || value.is_i64(),
        Some("boolean") => value.is_boolean(),
        _ => true,
    };
    let invalid = |message: String| Failure::invalid(path, schema.clone(), message);
    if !valid_type {
        return Err(invalid(format!("{path}: expected {}", schema["type"])));
    }
    if let Some(choices) = schema.get("enum").and_then(Value::as_array)
        && !choices.contains(value)
    {
        return Err(invalid(format!("{path}: value is not an advertised choice")));
    }
    validate_number(value, schema, path)?;
    validate_text(value, schema, path)?;
    validate_object(value, schema, path)
}

fn validate_number(value: &Value, schema: &Value, path: &str) -> Result<(), Failure> {
    for (keyword, lower) in [("minimum", true), ("maximum", false)] {
        if let (Some(number), Some(bound)) = (value.as_f64(), schema.get(keyword).and_then(Value::as_f64))
            && if lower { number < bound } else { number > bound }
        {
            return Err(Failure::invalid(path, schema.clone(), format!("{path}: {keyword} is {bound}")));
        }
    }
    Ok(())
}

fn validate_text(value: &Value, schema: &Value, path: &str) -> Result<(), Failure> {
    if let Some(text) = value.as_str() {
        let length = text.chars().count() as u64;
        for (keyword, lower) in [("minLength", true), ("maxLength", false)] {
            if let Some(bound) = schema.get(keyword).and_then(Value::as_u64)
                && if lower { length < bound } else { length > bound }
            {
                return Err(Failure::invalid(path, schema.clone(), format!("{path}: {keyword} is {bound}")));
            }
        }
    }
    Ok(())
}

fn validate_object(value: &Value, schema: &Value, path: &str) -> Result<(), Failure> {
    if let Some(fields) = value.as_object() {
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for field in required.iter().filter_map(Value::as_str) {
                if !fields.contains_key(field) {
                    return Err(Failure::invalid(pointer(path, field), schema["properties"][field].clone(), format!("Missing required field: {field}")));
                }
            }
        }
        for (field, child) in fields {
            if let Some(child_schema) = schema.get("properties").and_then(|properties| properties.get(field)) {
                validate_node(child, child_schema, &pointer(path, field))?;
            } else if schema.get("additionalProperties") == Some(&Value::Bool(false)) {
                return Err(Failure::invalid(
                    pointer(path, field),
                    json!({"allowedProperties":schema["properties"].as_object().map(|fields| fields.keys().collect::<Vec<_>>())}),
                    "Unknown argument field",
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_identifies_original_acceptance_mistakes_without_dispatch() {
        for (tool, args, path) in [
            ("photocraft_commands", json!({"limit":300}), "/limit"),
            ("photocraft_control", json!({}), "/method"),
            ("photocraft_inspect", json!({"scope":"document"}), "/scope"),
            ("photocraft_export", json!({"format":"png","scale":1}), "/name"),
            ("photocraft_execute", json!({"command":"file.new","params":[]}), "/params"),
            ("photocraft_control", json!({"method":"app.quit"}), "/method"),
        ] {
            let failure = validate(tool, &args).unwrap_err().reply();
            assert_eq!(failure["ok"], false);
            assert!(failure["error"].is_string());
            assert_eq!(failure["errorInfo"]["code"], "INVALID_ARGUMENTS");
            assert_eq!(failure["errorInfo"]["path"], path);
            assert_eq!(failure["errorInfo"]["execution"], "not_started");
            assert_eq!(failure["errorInfo"]["retrySafe"], true);
        }
    }

    #[test]
    fn errors_preserve_constraints_and_do_not_echo_payloads() {
        let reply = validate("photocraft_commands", &json!({"limit":300})).unwrap_err().reply();
        assert_eq!(reply["errorInfo"]["expected"]["maximum"], 200);
        let reply = validate("photocraft_import", &json!({"name":"a.png","base64":123456789})).unwrap_err().reply();
        assert!(!reply.to_string().contains("123456789"));
        assert_eq!(pointer("", "a~/b"), "/a~0~1b");
    }

    #[test]
    fn execution_failures_never_promise_safe_replay() {
        let reply = enrich(json!({"ok":false,"error":"out of bounds","job":42}));
        assert_eq!(reply["error"], "out of bounds");
        assert_eq!(reply["job"], 42);
        assert_eq!(reply["errorInfo"]["execution"], "unknown");
        assert_eq!(reply["errorInfo"]["retrySafe"], false);
        assert_eq!(reply["errorInfo"]["code"], "EXECUTION_FAILED");
        assert_eq!(enrich(reply.clone()), reply);
        let success = json!({"ok":true,"result":{}});
        assert_eq!(enrich(success.clone()), success);
        assert_eq!(Failure::interrupted("cancelled", true).reply()["errorInfo"]["execution"], "not_started");
        assert_eq!(Failure::interrupted("cancelled", false).reply()["errorInfo"]["code"], "EXECUTION_UNCERTAIN");
        let discarded = Failure::discarded("cancelled").reply();
        assert_eq!(discarded["errorInfo"]["execution"], "not_applied");
        assert_eq!(discarded["errorInfo"]["retrySafe"], true);
    }
}
