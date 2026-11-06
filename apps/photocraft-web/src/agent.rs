//! Browser agent operations. The transport uses the same command registry and control channel
//! as the desktop shell; file contents cross the browser boundary, never host filesystem paths.

use base64::Engine as _;
use photocraft_ui_egui::PhotocraftApp;
use serde::Deserialize;
use serde_json::{Value, json};

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
];

pub const TOOL_NAMES: &[&str] =
    &["photocraft_commands", "photocraft_execute", "photocraft_inspect", "photocraft_control", "photocraft_import", "photocraft_export"];

fn object(properties: Value, required: &[&str]) -> Value {
    json!({"type":"object", "properties":properties, "required":required, "additionalProperties":false})
}

/// A small, stable tool surface exposes the entire evolving registry without registering
/// thousands of callbacks or sending thousands of command descriptions to every agent turn.
pub fn tools() -> Vec<Value> {
    let any_object = json!({"type":"object"});
    let entries = [
        (
            TOOL_NAMES[0],
            "Discover FDE2E Photocraft commands",
            "Discover engine commands before editing. Each entry includes its exact id, semantic description, JSON Schema inputSchema, legacy params, journal flag and current enabled/disabledReason. Read inputSchema for parameter types, units, required fields, defaults, nested objects and examples; x-photocraft-coverage distinguishes complete authoring contracts from partial legacy-derived guidance. Page with nextOffset; search to avoid loading the full registry. Execute with photocraft_execute. Shell-only menus: photocraft_control ui.menu.list.",
            object(
                json!({"search":{"type":"string","maxLength":256},"offset":{"type":"integer","minimum":0},"limit":{"type":"integer","minimum":1,"maximum":200,"default":50}}),
                &[],
            ),
            true,
        ),
        (
            TOOL_NAMES[1],
            "Execute FDE2E Photocraft command",
            "Run an engine or UI command by exact id in the active document. First call photocraft_commands and construct params from that command's inputSchema, not its JSON-like legacy params string. Check schema coverage and enabled/disabledReason; partial schemas are guidance, not complete validation. Changes share the user's undo history. After an error/timeout inspect before retrying, since work may already have applied. Import with photocraft_import; export PNG/project copies with photocraft_export. Native-only commands return errors.",
            object(
                json!({"command":{"type":"string","minLength":1,"maxLength":256,"description":"Exact discovered command id, e.g. shape.create; not a menu label."},"params":{"type":"object","description":"Command-specific parameters matching the entry's inputSchema from photocraft_commands. Omit for an empty object. Coordinates/opacity units are command-specific; do not assume CSS pixels or percentages."}}),
                &["command"],
            ),
            false,
        ),
        (
            TOOL_NAMES[2],
            "Inspect FDE2E Photocraft",
            "Read open documents, layers, selection, undo history, tools, dialogs, GPU renderer, memory and frame timings. This does not change the document.",
            object(json!({}), &[]),
            true,
        ),
        (
            TOOL_NAMES[3],
            "Control FDE2E Photocraft interface",
            "Drive menus, dialogs, tools, pen strokes, keyboard, view state and screenshots through the same control protocol as the desktop. ui.screenshot returns PNG data without a filesystem path; app.save requires a download filename. Pointer coordinates are document pixels; click coordinates are CSS/egui points.",
            object(json!({"method":{"type":"string","enum":CONTROL_METHODS},"params":any_object}), &["method"]),
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
        .map(|(name, title, description, schema, read_only)| {
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Execute {
    command: String,
    #[serde(default = "empty_object")]
    params: Value,
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
}

fn decode<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, String> {
    serde_json::from_value(value).map_err(|e| format!("Invalid arguments: {e}"))
}

pub fn filename(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 255 || name.starts_with('.') || name.contains(['/', '\\', ':']) || name.chars().any(char::is_control) {
        return Err("name must be a filename (1–255 bytes), without directories, a URL or control characters".into());
    }
    Ok(())
}

fn params_object(params: &Value) -> Result<(), String> {
    if params.is_object() { Ok(()) } else { Err("params must be an object".into()) }
}

pub fn parse(tool: &str, args: Value) -> Result<Call, String> {
    params_object(&args)?;
    match tool {
        "photocraft_commands" => {
            let q: Query = decode(args)?;
            if q.search.len() > 256 || !(1..=200).contains(&q.limit) {
                return Err("search must be at most 256 bytes and limit must be 1–200".into());
            }
            Ok(Call::Commands(q))
        }
        "photocraft_execute" => {
            let p: Execute = decode(args)?;
            params_object(&p.params)?;
            if p.command.is_empty() || p.command.len() > 256 {
                return Err("command must contain 1–256 bytes".into());
            }
            Ok(Call::Control { method: "engine.execute".into(), params: json!({"command":p.command,"params":p.params}) })
        }
        "photocraft_inspect" => {
            if args.as_object().is_none_or(|o| !o.is_empty()) {
                return Err("inspect takes no arguments".into());
            }
            Ok(Call::Control { method: "ui.inspect".into(), params: args })
        }
        "photocraft_control" => {
            let p: Control = decode(args)?;
            params_object(&p.params)?;
            if !CONTROL_METHODS.contains(&p.method.as_str()) {
                return Err(format!("Method {} is not available in the browser", p.method));
            }
            if p.method == "ui.screenshot" && p.params.get("path").is_some_and(|v| !v.is_null()) {
                return Err("Browser screenshots return PNG data; omit path".into());
            }
            if p.method == "app.save" {
                filename(p.params.get("path").and_then(Value::as_str).ok_or("app.save requires path: a download filename")?)?;
            }
            Ok(Call::Control { method: p.method, params: p.params })
        }
        "photocraft_import" => {
            let p: Import = decode(args)?;
            filename(&p.name)?;
            if p.base64.len() > MAX_FILE_BYTES.div_ceil(3) * 4 {
                return Err("Import exceeds 64 MiB; use the browser file picker for larger files".into());
            }
            let bytes = base64::engine::general_purpose::STANDARD.decode(p.base64).map_err(|e| format!("Invalid base64: {e}"))?;
            if bytes.is_empty() || bytes.len() > MAX_FILE_BYTES {
                return Err("Import must contain 1 byte to 64 MiB".into());
            }
            Ok(Call::Import { name: p.name, bytes })
        }
        "photocraft_export" => {
            let p: Export = decode(args)?;
            filename(&p.name)?;
            if p.quality.is_some_and(|q| !(1..=100).contains(&q)) {
                return Err("quality must be 1–100".into());
            }
            Ok(Call::Export(p))
        }
        _ => Err(format!("Unknown FDE2E Photocraft tool: {tool}")),
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
