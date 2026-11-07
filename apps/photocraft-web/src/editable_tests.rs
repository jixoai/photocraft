use serde_json::{Value, json};

use crate::agent::{self, Call};
use photocraft_ui_egui::{ControlRequest, PhotocraftApp, Services, control::Outcome};

fn execute(app: &mut PhotocraftApp, command: &str, params: Value) -> Value {
    let Call::Control { method, params } = agent::parse("photocraft_execute", json!({"command":command,"params":params})).unwrap() else {
        panic!("expected shared command control");
    };
    let (request, _) = ControlRequest::new(method, params);
    let Outcome::Done(reply) = photocraft_ui_egui::control::handle(app, &egui::Context::default(), &request) else {
        panic!("expected synchronous command");
    };
    assert_eq!(reply["ok"], true, "{command}: {reply}");
    reply["result"].clone()
}

fn png(app: &PhotocraftApp) -> Vec<u8> {
    photocraft_io::export(&app.session.active().unwrap().doc, "check.png", &Default::default()).unwrap().bytes
}

#[test]
fn explicit_path_patch_uses_the_shared_web_control_and_undo_history() {
    let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), Services::default());
    execute(&mut app, "file.new", json!({"width":32,"height":24}));
    let subject = execute(&mut app, "shape.create", json!({"kind":"ellipse","rect":[2,2,12,10],"fill":"#247c98"}));
    let before = png(&app);
    let changed = execute(
        &mut app,
        "shape.edit",
        json!({
            "layer":subject["layer"],"kind":"path",
            "path":{"subpaths":[{"closed":true,"knots":[[3,3],[16,4],[8,18]]}]}
        }),
    );
    assert_eq!(changed["kind"], "path");
    assert!(changed["live"].is_null());
    let after = png(&app);
    assert_ne!(after, before);
    execute(&mut app, "edit.undo", json!({}));
    assert_eq!(png(&app), before);
    execute(&mut app, "edit.redo", json!({}));
    assert_eq!(png(&app), after);
}

#[test]
fn object_patch_preserves_neighbors_history_and_editability_at_all_depths() {
    for depth in [8, 16, 32] {
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), Services::default());
        execute(&mut app, "file.new", json!({"width":64,"height":48,"depth":depth}));
        let background = execute(&mut app, "shape.create", json!({"kind":"rect","rect":[2,2,60,44],"fill":"#e6d8ba","name":"Backdrop"}))["layer"].clone();
        let object = execute(&mut app, "shape.create", json!({"kind":"path","path":{"subpaths":[{"closed":true,"knots":[{"anchor":[9,25],"out":[9,5]},{"anchor":[28,8],"in":[20,4],"out":[39,12]},{"anchor":[30,29],"in":[36,25]}]}]},"fill":"#247c98","name":"Subject"}))["layer"].clone();
        let neighbor =
            execute(&mut app, "shape.create", json!({"kind":"star","rect":[39,20,15,16],"sides":5,"fill":"#f2b94c","name":"Accent"}))["layer"].clone();
        let stable_background = execute(&mut app, "shape.info", json!({"layer":background}));
        let stable_neighbor = execute(&mut app, "shape.info", json!({"layer":neighbor}));
        let original = execute(&mut app, "shape.info", json!({"layer":object}));
        let before = png(&app);
        execute(&mut app, "shape.edit", json!({"layer":object,"fill":"#bc4964","move":[3,2]}));
        let edited = execute(&mut app, "shape.info", json!({"layer":object}));
        let after = png(&app);
        assert_ne!(original, edited);
        assert_ne!(before, after);
        assert_eq!(execute(&mut app, "shape.info", json!({"layer":background})), stable_background);
        assert_eq!(execute(&mut app, "shape.info", json!({"layer":neighbor})), stable_neighbor);
        execute(&mut app, "edit.undo", json!({}));
        assert_eq!(execute(&mut app, "shape.info", json!({"layer":object})), original);
        assert_eq!(png(&app), before);
        execute(&mut app, "edit.redo", json!({}));
        assert_eq!(execute(&mut app, "shape.info", json!({"layer":object})), edited);
        assert_eq!(png(&app), after);
        let encoded = photocraft_io::export(&app.session.active().unwrap().doc, "objects.pcraft", &Default::default()).unwrap();
        let loaded = photocraft_io::import("objects.pcraft", &encoded.bytes).unwrap().document;
        assert_eq!(loaded.depth.bits(), depth);
        for id in [&background, &object, &neighbor] {
            assert!(matches!(&loaded.layer(photocraft_doc::LayerId(id.as_u64().unwrap())).unwrap().content, photocraft_doc::LayerContent::Shape(_)));
        }
        app.session.add_document(loaded, None);
        assert_eq!(png(&app), after);
        assert_eq!(execute(&mut app, "shape.info", json!({"layer":object})), edited);
        execute(&mut app, "shape.edit", json!({"layer":object,"name":"Reopened editable subject"}));
        assert_eq!(execute(&mut app, "shape.info", json!({"layer":neighbor})), stable_neighbor);
    }
}
