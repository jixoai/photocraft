use serde_json::{Value, json};

use super::{described, object};

pub(super) fn layer() -> Value {
    described(
        json!({"anyOf":[
            {"type":"integer","minimum":0,"maximum":u64::MAX},
            {"type":"string"},
            object(json!({"name":{"type":"string"}}), &["name"]),
            object(json!({"background":{"const":true}}), &["background"]),
            object(json!({"index":{"type":"integer","minimum":0,"maximum":u64::MAX}}), &["index"]),
            object(json!({"ordinal":{"enum":["Frnt","Back","Trgt","Frwr","Bckw"]}}), &["ordinal"]),
            {"type":"null"}
        ]}),
        "Layer selector: prefer the stable numeric ID from layer.list. A string or {name} finds the topmost matching name (case-insensitive fallback). {background:true} finds the real Background; {index} uses recorded bottom-to-top stack indices, including group-end entries; {ordinal} selects front/back/current/next/previous. Omit or use null for the command's active/selected layer target.",
    )
}

pub(super) fn tuple(count: usize, description: &str) -> Value {
    described(json!({"type":"array","items":{"type":"number"},"minItems":count,"maxItems":count}), description)
}

pub(super) fn point() -> Value {
    described(
        json!({"anyOf":[
            tuple(2, "[x,y] in document pixels."),
            object(json!({"x":{"type":"number"},"y":{"type":"number"}}), &["x","y"])
        ]}),
        "Document-space position [x,y] or {x,y}; origin at the canvas top-left, x increases rightwards and y downwards. Not viewport/CSS coordinates.",
    )
}

pub(super) fn rgb() -> Value {
    described(
        json!({"anyOf":[
            {"type":"string","pattern":"^#?([0-9a-fA-F]{6}|[0-9a-fA-F]{8})$"},
            {"type":"array","items":{"type":"number","minimum":0,"maximum":1},"minItems":3,"maxItems":4}
        ]}),
        "RGB colour: #RRGGBB, #RRGGBBAA, or [r,g,b,alpha?] with components in 0–1 (alpha defaults to 1). This input form is RGB, not native CMYK/Lab ink values.",
    )
}

pub(super) fn color() -> Value {
    let mut schema = rgb();
    if let Some(alternatives) = schema.get_mut("anyOf").and_then(Value::as_array_mut) {
        alternatives.push(object(
            json!({
                "mode":{"enum":["Bitmap","Grayscale","Indexed","Rgb","Cmyk","Lab","Multichannel","Duotone"]},
                "c":tuple(4, "Four native colour components; preserve the values returned by shape.info, including unused components."),
                "alpha":{"type":"number","minimum":0,"maximum":1}
            }),
            &["mode", "c", "alpha"],
        ));
    }
    schema["description"] = json!(
        "RGB hex/array, or stored Color {mode,c:[four native components],alpha}. Preserve stored colours when editing CMYK/Lab shapes; do not reinterpret their components as RGB."
    );
    schema
}

pub(super) fn target() -> Value {
    described(
        json!({"anyOf":[
            {"enum":["pixels","mask","filterMask","quickMask"]},
            object(json!({"channel":{"type":"integer","minimum":0}}), &["channel"]),
            object(json!({"color":{"type":"integer","minimum":0,"maximum":63}}), &["color"]),
            object(json!({"colors":{"type":"array","items":{"type":"integer","minimum":0,"maximum":63},"minItems":1,"maxItems":64}}), &["colors"])
        ]}),
        "Pixel-edit target. Omit to follow the Channels panel. pixels edits layer pixels; mask/filterMask edit the layer/smart-filter mask; quickMask edits Quick Mask; {channel:i} selects a saved alpha/spot channel by zero-based index; {color:i} or {colors:[i,...]} restrict native colour components. The requested target must exist and be supported by the command.",
    )
}

pub(super) fn blend(clear: bool, pass_through: bool) -> Value {
    let mut names: Vec<_> = photocraft_color::BlendMode::layer_modes().map(|mode| mode.label()).collect();
    if clear {
        names.push("clear");
    }
    if pass_through {
        names.push(photocraft_color::BlendMode::PassThrough.label());
    }
    described(
        json!({"type":"string","examples":names}),
        "Blend mode name from the examples. Names are case-insensitive and accept spaces/hyphens/underscores; use a canonical example to avoid ambiguity.",
    )
}

pub(super) fn operation() -> Value {
    described(
        json!({"type":"string","enum":["combine","add","union","subtract","minus","intersect","exclude","xor","join"]}),
        "Boolean combination with existing path components. combine/add/union add coverage; subtract/minus remove it; intersect keeps overlap; exclude/xor toggles overlap; join joins components.",
    )
}

pub(super) fn choices(names: &[&str], description: &str) -> Value {
    let alternatives: Vec<String> = names
        .iter()
        .map(|name| {
            let mut pattern = String::new();
            for character in name.chars() {
                if character.is_ascii_alphabetic() {
                    pattern.push('[');
                    pattern.push(character.to_ascii_lowercase());
                    pattern.push(character.to_ascii_uppercase());
                    pattern.push(']');
                } else {
                    if ".+*?()|[]{}^$\\".contains(character) {
                        pattern.push('\\');
                    }
                    pattern.push(character);
                }
            }
            pattern
        })
        .collect();
    described(json!({"type":"string","pattern":format!("^(?:{})$",alternatives.join("|")),"examples":names}), description)
}

pub(super) fn path() -> Value {
    let knot = object(
        json!({
            "anchor":point(),"in":point(),"out":point(),"inCtrl":point(),"outCtrl":point(),"in_ctrl":point(),"out_ctrl":point(),
            "smooth":{"type":"boolean","default":false,"description":"Whether the knot is marked as smooth; handles still specify the exact curve."}
        }),
        &["anchor"],
    );
    described(
        object(
            json!({
                "subpaths":{"type":"array","items":object(json!({
                    "closed":{"type":"boolean","default":true},
                    "op":operation(),
                    "knots":{"type":"array","items":{"anyOf":[tuple(2,"Corner knot [x,y]; handles coincide with its anchor."),knot]}}
                }), &["knots"])},
                "fillRule":{"type":"string","enum":["nonzero","evenodd","even-odd","even_odd"],"default":"nonzero"},
                "fill_rule":{"type":"string","enum":["nonzero","evenodd","even-odd","even_odd"],"description":"Stored-path alias of fillRule; fillRule wins when both are present."},
                "inverted":{"type":"boolean","default":false}
            }),
            &["subpaths"],
        ),
        "Vector geometry in document pixels. Each subpath contains corner [x,y] knots or Bézier knots {anchor,in,out,smooth}; handles are absolute coordinates, not offsets. Omitted handles coincide with the anchor. Use closed=false for an open curve. Geometry/tessellation budgets are checked by the engine.",
    )
}
