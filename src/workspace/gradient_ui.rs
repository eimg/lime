use std::sync::Arc;

use gpui::{Context, Pixels, Point, RenderImage, Window};
use image::RgbaImage;

use super::Workspace;
use super::layers::{MovePaint, with_layer_opacity};
use crate::document::{self, DocumentContent, FillMode, LayerId};
use crate::gradient::library::{self, Library, PresetValue};
use crate::paint::{Gradient, GradientBlend, GradientShape};
use crate::selection::Selection;
use crate::transform::ColorMode;

pub(super) struct PresetStorage {
    pub loading: bool,
    pub saving: bool,
    pub error: Option<String>,
    pub exit_after_save: Option<bool>,
    path: Option<std::path::PathBuf>,
    original: Option<Vec<u8>>,
    blocked: bool,
}

impl Default for PresetStorage {
    fn default() -> Self {
        Self {
            loading: false,
            saving: false,
            error: None,
            exit_after_save: None,
            path: None,
            original: None,
            blocked: true,
        }
    }
}

fn library_error(error: anyhow::Error) -> String {
    format!("{error:#}").chars().take(512).collect()
}

impl PresetStorage {
    pub(super) fn ready(&self) -> bool {
        !self.loading && !self.saving && !self.blocked && self.path.is_some()
    }
}

pub(super) struct GradientFrame {
    pub start: (f64, f64),
    pub end: (f64, f64),
}

pub(super) struct GradientSession {
    pub(super) epoch: u64,
    backdrop: Option<GradientBackdrop>,
    layer: LayerId,
    source: Arc<RgbaImage>,
    offset: (i32, i32),
    opacity: u8,
    start: (f64, f64),
    end: (f64, f64),
    drag: Option<GradientDrag>,
}

impl GradientSession {
    pub(super) fn dragging(&self) -> bool {
        self.drag.is_some()
    }
}

enum GradientDrag {
    New,
    Handle { index: usize, grab: (f64, f64) },
}

// Ownership is transferred between the worker, a waiting session, and its
// overlay. There is no cross-document cache and no second owner evicting GPU data.
struct GradientBackdrop {
    below: Option<Arc<RenderImage>>,
    above: Option<Arc<RenderImage>>,
}

impl GradientBackdrop {
    fn into_images(self) -> impl Iterator<Item = Arc<RenderImage>> {
        self.below.into_iter().chain(self.above)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GradientRequest {
    tab: usize,
    generation: u64,
    layer: LayerId,
    session: u64,
    request: u64,
}

#[derive(Debug, PartialEq, Eq)]
enum GradientCompletion {
    Publish,
    KeepBackdrop,
    Discard,
}

impl GradientRequest {
    fn completion(self, current: Option<Self>, has_overlay: bool) -> GradientCompletion {
        let Some(current) = current else {
            return GradientCompletion::Discard;
        };
        if self == current {
            GradientCompletion::Publish
        } else if (Self {
            request: current.request,
            ..self
        }) == current
            && !has_overlay
        {
            GradientCompletion::KeepBackdrop
        } else {
            GradientCompletion::Discard
        }
    }
}

struct PreparedGradient {
    backdrop: Option<GradientBackdrop>,
    base: Option<DocumentContent>,
    source: Arc<RgbaImage>,
    offset: (i32, i32),
    opacity: u8,
    layer: LayerId,
    gradient: Gradient,
    selection: Selection,
    canvas: (u32, u32),
    mode: ColorMode,
    alpha_locked: bool,
}

struct GradientPreview {
    below: Option<Arc<RenderImage>>,
    above: Option<Arc<RenderImage>>,
    sprite: Arc<RenderImage>,
    offset: (i32, i32),
    size: (u32, u32),
}

impl Workspace {
    fn gradient_value(&self, start: (f64, f64), end: (f64, f64)) -> Gradient {
        Gradient {
            shape: self.preferences.gradient_shape,
            blend: self.preferences.gradient_blend,
            start,
            end,
            foreground: document::solid_color(self.color_memory.swatches[0]),
            background: document::solid_color(self.color_memory.swatches[1]),
            ramp: self
                .preferences
                .gradient_use_custom
                .then_some(self.preferences.gradient_custom)
                .flatten(),
            reverse: self.preferences.gradient_reverse,
        }
    }

    fn dynamic_ramp(&self) -> crate::gradient::ColorRamp {
        let start = document::solid_color(self.color_memory.swatches[0]);
        let end = match self.preferences.gradient_blend {
            GradientBlend::ForegroundToBackground => {
                document::solid_color(self.color_memory.swatches[1])
            }
            GradientBlend::ForegroundToTransparent => {
                image::Rgba([start[0], start[1], start[2], 0])
            }
        };
        crate::gradient::ColorRamp::new(start, end)
    }

    fn gradient_settings_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.gradient_revision = self.gradient_revision.wrapping_add(1);
        self.preferences.gradient_selected_preset = None;
        self.refresh_gradient(window, cx);
        cx.notify();
    }

    pub(super) fn set_gradient_custom(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.preferences.gradient_use_custom {
            return;
        }
        if self.preferences.gradient_custom.is_none() {
            self.preferences.gradient_custom = Some(self.dynamic_ramp());
        }
        self.preferences.gradient_use_custom = true;
        if !super::settings_ui::ToolSection::GradientStops.expanded(&self.preferences.tool_sections)
        {
            super::settings_ui::ToolSection::GradientStops
                .toggle(&mut self.preferences.tool_sections);
        }
        self.gradient_settings_changed(window, cx);
    }

    pub(super) fn reverse_gradient(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.preferences.gradient_reverse = !self.preferences.gradient_reverse;
        self.gradient_settings_changed(window, cx);
    }

    pub(super) fn selected_gradient_stop(&self) -> Option<crate::gradient::ColorStop> {
        if !self.preferences.gradient_use_custom {
            return None;
        }
        self.preferences
            .gradient_custom
            .as_ref()?
            .stops()
            .get(self.preferences.gradient_selected_stop)
            .copied()
    }

    pub(super) fn select_gradient_stop(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.commit_numeric_edit(window, cx);
        if self
            .preferences
            .gradient_custom
            .as_ref()
            .is_some_and(|ramp| index < ramp.stops().len())
        {
            self.preferences.gradient_selected_stop = index;
            cx.notify();
        }
    }

    pub(super) fn add_gradient_stop(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self
            .preferences
            .gradient_custom
            .as_mut()
            .and_then(|ramp| ramp.add_stop())
        {
            self.preferences.gradient_selected_stop = index;
            self.gradient_settings_changed(window, cx);
        }
    }

    pub(super) fn remove_gradient_stop(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .preferences
            .gradient_custom
            .as_mut()
            .is_some_and(|ramp| ramp.remove_stop(self.preferences.gradient_selected_stop))
        {
            self.preferences.gradient_selected_stop =
                self.preferences.gradient_selected_stop.saturating_sub(1);
            self.gradient_settings_changed(window, cx);
        }
    }

    pub(super) fn move_gradient_stop(
        &mut self,
        position: u16,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .preferences
            .gradient_custom
            .as_mut()
            .is_some_and(|ramp| ramp.move_stop(self.preferences.gradient_selected_stop, position))
        {
            self.gradient_settings_changed(window, cx);
        }
    }

    pub(super) fn set_gradient_stop_alpha(
        &mut self,
        alpha: u8,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(mut stop) = self.selected_gradient_stop() else {
            return;
        };
        stop.color[3] = alpha;
        if self
            .preferences
            .gradient_custom
            .as_mut()
            .is_some_and(|ramp| ramp.set_color(self.preferences.gradient_selected_stop, stop.color))
        {
            self.gradient_settings_changed(window, cx);
        }
    }

    pub(super) fn set_gradient_stop_rgb(
        &mut self,
        revision: u64,
        index: usize,
        color: u32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if revision != self.gradient_revision || !self.preferences.gradient_use_custom {
            return;
        }
        let Some(ramp) = self.preferences.gradient_custom.as_mut() else {
            return;
        };
        let Some(stop) = ramp.stops().get(index) else {
            return;
        };
        let mut rgba = document::solid_color(color);
        rgba[3] = stop.color[3];
        if ramp.set_color(index, rgba) {
            self.gradient_settings_changed(window, cx);
        }
        self.remember_color(color, window, cx);
    }

    pub(super) fn current_gradient_preset(&self) -> PresetValue {
        PresetValue {
            ramp: if self.preferences.gradient_use_custom {
                self.preferences
                    .gradient_custom
                    .unwrap_or_else(|| self.dynamic_ramp())
            } else {
                self.dynamic_ramp()
            },
            shape: self.preferences.gradient_shape,
            reverse: self.preferences.gradient_reverse,
        }
    }

    pub(super) fn load_gradient_library(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.gradient_store.loading || self.gradient_store.saving {
            return;
        }
        self.gradient_store.path = library::default_path();
        let Some(path) = self.gradient_store.path.clone() else {
            self.gradient_store.blocked = true;
            self.gradient_store.error =
                Some("The gradient library location is unavailable.".into());
            cx.notify();
            return;
        };
        self.gradient_store.loading = true;
        self.gradient_store.blocked = true;
        self.gradient_store.error = None;
        let job = cx
            .background_executor()
            .spawn(async move { library::load(&path) });
        cx.spawn_in(window, async move |workspace, cx| {
            let result = job.await;
            let _ = workspace.update_in(cx, |this, _, cx| {
                this.gradient_store.loading = false;
                match result {
                    Ok((library, bytes)) => {
                        this.gradient_library = library;
                        this.gradient_store.original = bytes;
                        this.gradient_store.blocked = false;
                        this.reconcile_gradient_preset();
                    }
                    Err(error) => this.gradient_store.error = Some(library_error(error)),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn write_gradient_library(
        &mut self,
        candidate: Library,
        selected: Option<u64>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.gradient_store.ready() {
            return false;
        }
        if candidate == self.gradient_library {
            return true;
        }
        let Some(path) = self.gradient_store.path.clone() else {
            return false;
        };
        let original = self.gradient_store.original.clone();
        let saved = candidate.clone();
        self.gradient_store.saving = true;
        self.gradient_store.error = None;
        let job = cx
            .background_executor()
            .spawn(async move { library::save(&path, original.as_deref(), &saved) });
        cx.spawn_in(window, async move |workspace, cx| {
            let result = job.await;
            let _ = workspace.update_in(cx, |this, window, cx| {
                this.gradient_store.saving = false;
                let requested_exit = this.gradient_store.exit_after_save.take();
                match result {
                    Ok(bytes) => {
                        this.gradient_library = candidate;
                        this.gradient_store.original = Some(bytes);
                        this.preferences.gradient_selected_preset = selected.filter(|id| {
                            this.gradient_library.get(*id).is_some_and(|preset| {
                                preset.value == this.current_gradient_preset()
                            })
                        });
                        this.preferences.gradient_presets_expanded = false;
                        if let Some(quit) = requested_exit
                            && this.request_exit(quit, window, cx)
                        {
                            if quit {
                                cx.quit();
                            } else {
                                window.remove_window();
                            }
                        }
                    }
                    Err(error) => {
                        let message = library_error(error);
                        this.errors
                            .push(format!("Could not save gradients: {message}"));
                        this.gradient_store.error = Some(message);
                        this.gradient_store.blocked = true;
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
        true
    }

    pub(super) fn load_gradient_preset(
        &mut self,
        id: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.gradient_store.loading || self.gradient_store.saving {
            return;
        }
        self.commit_numeric_edit(window, cx);
        let Some(value) = self.gradient_library.get(id).map(|preset| preset.value) else {
            return;
        };
        self.preferences.gradient_custom = Some(value.ramp);
        self.preferences.gradient_use_custom = true;
        self.preferences.gradient_shape = value.shape;
        self.preferences.gradient_reverse = value.reverse;
        self.preferences.gradient_selected_stop = 0;
        self.gradient_settings_changed(window, cx);
        self.preferences.gradient_selected_preset = Some(id);
        self.preferences.gradient_presets_expanded = false;
    }

    pub(super) fn delete_gradient_preset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.gradient_store.ready() {
            return;
        }
        let Some(id) = self.preferences.gradient_selected_preset else {
            return;
        };
        let mut candidate = self.gradient_library.clone();
        if candidate.remove(id) {
            self.write_gradient_library(candidate, None, window, cx);
        }
    }

    pub(super) fn begin_gradient(
        &mut self,
        position: Point<Pixels>,
        scale: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.active_tab().is_none_or(|tab| tab.busy()) || self.refuse_text_pixels(false, cx) {
            return;
        }
        let Some(point) = self.canvas_point(position, scale) else {
            return;
        };
        if let Some(index) = self.gradient_handle_at(position, scale) {
            if let Some(session) = self.gradient.as_mut() {
                let anchor = if index == 0 {
                    session.start
                } else {
                    session.end
                };
                session.drag = Some(GradientDrag::Handle {
                    index,
                    grab: (anchor.0 - point.0, anchor.1 - point.1),
                });
            }
            cx.notify();
            return;
        }
        let Some(tab) = self.active_tab() else {
            return;
        };
        let Some(layer) = tab.document.content.layer(tab.active_layer) else {
            return;
        };
        if !layer.visible {
            self.notice("The active layer is hidden. Show it before filling.", cx);
            return;
        }
        let session = GradientSession {
            epoch: self.gradient_token.wrapping_add(1),
            backdrop: None,
            layer: layer.id,
            source: layer.pixels.clone(),
            offset: layer.offset,
            opacity: layer.opacity,
            start: point,
            end: point,
            drag: Some(GradientDrag::New),
        };
        if let Some(backdrop) = self.gradient.take().and_then(|session| session.backdrop) {
            super::drop_images(backdrop.into_images(), window, cx);
        }
        self.gradient = Some(session);
        self.request_gradient_preview(window, cx);
    }

    pub(super) fn extend_gradient(
        &mut self,
        position: Point<Pixels>,
        scale: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .gradient
            .as_ref()
            .is_none_or(|session| session.drag.is_none())
        {
            return;
        }
        let Some(point) = self.canvas_point(position, scale) else {
            return;
        };
        let mut changed = false;
        if let Some(session) = self.gradient.as_mut() {
            match session.drag {
                Some(GradientDrag::Handle { index, grab }) => {
                    let point = (point.0 + grab.0, point.1 + grab.1);
                    let anchor = if index == 0 {
                        &mut session.start
                    } else {
                        &mut session.end
                    };
                    changed = *anchor != point;
                    *anchor = point;
                }
                Some(GradientDrag::New) => {
                    changed = session.end != point;
                    session.end = point;
                }
                None => {}
            }
        }
        if changed {
            self.request_gradient_preview(window, cx);
        }
    }

    pub(super) fn end_gradient_drag(&mut self, cx: &mut Context<Self>) {
        let Some(session) = self.gradient.as_mut() else {
            return;
        };
        if session.drag.take().is_some() {
            cx.notify();
        }
    }

    pub(super) fn cancel_gradient(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.gradient_token = self.gradient_token.wrapping_add(1);
        self.gradient_dirty = false;
        if let Some(backdrop) = self.gradient.take().and_then(|session| session.backdrop) {
            super::drop_images(backdrop.into_images(), window, cx);
        }
        self.drop_gradient_overlay(window, cx);
    }

    pub(super) fn can_apply_gradient(&self) -> bool {
        self.active_tab().is_some_and(|tab| !tab.busy())
            && self
                .gradient
                .as_ref()
                .is_some_and(|session| session.drag.is_none())
    }

    pub(super) fn apply_gradient(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.refuse_text_pixels(false, cx) || !self.can_apply_gradient() {
            return;
        }
        let Some(gradient) = self.gradient_spec() else {
            return;
        };
        let Some(layer) = self.gradient.as_ref().map(|session| session.layer) else {
            return;
        };
        self.apply_preview(
            move |content, _, selection| content.apply_gradient(layer, gradient, selection),
            window,
            cx,
        );
    }

    pub(super) fn refresh_gradient(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.gradient.is_some() {
            self.request_gradient_preview(window, cx);
        }
    }

    pub(super) fn set_gradient_shape(
        &mut self,
        shape: GradientShape,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.preferences.gradient_shape == shape {
            return;
        }
        self.preferences.gradient_shape = shape;
        self.gradient_settings_changed(window, cx);
    }

    pub(super) fn set_gradient_blend(
        &mut self,
        blend: GradientBlend,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.preferences.gradient_blend == blend && !self.preferences.gradient_use_custom {
            return;
        }
        self.preferences.gradient_blend = blend;
        self.preferences.gradient_use_custom = false;
        self.gradient_settings_changed(window, cx);
    }

    pub(super) fn set_fill_mode(&mut self, mode: FillMode, cx: &mut Context<Self>) {
        if self.preferences.fill_mode != mode {
            self.preferences.fill_mode = mode;
            cx.notify();
        }
    }

    pub(super) fn set_sample_all(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.preferences.sample_all != enabled {
            self.preferences.sample_all = enabled;
            cx.notify();
        }
    }

    pub(super) fn gradient_frame(&self) -> Option<GradientFrame> {
        let session = self.gradient.as_ref()?;
        Some(GradientFrame {
            start: session.start,
            end: session.end,
        })
    }

    fn gradient_spec(&self) -> Option<Gradient> {
        let session = self.gradient.as_ref()?;
        Some(self.gradient_value(session.start, session.end))
    }

    fn gradient_request(&self) -> Option<GradientRequest> {
        let session = self.gradient.as_ref()?;
        let tab = self
            .active_tab()
            .filter(|tab| tab.active_layer == session.layer)?;
        Some(GradientRequest {
            tab: tab.id,
            generation: tab.history.generation(),
            layer: session.layer,
            session: session.epoch,
            request: self.gradient_token,
        })
    }

    fn prepare_gradient_preview(&self) -> Option<PreparedGradient> {
        let session = self.gradient.as_ref()?;
        let tab = self.active_tab()?;
        Some(PreparedGradient {
            backdrop: None,
            base: (self.gradient_overlay.is_none() && session.backdrop.is_none())
                .then(|| tab.document.content.clone()),
            source: session.source.clone(),
            offset: session.offset,
            opacity: session.opacity,
            layer: session.layer,
            gradient: self.gradient_value(session.start, session.end),
            selection: tab.selection.clone(),
            canvas: tab.document.content.size(),
            mode: tab.document.content.mode(),
            alpha_locked: tab.document.content.layer(session.layer)?.locks.alpha,
        })
    }

    fn request_gradient_preview(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.gradient.is_none() {
            return;
        }
        self.gradient_token = self.gradient_token.wrapping_add(1);
        self.gradient_dirty = true;
        self.pump_gradient_preview(window, cx);
        cx.notify();
    }

    fn pump_gradient_preview(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.gradient_busy || !self.gradient_dirty || self.gradient.is_none() {
            return;
        }
        let Some(request) = self.gradient_request() else {
            return;
        };
        let Some(mut prepared) = self.prepare_gradient_preview() else {
            return;
        };
        prepared.backdrop = self
            .gradient
            .as_mut()
            .and_then(|session| session.backdrop.take());
        self.gradient_dirty = false;
        self.gradient_busy = true;
        let job = cx
            .background_executor()
            .spawn(async move { preview_gradient(prepared) });
        cx.spawn_in(window, async move |workspace, cx| {
            let preview = job.await;
            let _ = workspace.update_in(cx, |workspace, window, cx| {
                workspace.gradient_busy = false;
                match request.completion(
                    workspace.gradient_request(),
                    workspace.gradient_overlay.is_some(),
                ) {
                    GradientCompletion::Publish => {
                        workspace.publish_gradient_preview(preview, window, cx)
                    }
                    GradientCompletion::KeepBackdrop => {
                        // Pointer/parameter changes invalidate only the sprite. The
                        // unchanged stack is expensive and still belongs to this session.
                        let GradientPreview {
                            below,
                            above,
                            sprite,
                            ..
                        } = preview;
                        workspace.gradient.as_mut().unwrap().backdrop =
                            Some(GradientBackdrop { below, above });
                        super::drop_images([sprite], window, cx);
                    }
                    GradientCompletion::Discard => drop_gradient_preview(preview, window, cx),
                }
                if workspace.gradient_dirty {
                    workspace.pump_gradient_preview(window, cx);
                }
            });
        })
        .detach();
    }

    fn publish_gradient_preview(
        &mut self,
        preview: GradientPreview,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let GradientPreview {
            below,
            above,
            sprite,
            offset,
            size,
        } = preview;
        if let Some(overlay) = self.gradient_overlay.as_mut() {
            let previous = std::mem::replace(&mut overlay.sprite, sprite);
            overlay.offset = offset;
            overlay.size = size;
            super::drop_images(std::iter::once(previous), window, cx);
            super::drop_images(below.into_iter().chain(above), window, cx);
        } else {
            self.gradient_overlay = Some(MovePaint {
                below,
                sprite,
                above,
                offset,
                size,
            });
        }
        cx.notify();
    }

    fn drop_gradient_overlay(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(overlay) = self.gradient_overlay.take() {
            super::drop_images(overlay.into_images(), window, cx);
        }
    }

    fn gradient_handle_at(&self, position: Point<Pixels>, scale: f32) -> Option<usize> {
        let tab = self.active_tab()?;
        let geometry =
            super::canvas_tools::PreviewGeometry::for_tab(tab, self.viewport.get(), scale)?;
        let frame = self.gradient_frame()?;
        let width = f64::from(tab.document.width());
        let height = f64::from(tab.document.height());
        if width <= 0.0 || height <= 0.0 {
            return None;
        }
        let project = |grip: (f64, f64)| {
            geometry.project(gpui::point(
                (grip.0 / width) as f32,
                (grip.1 / height) as f32,
            ))
        };
        nearest_gradient_handle([project(frame.start), project(frame.end)], position)
    }
}

fn nearest_gradient_handle(grips: [Point<Pixels>; 2], position: Point<Pixels>) -> Option<usize> {
    grips
        .into_iter()
        .enumerate()
        .filter_map(|(index, grip)| {
            let delta = grip - position;
            let distance = f32::from(delta.x).powi(2) + f32::from(delta.y).powi(2);
            (distance <= 64.0).then_some((index, distance))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|(index, _)| index)
}

fn constrain(gradient: Gradient, mode: ColorMode) -> Gradient {
    gradient.constrain(mode)
}

fn preview_gradient(prepared: PreparedGradient) -> GradientPreview {
    let PreparedGradient {
        backdrop,
        base,
        source,
        offset,
        opacity,
        layer,
        gradient,
        selection,
        canvas,
        mode,
        alpha_locked,
    } = prepared;
    let gradient = constrain(gradient, mode);
    let rendered = document::gradient_preview(
        &source,
        offset,
        gradient,
        &selection,
        canvas,
        alpha_locked,
        crate::adjustments::PREVIEW_EDGE,
    );
    let shown = with_layer_opacity(&rendered, opacity);
    let backdrop = backdrop.unwrap_or_else(|| {
        base.as_ref()
            .map(|content| prepare_gradient_backdrop(content, layer))
            .unwrap_or(GradientBackdrop {
                below: None,
                above: None,
            })
    });
    GradientPreview {
        below: backdrop.below,
        above: backdrop.above,
        sprite: document::preview_render(&shown),
        offset,
        size: source.dimensions(),
    }
}

// A single full-canvas Normal layer at full opacity already equals its
// partition composite, unless transparent pixels carry hidden RGB. Reuse only
// the proven-identical case and retain the existing compositor for every other
// stack. preview_render keeps exactly the existing dimensions and resampling.
fn reusable_partition(
    layers: &[document::RasterLayer],
    canvas: (u32, u32),
) -> Option<Option<&RgbaImage>> {
    let mut visible = layers
        .iter()
        .filter(|layer| layer.visible && layer.opacity != 0);
    let Some(layer) = visible.next() else {
        return Some(None);
    };
    if visible.next().is_some()
        || layer.offset != (0, 0)
        || layer.pixels.dimensions() != canvas
        || layer.opacity != 255
        || layer.blend_mode != document::BlendMode::Normal
        || layer.mask.as_ref().is_some_and(|mask| mask.enabled)
    {
        return None;
    }
    layer
        .pixels
        .pixels()
        .all(|pixel| pixel[3] != 0)
        .then_some(Some(&layer.pixels))
}

fn prepare_gradient_backdrop(content: &DocumentContent, layer: LayerId) -> GradientBackdrop {
    if !content.needs_composite_preview() {
        let index = content
            .layer_index(layer)
            .expect("the gradient layer exists");
        if let (Some(below), Some(above)) = (
            reusable_partition(&content.layers()[..index], content.size()),
            reusable_partition(&content.layers()[index + 1..], content.size()),
        ) {
            return GradientBackdrop {
                below: below.map(document::preview_render),
                above: above.map(document::preview_render),
            };
        }
    }
    let (below, above) = content.preview_around(layer);
    GradientBackdrop {
        below: below.as_ref().map(document::preview_render),
        above: above.as_ref().map(document::preview_render),
    }
}

fn drop_gradient_preview(
    preview: GradientPreview,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    super::drop_images(
        preview
            .below
            .into_iter()
            .chain(preview.above)
            .chain(std::iter::once(preview.sprite)),
        window,
        cx,
    );
}

#[cfg(test)]
mod startup_tests {
    use super::*;
    use image::Rgba;

    fn scene() -> (DocumentContent, LayerId) {
        let mut content = DocumentContent::single_layer(
            "Below".into(),
            RgbaImage::from_fn(32, 24, |x, y| {
                Rgba([x as u8 * 7, y as u8 * 9, 100, ((x + y) % 255 + 1) as u8])
            }),
        );
        let layer = content
            .add_layer(
                content.top_layer_id(),
                "Gradient".into(),
                Rgba([10, 20, 30, 70]),
            )
            .unwrap();
        content
            .add_layer(layer, "Above".into(), Rgba([160, 90, 30, 190]))
            .unwrap();
        (content, layer)
    }

    fn same_image(actual: Option<&Arc<RenderImage>>, expected: Option<&Arc<RenderImage>>) {
        match (actual, expected) {
            (None, None) => {}
            (Some(actual), Some(expected)) => {
                assert_eq!(actual.size(0), expected.size(0));
                assert_eq!(actual.as_bytes(0), expected.as_bytes(0));
            }
            _ => panic!("changed a background partition's visibility"),
        }
    }

    #[test]
    fn shortcut_and_fallback_preserve_background_pixels_and_partition_order() {
        let (original, active) = scene();
        assert!(reusable_partition(&original.layers()[..1], original.size()).is_some());
        for case in [
            "plain",
            "offset",
            "opacity",
            "transparent",
            "mask",
            "blend",
            "hidden",
            "multiple",
        ] {
            let mut layers = original.layers().to_vec();
            match case {
                "offset" => layers[0].offset = (-3, 2),
                "opacity" => layers[0].opacity = 130,
                "transparent" => {
                    Arc::make_mut(&mut layers[0].pixels).put_pixel(0, 0, Rgba([200, 100, 50, 0]))
                }
                "mask" => {
                    layers[0].mask = Some(document::LayerMask {
                        pixels: Arc::new(RgbaImage::from_pixel(32, 24, Rgba([0, 0, 0, 99]))),
                        enabled: true,
                    })
                }
                "blend" => layers[0].blend_mode = document::BlendMode::Multiply,
                "hidden" => layers[0].visible = false,
                "multiple" => {
                    let mut extra = layers[0].clone();
                    extra.id = LayerId(999);
                    layers.insert(0, extra);
                }
                _ => {}
            }
            let content = DocumentContent::from_layers(32, 24, layers).unwrap();
            let (below, above) = content.preview_around(active);
            let expected = GradientBackdrop {
                below: below.as_ref().map(document::preview_render),
                above: above.as_ref().map(document::preview_render),
            };
            let actual = prepare_gradient_backdrop(&content, active);
            same_image(actual.below.as_ref(), expected.below.as_ref());
            same_image(actual.above.as_ref(), expected.above.as_ref());
        }
    }

    fn prepared(content: &DocumentContent, layer: LayerId, reverse: bool) -> PreparedGradient {
        let target = content.layer(layer).unwrap();
        PreparedGradient {
            backdrop: None,
            base: Some(content.clone()),
            source: target.pixels.clone(),
            offset: target.offset,
            opacity: target.opacity,
            layer,
            gradient: Gradient {
                shape: GradientShape::Radial,
                blend: GradientBlend::ForegroundToTransparent,
                start: (1., 2.),
                end: (24., 16.),
                foreground: Rgba([100, 140, 220, 190]),
                background: Rgba([200, 20, 10, 20]),
                ramp: None,
                reverse,
            },
            selection: Selection::from_coverage(32, 24, |x, y| ((x + y) * 4) as u8),
            canvas: content.size(),
            mode: ColorMode::Grayscale,
            alpha_locked: true,
        }
    }

    #[test]
    fn superseded_gradient_reuses_only_its_unchanged_background_and_keeps_latest_sprite() {
        let (content, layer) = scene();
        let first_request = GradientRequest {
            tab: 3,
            generation: 7,
            layer,
            session: 10,
            request: 10,
        };
        let latest = GradientRequest {
            request: 11,
            ..first_request
        };
        let first = preview_gradient(prepared(&content, layer, false));
        assert_eq!(
            first_request.completion(Some(latest), false),
            GradientCompletion::KeepBackdrop
        );
        assert_eq!(
            first_request.completion(Some(latest), true),
            GradientCompletion::Discard
        );
        let mut next = prepared(&content, layer, true);
        next.base = None;
        let below_id = first.below.as_ref().unwrap().id;
        let above_id = first.above.as_ref().unwrap().id;
        next.backdrop = Some(GradientBackdrop {
            below: first.below,
            above: first.above,
        });
        let next = preview_gradient(next);
        assert_eq!(next.below.as_ref().unwrap().id, below_id);
        assert_eq!(next.above.as_ref().unwrap().id, above_id);
        let uncached = preview_gradient(prepared(&content, layer, true));
        same_image(Some(&next.sprite), Some(&uncached.sprite));
        assert_ne!(first.sprite.as_bytes(0), next.sprite.as_bytes(0));
        assert_eq!(
            latest.completion(Some(latest), false),
            GradientCompletion::Publish
        );
        for current in [
            None,
            Some(GradientRequest { tab: 4, ..latest }),
            Some(GradientRequest {
                generation: 8,
                ..latest
            }),
            Some(GradientRequest {
                layer: LayerId(999),
                ..latest
            }),
            Some(GradientRequest {
                session: 12,
                ..latest
            }),
        ] {
            assert_eq!(
                first_request.completion(current, false),
                GradientCompletion::Discard
            );
        }
    }
}
