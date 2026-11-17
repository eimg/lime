use gpui::{Context, CursorStyle, Pixels, Point, Window, point};

use super::Workspace;
use super::canvas_tools::PreviewGeometry;
use super::layers::MovePaint;
use super::preview_backdrop::{Backdrop, PreviewContext};
use crate::commands::Tool;
use crate::document::{self, LayerId, MAX_NEW_EDGE};
use crate::preview::{Dirty, PixelRect};

const SCALE_SLOP: f32 = 6.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ScaleEdges {
    left: bool,
    right: bool,
    top: bool,
    bottom: bool,
}

#[derive(Clone, Copy)]
pub(super) struct ScaleDrag {
    layer: LayerId,
    original: PixelRect,
    baseline: PixelRect,
    edges: ScaleEdges,
    start: (i64, i64),
    target: PixelRect,
    pointer: bool,
    anchor: crate::transform::Anchor,
}

#[derive(Clone, Copy)]
struct ScalePreviewRequest {
    token: u64,
    tab: usize,
    layer: LayerId,
    generation: u64,
    target: PixelRect,
    resample: crate::transform::Resample,
}

impl ScalePreviewRequest {
    fn can_publish(self, current: Self, dirty: bool, dragging: bool) -> bool {
        self.token == current.token && self.tab == current.tab
            && self.layer == current.layer && self.generation == current.generation
            && self.resample == current.resample
            // Pointer motion may show completed same-session frames while newer
            // geometry renders. Settled/numeric edits require the latest request.
            && (dragging || (!dirty && self.target == current.target))
    }
}

impl ScaleDrag {
    fn begin_pointer(&mut self, edges: ScaleEdges, start: (i64, i64)) {
        self.baseline = self.target;
        self.edges = edges;
        self.start = start;
        self.pointer = true;
    }

    fn begin_numeric(&mut self, anchor: crate::transform::Anchor) {
        self.baseline = self.target;
        self.anchor = anchor;
    }

    fn numeric_target(
        &self,
        width: bool,
        value: u32,
        locked: bool,
    ) -> Result<PixelRect, &'static str> {
        let (w, h) = scale_dimensions(
            (self.baseline.width, self.baseline.height),
            width,
            value,
            locked,
        )
        .ok_or("Enter a size within the layer dimension and memory limits.")?;
        let offset = crate::transform::anchored_offset(
            (self.baseline.x as i32, self.baseline.y as i32),
            (self.baseline.width, self.baseline.height),
            (w, h),
            self.anchor,
        )
        .ok_or("That anchored position exceeds the layer coordinate limits.")?;
        Ok(PixelRect::new(
            i64::from(offset.0),
            i64::from(offset.1),
            w,
            h,
        ))
    }

    pub(super) fn changed(&self) -> bool {
        self.target != self.original
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScaleHit {
    Outside,
    Inside,
    Edge(ScaleEdges),
}

impl Workspace {
    pub(super) fn scale_frame(&self) -> Option<PixelRect> {
        if self.preferences.tool != Tool::ScaleLayer {
            return None;
        }
        self.scale_drag.map(|drag| drag.target)
    }

    pub(super) fn scale_size(&self) -> Option<(u32, u32)> {
        self.scale_frame().map(|frame| (frame.width, frame.height))
    }

    pub(super) fn begin_scale(
        &mut self,
        position: Point<Pixels>,
        scale: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.commit_numeric_edit(window, cx);
        if self.preferences.tool != Tool::ScaleLayer
            || self.scale_pointer_dragging()
            || self.active_tab().is_none_or(|tab| tab.busy())
        {
            return;
        }
        if self.scale_drag.is_none() && !self.begin_scale_numeric(window, cx) {
            return;
        }
        let Some(baseline) = self.scale_frame() else {
            return;
        };
        let Some((left, top, right, bottom)) = self.scale_screen_rect(baseline, scale) else {
            return;
        };
        let ScaleHit::Edge(edges) = hit_scale(
            left,
            top,
            right,
            bottom,
            f32::from(position.x),
            f32::from(position.y),
            SCALE_SLOP,
        ) else {
            return;
        };
        let Some(start) = self.unclamped_pointer_pixel(position, scale) else {
            return;
        };
        if let Some(drag) = self.scale_drag.as_mut() {
            drag.begin_pointer(edges, start);
            cx.notify();
        }
    }

    pub(super) fn begin_scale_numeric(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.preferences.tool != Tool::ScaleLayer || self.scale_pointer_dragging() {
            return false;
        }
        if let Some(drag) = self.scale_drag.as_mut() {
            drag.begin_numeric(self.preferences.scale_anchor);
            return true;
        }
        let Some(tab) = self.active_tab().filter(|tab| !tab.busy()) else {
            return false;
        };
        let Some(layer) = tab.document.content.layer(tab.active_layer) else {
            return false;
        };
        if layer.locks.position {
            self.notice("Unlock the layer's position before scaling it.", cx);
            return false;
        }
        if !layer.visible || layer.text.is_some() {
            self.notice(
                "Show the layer and rasterize editable text before scaling.",
                cx,
            );
            return false;
        }
        let original = PixelRect::new(
            i64::from(layer.offset.0),
            i64::from(layer.offset.1),
            layer.pixels.width(),
            layer.pixels.height(),
        );
        let drag = ScaleDrag {
            layer: layer.id,
            original,
            baseline: original,
            target: original,
            start: (0, 0),
            edges: ScaleEdges {
                left: false,
                right: false,
                top: false,
                bottom: false,
            },
            pointer: false,
            anchor: self.preferences.scale_anchor,
        };
        self.start_scale_preview(drag, window, cx);
        true
    }

    fn start_scale_preview(
        &mut self,
        drag: ScaleDrag,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.scale_backdrop.reset(window, cx);
        self.scale_token = self.scale_token.wrapping_add(1);
        self.scale_drag = Some(drag);
        self.request_scale_preview(window, cx);
    }

    pub(super) fn request_scale_preview(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.scale_drag.is_none() {
            return;
        }
        self.scale_preview_dirty = true;
        self.pump_scale_preview(window, cx);
        cx.notify();
    }

    pub(super) fn scale_preview_context(&self) -> Option<PreviewContext> {
        let layer = self.scale_drag?.layer;
        self.preview_context(self.scale_backdrop.epoch, Some(layer), layer)
            .filter(|context| context.layer == layer)
    }

    fn pump_scale_preview(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.scale_preview_busy || !self.scale_preview_dirty {
            return;
        }
        let Some(drag) = self.scale_drag else {
            return;
        };
        let Some(tab) = self.active_tab() else {
            return;
        };
        let Some(layer) = tab.document.content.layer(drag.layer).cloned() else {
            return;
        };
        let tab_id = tab.id;
        let generation = tab.history.generation();
        let Some(context) = self.scale_preview_context() else {
            return;
        };
        let backdrop = self.scale_backdrop.take(context, window, cx);
        let base = (self.scale_overlay.is_none() && backdrop.is_none())
            .then(|| self.active_tab().unwrap().document.content.clone());
        let target = drag.target;
        let resample = self.preferences.warp_resample;
        self.scale_preview_dirty = false;
        self.scale_preview_busy = true;
        self.scale_token = self.scale_token.wrapping_add(1);
        let token = self.scale_token;
        let request = ScalePreviewRequest {
            token,
            tab: tab_id,
            layer: drag.layer,
            generation,
            target,
            resample,
        };
        let job = cx.background_executor().spawn(async move {
            let shown = scale_preview_pixels(&layer, (target.width, target.height), resample);
            let backdrop = backdrop.or_else(|| {
                base.as_ref()
                    .map(|content| Backdrop::prepare(content, Some(drag.layer), drag.layer))
            });
            let (below, above) =
                backdrop.map_or((None, None), |backdrop| (backdrop.below, backdrop.above));
            MovePaint {
                below,
                above,
                sprite: document::preview_render(&shown),
                offset: (target.x as i32, target.y as i32),
                size: (target.width, target.height),
            }
        });
        cx.spawn_in(window, async move |workspace, cx| {
            let mut overlay = job.await;
            let _ = workspace.update_in(cx, |this, window, cx| {
                this.scale_preview_busy = false;
                let current = this
                    .scale_drag
                    .zip(this.active_tab())
                    .filter(|(drag, tab)| drag.layer == tab.active_layer)
                    .map(|(drag, tab)| ScalePreviewRequest {
                        token: this.scale_token,
                        tab: tab.id,
                        layer: drag.layer,
                        generation: tab.history.generation(),
                        target: drag.target,
                        resample: this.preferences.warp_resample,
                    });
                if let Some(current) = current.filter(|current| {
                    this.scale_preview_context() == Some(context)
                        && request.can_publish(
                            *current,
                            this.scale_preview_dirty,
                            this.scale_pointer_dragging(),
                        )
                }) {
                    // An interim sprite follows the latest frame, rather than
                    // pulling the frame back to the geometry of its old request.
                    overlay.offset = (current.target.x as i32, current.target.y as i32);
                    overlay.size = (current.target.width, current.target.height);
                    if let Some(previous) = this.scale_overlay.replace(overlay) {
                        // Keep the background partitions when only the sprite changed.
                        let previous_sprite = previous.sprite;
                        let overlay = this.scale_overlay.as_mut().unwrap();
                        if overlay.below.is_none() {
                            overlay.below = previous.below;
                        } else {
                            super::drop_images(previous.below, window, cx);
                        }
                        if overlay.above.is_none() {
                            overlay.above = previous.above;
                        } else {
                            super::drop_images(previous.above, window, cx);
                        }
                        super::drop_images(std::iter::once(previous_sprite), window, cx);
                    }
                } else {
                    let backdrop = Backdrop::take(&mut overlay);
                    let current = this.scale_preview_context();
                    let discarded = this.scale_backdrop.retain(
                        context,
                        current,
                        this.scale_overlay.is_some(),
                        backdrop,
                    );
                    super::drop_images(
                        discarded.into_iter().flat_map(Backdrop::into_images),
                        window,
                        cx,
                    );
                    super::drop_images(overlay.into_images(), window, cx);
                }
                if this.scale_preview_dirty {
                    this.pump_scale_preview(window, cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn extend_scale(
        &mut self,
        position: Point<Pixels>,
        scale: f32,
        shift: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(drag) = self.scale_drag.filter(|drag| drag.pointer) else {
            return;
        };
        let Some(pointer) = self.unclamped_pointer_pixel(position, scale) else {
            return;
        };
        let target = scale_target(
            drag.baseline,
            drag.edges,
            drag.start,
            pointer,
            self.preferences.scale_lock ^ shift,
        );
        self.update_scale_target(target, window, cx);
    }

    pub(super) fn scale_pointer_dragging(&self) -> bool {
        self.scale_drag.is_some_and(|drag| drag.pointer)
    }

    pub(super) fn end_scale_drag(&mut self, cx: &mut Context<Self>) {
        if let Some(drag) = self.scale_drag.as_mut() {
            drag.pointer = false;
        }
        cx.notify();
    }

    pub(super) fn can_apply_scale(&self) -> bool {
        self.active_tab().is_some_and(|tab| !tab.busy())
            && self.scale_drag.is_some_and(|drag| !drag.pointer)
    }

    pub(super) fn preview_scale_size(
        &mut self,
        width: bool,
        value: u32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(drag) = self.scale_drag.filter(|drag| !drag.pointer) else {
            return;
        };
        match drag.numeric_target(width, value, self.preferences.scale_lock) {
            Ok(target) => {
                if !self.update_scale_target(target, window, cx) {
                    self.restore_scale_size(window, cx);
                }
            }
            Err(message) => {
                self.restore_scale_size(window, cx);
                self.notice(message, cx);
            }
        }
    }

    pub(super) fn set_scale_anchor(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.commit_numeric_edit(window, cx);
        let Some(anchor) = crate::transform::Anchor::from_grid(index) else {
            return;
        };
        if !self.scale_pointer_dragging() && self.preferences.scale_anchor != anchor {
            self.preferences.scale_anchor = anchor;
            cx.notify();
        }
    }

    pub(super) fn restore_scale_size(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(drag) = self.scale_drag {
            self.update_scale_target(drag.baseline, window, cx);
        }
    }

    pub(super) fn preview_scale_to_canvas(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.begin_scale_numeric(window, cx) {
            return;
        }
        let Some(bounds) = self.active_tab().and_then(|tab| {
            tab.document
                .content
                .scale_to_canvas_bounds(tab.active_layer)
        }) else {
            return;
        };
        self.update_scale_target(
            PixelRect::new(
                i64::from(bounds.x),
                i64::from(bounds.y),
                bounds.width,
                bounds.height,
            ),
            window,
            cx,
        );
    }

    fn update_scale_target(
        &mut self,
        target: PixelRect,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(drag) = self.scale_drag else {
            return false;
        };
        if target == drag.target {
            return true;
        }
        if self.active_tab().is_none_or(|tab| {
            !crate::transform::canvas_allowed(target.width, target.height)
                || !tab
                    .document
                    .content
                    .accepts_replacement(drag.layer, (target.width, target.height))
        }) {
            self.notice("That size exceeds the layer or image memory limit.", cx);
            return false;
        }
        if let Some(drag) = self.scale_drag.as_mut() {
            drag.target = target;
        }
        if let Some(overlay) = self.scale_overlay.as_mut() {
            overlay.offset = (target.x as i32, target.y as i32);
            overlay.size = (target.width, target.height);
        }
        self.request_scale_preview(window, cx);
        true
    }

    pub(super) fn finish_scale(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.commit_numeric_edit(window, cx);
        if !self.can_apply_scale() {
            return;
        }
        let Some(drag) = self.scale_drag else {
            return;
        };
        self.scale_token = self.scale_token.wrapping_add(1);
        self.scale_preview_dirty = false;
        if drag.target == drag.original {
            self.cancel_scale(window, cx);
            cx.notify();
            return;
        }
        let resample = self.preferences.warp_resample;
        self.apply_preview(
            move |content, _, _| commit_scale(content, drag, resample),
            window,
            cx,
        );
        cx.notify();
    }

    pub(super) fn cancel_scale(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .numeric_edit
            .as_ref()
            .is_some_and(|edit| edit.field.is_scale())
        {
            self.numeric_edit = None;
        }
        self.scale_token = self.scale_token.wrapping_add(1);
        self.scale_drag = None;
        self.scale_backdrop.reset(window, cx);
        self.scale_preview_dirty = false;
        if let Some(overlay) = self.scale_overlay.take() {
            super::drop_images(overlay.into_images(), window, cx);
        }
        cx.notify();
    }

    pub(super) fn scale_cursor(&self, scale: f32) -> CursorStyle {
        if self.preferences.tool != Tool::ScaleLayer {
            return CursorStyle::Crosshair;
        }
        if let Some(drag) = self.scale_drag.filter(|drag| drag.pointer) {
            return drag.edges.cursor();
        }
        let Some(frame) = self.scale_frame() else {
            return CursorStyle::Crosshair;
        };
        let Some(position) = self.canvas_cursor.position.get() else {
            return CursorStyle::Crosshair;
        };
        let Some((left, top, right, bottom)) = self.scale_screen_rect(frame, scale) else {
            return CursorStyle::Crosshair;
        };
        match hit_scale(
            left,
            top,
            right,
            bottom,
            f32::from(position.x),
            f32::from(position.y),
            SCALE_SLOP,
        ) {
            ScaleHit::Edge(edges) => edges.cursor(),
            ScaleHit::Inside | ScaleHit::Outside => CursorStyle::Crosshair,
        }
    }

    fn scale_screen_rect(&self, frame: PixelRect, scale: f32) -> Option<(f32, f32, f32, f32)> {
        let tab = self.active_tab()?;
        let (width, height) = tab.document.content.size();
        if width == 0 || height == 0 || frame.width == 0 || frame.height == 0 {
            return None;
        }
        let geometry = PreviewGeometry::for_tab(tab, self.viewport.get(), scale)?;
        let start = geometry.project(point(
            frame.x as f32 / width as f32,
            frame.y as f32 / height as f32,
        ));
        let end = geometry.project(point(
            (frame.x + i64::from(frame.width)) as f32 / width as f32,
            (frame.y + i64::from(frame.height)) as f32 / height as f32,
        ));
        Some((
            f32::from(start.x.min(end.x)),
            f32::from(start.y.min(end.y)),
            f32::from(start.x.max(end.x)),
            f32::from(start.y.max(end.y)),
        ))
    }
}

const SCALE_PREVIEW_EDGE: u32 = 1024;

fn scale_preview_pixels(
    layer: &document::RasterLayer,
    target: (u32, u32),
    resample: crate::transform::Resample,
) -> image::RgbaImage {
    let edge = target.0.max(target.1).max(1);
    let size = if edge > SCALE_PREVIEW_EDGE {
        let scaled = |length: u32| {
            (u64::from(length) * u64::from(SCALE_PREVIEW_EDGE) / u64::from(edge)).max(1) as u32
        };
        (scaled(target.0), scaled(target.1))
    } else {
        target
    };
    let mut shown = crate::transform::scale_pixels(&layer.pixels, size.0, size.1, resample);
    if let Some(mask) = layer.mask.as_ref().filter(|mask| mask.enabled) {
        let mask = crate::transform::scale_pixels(&mask.pixels, size.0, size.1, resample);
        for (pixel, mask) in shown.pixels_mut().zip(mask.pixels()) {
            pixel[3] = ((u16::from(pixel[3]) * u16::from(mask[3]) + 127) / 255) as u8;
        }
    }
    super::layers::with_layer_opacity(&shown, layer.opacity)
}

fn commit_scale(
    content: &mut crate::document::DocumentContent,
    drag: ScaleDrag,
    resample: crate::transform::Resample,
) -> Dirty {
    if drag.target == drag.original
        || i32::try_from(drag.target.x).is_err()
        || i32::try_from(drag.target.y).is_err()
        || content.layer(drag.layer).is_none_or(|layer| {
            layer.locks.position
                || !layer.visible
                || layer.text.is_some()
                || layer.pixels.dimensions() != (drag.original.width, drag.original.height)
                || (i64::from(layer.offset.0), i64::from(layer.offset.1))
                    != (drag.original.x, drag.original.y)
        })
    {
        return Dirty::None;
    }
    if (drag.target.width, drag.target.height) != (drag.original.width, drag.original.height)
        && !content.scale_layer(drag.layer, drag.target.width, drag.target.height, resample)
    {
        return Dirty::None;
    }
    let _ = content.set_offset(drag.layer, (drag.target.x as i32, drag.target.y as i32));
    Dirty::Canvas
}

fn scale_target(
    original: PixelRect,
    edges: ScaleEdges,
    start: (i64, i64),
    pointer: (i64, i64),
    locked: bool,
) -> PixelRect {
    let dx = pointer.0.saturating_sub(start.0);
    let dy = pointer.1.saturating_sub(start.1);
    let mut left = original.x;
    let mut top = original.y;
    let mut right = original.x.saturating_add(i64::from(original.width));
    let mut bottom = original.y.saturating_add(i64::from(original.height));
    if edges.left {
        left = left.saturating_add(dx);
    }
    if edges.right {
        right = right.saturating_add(dx);
    }
    if edges.top {
        top = top.saturating_add(dy);
    }
    if edges.bottom {
        bottom = bottom.saturating_add(dy);
    }
    if left >= right {
        if edges.left {
            left = right - 1;
        } else {
            right = left + 1;
        }
    }
    if top >= bottom {
        if edges.top {
            top = bottom - 1;
        } else {
            bottom = top + 1;
        }
    }
    let mut width = right.saturating_sub(left).clamp(1, i64::from(MAX_NEW_EDGE)) as u32;
    let mut height = bottom.saturating_sub(top).clamp(1, i64::from(MAX_NEW_EDGE)) as u32;
    if locked {
        let horizontal = edges.left || edges.right;
        let vertical = edges.top || edges.bottom;
        let width_axis = horizontal
            && (!vertical
                || (width as f64 / original.width as f64 - 1.).abs()
                    >= (height as f64 / original.height as f64 - 1.).abs());
        // Clamp the common scale, rather than either dimension independently.
        let factor = if width_axis {
            width as f64 / original.width as f64
        } else {
            height as f64 / original.height as f64
        };
        let factor = factor.min(MAX_NEW_EDGE as f64 / original.width.max(original.height) as f64);
        width = (original.width as f64 * factor).round().max(1.) as u32;
        height = (original.height as f64 * factor).round().max(1.) as u32;
        if !horizontal {
            left = original.x + (i64::from(original.width) - i64::from(width)) / 2;
        }
        if !vertical {
            top = original.y + (i64::from(original.height) - i64::from(height)) / 2;
        }
    }
    if edges.left {
        left = right - i64::from(width);
    }
    if edges.top {
        top = bottom - i64::from(height);
    }
    PixelRect::new(
        left.clamp(i64::from(i32::MIN), i64::from(i32::MAX)),
        top.clamp(i64::from(i32::MIN), i64::from(i32::MAX)),
        width,
        height,
    )
}

fn scale_dimensions(
    original: (u32, u32),
    width_axis: bool,
    value: u32,
    locked: bool,
) -> Option<(u32, u32)> {
    if value == 0 || value > MAX_NEW_EDGE {
        return None;
    }
    let (width, height) = if locked {
        let factor = value as f64 / if width_axis { original.0 } else { original.1 } as f64;
        (
            (original.0 as f64 * factor).round().max(1.) as u32,
            (original.1 as f64 * factor).round().max(1.) as u32,
        )
    } else if width_axis {
        (value, original.1)
    } else {
        (original.0, value)
    };
    (width <= MAX_NEW_EDGE
        && height <= MAX_NEW_EDGE
        && crate::transform::canvas_allowed(width, height))
    .then_some((width, height))
}

fn hit_scale(left: f32, top: f32, right: f32, bottom: f32, x: f32, y: f32, slop: f32) -> ScaleHit {
    if x < left - slop || x > right + slop || y < top - slop || y > bottom + slop {
        return ScaleHit::Outside;
    }
    let mut near_left = (x - left).abs() <= slop;
    let mut near_right = (x - right).abs() <= slop;
    let mut near_top = (y - top).abs() <= slop;
    let mut near_bottom = (y - bottom).abs() <= slop;
    if near_left && near_right {
        near_left = x - left <= right - x;
        near_right = !near_left;
    }
    if near_top && near_bottom {
        near_top = y - top <= bottom - y;
        near_bottom = !near_top;
    }
    if near_left || near_right || near_top || near_bottom {
        return ScaleHit::Edge(ScaleEdges {
            left: near_left,
            right: near_right,
            top: near_top,
            bottom: near_bottom,
        });
    }
    ScaleHit::Inside
}

impl ScaleEdges {
    fn cursor(self) -> CursorStyle {
        match (self.left || self.right, self.top || self.bottom) {
            (true, true) if (self.left && self.top) || (self.right && self.bottom) => {
                CursorStyle::ResizeUpLeftDownRight
            }
            (true, true) => CursorStyle::ResizeUpRightDownLeft,
            (true, false) => CursorStyle::ResizeLeftRight,
            (false, true) => CursorStyle::ResizeUpDown,
            (false, false) => CursorStyle::Crosshair,
        }
    }
}

#[cfg(test)]
mod preview_publication_tests {
    use super::*;

    #[test]
    fn pointer_frames_progress_but_numeric_frames_never_reverse_to_old_requests() {
        let old = ScalePreviewRequest {
            token: 1,
            tab: 2,
            layer: crate::document::LayerId(3),
            generation: 4,
            target: PixelRect::new(0, 0, 100, 100),
            resample: crate::transform::Resample::Bilinear,
        };
        let mut latest = old;
        latest.target = PixelRect::new(0, 0, 200, 200);
        assert!(old.can_publish(latest, true, true));
        assert!(!old.can_publish(latest, true, false));
        assert!(latest.can_publish(latest, false, false));
        latest.token += 1;
        assert!(!old.can_publish(latest, true, true));
        latest = old;
        latest.tab += 1;
        assert!(!old.can_publish(latest, true, true));
        latest = old;
        latest.resample = crate::transform::Resample::Nearest;
        assert!(!old.can_publish(latest, true, true));
    }
}
