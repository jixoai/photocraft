use serde_json::{Map, Value, json};

use super::CommandSpec;

mod brush;
mod core;
mod filters;
mod image;
mod layers;
mod notation;
mod paint;
mod text;
mod transform;
mod types;
mod vector;

const DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";

pub(super) fn command_description(spec: &CommandSpec) -> String {
    let purpose = core::description(spec.id)
        .or_else(|| vector::description(spec.id))
        .or_else(|| layers::description(spec.id))
        .or_else(|| paint::description(spec.id))
        .or_else(|| brush::description(spec.id))
        .or_else(|| text::description(spec.id))
        .or_else(|| transform::description(spec.id))
        .or_else(|| image::description(spec.id))
        .map(str::to_owned)
        .or_else(|| filters::description(spec.id))
        .unwrap_or_else(|| {
            let location = if spec.menu.is_empty() { String::new() } else { format!(" Menu: {}.", spec.menu.join(" > ")) };
            format!("{} ({}).{}", spec.label.trim_end_matches('…'), spec.id, location)
        });
    format!(
        "{purpose} {} Check enabled and disabledReason before running; inspect the result before retrying an edit.",
        if spec.journal { "This command is recorded in the session journal." } else { "This command is not recorded in the session journal." }
    )
}

pub(super) fn input_schema(spec: &CommandSpec) -> Value {
    let explicit = core::schema(spec.id)
        .or_else(|| vector::schema(spec.id))
        .or_else(|| layers::schema(spec.id))
        .or_else(|| paint::schema(spec.id))
        .or_else(|| brush::schema(spec.id))
        .or_else(|| text::schema(spec.id))
        .or_else(|| transform::schema(spec.id))
        .or_else(|| image::schema(spec.id))
        .or_else(|| filters::schema(spec.id, spec.params));
    let complete = explicit.is_some();
    let mut schema = explicit.unwrap_or_else(|| notation::schema(spec.params));
    if let Some(root) = schema.as_object_mut() {
        root.insert("$schema".into(), json!(DIALECT));
        root.insert("title".into(), json!(spec.label));
        root.insert("description".into(), json!(command_description(spec)));
        root.insert("x-photocraft-coverage".into(), json!(if complete { "complete" } else { "partial" }));
        if !complete {
            root.insert("$comment".into(), json!("Partial authoring contract, conservatively derived from legacy documentation or a partly verified override. Some required fields, nested objects and accepted variants may be undocumented; use params for the original notation. The command performs its own validation; this schema is not used to reject legacy calls."));
        }
        if spec.id != "file.new"
            && let Some(properties) = root.get_mut("properties").and_then(Value::as_object_mut)
        {
            properties.entry("coalesce").or_insert_with(|| {
                json!({
                    "type":"string",
                    "description":"Optional history grouping key. Consecutive edits with the same key share one history step; omit for independent undo steps."
                })
            });
            properties.entry("target").or_insert_with(types::target);
        }
    }
    schema
}

pub(super) fn object(properties: Value, required: &[&str]) -> Value {
    json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
}

fn described(mut schema: Value, description: &str) -> Value {
    if let Some(fields) = schema.as_object_mut() {
        fields.insert("description".into(), json!(description));
    }
    schema
}

fn properties(values: impl IntoIterator<Item = (String, Value)>) -> Value {
    Value::Object(values.into_iter().collect::<Map<_, _>>())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod workflow_tests;
