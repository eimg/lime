use gpui::{Context, Div, MouseButton, Pixels, Point, Window, div, prelude::*, px, rgb};

use super::Workspace;
use super::canvas_tools::PreviewGeometry;
use super::numeric::NumericField;
use super::stroke_ui::{PathStrokeStyle, StrokeSource};
use super::tooltip::{help_icon, tooltip};
use super::view::{
    ACCENT, BORDER, CHROME, DISABLED, HOVER, MUTED, SELECTED, TEXT, choice_button, icon,
    option_action, option_row, section_label, segmented, step_button,
};
use crate::commands::{PathToSelection, SelectionToPath};
use crate::paths::{self, Anchor, AnchorId, HandleMode, PathId, VectorPath};
use crate::preview::Dirty;

/// Tool departure must also finish a path whose last anchor is still committing.
#[derive(Clone, Copy)]
pub(super) struct PendingPathFinish {
    pub generation: u64,
    pub layer: crate::document::LayerId,
    pub tool: crate::commands::Tool,
    path: PathId,
}

impl PendingPathFinish {
    pub(super) fn apply(self, content: &mut crate::document::DocumentContent) {
        if content.active_path_id() == Some(self.path) {
            content.clear_active_path();
        }
    }
}

const HIT_PIXELS: f32 = 8.0;

pub(super) fn node_selected(
    active: Option<PathId>,
    selection: Option<&AnchorSelection>,
    path: PathId,
    anchor: AnchorId,
) -> bool {
    active == Some(path) && selection.is_some_and(|s| s.path == path && s.ids.contains(&anchor))
}

pub(super) fn node_marker_size(selected: bool) -> f32 {
    if selected { 9. } else { 7. }
}

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
        bases: Vec<Anchor>,
        down: (f64, f64),
    },
    ShapeAnchor {
        anchor: AnchorId,
        down: (f64, f64),
        threshold: f64,
        dragged: bool,
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
        match &mut self.edit {
            PathEdit::Place { anchor, origin } => {
                let Some(anchor) = self.path.anchor_mut(*anchor) else {
                    return;
                };
                let delta = (point.0 - origin.0, point.1 - origin.1);
                anchor.handle_out = point;
                anchor.handle_in = (origin.0 - delta.0, origin.1 - delta.1);
            }
            PathEdit::MovePoints { bases, down } => {
                self.path
                    .move_anchors_from(bases, (point.0 - down.0, point.1 - down.1));
            }
            PathEdit::ShapeAnchor {
                anchor,
                down,
                threshold,
                dragged,
            } => {
                let delta = (point.0 - down.0, point.1 - down.1);
                *dragged |= delta.0.hypot(delta.1) > *threshold;
                if *dragged && let Some(anchor) = self.path.anchor_mut(*anchor) {
                    anchor.handle_out = (anchor.point.0 + delta.0, anchor.point.1 + delta.1);
                    anchor.handle_in = (anchor.point.0 - delta.0, anchor.point.1 - delta.1);
                }
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

    fn close_on_click(&mut self) {
        if let PathEdit::ShapeAnchor {
            anchor,
            dragged: false,
            ..
        } = &self.edit
            && !self.path.closed
            && self.path.anchors.len() >= 3
            && self.path.anchors[0].id == *anchor
        {
            self.path.closed = true;
        }
    }

    fn for_target(
        path: VectorPath,
        target: Target,
        selection: &AnchorSelection,
        down: (f64, f64),
        modifiers: gpui::Modifiers,
        threshold: f64,
        mode: HandleMode,
    ) -> Self {
        let edit = match target {
            Target::Handle {
                anchor,
                incoming,
                at,
                ..
            } => PathEdit::MoveHandle {
                anchor,
                incoming,
                grab: (at.0 - down.0, at.1 - down.1),
                mode,
            },
            Target::Anchor { anchor, .. } if modifiers.secondary() => PathEdit::ShapeAnchor {
                anchor,
                down,
                threshold,
                dragged: false,
            },
            Target::Anchor { .. } => PathEdit::MovePoints {
                bases: selection.anchors(&path),
                down,
            },
        };
        Self {
            baseline: Some(path.clone()),
            path,
            edit,
        }
    }
}

// A selected hidden path may stay selected in the panel, but canvas creation
// must start a visible path rather than silently extending invisible geometry.
fn continuation_path(content: &crate::document::DocumentContent) -> Option<&VectorPath> {
    content
        .active_path()
        .filter(|path| path.visible && !path.closed)
}

// Only an endpoint can extend an open component; editing an interior node must
// not silently append to an unrelated endpoint or change the path topology.
fn continuation_index(path: &VectorPath, selection: Option<&AnchorSelection>) -> Option<usize> {
    if !path.visible || path.closed {
        return None;
    }
    let Some(selection) = selection.filter(|selection| selection.path == path.id) else {
        return Some(path.anchors.len());
    };
    let [anchor] = selection.ids.as_slice() else {
        return None;
    };
    if path.anchors.last().is_some_and(|last| last.id == *anchor) {
        Some(path.anchors.len())
    } else if path
        .anchors
        .first()
        .is_some_and(|first| first.id == *anchor)
    {
        Some(0)
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
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

impl Target {
    fn nearest(
        paths: &[VectorPath],
        active: Option<PathId>,
        selection: Option<&AnchorSelection>,
        point: (f64, f64),
        radius: f64,
    ) -> Option<Self> {
        let mut nearest = None;
        let mut best = radius;
        let ordered = || {
            paths
                .iter()
                .filter(|path| path.visible && Some(path.id) == active)
                .chain(
                    paths
                        .iter()
                        .filter(|path| path.visible && Some(path.id) != active),
                )
        };
        // The entire painted node square wins over handles, not only its
        // exact center. Use the renderer's marker size at the current zoom.
        for path in ordered() {
            for anchor in &path.anchors {
                let selected = node_selected(active, selection, path.id, anchor.id);
                let half =
                    f64::from(node_marker_size(selected)) / 2. * radius / f64::from(HIT_PIXELS);
                let delta = (anchor.point.0 - point.0, anchor.point.1 - point.1);
                let distance = delta.0.hypot(delta.1);
                if delta.0.abs() <= half
                    && delta.1.abs() <= half
                    && (nearest.is_none() || distance < best)
                {
                    best = distance;
                    nearest = Some(Self::Anchor {
                        path: path.id,
                        anchor: anchor.id,
                    });
                }
            }
        }
        if nearest.is_some() {
            return nearest;
        }
        // Outside the painted squares, choose the closest visible control.
        for path in ordered() {
            for anchor in &path.anchors {
                let distance = (anchor.point.0 - point.0).hypot(anchor.point.1 - point.1);
                if distance <= radius && (nearest.is_none() || distance < best) {
                    best = distance;
                    nearest = Some(Self::Anchor {
                        path: path.id,
                        anchor: anchor.id,
                    });
                }
            }
        }
        for path in ordered() {
            for anchor in &path.anchors {
                if !node_selected(active, selection, path.id, anchor.id) {
                    continue;
                }
                for (incoming, at) in [(true, anchor.handle_in), (false, anchor.handle_out)] {
                    let distance = (at.0 - point.0).hypot(at.1 - point.1);
                    if at != anchor.point
                        && distance <= radius
                        && (nearest.is_none() || distance < best)
                    {
                        best = distance;
                        nearest = Some(Self::Handle {
                            path: path.id,
                            anchor: anchor.id,
                            incoming,
                            at,
                        });
                    }
                }
            }
        }
        nearest
    }
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
                            .bg(rgb(if active == Some(id) { SELECTED } else { CHROME }))
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
                .border_1()
                .border_color(rgb(if selected { ACCENT } else { BORDER }))
                .bg(rgb(if selected { SELECTED } else { CHROME }))
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
        modifiers: gpui::Modifiers,
        cx: &mut Context<Self>,
    ) {
        if self.active_tab().is_none_or(|tab| tab.busy()) || self.path_gesture.is_some() {
            return;
        }
        let Some(point) = self.canvas_point(position, scale) else {
            return;
        };
        let edit = modifiers.secondary();
        let insert = edit || modifiers.alt;
        let toggle = modifiers.shift;
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
            self.path_gesture = Some(PathGesture::for_target(
                path,
                target,
                self.path_selection.as_ref().unwrap(),
                point,
                modifiers,
                self.path_hit_radius(scale) * 3. / f64::from(HIT_PIXELS),
                self.preferences.path_handle_mode,
            ));
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
                self.notice(
                    "Cmd/Ctrl-click or Alt/Option-click a visible curve to insert an anchor.",
                    cx,
                );
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
                continuation_path(content).cloned(),
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
            let Some(index) = continuation_index(&active, self.path_selection.as_ref()) else {
                self.notice(
                    "Select an endpoint to continue this path, or press Enter to start a new path.",
                    cx,
                );
                return;
            };
            let mut path = active.clone();
            path.anchors.insert(index, Anchor::corner(anchor_id, point));
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
        gesture.close_on_click();
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

    pub(super) fn finish_path_session(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let draft = self.path_gesture.take().map(|gesture| gesture.path);
        let path = draft
            .as_ref()
            .map(|path| path.id)
            .or_else(|| self.path_hold.as_ref().map(|path| path.id))
            .or_else(|| self.displayed_active_path());
        let Some(path) = path else {
            self.end_path_session(cx);
            return;
        };
        let Some(tab) = self.active_tab() else { return };
        let finish = PendingPathFinish {
            generation: tab.history.generation(),
            layer: tab.active_layer,
            tool: self.preferences.tool,
            path,
        };
        let id = tab.id;
        if tab.editing {
            self.end_path_session(cx);
            self.tabs[self.active.unwrap()].pending_path_finish = Some(finish);
            return;
        }
        // Path metadata changes need no pixel rendering. Publish synchronously
        // through ordinary history, including while an immutable save is running.
        let mut content = tab.document.content.clone();
        let placed = draft.is_some_and(|path| content.upsert_path(path));
        finish.apply(&mut content);
        self.end_path_session(cx);
        if placed {
            let tab = self.active_tab().unwrap();
            let patch = crate::document::RenderedCanvas::prepare_patch(
                &tab.document.canvas.pixels,
                tab.document.canvas.preview.has_transparency,
                tab.document.canvas.level_caches(),
                &content,
                Dirty::Unpainted,
            )
            .unwrap();
            let result = Some((content, tab.active_layer, tab.selection.clone(), patch));
            self.publish_change(
                id,
                finish.generation,
                super::Change::Edit,
                result,
                window,
                cx,
            );
        }
        cx.notify();
    }

    pub(super) fn end_path_session(&mut self, cx: &mut Context<Self>) {
        // Escape ends an open path without closing it. A late anchor commit
        // must not reselect it, even if the user already returned to Path.
        let path = self
            .path_hold
            .as_ref()
            .map(|path| path.id)
            .or_else(|| self.displayed_active_path());
        if let Some(index) = self.active
            && self.tabs[index].editing
            && let Some(path) = path
        {
            self.tabs[index].pending_path_finish = Some(PendingPathFinish {
                generation: self.tabs[index].history.generation(),
                layer: self.tabs[index].active_layer,
                tool: self.preferences.tool,
                path,
            });
        }
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

    pub(super) fn path_control_hit(&self, position: Point<Pixels>, scale: f32) -> bool {
        self.path_target(position, scale).is_some()
    }

    fn path_target(&self, position: Point<Pixels>, scale: f32) -> Option<Target> {
        let tab = self.active_tab()?;
        Target::nearest(
            tab.document.content.paths(),
            tab.document.content.active_path_id(),
            self.path_selection.as_ref(),
            self.canvas_point(position, scale)?,
            self.path_hit_radius(scale),
        )
    }
}

#[cfg(test)]
mod session_finish_tests {
    use super::*;
    use crate::document::{DocumentContent, RenderedCanvas};
    use crate::paths::AnchorId;

    fn path(count: u64) -> VectorPath {
        VectorPath {
            id: PathId(1),
            visible: true,
            closed: false,
            anchors: (0..count)
                .map(|index| {
                    Anchor::corner(
                        AnchorId(index + 1),
                        (2. + index as f64 * 3., if index == 1 { 12. } else { 2. }),
                    )
                })
                .collect(),
        }
    }

    fn prepared(tab: &super::super::Tab, content: DocumentContent) -> super::super::PreparedChange {
        let patch = RenderedCanvas::prepare_patch(
            &tab.document.canvas.pixels,
            tab.document.canvas.preview.has_transparency,
            tab.document.canvas.level_caches(),
            &content,
            Dirty::Unpainted,
        )
        .unwrap();
        (content, tab.active_layer, tab.selection.clone(), patch)
    }

    #[test]
    fn tool_departure_finishes_late_anchor_commit_and_keeps_one_undo_step() {
        let mut tab = super::super::preview_handoff_tests::tab();
        tab.document.content.upsert_path(path(2));
        let pixels = tab.document.canvas.pixels.clone();
        let mut content = tab.document.content.clone();
        content.upsert_path(path(3));
        let result = prepared(&tab, content);
        tab.pending_path_finish = Some(PendingPathFinish {
            generation: 0,
            layer: tab.active_layer,
            tool: crate::commands::Tool::Brush,
            path: PathId(1),
        });
        tab.document.content.clear_active_path();
        // The intent survives even if the user returns to Path before publication.
        assert!(
            tab.publish_change(0, super::super::Change::Edit, Some(result), 1)
                .0
        );
        assert_eq!(tab.document.content.paths()[0], path(3));
        assert_eq!(tab.document.content.paths()[0].anchors.len(), 3);
        assert!(tab.document.content.active_path().is_none());
        assert!(tab.pending_path_finish.is_none());
        assert!(std::sync::Arc::ptr_eq(&pixels, &tab.document.canvas.pixels));
        assert!(tab.history.is_dirty());
        let state = tab.state();
        let undone = tab.history.undo(state, 2).unwrap();
        assert_eq!(undone.content.paths()[0].anchors.len(), 2);
        assert!(!undone.content.paths()[0].closed);
    }

    #[test]
    fn path_finishing_preserves_open_closed_noop_and_stale_states() {
        for (count, closed) in [(1, false), (2, false), (3, false), (3, true), (8, false)] {
            let mut tab = super::super::preview_handoff_tests::tab();
            let mut original = path(count);
            original.closed = closed;
            tab.document.content.upsert_path(original.clone());
            let finish = PendingPathFinish {
                generation: 0,
                layer: tab.active_layer,
                tool: crate::commands::Tool::Move,
                path: PathId(1),
            };
            let mut content = tab.document.content.clone();
            finish.apply(&mut content);
            assert_eq!(content.paths()[0], original);
            assert!(content.active_path().is_none());
            // A no-op completion must not create a new edit or close any shape.
            tab.pending_path_finish = Some(finish);
            tab.document.content.clear_active_path();
            assert!(!tab.publish_change(0, super::super::Change::Edit, None, 1).0);
            assert_eq!(tab.document.content.paths()[0], original);
            assert_eq!(tab.history.generation(), 0);
            assert!(tab.document.content.active_path().is_none());
            // A real late anchor commit must retain exactly the incoming geometry.
            let mut incoming = tab.document.content.clone();
            incoming.select_path(PathId(1));
            let result = prepared(&tab, incoming);
            tab.pending_path_finish = Some(finish);
            tab.editing = true;
            assert!(
                tab.publish_change(0, super::super::Change::Edit, Some(result), 1)
                    .0
            );
            assert_eq!(tab.document.content.paths()[0], original);
            assert!(tab.document.content.active_path().is_none());
        }
        let mut tab = super::super::preview_handoff_tests::tab();
        tab.document.content.upsert_path(path(3));
        tab.pending_path_finish = Some(PendingPathFinish {
            generation: 0,
            layer: tab.active_layer,
            tool: crate::commands::Tool::Move,
            path: PathId(1),
        });
        let result = prepared(&tab, tab.document.content.clone());
        tab.history.commit(tab.state(), 1, true);
        assert!(
            !tab.publish_change(0, super::super::Change::Edit, Some(result), 2)
                .0
        );
        assert_eq!(tab.document.content.paths()[0], path(3));
        assert_eq!(tab.document.content.active_path_id(), Some(PathId(1)));
        assert!(tab.pending_path_finish.is_none());
        // Explicit closure remains available and is preserved by finishing.
        let mut closed = path(3);
        closed.closed = true;
        assert!(tab.document.content.upsert_path(closed));
        let finish = PendingPathFinish {
            generation: 1,
            layer: tab.active_layer,
            tool: crate::commands::Tool::Move,
            path: PathId(1),
        };
        finish.apply(&mut tab.document.content);
        assert!(tab.document.content.paths()[0].closed);
    }
    #[test]
    fn first_drag_moves_middle_node_despite_nearby_handles() {
        let mut original = path(3);
        let center = original.anchors[1].point;
        original.anchors[0].handle_out = (center.0 - 1., center.1);
        original.anchors[1].handle_in = (center.0 - 2., center.1);
        original.anchors[1].handle_out = (center.0 + 2., center.1);
        // A different selected node or no active path must not consume the first drag.
        for active in [None, Some(original.id)] {
            let target =
                Target::nearest(std::slice::from_ref(&original), active, None, center, 8.).unwrap();
            assert_eq!(
                target,
                Target::Anchor {
                    path: original.id,
                    anchor: AnchorId(2)
                }
            );
            let mut selection = Some(AnchorSelection::single(original.id, AnchorId(3)));
            AnchorSelection::click(&mut selection, original.id, AnchorId(2), false);
            let mut gesture = PathGesture::for_target(
                original.clone(),
                target,
                selection.as_ref().unwrap(),
                center,
                gpui::Modifiers::none(),
                3.,
                HandleMode::Independent,
            );
            gesture.update((center.0 + 1., center.1 + 2.));
            gesture.close_on_click();
            let mut expected = original.clone();
            expected.move_anchors_from(&[original.anchors[1]], (1., 2.));
            assert_eq!(gesture.path, expected);
        }
        // A selected handle outside the node square remains reachable within its hit radius.
        original.anchors[1].handle_out = (center.0 + 6., center.1);
        assert!(matches!(
            Target::nearest(
                std::slice::from_ref(&original),
                Some(original.id),
                Some(&AnchorSelection::single(original.id, AnchorId(2))),
                original.anchors[1].handle_out,
                8.
            ),
            Some(Target::Handle {
                anchor: AnchorId(2),
                incoming: false,
                ..
            })
        ));
        original.anchors[0].handle_out = center;
        assert_eq!(
            Target::nearest(std::slice::from_ref(&original), None, None, center, 8.),
            Some(Target::Anchor {
                path: original.id,
                anchor: AnchorId(2)
            })
        );
        original.visible = false;
        assert!(Target::nearest(&[original], None, None, center, 8.).is_none());
    }

    #[test]
    fn only_platform_edit_modifier_closes_or_shapes_handles() {
        let original = path(3);
        let start = original.anchors[0].point;
        let target = Target::Anchor {
            path: original.id,
            anchor: AnchorId(1),
        };
        let selection = AnchorSelection::single(original.id, AnchorId(1));
        for control in [false, true] {
            let mut gesture = PathGesture::for_target(
                original.clone(),
                target,
                &selection,
                start,
                if control {
                    gpui::Modifiers::secondary_key()
                } else {
                    gpui::Modifiers::none()
                },
                3.,
                HandleMode::Independent,
            );
            gesture.update(start);
            gesture.close_on_click();
            assert_eq!(gesture.path.closed, control);
            assert_eq!(gesture.path.anchors, original.anchors);
        }
        let mut gesture = PathGesture::for_target(
            original.clone(),
            target,
            &selection,
            start,
            gpui::Modifiers::none(),
            3.,
            HandleMode::Independent,
        );
        gesture.update((start.0 + 1., start.1));
        gesture.close_on_click();
        assert!(!gesture.path.closed);
        assert_eq!(gesture.path.anchors[0].point, (start.0 + 1., start.1));

        let mut gesture = PathGesture::for_target(
            original.clone(),
            target,
            &selection,
            start,
            gpui::Modifiers::secondary_key(),
            3.,
            HandleMode::Independent,
        );
        gesture.update((start.0 + 10., start.1 + 5.));
        gesture.close_on_click();
        assert!(!gesture.path.closed);
        assert_eq!(gesture.path.anchors[0].point, start);
        assert_eq!(
            gesture.path.anchors[0].handle_out,
            (start.0 + 10., start.1 + 5.)
        );
        assert_eq!(
            gesture.path.anchors[0].handle_in,
            (start.0 - 10., start.1 - 5.)
        );
        // Returning to the down position after a drag must never turn it into a close click.
        gesture.update(start);
        gesture.close_on_click();
        assert!(!gesture.path.closed);
        let mut short = PathGesture::for_target(
            path(2),
            target,
            &selection,
            start,
            gpui::Modifiers::secondary_key(),
            3.,
            HandleMode::Independent,
        );
        short.close_on_click();
        assert!(!short.path.closed);
    }

    #[test]
    fn endpoints_resume_in_the_selected_direction_and_groups_move_together() {
        let original = path(3);
        for (anchor, expected) in [(1, Some(0)), (2, None), (3, Some(3))] {
            assert_eq!(
                continuation_index(
                    &original,
                    Some(&AnchorSelection::single(original.id, AnchorId(anchor)))
                ),
                expected
            );
        }
        let mut selection = Some(AnchorSelection::single(original.id, AnchorId(1)));
        AnchorSelection::click(&mut selection, original.id, AnchorId(2), true);
        assert_eq!(continuation_index(&original, selection.as_ref()), None);
        AnchorSelection::click(&mut selection, original.id, AnchorId(2), false);
        let target = Target::Anchor {
            path: original.id,
            anchor: AnchorId(2),
        };
        let down = original.anchors[1].point;
        let mut gesture = PathGesture::for_target(
            original.clone(),
            target,
            selection.as_ref().unwrap(),
            down,
            gpui::Modifiers::none(),
            3.,
            HandleMode::Independent,
        );
        gesture.update((down.0 + 4., down.1 + 5.));
        gesture.close_on_click();
        let mut expected = original.clone();
        expected.move_anchors_from(&original.anchors[..2], (4., 5.));
        assert_eq!(gesture.path, expected);
        let mut closed = original.clone();
        closed.closed = true;
        assert_eq!(continuation_index(&closed, None), None);
        // Prepending preserves existing geometry and handles, without reversing the curve.
        let mut extended = original.clone();
        extended.anchors.insert(
            continuation_index(
                &original,
                Some(&AnchorSelection::single(original.id, AnchorId(1))),
            )
            .unwrap(),
            Anchor::corner(AnchorId(4), (0., 0.)),
        );
        assert_eq!(&extended.anchors[1..], original.anchors.as_slice());
    }
    #[test]
    fn node_square_wins_off_center_at_every_zoom_and_shift_toggles_it() {
        for units_per_point in [0.25, 1., 4., 32.] {
            let mut original = path(3);
            original.anchors = vec![
                Anchor::corner(AnchorId(1), (0., 0.)),
                Anchor::corner(AnchorId(2), (40. * units_per_point, 40. * units_per_point)),
                Anchor::corner(AnchorId(3), (80. * units_per_point, 0.)),
            ];
            let center = original.anchors[1].point;
            original.anchors[1].handle_in = (center.0 - 2. * units_per_point, center.1);
            original.anchors[1].handle_out = (center.0 + 2. * units_per_point, center.1);
            let mut selection = Some(AnchorSelection::single(original.id, AnchorId(2)));
            // These positions used to pick the short handles before moving the node.
            for (x, y) in [(-2., 0.), (2., 0.), (4.4, 4.4), (-4.4, -4.4)] {
                let down = (
                    center.0 + x * units_per_point,
                    center.1 + y * units_per_point,
                );
                let target = Target::nearest(
                    std::slice::from_ref(&original),
                    Some(original.id),
                    selection.as_ref(),
                    down,
                    8. * units_per_point,
                )
                .unwrap();
                assert_eq!(
                    target,
                    Target::Anchor {
                        path: original.id,
                        anchor: AnchorId(2)
                    }
                );
                let mut gesture = PathGesture::for_target(
                    original.clone(),
                    target,
                    selection.as_ref().unwrap(),
                    down,
                    gpui::Modifiers::none(),
                    3. * units_per_point,
                    HandleMode::Independent,
                );
                gesture.update((down.0 + 10. * units_per_point, down.1));
                let mut expected = original.clone();
                expected.move_anchors_from(&[original.anchors[1]], (10. * units_per_point, 0.));
                assert_eq!(gesture.path, expected);
            }
            // Shift-click reaches the node even where its short handle is painted.
            for _ in 0..2 {
                let target = Target::nearest(
                    std::slice::from_ref(&original),
                    Some(original.id),
                    selection.as_ref(),
                    original.anchors[1].handle_out,
                    8. * units_per_point,
                )
                .unwrap();
                let Target::Anchor { path, anchor } = target else {
                    panic!("handle stole Shift-click")
                };
                AnchorSelection::click(&mut selection, path, anchor, true);
            }
            assert_eq!(
                selection,
                Some(AnchorSelection::single(original.id, AnchorId(2)))
            );
            // Hidden handles are neither painted nor interactive.
            original.anchors[0].handle_out = (160. * units_per_point, 160. * units_per_point);
            assert!(
                Target::nearest(
                    std::slice::from_ref(&original),
                    Some(original.id),
                    selection.as_ref(),
                    original.anchors[0].handle_out,
                    8. * units_per_point
                )
                .is_none()
            );
        }
    }

    #[test]
    fn platform_edit_click_closes_through_an_overlapping_handle() {
        let mut original = path(3);
        original.anchors[0].handle_out = (3., 2.);
        let selection = AnchorSelection::single(original.id, AnchorId(1));
        let down = original.anchors[0].handle_out;
        let target = Target::nearest(
            std::slice::from_ref(&original),
            Some(original.id),
            Some(&selection),
            down,
            8.,
        )
        .unwrap();
        // Real GPUI modifiers: Command on macOS, Control on Windows/Linux.
        let mut gesture = PathGesture::for_target(
            original,
            target,
            &selection,
            down,
            gpui::Modifiers::secondary_key(),
            3.,
            HandleMode::Independent,
        );
        gesture.update(down);
        gesture.close_on_click();
        assert!(gesture.path.closed);
    }
    #[test]
    fn hidden_selected_path_is_excluded_from_canvas_continuation() {
        let mut tab = super::super::preview_handoff_tests::tab();
        let original = path(3);
        tab.document.content.upsert_path(original.clone());
        let selection = AnchorSelection::single(original.id, AnchorId(3));
        assert_eq!(continuation_path(&tab.document.content), Some(&original));
        assert!(tab.document.content.set_path_visible(original.id, false));
        assert_eq!(tab.document.content.active_path_id(), Some(original.id));
        // Both hiding the current path and explicitly selecting a hidden row
        // must take the new-visible-path branch when clicking empty canvas.
        for reselect in [false, true] {
            if reselect {
                tab.document.content.select_path(original.id);
            }
            assert!(continuation_path(&tab.document.content).is_none());
            let hidden = tab.document.content.active_path().unwrap();
            assert!(continuation_index(hidden, None).is_none());
            assert!(continuation_index(hidden, Some(&selection)).is_none());
            assert_eq!(hidden.anchors, original.anchors);
            assert!(
                Target::nearest(
                    tab.document.content.paths(),
                    Some(original.id),
                    Some(&selection),
                    original.anchors[2].point,
                    8.
                )
                .is_none()
            );
        }
        // Showing the path again restores explicit endpoint continuation.
        tab.document.content.set_path_visible(original.id, true);
        let shown = continuation_path(&tab.document.content).unwrap();
        assert_eq!(shown, &original);
        assert_eq!(continuation_index(shown, Some(&selection)), Some(3));
    }
}
