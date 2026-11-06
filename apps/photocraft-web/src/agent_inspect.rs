use photocraft_engine::Session;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Query {
    #[serde(default = "full")]
    pub scope: String,
    #[serde(default)]
    pub offset: usize,
    #[serde(default = "page_size")]
    pub limit: usize,
}

fn full() -> String {
    "full".into()
}
fn page_size() -> usize {
    50
}

pub fn inspect(session: &Session, query: &Query) -> Result<Value, String> {
    if !(1..=200).contains(&query.limit) {
        return Err("Inspection page size must be 1–200".into());
    }
    match query.scope.as_str() {
        "session" => Ok(json!({"scope":"session","session":photocraft_engine::inspect::session(session)})),
        "jobs" => Ok(json!({"scope":"jobs","running":session.jobs()})),
        "document" => Ok(json!({"scope":"document","document":session.active().map(|state| json!({
            "id":state.doc.id.0,"name":state.doc.name,"width":state.doc.size.width,"height":state.doc.size.height,
            "mode":format!("{:?}",state.doc.mode),"depth":state.doc.depth.bits(),"revision":state.revision,
            "dirty":state.is_dirty(),"activeLayer":state.active_layer.map(|id|id.0),"topLevelLayers":state.doc.layers.len(),
            "canUndo":state.history.can_undo(),"canRedo":state.history.can_redo()
        }))})),
        "layers" => layers(session, query),
        "history" => {
            let state = session.active().ok_or("No active document")?;
            let entries = state.history.entries();
            let total = entries.len();
            let entries: Vec<_> =
                entries.into_iter().enumerate().skip(query.offset).take(query.limit).map(|(index, label)| json!({"index":index,"label":label})).collect();
            Ok(
                json!({"scope":"history","documentId":state.doc.id.0,"revision":state.revision,"entries":entries,"total":total,"offset":query.offset,"nextOffset":next(query,total),"canUndo":state.history.can_undo(),"canRedo":state.history.can_redo()}),
            )
        }
        _ => Err("Use full UI inspection or an advertised inspection scope".into()),
    }
}

fn next(query: &Query, total: usize) -> Option<usize> {
    let next = query.offset.saturating_add(query.limit);
    (next < total).then_some(next)
}

fn layers(session: &Session, query: &Query) -> Result<Value, String> {
    let state = session.active().ok_or("No active document")?;
    let mut stack = vec![(None, 0usize, state.doc.layers.iter().rev())];
    let mut total = 0usize;
    let mut entries = Vec::new();
    while let Some((parent, depth, iterator)) = stack.last_mut() {
        let Some(layer) = iterator.next() else {
            stack.pop();
            continue;
        };
        let (parent, depth) = (*parent, *depth);
        if total >= query.offset && entries.len() < query.limit {
            entries.push(json!({"id":layer.id.0,"name":layer.name,"kind":layer.content.kind_name(),"parent":parent,"depth":depth,"visible":layer.visible,"opacity":layer.opacity}));
        }
        total = total.saturating_add(1);
        if let Some(children) = layer.children() {
            if depth >= 256 {
                return Err("Layer hierarchy exceeds inspection depth limit".into());
            }
            stack.push((Some(layer.id.0), depth.saturating_add(1), children.iter().rev()));
        }
    }
    Ok(
        json!({"scope":"layers","documentId":state.doc.id.0,"revision":state.revision,"layers":entries,"total":total,"offset":query.offset,"nextOffset":next(query,total)}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query(scope: &str, offset: usize, limit: usize) -> Query {
        Query { scope: scope.into(), offset, limit }
    }

    #[test]
    fn metadata_scopes_page_without_ui_payload_or_document_mutation() {
        let mut app = photocraft_ui_egui::PhotocraftApp::new(Session::new(), Default::default());
        app.run("file.new", json!({"width":32,"height":24})).unwrap();
        for index in 0..7 {
            app.run("shape.create", json!({"kind":"ellipse","rect":[index,3,4,5],"fill":"#35789c","name":format!("Object {index}")})).unwrap();
        }
        let before = app.session.active().unwrap().doc.clone();
        let journal = app.session.journal.len();
        let mut ids = std::collections::BTreeSet::new();
        let mut offset = 0;
        loop {
            let page = inspect(&app.session, &query("layers", offset, 3)).unwrap();
            assert_eq!(page["total"], 8);
            assert!(page["layers"].as_array().unwrap().len() <= 3);
            for layer in page["layers"].as_array().unwrap() {
                assert!(ids.insert(layer["id"].as_u64().unwrap()));
                assert!(layer.get("bounds").is_none());
                assert!(layer.get("path").is_none());
            }
            let Some(next) = page["nextOffset"].as_u64() else { break };
            offset = next as usize;
        }
        assert_eq!(ids.len(), 8);
        let summary = inspect(&app.session, &query("document", 0, 50)).unwrap();
        assert_eq!(summary["document"]["topLevelLayers"], 8);
        assert!(summary["document"].get("layers").is_none());
        let (request, _) = photocraft_ui_egui::ControlRequest::new("ui.inspect", json!({}));
        let photocraft_ui_egui::control::Outcome::Done(full) = photocraft_ui_egui::control::handle(&mut app, &egui::Context::default(), &request) else {
            panic!("synchronous inspection")
        };
        let (compact_bytes, full_bytes) = (summary.to_string().len(), full["result"].to_string().len());
        println!("compact document bytes={compact_bytes}; full UI bytes={full_bytes}");
        assert!(compact_bytes.saturating_mul(4) < full_bytes);
        assert_eq!(inspect(&app.session, &query("jobs", 0, 50)).unwrap()["running"], json!([]));
        assert_eq!(inspect(&app.session, &query("history", 1, 2)).unwrap()["entries"].as_array().unwrap().len(), 2);
        assert_eq!(inspect(&app.session, &query("session", 0, 50)).unwrap()["session"]["documents"].as_array().unwrap().len(), 1);
        assert!(inspect(&app.session, &query("layers", usize::MAX, 200)).unwrap()["layers"].as_array().unwrap().is_empty());
        assert_eq!(app.session.active().unwrap().doc, before);
        assert_eq!(app.session.journal.len(), journal);
    }

    #[test]
    fn inspect_contracts_preserve_full_default_and_reject_ambiguous_pagination() {
        use crate::agent::{Call, parse};
        assert!(matches!(parse("photocraft_inspect",json!({})).unwrap(),Call::Control {method,..} if method=="ui.inspect"));
        for args in [
            json!({"scope":"unknown"}),
            json!({"limit":1}),
            json!({"scope":"document","offset":1}),
            json!({"scope":"layers","limit":0}),
            json!({"scope":"layers","limit":201}),
            json!({"scope":"history","offset":-1}),
        ] {
            assert!(parse("photocraft_inspect", args).is_err());
        }
        let Call::Inspect(scoped) = parse("photocraft_inspect", json!({"scope":"layers","limit":2})).unwrap() else { panic!("scoped inspect") };
        assert_eq!(scoped.limit, 2);
        assert_eq!(scoped.scope, "layers");
        let empty = Session::new();
        assert_eq!(inspect(&empty, &query("document", 0, 50)).unwrap()["document"], Value::Null);
        assert!(inspect(&empty, &query("layers", 0, 50)).is_err());
    }

    #[test]
    fn layer_metadata_preserves_display_order_parents_and_rejects_excess_depth() {
        let mut source = Session::new();
        source.execute("file.new", json!({"width":2,"height":2})).unwrap();
        let mut document = (*source.active().unwrap().doc).clone();
        let lower = photocraft_doc::Layer::group("Lower", Vec::new());
        let upper = photocraft_doc::Layer::group("Upper", Vec::new());
        let group = photocraft_doc::Layer::group("Parent", vec![lower, upper]);
        let parent_id = group.id.0;
        document.layers = vec![group];
        let mut session = Session::new();
        session.add_document(document.clone(), None);
        let page = inspect(&session, &query("layers", 0, 50)).unwrap();
        let rows = page["layers"].as_array().unwrap();
        assert_eq!(rows.iter().map(|row| row["name"].as_str().unwrap()).collect::<Vec<_>>(), ["Parent", "Upper", "Lower"]);
        assert_eq!(rows[1]["parent"], parent_id);
        assert_eq!(rows[1]["depth"], 1);
        let mut nested = photocraft_doc::Layer::group("Leaf", Vec::new());
        for _ in 0..258 {
            nested = photocraft_doc::Layer::group("Nested", vec![nested]);
        }
        document.layers = vec![nested];
        let mut deep = Session::new();
        deep.add_document(document, None);
        assert!(inspect(&deep, &query("layers", 0, 1)).unwrap_err().contains("depth limit"));
    }
}
