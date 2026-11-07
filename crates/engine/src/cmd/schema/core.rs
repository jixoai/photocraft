use serde_json::{Value, json};

use super::{object, types};

pub(super) fn description(id: &str) -> Option<&'static str> {
    Some(match id {
        "file.new" => {
            "Create and activate a new document. Pixel dimensions are independent of display zoom; resolution is pixels per inch. Returns its session document index."
        }
        "command.list" => {
            "Discover all engine commands and their parameter schemas, semantic descriptions and current enabled state. Returns a bare array, not a commands wrapper. Contracts are available even before a document is open; inspect schema coverage before constructing a call."
        }
        "document.inspect" => {
            "Read a document's structure, layer IDs, selection bounds, revision and undo/redo state without modifying it. Omit document for the active document or supply its zero-based session index."
        }
        "document.pixel" => "Sample the composited active document at a document pixel coordinate. Returns RGBA components in 0–1, not 0–255.",
        "layer.list" => "Read the layer hierarchy in display order. Use the returned stable IDs for subsequent edits; names can be ambiguous.",
        "layer.hideLayers" => "Hide the specified layer, or all selected layers when layer is omitted. Does not delete pixels or change layer order.",
        "layer.showLayers" => "Show the specified layer, or all selected layers when layer is omitted. Does not change layer order.",
        "edit.undo" => "Undo the most recent history step in the active document. Inspect canUndo first; do not retry blindly after a timeout.",
        "edit.redo" => "Redo the next undone history step in the active document. Inspect canRedo first.",
        "select.all" => "Select the whole active document canvas. Coordinates remain in document pixels.",
        "select.deselect" => "Remove the active pixel selection without deleting image content.",
        "select.inverse" => "Invert selection coverage within the active document canvas.",
        "path.list" => "Read named paths, work path, clipping path and the active layer's shape/vector-mask or type path.",
        "path.info" => "Read the exact geometry of a named path, work path or active layer path without modifying it.",
        "shape.info" => "Read a shape layer's path, fill, stroke, live-shape parameters and document-space bounds.",
        "type.info" => "Read a type layer's text, styles, character-offset runs, transform, layout and bounds. Use this before a localized text edit.",
        "type.fonts" => "List font families, or available faces of one family. Font availability depends on the current installation/build.",
        "jobs.list" => "Read running background jobs and recent completion states, progress, results and errors.",
        "jobs.cancel" => {
            "Request cancellation of one background job, or every running job when job is omitted. Inspect jobs.list for completion; some committing jobs cannot be cancelled."
        }
        _ => return None,
    })
}

pub(super) fn schema(id: &str) -> Option<Value> {
    Some(match id {
        "file.new" => object(
            json!({
                "width":{"type":"number","minimum":1,"maximum":300000,"default":1920,"description":"Canvas width in pixels, rounded to the nearest whole pixel. Large documents remain subject to memory/allocation limits."},
                "height":{"type":"number","minimum":1,"maximum":300000,"default":1080,"description":"Canvas height in pixels, rounded to the nearest whole pixel."},
                "name":{"type":"string","default":"Untitled","description":"Document title; not a filesystem path."},
                "mode":{"type":"string","enum":["rgb","gray","grayscale","cmyk","lab"],"default":"rgb","description":"Document colour model. The chosen profile must match it."},
                "depth":{"type":"integer","enum":[8,16,32],"default":8,"description":"Bits per component: 8-bit integer, 16-bit integer or 32-bit float."},
                "background":{"type":"string","anyOf":[{"enum":["white","black","transparent","backgroundColor"]},{"pattern":"^#?([0-9a-fA-F]{6}|[0-9a-fA-F]{8})$"}],"default":"white","description":"Initial canvas colour. Use transparent for no Background layer, backgroundColor for the RGB triplet, or a 6/8-digit hexadecimal colour."},
                "backgroundColor":{"type":"array","items":{"type":"number","minimum":0,"maximum":1},"minItems":3,"maxItems":3,"description":"Exactly three RGB components in 0–1, used with background=backgroundColor; omit to use the toolbox background colour. Alpha is not accepted here."},
                "resolution":{"type":"number","minimum":1,"maximum":30000,"default":72,"description":"Print resolution in pixels per inch. Does not resample the pixel dimensions."},
                "artboard":{"type":"boolean","default":false,"description":"Create an artboard with an empty Layer 1 instead of a normal Background layer."},
                "profile":{"type":"string","minLength":1,"default":"working","description":"working, none, or a supported built-in ICC profile identifier. Native callers may also use a permitted ICC path; browser callers cannot read host paths."}
            }),
            &[],
        ),
        "command.list" | "edit.undo" | "edit.redo" | "select.all" | "select.deselect" | "select.inverse" | "path.list" | "jobs.list" => object(json!({}), &[]),
        "document.inspect" => object(
            json!({"document":{"type":"integer","minimum":0,"maximum":u64::MAX,"description":"Zero-based session document index; omit for the active document. Null is not an index."}}),
            &[],
        ),
        "document.pixel" => object(
            json!({
                "x":{"type":"integer","minimum":i32::MIN,"maximum":i32::MAX,"default":0,"description":"Horizontal document pixel coordinate; origin is the canvas top-left. Outside-canvas samples are transparent."},
                "y":{"type":"integer","minimum":i32::MIN,"maximum":i32::MAX,"default":0,"description":"Vertical document pixel coordinate; positive is downwards."}
            }),
            &[],
        ),
        "layer.list" => object(
            json!({
                "document":{"type":["integer","null"],"minimum":0,"description":"Zero-based session document index; omit or use null for the active document."},
                "flat":{"type":["boolean","null"],"default":false,"description":"Return rows with parent and depth rather than nested group children; null uses the default."}
            }),
            &[],
        ),
        "layer.hideLayers" | "layer.showLayers" | "shape.info" | "type.info" => object(json!({"layer":types::layer()}), &[]),
        "path.info" => object(
            json!({"name":{"type":"string","default":"work","description":"A stored path name, work for the work path, or layer for the active layer's path."}}),
            &[],
        ),
        "type.fonts" => object(json!({"family":{"type":"string","description":"Font family to inspect; omit to list all available families."}}), &[]),
        "jobs.cancel" => object(
            json!({"job":{"type":["integer","null"],"minimum":0,"maximum":u64::MAX,"description":"Job ID returned by jobs.list. Omit or use null to request cancellation of all running jobs."}}),
            &[],
        ),
        _ => return None,
    })
}
