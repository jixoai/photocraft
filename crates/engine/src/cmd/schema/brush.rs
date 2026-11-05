use photocraft_color::BlendMode;
use photocraft_paint::{BrushSettings, Control, MaskMode, PatternStyle};
use serde_json::{Value, json};

use super::{described, object};

pub(super) fn description(id: &str) -> Option<&'static str> {
    Some(match id {
        "brush.get" => {
            "Read the current complete BrushSettings as camelCase JSON without changing it. Nested percentages are 0–1 fractions, scatter may reach 10, sizes are document pixels and angles are degrees. Stored bitmaps can appear as references or embedded tiles."
        }
        "tools.setBrush" => {
            "Change the current brush without painting. Order: current brush, optional preset with section locks preserved, optional reset to initial defaults, optional nested brush patch, then root BrushSettings fields. Omitted fields are preserved, not reset. reset wins over preset identity. Read brush.get to verify the result."
        }
        "brush.presets.list" => {
            "List brush preset names and summaries. full=true also returns complete settings and loads stored tips where possible; bitmap JSON can be large. Listing does not select a preset."
        }
        "brush.presets.save" => {
            "Save or replace a case-insensitively named preset from the current brush plus an optional deep-merged brush patch. Capture flags control what a future preset pick restores. Marks the saved preset current but does not apply its patched brush to the current tool or paint."
        }
        "brush.presets.update" => {
            "Replace an existing preset's settings with the current brush plus an optional deep-merged patch. Omit name to update the current preset; errors if no preset is current. Does not pick the updated settings or paint."
        }
        "brush.presets.delete" => "Remove a case-insensitively named brush preset. Does not erase existing strokes; discover names using brush.presets.list.",
        "brush.presets.setCurrent" => {
            "Mark an existing preset as current by identity only. Does not apply its settings; use tools.setBrush with preset to pick it. Useful after independently copying tip settings."
        }
        "brush.defineFromSelection" => {
            "Create and select a sampled brush preset from the selected pixels of the active raster layer. Dark, opaque pixels deposit paint; white or transparent pixels do not. Crops empty margins and rejects an empty or oversized tip. Requires an existing pixel selection."
        }
        _ => return None,
    })
}

pub(super) fn schema(id: &str) -> Option<Value> {
    Some(match id {
        "brush.get" => object(json!({}), &[]),
        "brush.presets.list" => object(
            json!({"full":{"type":"boolean","default":false,"description":"Include complete settings and bitmap payloads; false returns compact summaries."}}),
            &[],
        ),
        "brush.presets.delete" | "brush.presets.setCurrent" | "brush.defineFromSelection" => object(json!({"name":name()}), &["name"]),
        "brush.presets.save" => with_definitions(object(
            json!({
                "name":name(), "brush":patch(),
                "captureSize":{"type":"boolean","default":true,"description":"A future pick restores the captured size; false preserves the tool size."},
                "toolSettings":{"type":"boolean","default":true,"description":"A future pick restores mode, opacity, flow and pressure buttons; false preserves them."},
                "color":{"type":"boolean","default":false,"description":"Capture the current foreground colour so picking the preset restores it."}
            }),
            &["name"],
        )),
        "brush.presets.update" => with_definitions(object(
            json!({
                "name":{"type":["string","null"],"pattern":"\\S","description":"Existing preset name; omit/null to use the current preset identity."},
                "brush":patch()
            }),
            &[],
        )),
        "tools.setBrush" => {
            let mut schema = settings();
            if let Some(fields) = schema.get_mut("properties").and_then(Value::as_object_mut) {
                fields.insert(
                    "preset".into(),
                    described(name(), "Preset to pick before applying reset and patches. Section locks and smoothing are preserved by the pick."),
                );
                fields.insert("reset".into(), json!({"type":"boolean","default":false,"description":"Start from initial BrushSettings after picking any preset; clears current preset identity. Subsequent patches still apply."}));
                fields.insert("brush".into(), patch());
            }
            schema["examples"] = json!([{"size":12,"shapeDynamics":{"enabled":true,"size":{"control":"penPressure","minimum":0.2}},"scattering":{"enabled":true,"scatter":{"jitter":1.5}}}]);
            with_definitions(schema)
        }
        _ => return None,
    })
}

fn name() -> Value {
    json!({"type":"string","pattern":"\\S","description":"Non-whitespace name; outer whitespace is trimmed and lookup is case-insensitive."})
}

fn number(minimum: f32, maximum: f32, description: &str) -> Value {
    json!({"type":"number","minimum":minimum,"maximum":maximum,"description":description})
}

fn fraction(description: &str) -> Value {
    number(0.0, 1.0, description)
}

fn flag(description: &str) -> Value {
    json!({"type":"boolean","description":description})
}

fn section(properties: Value, description: &str) -> Value {
    described(object(properties, &[]), description)
}

fn reference(name: &str, description: &str) -> Value {
    json!({"$ref":format!("#/$defs/{name}"),"description":description})
}

fn dynamic(description: &str) -> Value {
    reference("BrushDynamic", description)
}

pub(super) fn patch() -> Value {
    reference(
        "BrushSettings",
        "Partial deep merge over the current/preset brush. Omitted fields at every object level remain unchanged; arrays and enum variants replace rather than append. No defaults are inserted. See command description for colour, erase, seed and scalar override precedence.",
    )
}

pub(super) fn with_definitions(mut schema: Value) -> Value {
    schema["$defs"] = json!({
        "BrushSettings":settings(),
        "BrushDynamic":dynamic_schema(1.0),
        "BrushScatterDynamic":dynamic_schema(photocraft_paint::MAX_SCATTER),
        "BrushCurve":{
            "type":"array","minItems":2,"maxItems":photocraft_paint::curve::ResponseCurve::MAX_POINTS,
            "items":{"type":"array","items":{"type":"number","minimum":0,"maximum":1},"minItems":2,"maxItems":2},
            "description":"Monotone response points [input,output] in 0–1, in increasing input order with non-decreasing outputs. Replaces the entire curve. Runtime sanitizes, sorts and caps points; fewer than two usable points restores a linear response."
        },
        "BrushTip":{
            "oneOf":[{"const":"round"},object(json!({"sampled":reference("BrushGrayTile","Embedded grayscale tip.")}), &["sampled"]),object(json!({"stored":reference("BrushStoredTile","Owner-managed bitmap reference; may not resolve on another device or in a browser.")}), &["stored"])],
            "description":"Externally tagged tip: round, {sampled:{width,height,data}} or {stored:{key,width,height,preview}}. A changed variant replaces the old tip and requires a complete new payload; same-variant patches preserve omitted fields. Prefer sampled for portable custom tips."
        },
        "BrushGrayTile":object(json!({
            "width":{"type":"integer","minimum":1,"maximum":u32::MAX},
            "height":{"type":"integer","minimum":1,"maximum":u32::MAX},
            "data":{"oneOf":[{"type":"string","contentEncoding":"base64"},{"type":"array","minItems":1,"items":{"type":"number","minimum":0,"maximum":1}}],"description":"Row-major coverage, 1=full paint: either exactly width×height floats in 0–1, or base64 of exactly width×height little-endian u16 samples (0–65535). Length/dimension agreement is checked at runtime. Use small tiles to avoid large command payloads."}
        }), &[]),
        "BrushStoredTile":object(json!({
            "key":{"type":"string","description":"Existing bitmap-store key, not a path or URL; discovered from current settings/presets."},
            "width":{"type":"integer","minimum":1,"maximum":u32::MAX},
            "height":{"type":"integer","minimum":1,"maximum":u32::MAX},
            "preview":reference("BrushGrayTile","Small preview bitmap; the full image is never sent in this reference.")
        }), &[]),
        "BrushPattern":{
            "oneOf":[
                object(json!({"procedural":object(json!({
                    "style":{"enum":[PatternStyle::Noise,PatternStyle::Canvas,PatternStyle::Paper,PatternStyle::Dots]},
                    "size":{"type":"integer","minimum":1,"maximum":u32::MAX,"description":"Pattern tile side in pixels; generated size is bounded by the renderer."},
                    "seed":{"type":"integer","minimum":0,"maximum":u32::MAX}
                }), &[])}), &["procedural"]),
                object(json!({"tile":reference("BrushGrayTile","Embedded repeating grayscale pattern.")}), &["tile"]),
                object(json!({"stored":reference("BrushStoredTile","Owner-managed pattern reference.")}), &["stored"])
            ],
            "description":"Externally tagged pattern. Supply the complete payload when changing variant. Existing same-variant patches may preserve omitted fields, but complete payloads are portable and unambiguous."
        }
    });
    schema
}

fn dynamic_schema(maximum: f32) -> Value {
    section(
        json!({
            "jitter":number(0.0,maximum,"Random variation per dab; scatter alone permits up to 10 (1000%)."),
            "control":{"enum":[Control::Off,Control::Fade,Control::PenPressure,Control::PenTilt,Control::StylusWheel,Control::Rotation,Control::InitialDirection,Control::Direction],"description":"Input controller. initialDirection/direction are angle controls; angle ignores curve. Pen controls need matching stroke sample data."},
            "fadeSteps":{"type":"integer","minimum":1,"maximum":u32::MAX,"description":"Spacing steps for fade; renderer treats zero as one."},
            "minimum":fraction("Floor for size, roundness, depth or other controlled value."),
            "curve":reference("BrushCurve","Response applied after the global pen curve; omit to retain the current curve.")
        }),
        "Partial dynamic parameter patch. Takes effect only when its owning section is enabled, except options-bar pressure shortcuts.",
    )
}

fn settings() -> Value {
    let mask_mode = json!({"enum":[MaskMode::Multiply,MaskMode::Subtract,MaskMode::Darken,MaskMode::Overlay,MaskMode::ColorDodge,MaskMode::ColorBurn,MaskMode::LinearBurn,MaskMode::HardMix,MaskMode::LinearHeight,MaskMode::Height]});
    let rgba = json!({"type":"array","items":{"type":"number","minimum":0,"maximum":1},"minItems":4,"maxItems":4,"description":"Exactly four straight RGBA components in display RGB, each 0–1; nested settings do not accept hex strings."});
    let mut schema = section(
        json!({
            "size":number(0.5,photocraft_paint::MAX_BRUSH_SIZE,"Primary diameter in document pixels. Resizing scales an enabled dual tip proportionally unless dualBrush.size is explicitly patched."),
            "hardness":fraction("Computed round-tip edge hardness: 0 soft, 1 hard; sampled tips retain their bitmap edge."),
            "spacing":number(0.01,10.0,"Dab distance as a fraction of primary diameter; 0.1=10%."),
            "spacingEnabled":flag("Use distance spacing; false uses pointer speed/timestamps, falling back to distance without timestamps."),
            "opacity":fraction("Whole-stroke coverage ceiling, not percent."),"flow":fraction("Per-dab coverage, not percent."),
            "pressureSize":flag("Options-bar shortcut for size controlled by pen pressure."),"pressureOpacity":flag("Options-bar shortcut for opacity controlled by pen pressure."),
            "color":rgba,"background":rgba,
            "erase":flag("Current eraser setting; paint.stroke resets it from the top-level erase flag."),
            "behind":flag("Paint under existing coverage; top-level blend/erase mode can override it."),
            "mode":{"enum":BlendMode::layer_modes().chain([BlendMode::PassThrough]).collect::<Vec<_>>(),"description":"Exact serialized BlendMode variant, e.g. Normal or ColorDodge; unlike top-level mode this is case-sensitive and has no clear/behind aliases."},
            "tip":reference("BrushTip","Primary tip shape or bitmap."),
            "angle":{"type":"number","description":"Counter-clockwise tip angle in degrees, not radians."},
            "roundness":fraction("Computed ellipse ratio, 1=circle."),
            "flipX":flag("Mirror tip horizontally."),"flipY":flag("Mirror tip vertically."),
            "aliased":flag("Disable tip anti-aliasing, as the Pencil does."),"square":flag("With aliased=true use a whole-pixel square tip; normally controlled by the Eraser Block mode.")
        }),
        "BrushSettings authoring patch. Every object is a partial deep merge: omission preserves the existing field. Initial values below are factory BrushSettings values used by reset, not patch defaults or necessarily the live session brush. Read brush.get for current values. Use canonical camelCase fields; typed nested enums are case-sensitive.",
    );
    let dynamics = json!({
        "shapeDynamics":section(json!({
            "enabled":flag("Enable this section; child changes alone do not enable it."),
            "size":dynamic("Size jitter/controller; minimum is minimum diameter."),
            "tiltScale":number(0.0,2.0,"Tilt scale fraction, 2=200%; shapes size controlled by pen tilt."),
            "angle":dynamic("Angle jitter/controller; jitter=1 spans ±180 degrees and curve is ignored."),
            "roundness":dynamic("Roundness jitter/controller; minimum is minimum roundness."),
            "flipXJitter":flag("Randomly mirror tip horizontally."),"flipYJitter":flag("Randomly mirror tip vertically."),
            "brushProjection":flag("Project the tip using pen tilt and barrel rotation.")
        }), "Shape Dynamics patch; enabled must be true for section controls to affect dabs."),
        "scattering":section(json!({
            "enabled":flag("Enable scattering."),"scatter":reference("BrushScatterDynamic","Scatter amount/controller; jitter=1 is 100%, up to 10 (1000%)."),
            "bothAxes":flag("Scatter radially instead of perpendicular to the stroke."),
            "count":{"type":"integer","minimum":1,"maximum":16,"description":"Dabs per spacing step; renderer clamps to 1–16."},
            "countJitter":dynamic("Count variation/controller, reducing towards one dab.")
        }), "Scattering patch. Disabled sections retain settings without affecting strokes."),
        "texture":section(json!({
            "enabled":flag("Enable texture masking."),"pattern":reference("BrushPattern","Repeating grayscale texture source."),
            "invert":flag("Invert the adjusted pattern levels."),"scale":{"type":"number","exclusiveMinimum":0,"description":"Pattern scale ratio, 1=100%; renderer floors it at 0.01."},
            "brightness":number(-1.0,1.0,"Normalized brightness, corresponding to ±150 grayscale levels."),
            "contrast":number(-1.0,1.0,"Normalized texture contrast, not a percentage."),
            "eachTip":flag("Apply texture per dab; required for depthJitter to vary between dabs."),"mode":mask_mode,
            "depth":fraction("Texture depth."),"depthJitter":dynamic("Depth controller/jitter, active only with eachTip=true; minimum is minimum depth.")
        }), "Texture patch; enabled controls rendering and eachTip controls depth dynamics.")
    });
    let secondary = json!({
        "dualBrush":section(json!({
            "enabled":flag("Enable the second masking tip."),"mode":mask_mode,"tip":reference("BrushTip","Secondary masking tip."),
            "size":number(f32::MIN_POSITIVE,photocraft_paint::MAX_BRUSH_SIZE,"Secondary diameter in document pixels; subpixel values preserve the primary/dual ratio."),
            "hardness":fraction("Computed secondary tip hardness."),"roundness":fraction("Secondary ellipse ratio."),
            "angle":{"type":"number","description":"Secondary tip angle in degrees."},"spacing":number(0.01,10.0,"Secondary spacing as a fraction of its diameter."),
            "scatter":number(0.0,photocraft_paint::MAX_SCATTER,"Secondary scatter amount; 1=100%, maximum 10=1000%."),
            "bothAxes":flag("Scatter the second tip on both axes."),"count":{"type":"integer","minimum":1,"maximum":16},"flip":flag("Flip the secondary tip.")
        }), "Dual Brush patch. Explicit size overrides proportional scaling during a primary size edit."),
        "colorDynamics":section(json!({
            "enabled":flag("Enable colour variation."),"perTip":flag("Vary every dab; false chooses one variation per stroke."),
            "fgBg":dynamic("Mix foreground/background using jitter or controller."),
            "hueJitter":fraction("Fraction of ±half the hue wheel."),"saturationJitter":fraction("Saturation variation."),"brightnessJitter":fraction("Brightness variation."),
            "purity":number(-1.0,1.0,"-1 desaturates, 1 saturates.")
        }), "Color Dynamics patch; stroke foreground/background come from command/tool colours."),
        "transfer":section(json!({
            "enabled":flag("Enable transfer dynamics."),"opacity":dynamic("Per-dab opacity ceiling variation."),"flow":dynamic("Per-dab flow variation."),
            "wetness":dynamic("Mixer Brush wetness variation only."),"mix":dynamic("Mixer Brush mix variation only.")
        }), "Transfer patch; wetness/mix apply only to paint.mixerBrush."),
        "pose":section(json!({
            "enabled":flag("Enable fixed stylus-component overrides."),"tiltX":number(-90.0,90.0,"Fixed tilt X in degrees."),"tiltY":number(-90.0,90.0,"Fixed tilt Y in degrees."),
            "rotation":number(0.0,360.0,"Fixed barrel rotation in degrees."),"pressure":fraction("Fixed pressure."),
            "overrideTiltX":flag("Replace sample tiltX when pose is enabled."),"overrideTiltY":flag("Replace sample tiltY when pose is enabled."),
            "overrideTilt":flag("Legacy combined alias; applies to each axis not explicitly supplied by this patch."),
            "overrideRotation":flag("Replace sample rotation when pose is enabled."),"overridePressure":flag("Replace sample pressure when pose is enabled.")
        }), "Brush Pose patch. Values take effect only with enabled=true and the corresponding override flag.")
    });
    let options = json!({
        "noise":flag("Add grain to soft tip areas."),"wetEdges":flag("Lighten interiors and darken dab rims."),"buildUp":flag("Deposit dabs over elapsed time while the pointer lingers; supply point timestamps."),
        "buildUpRate":number(0.1,1000.0,"Airbrush buildup dabs per second; renderer clamps to this interval."),
        "smoothing":section(json!({
            "enabled":flag("Enable smoothing without discarding its settings."),"amount":fraction("Smoothing amount, not percent."),
            "pulledString":flag("Lazy mouse: pull a string of amount×100 document pixels taut before painting."),
            "catchUp":flag("Catch up while the pointer pauses; uses timestamps."),"catchUpOnEnd":flag("Finish at the last pointer position."),
            "adjustForZoom":flag("Divide the string length by the top-level view zoom.")
        }), "Stroke-smoothing patch; unlike other sections the initial enabled value is true, with zero amount."),
        "protectTexture":flag("Preserve texture pattern and scale when picking another textured preset."),
        "locks":section(json!({
            "shapeDynamics":flag("Preserve Shape Dynamics when picking presets."),"scattering":flag("Preserve Scattering when picking presets."),
            "texture":flag("Preserve Texture when picking presets."),"dualBrush":flag("Preserve Dual Brush when picking presets."),
            "colorDynamics":flag("Preserve Color Dynamics when picking presets."),"transfer":flag("Preserve Transfer when picking presets."),
            "pose":flag("Preserve Brush Pose when picking presets."),"noise":flag("Preserve Noise when picking presets."),"wetEdges":flag("Preserve Wet Edges when picking presets."),
            "buildUp":flag("Preserve buildup and rate when picking presets."),"smoothing":flag("Smoothing is already preserved by preset picks."),"protectTexture":flag("Preserve texture-protection setting when picking presets.")
        }), "Preset section locks, not document layer locks; do not prevent an explicit settings patch."),
        "mixer":section(json!({
            "wet":fraction("Mixer wetness fraction; top-level paint.mixerBrush wet instead uses 0–100 percent."),"load":fraction("Mixer reservoir load fraction."),
            "mix":fraction("Mixer colour mix fraction."),"flow":fraction("Mixer per-dab flow fraction."),
            "sampleAllLayers":flag("Sample visible layers while mixing."),"loadAfterStroke":flag("Reload with stroke colour after each stroke."),"cleanAfterStroke":flag("Clean the reservoir after each stroke.")
        }), "Mixer settings patch, used by paint.mixerBrush; numeric fields are fractions 0–1, unlike its top-level percent overrides."),
        "seed":{"type":"integer","minimum":0,"maximum":u64::MAX,"description":"Current jitter seed. Stroke commands replace it with top-level seed, or derive it from points."}
    });
    if let Some(fields) = schema.get_mut("properties").and_then(Value::as_object_mut) {
        for group in [dynamics, secondary, options] {
            if let Some(group) = group.as_object() {
                fields.extend(group.clone());
            }
        }
    }
    if let Ok(initial) = serde_json::to_value(BrushSettings::default()) {
        schema["x-photocraft-initial-default"] = initial;
    }
    schema
}

#[cfg(test)]
mod tests;
