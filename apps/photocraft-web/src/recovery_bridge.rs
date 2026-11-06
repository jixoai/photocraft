use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use photocraft_doc::Document;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const MAX_DOCUMENTS: usize = 16;
pub const MAX_FILE_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_STORE_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_METADATA_BYTES: usize = 4096;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Entry {
    pub key: String,
    pub name: String,
    pub revision: u64,
    pub bytes: usize,
}

pub fn valid_key(key: &str) -> bool {
    !key.is_empty() && key.len() <= 128 && key.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

impl Entry {
    pub fn validate(&self) -> Result<(), String> {
        if !valid_key(&self.key) || self.name.len() > 1024 || self.bytes == 0 || self.bytes > MAX_FILE_BYTES {
            return Err("Invalid or oversized recovery metadata".into());
        }
        Ok(())
    }
}

#[derive(Clone)]
pub enum Operation {
    Save { entry: Entry, document: Arc<Document> },
    Remove(String),
    Restore(String),
}

impl Operation {
    pub fn key(&self) -> &str {
        match self {
            Self::Save { entry, .. } => &entry.key,
            Self::Remove(key) | Self::Restore(key) => key,
        }
    }
}

#[derive(Default)]
struct State {
    owner: String,
    entries: BTreeMap<String, Entry>,
    documents: BTreeMap<u64, String>,
    queued: VecDeque<Operation>,
    active: Option<String>,
    results: Vec<(u64, u64, Result<(), String>)>,
    error: Option<String>,
    restored: BTreeMap<String, u64>,
}

#[derive(Clone, Default)]
pub struct Bridge(Arc<Mutex<State>>);

impl Bridge {
    fn state(&self) -> MutexGuard<'_, State> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn new(owner: String, entries: Vec<Entry>) -> Result<Self, String> {
        if !valid_key(&owner) || entries.len() > MAX_DOCUMENTS {
            return Err("Invalid recovery owner or entry count".into());
        }
        let bridge = Self::default();
        {
            let mut state = bridge.state();
            state.owner = owner;
            for entry in entries {
                entry.validate()?;
                if state.entries.insert(entry.key.clone(), entry).is_some() {
                    return Err("Duplicate recovery key".into());
                }
            }
        }
        Ok(bridge)
    }

    pub fn unavailable(&self, error: String) {
        self.state().error = Some(error);
    }

    pub fn save(&self, document: &Arc<Document>, revision: u64) -> Result<String, String> {
        let mut state = self.state();
        if let Some(error) = &state.error {
            return Err(error.clone());
        }
        if !valid_key(&state.owner) {
            return Err("Recovery storage is unavailable".into());
        }
        let key = format!("{}-{}", state.owner, document.id.0);
        if state.active.as_deref() == Some(&key) || state.queued.iter().any(|op| op.key() == key) {
            return Err("A checkpoint for this document is still pending".into());
        }
        if state.entries.get(&key).is_some_and(|entry| entry.revision == revision) {
            state.results.push((document.id.0, revision, Ok(())));
            return Ok(key);
        }
        if state.queued.len() >= MAX_DOCUMENTS || (!state.documents.contains_key(&document.id.0) && state.documents.len() >= MAX_DOCUMENTS) {
            return Err("Too many pending recovery operations".into());
        }
        state.documents.insert(document.id.0, key.clone());
        let entry = Entry { key: key.clone(), name: document.name.chars().take(256).collect(), revision, bytes: 0 };
        state.queued.push_back(Operation::Save { entry, document: document.clone() });
        Ok(key)
    }

    pub fn discard_document(&self, id: u64) {
        let mut state = self.state();
        if let Some(key) = state.documents.remove(&id) {
            state.queued.retain(|operation| operation.key() != key);
            state.queued.push_back(Operation::Remove(key));
        }
    }

    pub fn request(&self, key: &str, restore: bool) -> Result<(), String> {
        let mut state = self.state();
        if !valid_key(key) || !state.entries.contains_key(key) {
            return Err("Recovery snapshot not found".into());
        }
        if state.active.as_deref() == Some(key) || state.queued.iter().any(|operation| operation.key() == key) {
            return Err("Recovery operation already pending".into());
        }
        if state.queued.len() >= MAX_DOCUMENTS {
            return Err("Too many pending recovery operations".into());
        }
        if restore && state.restored.contains_key(key) {
            return Err("Snapshot already restored in this page; inspect its document before restoring again".into());
        }
        state.queued.push_back(if restore { Operation::Restore(key.into()) } else { Operation::Remove(key.into()) });
        Ok(())
    }

    pub fn begin(&self) -> Option<Operation> {
        let mut state = self.state();
        if state.active.is_some() {
            return None;
        }
        let operation = state.queued.pop_front()?;
        state.active = Some(operation.key().into());
        Some(operation)
    }

    pub fn finish(&self, operation: Operation, result: Result<Option<u64>, String>) {
        let mut state = self.state();
        state.active = None;
        if let Err(error) = &result {
            state.error = Some(error.clone());
        }
        match operation {
            Operation::Save { entry, document } => {
                state.results.push((document.id.0, entry.revision, result.clone().map(|_| ())));
                if result.is_ok() {
                    state.entries.insert(entry.key.clone(), entry);
                }
            }
            Operation::Remove(key) if result.is_ok() => {
                state.entries.remove(&key);
                state.restored.remove(&key);
            }
            Operation::Restore(key) => {
                if let Ok(Some(document)) = result {
                    state.restored.insert(key, document);
                }
            }
            Operation::Remove(_) => {}
        }
    }

    pub fn results(&self) -> Vec<(u64, u64, Result<(), String>)> {
        std::mem::take(&mut self.state().results)
    }
    pub fn retry(&self) {
        self.state().error = None;
    }
    #[cfg(test)]
    pub fn entries(&self) -> Vec<Entry> {
        self.state().entries.values().cloned().collect()
    }
    pub fn recoverable(&self) -> Vec<Entry> {
        let state = self.state();
        state.entries.values().filter(|entry| !state.documents.values().any(|key| key == &entry.key)).cloned().collect()
    }
    pub fn error(&self) -> Option<String> {
        self.state().error.clone()
    }
    pub fn name(&self, key: &str) -> Option<String> {
        self.state().entries.get(key).map(|entry| entry.name.clone())
    }
    pub fn reconcile(&self, session: &photocraft_engine::Session) {
        let closed: Vec<_> =
            self.state().documents.keys().filter(|id| !session.documents().iter().any(|document| document.doc.id.0 == **id)).copied().collect();
        for id in closed {
            self.discard_document(id);
        }
    }
    pub fn control(&self, session: &photocraft_engine::Session, method: &str, params: &Value) -> Result<Value, String> {
        match method {
            "recovery.list" => {}
            "recovery.checkpoint" => {
                let state = session.active().ok_or("No active document")?;
                let key = self.save(&state.doc, state.revision)?;
                return Ok(json!({"key":key,"accepted":true,"completed":false,"poll":"recovery.list"}));
            }
            "recovery.restore" | "recovery.discard" => {
                let key = params["key"].as_str().ok_or("A recovery key is required")?;
                if method == "recovery.discard" && params["confirm"] != true {
                    return Err("Explicit confirm: true required to discard a snapshot".into());
                }
                self.request(key, method == "recovery.restore")?;
                return Ok(json!({"key":key,"accepted":true,"completed":false,"poll":"recovery.list"}));
            }
            "recovery.retry" => self.retry(),
            _ => return Err("Unknown recovery method".into()),
        }
        Ok(self.status())
    }
    pub fn status(&self) -> Value {
        let state = self.state();
        json!({"entries":state.entries.values().collect::<Vec<_>>(),"pending":state.active,"queued":state.queued.iter().map(Operation::key).collect::<Vec<_>>(),"restored":state.restored,"error":state.error,"maxDocuments":MAX_DOCUMENTS,"maxFileBytes":MAX_FILE_BYTES,"maxStoreBytes":MAX_STORE_BYTES,"maxMetadataBytes":MAX_METADATA_BYTES,"storage":"indexeddb","browserMayEvict":true})
    }
}

pub fn encode(entry: &mut Entry, document: &Document) -> Result<Vec<u8>, String> {
    let exported = photocraft_io::export(document, "checkpoint.pcraft", &Default::default()).map_err(|error| error.to_string())?;
    entry.bytes = exported.bytes.len();
    entry.validate()?;
    Ok(exported.bytes)
}

pub fn restore_document(session: &mut photocraft_engine::Session, mut document: Document) -> Result<Value, photocraft_engine::EngineError> {
    document.id = photocraft_doc::DocId::fresh();
    session.add_document(document, None);
    let state = session.active_mut().ok_or(photocraft_engine::EngineError::NoDocument)?;
    state.saved_revision = 0;
    Ok(json!({"documentId":state.doc.id.0}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restored_copy_syncs_editor_views_before_drawing_status_bar() {
        let mut app = photocraft_ui_egui::PhotocraftApp::new(photocraft_engine::Session::new(), Default::default());
        let original = document();
        restore_document(&mut app.session, (*original).clone()).unwrap();
        assert_ne!(app.session.active().unwrap().doc.id, original.id);
        assert!(app.session.active().unwrap().is_dirty());
        app.sync_views();
        let ctx = egui::Context::default();
        ctx.run_ui(Default::default(), |ui| {
            photocraft_ui_egui::chrome_ui::status_bar_pro(&mut app, ui);
        })
        .textures_delta
        .clear();
    }
    #[test]
    fn recovery_controls_are_explicit_bounded_and_report_admission_not_completion() {
        let mut session = photocraft_engine::Session::new();
        let bridge = Bridge::new("page-a".into(), Vec::new()).unwrap();
        assert!(bridge.control(&session, "recovery.checkpoint", &json!({})).is_err());
        session.execute("file.new", json!({"width":8,"height":8})).unwrap();
        let reply = bridge.control(&session, "recovery.checkpoint", &json!({})).unwrap();
        assert_eq!(reply["completed"], false);
        let key = reply["key"].as_str().unwrap();
        assert!(bridge.control(&session, "recovery.checkpoint", &json!({})).is_err());
        let mut operation = bridge.begin().unwrap();
        if let Operation::Save { entry, document } = &mut operation {
            encode(entry, document).unwrap();
        }
        bridge.finish(operation, Ok(None));
        assert_eq!(bridge.results().len(), 1);
        assert!(bridge.recoverable().is_empty());
        assert!(bridge.error().is_none());
        assert!(bridge.control(&session, "recovery.discard", &json!({"key":key})).is_err());
        assert!(bridge.control(&session, "recovery.discard", &json!({"key":key,"confirm":false})).is_err());
        bridge.reconcile(&session);
        assert!(bridge.begin().is_none());
        bridge.reconcile(&photocraft_engine::Session::new());
        let remove = bridge.begin().unwrap();
        bridge.finish(remove, Ok(None));
        assert_eq!(bridge.control(&session, "recovery.list", &json!({})).unwrap()["entries"], json!([]));
        bridge.unavailable("denied".into());
        assert_eq!(bridge.control(&session, "recovery.retry", &json!({})).unwrap()["error"], Value::Null);
        assert!(bridge.control(&session, "recovery.unknown", &json!({})).is_err());
        let entry = Entry { key: "foreign-1".into(), name: "copy".into(), revision: 1, bytes: 1 };
        let bridge = Bridge::new("page-a".into(), vec![entry]).unwrap();
        assert_eq!(bridge.recoverable().len(), 1);
        bridge.control(&session, "recovery.restore", &json!({"key":"foreign-1"})).unwrap();
        bridge.finish(bridge.begin().unwrap(), Ok(Some(42)));
        bridge.control(&session, "recovery.discard", &json!({"key":"foreign-1","confirm":true})).unwrap();
    }

    #[test]
    fn recovery_contracts_reject_incomplete_and_unsafe_keys_without_dispatch() {
        for params in [json!({}), json!({"key":"../secret"}), json!({"key":"ok","extra":true})] {
            let failure = crate::agent::parse("photocraft_control", json!({"method":"recovery.restore","params":params})).unwrap_err();
            assert_eq!(failure.reply()["errorInfo"]["execution"], "not_started");
        }
        assert!(crate::agent::parse("photocraft_control", json!({"method":"recovery.discard","params":{"key":"ok","confirm":false}})).is_err());
        assert!(crate::agent::parse("photocraft_control", json!({"method":"recovery.discard"})).is_err());
        assert!(crate::agent::parse("photocraft_control", json!({"method":"recovery.checkpoint","params":{"document":1}})).is_err());
        assert!(crate::agent::parse("photocraft_control", json!({"method":"recovery.list"})).is_ok());
    }
    fn document() -> Arc<Document> {
        let mut session = photocraft_engine::Session::new();
        session.execute("file.new", json!({"width":8,"height":8})).unwrap();
        session.active().unwrap().doc.clone()
    }

    #[test]
    fn failed_write_keeps_previous_snapshot_and_close_is_ordered_after_pending_write() {
        let bridge = Bridge::new("page-a".into(), Vec::new()).unwrap();
        let document = document();
        let key = bridge.save(&document, 1).unwrap();
        let mut first = bridge.begin().unwrap();
        if let Operation::Save { entry, document } = &mut first {
            encode(entry, document).unwrap();
        }
        assert!(bridge.entries().is_empty());
        bridge.finish(first, Ok(None));
        bridge.save(&document, 2).unwrap();
        let second = bridge.begin().unwrap();
        bridge.finish(second, Err("QuotaExceededError".into()));
        assert_eq!(bridge.entries()[0].revision, 1);
        assert!(bridge.save(&document, 3).is_err());
        bridge.retry();
        bridge.save(&document, 3).unwrap();
        let writing = bridge.begin().unwrap();
        bridge.discard_document(document.id.0);
        assert!(bridge.begin().is_none());
        bridge.finish(writing, Ok(None));
        let remove = bridge.begin().unwrap();
        assert_eq!(remove.key(), key);
        bridge.finish(remove, Ok(None));
        assert!(bridge.entries().is_empty());
    }

    #[test]
    fn foreign_restore_is_a_copy_and_auto_cleanup_only_removes_owned_entries() {
        let entry = Entry { key: "other-page-1".into(), name: "other".into(), revision: 5, bytes: 42 };
        let bridge = Bridge::new("new-page".into(), vec![entry]).unwrap();
        assert_eq!(bridge.name("other-page-1").as_deref(), Some("other"));
        bridge.request("other-page-1", true).unwrap();
        let restore = bridge.begin().unwrap();
        bridge.finish(restore, Ok(Some(123)));
        bridge.discard_document(123);
        assert!(bridge.begin().is_none());
        assert_eq!(bridge.entries().len(), 1);
        assert_eq!(bridge.status()["restored"]["other-page-1"], 123);
        assert!(bridge.request("other-page-1", true).is_err());
        assert!(!valid_key("../bad"));
        assert!(Bridge::new(String::new(), Vec::new()).is_err());
    }

    #[test]
    fn checkpoint_roundtrip_preserves_all_supported_depths() {
        for depth in [8, 16, 32] {
            let mut session = photocraft_engine::Session::new();
            session.execute("file.new", json!({"width":8,"height":8,"depth":depth})).unwrap();
            session.execute("shape.create", json!({"kind":"ellipse","rect":[1,1,4,4],"fill":"#2468ac","name":"editable"})).unwrap();
            let original = &session.active().unwrap().doc;
            let mut entry = Entry { key: "page-1".into(), name: original.name.clone(), revision: 3, bytes: 0 };
            let bytes = encode(&mut entry, original).unwrap();
            let loaded = photocraft_io::import("checkpoint.pcraft", &bytes).unwrap().document;
            assert_eq!(loaded.layers.len(), original.layers.len());
            assert_eq!(loaded.depth, original.depth);
            assert!(loaded.layers.iter().any(|layer| layer.name == "editable"));
            assert_eq!(entry.bytes, bytes.len());
        }
    }
}
