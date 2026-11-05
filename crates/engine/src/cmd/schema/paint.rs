use serde_json::{Value, json};

use super::{brush, described, object, types};

pub(super) fn description(id: &str) -> Option<&'static str> {
    Some(match id {
        "paint.stroke" => {
            "Paint a brush stroke on the explicit/active raster layer or selected mask/channel, respecting the pixel selection and locks. points are document pixels, opacity/flow/hardness are 0–1, and spacing is a fraction of brush diameter. fill=true fills the enclosed polygon instead of stamping its outline. Use shape.create for editable vector geometry. Returns the damaged document rectangle."
        }
        "paint.bucket" => {
            "Flood-fill a region starting at document pixel (x,y), with foreground colour or a pattern. sampleAllLayers can read other visible layers, but writes only to the target. opacity is percent 1–100, unlike paint.stroke's 0–1. Honors selection, transparency locks and mask/channel target."
        }
        "edit.fill" => {
            "Fill the pixel selection, or the whole canvas when no selection exists, on the explicit/active layer or selected mask/channel. Source may be a colour, pattern, matching-size history state, or content-aware pixels. opacity is percent 0–100, not paint.stroke's 0–1 fraction. preserveTransparency and layer transparency locks retain existing alpha. Content-aware requires a selection on raster layer pixels."
        }
        "paint.gradient" => {
            "Raster-paint a gradient between document pixel positions from and to, within the selected target/selection. Prefer explicit colours or stops for reproducibility; otherwise uses the current gradient preset. opacity is percent 1–100. Use a gradient fill layer or vector shape fill when the gradient must remain editable."
        }
        "tools.setColors" => {
            "Set the toolbox foreground/background colours without painting. RGB inputs are 0–1 components or hex; optional CMYK ink arrays preserve the original picked inks for later CMYK painting. Omitted colours remain unchanged."
        }
        "tools.swapColors" => "Swap the current foreground and background colours without changing document pixels.",
        "tools.defaultColors" => "Reset toolbox colours to black/white for pixels or the application's mask defaults when a mask is targeted. Does not paint.",
        "select.rect" => {
            "Create a rectangular or elliptical pixel selection in document coordinates. x,y,width,height are whole pixels, not CSS points. mode combines it with the current selection; feather softens the new shape. This selects pixels but does not draw a rectangle layer."
        }
        _ => return None,
    })
}

pub(super) fn schema(id: &str) -> Option<Value> {
    Some(match id {
        "paint.stroke" => stroke(),
        "edit.fill" => fill(),
        "paint.bucket" => object(
            json!({
                "layer":types::layer(),"target":types::target(),
                "x":{"type":"number","default":0,"description":"Seed x in document pixels, floored to a whole pixel."},
                "y":{"type":"number","default":0,"description":"Seed y in document pixels, floored to a whole pixel."},
                "tolerance":{"type":"number","minimum":0,"maximum":255,"default":32,"description":"Colour similarity tolerance on the 0–255 dialog scale."},
                "contiguous":{"type":"boolean","default":true,"description":"Fill only connected matching pixels; false fills matching pixels throughout the target."},
                "antiAlias":{"type":"boolean","default":true,"description":"Soften the edge coverage of the filled region."},
                "sampleAllLayers":{"type":"boolean","default":false,"description":"Use the visible composite to find the region for a pixel target; still writes only to the selected target."},
                "contents":{"enum":["foreground","pattern"],"default":"foreground","description":"Fill source; contents wins over source when both are supplied."},
                "source":{"enum":["foreground","pattern"],"description":"Alias of contents."},
                "color":types::rgb(),
                "pattern":{"type":"string","description":"Installed/document pattern ID or name, required when filling a pattern unless patternName resolves it; discover with pattern.list."},
                "patternName":{"type":"string","description":"Fallback pattern name when a recorded pattern ID is unavailable."},
                "scale":{"type":"number","minimum":1,"maximum":1000,"default":100,"description":"Pattern scale percentage; only used for pattern fill."},
                "angle":{"type":"number","default":0,"description":"Pattern angle in degrees, wrapped to -180..180."},
                "phase":types::tuple(2,"Pattern phase [x,y] in document pixels, each within ±10000000; default [0,0]."),
                "opacity":percent_opacity(),"mode":types::blend(true,false)
            }),
            &[],
        ),
        "paint.gradient" => gradient(),
        "tools.swapColors" | "tools.defaultColors" => object(json!({}), &[]),
        "tools.setColors" => object(
            json!({
                "foreground":types::rgb(),"background":types::rgb(),
                "foregroundCmyk":inks(),"backgroundCmyk":inks()
            }),
            &[],
        ),
        "select.rect" => object(
            json!({
                "x":{"type":"integer","minimum":i32::MIN,"maximum":i32::MAX,"description":"Left document pixel coordinate; can be outside the canvas."},
                "y":{"type":"integer","minimum":i32::MIN,"maximum":i32::MAX,"description":"Top document pixel coordinate; positive is downwards."},
                "width":{"type":"integer","minimum":0,"maximum":i32::MAX,"description":"Selection width in pixels (not x1)."},
                "height":{"type":"integer","minimum":0,"maximum":i32::MAX,"description":"Selection height in pixels (not y1)."},
                "mode":{"enum":["replace","add","subtract","intersect"],"default":"replace","description":"How the new shape combines with current pixel selection."},
                "ellipse":{"type":"boolean","default":false,"description":"Use an ellipse inscribed in the rectangle instead of a rectangle."},
                "line":{"enum":["row","column"],"description":"Single-row/column marquee history label; provide the actual one-pixel thickness using height/width."},
                "antiAlias":{"type":"boolean","default":true,"description":"Anti-alias the ellipse boundary; rectangles have whole-pixel edges."},
                "feather":{"type":"number","minimum":0,"maximum":1000,"default":0,"description":"Feather radius in document pixels, applied to the new shape before combination."}
            }),
            &["x", "y", "width", "height"],
        ),
        _ => return None,
    })
}

fn percent_opacity() -> Value {
    json!({"type":"number","minimum":1,"maximum":100,"default":100,"description":"Opacity percentage (1–100), not a 0–1 fraction. Out-of-range values are clamped by the tool."})
}

fn fill() -> Value {
    let mut contents = types::choices(
        &crate::fill_cmds::CONTENTS,
        "Fill source, case-insensitive. Omit for color, using the foreground when color is absent. contentAware requires an existing selection and layer-pixel target; history requires a matching-size source containing this target.",
    );
    contents["default"] = json!("color");
    let mut mode = types::blend(false, false);
    mode["default"] = json!("Normal");
    let mut schema = object(
        json!({
            "layer":types::layer(),"target":types::target(),"contents":contents,"color":types::rgb(),
            "pattern":{"type":"string","description":"Pattern ID or name for contents=pattern, resolved before patternName. Discover with pattern.list."},
            "patternName":{"type":"string","description":"Fallback pattern name if pattern did not resolve."},
            "scale":{"type":"number","minimum":1,"maximum":1000,"default":100,"description":"Pattern scale percentage, not ratio; runtime clamps to 1–1000 percent."},
            "angle":{"type":"number","default":0,"description":"Pattern rotation in degrees; normalized at runtime."},
            "phase":described(json!({"type":"array","items":{"type":"number","minimum":-crate::pattern_fill_cmds::MAX_PHASE,"maximum":crate::pattern_fill_cmds::MAX_PHASE},"minItems":2,"maxItems":2}), "Pattern offset [x,y] in document pixels; omit for [0,0]."),
            "link":{"type":"boolean","default":true,"description":"Shared pattern-placement flag; Fill paints from the canvas origin regardless."},
            "state":{"type":["integer","null"],"minimum":0,"description":"History panel row, oldest=0 and redo rows last. For contents=history; mutually exclusive with a non-null snapshot. Omit/null uses the chosen History Brush source."},
            "snapshot":{"type":["integer","null"],"minimum":0,"maximum":u64::MAX,"description":"Snapshot ID for contents=history, not a history row index. Mutually exclusive with a non-null state."},
            "colorAdaptation":{"type":"boolean","default":true,"description":"Adapt source colours for content-aware fill only."},
            "scaleAdaptation":{"type":"boolean","default":false,"description":"Try scaled source candidates for content-aware fill only."},
            "mirror":{"type":"boolean","default":false,"description":"Try mirrored content-aware candidates."},
            "rotationAdaptation":{"type":"boolean","default":false,"description":"Try quarter-turn content-aware candidates within the sampling budget."},
            "seed":{"type":"integer","minimum":0,"maximum":u64::MAX,"default":1,"description":"Deterministic content-aware sampling seed; unused for other contents."},
            "mode":mode,
            "opacity":{"type":"number","minimum":0,"maximum":100,"default":100,"description":"Coverage percentage, not a fraction; runtime clamps to 0–100."},
            "preserveTransparency":{"type":"boolean","default":false,"description":"Keep existing target alpha; the layer transparency lock also enforces this."}
        }),
        &[],
    );
    schema["allOf"] = json!([{
        "if":{"properties":{"contents":types::choices(&["history"], "History source form.")},"required":["contents"]},
        "then":{"not":{"required":["state","snapshot"],"properties":{"state":{"type":"integer"},"snapshot":{"type":"integer"}}}}
    }]);
    schema["examples"] = json!([{"contents":"color","color":"#2f718c","opacity":50,"preserveTransparency":false}]);
    schema
}

fn inks() -> Value {
    described(
        json!({"anyOf":[{"type":"null"},{"type":"array","items":{"type":"number","minimum":0,"maximum":1},"minItems":4,"maxItems":4}]}),
        "Picked CMYK inks [cyan,magenta,yellow,black] in 0–1; omit/null to leave the remembered inks unchanged. Pair with the corresponding RGB display colour.",
    )
}

fn gradient() -> Value {
    let stop_color = json!({"type":"string","anyOf":[{"enum":["foreground","background"]},{"pattern":"^#?[0-9a-fA-F]{6}$"}]});
    let stop = json!({"type":"array","prefixItems":[{"type":"number","minimum":0,"maximum":1},stop_color],"minItems":2,"maxItems":2});
    let mut schema = object(
        json!({
            "layer":types::layer(),"target":types::target(),
            "from":types::tuple(2,"Gradient start [x,y] in document pixels."),
            "to":types::tuple(2,"Gradient end [x,y] in document pixels."),
            "style":{"enum":["linear","radial","angle","reflected","diamond"],"default":"linear","description":"Gradient geometry around the from/to axis."},
            "colors":{"type":"array","items":types::rgb(),"description":"Legacy evenly spaced colours. A non-empty list takes precedence over gradient/stops/current preset."},
            "gradient":{"type":"string","description":"Preset name from gradient.presets.list; takes precedence over stops when colors is absent/empty."},
            "stops":{"type":"array","minItems":2,"items":{"anyOf":[stop,stop_color]},"description":"[position,colour] pairs in 0–1, or evenly spaced colour strings. Colours are #RRGGBB, foreground or background; alpha/colour arrays are not accepted by this preset form."},
            "transparency":{"type":"array","maxItems":1024,"items":{"type":"array","prefixItems":[{"type":"number","minimum":0,"maximum":1},{"type":"number","minimum":0,"maximum":100}],"minItems":2,"maxItems":2},"description":"Opacity stops [position 0–1, opacity percent 0–100]. Empty means opaque; supplied stops override preset opacity stops."},
            "method":{"enum":["classic","perceptual","linear","smooth"],"description":"Optional Photoshop-style interpolation; omit to interpolate the resolved stops directly."},
            "smoothness":{"type":"number","minimum":0,"maximum":100,"default":100,"description":"Interpolation smoothness percentage, used only when method is supplied."},
            "reverse":{"type":"boolean","default":false,"description":"Reverse gradient direction."},
            "dither":{"type":"boolean","default":true,"description":"Dither to reduce visible banding."},
            "opacity":percent_opacity(),"mode":types::blend(false,false)
        }),
        &["from", "to"],
    );
    schema["examples"] = json!([{"from":[0,0],"to":[64,0],"colors":["#2f718c","#f0dfb8"],"opacity":100}]);
    schema
}

fn stroke() -> Value {
    let point_object = object(
        json!({
            "x":{"type":"number","minimum":-1000000,"maximum":1000000,"default":0},
            "y":{"type":"number","minimum":-1000000,"maximum":1000000,"default":0},
            "pressure":{"type":"number","minimum":0,"maximum":1,"default":1},
            "tiltX":{"type":"number","minimum":-90,"maximum":90,"default":0},
            "tiltY":{"type":"number","minimum":-90,"maximum":90,"default":0},
            "rotation":{"type":"number","minimum":0,"maximum":360,"default":0},
            "time":{"type":"number","default":0},"wheel":{"type":"number","minimum":0,"maximum":1,"default":1}
        }),
        &[],
    );
    let point_array = json!({"type":"array","prefixItems":[
        {"type":"number","minimum":-1000000,"maximum":1000000},{"type":"number","minimum":-1000000,"maximum":1000000},
        {"type":"number","minimum":0,"maximum":1},{"type":"number","minimum":-90,"maximum":90},{"type":"number","minimum":-90,"maximum":90},
        {"type":"number","minimum":0,"maximum":360},{"type":"number"},{"type":"number","minimum":0,"maximum":1}
    ],"minItems":2,"maxItems":8});
    let mut schema = object(
        json!({
            "layer":types::layer(),"target":types::target(),
            "points":{"type":"array","minItems":1,"items":{"anyOf":[point_array,point_object]},"description":"Samples [x,y,pressure?,tiltX?,tiltY?,rotation?,timeMs?,wheel?] or StrokePoint objects (timestamp key time). Document pixels, degrees and milliseconds; omitted stylus data uses mouse defaults."},
            "brush":brush::patch(),
            "preset":{"type":"string","description":"Brush preset name; omit to use the current brush. Preset picking respects section locks and preserves smoothing. Scalar overrides apply after the preset and brush patch. Stroke colour/background always come from top-level color/tool colours; erase and seed always come from top-level values or their stroke defaults, not the nested patch."},
            "size":{"type":"number","minimum":0.5,"maximum":5000,"description":"Brush diameter in document pixels."},
            "hardness":{"type":"number","minimum":0,"maximum":1,"description":"0 soft edge, 1 hard edge."},
            "opacity":{"type":"number","minimum":0,"maximum":1,"description":"Whole-stroke coverage ceiling (fraction, not percent)."},
            "flow":{"type":"number","minimum":0,"maximum":1,"description":"Per-dab coverage (fraction, not percent)."},
            "spacing":{"type":"number","minimum":0.01,"maximum":10,"description":"Dab distance as a fraction of brush diameter; 0.1 = 10%."},
            "color":types::rgb(),"mode":types::blend(true,false),
            "erase":{"type":"boolean","default":false,"description":"Erase instead of paint; on a locked-transparent layer/mask it paints the background colour."},
            "fill":{"type":"boolean","default":false,"description":"Fill the polygon enclosed by points; requires at least three points and cannot combine with erase/clear."},
            "smoothing":{"type":"number","minimum":0,"maximum":1,"description":"Override stroke smoothing amount, enabling smoothing."},
            "zoom":{"type":"number","exclusiveMinimum":0,"default":1,"description":"View zoom ratio used by zoom-adjusted smoothing; coordinates remain document pixels."},
            "seed":{"type":"integer","minimum":0,"maximum":u64::MAX,"description":"Deterministic jitter seed; omitted derives it from points."}
        }),
        &["points"],
    );
    schema["allOf"] = json!([{
        "if":{"properties":{"fill":{"const":true}},"required":["fill"]},
        "then":{"properties":{"points":{"minItems":3},"erase":{"const":false},"mode":{"not":{"pattern":"^[cC][lL][eE][aA][rR]$"}}}}
    }]);
    if let Some(examples) = schema.pointer_mut("/properties/mode/examples").and_then(Value::as_array_mut) {
        examples.push(json!("behind"));
    }
    schema["examples"] = json!([{"points":[[8,8,1],[32,24,1]],"size":6,"hardness":0.8,"opacity":1,"color":"#2f718c","seed":7}]);
    brush::with_definitions(schema)
}
