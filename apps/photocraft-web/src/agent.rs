//! Browser agent operations. The transport uses the same command registry and control channel
//! as the desktop shell; file contents cross the browser boundary, never host filesystem paths.

use base64::Engine as _;
use photocraft_ui_egui::PhotocraftApp;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::agent_error::{self, Failure};

pub const MAX_FILE_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_JSON_BYTES: usize = 1024 * 1024;
pub const MAX_IMPORT_JSON_BYTES: usize = MAX_FILE_BYTES.div_ceil(3) * 4 + 4096;

/// Methods with useful browser semantics. Native window management, filesystem reads and
/// process termination are deliberately not advertised as browser capabilities.
pub const CONTROL_METHODS: &[&str] = &[
    "engine.commands",
    "engine.execute",
    "jobs.list",
    "jobs.cancel",
    "ui.inspect",
    "ui.set",
    "ui.menu.list",
    "ui.menu.invoke",
    "ui.context.choose",
    "ui.dialog.open",
    "ui.dialog.set",
    "ui.dialog.confirm",
    "ui.dialog.cancel",
    "ui.dialog.apply",
    "ui.pointer",
    "ui.click",
    "ui.move",
    "ui.key",
    "ui.type",
    "ui.screenshot",
    "app.save",
    "scope.status",
    "requests.status",
    "recovery.list",
    "recovery.checkpoint",
    "recovery.restore",
    "recovery.discard",
    "recovery.retry",
];

pub const TOOL_NAMES: &[&str] =
    &["photocraft_commands", "photocraft_execute", "photocraft_inspect", "photocraft_control", "photocraft_import", "photocraft_export"];

fn object(properties: Value, required: &[&str]) -> Value {
    json!({"type":"object", "properties":properties, "required":required, "additionalProperties":false})
}

fn control_schema() -> Value {
    let mut schema = object(json!({"method":{"type":"string","enum":CONTROL_METHODS},"params":{"type":"object"}}), &["method"]);
    let mut status = object(
        json!({
            "requestId":{"type":"string","minLength":1,"maxLength":128},
            "instanceId":{"type":"string","minLength":1,"maxLength":128}
        }),
        &[],
    );
    status["dependentRequired"] = json!({"requestId":["instanceId"]});
    schema["allOf"] = json!([{
        "if":{"properties":{"method":{"const":"requests.status"}},"required":["method"]},
        "then":{"properties":{"params":status}}
    }]);
    if let Some(conditions) = schema["allOf"].as_array_mut() {
        for method in ["scope.status", "recovery.list", "recovery.checkpoint", "recovery.restore", "recovery.discard", "recovery.retry"] {
            let keyed = matches!(method, "recovery.restore" | "recovery.discard");
            let mut fields = json!({});
            let mut required = Vec::new();
            if keyed {
                fields["key"] = json!({"type":"string","minLength":1,"maxLength":128,"pattern":"^[A-Za-z0-9_-]+$","description":"Exact snapshot key from recovery.list; restore creates a new dirty document and retains the original snapshot."});
                required.push("key");
            }
            if method == "recovery.discard" {
                fields["confirm"] = json!({"type":"boolean","enum":[true],"description":"Explicit permission to permanently delete this stored snapshot, not the open document."});
                required.push("confirm");
            }
            conditions.push(
                json!({"if":{"properties":{"method":{"const":method}},"required":["method"]},"then":{"properties":{"params":object(fields, &required)}}}),
            );
        }
    }
    schema
}

/// A small, stable tool surface exposes the entire evolving registry without registering
/// thousands of callbacks or sending thousands of command descriptions to every agent turn.
pub fn tools() -> Vec<Value> {
    let entries = [
        (
            TOOL_NAMES[0],
            "Discover FDE2E Photocraft commands",
            "Discover engine commands before editing. Each entry includes its exact id, semantic description, JSON Schema inputSchema, legacy params, journal flag and current enabled/disabledReason. Read inputSchema for parameter types, units, required fields, defaults, nested objects and examples; x-photocraft-coverage distinguishes complete authoring contracts from partial legacy-derived guidance. Page with nextOffset; search to avoid loading the full registry. Execute with photocraft_execute. Shell-only menus: photocraft_control ui.menu.list. Invalid arguments return errorInfo with path/expected. If the browser rejects before the callback with UnknownError, window.photocraft.validateTool(name, args) diagnoses envelope errors without editing.",
            object(
                json!({"search":{"type":"string","maxLength":256},"offset":{"type":"integer","minimum":0},"limit":{"type":"integer","minimum":1,"maximum":200,"default":50}}),
                &[],
            ),
            true,
        ),
        (
            TOOL_NAMES[1],
            "Execute FDE2E Photocraft command",
            "Run an engine or UI command by exact id in the active document. First call photocraft_commands and construct params from that command's inputSchema, not its JSON-like legacy params string. Check schema coverage and enabled/disabledReason; partial schemas are guidance, not complete validation. Changes share the user's undo history. Check ok, not Promise resolution. Failures include errorInfo.execution and retrySafe; unknown execution requires inspection before replay because work may already have applied. retrySafe is duplicate-edit safety, not an instruction to automatically retry unchanged arguments. Import with photocraft_import; export PNG/project copies with photocraft_export. Native-only commands return errors.",
            object(
                json!({"command":{"type":"string","minLength":1,"maxLength":256,"description":"Exact discovered command id, e.g. shape.create; not a menu label."},"params":{"type":"object","description":"Command-specific parameters matching the entry's inputSchema from photocraft_commands. Omit for an empty object. Coordinates/opacity units are command-specific; do not assume CSS pixels or percentages."},"requestId":{"type":"string","minLength":1,"maxLength":128,"description":"Optional unique step ID. Requires instanceId; returns an admission/status acknowledgement, not completion. Resubmitting identical arguments never executes this step twice within the same page instance."},"instanceId":{"type":"string","minLength":1,"maxLength":128,"description":"Required with requestId. Obtain from photocraft_control requests.status. A new page invalidates the old instance; reconcile before replaying."}}),
                &["command"],
            ),
            false,
        ),
        (
            TOOL_NAMES[2],
            "Inspect FDE2E Photocraft",
            "Inspect without editing. Omit scope (or full) for the backward-compatible complete UI snapshot. Prefer session for open-document metadata, document for active document ID/revision/dimensions/depth/dirty/history availability, layers for a paginated flat metadata tree (display-order depth-first, no paths or raster bounds), history for paginated undo labels, jobs for currently running jobs only. Only layers/history accept offset/limit (1-200, default 50). Scoped results bypass full UI inspection and raster-bound scans. Keep documentId/revision constant while paging; restart if they change. Use shape.info/type.info for one object's editable data, photocraft_export PNG for artwork previews and ui.screenshot for editor UI. Does not contain image pixels.",
            object(
                json!({"scope":{"type":"string","enum":["full","session","document","layers","history","jobs"],"default":"full"},"offset":{"type":"integer","minimum":0,"description":"Starting row for layers/history only; default 0."},"limit":{"type":"integer","minimum":1,"maximum":200,"default":50,"description":"Page size for layers/history only."}}),
                &[],
            ),
            true,
        ),
        (
            TOOL_NAMES[3],
            "Control FDE2E Photocraft interface",
            "Drive menus, dialogs, tools, pen strokes, keyboard, view state and screenshots through the same control protocol as the desktop. requests.status with empty params returns the page instanceId and tracking limits; params {instanceId,requestId} queries one admitted edit without waiting behind it. Tracked execute acknowledgements are not completion: require state=succeeded before the next dependent edit. Stop on failed/unknown. recovery.list returns browser-local checkpoints, pending/queued keys, restored document IDs and error. recovery.checkpoint queues the active document; recovery.restore {key} opens a dirty copy and retains the source; recovery.discard {key,confirm:true} permanently deletes that stored snapshot. Poll recovery.list after admissions, require no pending/queued key, no error and matching entry revision/restored ID; browser storage may be denied or evicted, so also export a project. recovery.retry clears the error for an explicit retry, not a reload. ui.screenshot returns PNG data without a filesystem path; app.save requires a download filename. Pointer coordinates are document pixels; click coordinates are CSS/egui points.",
            control_schema(),
            false,
        ),
        (
            TOOL_NAMES[4],
            "Import into FDE2E Photocraft",
            "Open a document, image or preset from base64 bytes (up to 64 MiB). Supports the browser app's formats including PSD, PSB, PNG, JPEG and .pcraft. Returns import warnings and document state. No network or host file access.",
            object(
                json!({"name":{"type":"string","minLength":1,"maxLength":255},"base64":{"type":"string","maxLength":MAX_FILE_BYTES.div_ceil(3)*4}}),
                &["name", "base64"],
            ),
            false,
        ),
        (
            TOOL_NAMES[5],
            "Export FDE2E Photocraft document",
            "Export a copy of the active document. The filename extension selects the format (.pcraft preserves the project). download=true triggers a browser download; otherwise returns base64 (up to 64 MiB). Warnings report format limitations. Exporting a copy does not mark unsaved work as saved; app.save downloads and marks a successful save.",
            object(
                json!({"name":{"type":"string","minLength":1,"maxLength":255},"quality":{"type":"integer","minimum":1,"maximum":100},"download":{"type":"boolean","default":false}}),
                &["name"],
            ),
            false,
        ),
    ];
    entries
        .into_iter()
        .map(|(name, title, description, mut schema, read_only)| {
            schema["properties"]["scopeId"] = json!({"type":"string","minLength":1,"maxLength":64,"pattern":"^[A-Za-z0-9_-]+$","description":"Stable task identity from photocraft_control scope.status. Required for non-discovery calls on explicitly scoped pages; a supplied mismatch always rejects before admission. Never inferred from the currently active browser tab."});
            if name == "photocraft_execute" {
                schema["dependentRequired"] = json!({"requestId":["instanceId"],"instanceId":["requestId"]});
                schema["properties"]["ifDocument"] = json!({"type":"object","properties":{"id":{"type":"integer","minimum":0},"revision":{"type":"integer","minimum":0}},"required":["id"],"additionalProperties":false,"description":"Optional active-document precondition, checked immediately before command dispatch. Use actual id/revision from document inspection. A mismatch applies no command; inspect and reconcile rather than retrying unchanged. Included in tracked request identity."});
            }
            let description = format!("{description} For independent tasks open a dedicated ?scope=TASK page, discover scope.status and pass scopeId on every non-discovery call. Scope isolates accidental operations and storage, not hostile same-origin code.");
            json!({
                "name":name,"title":title,"description":description,"inputSchema":schema,
                "annotations":{"readOnlyHint":read_only}
            })
        })
        .collect()
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Query {
    #[serde(default)]
    search: String,
    #[serde(default)]
    offset: usize,
    #[serde(default = "page_size")]
    limit: usize,
}

fn page_size() -> usize {
    50
}
fn empty_object() -> Value {
    json!({})
}

pub fn execution_params(command: String, params: Value, expected: Option<Value>) -> Value {
    let mut result = json!({"command":command,"params":params});
    if let Some(expected) = expected {
        result["ifDocument"] = expected;
    }
    result
}

pub fn tracked_fingerprint(command: &str, params: &Value, expected: &Option<Value>) -> Result<String, Failure> {
    crate::agent_requests::Ledger::fingerprint(command, &execution_params(command.into(), params.clone(), expected.clone()))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Execute {
    command: String,
    #[serde(default = "empty_object")]
    params: Value,
    #[serde(rename = "requestId")]
    request_id: Option<String>,
    #[serde(rename = "instanceId")]
    instance_id: Option<String>,
    #[serde(rename = "ifDocument")]
    expected: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RequestStatus {
    pub request_id: Option<String>,
    pub instance_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    method: String,
    #[serde(default = "empty_object")]
    params: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Import {
    name: String,
    base64: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Export {
    pub name: String,
    quality: Option<u8>,
    #[serde(default)]
    pub download: bool,
}

#[derive(Debug)]
pub enum Call {
    Commands(Query),
    Control { method: String, params: Value },
    Import { name: String, bytes: Vec<u8> },
    Export(Export),
    Tracked { id: String, instance: String, command: String, params: Value, expected: Option<Value> },
    RequestStatus(RequestStatus),
    Inspect(crate::agent_inspect::Query),
}

fn decode<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, Failure> {
    serde_json::from_value(value).map_err(|error| Failure::invalid("", json!({"type":"object"}), format!("Invalid arguments: {error}")))
}

pub fn filename(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 255 || name.starts_with('.') || name.contains(['/', '\\', ':']) || name.chars().any(char::is_control) {
        return Err("name must be a filename (1–255 bytes), without directories, a URL or control characters".into());
    }
    Ok(())
}

pub fn parse_scoped(scope: &crate::task_scope::Scope, tool: &str, args: Value) -> Result<Call, Failure> {
    agent_error::validate(tool, &args)?;
    scope.authorize(tool, &args)?;
    parse(tool, args)
}

pub fn parse(tool: &str, mut args: Value) -> Result<Call, Failure> {
    agent_error::validate(tool, &args)?;
    if let Some(fields) = args.as_object_mut() {
        fields.remove("scopeId");
    }
    match tool {
        "photocraft_commands" => {
            let q: Query = decode(args)?;
            if q.search.len() > 256 {
                return Err(Failure::invalid("/search", json!({"maxBytes":256}), "search must be at most 256 UTF-8 bytes"));
            }
            Ok(Call::Commands(q))
        }
        "photocraft_execute" => {
            let p: Execute = decode(args)?;
            if p.command.is_empty() || p.command.len() > 256 {
                return Err(Failure::invalid("/command", json!({"minBytes":1,"maxBytes":256}), "command must contain 1–256 UTF-8 bytes"));
            }
            if p.request_id.is_some() || p.instance_id.is_some() {
                let id = p
                    .request_id
                    .ok_or_else(|| Failure::invalid("/requestId", json!({"type":"string"}), "requestId and instanceId must be supplied together"))?;
                let instance = p
                    .instance_id
                    .ok_or_else(|| Failure::invalid("/instanceId", json!({"type":"string"}), "requestId and instanceId must be supplied together"))?;
                for (field, value) in [("requestId", &id), ("instanceId", &instance)] {
                    if value.len() > 128 {
                        return Err(Failure::invalid(format!("/{field}"), json!({"maxBytes":128}), "Request identity must be at most 128 UTF-8 bytes"));
                    }
                }
                return Ok(Call::Tracked { id, instance, command: p.command, params: p.params, expected: p.expected });
            }
            Ok(Call::Control { method: "engine.execute".into(), params: execution_params(p.command, p.params, p.expected) })
        }
        "photocraft_inspect" => {
            let query: crate::agent_inspect::Query = decode(args.clone())?;
            if !matches!(query.scope.as_str(), "layers" | "history") && (args.get("offset").is_some() || args.get("limit").is_some()) {
                return Err(Failure::invalid(
                    if args.get("offset").is_some() { "/offset" } else { "/limit" },
                    json!({"paginationScopes":["layers","history"]}),
                    "offset and limit are only supported for layers/history scopes",
                ));
            }
            if query.scope == "full" { Ok(Call::Control { method: "ui.inspect".into(), params: json!({}) }) } else { Ok(Call::Inspect(query)) }
        }
        "photocraft_control" => {
            let p: Control = decode(args)?;
            if matches!(p.method.as_str(), "recovery.restore" | "recovery.discard") && !p.params["key"].as_str().is_some_and(crate::recovery_bridge::valid_key)
            {
                return Err(Failure::invalid(
                    "/params/key",
                    json!({"pattern":"^[A-Za-z0-9_-]+$","maxLength":128}),
                    "Use an exact recovery key from recovery.list",
                ));
            }
            if p.method == "requests.status" {
                let status: RequestStatus = decode(p.params)?;
                if status.request_id.is_some() && status.instance_id.is_none() {
                    return Err(Failure::invalid("/params/instanceId", json!({"type":"string"}), "An individual request query requires its page instanceId"));
                }
                for (field, value) in [("requestId", &status.request_id), ("instanceId", &status.instance_id)] {
                    if value.as_ref().is_some_and(|value| value.is_empty() || value.len() > 128) {
                        return Err(Failure::invalid(
                            format!("/params/{field}"),
                            json!({"minBytes":1,"maxBytes":128}),
                            "Request identity must contain 1–128 UTF-8 bytes",
                        ));
                    }
                }
                return Ok(Call::RequestStatus(status));
            }
            if p.method == "ui.screenshot" && p.params.get("path").is_some_and(|v| !v.is_null()) {
                return Err(Failure::invalid("/params/path", json!({"type":"null"}), "Browser screenshots return PNG data; omit path"));
            }
            if p.method == "app.save" {
                let path = p
                    .params
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Failure::invalid("/params/path", json!({"type":"string"}), "app.save requires path: a download filename"))?;
                filename(path).map_err(|message| Failure::invalid("/params/path", json!({"format":"filename","maxBytes":255}), message))?;
            }
            Ok(Call::Control { method: p.method, params: p.params })
        }
        "photocraft_import" => {
            let p: Import = decode(args)?;
            filename(&p.name).map_err(|message| Failure::invalid("/name", json!({"format":"filename","maxBytes":255}), message))?;
            if p.base64.len() > MAX_FILE_BYTES.div_ceil(3) * 4 {
                return Err(Failure::invalid(
                    "/base64",
                    json!({"maxDecodedBytes":MAX_FILE_BYTES}),
                    "Import exceeds 64 MiB; use the browser file picker for larger files",
                ));
            }
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(p.base64)
                .map_err(|error| Failure::invalid("/base64", json!({"contentEncoding":"base64"}), format!("Invalid base64: {error}")))?;
            if bytes.is_empty() || bytes.len() > MAX_FILE_BYTES {
                return Err(Failure::invalid("/base64", json!({"minDecodedBytes":1,"maxDecodedBytes":MAX_FILE_BYTES}), "Import must contain 1 byte to 64 MiB"));
            }
            Ok(Call::Import { name: p.name, bytes })
        }
        "photocraft_export" => {
            let p: Export = decode(args)?;
            filename(&p.name).map_err(|message| Failure::invalid("/name", json!({"format":"filename","maxBytes":255}), message))?;
            Ok(Call::Export(p))
        }
        _ => Err(Failure::rejected("UNKNOWN_TOOL", "Unknown browser tool", "Discover tools and use an exact registered tool name.")),
    }
}

pub fn commands(app: &PhotocraftApp, q: &Query) -> Value {
    let mut total = 0usize;
    let mut entries = Vec::new();
    let needle = q.search.to_lowercase();
    for c in photocraft_engine::command_specs() {
        if !needle.is_empty()
            && ![Some(c.id), Some(c.label), photocraft_engine::commands::search_keywords(c.id)]
                .into_iter()
                .flatten()
                .any(|s| s.to_lowercase().contains(&needle))
        {
            continue;
        }
        if total >= q.offset && entries.len() < q.limit {
            entries.push(photocraft_engine::commands::list_entry(&app.session, c, (c.enabled)(&app.session).err()));
        }
        total = total.saturating_add(1);
    }
    let next = q.offset.saturating_add(entries.len());
    json!({"commands":entries,"total":total,"offset":q.offset,"nextOffset":(next < total).then_some(next)})
}

pub fn import(app: &mut PhotocraftApp, name: &str, bytes: &[u8]) -> Result<Value, String> {
    let warnings = app.open_automation_bytes(name, bytes)?;
    Ok(
        json!({"warnings":warnings,"session":photocraft_engine::inspect::session(&app.session),"document":app.session.active().map(photocraft_engine::inspect::document)}),
    )
}

pub fn export(app: &PhotocraftApp, p: &Export) -> Result<(Vec<u8>, Vec<String>), String> {
    let settings = export_settings(app, p)?;
    let state = app.session.active().ok_or("No active document")?;
    let export = app.services.export.as_ref().ok_or("No browser exporter configured")?;
    let (bytes, warnings) = export(&state.doc, &p.name, &settings)?;
    if !p.download && bytes.len() > MAX_FILE_BYTES {
        return Err("Export exceeds 64 MiB; use download: true".into());
    }
    Ok((bytes, warnings))
}

pub fn export_settings(app: &PhotocraftApp, p: &Export) -> Result<photocraft_ui_egui::ExportSettings, String> {
    let state = app.session.active().ok_or("No active document")?;
    let settings = photocraft_ui_egui::ExportSettings {
        jpeg_quality: p.quality,
        webp_quality: p.quality,
        webp_lossless: p.quality.is_none(),
        read_linked_files: false,
        pcraft_cache: Some(state.pcraft_cache.clone()),
        ..Default::default()
    };
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use photocraft_ui_egui::{
        Services,
        control::{ControlRequest, Outcome},
    };

    fn app() -> PhotocraftApp {
        PhotocraftApp::new(
            photocraft_engine::Session::new(),
            Services {
                import: Some(Box::new(|name, bytes, opts| {
                    photocraft_io::import_with_options(name, bytes, opts).map(|r| (r.document, r.warnings)).map_err(|e| e.to_string())
                })),
                export: Some(Box::new(|doc, name, opts| {
                    photocraft_io::export(doc, name, &opts.io_options()).map(|r| (r.bytes, r.warnings)).map_err(|e| e.to_string())
                })),
                ..Default::default()
            },
        )
    }

    #[test]
    fn scoped_routing_rejects_before_execution_or_tracking_admission() {
        let scope = crate::task_scope::Scope::new(Some("task-a".into())).unwrap();
        for (tool, args) in [
            ("photocraft_execute", json!({"command":"file.new","scopeId":"task-b","requestId":"step","instanceId":"page"})),
            ("photocraft_execute", json!({"command":"file.new"})),
            ("photocraft_control", json!({"method":"ui.key","params":{"key":"Delete"}})),
            ("photocraft_import", json!({"name":"image.png","base64":"YQ=="})),
            ("photocraft_export", json!({"name":"image.png"})),
            ("photocraft_inspect", json!({"scopeId":"task-b"})),
        ] {
            let reply = parse_scoped(&scope, tool, args).unwrap_err().reply();
            assert_eq!(reply["ok"], false);
            assert_eq!(reply["errorInfo"]["execution"], "not_started");
            assert_eq!(reply["errorInfo"]["retrySafe"], true);
            assert!(matches!(reply["errorInfo"]["code"].as_str(), Some("SCOPE_REQUIRED" | "SCOPE_MISMATCH")));
        }
        for (tool, args) in [
            ("photocraft_control", json!({"method":"scope.status"})),
            ("photocraft_inspect", json!({"scope":"document"})),
            ("photocraft_commands", json!({})),
            ("photocraft_execute", json!({"scopeId":"task-a","command":"file.new"})),
        ] {
            assert!(parse_scoped(&scope, tool, args).is_ok());
        }
        assert!(parse_scoped(&crate::task_scope::Scope::default(), "photocraft_execute", json!({"command":"file.new"})).is_ok());
        assert!(parse_scoped(&crate::task_scope::Scope::default(), "photocraft_execute", json!({"command":"file.new","scopeId":"task-a"})).is_err());
    }

    #[test]
    fn document_preconditions_survive_routing_and_tracked_identity() {
        let guard = json!({"id":7,"revision":3});
        let Call::Control { params, .. } = parse("photocraft_execute", json!({"command":"edit.undo","ifDocument":guard})).unwrap() else {
            panic!("expected control call")
        };
        assert_eq!(params["ifDocument"], guard);
        let Call::Tracked { expected, .. } =
            parse("photocraft_execute", json!({"command":"edit.undo","ifDocument":guard,"requestId":"step","instanceId":"page"})).unwrap()
        else {
            panic!("expected tracked call")
        };
        assert_eq!(expected, Some(guard.clone()));
        let fingerprint = tracked_fingerprint("edit.undo", &json!({}), &expected).unwrap();
        let mut ledger = crate::agent_requests::Ledger::new("page".into());
        ledger.reserve("page", "step", fingerprint.clone()).unwrap();
        assert!(ledger.existing("page", "step", &fingerprint).unwrap().is_some());
        for changed in [None, Some(json!({"id":8,"revision":3})), Some(json!({"id":7,"revision":4}))] {
            let changed = tracked_fingerprint("edit.undo", &json!({}), &changed).unwrap();
            assert_eq!(ledger.existing("page", "step", &changed).unwrap_err().reply()["errorInfo"]["code"], "REQUEST_ID_CONFLICT");
        }
        for invalid in [json!({}), json!({"id":-1}), json!({"id":1,"revision":null}), json!({"id":1,"other":true})] {
            assert!(parse("photocraft_execute", json!({"command":"edit.undo","ifDocument":invalid})).is_err());
        }
    }

    #[test]
    fn every_command_is_discoverable_once_and_bad_pages_are_bounded() {
        let app = app();
        let mut ids = std::collections::BTreeSet::new();
        let mut offset = 0;
        loop {
            let value = commands(&app, &Query { search: String::new(), offset, limit: 200 });
            for row in value["commands"].as_array().unwrap() {
                assert!(ids.insert(row["id"].as_str().unwrap().to_owned()));
                assert!(row["description"].is_string());
                assert!(row["journal"].is_boolean());
                assert_eq!(row["inputSchema"]["type"], "object");
                assert!(matches!(row["inputSchema"]["x-photocraft-coverage"].as_str(), Some("complete" | "partial")));
                assert_eq!(row["inputSchema"], *photocraft_engine::commands::input_schema(row["id"].as_str().unwrap()).unwrap());
            }
            let Some(next) = value["nextOffset"].as_u64() else { break };
            offset = next as usize;
        }
        assert_eq!(ids.len(), photocraft_engine::command_specs().len());
        assert!(commands(&app, &Query { search: String::new(), offset: usize::MAX, limit: 50 })["commands"].as_array().unwrap().is_empty());
        let results = commands(&app, &Query { search: "gaussian".into(), offset: 0, limit: 50 });
        assert!(results["total"].as_u64().unwrap() > 0);
    }

    #[test]
    fn discovery_preserves_nested_authoring_contracts_before_a_document_exists() {
        let app = app();
        let value = commands(&app, &Query { search: "shape.create".into(), offset: 0, limit: 1 });
        let entry = &value["commands"][0];
        assert_eq!(entry["id"], "shape.create");
        assert_eq!(entry["enabled"], false);
        assert_eq!(entry["inputSchema"]["x-photocraft-coverage"], "complete");
        assert_eq!(entry["inputSchema"]["properties"]["path"]["required"], json!(["subpaths"]));
        assert!(entry["inputSchema"]["examples"].is_array());
        assert!(entry["description"].as_str().unwrap().contains("Bézier"));
    }

    #[test]
    fn brush_discovery_keeps_local_definitions_and_patch_semantics() {
        let app = app();
        for (command, enabled) in [("tools.setBrush", true), ("paint.stroke", false)] {
            let value = commands(&app, &Query { search: command.into(), offset: 0, limit: 50 });
            let entry = value["commands"].as_array().unwrap().iter().find(|entry| entry["id"] == command).unwrap();
            let schema = &entry["inputSchema"];
            assert_eq!(entry["enabled"], enabled);
            assert_eq!(schema["x-photocraft-coverage"], "complete");
            assert_eq!(schema["properties"]["brush"]["$ref"], "#/$defs/BrushSettings");
            assert!(schema["$defs"]["BrushSettings"]["properties"]["shapeDynamics"]["properties"]["size"].get("$ref").is_some());
            assert!(schema["$defs"]["BrushTip"]["oneOf"].is_array());
            assert!(schema["$defs"]["BrushSettings"].get("x-photocraft-initial-default").is_some());
            assert!(schema["$defs"]["BrushSettings"]["properties"]["size"].get("default").is_none());
        }
    }

    #[test]
    fn tools_describe_valid_bounded_requests() {
        let definitions = tools();
        assert_eq!(definitions.len(), TOOL_NAMES.len());
        const { assert!(MAX_IMPORT_JSON_BYTES > MAX_FILE_BYTES) };
        const { assert!(MAX_JSON_BYTES < MAX_IMPORT_JSON_BYTES) };
        for (definition, name) in definitions.iter().zip(TOOL_NAMES) {
            assert_eq!(definition["name"], *name);
            assert_eq!(definition["inputSchema"]["type"], "object");
            assert_eq!(definition["inputSchema"]["additionalProperties"], false);
        }
        let Call::Commands(query) = parse("photocraft_commands", json!({})).unwrap() else { panic!("query") };
        assert_eq!(query.limit, 50);
        let Call::Export(export) = parse("photocraft_export", json!({"name":"图片.png","download":true})).unwrap() else { panic!("export") };
        assert!(export.download);
        assert!(parse("photocraft_control", json!({"method":"app.save","params":{"path":"图片.pcraft"}})).is_ok());
        assert!(parse("photocraft_control", json!({"method":"ui.screenshot"})).is_ok());
    }

    #[test]
    fn tool_arguments_reject_malformed_input_before_any_edit() {
        for (tool, args) in [
            ("photocraft_commands", json!({"limit":0})),
            ("photocraft_commands", json!({"limit":201})),
            ("photocraft_commands", json!({"offset":-1})),
            ("photocraft_execute", json!({"command":"file.new","params":[]})),
            ("photocraft_execute", json!({"command":""})),
            ("photocraft_execute", json!({"command":"file.new","typo":true})),
            ("photocraft_inspect", json!({"unused":1})),
            ("photocraft_control", json!({"method":"app.quit"})),
            ("photocraft_control", json!({"method":"ui.screenshot","params":{"path":"image.png"}})),
            ("photocraft_control", json!({"method":"app.save","params":{"path":"../document.psd"}})),
            ("photocraft_import", json!({"name":"a.png","base64":"!"})),
            ("photocraft_import", json!({"name":"a.png","base64":""})),
            ("photocraft_export", json!({"name":"https://host/file.png"})),
            ("photocraft_export", json!({"name":"a.png","quality":0})),
            ("photocraft_export", json!({"name":"a.png","quality":101})),
        ] {
            assert!(parse(tool, args.clone()).is_err(), "{tool}: {args}");
        }
        for tool in TOOL_NAMES {
            assert!(parse(tool, Value::Null).is_err());
        }
    }

    #[test]
    fn semantic_argument_errors_identify_the_field_before_dispatch() {
        for (tool, args, path) in [
            ("photocraft_import", json!({"name":"a.png","base64":"!"}), "/base64"),
            ("photocraft_export", json!({"name":"../a.png"}), "/name"),
            ("photocraft_control", json!({"method":"app.save","params":{}}), "/params/path"),
            ("photocraft_control", json!({"method":"ui.screenshot","params":{"path":"a.png"}}), "/params/path"),
        ] {
            let reply = parse(tool, args).unwrap_err().reply();
            assert_eq!(reply["errorInfo"]["path"], path);
            assert_eq!(reply["errorInfo"]["execution"], "not_started");
        }
    }

    #[test]
    fn tracked_requests_parse_without_changing_legacy_execution() {
        let Call::Tracked { id, instance, command, params, .. } = parse(
            "photocraft_execute",
            json!({
                "requestId":"plan:1","instanceId":"page-a","command":"layer.new.layer","params":{"name":"object"}
            }),
        )
        .unwrap() else {
            panic!("tracked")
        };
        assert_eq!((id.as_str(), instance.as_str(), command.as_str()), ("plan:1", "page-a", "layer.new.layer"));
        assert_eq!(params["name"], "object");
        let Call::RequestStatus(status) =
            parse("photocraft_control", json!({"method":"requests.status","params":{"requestId":"plan:1","instanceId":"page-a"}})).unwrap()
        else {
            panic!("status")
        };
        assert_eq!(status.request_id.as_deref(), Some("plan:1"));
        assert_eq!(status.instance_id.as_deref(), Some("page-a"));
        assert!(parse("photocraft_control", json!({"method":"requests.status"})).is_ok());
        for args in [
            json!({"command":"file.new","requestId":"plan:1"}),
            json!({"command":"file.new","instanceId":"page-a"}),
            json!({"command":"file.new","instanceId":"page-a","requestId":"中".repeat(128)}),
        ] {
            assert!(parse("photocraft_execute", args).is_err());
        }
        assert!(parse("photocraft_control", json!({"method":"requests.status","params":{"requestId":"plan:1"}})).is_err());
        assert!(parse("photocraft_control", json!({"method":"requests.status","params":{"unused":true}})).is_err());
        assert!(parse("photocraft_control", json!({"method":"requests.status","params":{"requestId":null}})).is_err());
    }

    #[test]
    fn interrupted_client_resumes_a_command_prefix_without_duplicate_layers() {
        let mut app = app();
        app.run("file.new", json!({"width":32,"height":24})).unwrap();
        let ctx = egui::Context::default();
        let mut ledger = crate::agent_requests::Ledger::new("page-a".into());
        let mut dispatched = 0usize;
        for prefix in [1, 2, 2] {
            for index in 0..prefix {
                let id = format!("plan:{index}");
                let params = json!({"name":format!("object-{index}")});
                let fingerprint = crate::agent_requests::Ledger::fingerprint("layer.new.layer", &params).unwrap();
                if ledger.existing("page-a", &id, &fingerprint).unwrap().is_some() {
                    continue;
                }
                ledger.reserve("page-a", &id, fingerprint).unwrap();
                ledger.started(&id);
                let (request, _) = ControlRequest::new("engine.execute", json!({"command":"layer.new.layer","params":params}));
                let Outcome::Done(reply) = photocraft_ui_egui::control::handle(&mut app, &ctx, &request) else { panic!("synchronous") };
                ledger.finish(&id, crate::agent_error::enrich(reply));
                dispatched += 1;
            }
        }
        assert_eq!(dispatched, 2);
        let doc = &app.session.active().unwrap().doc;
        for index in 0..2 {
            assert_eq!(doc.layers.iter().filter(|layer| layer.name == format!("object-{index}")).count(), 1);
            assert_eq!(ledger.status(Some("page-a"), Some(&format!("plan:{index}"))).unwrap()["state"], "succeeded");
        }
    }

    #[test]
    fn agent_commands_share_editor_history_and_project_roundtrips() {
        let mut app = app();
        let ctx = egui::Context::default();
        let Call::Control { method, params } = parse("photocraft_execute", json!({"command":"file.new","params":{"width":32,"height":24,"depth":16}})).unwrap()
        else {
            panic!("control")
        };
        let (req, _) = ControlRequest::new(method, params);
        let Outcome::Done(result) = photocraft_ui_egui::control::handle(&mut app, &ctx, &req) else { panic!("synchronous") };
        assert_eq!(result["ok"], true, "{result}");
        app.run("layer.new.layer", json!({"name":"Agent layer"})).unwrap();
        assert!(app.session.active().unwrap().history.can_undo());
        let options: Export = decode(json!({"name":"roundtrip.pcraft"})).unwrap();
        let (bytes, _) = export(&app, &options).unwrap();
        let Call::Import { name, bytes } =
            parse("photocraft_import", json!({"name":options.name,"base64":base64::engine::general_purpose::STANDARD.encode(bytes)})).unwrap()
        else {
            panic!("import")
        };
        let result = import(&mut app, &name, &bytes).unwrap();
        assert_eq!(result["document"]["width"], 32);
        assert_eq!(result["document"]["depth"], 16);
        assert_eq!(app.session.documents().len(), 2);
        assert_eq!(app.session.active().unwrap().doc.layers.len(), 2);
    }
}
