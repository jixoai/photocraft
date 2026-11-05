use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};

use photocraft_doc::{DocId, Document};
use photocraft_engine::display_color::CanvasDisplay;

use super::display::{cache_key, display_image};
use super::textures::{DetailPlan, canvas_texture_options, detail_plan, texture_factor, texture_side};
use super::{CanvasCache, CpuTextures, DRect, DetailTexture, InView};
use crate::PhotocraftApp;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Key {
    revision: u64,
    preview: u64,
    factor: u32,
    detail: Option<(u64, DetailPlan)>,
}

struct Rendered {
    image: Option<egui::ColorImage>,
    opaque: bool,
    detail: Option<(egui::ColorImage, DRect)>,
    ms: f64,
}

struct Pending {
    key: Key,
    source: Arc<Document>,
    stop: Arc<AtomicBool>,
    receiver: Receiver<Result<Rendered, String>>,
    requested_frame: u64,
}

impl Drop for Pending {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}

#[derive(Default)]
pub(crate) struct Redraws {
    pending: HashMap<(DocId, u32), Pending>,
}

impl Redraws {
    pub fn retain(&mut self, documents: impl Iterator<Item = DocId>, frame: u64) {
        let documents: Vec<_> = documents.collect();
        self.pending.retain(|(id, _), pending| documents.contains(id) && pending.requested_frame.saturating_add(1) >= frame);
    }

    pub fn pending_for(&self, document: DocId) -> bool {
        self.pending.keys().any(|(id, _)| *id == document)
    }
}

fn compute(doc: &Document, display: Option<&CanvasDisplay>, key: Key, full: bool, stop: &AtomicBool) -> Result<Rendered, String> {
    let started = web_time::Instant::now();
    let check = || if stop.load(Ordering::Acquire) { Err("Canvas redraw superseded".to_string()) } else { Ok(()) };
    let mut opaque = true;
    let image = if full {
        let width = (doc.size.width / key.factor).max(1);
        let height = (doc.size.height / key.factor).max(1);
        let count = (width as usize).checked_mul(height as usize).ok_or("Canvas dimensions overflow")?;
        let mut image = egui::ColorImage::new([width as usize, height as usize], vec![egui::Color32::TRANSPARENT; count]);
        for row in (0..height).step_by(128) {
            check()?;
            let top = u64::from(row) * u64::from(doc.size.height) / u64::from(height);
            let bottom = u64::from(row.saturating_add(128).min(height)) * u64::from(doc.size.height) / u64::from(height);
            let area = DRect::new(
                0,
                i32::try_from(top).map_err(|_| "Canvas dimensions overflow")?,
                doc.bounds().x1,
                i32::try_from(bottom).map_err(|_| "Canvas dimensions overflow")?,
            );
            let buffer = photocraft_compose::render_reduced_damage(doc, width, height, area);
            let band = display_image(display, &buffer);
            opaque = opaque && band.pixels.iter().all(egui::Color32::is_opaque);
            let offset = usize::try_from(buffer.rect.y0).ok().and_then(|top| top.checked_mul(width as usize)).ok_or("Canvas band outside texture")?;
            let end = offset.checked_add(band.pixels.len()).ok_or("Canvas band size overflow")?;
            image.pixels.get_mut(offset..end).ok_or("Canvas band outside texture")?.copy_from_slice(&band.pixels);
        }
        Some(image)
    } else {
        None
    };
    check()?;
    let detail = key.detail.map(|(_, plan)| {
        let width = (doc.size.width / plan.factor).max(1);
        let height = (doc.size.height / plan.factor).max(1);
        let buffer = photocraft_compose::render_reduced_damage(doc, width, height, plan.region);
        (display_image(display, &buffer), buffer.rect)
    });
    check()?;
    Ok(Rendered { image, opaque, detail, ms: started.elapsed().as_secs_f64() * 1000.0 })
}

fn matches(pending: &Pending, key: Key, doc: &Arc<Document>) -> bool {
    pending.key == key && Arc::ptr_eq(&pending.source, doc)
}

fn publish(ctx: &egui::Context, cache: &mut CanvasCache, key: Key, slot: (DocId, u32), rendered: &mut Rendered) {
    if let Some(image) = rendered.image.take() {
        cache.texture_opaque = rendered.opaque;
        match cache.texture.as_mut() {
            Some(texture) if texture.size() == image.size => texture.set(image, canvas_texture_options(key.factor)),
            _ => cache.texture = Some(ctx.load_texture(format!("canvas-{}-{}", slot.0.0, slot.1), image, canvas_texture_options(key.factor))),
        }
        cache.scale = 1.0 / key.factor as f32;
        cache.tex_revision = key.revision;
        cache.tex_preview_key = key.preview;
    }
    if let (Some((view, plan)), Some((image, out))) = (key.detail, rendered.detail.take()) {
        cache.detail.retain(|(id, _)| *id != view);
        cache.detail.push((
            view,
            DetailTexture {
                texture: ctx.load_texture(format!("canvas-detail-{}-{view}", slot.0.0), image, canvas_texture_options(plan.factor)),
                plan,
                out,
                revision: key.revision,
                preview_key: key.preview,
                rendered: false,
            },
        ));
        if cache.detail.len() > 4 {
            cache.detail.remove(0);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn ensure(
    app: &mut PhotocraftApp,
    ctx: &egui::Context,
    doc: Arc<Document>,
    output: Option<u32>,
    revision: u64,
    preview: u64,
    display: Option<Arc<CanvasDisplay>>,
    bounds: DRect,
    view: Option<InView>,
) -> Option<CpuTextures> {
    let slot = cache_key(doc.id, output);
    let size = [doc.size.width, doc.size.height];
    let factor = texture_factor(size, texture_side(ctx));
    let detail = view.and_then(|view| detail_plan(size, texture_side(ctx), view.zoom, view.visible).map(|plan| (view.key, plan)));
    let key = Key { revision, preview, factor, detail };
    let cache = app.canvases.entry(slot).or_insert(CanvasCache {
        revision: 0,
        texture: None,
        scale: 1.0,
        preview_key: 0,
        on_gpu: false,
        tex_revision: 0,
        tex_preview_key: 0,
        texture_opaque: false,
        grid: None,
        detail: Vec::new(),
    });
    if let Some(pending) = app.cpu_redraws.pending.get_mut(&slot) {
        pending.requested_frame = app.frame;
        let fresh = matches(pending, key, &doc);
        if !fresh {
            pending.stop.store(true, Ordering::Release);
        }
        match pending.receiver.try_recv() {
            Ok(result) => {
                app.cpu_redraws.pending.remove(&slot);
                if fresh {
                    match result {
                        Ok(mut rendered) => {
                            let started = web_time::Instant::now();
                            publish(ctx, cache, key, slot, &mut rendered);
                            record_publish(&mut app.perf, u64::from(size[0]) * u64::from(size[1]), rendered.ms, started.elapsed().as_secs_f64() * 1000.0);
                        }
                        Err(error) => {
                            app.ui.status = error;
                            app.ui.status_error = true;
                        }
                    }
                }
            }
            Err(TryRecvError::Disconnected) => {
                app.cpu_redraws.pending.remove(&slot);
            }
            Err(TryRecvError::Empty) => {}
        }
    }
    let full = cache.texture.is_none() || cache.tex_revision != revision || cache.tex_preview_key != preview || cache.scale != 1.0 / factor as f32;
    let detail_fresh = detail.is_none_or(|(view, plan)| {
        cache.detail.iter().any(|(id, detail)| *id == view && detail.plan == plan && detail.revision == revision && detail.preview_key == preview)
    });
    if !app.cpu_redraws.pending.contains_key(&slot) && (full || !detail_fresh) {
        let (sender, receiver) = std::sync::mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let source = doc.clone();
        let cancel = stop.clone();
        match photocraft_engine::background::spawn("cpu-canvas", move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| compute(&source, display.as_deref(), key, full, &cancel)))
                .unwrap_or_else(|_| Err("Canvas redraw failed".into()));
            let _ = sender.send(result);
        }) {
            Ok(_) => {
                app.cpu_redraws.pending.insert(slot, Pending { key, source: doc.clone(), stop, receiver, requested_frame: app.frame });
            }
            Err(error) => {
                app.ui.status = error.to_string();
                app.ui.status_error = true;
            }
        }
    }
    if app.cpu_redraws.pending.contains_key(&slot) {
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
    let texture = cache.texture.as_ref()?.id();
    let shown = detail.and_then(|(view, plan)| {
        let detail = &cache.detail.iter().find(|(id, detail)| *id == view && detail.plan == plan)?.1;
        let width = (size[0] / plan.factor).max(1);
        let height = (size[1] / plan.factor).max(1);
        let scale_x = size[0] as f32 / width as f32;
        let scale_y = size[1] as f32 / height as f32;
        Some((
            detail.texture.id(),
            [detail.out.x0 as f32 * scale_x, detail.out.y0 as f32 * scale_y, detail.out.x1 as f32 * scale_x, detail.out.y1 as f32 * scale_y],
        ))
    });
    Some(CpuTextures { texture, scale: cache.scale, bounds, detail: shown })
}

fn record_publish(perf: &mut crate::gpu_canvas::Perf, pixels: u64, worker_ms: f64, publish_ms: f64) {
    perf.span("cpu-worker", worker_ms);
    perf.record("background-cpu", pixels, 0.0, publish_ms);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banded_redraw_matches_full_composite_at_odd_sizes_and_depths() {
        for depth in [8, 16, 32] {
            let mut session = photocraft_engine::Session::new();
            session.execute("file.new", serde_json::json!({"width":513,"height":271,"depth":depth})).unwrap();
            session.execute("filter.noise.addNoise", serde_json::json!({"amount":10})).unwrap();
            let doc = &session.active().unwrap().doc;
            for factor in [1, 2, 4] {
                let key = Key { revision: 1, preview: 0, factor, detail: None };
                let rendered = compute(doc, None, key, true, &AtomicBool::new(false)).unwrap();
                let result = rendered.image.unwrap();
                let reference = display_image(None, &photocraft_compose::render_reduced(doc, (513 / factor).max(1), (271 / factor).max(1)));
                assert_eq!(result.pixels, reference.pixels, "depth={depth}, factor={factor}");
                assert_eq!(rendered.opaque, reference.pixels.iter().all(egui::Color32::is_opaque));
                let pending = Pending { key, source: doc.clone(), stop: Arc::default(), receiver: std::sync::mpsc::channel().1, requested_frame: 0 };
                assert!(matches(&pending, key, doc));
                assert!(!matches(&pending, Key { revision: 2, ..key }, doc));
            }
        }
    }

    #[test]
    fn hidden_output_redraws_are_cancelled_and_released() {
        let mut session = photocraft_engine::Session::new();
        session.execute("file.new", serde_json::json!({"width":3,"height":2})).unwrap();
        let doc = session.active().unwrap().doc.clone();
        let key = Key { revision: 0, preview: 0, factor: 1, detail: None };
        let stop = Arc::new(AtomicBool::new(false));
        let mut redraws = Redraws::default();
        redraws
            .pending
            .insert((doc.id, 1), Pending { key, source: doc.clone(), stop: stop.clone(), receiver: std::sync::mpsc::channel().1, requested_frame: 0 });
        redraws
            .pending
            .insert((doc.id, 2), Pending { key, source: doc.clone(), stop: Arc::default(), receiver: std::sync::mpsc::channel().1, requested_frame: 2 });
        redraws.retain([doc.id].into_iter(), 3);
        assert!(stop.load(Ordering::Acquire));
        assert_eq!(redraws.pending.len(), 1);
        assert!(redraws.pending_for(doc.id));
        redraws.retain(std::iter::empty(), 4);
        assert!(!redraws.pending_for(doc.id));
    }

    #[test]
    fn worker_time_is_not_recorded_as_main_thread_compositing() {
        let mut perf = crate::gpu_canvas::Perf::default();
        record_publish(&mut perf, 24_000_000, 1800.0, 2.0);
        assert_eq!(perf.composite_ms, 0.0);
        assert_eq!(perf.upload_ms, 2.0);
        assert_eq!(perf.spans.get("cpu-worker"), Some(&1800.0));
    }

    #[test]
    fn cancelled_redraw_never_publishes_pixels() {
        let mut session = photocraft_engine::Session::new();
        session.execute("file.new", serde_json::json!({"width":4,"height":4})).unwrap();
        let key = Key { revision: 1, preview: 0, factor: 1, detail: None };
        assert!(compute(&session.active().unwrap().doc, None, key, true, &AtomicBool::new(true)).is_err());
    }
}
