mod adjust_ui;
mod canvas_tools;
mod clipboard;
mod color;
mod compact_inspector;
mod composite_preview;
mod cursor;
mod dialogs;
mod effects;
mod files;
mod filter_ui;
mod gradient_ui;
mod guide_ui;
mod history;
mod info;
mod layer_menu;
mod layers;
#[cfg(any(target_os = "linux", test))]
mod linux_menu;
mod numeric;
mod paint_ui;
mod path_ui;
mod pdf_preview;
mod pdf_ui;
mod preview_backdrop;
mod recovery_ui;
mod scale_ui;
mod selection;
mod settings_ui;
mod stroke_ui;
mod text_ui;
mod tool_reset;
mod tooltip;
mod transform_ui;
mod view;

use std::{cell::Cell, path::PathBuf, rc::Rc, sync::Arc};

use futures::{StreamExt, channel::mpsc};
use gpui::{Bounds, Context, FocusHandle, Pixels, Point, RenderImage, ScrollHandle, Window};

use crate::commands::{Tool, Zoom};
use crate::document::{
    CropRect, Document, DocumentContent, LayerId, RenderedCanvas, build_level, file_name,
};
use crate::preview::{self, Dirty};
use crate::selection::Selection;

use self::{
    cursor::CanvasCursor,
    dialogs::Form,
    history::{History, LIMITS, State},
};

type PreparedChange = (
    DocumentContent,
    LayerId,
    Selection,
    crate::document::CanvasPatch,
);

// Display-only handoff. It never enters history, saving, or recovery snapshots.
struct PendingCanvas {
    generation: u64,
    layer: LayerId,
    paint: Option<layers::MovePaint>,
    crop: Option<CropRect>,
    selection: Option<Selection>,
    free_points: Vec<Point<f32>>,
}

struct Tab {
    id: usize,
    stroke_anchor: Option<(LayerId, f64, f64)>,
    document: Document,
    active_layer: LayerId,
    zoom: Zoom,
    pan: Point<Pixels>,
    history: History,
    editing: bool,
    saving: Option<PathBuf>,
    exporting: bool,
    notice: Option<String>,
    building_level: Option<u32>,
    selection: Selection,
    // Keep a recoverable draft until its background commit has published.
    pending_text: Option<text_ui::TextDraft>,
    // Keep the slider at the released value while the canvas patch renders.
    pending_opacity: Option<(LayerId, u8)>,
    pending_guides: Option<Vec<crate::guides::Guide>>,
    pending_canvas: Option<PendingCanvas>,
}

impl Tab {
    fn pending_canvas(&self) -> Option<&PendingCanvas> {
        self.pending_canvas.as_ref().filter(|pending| {
            self.editing
                && pending.generation == self.history.generation()
                && pending.layer == self.active_layer
        })
    }

    fn layer_opacity(&self) -> u8 {
        self.pending_opacity
            .filter(|(id, _)| *id == self.active_layer)
            .map(|(_, opacity)| opacity)
            .or_else(|| {
                self.document
                    .content
                    .layer(self.active_layer)
                    .map(|layer| layer.opacity)
            })
            .unwrap_or(255)
    }

    fn busy(&self) -> bool {
        self.editing || self.saving.is_some()
    }

    fn dirty(&self) -> bool {
        self.history.is_dirty()
            || self
                .pending_text
                .as_ref()
                .is_some_and(text_ui::TextDraft::changed)
    }

    fn settle_text_commit(&mut self, published: bool) -> bool {
        if published {
            self.pending_text = None;
            return false;
        }
        if self.pending_text.is_none() {
            return false;
        }
        self.notice = Some(
            "Text could not be committed. The draft was kept; check its dimensions and the layer limit."
                .into(),
        );
        true
    }

    fn state(&self) -> State {
        State {
            content: self.document.content.clone(),
            active_layer: self.active_layer,
            selection: self.selection.clone(),
        }
    }

    // Return replaced or rejected images to the workspace for GPU cache cleanup.
    fn publish_change(
        &mut self,
        generation: u64,
        change: Change,
        result: Option<PreparedChange>,
        stamp: u64,
    ) -> (bool, Vec<Arc<RenderImage>>) {
        if self
            .pending_canvas
            .as_ref()
            .is_some_and(|pending| pending.generation != generation)
        {
            return (
                false,
                result.map_or_else(Vec::new, |(_, _, _, patch)| patch.into_images()),
            );
        }
        let pending = self.pending_canvas.take();
        let (published, mut dropped) =
            self.publish_prepared_change(generation, change, result, stamp);
        if let Some(paint) = pending.and_then(|pending| pending.paint) {
            dropped.extend(paint.into_images());
        }
        (published, dropped)
    }

    // Return replaced or rejected images to the workspace for GPU cache cleanup.
    fn publish_prepared_change(
        &mut self,
        generation: u64,
        change: Change,
        result: Option<PreparedChange>,
        stamp: u64,
    ) -> (bool, Vec<Arc<RenderImage>>) {
        self.editing = false;
        self.pending_opacity = None;
        self.pending_guides = None;
        let Some((content, edited_layer, edited_selection, patch)) = result else {
            return (false, Vec::new());
        };
        if self.history.generation() != generation {
            return (false, patch.into_images());
        }
        let current = self.state();
        let (content, active_layer, selection) = match change {
            Change::Edit | Change::Stroke { .. } | Change::Selection => {
                self.history
                    .commit(current, stamp, !matches!(change, Change::Selection));
                (content, edited_layer, edited_selection)
            }
            Change::Undo | Change::Redo => {
                let state = if matches!(change, Change::Undo) {
                    self.history.undo(current, stamp)
                } else {
                    self.history.redo(current, stamp)
                };
                // Another tab can evict this history target while the patch is rendering.
                let Some(state) = state else {
                    return (false, patch.into_images());
                };
                (state.content, state.active_layer, state.selection)
            }
        };
        match change {
            Change::Stroke { layer, end } => self.stroke_anchor = Some((layer, end.0, end.1)),
            Change::Selection => {}
            _ => self.stroke_anchor = None,
        }
        self.selection = selection;
        self.active_layer = if content.layer(active_layer).is_some() {
            active_layer
        } else {
            content.top_layer_id()
        };
        if content.size() != self.document.content.size() {
            self.pan = Point::default();
        }
        self.document.content = content;
        let replaced = self.document.canvas.apply(patch);
        self.history
            .enforce_tab_limits(&self.document.content, &self.selection, &LIMITS);
        (true, replaced)
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum InspectorTab {
    #[default]
    Layers,
    Paths,
}

#[derive(Clone, Copy)]
enum Change {
    Edit,
    Stroke { layer: LayerId, end: (f64, f64) },
    Selection,
    Undo,
    Redo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CropEdges {
    left: bool,
    right: bool,
    top: bool,
    bottom: bool,
}

#[derive(Clone, Copy)]
enum CropDrag {
    Create {
        origin: Point<Pixels>,
        start: (u32, u32),
    },
    Move {
        crop: CropRect,
        grab: (i64, i64),
    },
    Resize {
        crop: CropRect,
        edges: CropEdges,
        anchor: (i64, i64),
    },
}

#[derive(Clone)]
struct EffectImages {
    before: Arc<RenderImage>,
    after: Arc<RenderImage>,
}

impl EffectImages {
    fn new(context: &crate::effect_preview::EffectPreview, after: &image::RgbaImage) -> Self {
        Self {
            before: crate::document::preview_render(&crate::effect_preview::checker(
                context.original(),
            )),
            after: crate::document::preview_render(&crate::effect_preview::checker(after)),
        }
    }

    fn images(self) -> [Arc<RenderImage>; 2] {
        [self.before, self.after]
    }
}

pub struct Workspace {
    layer_menu: Option<layer_menu::LayerMenu>,
    #[cfg(any(target_os = "linux", test))]
    linux_menu: linux_menu::LinuxMenu,
    focus: FocusHandle,
    numeric_focus: FocusHandle,
    inspector_toggle_focus: FocusHandle,
    numeric_edit: Option<numeric::NumericEdit>,
    numeric_followup: Option<numeric::PendingNumeric>,
    tab_scroll: ScrollHandle,
    inspector_scroll: ScrollHandle,
    layers_scroll: ScrollHandle,
    paths_scroll: ScrollHandle,
    compact_inspector_scroll: [ScrollHandle; 3],
    tabs: Vec<Tab>,
    recovery: recovery_ui::Recovery,
    active: Option<usize>,
    next_id: usize,
    preferences: settings_ui::Preferences,
    color_memory: crate::color_memory::Memory,
    settings_store: settings_ui::Storage,
    guide_drag: Option<guide_ui::GuideDrag>,
    viewport: Rc<Cell<Bounds<Pixels>>>,
    canvas_cursor: Rc<CanvasCursor>,
    pan_drag: Option<(gpui::MouseButton, Point<Pixels>)>,
    space_pan: bool,
    crop_drag: Option<CropDrag>,
    crop: Option<CropRect>,
    selection_drag: Option<selection::SelectionGesture>,
    selection_preview: Option<Selection>,
    pending_selection_inputs: Option<selection::PendingSelectionInputs>,
    in_flight_selection_input: Option<selection::InFlightSelectionInput>,
    free_points: Vec<Point<f32>>,
    free_nodes: Vec<usize>,
    free_node_drag: Option<(usize, Point<f32>)>,
    free_op: crate::selection::SelectionOp,
    free_drag_origin: Option<Point<Pixels>>,
    free_dragging: bool,
    move_drag: Option<layers::MoveDrag>,
    move_overlay: Option<layers::MovePaint>,
    composite_preview: composite_preview::CompositePreview,
    scale_drag: Option<scale_ui::ScaleDrag>,
    scale_overlay: Option<layers::MovePaint>,
    scale_backdrop: preview_backdrop::BackdropCache,
    scale_token: u64,
    scale_preview_busy: bool,
    scale_preview_dirty: bool,
    move_settled: Option<(i32, i32)>,
    move_token: u64,
    pending_nudge: Option<layers::PendingNudge>,
    warp: Option<transform_ui::WarpSession>,
    warp_overlay: Option<layers::MovePaint>,
    warp_backdrop: preview_backdrop::BackdropCache,
    warp_token: u64,
    warp_busy: bool,
    warp_preview_edge: u32,
    warp_dirty: bool,
    warp_applying: bool,
    warp_error: Option<&'static str>,
    gradient_revision: u64,
    gradient_library: crate::gradient::library::Library,
    gradient_store: gradient_ui::PresetStorage,
    clone_source: Option<paint_ui::CloneSource>,
    gradient: Option<gradient_ui::GradientSession>,
    gradient_overlay: Option<layers::MovePaint>,
    gradient_token: u64,
    gradient_busy: bool,
    gradient_dirty: bool,
    adjustment: Option<adjust_ui::AdjustmentSession>,
    adjustment_preview: Option<EffectImages>,
    adjustment_generation: crate::adjustments::PreviewGeneration,
    adjustment_busy: bool,
    adjustment_dirty: bool,
    adjustment_histogram: Option<crate::adjustments::Histogram>,
    histogram_token: u64,
    filter: Option<filter_ui::FilterSession>,
    filter_preview: Option<EffectImages>,
    effect_before: bool,
    applying_form: Option<(usize, u64)>,
    effect_reediting: bool,
    effect_memory: effects::EffectMemory,
    filter_generation: crate::adjustments::PreviewGeneration,
    filter_busy: bool,
    filter_dirty: bool,
    path_gesture: Option<path_ui::PathGesture>,
    path_hold: Option<crate::paths::VectorPath>,
    path_selection: Option<path_ui::AnchorSelection>,
    stroke: Option<paint_ui::StrokeSession>,
    stroke_token: u64,
    stroke_commit_preview: Option<(usize, Vec<paint_ui::StrokePaint>)>,
    text_draft: Option<text_ui::TextDraft>,
    text_generation: u64,
    text_busy: bool,
    text_dirty: bool,
    text_error: Option<crate::text::TextError>,
    text_overlay: Option<layers::MovePaint>,
    text_backdrop: preview_backdrop::BackdropCache,
    // The tab whose committed text still shows the preview until its patch lands.
    text_settled: Option<usize>,
    text_continuation: Option<(usize, text_ui::TextContinuation)>,
    font_families: Option<Arc<crate::text::FontFamilies>>,
    font_loading: bool,
    font_list_open: bool,
    font_search: text_ui::FontSearch,
    font_search_focus: FocusHandle,
    font_scroll: ScrollHandle,
    font_input_scroll: ScrollHandle,
    caret_shown_at: std::time::Instant,
    caret_blinking: bool,
    copy_token: u64,
    paste_token: u64,
    pdf_import_token: u64,
    pdf_job: Option<pdf_ui::PdfJob>,
    pdf_previews: pdf_preview::PdfPreviews,
    pending_opens: std::collections::VecDeque<pdf_ui::PendingOpen>,
    open_scheduled: bool,
    dialog_open: bool,
    form: Option<Form>,
    creating: bool,
    next_untitled: usize,
    loader: mpsc::UnboundedSender<PathBuf>,
    pending: usize,
    errors: Vec<String>,
    history_clock: u64,
    opacity_drag: Option<layers::OpacityDrag>,
    toolbox_drag: Option<(f32, f32)>,
}

impl Workspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let entity = cx.weak_entity();
        window.on_window_should_close(cx, move |window, cx| {
            entity
                .update(cx, |workspace, cx| {
                    workspace.request_exit(false, window, cx)
                })
                .unwrap_or(true)
        });
        cx.observe_window_activation(window, |workspace, window, cx| {
            if !window.is_window_active() {
                workspace.layer_menu = None;
                #[cfg(any(target_os = "linux", test))]
                workspace.linux_menu.close();
                workspace.canvas_cursor.leave();
                workspace.pan_drag = None;
                workspace.space_pan = false;
                if let Some(draft) = workspace.text_draft.as_mut() {
                    draft.end_pointer_drag();
                }
                workspace.cancel_guide(cx);
                workspace.pending_nudge = None;
                workspace.crop_drag = None;
                workspace.selection_drag = None;
                workspace.selection_preview = None;
                workspace.opacity_drag = None;
                workspace.toolbox_drag = None;
                if workspace.preferences.tool == Tool::FreeSelect {
                    workspace.clear_free_draft();
                }
                workspace.cancel_path_gesture(cx);
                if workspace.move_drag.is_some() {
                    workspace.cancel_move(window, cx);
                }
                if workspace.stroke.is_some() {
                    workspace.cancel_stroke(window, cx);
                }
                workspace.end_warp_drag(cx);
                workspace.end_gradient_drag(cx);
                workspace.end_scale_drag(cx);
            }
            cx.notify();
        })
        .detach();
        let focus = cx.focus_handle();
        focus.focus(window);
        let numeric_focus = cx.focus_handle();
        // Tracked handles own their tab-stop policy, rather than inheriting it
        // from the element's tab_index as generated handles do.
        let inspector_toggle_focus = cx.focus_handle().tab_stop(true);
        let font_search_focus = cx.focus_handle();
        cx.on_blur(&font_search_focus, window, |workspace, _, cx| {
            workspace.font_list_open = false;
            cx.notify();
        })
        .detach();
        cx.on_blur(&numeric_focus, window, |workspace, window, cx| {
            workspace.commit_numeric_edit(window, cx);
        })
        .detach();
        let numeric_entity = cx.weak_entity();
        // Inline editors consume keystrokes before key bindings and element
        // listeners run. Stopping propagation here also skips `on_key_down`,
        // so the edit must be applied in the interceptor itself.
        cx.intercept_keystrokes(move |event, window, cx| {
            let handled = numeric_entity
                .update(cx, |workspace, cx| {
                    if workspace.info_key(&event.keystroke, window, cx) {
                        return true;
                    }
                    if workspace.layer_menu_key(&event.keystroke, window, cx) {
                        return true;
                    }
                    #[cfg(any(target_os = "linux", test))]
                    if workspace.linux_menu_key(&event.keystroke, window, cx) {
                        return true;
                    }
                    if workspace.font_list_open && workspace.font_search_focus.is_focused(window) {
                        return workspace.font_search_key(&event.keystroke, window, cx);
                    }
                    if workspace.numeric_edit.is_none() {
                        return false;
                    }
                    if workspace.numeric_key(&event.keystroke, window, cx) {
                        return true;
                    }
                    // Unhandled shortcuts such as Undo or Save act on the
                    // committed value rather than a half-typed draft.
                    workspace.commit_numeric_with_followup(
                        numeric::NumericFollowup::Key(event.keystroke.clone()),
                        window,
                        cx,
                    )
                })
                .unwrap_or(false);
            if handled {
                cx.stop_propagation();
            }
        })
        .detach();
        let (loader, mut requests) = mpsc::unbounded::<PathBuf>();
        let executor = cx.background_executor().clone();
        cx.spawn_in(window, async move |workspace, cx| {
            while let Some(path) = requests.next().await {
                let name = file_name(&path);
                if crate::pdf::is_pdf_path(&path) {
                    if workspace
                        .update_in(cx, |workspace, _, cx| {
                            workspace.pending -= 1;
                            workspace
                                .pending_opens
                                .push_back(pdf_ui::PendingOpen::Pdf { path });
                            cx.notify();
                        })
                        .is_err()
                    {
                        break;
                    }
                    continue;
                }
                let result = executor.spawn(async move { Document::open(&path) }).await;
                if workspace
                    .update_in(cx, |workspace, _window, cx| {
                        workspace.pending -= 1;
                        match result {
                            Ok(document) => workspace
                                .pending_opens
                                .push_back(pdf_ui::PendingOpen::Raster(Box::new(document))),
                            Err(error) => workspace.errors.push(format!("{name}: {error}")),
                        }
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();

        let mut workspace = Self {
            layer_menu: None,
            #[cfg(any(target_os = "linux", test))]
            linux_menu: Default::default(),
            focus,
            numeric_focus,
            inspector_toggle_focus,
            numeric_edit: None,
            numeric_followup: None,
            tab_scroll: ScrollHandle::new(),
            inspector_scroll: ScrollHandle::new(),
            layers_scroll: ScrollHandle::new(),
            paths_scroll: ScrollHandle::new(),
            compact_inspector_scroll: std::array::from_fn(|_| ScrollHandle::new()),
            tabs: Vec::new(),
            recovery: Default::default(),
            active: None,
            next_id: 0,
            preferences: Default::default(),
            color_memory: Default::default(),
            settings_store: Default::default(),
            guide_drag: None,
            viewport: Rc::default(),
            canvas_cursor: Rc::default(),
            pan_drag: None,
            space_pan: false,
            crop_drag: None,
            crop: None,
            selection_drag: None,
            selection_preview: None,
            pending_selection_inputs: None,
            in_flight_selection_input: None,
            free_points: Vec::new(),
            free_nodes: Vec::new(),
            free_node_drag: None,
            free_op: crate::selection::SelectionOp::Replace,
            free_drag_origin: None,
            free_dragging: false,
            move_drag: None,
            move_overlay: None,
            composite_preview: Default::default(),
            scale_drag: None,
            scale_overlay: None,
            scale_backdrop: Default::default(),
            scale_token: 0,
            scale_preview_busy: false,
            scale_preview_dirty: false,
            move_settled: None,
            move_token: 0,
            pending_nudge: None,
            warp: None,
            warp_overlay: None,
            warp_backdrop: Default::default(),
            warp_token: 0,
            warp_busy: false,
            warp_preview_edge: 0,
            warp_dirty: false,
            warp_applying: false,
            warp_error: None,
            gradient_revision: 0,
            gradient_library: Default::default(),
            gradient_store: Default::default(),
            clone_source: None,
            gradient: None,
            gradient_overlay: None,
            gradient_token: 0,
            gradient_busy: false,
            gradient_dirty: false,
            adjustment: None,
            adjustment_preview: None,
            adjustment_generation: crate::adjustments::PreviewGeneration::default(),
            adjustment_busy: false,
            adjustment_dirty: false,
            adjustment_histogram: None,
            histogram_token: 0,
            filter: None,
            filter_preview: None,
            effect_before: false,
            applying_form: None,
            effect_reediting: false,
            effect_memory: effects::EffectMemory::default(),
            filter_generation: crate::adjustments::PreviewGeneration::default(),
            filter_busy: false,
            filter_dirty: false,
            path_gesture: None,
            path_hold: None,
            path_selection: None,
            stroke: None,
            stroke_token: 0,
            stroke_commit_preview: None,
            text_draft: None,
            text_generation: 0,
            text_busy: false,
            text_dirty: false,
            text_error: None,
            text_overlay: None,
            text_backdrop: Default::default(),
            text_settled: None,
            text_continuation: None,
            font_families: None,
            font_loading: false,
            font_list_open: false,
            font_search: text_ui::FontSearch::default(),
            font_search_focus,
            font_scroll: ScrollHandle::new(),
            font_input_scroll: ScrollHandle::new(),
            caret_shown_at: std::time::Instant::now(),
            caret_blinking: false,
            copy_token: 0,
            paste_token: 0,
            pdf_import_token: 0,
            pdf_job: None,
            pdf_previews: Default::default(),
            pending_opens: Default::default(),
            open_scheduled: false,
            dialog_open: false,
            form: None,
            creating: false,
            next_untitled: 1,
            loader,
            pending: 0,
            errors: Vec::new(),
            history_clock: 0,
            opacity_drag: None,
            toolbox_drag: None,
        };
        workspace.load_gradient_library(window, cx);
        workspace.load_settings(window, cx);
        workspace
    }

    pub fn open_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        if self.pending == 0 {
            self.errors.clear();
        }
        for path in paths {
            if self.loader.unbounded_send(path).is_ok() {
                self.pending += 1;
            }
        }
        cx.notify();
    }

    fn show_document(
        &mut self,
        mut document: Document,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let history = History::new(document.path.is_some());
        if let Some(path) = &document.path {
            if let Some(index) = self.tabs.iter().position(|tab| {
                tab.document.path.as_ref() == Some(path) || tab.saving.as_ref() == Some(path)
            }) {
                drop_canvas(document.canvas, window, cx);
                self.select_tab(index, window, cx);
                return;
            }
            cx.add_recent_document(path);
        }
        let notice = document
            .content
            .take_link_migration_notice()
            .map(str::to_owned);
        self.tabs.push(Tab {
            stroke_anchor: None,
            id: self.next_id,
            active_layer: document.content.preferred_active_layer(),
            document,
            zoom: Zoom::default(),
            pan: Point::default(),
            history,
            editing: false,
            saving: None,
            exporting: false,
            notice,
            building_level: None,
            selection: Selection::Unrestricted,
            pending_text: None,
            pending_opacity: None,
            pending_guides: None,
            pending_canvas: None,
        });
        self.next_id += 1;
        self.select_tab(self.tabs.len() - 1, window, cx);
    }

    fn active_document(&self) -> Option<&Document> {
        self.active_tab().map(|tab| &tab.document)
    }

    fn active_tab(&self) -> Option<&Tab> {
        self.active.map(|index| &self.tabs[index])
    }

    fn tab_index(&self, id: usize) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.id == id)
    }

    fn select_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let switched = self.active != Some(index);
        if switched {
            self.cancel_guide(cx);
            self.pending_nudge = None;
            self.clone_source = None;
            self.commit_numeric_edit(window, cx);
            self.numeric_followup = None;
            // A commit that starts here finishes on the tab it belongs to.
            self.finish_text(window, cx);
            self.drop_settled_text(window, cx);
            self.text_continuation = None;
            self.dismiss_form(window, cx);
            self.clear_path_edit();
            self.clear_crop();
            self.clear_selection_gesture();
            self.clear_move_preview(window, cx);
            self.cancel_warp(window, cx);
            self.cancel_stroke(window, cx);
            self.cancel_gradient(window, cx);
            self.cancel_scale(window, cx);
            self.cancel_text(window, cx);
            self.opacity_drag = None;
        }
        self.active = Some(index);
        self.recover_pending_text(window, cx);
        self.pan_drag = None;
        self.tab_scroll.scroll_to_item(index);
        self.update_title(window);
        self.ensure_preview_level(window.scale_factor(), window, cx);
        if switched {
            self.begin_layer_transform_preview(window, cx);
        }
        cx.notify();
    }

    fn cycle_tabs(&mut self, backwards: bool, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.active {
            let count = self.tabs.len();
            let step = if backwards { count - 1 } else { 1 };
            self.select_tab((index + step) % count, window, cx);
        }
    }

    fn close_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.pan_drag = None;
        let closed_active = self.active == Some(index);
        if closed_active {
            self.cancel_guide(cx);
            self.pending_nudge = None;
            self.clone_source = None;
            self.numeric_followup = None;
            self.opacity_drag = None;
            self.dismiss_form(window, cx);
            self.clear_path_edit();
            self.clear_crop();
            self.clear_selection_gesture();
            self.clear_move_preview(window, cx);
            self.cancel_warp(window, cx);
            self.cancel_stroke(window, cx);
            self.cancel_gradient(window, cx);
            self.cancel_scale(window, cx);
            self.cancel_text(window, cx);
            self.drop_settled_text(window, cx);
            self.text_continuation = None;
        }
        let removed = self.tabs.remove(index);
        self.remove_recovery_tab(removed.id, cx);
        if let Some(paint) = removed.pending_canvas.and_then(|pending| pending.paint) {
            drop_images(paint.into_images(), window, cx);
        }
        drop_canvas(removed.document.canvas, window, cx);

        self.active = match self.active {
            _ if self.tabs.is_empty() => None,
            Some(active) if active == index => Some(index.min(self.tabs.len() - 1)),
            Some(active) if active > index => Some(active - 1),
            active => active,
        };
        if let Some(active) = self.active {
            self.tab_scroll.scroll_to_item(active);
        }
        self.recover_pending_text(window, cx);
        self.update_title(window);
        if closed_active && self.active.is_some() {
            self.begin_layer_transform_preview(window, cx);
        }
        cx.notify();
    }

    fn clear_crop(&mut self) {
        if self
            .numeric_edit
            .as_ref()
            .is_some_and(|edit| edit.is_crop())
        {
            self.numeric_edit = None;
        }
        self.crop = None;
        self.crop_drag = None;
    }

    // The edit may also choose a new active layer.
    fn edit_content(
        &mut self,
        edit: impl FnOnce(&mut DocumentContent, &mut LayerId, &mut Selection) -> Dirty + Send + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.start_change(Change::Edit, edit, window, cx);
    }

    fn can_step_history(&self, change: Change) -> bool {
        self.text_draft.is_none()
            && self.active_tab().is_some_and(|tab| {
                !tab.busy()
                    && match change {
                        Change::Edit | Change::Stroke { .. } | Change::Selection => false,
                        Change::Undo => tab.history.undo_target().is_some(),
                        Change::Redo => tab.history.redo_target().is_some(),
                    }
            })
    }

    fn step_history(&mut self, change: Change, window: &mut Window, cx: &mut Context<Self>) {
        if self.can_step_history(change) {
            self.clear_path_edit();
            self.clear_crop();
            self.clear_selection_gesture();
            self.clear_move_preview(window, cx);
            self.cancel_warp(window, cx);
            self.cancel_stroke(window, cx);
            self.cancel_gradient(window, cx);
            self.cancel_text(window, cx);
            self.start_change(change, |_, _, _| Dirty::Canvas, window, cx);
        }
    }

    fn clear_content_previews(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.cancel_guide(cx);
        self.dismiss_effect_form(window, cx);
        self.clear_crop();
        self.clear_selection_gesture();
        self.cancel_path_gesture(cx);
        self.opacity_drag = None;
        self.cancel_text(window, cx);
        self.cancel_warp(window, cx);
        self.cancel_stroke(window, cx);
        self.cancel_gradient(window, cx);
        self.cancel_scale(window, cx);
    }

    fn apply_preview(
        &mut self,
        edit: impl FnOnce(&mut DocumentContent, &mut LayerId, &mut Selection) -> Dirty + Send + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.start_preview_change(Change::Edit, edit, true, window, cx);
    }

    fn start_change(
        &mut self,
        change: Change,
        edit: impl FnOnce(&mut DocumentContent, &mut LayerId, &mut Selection) -> Dirty + Send + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.start_preview_change(change, edit, false, window, cx);
    }

    fn start_preview_change(
        &mut self,
        change: Change,
        edit: impl FnOnce(&mut DocumentContent, &mut LayerId, &mut Selection) -> Dirty + Send + 'static,
        retain_preview: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.move_drag.is_some() {
            return;
        }
        let Some(index) = self.active else {
            return;
        };
        if self.tabs[index].busy() {
            return;
        }
        // Typed text lands as its own undo step before this change runs.
        if self.finish_text(window, cx) {
            self.defer_after_text(move |this, window, cx| {
                this.start_preview_change(change, edit, retain_preview, window, cx);
            });
            return;
        }
        if retain_preview || matches!(change, Change::Selection) {
            let paint = retain_preview.then(|| self.take_move_paint()).flatten();
            let tab = &mut self.tabs[index];
            tab.pending_canvas = Some(PendingCanvas {
                generation: tab.history.generation(),
                layer: tab.active_layer,
                paint,
                crop: retain_preview.then_some(self.crop).flatten(),
                selection: matches!(change, Change::Selection).then(|| {
                    self.selection_preview
                        .clone()
                        .unwrap_or_else(|| tab.selection.clone())
                }),
                free_points: if matches!(change, Change::Selection) {
                    self.free_points.clone()
                } else {
                    Vec::new()
                },
            });
        }
        self.clear_content_previews(window, cx);
        self.clear_free_draft();
        let tab = &mut self.tabs[index];
        let target = match change {
            Change::Edit | Change::Stroke { .. } | Change::Selection => None,
            Change::Undo => tab.history.undo_target(),
            Change::Redo => tab.history.redo_target(),
        };
        let mut content = target
            .map_or(&tab.document.content, |(state, _)| &state.content)
            .clone();
        let session_only = target.is_some_and(|(_, revision)| revision == tab.history.revision());
        tab.editing = true;
        tab.notice = None;
        let id = tab.id;
        let generation = tab.history.generation();
        let pixels = tab.document.canvas.pixels.clone();
        let has_transparency = tab.document.canvas.preview.has_transparency;
        let levels = tab.document.canvas.level_caches();
        let mut active_layer = tab.active_layer;
        let mut selection = tab.selection.clone();
        let job = cx.background_executor().spawn(async move {
            let dirty = if session_only {
                Dirty::Unpainted
            } else {
                edit(&mut content, &mut active_layer, &mut selection)
            };
            if dirty != Dirty::None {
                selection.outline_edges();
            }
            RenderedCanvas::prepare_patch(&pixels, has_transparency, levels, &content, dirty)
                .map(|patch| (content, active_layer, selection, patch))
        });
        cx.spawn_in(window, async move |workspace, cx| {
            let result = job.await;
            let _ = workspace.update_in(cx, |workspace, window, cx| {
                workspace.finish_change(id, generation, change, result, window, cx);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn finish_change(
        &mut self,
        id: usize,
        generation: u64,
        change: Change,
        result: Option<PreparedChange>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.text_settled == Some(id) {
            self.drop_settled_text(window, cx);
        }
        let published = self.publish_change(id, generation, change, result, window, cx);
        if self.applying_form == Some((id, generation)) {
            self.applying_form = None;
            self.dismiss_form(window, cx);
        }
        self.effect_memory.finish(id, generation, published);
        self.preferences.effects = self.effect_memory.preferences();
        self.finish_clone_alignment(id, generation, published);
        if let Some(index) = self.tab_index(id)
            && self.tabs[index].settle_text_commit(published)
        {
            if self
                .text_continuation
                .as_ref()
                .is_some_and(|(tab, _)| *tab == id)
            {
                self.text_continuation = None;
            }
            self.recover_pending_text(window, cx);
        }
        if self
            .text_continuation
            .as_ref()
            .is_some_and(|(tab, _)| *tab == id)
            && let Some((_, then)) = self.text_continuation.take()
            && self.active_tab().is_some_and(|tab| tab.id == id)
        {
            then(self, window, cx);
        }
        self.finish_numeric_followup(id, generation, published, window, cx);
        self.finish_pending_nudge(id, generation, published, window, cx);
        self.finish_pending_selection_inputs(id, generation, change, published, window, cx);
    }

    // Publishes a background result only if the tab has not changed since the job started.
    fn publish_change(
        &mut self,
        id: usize,
        generation: u64,
        change: Change,
        result: Option<PreparedChange>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(index) = self.tab_index(id) else {
            if let Some((_, _, _, patch)) = result {
                drop_images(patch.into_images(), window, cx);
            }
            return false;
        };
        if self.active == Some(index) && self.move_settled.is_some() {
            self.move_token = self.move_token.wrapping_add(1);
            self.drop_move_overlay(window, cx);
        }
        if self
            .stroke_commit_preview
            .as_ref()
            .is_some_and(|(tab, _)| *tab == id)
        {
            self.clear_stroke_commit_preview(window, cx);
        }
        if self.active == Some(index) {
            self.path_hold = None;
        }
        self.history_clock += 1;
        let stamp = self.history_clock;
        let (published, replaced) =
            self.tabs[index].publish_change(generation, change, result, stamp);
        drop_images(replaced, window, cx);
        if !published {
            return false;
        }
        history::enforce_app_budget(
            self.tabs
                .iter_mut()
                .map(|tab| (&mut tab.history, &tab.document.content, &tab.selection)),
            &LIMITS,
        );
        self.update_title(window);
        self.ensure_preview_level(window.scale_factor(), window, cx);
        true
    }

    fn update_title(&self, window: &mut Window) {
        let title = self.active_tab().map_or_else(
            || "Lime — Lite Image Editor".to_owned(),
            |tab| {
                let marker = if tab.dirty() { " •" } else { "" };
                format!("{}{marker} — Lime", tab.document.name)
            },
        );
        window.set_window_title(&title);
        window.set_window_edited(self.active_tab().is_some_and(|tab| tab.dirty()));
    }
}

impl Workspace {
    fn request_preview_level(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let scale = window.scale_factor();
        let Some(index) = self.active else {
            return;
        };
        let tab = &self.tabs[index];
        let Some(shift) = preview_shift(tab, self.viewport.get(), scale) else {
            return;
        };
        if shift == 0
            || tab.document.canvas.has_level(shift)
            || tab.building_level.is_some()
            || tab.busy()
        {
            return;
        }
        cx.defer_in(window, move |workspace, window, cx| {
            workspace.ensure_preview_level(scale, window, cx);
        });
    }

    pub(super) fn ensure_preview_level(
        &mut self,
        scale: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = self.active else {
            return;
        };
        let Some(shift) = preview_shift(&self.tabs[index], self.viewport.get(), scale) else {
            return;
        };
        let tab = &mut self.tabs[index];
        if shift == 0
            || tab.document.canvas.has_level(shift)
            || tab.building_level.is_some()
            || tab.busy()
        {
            return;
        }
        tab.building_level = Some(shift);
        let id = tab.id;
        let source_pixels = Arc::clone(&tab.document.canvas.pixels);
        let content = tab.document.content.clone();
        let job = cx
            .background_executor()
            .spawn(async move { build_level(&content, shift) });
        cx.spawn_in(window, async move |workspace, cx| {
            let cache = job.await;
            let _ = workspace.update_in(cx, |workspace, window, cx| {
                let Some(index) = workspace.tab_index(id) else {
                    return;
                };
                let tab = &mut workspace.tabs[index];
                tab.building_level = None;
                // A rename keeps the same composite. A painted edit replaces it, so this
                // reduction belongs to the previous pixels and must not be installed.
                if Arc::ptr_eq(&tab.document.canvas.pixels, &source_pixels) {
                    let replaced = tab.document.canvas.install_cache(cache);
                    drop_images(replaced, window, cx);
                    cx.notify();
                }
                workspace.ensure_preview_level(window.scale_factor(), window, cx);
            });
        })
        .detach();
    }
}

fn preview_shift(tab: &Tab, viewport: Bounds<Pixels>, scale: f32) -> Option<u32> {
    let geometry = canvas_tools::PreviewGeometry::for_tab(tab, viewport, scale)?;
    let display = f32::from(geometry.bounds.size.width)
        .max(f32::from(geometry.bounds.size.height))
        .max(1.)
        * scale;
    Some(preview::fit_level_shift(
        tab.document.width().max(tab.document.height()),
        display as u32,
    ))
}

fn drop_images(
    images: impl IntoIterator<Item = Arc<RenderImage>>,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    for image in images {
        cx.drop_image(image, Some(window));
    }
}

fn drop_canvas(canvas: RenderedCanvas, window: &mut Window, cx: &mut Context<Workspace>) {
    for image in canvas.into_images() {
        cx.drop_image(image, Some(window));
    }
}

#[cfg(test)]
mod preview_handoff_tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    pub(super) fn tab() -> Tab {
        let content = DocumentContent::single_layer(
            "Test".into(),
            RgbaImage::from_pixel(16, 16, Rgba([20, 40, 60, 255])),
        );
        Tab {
            id: 1,
            stroke_anchor: None,
            active_layer: content.top_layer_id(),
            document: Document::from_content("Test".into(), content),
            zoom: Zoom::default(),
            pan: Point::default(),
            history: History::new(true),
            editing: true,
            saving: None,
            exporting: false,
            notice: None,
            building_level: None,
            selection: Selection::Unrestricted,
            pending_text: None,
            pending_opacity: None,
            pending_guides: None,
            pending_canvas: None,
        }
    }

    #[test]
    fn visible_handoff_leaves_committed_data_untouched_and_settles_with_result() {
        for outcome in ["publish", "noop", "stale"] {
            let mut tab = tab();
            let original = tab.document.canvas.pixels.clone();
            let preview = crate::document::preview_render(&RgbaImage::from_pixel(
                16,
                16,
                Rgba([240, 30, 10, 255]),
            ));
            let preview_id = preview.id;
            tab.pending_canvas = Some(PendingCanvas {
                generation: 0,
                layer: tab.active_layer,
                paint: Some(layers::MovePaint {
                    below: None,
                    above: None,
                    sprite: preview,
                    offset: (0, 0),
                    size: (16, 16),
                }),
                crop: None,
                selection: None,
                free_points: Vec::new(),
            });
            assert!(tab.pending_canvas().is_some());
            // A completion from a different job must not consume this handoff.
            assert!(!tab.publish_change(99, Change::Edit, None, 0).0);
            assert!(tab.pending_canvas().is_some());
            let layer = tab.active_layer;
            tab.active_layer = LayerId(999);
            assert!(tab.pending_canvas().is_none());
            tab.active_layer = layer;
            assert!(!tab.dirty());
            assert!(Arc::ptr_eq(&tab.document.canvas.pixels, &original));
            assert_eq!(tab.state().content.composite(), original);
            let mut content = tab.document.content.clone();
            content.apply_adjustment(
                tab.active_layer,
                crate::adjustments::Adjustment::Invert,
                &Selection::Unrestricted,
            );
            let expected = content.composite();
            let result = (outcome != "noop").then(|| {
                let patch = RenderedCanvas::prepare_patch(
                    &original,
                    false,
                    tab.document.canvas.level_caches(),
                    &content,
                    Dirty::Canvas,
                )
                .unwrap();
                (content, tab.active_layer, Selection::Unrestricted, patch)
            });
            if outcome == "stale" {
                tab.history.commit(tab.state(), 1, false);
                assert!(tab.pending_canvas().is_none());
            }
            let (published, dropped) = tab.publish_change(0, Change::Edit, result, 2);
            assert_eq!(published, outcome == "publish");
            assert!(tab.pending_canvas.is_none());
            assert!(!tab.editing);
            assert!(dropped.iter().any(|image| image.id == preview_id));
            assert_eq!(
                tab.document.canvas.pixels,
                if published { expected } else { original }
            );
            assert_eq!(
                tab.history.generation(),
                u64::from(published || outcome == "stale")
            );
            assert_eq!(tab.dirty(), published);
        }
    }
}
