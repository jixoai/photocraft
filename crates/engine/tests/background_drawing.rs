use photocraft_engine::{Session, jobs::Started};
use serde_json::{Value, json};

fn session(depth: u32) -> Session {
    let mut session = Session::new();
    session.execute("file.new", json!({"width":129,"height":257,"depth":depth})).unwrap();
    session.execute("layer.new.layer", json!({"name":"drawing"})).unwrap();
    session.execute("path.set", json!({"path":{"subpaths":[{"closed":true,"knots":[[12,20],[100,20],[100,200],[12,200]]}]}})).unwrap();
    session
}

fn pixels(session: &Session) -> Vec<u8> {
    photocraft_io::export(&session.active().unwrap().doc, "drawing.png", &Default::default()).unwrap().bytes
}

#[test]
fn background_drawing_matches_inline_at_all_depths_and_undoes_once() {
    for depth in [8, 16, 32] {
        for (command, params) in [
            ("paint.gradient", json!({"from":[0,0],"to":[128,256],"colors":["#fae8bc","#81b3a9"],"dither":true})),
            ("path.fill", json!({"color":"#efb867","feather":1})),
            ("path.stroke", json!({"color":"#25494b","size":3,"hardness":1})),
        ] {
            let mut inline = session(depth);
            let mut background = session(depth);
            let before = pixels(&background);
            inline.execute(command, params.clone()).unwrap();
            let started = background.start(command, params).unwrap();
            let Started::Job(job) = started else { panic!("{command} must start a job") };
            assert_eq!(pixels(&background), before);
            background.wait_job(job).unwrap();
            assert_eq!(pixels(&background), pixels(&inline), "{command}, depth={depth}");
            background.execute("edit.undo", Value::Null).unwrap();
            assert_eq!(pixels(&background), before);
        }
    }
}

#[test]
fn cancelled_drawing_discards_its_result() {
    let mut session = session(16);
    let before = pixels(&session);
    let Started::Job(job) = session.start("path.fill", json!({"color":"#efb867"})).unwrap() else { panic!("expected background job") };
    session.cancel_job(job);
    session.join_cancelled_jobs();
    session.poll_jobs();
    assert_eq!(pixels(&session), before);
}

#[test]
fn malformed_drawing_parameters_fail_without_editing() {
    let mut session = session(8);
    let before = pixels(&session);
    for command in ["path.fill", "path.stroke", "paint.gradient"] {
        assert!(session.start(command, json!([])).is_err());
    }
    assert_eq!(pixels(&session), before);
}
