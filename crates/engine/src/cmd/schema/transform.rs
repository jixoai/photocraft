use serde_json::{Value, json};

use super::{described, object, types};

pub(super) fn description(id: &str) -> Option<&'static str> {
    (id == "edit.transform").then_some("Transform an explicit layer, explicit layer IDs, or selected layers in one undo step. Vector/text/smart-object layers retain editable content; raster layers move the selected pixels. rect uses [x0,y0,x1,y1] corners, unlike shape.create's [x,y,width,height]. Choose quad, matrix, or relative scale/rotate/move; scale is percent and positive rotation is clockwise. Mask/channel targets transform their own surface.")
}

pub(super) fn schema(id: &str) -> Option<Value> {
    if id != "edit.transform" {
        return None;
    }
    let mut schema = object(
        json!({
            "layer":types::layer(),
            "layers":{"type":["array","null"],"items":{"type":"integer","minimum":0,"maximum":u64::MAX},"minItems":1,"maxItems":10000,"description":"Explicit stable layer IDs, deduplicated. Mutually exclusive with a non-null layer or mask/channel target; omitted uses current selected layers, excluding Background."},
            "target":types::target(),
            "rect":types::tuple(4,"Source frame [x0,y0,x1,y1] in document pixels, not [x,y,width,height]. Omit to use selected content bounds; the frame must not collapse."),
            "quad":{"type":"array","items":types::tuple(2,"Destination [x,y] in document pixels."),"minItems":4,"maxItems":4,"description":"Destination corners clockwise from top-left. Highest-priority form, overriding matrix and relative parameters; must form a nondegenerate transform."},
            "matrix":types::tuple(6,"Affine [a,b,c,d,e,f]: x'=a*x+c*y+e, y'=b*x+d*y+f. Used when quad is absent; overrides relative parameters and must be invertible."),
            "scaleX":{"type":["number","null"],"not":{"type":"number","exclusiveMinimum":-0.0001,"exclusiveMaximum":0.0001},"description":"Horizontal scale percent (100 unchanged, negative flips); omit/null uses 100. Near-zero scaling is rejected."},
            "scaleY":{"type":["number","null"],"not":{"type":"number","exclusiveMinimum":-0.0001,"exclusiveMaximum":0.0001},"description":"Vertical scale percent (100 unchanged, negative flips); omit/null uses 100. Near-zero scaling is rejected."},
            "rotate":{"type":["number","null"],"description":"Clockwise rotation in degrees; omit/null means 0 when another relative parameter is supplied."},
            "dx":{"type":["number","null"],"description":"Horizontal document-pixel translation applied after relative scaling/rotation."},
            "dy":{"type":["number","null"],"description":"Vertical document-pixel translation applied after relative scaling/rotation."},
            "reference":described(json!({"anyOf":[{"enum":["center","topLeft","top","topRight","left","right","bottomLeft","bottom","bottomRight",null]},types::tuple(2,"Explicit relative-transform pivot [x,y] in document pixels.")]}),"Scale/rotation pivot. Omit/null means source-frame centre; ignored for quad/matrix."),
            "interpolation":{"enum":["bicubic","bilinear","nearest","bicubicSharper","bicubicSmoother","bicubicAutomatic"],"default":"bicubic","description":"Raster resampling method. Bicubic variants currently use Bicubic; vector/text affine transforms retain editable geometry."}
        }),
        &[],
    );
    schema["anyOf"] = json!([
        {"required":["quad"]},{"required":["matrix"]},
        {"required":["scaleX"],"properties":{"scaleX":{"type":"number"}}},
        {"required":["scaleY"],"properties":{"scaleY":{"type":"number"}}},
        {"required":["rotate"],"properties":{"rotate":{"type":"number"}}},
        {"required":["dx"],"properties":{"dx":{"type":"number"}}},
        {"required":["dy"],"properties":{"dy":{"type":"number"}}}
    ]);
    schema["examples"] = json!([{"dx":8,"dy":-4,"rotate":5,"reference":"center"}]);
    Some(schema)
}
