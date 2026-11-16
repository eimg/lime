//! A modal, one-shot operation. Immutable text/guide snapshots and revision
//! checks keep asynchronous preview results from targeting another document.
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use anyhow::{Result, ensure};
use gpui::{
    Context, Div, KeyDownEvent, ObjectFit, RenderImage, Window, div, img, prelude::*, px, rgb,
};

use super::{
    ACCENT, Field, FieldMode, FormKind, MUTED, Workspace, choice_button, form_button, segmented,
};
use crate::document::{DocumentContent, LayerId};
use crate::paths::{PathId, VectorPath};
use crate::preview::Dirty;
use crate::text::{
    TextAlign, TextSpec,
    path::{Options, Prepared},
};
use crate::workspace::tooltip::tooltip;
use crate::workspace::view::step_button;
use crate::workspace::{Tab, numeric::NumericFollowup};

const PREVIEW_DELAY: Duration = Duration::from_millis(80);
const PROGRESS_DELAY: Duration = Duration::from_millis(500);
const MAX_OFFSET: f64 = 1_000_000.;

fn offset_value(text: &str) -> Option<f64> {
    text.parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && value.abs() <= MAX_OFFSET)
}

fn stepped_offset(text: &str, previous: f64, delta: f64) -> String {
    let value = (offset_value(text).unwrap_or(previous) + delta).clamp(-MAX_OFFSET, MAX_OFFSET);
    let text = format!("{value:.6}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" {
        "0".into()
    } else {
        text.into()
    }
}

#[derive(Clone, Copy)]
struct Target {
    tab: usize,
    layer: LayerId,
    generation: u64,
    path: PathId,
}

impl Target {
    fn matches(self, tab: &Tab) -> bool {
        tab.id == self.tab
            && tab.active_layer == self.layer
            && !tab.busy()
            && tab.history.generation() == self.generation
            && tab.document.content.active_path_id() == Some(self.path)
    }
}

pub(in crate::workspace) struct TextPathDraft {
    target: Target,
    content: DocumentContent,
    spec: TextSpec,
    guide: VectorPath,
    fields: [Field; 2],
    align: TextAlign,
    reverse: bool,
    active: usize, // alignment, distance, baseline, reverse
    revision: u64,
    running: bool,
    preview_slow: bool,
    preview_timer: bool,
    preview_after: Instant,
    last_offsets: [f64; 2],
    overflow: bool,
    pub(super) applying: bool,
    dirty: bool,
    canceled: Arc<AtomicBool>,
    ready: Option<Arc<Prepared>>,
    preview: Option<Arc<RenderImage>>,
    error: Option<String>,
}

impl Drop for TextPathDraft {
    fn drop(&mut self) {
        self.canceled.store(true, Ordering::Relaxed);
    }
}

impl TextPathDraft {
    fn capture(tab: &Tab) -> Option<Self> {
        let content = &tab.document.content;
        let style = content.layer(tab.active_layer)?.text.as_ref()?;
        let guide = content.active_path()?;
        Some(Self {
            target: Target {
                tab: tab.id,
                layer: tab.active_layer,
                generation: tab.history.generation(),
                path: guide.id,
            },
            content: content.clone(),
            spec: TextSpec {
                source: style.source.clone(),
                family: style.family.clone(),
                face: style.face,
                size: style.size,
                color: style.color,
                spacing: style.spacing,
                layout: style.layout,
            },
            guide: guide.clone(),
            fields: [Field::new("0".into()), Field::new("0".into())],
            align: style.layout.align,
            reverse: false,
            active: 1,
            revision: 0,
            running: false,
            preview_slow: false,
            preview_timer: false,
            preview_after: Instant::now(),
            last_offsets: [0.; 2],
            overflow: false,
            applying: false,
            dirty: true,
            canceled: Arc::new(AtomicBool::new(false)),
            ready: None,
            preview: None,
            error: None,
        })
    }

    fn options(&self) -> Result<Options> {
        let mut values = [0.; 2];
        for (value, field) in values.iter_mut().zip(&self.fields) {
            *value = offset_value(&field.text).unwrap_or(f64::NAN);
            ensure!(
                value.is_finite() && value.abs() <= 1_000_000.,
                "Offsets must be between −1000000 and 1000000 pixels."
            );
        }
        Ok(Options {
            align: self.align,
            offset: values[0],
            baseline: values[1],
            reverse: self.reverse,
        })
    }

    fn changed(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.ready = None;
        self.error = None;
        self.dirty = true;
        self.preview_slow = false;
        self.preview_after = Instant::now() + PREVIEW_DELAY;
        for (last, field) in self.last_offsets.iter_mut().zip(&self.fields) {
            if let Some(value) = offset_value(&field.text) {
                *last = value;
            }
        }
    }

    fn step_offset(&mut self, index: usize, delta: f64) {
        if self.applying || index >= self.fields.len() {
            return;
        }
        let text = stepped_offset(&self.fields[index].text, self.last_offsets[index], delta);
        self.active = index + 1;
        if self.fields[index].text != text {
            self.fields[index] = Field::new(text);
            self.changed();
        }
    }

    pub(super) fn handle_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Workspace>) {
        if self.applying {
            return;
        }
        let key = &event.keystroke;
        if key.key == "tab" {
            self.active = (self.active + if key.modifiers.shift { 3 } else { 1 }) % 4;
            if (1..=2).contains(&self.active) {
                self.fields[self.active - 1].select_all();
            }
        } else if self.active == 0
            && !key.modifiers.modified()
            && matches!(key.key.as_str(), "left" | "right")
        {
            let index = match self.align {
                TextAlign::Left => 0,
                TextAlign::Center => 1,
                TextAlign::Right => 2,
            };
            self.align = [TextAlign::Left, TextAlign::Center, TextAlign::Right]
                [(index + if key.key == "left" { 2 } else { 1 }) % 3];
            self.changed();
        } else if self.active == 3 && key.key == "space" && !key.modifiers.modified() {
            self.reverse = !self.reverse;
            self.changed();
        } else if (1..=2).contains(&self.active) {
            if matches!(key.key.as_str(), "up" | "down")
                && !key.modifiers.secondary()
                && !key.modifiers.alt
                && !key.modifiers.control
            {
                let delta = if key.modifiers.shift { 10. } else { 1. }
                    * if key.key == "up" { 1. } else { -1. };
                self.step_offset(self.active - 1, delta);
                return;
            }
            let field = &mut self.fields[self.active - 1];
            let before = field.text.clone();
            field.handle_key(event, FieldMode::SignedDecimal, cx);
            if field.text != before {
                self.changed();
            }
        }
    }

    fn can_apply(&self, tab: Option<&Tab>) -> bool {
        !self.dirty
            && !self.running
            && !self.applying
            && self.options().is_ok()
            && self.ready.is_some()
            && tab.is_some_and(|tab| self.target.matches(tab))
    }
}

impl Workspace {
    pub(in crate::workspace) fn bind_text_path_actions(
        &self,
        root: Div,
        cx: &Context<Self>,
    ) -> Div {
        let idle = self.active_tab().is_some_and(|tab| !tab.busy())
            && self.layer_metadata_preview_blocker().is_none()
            && !self.pointer_gesture_active(cx);
        root.when(idle, |root| {
            root.on_action(cx.listener(
                |this, _: &crate::commands::AlphaToSelection, window, cx| {
                    this.alpha_to_selection(window, cx);
                },
            ))
        })
        .when(idle && self.can_text_along_path(), |root| {
            root.on_action(
                cx.listener(|this, _: &crate::commands::TextAlongPath, window, cx| {
                    this.text_path_form(window, cx);
                }),
            )
        })
    }

    fn can_text_along_path(&self) -> bool {
        self.active_tab().is_some_and(|tab| {
            !tab.busy()
                && tab.document.content.can_add_layer()
                && (self.text_draft.is_some()
                    || tab
                        .document
                        .content
                        .layer(tab.active_layer)
                        .is_some_and(|layer| layer.visible && layer.text.is_some()))
                && tab
                    .document
                    .content
                    .active_path()
                    .is_some_and(|path| path.visible && path.anchors.len() >= 2)
        })
    }

    pub(in crate::workspace) fn alpha_to_selection(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.form.is_some()
            || self.dialog_open
            || self.pdf_job.is_some()
            || self.pointer_gesture_active(cx)
        {
            return;
        }
        let Some(tab) = self.active_tab() else { return };
        let layer = tab.active_layer;
        if self.commit_numeric_with_followup(
            NumericFollowup::TextOperation {
                along_path: false,
                layer,
                path: None,
            },
            window,
            cx,
        ) {
            return;
        }
        if self.finish_text(window, cx) {
            self.defer_after_text(|this, window, cx| this.alpha_to_selection(window, cx));
            return;
        }
        if self.form.is_some() || self.dialog_open || self.refuse_pending_file_edit(cx) {
            return;
        }
        let Some(tab) = self.active_tab().filter(|tab| !tab.busy()) else {
            return;
        };
        let layer = tab.active_layer;
        self.start_change(
            crate::workspace::Change::Selection,
            move |content, _, selection| {
                let Some(alpha) = content.alpha_selection(layer) else {
                    return Dirty::None;
                };
                if *selection == alpha {
                    return Dirty::None;
                }
                *selection = alpha;
                Dirty::Unpainted
            },
            window,
            cx,
        );
    }

    pub(in crate::workspace) fn text_path_form(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pointer_gesture_active(cx) {
            return;
        }
        if self.form.is_some() || self.dialog_open || self.pdf_job.is_some() {
            return;
        }
        let Some(tab) = self.active_tab() else { return };
        let layer = tab.active_layer;
        let path = tab.document.content.active_path_id();
        if self.commit_numeric_with_followup(
            NumericFollowup::TextOperation {
                along_path: true,
                layer,
                path,
            },
            window,
            cx,
        ) {
            return;
        }
        if self.finish_text(window, cx) {
            self.defer_after_text(|this, window, cx| this.text_path_form(window, cx));
            return;
        }
        if self.refuse_pending_file_edit(cx) || !self.can_text_along_path() {
            return;
        }
        let Some(tab) = self.active_tab() else { return };
        let Some(draft) = TextPathDraft::capture(tab) else {
            return;
        };
        self.show_form(FormKind::TextPath(draft), window, cx);
        self.pump_text_path_preview(window, cx);
    }

    pub(super) fn clear_text_path_preview(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form
            && let FormKind::TextPath(draft) = &mut form.kind
        {
            draft.canceled.store(true, Ordering::Relaxed);
            if let Some(image) = draft.preview.take() {
                crate::workspace::drop_images([image], window, cx);
            }
        }
    }

    pub(super) fn pump_text_path_preview(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = &mut self.form else { return };
        let FormKind::TextPath(draft) = &mut form.kind else {
            return;
        };
        if draft.running || !draft.dirty {
            return;
        }
        if let Some(wait) = draft.preview_after.checked_duration_since(Instant::now()) {
            if !draft.preview_timer {
                draft.preview_timer = true;
                let identity = draft.canceled.clone();
                cx.spawn_in(window, async move |workspace, cx| {
                    cx.background_executor().timer(wait).await;
                    let _ = workspace.update_in(cx, |this, window, cx| {
                        if let Some(form) = &mut this.form
                            && let FormKind::TextPath(draft) = &mut form.kind
                            && Arc::ptr_eq(&identity, &draft.canceled)
                        {
                            draft.preview_timer = false;
                            this.pump_text_path_preview(window, cx);
                        }
                    });
                })
                .detach();
            }
            return;
        }
        draft.dirty = false;
        let options = match draft.options() {
            Ok(options) => options,
            Err(error) => {
                draft.error = Some(error.to_string());
                cx.notify();
                return;
            }
        };
        draft.last_offsets = [options.offset, options.baseline];
        draft.running = true;
        draft.preview_slow = false;
        let revision = draft.revision;
        let canceled = draft.canceled.clone();
        let identity = canceled.clone();
        let spec = draft.spec.clone();
        let guide = draft.guide.clone();
        let content = draft.content.clone();
        let target = draft.target;
        let layer = target.layer;
        let edge = crate::effect_preview::display_edge(
            content.size(),
            (296., 220.),
            window.scale_factor(),
        );
        let progress_identity = identity.clone();
        cx.spawn_in(window, async move |workspace, cx| {
            cx.background_executor().timer(PROGRESS_DELAY).await;
            let _ = workspace.update_in(cx, |this, _, cx| {
                if let Some(form) = &mut this.form
                    && let FormKind::TextPath(draft) = &mut form.kind
                    && Arc::ptr_eq(&progress_identity, &draft.canceled)
                    && draft.running
                    && draft.revision == revision
                {
                    draft.preview_slow = true;
                    cx.notify();
                }
            });
        })
        .detach();
        let job = cx.background_executor().spawn(async move {
            let prepared = crate::text::path::prepare(&spec, &guide, options, &canceled)?;
            ensure!(
                content.can_insert_path_text(layer, prepared.size),
                "The curved text does not fit in the layer stack's memory or layer limits."
            );
            let pixels = prepared.preview(content.size(), edge)?;
            ensure!(
                !canceled.load(Ordering::Relaxed),
                "Text placement canceled."
            );
            let context = crate::effect_preview::EffectPreview::new(
                content,
                layer,
                crate::selection::Selection::Unrestricted,
                edge,
            );
            let image = context.replacement(&pixels);
            Ok::<_, anyhow::Error>((
                Arc::new(prepared),
                crate::document::preview_render(&crate::effect_preview::checker(&image)),
            ))
        });
        cx.spawn_in(window, async move |workspace, cx| {
            let result = job.await;
            let _ = workspace.update_in(cx, |this, window, cx| {
                let current = this.active_tab().is_some_and(|tab| target.matches(tab));
                let Some(form) = &mut this.form else {
                    if let Ok((_, image)) = result {
                        crate::workspace::drop_images([image], window, cx);
                    }
                    return;
                };
                let FormKind::TextPath(draft) = &mut form.kind else {
                    if let Ok((_, image)) = result {
                        crate::workspace::drop_images([image], window, cx);
                    }
                    return;
                };
                if !Arc::ptr_eq(&identity, &draft.canceled) {
                    if let Ok((_, image)) = result {
                        crate::workspace::drop_images([image], window, cx);
                    }
                    return;
                }
                draft.running = false;
                draft.preview_slow = false;
                if !current {
                    draft.ready = None;
                    draft.dirty = false;
                    draft.error =
                        Some("The text or guide changed. Close this dialog and try again.".into());
                    if let Ok((_, image)) = result {
                        crate::workspace::drop_images([image], window, cx);
                    }
                    cx.notify();
                    return;
                }
                if revision == draft.revision {
                    match result {
                        Ok((prepared, image)) => {
                            if let Some(old) = draft.preview.replace(image) {
                                crate::workspace::drop_images([old], window, cx);
                            }
                            draft.overflow = prepared.overflow;
                            draft.ready = Some(prepared);
                            draft.error = None;
                        }
                        Err(error) => {
                            draft.error = Some(error.to_string());
                        }
                    }
                } else if let Ok((_, image)) = result {
                    crate::workspace::drop_images([image], window, cx);
                }
                this.pump_text_path_preview(window, cx);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn submit_text_path(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = &self.form else { return };
        let FormKind::TextPath(draft) = &form.kind else {
            return;
        };
        if !draft.can_apply(self.active_tab()) {
            return;
        }
        let prepared = draft.ready.as_ref().unwrap().clone();
        let target = draft.target;
        if !draft
            .content
            .can_insert_path_text(target.layer, prepared.size)
        {
            return;
        }
        let canceled = draft.canceled.clone();
        let identity = canceled.clone();
        let offset = prepared.offset;
        if let Some(form) = &mut self.form
            && let FormKind::TextPath(draft) = &mut form.kind
        {
            draft.applying = true;
            form.error = None;
        }
        let job = cx.background_executor().spawn(async move {
            ensure!(!canceled.load(Ordering::Relaxed), "Text placement canceled.");
            let pixels = prepared.render()?;
            ensure!(pixels.pixels().any(|pixel| pixel[3] != 0), "The rendered text is fully transparent. Increase its size or opacity and try again.");
            ensure!(!canceled.load(Ordering::Relaxed), "Text placement canceled.");
            Ok::<_,anyhow::Error>(pixels)
        });
        cx.spawn_in(window, async move |workspace, cx| {
            let result = job.await;
            let _ = workspace.update_in(cx, |this, window, cx| {
                let current = this.active_tab().is_some_and(|tab| target.matches(tab));
                let Some(form) = &mut this.form else { return };
                let FormKind::TextPath(draft) = &mut form.kind else {
                    return;
                };
                if !Arc::ptr_eq(&identity, &draft.canceled) {
                    return;
                }
                draft.applying = false;
                let result = result.and_then(|pixels| {
                    ensure!(
                        current
                            && draft
                                .content
                                .can_insert_path_text(target.layer, pixels.dimensions()),
                        "The text or guide changed. Close this dialog and try again."
                    );
                    Ok(pixels)
                });
                match result {
                    Err(error) => {
                        form.error = Some(error.to_string());
                        cx.notify();
                    }
                    Ok(pixels) => {
                        draft.applying = true;
                        this.applying_form = Some((target.tab, target.generation));
                        this.edit_content(
                            move |content, active, _| {
                                let Some(id) =
                                    content.insert_path_text(target.layer, offset, pixels)
                                else {
                                    return Dirty::None;
                                };
                                *active = id;
                                Dirty::Canvas
                            },
                            window,
                            cx,
                        );
                    }
                }
            });
        })
        .detach();
        cx.notify();
    }

    fn nudge_text_path_offset(
        &mut self,
        index: usize,
        delta: f64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(form) = &mut self.form
            && let FormKind::TextPath(draft) = &mut form.kind
        {
            if draft.applying {
                return;
            }
            draft.step_offset(index, delta);
            form.error = None;
        }
        self.pump_text_path_preview(window, cx);
        cx.notify();
    }

    pub(super) fn text_path_controls(
        &self,
        draft: &TextPathDraft,
        wide: bool,
        cx: &Context<Self>,
    ) -> Div {
        let preview = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .h(px(220.))
                    .flex_shrink_0()
                    .w_full()
                    .bg(rgb(super::CANVAS))
                    .rounded_md()
                    .when_some(draft.preview.clone(), |view, image| {
                        view.child(img(image).size_full().object_fit(ObjectFit::Contain))
                    }),
            )
            .child(
                div()
                    .h(px(18.))
                    .flex_shrink_0()
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .child(if draft.applying {
                        "Rendering full-size text…"
                    } else if draft.preview_slow && draft.running {
                        "Updating preview…"
                    } else {
                        ""
                    }),
            );
        let alignment = segmented(
            [
                (TextAlign::Left, "Start"),
                (TextAlign::Center, "Center"),
                (TextAlign::Right, "End"),
            ]
            .map(|(align, label)| {
                choice_button(label, draft.align == align).on_click(cx.listener(
                    move |this, _, window, cx| {
                        if let Some(form) = &mut this.form
                            && let FormKind::TextPath(draft) = &mut form.kind
                        {
                            if draft.applying {
                                return;
                            }
                            draft.align = align;
                            draft.active = 0;
                            draft.changed();
                            form.error = None;
                        }
                        this.pump_text_path_preview(window, cx);
                        cx.notify();
                    },
                ))
            }),
        )
        .when(draft.active == 0, |row| row.border_color(rgb(ACCENT)));
        let fields = ["Path offset (px)", "Baseline offset (px)"]
            .into_iter()
            .enumerate()
            .map(|(index, label)| {
                let row = div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .mt_1()
                    .child(
                        step_button(("text-path-minus", index), "−")
                            .tooltip(tooltip("Decrease by 1 px (Shift: 10 px)"))
                            .on_click(cx.listener(
                                move |this, event: &gpui::ClickEvent, window, cx| {
                                    cx.stop_propagation();
                                    this.nudge_text_path_offset(
                                        index,
                                        if event.modifiers().shift { -10. } else { -1. },
                                        window,
                                        cx,
                                    );
                                },
                            )),
                    )
                    .child(
                        draft.fields[index]
                            .view(draft.active == index + 1)
                            .id(("text-path-offset", index))
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(form) = &mut this.form
                                    && let FormKind::TextPath(draft) = &mut form.kind
                                {
                                    if draft.applying {
                                        return;
                                    }
                                    draft.active = index + 1;
                                    cx.notify();
                                }
                            })),
                    )
                    .child(
                        step_button(("text-path-plus", index), "+")
                            .tooltip(tooltip("Increase by 1 px (Shift: 10 px)"))
                            .on_click(cx.listener(
                                move |this, event: &gpui::ClickEvent, window, cx| {
                                    cx.stop_propagation();
                                    this.nudge_text_path_offset(
                                        index,
                                        if event.modifiers().shift { 10. } else { 1. },
                                        window,
                                        cx,
                                    );
                                },
                            )),
                    );
                div().child(label).child(row)
            });
        // Reserve the same geometry for success, overflow, validation failures,
        // and pending previews. Centering the modal must not move its inputs.
        let error = draft
            .error
            .as_ref()
            .or_else(|| self.form.as_ref().and_then(|form| form.error.as_ref()));
        let feedback = div()
            .id("text-path-feedback")
            .h(px(72.))
            .flex_shrink_0()
            .overflow_y_scroll()
            .text_sm()
            .when_some(error, |view, error| {
                view.text_color(rgb(0xffb8ad)).child(error.clone())
            })
            .when(error.is_none() && draft.overflow, |view| {
                view.text_color(rgb(0xffcf8a))
                    .child("Text exceeds the path. Only text that fits will be included.")
            });
        let reverse = form_button("Reverse direction", draft.reverse)
            .when(draft.active == 3, |button| button.border_color(rgb(ACCENT)))
            .on_click(cx.listener(|this, _, window, cx| {
                if let Some(form) = &mut this.form
                    && let FormKind::TextPath(draft) = &mut form.kind
                {
                    if draft.applying {
                        return;
                    }
                    draft.reverse = !draft.reverse;
                    draft.active = 3;
                    draft.changed();
                    form.error = None;
                }
                this.pump_text_path_preview(window, cx);
                cx.notify();
            }));
        let controls = div().flex().flex_col().gap_3()
            .child(div().text_sm().child("Alignment")).child(alignment)
            .children(fields).child(reverse)
            .child(div().text_xs().text_color(rgb(MUTED)).child("Creates a raster layer and hides the original text. The guide stays unchanged. Fixed text boxes are ignored; line breaks are not supported."))
            .child(feedback);
        super::effect_dialog_layout(preview, controls, wide)
    }

    pub(super) fn can_submit_text_path(&self, draft: &TextPathDraft) -> bool {
        draft.can_apply(self.active_tab())
    }
}
