use std::sync::{Arc, Mutex, PoisonError, mpsc};

use photocraft_engine::{
    EngineError,
    jobs::{JobId, Started},
};
use photocraft_ui_egui::PhotocraftApp;
use serde_json::json;
use web_sys::IdbDatabase;

use crate::recovery_bridge::{Bridge, Operation};

type Loaded = (Operation, Result<Vec<u8>, String>);
type Encoded = Arc<Mutex<Option<Vec<u8>>>>;

pub struct Recovery {
    pub bridge: Bridge,
    pub database: Option<IdbDatabase>,
    sender: mpsc::Sender<Loaded>,
    receiver: mpsc::Receiver<Loaded>,
    job: Option<(Operation, JobId, Encoded)>,
    next_checkpoint: f64,
}

impl Recovery {
    pub async fn load() -> Self {
        let (sender, receiver) = mpsc::channel();
        let owner = crate::webmcp::page_identity().unwrap_or_default();
        let (database, bridge) = match crate::indexed_recovery::load().await {
            Ok((database, entries)) => match Bridge::new(owner, entries) {
                Ok(bridge) => (Some(database), bridge),
                Err(error) => {
                    database.close();
                    let bridge = Bridge::default();
                    bridge.unavailable(error);
                    (None, bridge)
                }
            },
            Err(error) => {
                let bridge = Bridge::default();
                bridge.unavailable(error);
                (None, bridge)
            }
        };
        Self { bridge, database, sender, receiver, job: None, next_checkpoint: 0.0 }
    }

    pub fn schedule(&mut self, app: &mut PhotocraftApp, ctx: &egui::Context) {
        let now =
            web_sys::window().and_then(|window| window.performance()).map_or_else(|| ctx.input(|input| input.time), |performance| performance.now() / 1000.0);
        if now >= self.next_checkpoint {
            self.next_checkpoint = now + 30.0;
            photocraft_ui_egui::prefs_ui::autosave_now(app);
        }
        ctx.request_repaint_after(std::time::Duration::from_secs(30));
    }

    pub fn tick(&mut self, app: &mut PhotocraftApp, ctx: &egui::Context) {
        self.bridge.reconcile(&app.session);
        if let Some((operation, job, output)) = self.job.take() {
            if let Some(info) = app.session.jobs_with_recent().into_iter().find(|info| info.id == job && info.state != "running") {
                let result =
                    info.result.ok_or_else(|| EngineError::Other(info.error.unwrap_or_else(|| "Recovery job cancelled; stored snapshot retained".into())));
                self.started(operation, result.map(Started::Done), output, ctx);
            } else {
                if app.session.job(job).is_some() {
                    self.job = Some((operation, job, output));
                    ctx.request_repaint_after(std::time::Duration::from_millis(50));
                } else {
                    self.bridge
                        .finish(operation, Err("Recovery job receipt unavailable; inspect documents before restoring again. Stored snapshot retained".into()));
                }
            }
        }
        if let Ok((operation, result)) = self.receiver.try_recv() {
            match result {
                Ok(bytes) => self.restore(app, operation, bytes, ctx),
                Err(error) => self.bridge.finish(operation, Err(error)),
            }
        }
        let Some(operation) = self.bridge.begin() else { return };
        match &operation {
            Operation::Save { entry, document } => {
                let mut entry = entry.clone();
                let document = document.clone();
                let output: Encoded = Arc::default();
                let target = output.clone();
                let started = app.session.start_job(
                    "web.checkpoint",
                    json!({}),
                    "Saving recovery snapshot",
                    false,
                    move |ctx| {
                        ctx.check()?;
                        crate::recovery_bridge::encode(&mut entry, &document).map_err(EngineError::Other)
                    },
                    move |_, bytes| {
                        *target.lock().unwrap_or_else(PoisonError::into_inner) = Some(bytes);
                        Ok(json!({}))
                    },
                );
                self.started(operation, started, output, ctx);
            }
            Operation::Remove(_) => self.store(operation, None, ctx),
            Operation::Restore(key) => {
                let Some(database) = self.database.clone() else {
                    self.bridge.finish(operation, Err("Browser recovery storage unavailable; keep this tab open and export your work".into()));
                    return;
                };
                let key = key.clone();
                let sender = self.sender.clone();
                let ctx = ctx.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let result = crate::indexed_recovery::read(&database, &key).await;
                    let _ = sender.send((operation, result));
                    ctx.request_repaint();
                });
            }
        }
    }

    fn started(&mut self, operation: Operation, started: Result<Started, EngineError>, output: Encoded, ctx: &egui::Context) {
        match started {
            Ok(Started::Job(job)) => self.job = Some((operation, job, output)),
            Ok(Started::Done(result)) => match operation {
                Operation::Save { mut entry, document } => {
                    let bytes = output.lock().unwrap_or_else(PoisonError::into_inner).take();
                    if let Some(bytes) = bytes {
                        entry.bytes = bytes.len();
                        self.store(Operation::Save { entry, document }, Some(bytes), ctx);
                    } else {
                        self.bridge.finish(Operation::Save { entry, document }, Err("Checkpoint worker returned no data".into()));
                    }
                }
                operation => self.bridge.finish(operation, Ok(result["documentId"].as_u64())),
            },
            Err(error) => self.bridge.finish(operation, Err(error.to_string())),
        }
        ctx.request_repaint();
    }

    fn restore(&mut self, app: &mut PhotocraftApp, operation: Operation, bytes: Vec<u8>, ctx: &egui::Context) {
        let name = self.bridge.name(operation.key());
        let started = app.session.start_job(
            "web.recover",
            json!({}),
            "Restoring document copy",
            false,
            move |ctx| {
                ctx.check()?;
                let mut document = photocraft_io::import("checkpoint.pcraft", &bytes).map_err(|error| EngineError::Other(error.to_string()))?.document;
                if let Some(name) = name {
                    document.name = name;
                }
                Ok(document)
            },
            crate::recovery_bridge::restore_document,
        );
        self.started(operation, started, Arc::default(), ctx);
    }

    fn store(&self, operation: Operation, bytes: Option<Vec<u8>>, ctx: &egui::Context) {
        let Some(database) = self.database.clone() else {
            self.bridge.finish(operation, Err("Recovery storage unavailable; export your document before leaving".into()));
            return;
        };
        let bridge = self.bridge.clone();
        let ctx = ctx.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let snapshot = match (&operation, bytes.as_deref()) {
                (Operation::Save { entry, .. }, Some(bytes)) => Some((entry, bytes)),
                _ => None,
            };
            let result = crate::indexed_recovery::write(&database, operation.key(), snapshot).await;
            bridge.finish(operation, result.map(|_| None));
            ctx.request_repaint();
        });
    }
}
