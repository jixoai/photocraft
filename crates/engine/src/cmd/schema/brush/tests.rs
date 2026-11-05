use super::*;

use photocraft_paint::{GrayTile, Pattern, StoredTile, TipShape};
use std::sync::Arc;

fn contract(command: &str) -> &'static Value {
    crate::commands::input_schema(command).unwrap()
}

fn execute(session: &mut crate::Session, command: &str, params: Value) -> Value {
    jsonschema::validate(contract(command), &params).unwrap_or_else(|error| panic!("{command}: {params}: {error}"));
    session.execute(command, params).unwrap()
}

fn documented_fields(root: &Value, schema: &Value, value: &Value, path: &str) {
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        let target = root.pointer(reference.strip_prefix('#').unwrap()).unwrap();
        documented_fields(root, target, value, path);
        return;
    }
    if let Some(fields) = value.as_object() {
        if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
            for (key, child) in fields {
                let child_schema = properties.get(key).unwrap_or_else(|| panic!("undocumented serialized field: {path}/{key}"));
                documented_fields(root, child_schema, child, &format!("{path}/{key}"));
            }
        } else if let Some(variants) = schema.get("oneOf").and_then(Value::as_array) {
            let variant = variants
                .iter()
                .find(|variant| fields.keys().all(|key| variant.get("properties").and_then(|properties| properties.get(key)).is_some()))
                .unwrap();
            documented_fields(root, variant, value, path);
        } else {
            panic!("untyped serialized object: {path}");
        }
    }
}

#[test]
fn every_serialized_brush_field_and_bitmap_variant_is_documented() {
    let schema = contract("tools.setBrush");
    let initial = &schema["$defs"]["BrushSettings"]["x-photocraft-initial-default"];
    assert_eq!(*initial, serde_json::to_value(BrushSettings::default()).unwrap());
    assert!(schema["$defs"]["BrushSettings"]["properties"]["size"].get("default").is_none());
    let tile = GrayTile::from_f32(2, 2, &[1.0, 0.5, 0.0, 1.0]);
    let stored = StoredTile { key: "schema-test-only".into(), width: 2, height: 2, preview: Arc::new(tile.clone()), full: None };
    let variants = [
        (TipShape::Round, Pattern::default()),
        (TipShape::Sampled(tile.clone()), Pattern::Tile(tile.clone())),
        (TipShape::Stored(stored.clone()), Pattern::Stored(stored.clone())),
        (TipShape::Stored(StoredTile { full: Some(Arc::new(tile)), ..stored.clone() }), Pattern::Stored(stored)),
    ];
    for (tip, pattern) in variants {
        let mut brush = BrushSettings { tip, square: true, ..Default::default() };
        brush.dual_brush.tip = brush.tip.clone();
        brush.texture.pattern = pattern;
        let params = serde_json::to_value(brush).unwrap();
        jsonschema::validate(schema, &params).unwrap();
        documented_fields(schema, &schema["$defs"]["BrushSettings"], &params, "brush");
    }
}

#[test]
fn deep_patches_preserve_omissions_resize_dual_tips_and_apply_pose_aliases() {
    let mut session = crate::Session::new();
    execute(
        &mut session,
        "tools.setBrush",
        json!({"size":40,"dualBrush":{"enabled":true,"size":20},"shapeDynamics":{"enabled":true,"size":{"control":"penPressure","minimum":0.3}}}),
    );
    execute(&mut session, "tools.setBrush", json!({"brush":{"size":80,"shapeDynamics":{"size":{"jitter":0.25}}},"size":60}));
    assert_eq!(session.tools.brush.size, 60.0);
    assert_eq!(session.tools.brush.dual_brush.size, 30.0);
    assert!(session.tools.brush.shape_dynamics.enabled);
    assert_eq!(session.tools.brush.shape_dynamics.size.control, Control::PenPressure);
    assert_eq!(session.tools.brush.shape_dynamics.size.minimum, 0.3);
    execute(&mut session, "tools.setBrush", json!({"size":30,"dualBrush":{"size":7},"pose":{"enabled":true,"overrideTilt":true,"overrideTiltX":false}}));
    assert_eq!(session.tools.brush.dual_brush.size, 7.0);
    assert!(!session.tools.brush.pose.override_tilt_x);
    assert!(session.tools.brush.pose.override_tilt_y);
    execute(&mut session, "tools.setBrush", json!({"scattering":{"scatter":{"jitter":2.0}}}));
    assert!(!session.tools.brush.scattering.enabled);
    execute(&mut session, "tools.setBrush", json!({"reset":true,"hardness":0.5}));
    assert_eq!(session.tools.brush.size, BrushSettings::default().size);
    assert!(!session.tools.brush.dual_brush.enabled);
    assert_eq!(session.tools.brush.hardness, 0.5);
}

#[test]
fn nested_types_and_validated_brush_limits_match_real_errors_without_mutation() {
    let mut session = crate::Session::new();
    let before = session.tools.brush.clone();
    for params in [
        json!({"size":5001}),
        json!({"dualBrush":{"enabled":true,"size":5001}}),
        json!({"scattering":{"enabled":true,"scatter":{"jitter":10.1}}}),
        json!({"dualBrush":{"enabled":true,"scatter":10.1}}),
        json!({"shapeDynamics":{"size":{"control":"pressure"}}}),
        json!({"pose":{"overrideTiltX":null}}),
        json!({"mode":"multiply"}),
        json!({"texture":{"mode":"Multiply"}}),
        json!({"color":"#ff0000"}),
        json!({"shapeDynamics":{"size":{"curve":[[0,0,0],[1,1,1]]}}}),
    ] {
        assert!(jsonschema::validate(contract("tools.setBrush"), &params).is_err(), "{params}");
        assert!(session.execute("tools.setBrush", params.clone()).is_err(), "{params}");
        assert_eq!(session.tools.brush, before);
    }
    for params in [json!({"scattering":{"amount":2}}), json!({"mixer":{"wet":50}}), json!({"shapeDynamics":{"size":{"curve":[[0,0]]}}})] {
        assert!(jsonschema::validate(contract("tools.setBrush"), &params).is_err(), "bad authoring input: {params}");
    }
}

#[test]
fn sampled_tiles_curves_and_variant_patches_validate_and_execute() {
    let mut session = crate::Session::new();
    execute(
        &mut session,
        "tools.setBrush",
        json!({
            "tip":{"sampled":{"width":2,"height":2,"data":[1,0.5,0,1]}},
            "texture":{"enabled":true,"pattern":{"tile":{"width":1,"height":1,"data":"//8="}}},
            "shapeDynamics":{"enabled":true,"size":{"control":"penPressure","curve":[[0,0],[0.5,0.2],[1,1]]}}
        }),
    );
    execute(&mut session, "tools.setBrush", json!({"tip":{"sampled":{"data":[0,1,1,0]}}}));
    assert_eq!(session.tools.brush.tip.bitmap().unwrap().width, 2);
    execute(&mut session, "tools.setBrush", json!({"texture":{"pattern":{"procedural":{"style":"canvas","size":16,"seed":4}}}}));
    execute(&mut session, "tools.setBrush", json!({"texture":{"pattern":{"procedural":{"seed":9}}},"tip":"round"}));
    assert_eq!(session.tools.brush.texture.pattern, Pattern::Procedural { style: PatternStyle::Canvas, size: 16, seed: 9 });
    for params in [
        json!({"tip":{"sampled":{"width":3,"height":2,"data":[1]}}}),
        json!({"tip":{"sampled":{"width":1,"height":1,"data":"invalid base64"}}}),
        json!({"tip":{"sampled":{"data":[1]}}}),
    ] {
        jsonschema::validate(contract("tools.setBrush"), &params).unwrap();
        assert!(session.execute("tools.setBrush", params).is_err());
    }
}

#[test]
fn stroke_resolution_precedence_matches_the_advertised_contract() {
    let mut session = crate::Session::new();
    execute(
        &mut session,
        "tools.setBrush",
        json!({"size":40,"smoothing":{"enabled":false,"amount":0.2},"locks":{"shapeDynamics":true},"shapeDynamics":{"enabled":true,"size":{"jitter":0.3}}}),
    );
    execute(&mut session, "tools.setColors", json!({"foreground":"#336699","background":"#ffffff"}));
    execute(&mut session, "brush.presets.save", json!({"name":"Contract Test","brush":{"size":8,"shapeDynamics":{"enabled":false}},"captureSize":true}));
    let params = json!({"points":[[2,4],[16,4]],"preset":"Contract Test","brush":{"size":12,"color":[1,0,0,1],"background":[0,0,0,1],"erase":true,"seed":99},"size":6,"smoothing":0.5,"seed":7});
    jsonschema::validate(contract("paint.stroke"), &params).unwrap();
    let resolved = crate::brush_cmds::resolve_brush(&session, &params, "paint.stroke").unwrap();
    assert_eq!(resolved.size, 6.0);
    assert_eq!(resolved.color, session.tools.foreground);
    assert_eq!(resolved.background, session.tools.background);
    assert!(!resolved.erase);
    assert_eq!(resolved.seed, 7);
    assert!(resolved.smoothing.enabled);
    assert_eq!(resolved.smoothing.amount, 0.5);
    assert!(resolved.shape_dynamics.enabled);
    assert_eq!(resolved.shape_dynamics.size.jitter, 0.3);
    execute(&mut session, "tools.setBrush", json!({"preset":"Contract Test","reset":true,"size":5}));
    assert_eq!(session.tools.brush.size, 5.0);
    assert!(session.tools.current_preset.is_none());
}

#[test]
fn preset_contracts_distinguish_identity_capture_and_picking() {
    let mut session = crate::Session::new();
    execute(&mut session, "tools.setBrush", json!({"size":20}));
    execute(&mut session, "brush.presets.save", json!({"name":"Contract Preset","brush":{"size":9},"captureSize":false,"toolSettings":false}));
    assert_eq!(session.tools.brush.size, 20.0);
    let listed = execute(&mut session, "brush.presets.list", json!({"full":true}));
    assert!(listed["presets"].as_array().unwrap().iter().any(|preset| preset["name"] == "Contract Preset" && preset["brush"]["size"].as_f64() == Some(9.0)));
    execute(&mut session, "tools.setBrush", json!({"preset":"contract preset"}));
    assert_eq!(session.tools.brush.size, 20.0);
    execute(&mut session, "brush.presets.update", json!({"brush":{"hardness":0.3}}));
    assert_eq!(session.tools.brush.hardness, 1.0);
    execute(&mut session, "brush.presets.setCurrent", json!({"name":"Contract Preset"}));
    assert_eq!(session.tools.brush.hardness, 1.0);
    execute(&mut session, "brush.presets.delete", json!({"name":"Contract Preset"}));
    assert!(session.execute("brush.presets.setCurrent", json!({"name":"Contract Preset"})).is_err());
    for command in ["brush.presets.save", "brush.presets.delete", "brush.presets.setCurrent", "brush.defineFromSelection"] {
        assert!(jsonschema::validate(contract(command), &json!({"name":" \t "})).is_err());
        assert!(session.execute(command, json!({"name":" \t "})).is_err());
    }
}

#[test]
fn filled_stroke_contract_rejects_eraser_and_short_polygons() {
    let mut session = crate::Session::new();
    session.execute("file.new", json!({"width":16,"height":16,"background":"transparent"})).unwrap();
    for params in [
        json!({"points":[[1,1],[8,1]],"fill":true}),
        json!({"points":[[1,1],[8,1],[8,8]],"fill":true,"erase":true}),
        json!({"points":[[1,1],[8,1],[8,8]],"fill":true,"mode":"CLEAR"}),
    ] {
        assert!(jsonschema::validate(contract("paint.stroke"), &params).is_err());
        assert!(session.execute("paint.stroke", params).is_err());
    }
    execute(&mut session, "paint.stroke", json!({"points":[[1,1],[12,1],[12,12]],"fill":true,"brush":{"hardness":1},"color":"#2f718c"}));
}
