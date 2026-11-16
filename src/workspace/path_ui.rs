use gpui::{Context, Div, MouseButton, Pixels, Point, Window, div, point, prelude::*, px, rgb};

use super::Workspace;
use super::canvas_tools::PreviewGeometry;
use super::numeric::NumericField;
use super::stroke_ui::{PathStrokeStyle, StrokeSource};
use super::tooltip::{help_icon, tooltip};
use super::view::{
    ACCENT, BORDER, DISABLED, HOVER, MUTED, SELECTED, TEXT, choice_button, icon, option_action,
    option_row, section_label, segmented, step_button,
};
use crate::commands::{PathToSelection, SelectionToPath};
use crate::paths::{self, Anchor, AnchorId, HandleMode, PathId, VectorPath};
use crate::preview::Dirty;

const HIT_PIXELS: f32 = 8.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct AnchorSelection {
    pub path: PathId,
    pub ids: Vec<AnchorId>,
}

impl AnchorSelection {
    fn single(path: PathId, anchor: AnchorId) -> Self {
        Self {
            path,
            ids: vec![anchor],
        }
    }

    fn click(selection: &mut Option<Self>, path: PathId, anchor: AnchorId, toggle: bool) {
        if let Some(selected) = selection.as_mut().filter(|selected| selected.path == path) {
            if let Some(index) = selected.ids.iter().position(|id| *id == anchor) {
                if toggle {
                    selected.ids.remove(index);
                    if selected.ids.is_empty() {
                        *selection = None;
                    }
                }
                return;
            }
            if toggle {
                selected.ids.push(anchor);
                return;
            }
        }
        *selection = Some(Self::single(path, anchor));
    }

    fn anchors(&self, path: &VectorPath) -> Vec<Anchor> {
        if path.id != self.path || !path.visible {
            return Vec::new();
        }
        path.anchors
            .iter()
            .filter(|anchor| self.ids.contains(&anchor.id))
            .copied()
            .collect()
    }
}

pub(super) struct PathGesture {
    pub path: VectorPath,
    baseline: Option<VectorPath>,
    edit: PathEdit,
}

enum PathEdit {
    Place {
        anchor: AnchorId,
        origin: (f64, f64),
    },
    MovePoints {
        anchor: AnchorId,
        bases: Vec<Anchor>,
        down: (f64, f64),
    },
    MoveHandle {
        anchor: AnchorId,
        incoming: bool,
        grab: (f64, f64),
        mode: HandleMode,
    },
}

impl PathGesture {
    fn update(&mut self, point: (f64, f64)) {
        match &self.edit {
            PathEdit::Place { anchor, origin } => {
                let Some(anchor) = self.path.anchor_mut(*anchor) else {
                    return;
                };
                let delta = (point.0 - origin.0, point.1 - origin.1);
                anchor.handle_out = point;
                anchor.handle_in = (origin.0 - delta.0, origin.1 - delta.1);
            }
            PathEdit::MovePoints { bases, down, .. } => {
                self.path
                    .move_anchors_from(bases, (point.0 - down.0, point.1 - down.1));
            }
            PathEdit::MoveHandle {
                anchor,
                incoming,
                grab,
                mode,
            } => {
                let next = (point.0 + grab.0, point.1 + grab.1);
                let Some(anchor) = self.path.anchor_mut(*anchor) else {
                    return;
                };
                let Some(base) = self
                    .baseline
                    .as_ref()
                    .and_then(|path| path.anchor(anchor.id))
                    .copied()
                else {
                    return;
                };
                *anchor = base;
                anchor.move_handle(*incoming, next, *mode);
            }
        }
    }

    fn close_on_click(&mut self, point: (f64, f64), radius: f64) {
        if let PathEdit::MovePoints {
            anchor,
            down,
            bases,
        } = &self.edit
            && bases.len() == 1
            && let Some(base) = self.baseline.as_ref()
            && !base.closed
            && base.anchors.len() >= 3
            && base.anchors[0].id == *anchor
            && (point.0 - down.0).hypot(point.1 - down.1) <= radius
        {
            self.path = base.clone();
            self.path.closed = true;
        }
    }
}

enum Target {
    Handle {
        path: PathId,
        anchor: AnchorId,
        incoming: bool,
        at: (f64, f64),
    },
    Anchor {
        path: PathId,
        anchor: AnchorId,
    },
}

impl Workspace {
    pub(super) fn path_edit_options(&self, cx: &Context<Self>) -> Div {
        let count = self
            .selected_path_anchors()
            .map_or(0, |(_, anchors)| anchors.len());
        let ready = self.path_gesture.is_none() && self.active_tab().is_some_and(|tab| !tab.busy());
        let enabled = ready && count > 0;
        let has_path = ready
            && self.active_tab().is_some_and(|tab| {
                tab.document
                    .content
                    .active_path()
                    .is_some_and(|path| path.visible && !path.anchors.is_empty())
            });
        div().flex().flex_col().gap_2()
            .child(section_label("HANDLE DRAG"))
            .child(segmented([
                ("Independent", HandleMode::Independent), ("Aligned", HandleMode::Aligned), ("Symmetric", HandleMode::Symmetric),
            ].map(|(label, mode)| choice_button(label, self.preferences.path_handle_mode == mode).on_click(cx.listener(move |this, _, _, cx| {
                if this.path_gesture.is_none() { this.preferences.path_handle_mode = mode; cx.notify(); }
            })))))
            .child(div().flex().justify_between().child(section_label("SELECTED ANCHORS")).child(div().text_size(px(11.)).text_color(rgb(MUTED)).child(count.to_string())))
            .child(div().flex().gap_2().children([
                ("All Anchors", true, has_path), ("Deselect", false, enabled),
            ].map(|(label, all, available)| option_action(label, false).flex_1()
                .when(available, |button| button.on_click(cx.listener(move |this, _, _, cx| this.select_path_anchors(all, cx))))
                .when(!available, |button| button.opacity(0.45).cursor(gpui::CursorStyle::Arrow)))))
            .child(div().flex().gap_2().children([
                ("Corner", false, "Collapse both handles at the selected anchors"),
                ("Smooth", true, "Align handles, preserving their lengths or creating them from neighboring points"),
            ].map(|(label, smooth, help)| option_action(label, false).flex_1().tooltip(tooltip(help))
                .when(enabled, |button| button.on_click(cx.listener(move |this, _, window, cx| this.shape_selected_anchor(smooth, window, cx))))
                .when(!enabled, |button| button.opacity(0.45).cursor(gpui::CursorStyle::Arrow)))))
    }

    fn selected_path_anchors(&self) -> Option<(PathId, Vec<Anchor>)> {
        let selected = self.path_selection.as_ref()?;
        let tab = self.active_tab()?;
        let path = self
            .path_gesture
            .as_ref()
            .map(|gesture| &gesture.path)
            .or(self.path_hold.as_ref())
            .or_else(|| tab.document.content.active_path())?;
        let anchors = selected.anchors(path);
        (!anchors.is_empty()).then_some((path.id, anchors))
    }

    fn select_path_anchors(&mut self, all: bool, cx: &mut Context<Self>) {
        if self.path_gesture.is_some() || self.active_tab().is_none_or(|tab| tab.busy()) {
            return;
        }
        self.path_selection = if all {
            self.active_tab()
                .and_then(|tab| tab.document.content.active_path())
                .filter(|path| path.visible && !path.anchors.is_empty())
                .map(|path| AnchorSelection {
                    path: path.id,
                    ids: path.anchors.iter().map(|a| a.id).collect(),
                })
        } else {
            None
        };
        cx.notify();
    }

    fn shape_selected_anchor(&mut self, smooth: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.path_gesture.is_some() || self.form.is_some() || self.dialog_open {
            return;
        }
        let Some((id, anchors)) = self.selected_path_anchors() else {
            return;
        };
        let Some(tab) = self.active_tab().filter(|tab| !tab.busy()) else {
            return;
        };
        let Some(mut path) = tab
            .document
            .content
            .active_path()
            .filter(|path| path.id == id && path.visible)
            .cloned()
        else {
            return;
        };
        let mut changed = false;
        for anchor in anchors {
            changed |= if smooth {
                path.make_smooth(anchor.id)
            } else {
                path.make_corner(anchor.id)
            };
        }
        if !changed {
            return;
        }
        self.path_hold = Some(path.clone());
        self.edit_content(
            move |content, _, _| {
                if content.upsert_path(path) {
                    Dirty::Unpainted
                } else {
                    Dirty::None
                }
            },
            window,
            cx,
        );
    }

    pub(super) fn bind_path_actions(&self, root: Div, cx: &Context<Self>) -> Div {
        let available = self.can_convert_visible_paths();
        let selection = self.active_tab().is_some_and(|tab| {
            !tab.busy()
                && tab.document.content.paths().len() < crate::paths::MAX_PATHS
                && !matches!(
                    tab.selection,
                    crate::selection::Selection::Unrestricted | crate::selection::Selection::Empty
                )
        });
        root.when(available, |root| {
            root.on_action(cx.listener(|this, _: &PathToSelection, window, cx| {
                this.path_to_selection(window, cx);
            }))
        })
        .when(selection, |root| {
            root.on_action(cx.listener(|this, _: &SelectionToPath, window, cx| {
                this.selection_to_path(window, cx);
            }))
        })
    }

    pub(super) fn path_panel(&self, cx: &Context<Self>) -> Div {
        self.path_options(div().flex().flex_col().gap_3().p_4(), cx)
    }

    fn path_visibility_button(
        &self,
        tab: usize,
        id: PathId,
        visible: bool,
        compact: bool,
        cx: &Context<Self>,
    ) -> gpui::Stateful<Div> {
        let enabled = self.form.is_none()
            && !self.dialog_open
            && self.pdf_job.is_none()
            && self.active_tab().is_some_and(|tab| !tab.busy())
            && self.layer_metadata_preview_blocker().is_none();
        let button = div()
            .id((
                if compact {
                    "compact-path-eye"
                } else {
                    "path-eye"
                },
                id.0,
            ))
            .size(px(22.))
            .flex_shrink_0()
            .rounded_sm()
            .flex()
            .items_center()
            .justify_center()
            .text_color(rgb(if visible { MUTED } else { DISABLED }))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .tooltip(tooltip(
                if self.layer_metadata_preview_blocker().is_some() {
                    "Apply or cancel the current canvas edit before changing visibility"
                } else if visible {
                    "Hide path"
                } else {
                    "Show path"
                },
            ))
            .when_else(
                enabled,
                |button| {
                    button
                        .cursor_pointer()
                        .hover(|button| button.bg(rgb(HOVER)).text_color(rgb(TEXT)))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            this.edit_path_visibility(tab, id, !visible, window, cx);
                        }))
                },
                |button| button.opacity(0.4),
            )
            .child(icon(if visible { "eye" } else { "eye-off" }, 14.));
        if compact && enabled {
            super::compact_inspector::compact_control(button)
        } else {
            button
        }
    }

    pub(super) fn compact_paths(&self, cx: &Context<Self>) -> Div {
        let panel = div().flex().flex_col().gap_1().p(px(5.));
        let Some(tab) = self
            .active_tab()
            .filter(|tab| !tab.document.content.paths().is_empty())
        else {
            return panel.child(
                div()
                    .text_size(px(10.))
                    .text_color(rgb(MUTED))
                    .text_center()
                    .child("—"),
            );
        };
        let tab_id = tab.id;
        let active = tab.document.content.active_path_id();
        panel.children(
            tab.document
                .content
                .paths()
                .iter()
                .enumerate()
                .map(|(index, path)| {
                    let id = path.id;
                    let visible = path.visible;
                    super::compact_inspector::compact_control(
                        div()
                            .id(("compact-path", id.0))
                            .w_full()
                            .h(px(28.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(if active == Some(id) { ACCENT } else { BORDER }))
                            .when(active == Some(id), |row| row.bg(rgb(SELECTED)))
                            .hover(|row| row.bg(rgb(HOVER)))
                            .cursor_pointer()
                            .tooltip(tooltip(format!(
                                "Path {} · {} anchors",
                                index + 1,
                                path.anchors.len()
                            ))),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.active_tab().is_some_and(|tab| tab.id == tab_id) {
                            this.select_path(id, cx);
                        }
                    }))
                    .child(div().flex_1().h_full())
                    .child(self.path_visibility_button(tab_id, id, visible, true, cx))
                }),
        )
    }

    pub(super) fn path_options(&self, panel: Div, cx: &Context<Self>) -> Div {
        let tab = self.active_tab();
        let tab_id = tab.map_or(0, |tab| tab.id);
        let paths = tab.map_or(&[][..], |tab| tab.document.content.paths());
        let active = tab.and_then(|tab| tab.document.content.active_path_id());
        let rows = paths.iter().enumerate().map(|(index, path)| {
            let id = path.id;
            let visible = path.visible;
            let selected = active == Some(id);
            div()
                .id(("path-row", id.0))
                .flex()
                .items_center()
                .gap_1()
                .px_1()
                .py_1()
                .rounded_md()
                .when(selected, |row| row.bg(rgb(SELECTED)))
                .hover(|row| row.bg(rgb(HOVER)))
                .cursor_pointer()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_ellipsis()
                        .text_color(rgb(if visible { TEXT } else { MUTED }))
                        .child(format!(
                            "Path {} · {} anchors",
                            index + 1,
                            path.anchors.len()
                        )),
                )
                .child(self.path_visibility_button(tab_id, id, visible, false, cx))
                .child(
                    div()
                        .id(("path-delete", id.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(22.))
                        .flex_shrink_0()
                        .rounded_sm()
                        .text_color(rgb(MUTED))
                        .hover(|button| button.bg(rgb(HOVER)).text_color(rgb(TEXT)))
                        .tooltip(tooltip("Delete path"))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            this.delete_path(id, window, cx);
                        }))
                        .child(icon("trash", 14.)),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.select_path(id, cx);
                }))
        });
        let enabled = self.can_stroke_outline(StrokeSource::Path)
            && self.preferences.brushes.outline.opacity > 0;
        let style = self.preferences.path_stroke_style;
        let selection = tab.is_some_and(|tab| {
            !tab.busy()
                && tab.document.content.paths().len() < crate::paths::MAX_PATHS
                && !matches!(
                    tab.selection,
                    crate::selection::Selection::Unrestricted | crate::selection::Selection::Empty
                )
        });
        panel
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(section_label("PATHS"))
                    .child(help_icon(
                        "paths-help",
                        "Paths",
                        crate::commands::Tool::Path.help(),
                    )),
            )
            .child(div().flex().flex_col().gap_1().children(rows))
            .when(paths.is_empty(), |panel| {
                panel.child(div().text_color(rgb(MUTED)).child("No paths yet."))
            })
            .child(
                option_action("Visible Paths to Selection", false)
                    .w_full()
                    .when(self.can_convert_visible_paths(), |button| {
                        button.on_click(cx.listener(|this, _, window, cx| {
                            this.path_to_selection(window, cx);
                        }))
                    })
                    .when(!self.can_convert_visible_paths(), |button| {
                        button.opacity(0.45).cursor(gpui::CursorStyle::Arrow)
                    }),
            )
            .child(
                option_action("Selection to Path", false)
                    .w_full()
                    .when(selection, |button| {
                        button.on_click(cx.listener(|this, _, window, cx| {
                            this.selection_to_path(window, cx);
                        }))
                    })
                    .when(!selection, |button| {
                        button.opacity(0.45).cursor(gpui::CursorStyle::Arrow)
                    }),
            )
            .child(section_label("STROKE"))
            .child(segmented([
                choice_button("Pencil", style == PathStrokeStyle::Pencil).on_click(cx.listener(
                    |this, _, _, cx| {
                        this.set_path_stroke_style(PathStrokeStyle::Pencil, cx);
                    },
                )),
                choice_button("Brush", style == PathStrokeStyle::Brush).on_click(cx.listener(
                    |this, _, _, cx| {
                        this.set_path_stroke_style(PathStrokeStyle::Brush, cx);
                    },
                )),
            ]))
            .child(option_row(
                "Size",
                self.numeric_stepper(
                    NumericField::PathStrokeSize,
                    step_button("path-stroke-down", "−").on_click(cx.listener(|this, _, _, cx| {
                        this.preferences.brushes.outline.diameter = this
                            .preferences
                            .brushes
                            .outline
                            .diameter
                            .saturating_sub(1)
                            .max(1);
                        cx.notify();
                    })),
                    step_button("path-stroke-up", "+").on_click(cx.listener(|this, _, _, cx| {
                        this.preferences.brushes.outline.diameter = this
                            .preferences
                            .brushes
                            .outline
                            .diameter
                            .saturating_add(1)
                            .min(500);
                        cx.notify();
                    })),
                    cx,
                ),
            ))
            .child(
                option_action("Stroke", enabled)
                    .w_full()
                    .when(enabled, |button| {
                        button.on_click(cx.listener(|this, _, window, cx| {
                            this.stroke_outline(StrokeSource::Path, window, cx);
                        }))
                    })
                    .when(!enabled, |button| {
                        button.opacity(0.45).cursor(gpui::CursorStyle::Arrow)
                    }),
            )
    }

    fn select_path(&mut self, id: PathId, cx: &mut Context<Self>) {
        if let Some(index) = self.active
            && !self.tabs[index].busy()
            && self.tabs[index].document.content.select_path(id)
        {
            self.path_selection = None;
            self.path_gesture = None;
            self.path_hold = None;
            self.tabs[index].notice = None;
            cx.notify();
        }
    }

    pub(super) fn edit_path_visibility(
        &mut self,
        tab_id: usize,
        id: PathId,
        visible: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.active_tab().is_none_or(|tab| {
            tab.id != tab_id
                || tab.busy()
                || !tab
                    .document
                    .content
                    .paths()
                    .iter()
                    .any(|path| path.id == id)
        }) || self.form.is_some()
            || self.dialog_open
            || self.pdf_job.is_some()
            || self.refuse_layer_metadata_preview(cx)
        {
            return;
        }
        if self.commit_numeric_with_followup(
            super::numeric::NumericFollowup::PathVisibility {
                tab: tab_id,
                path: id,
                visible,
            },
            window,
            cx,
        ) {
            return;
        }
        if self.finish_text(window, cx) {
            self.defer_after_text(move |this, window, cx| {
                this.edit_path_visibility(tab_id, id, visible, window, cx)
            });
            return;
        }
        if self.refuse_layer_metadata_preview(cx) {
            return;
        }
        if !visible
            && self
                .path_selection
                .as_ref()
                .is_some_and(|selected| selected.path == id)
            && self.active_tab().is_some_and(|tab| !tab.busy())
        {
            self.clear_path_edit();
        }
        self.edit_content(
            move |content, _, _| {
                if content.set_path_visible(id, visible) {
                    Dirty::Unpainted
                } else {
                    Dirty::None
                }
            },
            window,
            cx,
        );
    }

    fn delete_path(&mut self, id: PathId, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .path_selection
            .as_ref()
            .is_some_and(|selected| selected.path == id)
            && self.active_tab().is_some_and(|tab| !tab.busy())
        {
            self.clear_path_edit();
        }
        self.edit_content(
            move |content, _, _| {
                if content.delete_path(id) {
                    Dirty::Unpainted
                } else {
                    Dirty::None
                }
            },
            window,
            cx,
        );
    }

    pub(super) fn displayed_paths(&self) -> Vec<VectorPath> {
        let Some(tab) = self.active_tab() else {
            return Vec::new();
        };
        let mut paths: Vec<_> = tab
            .document
            .content
            .paths()
            .iter()
            .filter(|path| path.visible)
            .cloned()
            .collect();
        let overlay = self
            .path_gesture
            .as_ref()
            .map(|gesture| &gesture.path)
            .or_else(|| self.path_hold.as_ref().filter(|hold| !paths.contains(hold)));
        if let Some(overlay) = overlay {
            if let Some(existing) = paths.iter_mut().find(|path| path.id == overlay.id) {
                *existing = overlay.clone();
            } else {
                paths.push(overlay.clone());
            }
        }
        paths
    }

    pub(super) fn displayed_active_path(&self) -> Option<PathId> {
        self.path_gesture
            .as_ref()
            .map(|gesture| gesture.path.id)
            .or_else(|| self.active_tab()?.document.content.active_path_id())
    }

    pub(super) fn begin_path(
        &mut self,
        position: Point<Pixels>,
        scale: f32,
        insert: bool,
        toggle: bool,
        cx: &mut Context<Self>,
    ) {
        if self.active_tab().is_none_or(|tab| tab.busy()) || self.path_gesture.is_some() {
            return;
        }
        let Some(point) = self.canvas_point(position, scale) else {
            return;
        };
        if let Some(target) = self.path_target(position, scale) {
            let path_id = match target {
                Target::Handle { path, .. } | Target::Anchor { path, .. } => path,
            };
            let Some(path) = self
                .active_tab()
                .and_then(|tab| {
                    tab.document
                        .content
                        .paths()
                        .iter()
                        .find(|path| path.id == path_id)
                })
                .cloned()
            else {
                return;
            };
            let anchor_id = match target {
                Target::Handle { anchor, .. } | Target::Anchor { anchor, .. } => anchor,
            };
            let toggle_anchor = toggle && matches!(target, Target::Anchor { .. });
            AnchorSelection::click(&mut self.path_selection, path.id, anchor_id, toggle_anchor);
            if let Some(index) = self.active {
                self.tabs[index].document.content.select_path(path.id);
            }
            if toggle_anchor {
                cx.notify();
                return;
            }
            let edit = match target {
                Target::Handle {
                    anchor,
                    incoming,
                    at,
                    ..
                } => PathEdit::MoveHandle {
                    anchor,
                    incoming,
                    grab: (at.0 - point.0, at.1 - point.1),
                    mode: self.preferences.path_handle_mode,
                },
                Target::Anchor { anchor, .. } => {
                    let bases = self
                        .path_selection
                        .as_ref()
                        .map(|selected| selected.anchors(&path))
                        .unwrap_or_default();
                    PathEdit::MovePoints {
                        anchor,
                        bases,
                        down: point,
                    }
                }
            };
            self.path_gesture = Some(PathGesture {
                baseline: Some(path.clone()),
                path,
                edit,
            });
            cx.notify();
            return;
        }
        if toggle && !insert {
            return;
        }
        if insert {
            let Some(tab) = self.active_tab() else {
                return;
            };
            let active = tab.document.content.active_path_id();
            let radius = self.path_hit_radius(scale);
            let mut nearest = None;
            for path in tab
                .document
                .content
                .paths()
                .iter()
                .filter(|path| path.visible && Some(path.id) == active)
                .chain(
                    tab.document
                        .content
                        .paths()
                        .iter()
                        .filter(|path| path.visible && Some(path.id) != active),
                )
            {
                if let Some(hit) = path.nearest_segment(point, radius)
                    && nearest.as_ref().is_none_or(
                        |(_, best): &(&VectorPath, paths::SegmentHit)| hit.distance < best.distance,
                    )
                {
                    nearest = Some((path, hit));
                }
            }
            let Some((baseline, hit)) = nearest else {
                self.notice("Alt-click a visible curve to insert an anchor.", cx);
                return;
            };
            if baseline.anchors.len() >= paths::MAX_ANCHORS {
                self.notice("This path has enough anchors.", cx);
                return;
            }
            let Some(id) = tab.document.content.next_anchor_id() else {
                return;
            };
            let baseline = baseline.clone();
            let mut path = baseline.clone();
            let Some(anchor) = path.split_segment(hit.segment, hit.t, id) else {
                return;
            };
            self.path_selection = Some(AnchorSelection::single(path.id, id));
            self.path_gesture = Some(PathGesture {
                baseline: Some(baseline),
                path,
                edit: PathEdit::MovePoints {
                    anchor: id,
                    bases: vec![anchor],
                    down: point,
                },
            });
            cx.notify();
            return;
        }
        let Some((anchor_id, path_id, path_count, appending)) = self.active_tab().map(|tab| {
            let content = &tab.document.content;
            (
                content.next_anchor_id(),
                content.next_path_id(),
                content.paths().len(),
                content.active_path().filter(|path| !path.closed).cloned(),
            )
        }) else {
            return;
        };
        let (Some(anchor_id), Some(path_id)) = (anchor_id, path_id) else {
            return;
        };
        if let Some(active) = appending {
            if active.anchors.len() >= paths::MAX_ANCHORS {
                self.notice("This path has enough anchors.", cx);
                return;
            }
            let mut path = active.clone();
            path.anchors.push(Anchor::corner(anchor_id, point));
            self.path_selection = Some(AnchorSelection::single(path.id, anchor_id));
            self.path_gesture = Some(PathGesture {
                baseline: Some(active),
                edit: PathEdit::Place {
                    anchor: anchor_id,
                    origin: point,
                },
                path,
            });
        } else if path_count >= paths::MAX_PATHS {
            self.notice("The document has enough paths.", cx);
            return;
        } else {
            let path = VectorPath {
                id: path_id,
                visible: true,
                closed: false,
                anchors: vec![Anchor::corner(anchor_id, point)],
            };
            self.path_selection = Some(AnchorSelection::single(path_id, anchor_id));
            self.path_gesture = Some(PathGesture {
                baseline: None,
                edit: PathEdit::Place {
                    anchor: anchor_id,
                    origin: point,
                },
                path,
            });
        }
        cx.notify();
    }

    pub(super) fn extend_path(
        &mut self,
        position: Point<Pixels>,
        scale: f32,
        cx: &mut Context<Self>,
    ) {
        let Some(point) = self.canvas_point(position, scale) else {
            return;
        };
        let Some(gesture) = self.path_gesture.as_mut() else {
            return;
        };
        gesture.update(point);
        cx.notify();
    }

    pub(super) fn finish_path(
        &mut self,
        position: Point<Pixels>,
        scale: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.extend_path(position, scale, cx);
        let Some(mut gesture) = self.path_gesture.take() else {
            return;
        };
        if let Some(point) = self.canvas_point(position, scale) {
            gesture.close_on_click(point, self.path_hit_radius(scale));
        }
        let path = gesture.path;
        if gesture.baseline.as_ref() == Some(&path)
            && self
                .active_document()
                .is_some_and(|doc| doc.content.active_path_id() == Some(path.id))
        {
            return;
        }
        self.path_hold = Some(path.clone());
        self.edit_content(
            move |content, _, _| {
                if content.upsert_path(path) {
                    Dirty::Unpainted
                } else {
                    Dirty::None
                }
            },
            window,
            cx,
        );
    }

    pub(super) fn end_path_session(&mut self, cx: &mut Context<Self>) {
        let gesture = self.path_gesture.take().is_some();
        let hold = self.path_hold.take().is_some();
        let selection = self.path_selection.take().is_some();
        let active = self
            .active
            .is_some_and(|index| self.tabs[index].document.content.clear_active_path());
        if gesture || hold || selection || active {
            cx.notify();
        }
    }

    pub(super) fn cancel_path_gesture(&mut self, cx: &mut Context<Self>) {
        if self.path_gesture.take().is_some() {
            // A canceled insertion/placement has no live anchor. Keep only a
            // selection that still belongs to the document, before IDs can be reused.
            self.path_selection = self.path_selection.take().and_then(|mut selected| {
                let path = self
                    .active_tab()?
                    .document
                    .content
                    .paths()
                    .iter()
                    .find(|path| path.id == selected.path && path.visible)?;
                selected.ids.retain(|id| path.anchor(*id).is_some());
                (!selected.ids.is_empty()).then_some(selected)
            });
            cx.notify();
        }
    }

    pub(super) fn clear_path_edit(&mut self) {
        self.path_gesture = None;
        self.path_hold = None;
        self.path_selection = None;
    }

    pub(super) fn close_active_path(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(mut gesture) = self.path_gesture.take() {
            if gesture.path.anchors.len() < 3 {
                self.path_gesture = Some(gesture);
                return;
            }
            gesture.path.closed = true;
            let path = gesture.path;
            self.path_hold = Some(path.clone());
            self.edit_content(
                move |content, _, _| {
                    if content.upsert_path(path) {
                        Dirty::Unpainted
                    } else {
                        Dirty::None
                    }
                },
                window,
                cx,
            );
            return;
        }
        let Some(id) = self.displayed_active_path() else {
            return;
        };
        self.edit_content(
            move |content, _, _| {
                if content.close_path(id) {
                    Dirty::Unpainted
                } else {
                    Dirty::None
                }
            },
            window,
            cx,
        );
    }

    pub(super) fn delete_path_anchor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.active_tab().is_none_or(|tab| tab.busy()) {
            return;
        }
        self.path_gesture = None;
        let Some(tab) = self.active_tab() else {
            return;
        };
        let (path_id, anchor_ids) = if self.path_selection.is_some() {
            let Some((path, anchors)) = self.selected_path_anchors() else {
                self.path_selection = None;
                cx.notify();
                return;
            };
            (
                path,
                anchors.iter().map(|anchor| anchor.id).collect::<Vec<_>>(),
            )
        } else if let Some(path) = tab
            .document
            .content
            .active_path()
            .filter(|path| path.visible)
        {
            let Some(anchor) = path.anchors.last() else {
                return;
            };
            (path.id, vec![anchor.id])
        } else {
            return;
        };
        self.path_selection = None;
        self.edit_content(
            move |content, _, _| {
                if content.delete_anchors(path_id, &anchor_ids) {
                    Dirty::Unpainted
                } else {
                    Dirty::None
                }
            },
            window,
            cx,
        );
    }

    pub(super) fn selection_to_path(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.active_tab().is_none_or(|tab| {
            tab.busy()
                || tab.document.content.paths().len() >= crate::paths::MAX_PATHS
                || matches!(
                    tab.selection,
                    crate::selection::Selection::Unrestricted | crate::selection::Selection::Empty
                )
        }) {
            return;
        }
        self.edit_content(
            |content, _, selection| {
                let paths = content.paths_from_selection(selection);
                let mut changed = false;
                for path in paths {
                    if !content.upsert_path(path) {
                        break;
                    }
                    changed = true;
                }
                if changed {
                    Dirty::Unpainted
                } else {
                    Dirty::None
                }
            },
            window,
            cx,
        );
    }

    fn can_convert_visible_paths(&self) -> bool {
        self.active_tab().is_some_and(|tab| {
            !tab.busy()
                && tab
                    .document
                    .content
                    .paths()
                    .iter()
                    .any(|path| path.visible && path.closed && path.anchors.len() >= 3)
        })
    }

    pub(super) fn path_to_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_convert_visible_paths() {
            return;
        }
        self.start_change(
            super::Change::Selection,
            |content, _, selection| {
                let next = content.path_selection();
                if *selection == next {
                    return Dirty::None;
                }
                *selection = next;
                Dirty::Unpainted
            },
            window,
            cx,
        );
    }

    fn path_hit_radius(&self, scale: f32) -> f64 {
        let Some(tab) = self.active_tab() else {
            return 6.0;
        };
        let Some(geometry) = PreviewGeometry::for_tab(tab, self.viewport.get(), scale) else {
            return 6.0;
        };
        let width = f64::from(tab.document.width());
        let screen = f64::from(geometry.bounds.size.width);
        if screen <= 0.0 {
            return 6.0;
        }
        f64::from(HIT_PIXELS) * width / screen
    }

    fn path_target(&self, position: Point<Pixels>, scale: f32) -> Option<Target> {
        let tab = self.active_tab()?;
        let geometry = PreviewGeometry::for_tab(tab, self.viewport.get(), scale)?;
        let (width, height) = tab.document.content.size();
        if width == 0 || height == 0 {
            return None;
        }
        let paths: Vec<_> = tab
            .document
            .content
            .paths()
            .iter()
            .filter(|path| path.visible)
            .cloned()
            .collect();
        let active = tab.document.content.active_path_id();
        let paths = paths.as_slice();
        let mut ordered: Vec<&VectorPath> = paths
            .iter()
            .filter(|path| Some(path.id) == active)
            .collect();
        ordered.extend(paths.iter().filter(|path| Some(path.id) != active));
        let near = |at: (f64, f64)| -> bool {
            let projected = geometry.project(point(
                (at.0 / f64::from(width)) as f32,
                (at.1 / f64::from(height)) as f32,
            ));
            let dx = f32::from(projected.x - position.x);
            let dy = f32::from(projected.y - position.y);
            dx * dx + dy * dy <= HIT_PIXELS * HIT_PIXELS
        };
        for path in &ordered {
            for anchor in &path.anchors {
                if anchor.handle_in != anchor.point && near(anchor.handle_in) {
                    return Some(Target::Handle {
                        path: path.id,
                        anchor: anchor.id,
                        incoming: true,
                        at: anchor.handle_in,
                    });
                }
                if anchor.handle_out != anchor.point && near(anchor.handle_out) {
                    return Some(Target::Handle {
                        path: path.id,
                        anchor: anchor.id,
                        incoming: false,
                        at: anchor.handle_out,
                    });
                }
            }
        }
        for path in &ordered {
            for anchor in &path.anchors {
                if near(anchor.point) {
                    return Some(Target::Anchor {
                        path: path.id,
                        anchor: anchor.id,
                    });
                }
            }
        }
        None
    }
}
