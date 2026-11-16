use gpui::{Action, Context, CursorStyle, Div, Pixels, Point, Window, point, prelude::*};

use super::Workspace;
use super::canvas_tools::PreviewGeometry;
use crate::commands::{
    ClearToTransparency, CropToSelection, InvertSelection, SelectAll, SelectNone, Tool,
};
use crate::document::CropRect;
use crate::preview::{Dirty, PixelRect};
use crate::selection::{Selection, SelectionOp, SelectionShape};

const HANDLE_SLOP: f32 = 6.;
const POLYGON_CLOSE_SLOP: f32 = 8.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MaskEdit {
    Feather,
    Grow,
    Shrink,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SelectionGesture {
    Create {
        origin: Point<Pixels>,
        anchor: (i64, i64),
        op: SelectionOp,
    },
    Move {
        rect: PixelRect,
        grab: (i64, i64),
    },
    Resize {
        rect: PixelRect,
        edges: SelectionEdges,
        anchor: (i64, i64),
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SelectionEdges {
    left: bool,
    right: bool,
    top: bool,
    bottom: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SelectionHit {
    Outside,
    Inside,
    Edge(SelectionEdges),
}

/// Latest requested ratio while a selection-only commit is in flight.
/// Its context must survive that exact commit before another edit can run.
#[derive(Clone, Copy)]
pub(super) struct PendingAspectRatio {
    tab: usize,
    layer: crate::document::LayerId,
    tool: Tool,
    generation: u64,
    crop: bool,
    ratio: Option<crate::constraints::AspectRatio>,
}

impl PendingAspectRatio {
    fn new(
        tab: &super::Tab,
        tool: Tool,
        crop: bool,
        ratio: Option<crate::constraints::AspectRatio>,
    ) -> Self {
        Self {
            tab: tab.id,
            layer: tab.active_layer,
            tool,
            generation: tab.history.generation(),
            crop,
            ratio,
        }
    }

    fn accepts(self, tab: &super::Tab, tool: Tool, generation: u64, published: bool) -> bool {
        self.tab == tab.id
            && self.layer == tab.active_layer
            && self.tool == tool
            && self.generation == generation
            && !tab.busy()
            && generation.checked_add(u64::from(published)) == Some(tab.history.generation())
    }
}

pub(super) struct PendingRadius {
    context: PendingAspectRatio,
    value: u32,
}

fn stepped_radius(value: u32, delta: i32, max: u32) -> u32 {
    (i64::from(value) + i64::from(delta)).clamp(0, i64::from(max)) as u32
}

impl Workspace {
    fn creation_feather_radius(&self) -> u32 {
        if self.preferences.selection_feather_enabled {
            self.preferences.selection_feather_radius
        } else {
            0
        }
    }

    pub(super) fn active_selection_radius(&self) -> u32 {
        if let Some(pending) = &self.pending_radius {
            return pending.value;
        }
        match self
            .selection_preview
            .as_ref()
            .or_else(|| {
                self.active_tab()
                    .and_then(|tab| tab.pending_canvas())
                    .and_then(|pending| pending.selection.as_ref())
            })
            .or_else(|| self.active_tab().map(|tab| &tab.selection))
        {
            Some(selection) if selection.feather_geometry().is_some() => {
                selection.feather_geometry().unwrap().radius
            }
            Some(Selection::RoundedRect(shape)) => shape.radius(),
            Some(Selection::Rect(_)) => 0,
            _ => self.preferences.selection_radius,
        }
    }

    pub(super) fn effective_selection_ratio(&self) -> Option<crate::constraints::AspectRatio> {
        if self.preferences.tool == Tool::OvalSelect && self.preferences.oval_circle {
            Some(crate::constraints::AspectRatio::SQUARE)
        } else {
            self.preferences.selection_ratio
        }
    }

    pub(super) fn active_selection_antialias(&self) -> bool {
        self.active_tab()
            .filter(|tab| {
                tab.selection.feather_geometry().is_some()
                    || matches!(
                        tab.selection,
                        Selection::Ellipse(_) | Selection::RoundedRect(_)
                    )
            })
            .map_or(self.preferences.selection_antialias, |tab| {
                tab.selection.is_antialiased()
            })
    }

    pub(super) fn set_aspect_ratio(
        &mut self,
        crop: bool,
        ratio: Option<crate::constraints::AspectRatio>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // A typed selection edit may itself start the commit we must wait for.
        self.commit_numeric_edit(window, cx);
        if let Some(tab) = self.active_tab().filter(|tab| tab.editing) {
            self.pending_aspect_ratio = Some(PendingAspectRatio::new(
                tab,
                self.preferences.tool,
                crop,
                ratio,
            ));
            return;
        }
        if self.active_tab().is_some_and(|tab| tab.busy()) {
            return;
        }
        self.pending_aspect_ratio = None;
        if crop {
            self.preferences.crop_ratio = ratio;
            if let Some(ratio) = ratio {
                self.preferences.crop_ratio_value = ratio;
                self.crop = self.crop.map(|crop| ratio_crop(crop, ratio));
            }
        } else {
            self.preferences.oval_circle = false;
            self.preferences.selection_ratio = ratio;
            if let Some(ratio) = ratio {
                self.preferences.selection_ratio_value = ratio;
                if self.selection_shape_rect().is_some() {
                    self.update_selection(
                        move |current, size| refit_selection(current, ratio, size),
                        window,
                        cx,
                    );
                }
            }
        }
        cx.notify();
    }

    pub(super) fn finish_pending_aspect_ratio(
        &mut self,
        id: usize,
        generation: u64,
        change: super::Change,
        published: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .pending_aspect_ratio
            .as_ref()
            .is_none_or(|pending| pending.tab != id)
        {
            return;
        }
        let pending = self.pending_aspect_ratio.take().unwrap();
        if !matches!(change, super::Change::Selection)
            || self.form.is_some()
            || self.dialog_open
            || self.numeric_edit.is_some()
            || self.active_tab().is_none_or(|tab| {
                !pending.accepts(tab, self.preferences.tool, generation, published)
            })
        {
            return;
        }
        self.set_aspect_ratio(pending.crop, pending.ratio, window, cx);
    }

    pub(super) fn set_selection_antialias(
        &mut self,
        antialias: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.preferences.selection_antialias = antialias;
        let Some(tab) = self.active_tab().filter(|tab| !tab.busy()) else {
            cx.notify();
            return;
        };
        let _ = tab;
        self.update_selection(
            move |selection, size| selection.with_antialias(antialias, size.0, size.1),
            window,
            cx,
        );
        cx.notify();
    }

    pub(super) fn clear_selection_gesture(&mut self) {
        self.pending_aspect_ratio = None;
        self.pending_radius = None;
        if self
            .numeric_edit
            .as_ref()
            .is_some_and(|edit| edit.field.is_selection())
        {
            self.numeric_edit = None;
        }
        self.selection_drag = None;
        self.selection_preview = None;
        self.clear_free_draft();
    }

    pub(super) fn begin_selection(
        &mut self,
        position: Point<Pixels>,
        scale: f32,
        op: SelectionOp,
        cx: &mut Context<Self>,
    ) {
        if self.active_tab().is_none_or(|tab| tab.busy()) {
            return;
        }
        self.clear_selection_gesture();
        let Some(selection) = self.active_tab().map(|tab| tab.selection.clone()) else {
            return;
        };
        let Some(anchor) = self.unclamped_pointer_pixel(position, scale) else {
            return;
        };
        let gesture =
            if let Some(rect) = adjustable_rect(selection).filter(|_| op == SelectionOp::Replace) {
                let hit = self
                    .selection_screen_rect(rect, scale)
                    .map(|(left, top, right, bottom)| {
                        hit_selection(
                            left,
                            top,
                            right,
                            bottom,
                            f32::from(position.x),
                            f32::from(position.y),
                            HANDLE_SLOP,
                        )
                    })
                    .unwrap_or(SelectionHit::Outside);
                match hit {
                    SelectionHit::Edge(edges) => {
                        let Some(anchor) = self.unclamped_pointer_pixel(position, scale) else {
                            return;
                        };
                        SelectionGesture::Resize {
                            rect,
                            edges,
                            anchor,
                        }
                    }
                    SelectionHit::Inside => {
                        let pixel = self
                            .pointer_pixel(position, scale)
                            .unwrap_or((rect.x, rect.y));
                        SelectionGesture::Move {
                            rect,
                            grab: (pixel.0 - rect.x, pixel.1 - rect.y),
                        }
                    }
                    SelectionHit::Outside => SelectionGesture::Create {
                        origin: position,
                        anchor,
                        op,
                    },
                }
            } else {
                SelectionGesture::Create {
                    origin: position,
                    anchor,
                    op,
                }
            };
        self.selection_drag = Some(gesture);
        cx.notify();
    }

    pub(super) fn extend_selection(
        &mut self,
        position: Point<Pixels>,
        scale: f32,
        cx: &mut Context<Self>,
    ) {
        let Some(drag) = self.selection_drag else {
            return;
        };
        let Some(tab) = self.active_tab() else {
            return;
        };
        let (width, height) = tab.document.content.size();
        let previous = tab.selection.clone();
        let pointer = self.pointer_pixel(position, scale);
        let preview = match drag {
            SelectionGesture::Create {
                origin,
                anchor: start,
                ..
            } => {
                let delta = position - origin;
                if f32::from(delta.x).abs().max(f32::from(delta.y).abs()) < 4. {
                    None
                } else {
                    let shape = if self.preferences.tool == Tool::Select {
                        SelectionShape::Rectangle
                    } else if self.preferences.oval_circle {
                        SelectionShape::Circle
                    } else {
                        SelectionShape::Ellipse
                    };
                    let radius = self.preferences.selection_radius;
                    self.unclamped_pointer_pixel(position, scale).map(|end| {
                        Selection::shape_drag(
                            shape,
                            start,
                            end,
                            (width, height),
                            radius,
                            self.preferences.selection_antialias,
                            self.effective_selection_ratio(),
                        )
                    })
                }
            }
            SelectionGesture::Move { rect, grab } => pointer.map(|pixel| {
                placed_selection(
                    previous,
                    move_rect(rect, pixel, grab, width, height),
                    width,
                    height,
                )
            }),
            SelectionGesture::Resize {
                rect,
                edges,
                anchor,
            } => self.unclamped_pointer_pixel(position, scale).map(|pixel| {
                let delta = (
                    pixel.0.saturating_sub(anchor.0),
                    pixel.1.saturating_sub(anchor.1),
                );
                placed_selection(
                    previous,
                    self.effective_selection_ratio().map_or_else(
                        || resize_rect(rect, edges, delta, width, height),
                        |ratio| {
                            ratio.resize(
                                rect,
                                (edges.left, edges.right, edges.top, edges.bottom),
                                delta,
                                (width, height),
                            )
                        },
                    ),
                    width,
                    height,
                )
            }),
        };
        if self.selection_preview != preview {
            self.selection_preview = preview;
            cx.notify();
        }
    }

    pub(super) fn finish_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let gesture = self.selection_drag.take();
        let preview = self.selection_preview.clone();
        match (gesture, preview) {
            (Some(SelectionGesture::Create { op, .. }), Some(shape)) => {
                let feather = self.creation_feather_radius();
                self.update_selection(
                    move |current, size| {
                        combine_created_selection(current, shape, op, size, feather)
                    },
                    window,
                    cx,
                );
            }
            // A click outside a replace selection deselects.
            (
                Some(SelectionGesture::Create {
                    op: SelectionOp::Replace,
                    ..
                }),
                None,
            ) => {
                self.select_none(window, cx);
            }
            (Some(_), Some(selection)) => self.commit_placed_selection(selection, window, cx),
            (Some(_), None) => cx.notify(),
            (None, _) => {
                self.selection_preview = None;
            }
        }
    }

    pub(super) fn selection_cursor(&self, scale: f32) -> CursorStyle {
        if let Some(gesture) = self.selection_drag {
            return match gesture {
                SelectionGesture::Create { .. } => CursorStyle::Crosshair,
                SelectionGesture::Move { .. } => CursorStyle::ClosedHand,
                SelectionGesture::Resize { edges, .. } => edges.cursor(),
            };
        }
        let Some(position) = self.canvas_cursor.position.get() else {
            return CursorStyle::Crosshair;
        };
        let Some(selection) = self.active_tab().map(|tab| tab.selection.clone()) else {
            return CursorStyle::Crosshair;
        };
        let Some(rect) = adjustable_rect(selection) else {
            return CursorStyle::Crosshair;
        };
        let Some((left, top, right, bottom)) = self.selection_screen_rect(rect, scale) else {
            return CursorStyle::Crosshair;
        };
        match hit_selection(
            left,
            top,
            right,
            bottom,
            f32::from(position.x),
            f32::from(position.y),
            HANDLE_SLOP,
        ) {
            SelectionHit::Inside => CursorStyle::OpenHand,
            SelectionHit::Edge(edges) => edges.cursor(),
            SelectionHit::Outside => CursorStyle::Crosshair,
        }
    }

    fn pointer_pixel(&self, position: Point<Pixels>, scale: f32) -> Option<(i64, i64)> {
        let (x, y) = self.unclamped_pointer_pixel(position, scale)?;
        let (width, height) = self.active_document()?.content.size();
        Some((
            x.clamp(0, i64::from(width) - 1),
            y.clamp(0, i64::from(height) - 1),
        ))
    }

    pub(super) fn unclamped_pointer_pixel(
        &self,
        position: Point<Pixels>,
        scale: f32,
    ) -> Option<(i64, i64)> {
        let tab = self.active_tab()?;
        let geometry = PreviewGeometry::for_tab(tab, self.viewport.get(), scale)?;
        let fraction = geometry.fraction(position);
        Some((
            fraction_pixel(fraction.x, tab.document.width()),
            fraction_pixel(fraction.y, tab.document.height()),
        ))
    }

    fn selection_screen_rect(&self, rect: PixelRect, scale: f32) -> Option<(f32, f32, f32, f32)> {
        let tab = self.active_tab()?;
        let (width, height) = tab.document.content.size();
        if width == 0 || height == 0 {
            return None;
        }
        let geometry = PreviewGeometry::for_tab(tab, self.viewport.get(), scale)?;
        let start = geometry.project(point(
            rect.x as f32 / width as f32,
            rect.y as f32 / height as f32,
        ));
        let end = geometry.project(point(
            rect.right() as f32 / width as f32,
            rect.bottom() as f32 / height as f32,
        ));
        Some((
            f32::from(start.x.min(end.x)),
            f32::from(start.y.min(end.y)),
            f32::from(start.x.max(end.x)),
            f32::from(start.y.max(end.y)),
        ))
    }

    pub(super) fn select_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.commit_selection(Selection::Full, window, cx);
    }

    pub(super) fn select_none(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.commit_selection(Selection::Unrestricted, window, cx);
    }

    pub(super) fn invert_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab() else {
            return;
        };
        let (width, height) = tab.document.content.size();
        let selection = tab.selection.clone().invert(width, height);
        self.commit_selection(selection, window, cx);
    }

    pub(super) fn clear_to_transparency(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.refuse_text_pixels(false, cx) {
            return;
        }
        let Some(tab) = self.active.map(|index| &mut self.tabs[index]) else {
            return;
        };
        if tab.selection == Selection::Empty {
            tab.notice = Some("Nothing is selected.".into());
            cx.notify();
            return;
        }
        let layer = tab.active_layer;
        if tab
            .document
            .content
            .layer(layer)
            .is_some_and(|layer| !layer.visible)
        {
            tab.notice = Some("The active layer is hidden. Show it before clearing.".into());
            cx.notify();
            return;
        }
        self.edit_content(
            move |content, _, selection| {
                content
                    .clear(layer, selection)
                    .map(Dirty::Region)
                    .unwrap_or(Dirty::None)
            },
            window,
            cx,
        );
    }

    pub(super) fn crop_to_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.refuse_text_pixels(true, cx) {
            return;
        }
        let Some(bounds) = self.selection_crop() else {
            return;
        };
        let crop = CropRect {
            x: bounds.x as u32,
            y: bounds.y as u32,
            width: bounds.width,
            height: bounds.height,
        };
        self.edit_content(
            move |content, _, selection| {
                if content.crop(crop) {
                    *selection = selection.clone().rebase(
                        i64::from(crop.x),
                        i64::from(crop.y),
                        crop.width,
                        crop.height,
                    );
                    Dirty::Canvas
                } else {
                    Dirty::None
                }
            },
            window,
            cx,
        );
    }

    pub(super) fn bind_selection_actions(&self, root: Div, cx: &Context<Self>) -> Div {
        if self.text_draft.is_some() {
            return root
                .on_action(cx.listener(|this, _: &SelectAll, _, cx| this.select_all_text(cx)));
        }
        let root = self.bind_selection::<SelectAll>(root, SelectionCommand::All, cx);
        let root = self.bind_selection::<SelectNone>(root, SelectionCommand::None, cx);
        let root = self.bind_selection::<InvertSelection>(root, SelectionCommand::Invert, cx);
        let root = self.bind_selection::<ClearToTransparency>(root, SelectionCommand::Clear, cx);
        self.bind_selection::<CropToSelection>(root, SelectionCommand::Crop, cx)
    }

    fn bind_selection<A: Action>(
        &self,
        root: Div,
        command: SelectionCommand,
        cx: &Context<Self>,
    ) -> Div {
        if !self.selection_command_enabled(command) {
            return root;
        }
        root.on_action(cx.listener(move |this, _: &A, window, cx| match command {
            SelectionCommand::All => this.select_all(window, cx),
            SelectionCommand::None => this.select_none(window, cx),
            SelectionCommand::Invert => this.invert_selection(window, cx),
            SelectionCommand::Clear => this.clear_to_transparency(window, cx),
            SelectionCommand::Crop => this.crop_to_selection(window, cx),
        }))
    }

    pub(super) fn selection_command_enabled(&self, command: SelectionCommand) -> bool {
        let Some(tab) = self.active_tab() else {
            return false;
        };
        if tab.busy() {
            return false;
        }
        match command {
            SelectionCommand::All | SelectionCommand::None | SelectionCommand::Invert => true,
            SelectionCommand::Clear => {
                tab.selection != Selection::Empty
                    && tab
                        .document
                        .content
                        .layer(tab.active_layer)
                        .is_some_and(|layer| !layer.locks.alpha)
            }
            SelectionCommand::Crop => self.selection_crop().is_some(),
        }
    }

    fn selection_crop(&self) -> Option<PixelRect> {
        let tab = self.active_tab()?;
        let (width, height) = tab.document.content.size();
        tab.selection.crop_bounds(width, height)
    }

    pub(super) fn commit_placed_selection(
        &mut self,
        selection: Selection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.active_tab().is_none_or(|tab| tab.busy()) {
            return;
        }
        self.selection_preview = Some(selection.clone());
        self.update_selection(
            move |current, size| {
                if let Some(geometry) = current.feather_geometry() {
                    selection.feather(geometry.feather, size.0, size.1)
                } else {
                    selection
                }
            },
            window,
            cx,
        );
    }

    pub(super) fn commit_selection(
        &mut self,
        selection: Selection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_selection(move |_, _| selection, window, cx);
    }

    fn update_selection(
        &mut self,
        update: impl FnOnce(Selection, (u32, u32)) -> Selection + Send + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.start_change(
            super::Change::Selection,
            move |content, _, current| {
                let selection = update(current.clone(), content.size());
                if *current == selection {
                    return Dirty::None;
                }
                *current = selection;
                Dirty::Unpainted
            },
            window,
            cx,
        );
    }

    pub(super) fn begin_free(
        &mut self,
        position: Point<Pixels>,
        modifiers: &gpui::Modifiers,
        scale: f32,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.active_tab().is_none_or(|tab| tab.busy()) {
            return;
        }
        let Some(tab) = self.active_tab() else {
            return;
        };
        let Some(geometry) = PreviewGeometry::for_tab(tab, self.viewport.get(), scale) else {
            return;
        };
        if self.preferences.tool != Tool::FreeSelect {
            return;
        }
        let point = geometry.fraction(position);
        self.free_drag_origin = Some(position);
        self.free_dragging = false;
        self.free_node_drag =
            free_node_at(&self.free_points, &self.free_nodes, &geometry, position)
                .map(|index| (index, self.free_points[index]));
        if self.free_node_drag.is_some() {
            cx.notify();
            return;
        }
        if self.free_points.is_empty() {
            self.free_op = selection_op(modifiers);
        }
        if self.free_points.last() != Some(&point) {
            self.free_nodes.push(self.free_points.len());
            self.free_points.push(point);
        }
        cx.notify();
    }

    pub(super) fn extend_free(
        &mut self,
        position: Point<Pixels>,
        scale: f32,
        cx: &mut Context<Self>,
    ) {
        if self.preferences.tool != Tool::FreeSelect || self.free_points.is_empty() {
            return;
        }
        let Some(origin) = self.free_drag_origin else {
            return;
        };
        let Some(geometry) = self
            .active_tab()
            .and_then(|tab| PreviewGeometry::for_tab(tab, self.viewport.get(), scale))
        else {
            return;
        };
        if !self.free_dragging {
            let delta = position - origin;
            if f32::from(delta.x).abs().max(f32::from(delta.y).abs()) < 3. {
                return;
            }
            self.free_dragging = true;
        }
        if let Some((index, original)) = self.free_node_drag {
            // Preserve the grab offset and derive every frame from the gesture baseline.
            let delta = geometry.fraction(position) - geometry.fraction(origin);
            self.free_points[index] = original + delta;
        } else {
            let point = geometry.fraction(position);
            if self.free_points.last() != Some(&point) {
                self.free_points.push(point);
            }
        }
        cx.notify();
    }

    pub(super) fn finish_free(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.preferences.tool != Tool::FreeSelect || self.free_points.is_empty() {
            return;
        }
        if self.free_drag_origin.is_none() {
            return;
        }
        self.extend_free(position, window.scale_factor(), cx);
        let close = self.free_node_drag.is_some_and(|(index, _)| index == 0)
            && !self.free_dragging
            && self.free_points.len() >= 3;
        if self.free_node_drag.is_none() {
            let last = self.free_points.len() - 1;
            if self.free_nodes.last() != Some(&last) {
                self.free_nodes.push(last);
            }
        }
        self.free_node_drag = None;
        self.free_drag_origin = None;
        self.free_dragging = false;
        if close {
            self.close_free(window, cx);
        }
        cx.notify();
    }

    pub(super) fn close_free(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.free_points.len() < 3 || self.active_tab().is_none_or(|tab| tab.busy()) {
            return;
        }
        let points = self.free_points.clone();
        let op = self.free_op;
        let antialias = self.preferences.free_antialias;
        let feather = self.creation_feather_radius();
        self.update_selection(
            move |current, (width, height)| {
                let points: Vec<_> = points
                    .into_iter()
                    .map(|point| {
                        (
                            f64::from(point.x) * f64::from(width),
                            f64::from(point.y) * f64::from(height),
                        )
                    })
                    .collect();
                let shape = Selection::polygon_quality(&points, width, height, antialias);
                if shape == Selection::Empty {
                    current
                } else {
                    combine_created_selection(current, shape, op, (width, height), feather)
                }
            },
            window,
            cx,
        );
    }

    pub(super) fn clear_free_draft(&mut self) {
        self.free_points.clear();
        self.free_nodes.clear();
        self.free_node_drag = None;
        self.free_drag_origin = None;
        self.free_dragging = false;
    }

    pub(super) fn cancel_free(&mut self, cx: &mut Context<Self>) {
        if self.free_points.is_empty() {
            return;
        }
        self.clear_free_draft();
        cx.notify();
    }

    pub(super) fn nudge_region_match(
        &mut self,
        wand: bool,
        gap: bool,
        delta: i16,
        cx: &mut Context<Self>,
    ) {
        let matching = if wand {
            &mut self.preferences.wand_match
        } else {
            &mut self.preferences.fill_match
        };
        let (value, max) = if gap {
            (&mut matching.gap_radius, crate::selection::MAX_GAP_RADIUS)
        } else {
            (&mut matching.tolerance, u8::MAX)
        };
        *value = (i16::from(*value) + delta).clamp(0, i16::from(max)) as u8;
        cx.notify();
    }

    pub(super) fn wand_at(
        &mut self,
        position: Point<Pixels>,
        modifiers: &gpui::Modifiers,
        scale: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((x, y)) = self.canvas_pixel(position, scale, false) else {
            return;
        };
        let Some(tab) = self.active_tab() else {
            return;
        };
        if tab.busy() {
            return;
        }
        let sample_all = self.preferences.wand_sample_all;
        if !sample_all
            && wand_source(&tab.document.content, tab.active_layer, false, (x, y)).is_none()
        {
            self.notice("The active layer does not cover that pixel.", cx);
            return;
        }
        let content = tab.document.content.clone();
        let layer = tab.active_layer;
        let op = selection_op(modifiers);
        let matching = self.preferences.wand_match;
        let antialias = self.preferences.wand_antialias;
        let feather = self.creation_feather_radius();
        self.update_selection(
            move |current, size| {
                let Some(source) = wand_source(&content, layer, sample_all, (x, y)) else {
                    return current;
                };
                let shape = Selection::flood_region_quality(
                    &source.pixels,
                    source.point,
                    matching,
                    source.offset,
                    size,
                    None,
                    antialias,
                );
                combine_created_selection(current, shape, op, size, feather)
            },
            window,
            cx,
        );
    }

    pub(super) fn reshape_selection(
        &mut self,
        edit: MaskEdit,
        radius: u32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.active_tab() else {
            return;
        };
        if tab.busy() {
            return;
        }
        if matches!(tab.selection, Selection::Unrestricted | Selection::Empty) {
            self.notice("Select part of the image first.", cx);
            return;
        }
        self.update_selection(
            move |current, (width, height)| match edit {
                MaskEdit::Feather => current.feather(radius, width, height),
                MaskEdit::Grow => current.grow(radius, width, height),
                MaskEdit::Shrink => current.shrink(radius, width, height),
            },
            window,
            cx,
        );
    }

    pub(super) fn nudge_selection_radius(
        &mut self,
        delta: i32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.commit_numeric_edit(window, cx);
        if self.active_tab().is_some_and(|tab| {
            tab.busy()
                && tab
                    .pending_canvas()
                    .is_none_or(|pending| pending.selection.is_none())
        }) {
            return;
        }
        let max = self.selection_shape_rect().map_or_else(
            || {
                self.active_document().map_or(1, |document| {
                    let (width, height) = document.content.size();
                    width.max(height)
                })
            },
            |rect| rect.width.min(rect.height) / 2,
        );
        let max = max.min(i32::MAX as u32) as i32;
        let value = stepped_radius(self.active_selection_radius(), delta, max as u32);
        if let Some(tab) = self.active_tab().filter(|tab| tab.editing) {
            self.pending_radius = Some(PendingRadius {
                context: PendingAspectRatio::new(tab, self.preferences.tool, false, None),
                value,
            });
            cx.notify();
            return;
        }
        self.preferences.selection_radius = value;
        let selection = self
            .selection_preview
            .clone()
            .or_else(|| self.active_tab().map(|tab| tab.selection.clone()));
        if let Some(rect) = selection.as_ref().and_then(rect_for_radius) {
            let Some(tab) = self.active_tab() else {
                return;
            };
            let shape = editable_selection_for_rect(
                selection.as_ref().unwrap(),
                rect,
                Some(self.preferences.selection_radius),
                self.preferences.selection_antialias,
                tab.document.content.size(),
            );
            if let Some(shape) = shape {
                self.commit_placed_selection(shape, window, cx);
            }
        }
        cx.notify();
    }

    pub(super) fn finish_pending_radius(
        &mut self,
        id: usize,
        generation: u64,
        change: super::Change,
        published: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .pending_radius
            .as_ref()
            .is_none_or(|pending| pending.context.tab != id)
        {
            return;
        }
        let pending = self.pending_radius.take().unwrap();
        if !matches!(change, super::Change::Selection)
            || self.form.is_some()
            || self.dialog_open
            || self.numeric_edit.is_some()
            || self.active_tab().is_none_or(|tab| {
                !pending
                    .context
                    .accepts(tab, self.preferences.tool, generation, published)
            })
        {
            return;
        }
        let delta = i64::from(pending.value) - i64::from(self.active_selection_radius());
        self.nudge_selection_radius(
            delta.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
            window,
            cx,
        );
    }

    pub(super) fn selection_shape_rect(&self) -> Option<PixelRect> {
        let selection = self
            .selection_preview
            .as_ref()
            .or_else(|| {
                self.active_tab()
                    .and_then(|tab| tab.pending_canvas())
                    .and_then(|pending| pending.selection.as_ref())
            })
            .or_else(|| self.active_tab().map(|tab| &tab.selection))?;
        if let Some(geometry) = selection.feather_geometry() {
            return Some(geometry.rect);
        }
        match selection {
            Selection::Rect(rect) => Some(*rect),
            Selection::Ellipse(ellipse) => Some(ellipse.rect()),
            Selection::RoundedRect(shape) => Some(shape.rect()),
            _ => None,
        }
    }
}

struct WandSource {
    pixels: std::sync::Arc<image::RgbaImage>,
    offset: (i32, i32),
    point: (u32, u32),
}

fn combine_created_selection(
    current: Selection,
    incoming: Selection,
    op: SelectionOp,
    size: (u32, u32),
    feather: u32,
) -> Selection {
    // Only soften the new region. Repeated additions must not blur old coverage.
    current.combine(
        incoming.feather(feather, size.0, size.1),
        op,
        size.0,
        size.1,
    )
}

fn wand_source(
    content: &crate::document::DocumentContent,
    layer: crate::document::LayerId,
    sample_all: bool,
    point: (u32, u32),
) -> Option<WandSource> {
    if sample_all {
        return Some(WandSource {
            pixels: content.composite(),
            offset: (0, 0),
            point,
        });
    }
    let layer = content.layer(layer)?;
    let x = i64::from(point.0) - i64::from(layer.offset.0);
    let y = i64::from(point.1) - i64::from(layer.offset.1);
    if x < 0
        || y < 0
        || x >= i64::from(layer.pixels.width())
        || y >= i64::from(layer.pixels.height())
    {
        return None;
    }
    Some(WandSource {
        pixels: layer.pixels.clone(),
        offset: layer.offset,
        point: (x as u32, y as u32),
    })
}

pub(super) fn editable_selection_for_rect(
    current: &Selection,
    rect: PixelRect,
    radius: Option<u32>,
    default_antialias: bool,
    size: (u32, u32),
) -> Option<Selection> {
    if let Some(mut geometry) = current.feather_geometry() {
        if !geometry.ellipse
            && let Some(radius) = radius
        {
            geometry.radius = radius.min(rect.width.min(rect.height) / 2);
        }
        return Some(geometry.shape(rect, size.0, size.1));
    }
    match current {
        Selection::Rect(_) | Selection::RoundedRect(_) if radius.is_some() => {
            Some(Selection::rounded_rect_quality(
                rect,
                radius?,
                size.0,
                size.1,
                if matches!(current, Selection::RoundedRect(_)) {
                    current.is_antialiased()
                } else {
                    default_antialias
                },
            ))
        }
        Selection::Rect(_) | Selection::RoundedRect(_) | Selection::Ellipse(_) => {
            Some(placed_selection(current.clone(), rect, size.0, size.1))
        }
        _ => None,
    }
}

pub(super) fn ratio_crop(crop: CropRect, ratio: crate::constraints::AspectRatio) -> CropRect {
    let rect = ratio.inscribe(PixelRect::new(
        i64::from(crop.x),
        i64::from(crop.y),
        crop.width,
        crop.height,
    ));
    CropRect {
        x: rect.x as u32,
        y: rect.y as u32,
        width: rect.width,
        height: rect.height,
    }
}

fn refit_selection(
    current: Selection,
    ratio: crate::constraints::AspectRatio,
    size: (u32, u32),
) -> Selection {
    let rect = current
        .feather_geometry()
        .map(|geometry| geometry.rect)
        .or_else(|| match &current {
            Selection::Rect(rect) => Some(*rect),
            Selection::Ellipse(shape) => Some(shape.rect()),
            Selection::RoundedRect(shape) => Some(shape.rect()),
            _ => None,
        });
    let Some(rect) = rect else {
        return current;
    };
    let next = editable_selection_for_rect(
        &current,
        ratio.inscribe(rect),
        None,
        current.is_antialiased(),
        size,
    );
    match next {
        Some(next) => {
            if let Some(geometry) = current.feather_geometry() {
                next.feather(geometry.feather, size.0, size.1)
            } else {
                next
            }
        }
        None => current,
    }
}

fn rect_for_radius(selection: &Selection) -> Option<PixelRect> {
    if let Some(geometry) = selection
        .feather_geometry()
        .filter(|geometry| !geometry.ellipse)
    {
        return Some(geometry.rect);
    }
    match selection {
        Selection::Rect(rect) => Some(*rect),
        Selection::RoundedRect(shape) => Some(shape.rect()),
        _ => None,
    }
}

pub(super) fn selection_op(modifiers: &gpui::Modifiers) -> SelectionOp {
    match (modifiers.shift, modifiers.control) {
        (true, true) => SelectionOp::Intersect,
        (true, false) => SelectionOp::Add,
        (false, true) => SelectionOp::Subtract,
        (false, false) => SelectionOp::Replace,
    }
}

pub(super) fn free_node_at(
    points: &[Point<f32>],
    nodes: &[usize],
    geometry: &PreviewGeometry,
    pointer: Point<Pixels>,
) -> Option<usize> {
    nodes
        .iter()
        .copied()
        .filter_map(|index| {
            let delta = geometry.project(*points.get(index)?) - pointer;
            let distance = f32::from(delta.x).hypot(f32::from(delta.y));
            (distance <= POLYGON_CLOSE_SLOP).then_some((index, distance))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(index, _)| index)
}

pub(super) fn near_polygon_start(origin: Point<Pixels>, pointer: Point<Pixels>) -> bool {
    let delta = pointer - origin;
    f32::from(delta.x).hypot(f32::from(delta.y)) <= POLYGON_CLOSE_SLOP
}

pub(super) fn adjustable_rect(selection: Selection) -> Option<PixelRect> {
    if let Some(geometry) = selection.feather_geometry() {
        return Some(geometry.rect);
    }
    match selection {
        Selection::Rect(rect) | Selection::Inverse(rect) if rect.width > 0 && rect.height > 0 => {
            Some(rect)
        }
        Selection::Ellipse(ellipse) if ellipse.rect().width > 0 && ellipse.rect().height > 0 => {
            Some(ellipse.rect())
        }
        Selection::RoundedRect(shape) if shape.rect().width > 0 && shape.rect().height > 0 => {
            Some(shape.rect())
        }
        _ => None,
    }
}

pub(super) fn placed_selection(
    previous: Selection,
    rect: PixelRect,
    width: u32,
    height: u32,
) -> Selection {
    if let Some(geometry) = previous.feather_geometry() {
        return geometry.shape(rect, width, height);
    }
    let covers = width > 0
        && height > 0
        && rect.x <= 0
        && rect.y <= 0
        && rect.right() >= i64::from(width)
        && rect.bottom() >= i64::from(height);
    match previous {
        Selection::Inverse(_) => {
            if covers {
                Selection::Empty
            } else {
                Selection::Inverse(rect)
            }
        }
        Selection::Ellipse(shape) => Selection::ellipse_rect_quality(
            rect,
            width,
            height,
            Selection::Ellipse(shape).is_antialiased(),
        ),
        Selection::RoundedRect(shape) => Selection::rounded_rect_quality(
            rect,
            shape.radius(),
            width,
            height,
            Selection::RoundedRect(shape).is_antialiased(),
        ),
        _ => {
            if covers {
                Selection::Full
            } else {
                Selection::Rect(rect)
            }
        }
    }
}

fn move_rect(
    rect: PixelRect,
    pointer: (i64, i64),
    grab: (i64, i64),
    width: u32,
    height: u32,
) -> PixelRect {
    let max_x = i64::from(width)
        .saturating_sub(i64::from(rect.width))
        .max(0);
    let max_y = i64::from(height)
        .saturating_sub(i64::from(rect.height))
        .max(0);
    PixelRect::new(
        (pointer.0 - grab.0).clamp(rect.x.min(0), max_x.max(rect.x)),
        (pointer.1 - grab.1).clamp(rect.y.min(0), max_y.max(rect.y)),
        rect.width,
        rect.height,
    )
}

fn resize_rect(
    rect: PixelRect,
    edges: SelectionEdges,
    delta: (i64, i64),
    width: u32,
    height: u32,
) -> PixelRect {
    let clamp_x = |value: i64| value.clamp(0, i64::from(width).saturating_sub(1).max(0));
    let clamp_y = |value: i64| value.clamp(0, i64::from(height).saturating_sub(1).max(0));
    let mut x0 = rect.x;
    let mut y0 = rect.y;
    let mut x1 = rect.right().saturating_sub(1);
    let mut y1 = rect.bottom().saturating_sub(1);
    if edges.left {
        x0 = clamp_x(x0.saturating_add(delta.0));
    }
    if edges.right {
        x1 = clamp_x(x1.saturating_add(delta.0));
    }
    if edges.top {
        y0 = clamp_y(y0.saturating_add(delta.1));
    }
    if edges.bottom {
        y1 = clamp_y(y1.saturating_add(delta.1));
    }
    let left = x0.min(x1);
    let right = x0.max(x1);
    let top = y0.min(y1);
    let bottom = y0.max(y1);
    PixelRect::new(
        left,
        top,
        (right - left + 1) as u32,
        (bottom - top + 1) as u32,
    )
}

fn fraction_pixel(fraction: f32, edge: u32) -> i64 {
    (fraction * edge as f32).floor() as i64
}

fn hit_selection(
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
    x: f32,
    y: f32,
    slop: f32,
) -> SelectionHit {
    if x < left - slop || x > right + slop || y < top - slop || y > bottom + slop {
        return SelectionHit::Outside;
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
        return SelectionHit::Edge(SelectionEdges {
            left: near_left,
            right: near_right,
            top: near_top,
            bottom: near_bottom,
        });
    }
    SelectionHit::Inside
}

impl SelectionEdges {
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

#[derive(Clone, Copy)]
pub(super) enum SelectionCommand {
    All,
    None,
    Invert,
    Clear,
    Crop,
}

#[cfg(test)]
mod radius_handoff_tests {
    use super::*;

    #[test]
    fn queued_steps_clamp_in_order_and_require_the_original_selection_commit() {
        let mut tab = super::super::preview_handoff_tests::tab();
        let pending = PendingAspectRatio::new(&tab, Tool::Select, false, None);
        let mut value = 8;
        for delta in [1, 1, 1, -1] {
            value = stepped_radius(value, delta, 8);
        }
        assert_eq!(value, 7); // Saturated + clicks must not cancel the later - click.
        assert_eq!(stepped_radius(0, -1, 8), 0);
        assert!(!pending.accepts(&tab, Tool::Select, 0, false)); // still busy
        tab.publish_change(0, super::super::Change::Selection, None, 1);
        assert!(pending.accepts(&tab, Tool::Select, 0, false)); // no-op may drain clicks
        assert!(!pending.accepts(&tab, Tool::OvalSelect, 0, false));
        tab.id += 1;
        assert!(!pending.accepts(&tab, Tool::Select, 0, false));
        tab.id -= 1;
        tab.history.commit(tab.state(), 2, false);
        assert!(pending.accepts(&tab, Tool::Select, 0, true));
        tab.history.commit(tab.state(), 3, true);
        assert!(!pending.accepts(&tab, Tool::Select, 0, true));
    }
}
