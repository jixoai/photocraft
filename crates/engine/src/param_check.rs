//! Reject params a command does not document (#4336).
//!
//! Commands read the keys they know from a JSON object and used to ignore the rest, so a
//! misspelled key (`{"ids":[4]}` for `{"layer":4}`) ran the command on its defaults and reported
//! success. The dispatcher checks top-level keys against complete, closed input schemas for
//! every transport (CLI, MCP, control channel, UI, actions). Partial schemas remain advisory.
//!
//! Legacy prose-based checking remains opt-in through [`STRICT`] because many `params` docs
//! are incomplete. A strict doc must name every accepted key as `"key":...` and contain no
//! top-level `…`; `strict_commands_have_closed_docs` checks it is parseable.

use serde_json::Value;

use crate::commands::CommandSpec;
use crate::{EngineError, Result};

/// Keys any command accepts: the history-coalescing key every command honours (see
/// `Session::execute`) and the Channels-panel `"target"` that pixel commands follow.
const UNIVERSAL: &[&str] = &["coalesce", "target"];

/// The commands whose unknown params are rejected: their `params` doc is complete.
const STRICT: &[&str] = &["layer.hideLayers", "layer.showLayers"];

pub(crate) fn unexpected_parameter<'a>(spec: &CommandSpec, params: &'a Value) -> Option<(&'a str, Vec<String>)> {
    let fields = params.as_object()?;
    let schema = crate::commands::input_schema(spec.id)?;
    if schema.get("x-photocraft-coverage")?.as_str()? != "complete" || schema.get("additionalProperties")? != &Value::Bool(false) {
        return None;
    }
    let properties = schema.get("properties")?.as_object()?;
    let field = fields.keys().find(|field| !properties.contains_key(*field))?;
    Some((field.as_str(), properties.keys().cloned().collect()))
}

/// The keys a `params` doc names, or `None` when the doc is open-ended.
fn documented_keys(doc: &str) -> Option<Vec<&str>> {
    let bytes = doc.as_bytes();
    let (mut depth, mut in_str, mut start) = (0usize, false, 0usize);
    let mut quoted: Vec<(usize, usize)> = Vec::new();
    for (i, &b) in bytes.iter().enumerate() {
        if in_str {
            if b == b'"' {
                in_str = false;
                quoted.push((start, i));
            }
            continue;
        }
        match b {
            b'"' => {
                in_str = true;
                start = i + 1;
            }
            b'{' | b'[' => depth += 1,
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
        // `…` / `...` straight inside the top-level braces: the list is not complete.
        if depth == 1
            && (doc.get(i..).is_some_and(|r| r.starts_with('…') || r.starts_with("...")))
            && doc.get(..i).is_some_and(|l| l.trim_start().starts_with('{'))
        {
            return None;
        }
    }
    let mut keys = Vec::new();
    for (n, &(a, b)) in quoted.iter().enumerate() {
        let Some(word) = doc.get(a..b) else { continue };
        if word.is_empty() || !word.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') {
            continue;
        }
        // A key is followed by `:`, or by `,"other"` pairs that end in one (`"all","pixels":bool`).
        let mut end = b + 1;
        let mut k = n;
        loop {
            let rest = doc.get(end..).unwrap_or("");
            if rest.starts_with(':') {
                keys.push(word);
                break;
            }
            let Some(after) = rest.strip_prefix(',') else { break };
            let Some(&(na, nb)) = quoted.get(k + 1) else { break };
            if na != end + 2 || after.as_bytes().first() != Some(&b'"') {
                break;
            }
            end = nb + 1;
            k += 1;
        }
    }
    Some(keys)
}

/// Reject unknown top-level keys in a complete contract or an opted-in legacy params doc.
pub(crate) fn check(spec: &CommandSpec, params: &Value) -> Result<()> {
    if !STRICT.contains(&spec.id) {
        if let Some((field, allowed)) = unexpected_parameter(spec, params) {
            return Err(EngineError::BadParams { cmd: spec.id.to_string(), msg: format!("unknown parameter `{field}` (expected: {})", allowed.join(", ")) });
        }
        return Ok(());
    }
    let Some(map) = params.as_object() else { return Ok(()) };
    let Some(keys) = documented_keys(spec.params) else { return Ok(()) };
    for key in map.keys() {
        if UNIVERSAL.contains(&key.as_str()) || keys.contains(&key.as_str()) {
            continue;
        }
        let expected = if keys.is_empty() { "no parameters".to_string() } else { format!("expected: {}", keys.join(", ")) };
        return Err(EngineError::BadParams { cmd: spec.id.to_string(), msg: format!("unknown parameter `{key}` ({expected})") });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::Session;

    #[test]
    fn complete_contracts_reject_unknown_keys_before_any_target_is_used() {
        let mut session = Session::new();
        session.execute("file.new", json!({"width":24,"height":16,"profile":"none"})).unwrap();
        let subject = session.execute("shape.create", json!({"rect":[1,1,8,8],"name":"Subject"})).unwrap()["layer"].clone();
        let neighbor = session.execute("shape.create", json!({"rect":[12,2,8,8],"name":"Neighbor"})).unwrap()["layer"].clone();
        let before = session.active().unwrap().doc.clone();
        let history = session.active().unwrap().history.past_len();
        let revision = session.active().unwrap().revision;
        let journal = session.journal.clone();
        for (command, params) in [
            ("shape.info", json!({"id":subject})),
            ("shape.edit", json!({"id":subject,"fill":"#ff0000"})),
            ("shape.edit", json!({"layer":subject,"mvoe":[2,3]})),
            ("shape.create", json!({"rect":[1,1,4,4],"flll":"#ff0000"})),
            ("layer.delete", json!({"id":subject})),
            ("layer.delete", json!({"layers":[subject]})),
            ("document.inspect", json!({"id":1})),
        ] {
            assert!(session.precheck(command, &params).is_err(), "{command}: {params}");
            assert!(session.execute(command, params.clone()).is_err(), "{command}: {params}");
            assert!(session.start(command, params).is_err(), "{command}");
            assert_eq!(session.active().unwrap().doc, before, "{command}");
            assert_eq!(session.active().unwrap().history.past_len(), history);
            assert_eq!(session.active().unwrap().revision, revision);
            assert_eq!(session.journal, journal);
            assert_eq!(session.active().unwrap().active_layer.map(|layer| layer.0), neighbor.as_u64());
        }
        assert_eq!(session.execute("shape.info", json!({})).unwrap()["layer"], neighbor);
        session.execute("shape.edit", json!({"fill":"#336699","coalesce":"paint"})).unwrap();
        assert_eq!(session.execute("shape.info", json!({"layer":subject})).unwrap()["name"], "Subject");
        session.execute("layer.delete", json!({"layer":subject})).unwrap();
        assert_eq!(session.execute("shape.info", json!({})).unwrap()["layer"], neighbor);
    }

    #[test]
    fn partial_contracts_keep_legacy_parameter_validation() {
        let spec = crate::command_specs()
            .iter()
            .find(|spec| {
                crate::commands::input_schema(spec.id).is_some_and(|schema| schema["x-photocraft-coverage"] == "partial") && !STRICT.contains(&spec.id)
            })
            .unwrap();
        assert!(crate::commands::unexpected_parameter(spec.id, &json!({"legacyExtension":true})).is_none());
        assert!(check(spec, &json!({"legacyExtension":true})).is_ok());
    }

    #[test]
    fn documented_keys_reads_keys_and_stops_at_open_docs() {
        assert_eq!(documented_keys(r##"{"layer":id?} (shows them instead)"##), Some(vec!["layer"]));
        assert_eq!(documented_keys(r##"{"locks":{"all","pixels":bool}?,"name":str?}"##), Some(vec!["locks", "all", "pixels", "name"]));
        assert_eq!(documented_keys("{}"), Some(vec![]));
        assert_eq!(documented_keys(r##"{"layer":id?, …params of that kind}"##), None);
    }

    #[test]
    fn strict_commands_have_closed_docs() {
        for id in STRICT {
            let spec = crate::commands::find(id).expect("strict command exists");
            assert!(documented_keys(spec.params).is_some_and(|k| !k.is_empty()), "{id}: params doc must list its keys");
        }
    }

    #[test]
    fn unknown_param_is_an_error_naming_the_key_and_the_accepted_ones() {
        let mut s = Session::new();
        s.execute("file.new", json!({"width": 8, "height": 8})).unwrap();
        let a = s.execute("layer.new.layer", json!({})).unwrap()["layer"].as_u64().unwrap();
        let visible = |s: &Session| s.active().unwrap().doc.layer(photocraft_doc::LayerId(a)).unwrap().visible;
        for cmd in ["layer.hideLayers", "layer.showLayers"] {
            let err = s.execute(cmd, json!({"ids": [a]})).unwrap_err().to_string();
            assert!(err.contains("unknown parameter `ids`") && err.contains("expected: layer"), "{err}");
            assert!(visible(&s), "{cmd}: nothing ran");
        }
        s.execute("layer.hideLayers", json!({"layer": a, "coalesce": "k"})).unwrap();
        assert!(!visible(&s));
    }
}
