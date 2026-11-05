use serde_json::{Value, json};

use super::{described, object, types};

pub(super) fn description(id: &str) -> Option<&'static str> {
    Some(match id {
        "layer.new.layer" => {
            "Create and activate an empty raster layer above the active layer, or inside an expanded active group. Supports clipping, blend mode, colour label and neutral fill in one undo step. opacity is a 0–1 fraction. Returns a stable layer ID; this does not paint the requested artwork."
        }
        "layer.new.group" => {
            "Create and activate an empty layer group above the active layer or inside an expanded active group. Group opacity uses percent (0–100), unlike layer.new.layer and layer.setProps (0–1). Returns the group's stable layer ID."
        }
        "layer.select" => {
            "Change the Layers-panel selection without changing pixels. Use a stable layer ID when possible. Top-level name is an exact-name lookup and requires occurrence for duplicate names; layer:string uses the general name selector. A recorded group-end index is a no-op."
        }
        "layer.setProps" => {
            "Change properties on one explicit/active layer or all IDs in layers in one history step. Omitted properties remain unchanged. opacity and fill are fractions 0–1, not percentages. Pass Through applies only to groups. A Background becomes a normal layer when changing its compositing properties."
        }
        "layer.delete" => {
            "Delete one explicit layer or, when omitted, all selected layers. Removes their contents, masks and children; use hideLayers to hide without deletion. Undo restores the deletion."
        }
        "layer.duplicate" => {
            "Duplicate an explicit layer, or the selected layer set when layer is omitted, with editable content preserved. The copy becomes active. Artboards are placed beside originals unless inPlace=true. document may be the active index or new; copying to another open document requires layer.copyToDocument. name applies only to a single copy."
        }
        "layer.renameLayer" => {
            "Rename one explicit or active layer without changing its content. A non-whitespace name is required; surrounding spaces are preserved."
        }
        _ => return None,
    })
}

pub(super) fn schema(id: &str) -> Option<Value> {
    Some(match id {
        "layer.new.layer" | "layer.new.group" => new_layer(id == "layer.new.group"),
        "layer.delete" => object(json!({"layer":types::layer()}), &[]),
        "layer.duplicate" => object(
            json!({
                "layer":types::layer(),
                "inPlace":{"type":"boolean","default":false,"description":"Keep a copied artboard at its source position instead of placing it beside the original; ordinary layers stay in place either way."},
                "name":{"type":"string","description":"Non-empty copy name for a single layer; ignored for a multi-layer selection. Omit/empty uses the normal copy naming preference."},
                "document":{"anyOf":[{"type":"integer","minimum":0,"maximum":u64::MAX},{"const":"new"}],"description":"Omit for the active document, or provide its zero-based index. new creates/activates a same-size document holding the copies. Other open-document indices are rejected; use layer.copyToDocument."},
                "documentName":{"type":"string","description":"New document name when document=new; omit/empty uses the source name plus copy. Ignored otherwise."}
            }),
            &[],
        ),
        "layer.renameLayer" => object(
            json!({"layer":types::layer(),"name":{"type":"string","pattern":"\\S","description":"New layer name containing at least one non-whitespace character; surrounding spaces are retained."}}),
            &["name"],
        ),
        "layer.select" => {
            let mut layer = types::layer();
            if let Some(variants) = layer.get_mut("anyOf").and_then(Value::as_array_mut) {
                variants.retain(|variant| variant.get("type").and_then(Value::as_str) != Some("null"));
            }
            let mut schema = object(
                json!({
                    "layer":layer,
                    "name":{"type":"string","description":"Exact layer name; use occurrence for duplicate names. Mutually exclusive with layer."},
                    "occurrence":{"type":"integer","minimum":0,"description":"Zero-based match in document walk order, not panel order. Requires name."},
                    "mode":{"enum":["replace","toggle","range","add","remove"],"default":"replace","description":"replace selects only this layer; toggle is Cmd/Ctrl-click; range is Shift-click from the current anchor; add extends; remove deselects while retaining at least one layer."}
                }),
                &[],
            );
            schema["oneOf"] = json!([{"required":["layer"],"not":{"required":["name"]}},{"required":["name"],"not":{"required":["layer"]}}]);
            schema["dependentRequired"] = json!({"occurrence":["name"]});
            schema
        }
        "layer.setProps" => object(
            json!({
                "layer":types::layer(),
                "layers":{"type":["array","null"],"items":{"type":"integer","minimum":0,"maximum":u64::MAX},"minItems":1,"maxItems":10000,"description":"Stable numeric IDs; overrides layer. Repeated IDs are deduplicated; omit/null targets one explicit or active layer."},
                "name":{"type":"string","description":"New name; valid only when the resolved target contains one distinct layer."},
                "visible":{"type":"boolean","description":"Show/hide without deleting pixels. Visibility history follows the user's history preference."},
                "opacity":{"type":"number","minimum":0,"maximum":1,"description":"Layer opacity fraction (1 = 100%). Values outside 0–1 are errors."},
                "fill":{"type":"number","minimum":0,"maximum":1,"description":"Fill opacity fraction, independent of layer-style opacity."},
                "blend":types::blend(false,true),
                "clipped":{"type":"boolean","description":"Whether this layer is clipped to the layer beneath it."},
                "locked":{"type":"boolean","description":"Set all-lock flag; granular locks can be changed using locks."},
            "locks":described(object(json!({
                "transparency":{"type":"boolean","description":"Lock transparency; painting cannot change transparent/opaque coverage."},
                "pixels":{"type":"boolean","description":"Lock pixel edits on this layer. Enabling this clears the separate transparency lock, whose protection is included."},
                "position":{"type":"boolean","description":"Lock layer position and transforms."},
                "artboard":{"type":"boolean","description":"Lock artboard-specific editing."},
                "all":{"type":"boolean","description":"Set or clear the all-lock flag without changing the individual lock flags."}
            }), &[]), "Granular layer locks. Omitted members remain unchanged except that enabling pixels clears the separate transparency lock."),
                "channels":{"type":"array","items":{"type":"boolean"},"maxItems":32,"description":"Advanced Blending channel inclusion flags in the document's native component order; true blends that component, false excludes it."}
            }),
            &[],
        ),
        _ => return None,
    })
}

fn new_layer(group: bool) -> Value {
    let mut blend = types::blend(false, group);
    blend["default"] = json!(if group { "Pass Through" } else { "Normal" });
    let mut fields = json!({
        "name":{"type":["string","null"],"description":"New layer/group name; omit/null for an automatically generated name."},
        "color":{"enum":["none","red","orange","yellow","green","seafoam","blue","indigo","magenta","fuchsia","violet","gray",null],"default":"none","description":"Layers-panel colour label, not a painted fill colour."},
        "mode":blend,
        "opacity":{"type":["number","null"],"minimum":0,"maximum":if group {100.0}else{1.0},"default":if group {100.0}else{1.0},"description":if group {"Group opacity percentage (0–100); not a fraction."}else{"Raster layer opacity fraction (0–1); not a percentage."}},
        "position":{"enum":["above","below",null],"default":"above","description":"Insertion position; below never inserts under the Background."}
    });
    if !group && let Some(properties) = fields.as_object_mut() {
        properties.insert("clipToPrevious".into(), json!({"type":["boolean","null"],"default":false,"description":"Create a clipping mask with the previous layer. A layer inserted inside a clipping group joins it."}));
        properties.insert("fillNeutral".into(), json!({"type":["boolean","null"],"default":false,"description":"Fill with the blend mode's neutral colour; errors for modes with no neutral fill. Omit for an empty layer."}));
    }
    let mut schema = object(fields, &[]);
    if let Some(mode) = schema.pointer_mut("/properties/mode") {
        *mode = described(
            json!({"anyOf":[mode.clone(),{"type":"null"}]}),
            "Blend mode name, or null for the new-layer default. Pass Through is supported only for groups.",
        );
    }
    schema
}
