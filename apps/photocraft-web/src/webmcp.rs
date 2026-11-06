//! WebMCP bindings written in Rust. Supports the current document.modelContext API and the
//! earlier navigator.modelContext implementation without installing a counterfeit browser API.
//! Requests wake the editor and run one at a time between frames. Idle pages have no poll timer.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};

use base64::Engine as _;
use js_sys::{Function, Object, Promise, Reflect};
use photocraft_ui_egui::{ControlRequest, PhotocraftApp};
use serde_json::{Value, json};
use wasm_bindgen::{JsCast as _, JsValue, closure::Closure};

use crate::agent::{self, Call};

const MAX_PENDING: usize = 16;
const MAX_QUEUED_BYTES: usize = agent::MAX_IMPORT_JSON_BYTES;
const DEADLINE_MS: f64 = 60_000.0;

fn diagnostic(call: Option<&Call>) -> bool {
    match call {
        Some(Call::Commands(_)) => true,
        Some(Call::Control { method, .. }) => matches!(method.as_str(), "jobs.list" | "jobs.cancel" | "ui.inspect" | "engine.commands" | "document.inspect"),
        _ => false,
    }
}

fn file_work(call: &Call) -> bool {
    match call {
        Call::Export(_) => true,
        Call::Import { name, .. } => {
            !matches!(name.rsplit('.').next().unwrap_or("").to_ascii_lowercase().as_str(), "abr" | "grd" | "asl" | "aco" | "act" | "ase")
        }
        _ => false,
    }
}

fn start_file_job(app: &mut PhotocraftApp, call: Call) -> Result<photocraft_engine::jobs::Started, String> {
    use photocraft_engine::EngineError;
    match call {
        Call::Import { name, bytes } => {
            let options = photocraft_engine::jobs::import_options(app.session.prefs());
            let source_read_only = photocraft_io::affinity::is_affinity(&bytes);
            app.session.start_job("web.import", json!({"name":name}), "Open", false, move |ctx| {
                ctx.check()?;
                let result = photocraft_io::import_with_options(&name, &bytes, &options).map_err(|error| EngineError::Other(error.to_string()))?;
                ctx.check()?;
                Ok(result)
            }, move |session, imported| {
                let mut document = imported.document;
                photocraft_engine::file_cmds::raw_defaults(session, &imported.warnings, &mut document);
                session.add_document(document, None);
                if let Some(state) = session.active_mut() { state.source_read_only = source_read_only; }
                Ok(json!({"warnings":imported.warnings,"session":photocraft_engine::inspect::session(session),"document":session.active().map(photocraft_engine::inspect::document)}))
            }).map_err(|error| error.to_string())
        }
        Call::Export(options) => {
            let settings = agent::export_settings(app, &options)?.io_options();
            let document = app.session.active().ok_or("No active document")?.doc.clone();
            let name = options.name.clone();
            app.session.start_job("web.export", json!({"name":name}), "Export", false, move |ctx| {
                ctx.check()?;
                let exported = photocraft_io::export(&document, &name, &settings).map_err(|error| EngineError::Other(error.to_string()))?;
                ctx.check()?;
                if !options.download && exported.bytes.len() > agent::MAX_FILE_BYTES {
                    return Err(EngineError::Other("Export exceeds 64 MiB; use download: true".into()));
                }
                let encoded = (!options.download).then(|| base64::engine::general_purpose::STANDARD.encode(&exported.bytes));
                Ok((exported, options, encoded))
            }, |_, (exported, options, encoded)| {
                let mut result = json!({"name":options.name,"bytes":exported.bytes.len(),"mimeType":crate::web::mime_for(&options.name),"warnings":exported.warnings,"download":options.download});
                if options.download { crate::web::download(&options.name, &exported.bytes).map_err(EngineError::Other)?; }
                if let Some(encoded) = encoded { result["base64"] = json!(encoded); }
                Ok(result)
            }).map_err(|error| error.to_string())
        }
        _ => Err("Not a file operation".into()),
    }
}

#[derive(Default)]
struct Queue {
    pending: VecDeque<Pending>,
    /// Includes the active request until its reply settles.
    bytes: usize,
    count: usize,
}

struct Pending {
    call: Option<Call>,
    resolve: Function,
    reject: Function,
    signal: Option<web_sys::AbortSignal>,
    admitted: f64,
    bytes: usize,
}

struct Active {
    pending: Pending,
    receiver: Receiver<Value>,
    dispatch: photocraft_ui_egui::control::Dispatch,
}

struct Work {
    pending: Pending,
    job: photocraft_engine::jobs::JobId,
}

pub struct Bridge {
    queue: Rc<RefCell<Queue>>,
    sender: Sender<ControlRequest>,
    active: Option<Active>,
    auxiliary: Vec<Active>,
    work: Option<Work>,
}

fn now() -> f64 {
    web_sys::window().and_then(|w| w.performance()).map_or(0.0, |p| p.now())
}

fn error(message: impl AsRef<str>) -> JsValue {
    js_sys::Error::new(message.as_ref()).into()
}

fn set(object: &JsValue, key: &str, value: &JsValue) -> Result<(), JsValue> {
    if Reflect::set(object, &JsValue::from_str(key), value)? { Ok(()) } else { Err(error(format!("Could not set {key}"))) }
}

fn js(value: &Value) -> Result<JsValue, JsValue> {
    js_sys::JSON::parse(&value.to_string())
}

fn member(object: &JsValue, key: &str) -> Option<JsValue> {
    Reflect::get(object, &JsValue::from_str(key)).ok().filter(|v| !v.is_null() && !v.is_undefined())
}

fn enqueue(queue: &Rc<RefCell<Queue>>, ctx: &egui::Context, tool: &str, input: JsValue, options: JsValue) -> Promise {
    let prepared = (|| -> Result<_, JsValue> {
        let q = queue.try_borrow().map_err(|_| error("FDE2E Photocraft is busy"))?;
        if q.count >= MAX_PENDING {
            return Err(error("FDE2E Photocraft's request queue is full; wait for an outstanding call"));
        }
        drop(q);
        let signal = member(&options, "signal").and_then(|v| v.dyn_into::<web_sys::AbortSignal>().ok());
        if signal.as_ref().is_some_and(web_sys::AbortSignal::aborted) {
            return Err(error("Cancelled before dispatch; no changes applied"));
        }
        let input = if input.is_undefined() { Object::new().into() } else { input };
        let encoded = js_sys::JSON::stringify(&input)?;
        let limit = if tool == "photocraft_import" { agent::MAX_IMPORT_JSON_BYTES } else { agent::MAX_JSON_BYTES };
        // Check the JS string length before copying into Wasm, then the UTF-8 length too.
        if encoded.length() as usize > limit {
            return Err(error("Tool arguments exceed the browser request limit"));
        }
        let encoded = encoded.as_string().ok_or_else(|| error("Arguments must be JSON serializable"))?;
        let bytes = encoded.len();
        if bytes > limit {
            return Err(error("Tool arguments exceed the browser request limit"));
        }
        let value = serde_json::from_str(&encoded).map_err(|e| error(format!("Invalid JSON arguments: {e}")))?;
        let call = agent::parse(tool, value).map_err(error)?;
        Ok((call, signal, bytes))
    })();
    let (call, signal, bytes) = match prepared {
        Ok(p) => p,
        Err(e) => return Promise::reject(&e),
    };
    let mut call = Some(call);
    Promise::new(&mut |resolve, reject| {
        let Ok(mut q) = queue.try_borrow_mut() else {
            let _ = reject.call1(&JsValue::NULL, &error("FDE2E Photocraft is busy"));
            return;
        };
        if q.count >= MAX_PENDING || q.bytes.saturating_add(bytes) > MAX_QUEUED_BYTES {
            let _ = reject.call1(&JsValue::NULL, &error("FDE2E Photocraft's request queue is full; wait for an outstanding call"));
            return;
        }
        q.bytes = q.bytes.saturating_add(bytes);
        q.count = q.count.saturating_add(1);
        q.pending.push_back(Pending { call: call.take(), resolve, reject, signal: signal.clone(), admitted: now(), bytes });
        ctx.request_repaint();
    })
}

impl Bridge {
    pub fn new(ctx: &egui::Context) -> (Self, Receiver<ControlRequest>) {
        let (sender, receiver) = std::sync::mpsc::channel();
        let bridge = Self { queue: Rc::default(), sender, active: None, auxiliary: Vec::new(), work: None };
        if let Err(e) = bridge.install(ctx) {
            log::warn!("FDE2E Photocraft WebMCP setup: {e:?}");
        }
        (bridge, receiver)
    }

    fn install(&self, ctx: &egui::Context) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or_else(|| error("No browser window"))?;
        let definitions = agent::tools();
        let tools = js_sys::Array::new();
        for (definition, name) in definitions.iter().zip(agent::TOOL_NAMES) {
            let tool = js(definition)?;
            let queue = self.queue.clone();
            let wake = ctx.clone();
            let execute = Closure::<dyn FnMut(JsValue, JsValue) -> Promise>::new(move |input, options| enqueue(&queue, &wake, name, input, options));
            set(&tool, "execute", &execute.into_js_value())?;
            tools.push(&tool);
        }
        // Also provide an explicit same-page API for integrations in browsers that have not
        // implemented WebMCP. This is not a network endpoint or a modelContext polyfill.
        let api = Object::new();
        set(&api, "tools", &js(&json!(definitions))?)?;
        set(&api, "ready", &JsValue::TRUE)?;
        set(&api, "backgroundWorkers", &JsValue::from_bool(photocraft_engine::background::available()))?;
        let queue = self.queue.clone();
        let wake = ctx.clone();
        let call =
            Closure::<dyn FnMut(String, JsValue, JsValue) -> Promise>::new(move |name: String, input, options| enqueue(&queue, &wake, &name, input, options));
        set(&api, "callTool", &call.into_js_value())?;
        let context = window
            .document()
            .and_then(|d| member(d.as_ref(), "modelContext"))
            .map(|c| ("document.modelContext", c))
            .or_else(|| member(window.navigator().as_ref(), "modelContext").map(|c| ("navigator.modelContext", c)));
        let status = Object::new();
        set(&status, "registered", &JsValue::from_f64(0.0))?;
        set(&status, "supported", &JsValue::from_bool(context.is_some()))?;
        set(&api, "webmcp", &status)?;
        set(&window, "photocraft", &api)?;
        if let Some((kind, context)) = context {
            set(&status, "api", &JsValue::from_str(kind))?;
            wasm_bindgen_futures::spawn_local(async move {
                let result = register(&context, &tools, &status).await;
                if let Err(e) = result {
                    let message = member(&e, "message").and_then(|m| m.as_string()).unwrap_or_else(|| format!("{e:?}"));
                    let _ = set(&status, "error", &JsValue::from_str(&message));
                    log::warn!("FDE2E Photocraft WebMCP registration: {message}");
                }
            });
        }
        Ok(())
    }

    /// Start at most one tool per frame, preserving command order and letting the browser paint
    /// between calls. The control channel owns input/job/screenshot completion semantics.
    pub fn dispatch(&mut self, app: &mut PhotocraftApp, ctx: &egui::Context) {
        let busy = self.active.is_some() || self.work.is_some();
        let pending = self.queue.try_borrow_mut().ok().and_then(|mut queue| {
            if !busy {
                return queue.pending.pop_front();
            }
            let index = queue.pending.iter().position(|pending| diagnostic(pending.call.as_ref()))?;
            queue.pending.remove(index)
        });
        let Some(mut pending) = pending else { return };
        if pending.signal.as_ref().is_some_and(web_sys::AbortSignal::aborted) || now() - pending.admitted > DEADLINE_MS {
            self.finish(pending, Err("Cancelled or expired before dispatch; no changes applied".into()));
            return;
        }
        let Some(call) = pending.call.take() else {
            self.finish(pending, Err("Missing browser request".into()));
            return;
        };
        if photocraft_engine::background::available() && file_work(&call) {
            match start_file_job(app, call) {
                Ok(photocraft_engine::jobs::Started::Job(job)) => self.work = Some(Work { pending, job }),
                Ok(photocraft_engine::jobs::Started::Done(result)) => self.finish(pending, Ok(json!({"ok":true,"result":result}))),
                Err(error) => self.finish(pending, Err(error)),
            }
            return;
        }
        let result = match call {
            Call::Control {method,params} => {
                let (request,receiver) = ControlRequest::new(method,params);
                let dispatch = request.dispatch.clone();
                if self.sender.send(request).is_err() { self.finish(pending,Err("Editor control channel closed".into())); return; }
                let active = Active {pending,receiver,dispatch};
                if busy { self.auxiliary.push(active); } else { self.active = Some(active); }
                return;
            }
            Call::Commands(query) => Ok(agent::commands(app,&query)),
            Call::Import {name,bytes} => agent::import(app,&name,&bytes),
            Call::Export(options) => agent::export(app,&options).and_then(|(bytes,warnings)| {
                let mut result = json!({"name":options.name,"bytes":bytes.len(),"mimeType":crate::web::mime_for(&options.name),"warnings":warnings,"download":options.download});
                if options.download { crate::web::download(&options.name,&bytes)?; }
                else { result["base64"] = json!(base64::engine::general_purpose::STANDARD.encode(bytes)); }
                Ok(result)
            }),
        };
        self.finish(pending, result.map(|result| json!({"ok":true,"result":result})));
        ctx.request_repaint();
    }

    pub fn settle(&mut self, app: &mut PhotocraftApp, ctx: &egui::Context) {
        for active in std::mem::take(&mut self.auxiliary) {
            match active.receiver.try_recv() {
                Ok(value) => self.finish(active.pending, Ok(value)),
                Err(TryRecvError::Disconnected) => self.finish(active.pending, Err("Editor closed before replying".into())),
                Err(TryRecvError::Empty) => {
                    if active.pending.signal.as_ref().is_some_and(web_sys::AbortSignal::aborted) || now() - active.pending.admitted > DEADLINE_MS {
                        let reason = active.dispatch.give_up();
                        self.finish(active.pending, Err(reason));
                    } else {
                        self.auxiliary.push(active);
                    }
                }
            }
        }
        if let Some(work) = self.work.take() {
            let job = app.session.jobs_with_recent().into_iter().find(|job| job.id == work.job);
            if let Some(job) = job.filter(|job| job.state != "running") {
                match job.result {
                    Some(result) => self.finish(work.pending, Ok(json!({"ok":true,"result":result}))),
                    None => self.finish(work.pending, Err(job.error.unwrap_or_else(|| "Operation cancelled; no changes applied".into()))),
                }
            } else if work.pending.signal.as_ref().is_some_and(web_sys::AbortSignal::aborted) || now() - work.pending.admitted > DEADLINE_MS {
                app.session.cancel_job(work.job);
                self.finish(work.pending, Err("Operation cancelled; its result will not be applied".into()));
            } else {
                self.work = Some(work);
            }
        }
        if let Some(active) = self.active.take() {
            match active.receiver.try_recv() {
                Ok(value) => self.finish(active.pending, Ok(value)),
                Err(TryRecvError::Disconnected) => self.finish(active.pending, Err("Editor closed before replying".into())),
                Err(TryRecvError::Empty) => {
                    if active.pending.signal.as_ref().is_some_and(web_sys::AbortSignal::aborted) || now() - active.pending.admitted > DEADLINE_MS {
                        // A dispatched operation can still finish; never tell the caller it is
                        // safe to retry. Queued requests are atomically abandoned before editing.
                        let reason = active.dispatch.give_up();
                        self.finish(active.pending, Err(reason));
                    } else {
                        self.active = Some(active);
                    }
                }
            }
        }
        if self.active.is_some() || self.work.is_some() || !self.auxiliary.is_empty() || self.queue.try_borrow().is_ok_and(|q| !q.pending.is_empty()) {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
    }

    fn finish(&self, pending: Pending, result: Result<Value, String>) {
        if let Ok(mut q) = self.queue.try_borrow_mut() {
            q.count = q.count.saturating_sub(1);
            q.bytes = q.bytes.saturating_sub(pending.bytes);
        }
        let value = result.unwrap_or_else(|e| json!({"ok":false,"error":e}));
        match js(&value) {
            Ok(value) => {
                let _ = pending.resolve.call1(&JsValue::NULL, &value);
            }
            Err(e) => {
                let _ = pending.reject.call1(&JsValue::NULL, &e);
            }
        }
    }
}

async fn register(context: &JsValue, tools: &js_sys::Array, status: &JsValue) -> Result<(), JsValue> {
    let register = member(context, "registerTool").ok_or_else(|| error("WebMCP has no registerTool"))?.dyn_into::<Function>()?;
    for (index, tool) in tools.iter().enumerate() {
        // Current API returns a Promise; the earlier navigator API returns undefined.
        let result = register.call1(context, &tool)?;
        wasm_bindgen_futures::JsFuture::from(Promise::resolve(&result)).await?;
        set(status, "registered", &JsValue::from_f64((index + 1) as f64))?;
    }
    log::info!("FDE2E Photocraft: registered {} WebMCP tools", tools.length());
    Ok(())
}
