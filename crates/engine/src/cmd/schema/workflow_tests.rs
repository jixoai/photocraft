use serde_json::{Value, json};

fn execute(session: &mut crate::Session, command: &str, params: Value) -> Value {
    let schema = crate::commands::input_schema(command).unwrap();
    assert_eq!(schema["x-photocraft-coverage"], "complete", "{command}");
    jsonschema::validate(schema, &params).unwrap_or_else(|error| panic!("{command}: {params}: {error}"));
    session.execute(command, params).unwrap()
}

#[test]
fn fill_duplicate_and_resize_contracts_execute_at_every_document_depth() {
    for depth in [8, 16, 32] {
        let mut session = crate::Session::new();
        execute(&mut session, "file.new", json!({"width":24,"height":16,"depth":depth,"background":"transparent","profile":"none"}));
        execute(&mut session, "select.rect", json!({"x":2,"y":2,"width":8,"height":8}));
        execute(&mut session, "edit.fill", json!({"contents":"CoLoR","color":"#336699","opacity":50}));
        let active = session.active().unwrap();
        let surface = active.doc.layer(active.active_layer.unwrap()).unwrap().surface().unwrap();
        assert!((surface.rgba(4, 4)[3] - 0.5).abs() < 0.01);
        assert_eq!(surface.rgba(0, 0)[3], 0.0);
        session.execute("select.deselect", json!({})).unwrap();
        let copied = execute(&mut session, "layer.duplicate", json!({"name":"Editable copy","inPlace":true}));
        assert_eq!(session.active().unwrap().doc.layer(photocraft_doc::LayerId(copied["layer"].as_u64().unwrap())).unwrap().name, "Editable copy");
        let resized = execute(&mut session, "image.imageSize", json!({"width":12,"resample":"nearest"}));
        assert_eq!(resized, json!({"width":12,"height":8}));
        execute(&mut session, "image.canvasSize", json!({"width":4,"height":-2,"relative":true,"anchor":"topLeft"}));
        assert_eq!(session.active().unwrap().doc.size, photocraft_geom::Size::new(16, 6));
        execute(&mut session, "image.canvasSize", json!({"width":50,"units":"percent"}));
        assert_eq!(session.active().unwrap().doc.size, photocraft_geom::Size::new(8, 6));
    }
}

#[test]
fn image_size_resolution_percent_and_point_units_match_precedence() {
    let mut session = crate::Session::new();
    execute(&mut session, "file.new", json!({"width":24,"height":16,"profile":"none"}));
    let resolution = execute(&mut session, "image.imageSize", json!({"width":100,"height":100,"resolution":144,"resample":"none"}));
    assert_eq!(resolution, json!({"width":24,"height":16}));
    let points = execute(&mut session, "image.imageSize", json!({"width":12,"units":"pt","resample":"nearest"}));
    assert_eq!(points, json!({"width":24,"height":16}));
    let percent = execute(&mut session, "image.imageSize", json!({"width":12,"percent":50,"resample":"nearest"}));
    assert_eq!(percent, json!({"width":12,"height":8}));
    let print_size = execute(&mut session, "image.imageSize", json!({"resolution":288,"keepPrintSize":true,"resample":"nearest"}));
    assert_eq!(print_size, json!({"width":24,"height":16}));
}

#[test]
fn duplicate_destination_contract_creates_new_document_without_mutating_source() {
    let mut session = crate::Session::new();
    execute(&mut session, "file.new", json!({"width":16,"height":12,"profile":"none"}));
    let original = session.active().unwrap().doc.clone();
    let result = execute(&mut session, "layer.duplicate", json!({"document":"new","documentName":"Copy destination","name":"Copy layer"}));
    assert_eq!(result["document"], 1);
    assert_eq!(session.documents().len(), 2);
    assert_eq!(session.active().unwrap().doc.name, "Copy destination");
    assert_eq!(session.active().unwrap().doc.size, original.size);
    assert_eq!(session.documents().first().unwrap().doc.layers.len(), original.layers.len());
    assert_eq!(session.active().unwrap().doc.layer(photocraft_doc::LayerId(result["layer"].as_u64().unwrap())).unwrap().name, "Copy layer");
    assert!(session.execute("layer.duplicate", json!({"document":0})).is_err());
}

#[test]
fn duplicate_contract_preserves_upstream_link_group_behavior_and_undo() {
    let mut session = crate::Session::new();
    execute(&mut session, "file.new", json!({"width":16,"height":12,"profile":"none","background":"transparent"}));
    let first = session.active().unwrap().active_layer.unwrap();
    let second = execute(&mut session, "layer.new.layer", json!({"name":"Linked partner"}))["layer"].as_u64().unwrap();
    execute(&mut session, "layer.select", json!({"layer":first.0}));
    execute(&mut session, "layer.select", json!({"layer":second,"mode":"add"}));
    session.execute("layer.linkLayers", json!({"linked":true})).unwrap();
    let group = session.active().unwrap().doc.layer(first).unwrap().link_group;
    assert!(group.is_some());
    let copied = execute(&mut session, "layer.duplicate", json!({"layer":first.0,"name":"Linked copy"}));
    let copied = photocraft_doc::LayerId(copied["layer"].as_u64().unwrap());
    assert_eq!(session.active().unwrap().doc.layer(copied).unwrap().link_group, group);
    session.execute("edit.undo", json!({})).unwrap();
    assert!(session.active().unwrap().doc.layer(copied).is_none());
    session.execute("edit.redo", json!({})).unwrap();
    assert_eq!(session.active().unwrap().doc.layer(copied).unwrap().link_group, group);
    assert!(crate::commands::input_schema("layer.duplicate").unwrap()["description"].as_str().unwrap().contains("link group"));
}

#[test]
fn audited_workflow_schemas_reject_invalid_types_units_and_dependencies() {
    let mut session = crate::Session::new();
    execute(&mut session, "file.new", json!({"width":16,"height":12,"profile":"none"}));
    for (command, params) in [
        ("edit.fill", json!({"contents":"unknown"})),
        ("edit.fill", json!({"opacity":"half"})),
        ("edit.fill", json!({"preserveTransparency":1})),
        ("edit.fill", json!({"contents":"history","state":0,"snapshot":0})),
        ("image.imageSize", json!({"units":"cm"})),
        ("image.imageSize", json!({"percent":0})),
        ("image.canvasSize", json!({"units":"percent","relative":true})),
        ("image.canvasSize", json!({"units":"percent","width":0})),
        ("image.canvasSize", json!({"anchor":"topleft"})),
        ("layer.duplicate", json!({"document":false})),
    ] {
        assert!(jsonschema::validate(crate::commands::input_schema(command).unwrap(), &params).is_err(), "{command}: {params}");
        assert!(session.execute(command, params).is_err(), "{command}");
    }
}
