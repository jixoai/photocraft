use serde_json::{Value, json};

use super::{described, object, properties};

const MAX_DEPTH: usize = 16;

pub(super) fn schema(documentation: &str) -> Value {
    let text = documentation.trim();
    let mut result = if text.starts_with('{') { object_schema(text, 0) } else { object(json!({}), &[]) };
    result["additionalProperties"] = json!(true);
    result
}

fn segments(text: &str, separator: char) -> Vec<&str> {
    let mut output = Vec::new();
    let mut nesting = Vec::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut start = 0usize;
    for (offset, character) in text.char_indices() {
        if quoted {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
            continue;
        }
        match character {
            '"' => quoted = true,
            '{' | '[' | '(' => nesting.push(character),
            '}' | ']' | ')' => {
                nesting.pop();
            }
            other if other == separator && nesting.is_empty() => {
                if let Some(part) = text.get(start..offset) {
                    output.push(part.trim());
                }
                start = offset.saturating_add(character.len_utf8());
            }
            _ => {}
        }
    }
    if let Some(part) = text.get(start..) {
        output.push(part.trim());
    }
    output
}

fn enclosed(text: &str, open: char, close: char) -> Option<&str> {
    if !text.starts_with(open) {
        return None;
    }
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    for (offset, character) in text.char_indices() {
        if quoted {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
            continue;
        }
        if character == '"' {
            quoted = true;
        } else if character == open {
            depth = depth.saturating_add(1);
        } else if character == close {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return text.get(open.len_utf8()..offset);
            }
        }
    }
    None
}

fn key(text: &str) -> Option<(&str, &str)> {
    let rest = text.trim().strip_prefix('"')?;
    let end = rest.find('"')?;
    let name = rest.get(..end)?;
    if name.is_empty() || !name.chars().all(|character| character.is_ascii_alphanumeric() || character == '_') {
        return None;
    }
    Some((name, rest.get(end.saturating_add(1)..)?.trim()))
}

fn object_schema(text: &str, depth: usize) -> Value {
    let Some(body) = enclosed(text, '{', '}') else {
        let mut result = object(json!({}), &[]);
        result["additionalProperties"] = json!(true);
        return result;
    };
    let mut fields = Vec::new();
    let mut pending = Vec::new();
    for part in segments(body, ',') {
        let Some((name, tail)) = key(part) else { continue };
        pending.push(name);
        let Some(notation) = tail.strip_prefix(':') else { continue };
        for field in pending.drain(..) {
            fields.push((field.to_owned(), described(value_schema(notation, depth.saturating_add(1)), &format!("{field}: {notation}"))));
        }
    }
    let mut result = object(properties(fields), &[]);
    result["additionalProperties"] = json!(true);
    result
}

fn value_schema(notation: &str, depth: usize) -> Value {
    if depth > MAX_DEPTH {
        return json!({});
    }
    let without_default = split_default(notation).map_or(notation, |(value, _)| value);
    let alternatives = segments(without_default, '|');
    let value = without_default.trim().trim_end_matches('?').trim();
    let mut result = if alternatives.len() > 1 {
        let variants: Vec<_> = alternatives.into_iter().map(|part| value_schema(part, depth.saturating_add(1))).collect();
        json!({"anyOf":variants})
    } else if value.starts_with('{') {
        object_schema(value, depth)
    } else if value.starts_with('[') {
        array_schema(value, depth)
    } else if let Some(text) = quoted(value) {
        string_schema(text)
    } else {
        primitive_schema(value)
    };
    if let Some(default) = literal_default(notation)
        && let Some(fields) = result.as_object_mut()
    {
        fields.insert("default".into(), default);
    }
    result
}

fn quoted(text: &str) -> Option<&str> {
    let rest = text.strip_prefix('"')?;
    let end = rest.find('"')?;
    rest.get(..end)
}

fn string_schema(text: &str) -> Value {
    if text.contains('…') || text.contains("...") || text.starts_with('#') || text.contains(' ') {
        return json!({"type":"string"});
    }
    let values: Vec<_> = text.split('|').collect();
    if values.len() > 1 { json!({"type":"string","enum":values}) } else { json!({"type":"string","const":text}) }
}

fn primitive_schema(text: &str) -> Value {
    let token = text.split_whitespace().next().unwrap_or("").trim_end_matches('?');
    if let Some((lower, upper)) = token.split_once("..") {
        let minimum = lower.parse::<f64>().ok().filter(|number| number.is_finite());
        let maximum = upper.trim_start_matches('=').parse::<f64>().ok().filter(|number| number.is_finite());
        if let (Some(minimum), Some(maximum)) = (minimum, maximum)
            && minimum <= maximum
        {
            return json!({"type":"number","minimum":minimum,"maximum":maximum});
        }
    }
    if let Ok(value) = serde_json::from_str::<Value>(token)
        && matches!(value, Value::Bool(_) | Value::Number(_) | Value::Null)
    {
        return json!({"const":value});
    }
    match token {
        "bool" | "boolean" => json!({"type":"boolean"}),
        "str" | "string" | "text" | "name" | "path" | "folder" | "file" | "base64" | "hex" => json!({"type":"string"}),
        "u32" => json!({"type":"integer","minimum":0,"maximum":u32::MAX}),
        "u64" | "uint" | "index" | "i" | "n" | "char" => json!({"type":"integer","minimum":0}),
        "i32" => json!({"type":"integer","minimum":i32::MIN,"maximum":i32::MAX}),
        "id" | "layerId" => json!({"type":"integer","minimum":0}),
        "px" | "pt" | "deg" | "%" | "number" | "float" | "f32" | "f64" | "x" | "y" | "x0" | "y0" | "x1" | "y1" | "dx" | "dy" | "w" | "h" | "r" | "g" | "b"
        | "a" | "c" | "d" | "e" | "f" | "t" => json!({"type":"number"}),
        _ => json!({}),
    }
}

fn array_schema(text: &str, depth: usize) -> Value {
    let Some(body) = enclosed(text, '[', ']') else { return json!({"type":"array"}) };
    let parts = segments(body, ',');
    if parts.iter().any(|part| matches!(*part, "…" | "...")) {
        let item = parts.first().filter(|part| !matches!(**part, "…" | "...")).map(|part| value_schema(part, depth.saturating_add(1))).unwrap_or(json!({}));
        return json!({"type":"array","items":item});
    }
    if parts.len() == 1 {
        return json!({"type":"array","items":value_schema(body, depth.saturating_add(1))});
    }
    let items: Vec<_> = parts.iter().map(|part| value_schema(part, depth.saturating_add(1))).collect();
    let required = parts.iter().take_while(|part| !part.contains('?')).count();
    json!({"type":"array","prefixItems":items,"minItems":required,"maxItems":parts.len()})
}

fn literal_default(text: &str) -> Option<Value> {
    let (_, tail) = split_default(text)?;
    let tail = tail.trim();
    let mut stream = serde_json::Deserializer::from_str(tail).into_iter::<Value>();
    stream.next()?.ok().filter(|value| matches!(value, Value::String(_) | Value::Number(_) | Value::Bool(_) | Value::Null))
}

fn split_default(text: &str) -> Option<(&str, &str)> {
    let mut nesting = Vec::new();
    let mut quoted = false;
    let mut escaped = false;
    for (offset, character) in text.char_indices() {
        if quoted {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
            continue;
        }
        match character {
            '"' => quoted = true,
            '{' | '[' | '(' => nesting.push(character),
            '}' | ']' | ')' => {
                nesting.pop();
            }
            '=' if nesting.is_empty() && !text.get(..offset).is_some_and(|prefix| prefix.ends_with('.')) => {
                return Some((text.get(..offset)?.trim(), text.get(offset.saturating_add(character.len_utf8())..)?.trim()));
            }
            _ => {}
        }
    }
    None
}
