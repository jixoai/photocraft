use serde_json::{Value, json};

use super::object;

pub(super) fn description(id: &str) -> Option<&'static str> {
    Some(match id {
        "image.imageSize" => {
            "Resize document pixels and editable geometry, or change print resolution only with resample=none. A single explicit dimension preserves aspect ratio; two dimensions may distort. percent supplies missing sides, not a bound on an explicit side. Dimensions round/clamp to 1–300000 px; allocation budgets and document state remain runtime checks. Returns actual resulting pixel dimensions."
        }
        "image.canvasSize" => {
            "Change the canvas frame without resampling existing content. anchor determines how the old content shifts. relative=true treats pixel dimensions as signed additions; absolute dimensions can use px or percent. Omitted sides remain unchanged. Background extensions use extensionColor; transparent extension unlocks the Background into a normal layer. Normal-layer extensions stay transparent. A smaller frame can hide off-canvas content; it is not destructive image resampling."
        }
        _ => return None,
    })
}

pub(super) fn schema(id: &str) -> Option<Value> {
    Some(match id {
        "image.imageSize" => {
            let mut schema = object(
                json!({
                    "width":{"type":["number","null"],"exclusiveMinimum":0,"description":"Desired width in units; omit/null preserves aspect ratio when only height is supplied. Ignored when resample=none."},
                    "height":{"type":["number","null"],"exclusiveMinimum":0,"description":"Desired height in units; omit/null preserves aspect ratio when only width is supplied. Ignored when resample=none."},
                    "units":{"enum":["px","pt"],"default":"px","description":"pt converts using current document resolution/72 before any new resolution is applied."},
                    "percent":{"type":"number","exclusiveMinimum":0,"description":"Percent of both current pixel sides; an explicit width/height overrides that side. 100 is unchanged, not 1."},
                    "resolution":{"type":"number","minimum":1,"maximum":10000,"description":"New print resolution in pixels per inch; omit to retain current resolution. Runtime clamps to 1–10000."},
                    "keepPrintSize":{"type":"boolean","default":false,"description":"With only resolution and resampling enabled, scale pixels by new/old resolution to preserve physical print size. Ignored if dimensions/percent supply a size or resample=none."},
                    "scaleStyles":{"type":"boolean","default":true,"description":"Scale layer effects with resampled content."},
                    "resample":{"enum":["automatic","preserveDetails","bicubicSmoother","bicubicSharper","bicubicAutomatic","bicubic","nearest","nearestNeighbor","bilinear","lanczos","none"],"default":"bicubic","description":"none changes resolution only and ignores requested dimensions. automatic selects enlargement/reduction methods; Indexed Color uses nearest for resampling."}
                }),
                &[],
            );
            schema["examples"] = json!([{"width":32,"resample":"nearest"},{"resolution":144,"resample":"none"}]);
            schema
        }
        "image.canvasSize" => {
            let mut schema = object(
                json!({
                    "width":{"type":"number","description":"Absolute width, or signed pixel addition when relative=true. Omitted keeps the current side. Final side rounds/clamps to 1–300000 pixels."},
                    "height":{"type":"number","description":"Absolute height, or signed pixel addition when relative=true. Omitted keeps the current side. Final side rounds/clamps to 1–300000 pixels."},
                    "units":{"enum":["px","percent"],"default":"px","description":"percent measures each side relative to its current pixels; allowed only for absolute dimensions."},
                    "relative":{"type":"boolean","default":false,"description":"Treat width/height as pixel additions or removals; cannot combine with units=percent."},
                    "anchor":{"enum":["topLeft","top","topRight","left","center","right","bottomLeft","bottom","bottomRight"],"default":"center","description":"Point on the old frame kept fixed as the canvas changes; exact case-sensitive name."},
                    "extensionColor":{"type":"string","anyOf":[{"enum":["background","foreground","white","black","transparent"]},{"pattern":"^#[0-9a-fA-F]{6}$"}],"default":"background","description":"Colour for newly exposed Background area only; does not fill ordinary layers."}
                }),
                &[],
            );
            schema["allOf"] = json!([
                {"if":{"properties":{"units":{"const":"percent"}},"required":["units"]},"then":{"properties":{"relative":{"const":false},"width":{"exclusiveMinimum":0},"height":{"exclusiveMinimum":0}}}},
                {"if":{"not":{"properties":{"relative":{"const":true}},"required":["relative"]}},"then":{"properties":{"width":{"exclusiveMinimum":0},"height":{"exclusiveMinimum":0}}}}
            ]);
            schema["examples"] = json!([{"width":8,"height":-4,"relative":true,"anchor":"topLeft"},{"width":150,"units":"percent","anchor":"center"}]);
            schema
        }
        _ => return None,
    })
}
