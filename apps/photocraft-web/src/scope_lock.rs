use std::cell::{Cell, RefCell};
use std::rc::Rc;

use js_sys::{Function, Object, Promise, Reflect};
use wasm_bindgen::{JsCast as _, JsValue, closure::Closure};
use wasm_bindgen_futures::JsFuture;

use crate::task_scope::Scope;

pub struct Writer {
    release: Option<Function>,
}

impl Writer {
    pub async fn acquire(scope: &Scope) -> Result<Self, String> {
        if scope.id().is_none() {
            return Ok(Self { release: None });
        }
        let window = web_sys::window().ok_or("Browser window unavailable")?;
        let manager = Reflect::get(window.navigator().as_ref(), &JsValue::from_str("locks")).map_err(message)?;
        let unsupported = || {
            "Task isolation requires the browser Web Locks API. Use a supported browser on HTTPS or trusted localhost; no shared-storage fallback was opened"
                .to_owned()
        };
        if manager.is_null() || manager.is_undefined() {
            return Err(unsupported());
        }
        let request = Reflect::get(&manager, &JsValue::from_str("request")).map_err(message)?.dyn_into::<Function>().map_err(|_| unsupported())?;
        let options = Object::new();
        Reflect::set(&options, &JsValue::from_str("ifAvailable"), &JsValue::TRUE).map_err(message)?;
        let release = Rc::new(RefCell::new(None));
        let acquired = Rc::new(RefCell::new(None));
        let ready = Promise::new(&mut |resolve, _| *acquired.borrow_mut() = Some(resolve));
        let target = release.clone();
        let signal = acquired.clone();
        let abandoned = Rc::new(Cell::new(false));
        let cancelled = abandoned.clone();
        let callback = Closure::<dyn FnMut(JsValue) -> JsValue>::new(move |lock: JsValue| {
            if cancelled.get() {
                return JsValue::UNDEFINED;
            }
            if lock.is_null() || lock.is_undefined() {
                if let Some(resolve) = signal.borrow_mut().take() {
                    let _ = resolve.call1(&JsValue::NULL, &JsValue::FALSE);
                }
                return JsValue::UNDEFINED;
            }
            let held = Promise::new(&mut |resolve, _| *target.borrow_mut() = Some(resolve));
            if let Some(resolve) = signal.borrow_mut().take() {
                let _ = resolve.call1(&JsValue::NULL, &JsValue::TRUE);
            }
            held.into()
        });
        let requested = request
            .call3(&manager, &JsValue::from_str(&scope.storage("photocraft.writer")), &options, callback.as_ref())
            .map_err(message)?
            .dyn_into::<Promise>()
            .map_err(message)?;
        let pending = acquired.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(error) = JsFuture::from(requested).await
                && let Some(resolve) = pending.borrow_mut().take()
            {
                let _ = resolve.call1(&JsValue::NULL, &error);
            }
            drop(callback);
        });
        let result = match crate::indexed_presets::before(&ready, js_sys::Date::now() + 10_000.0).await {
            Ok(result) => result,
            Err(error) => {
                abandoned.set(true);
                if let Some(release) = release.borrow_mut().take() {
                    let _ = release.call0(&JsValue::NULL);
                }
                return Err(format!("Task workspace writer lock unavailable: {error}"));
            }
        };
        if result.as_bool() != Some(true) {
            return Err(if result.as_bool() == Some(false) {
                "This task workspace is already open for writing. Continue in its existing page, or choose a different scope. No takeover was attempted".into()
            } else {
                format!("Task workspace writer lock unavailable: {}", message(result))
            });
        }
        let release = release.borrow_mut().take();
        Ok(Self { release })
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.call0(&JsValue::NULL);
        }
    }
}

fn message(error: JsValue) -> String {
    error.as_string().unwrap_or_else(|| format!("{error:?}"))
}
