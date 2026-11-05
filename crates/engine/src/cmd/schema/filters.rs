use photocraft_algo::{GalleryFilter, GalleryParamKind};
use serde_json::{Value, json};

use super::{described, notation, object, types};

pub(super) fn description(id: &str) -> Option<String> {
    if let Some(filter) = id.strip_prefix("filter.gallery.").and_then(GalleryFilter::from_key) {
        return Some(format!(
            "Apply the {} gallery effect ({}) to the target layer within the pixel selection. On a smart object, records an editable smart filter; otherwise changes pixels in one undo step. Omitted parameters use effect defaults; colours come from the toolbox unless supplied.",
            filter.name(),
            filter.category()
        ));
    }
    crate::filters::input_schema(id).map(|_| {
        format!("Apply {id} to the explicit or active layer, respecting its pixel selection, channel/mask target and locks. Smart objects retain an editable smart filter; a raster layer's pixels are changed in one undo step. Numeric units match the filter dialog, not normalized fractions unless stated.")
    })
}

pub(super) fn schema(id: &str, documentation: &str) -> Option<Value> {
    let legacy = notation::schema(documentation);
    if let Some(mut schema) = crate::filters::input_schema(id) {
        if let Some(fields) = schema.get_mut("properties").and_then(Value::as_object_mut) {
            for (name, property) in fields.iter_mut() {
                if let Some(default) = legacy.get("properties").and_then(|fields| fields.get(name)).and_then(|property| property.get("default")) {
                    property["default"] = default.clone();
                }
            }
            fields.insert(
                "layer".into(),
                described(
                    json!({"anyOf":[{"type":"integer","minimum":0,"maximum":u64::MAX},{"type":"string"}]}),
                    "Stable layer ID or layer name; omit for the active layer. These strictly validated filters do not accept recorded object selectors.",
                ),
            );
            fields.insert("target".into(), types::target());
        }
        return Some(schema);
    }
    let filter = id.strip_prefix("filter.gallery.").and_then(GalleryFilter::from_key)?;
    let mut fields = serde_json::Map::new();
    for parameter in filter.params() {
        let property = match parameter.kind {
            GalleryParamKind::Range { min, max, default } => json!({"type":"number","minimum":min,"maximum":max,"default":default}),
            GalleryParamKind::Choice(names) => {
                json!({"anyOf":[types::choices(&names,"Case-insensitive choice name; use a canonical example."),{"type":"number"}],"default":names.first(),"description":"Choice name (case-insensitive) or numeric choice index, clamped to the available options; the first named option is the default."})
            }
            GalleryParamKind::Bool(default) => json!({"type":"boolean","default":default}),
        };
        let property = if property.get("description").is_some() {
            property
        } else {
            described(property, &format!("{}: gallery dialog parameter; defaults/ranges match the effect metadata.", parameter.key))
        };
        fields.insert(parameter.key.into(), property);
    }
    for key in ["foreground", "background", "glowColor"] {
        fields.insert(key.into(), described(
            json!({"anyOf":[{"type":"string","pattern":"^#[0-9a-fA-F]{6}$"},{"type":"array","items":{"type":"number","minimum":0,"maximum":1},"minItems":3,"maxItems":4}]}),
            &format!("{key} colour override: #RRGGBB or RGB(A) components in 0–1. Omit to use the captured toolbox/effect colour."),
        ));
    }
    fields.insert("layer".into(), types::layer());
    fields.insert("target".into(), types::target());
    Some(object(Value::Object(fields), &[]))
}
