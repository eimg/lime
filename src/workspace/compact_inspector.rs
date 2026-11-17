//! The folded inspector keeps quick controls, layer thumbnails and paths available.
use gpui::{
    Action, Bounds, Context, Div, MouseButton, ScrollHandle, Stateful, canvas, div, fill, point,
    prelude::*, px, relative, rgb, rgba, size,
};

use super::Workspace;
use super::numeric::NumericField;
use super::tooltip::{tooltip, tooltip_with_detail};
use super::view::{
    ACCENT, BORDER, CANVAS, HOVER, MUTED, PANEL, SELECTED, TEXT, TOOLBOX_FOLDED_WIDTH,
    align_button, icon,
};
use crate::commands::*;

impl Workspace {
    pub(super) fn compact_inspector(&self, cx: &Context<Self>) -> Div {
        let tool = self.preferences.tool.info();
        let has_paths = self
            .active_tab()
            .is_some_and(|tab| !tab.document.content.paths().is_empty());
        div()
            .w(px(TOOLBOX_FOLDED_WIDTH))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .border_l_1()
            .border_color(rgb(BORDER))
            .track_focus(&self.numeric_focus)
            .child(
                div()
                    .h(px(36.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .child(self.inspector_toggle(cx)),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h(relative(0.28))
                            .max_h(px(208.))
                            .min_h(px(80.))
                            .flex_shrink_0()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .child(
                                compact_control(compact_header(Some(tool.icon), tool.label))
                                    .id("compact-current-tool")
                                    .tooltip(tooltip_with_detail(
                                        tool.label,
                                        "Expand the sidebar for all tool options",
                                    ))
                                    .cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                        cx.stop_propagation()
                                    })
                                    .on_click(|_, window, cx| {
                                        window.dispatch_action(Box::new(ToggleRightSidebar), cx)
                                    }),
                            )
                            .child(compact_scroll(
                                "compact-tool-scroll",
                                &self.compact_inspector_scroll[0],
                                self.compact_tool_options(cx),
                            )),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h(px(48.))
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .border_t_1()
                            .border_color(rgb(BORDER))
                            .child(compact_header(None, "Layers"))
                            .child(compact_scroll(
                                "compact-layer-scroll",
                                &self.compact_inspector_scroll[1],
                                self.compact_layers(cx),
                            )),
                    )
                    .child(
                        div()
                            .h(if has_paths {
                                relative(0.22)
                            } else {
                                relative(0.)
                            })
                            .max_h(px(144.))
                            .min_h(px(48.))
                            .flex_shrink_0()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .border_t_1()
                            .border_color(rgb(BORDER))
                            .child(compact_header(None, "Paths"))
                            .child(compact_scroll(
                                "compact-path-scroll",
                                &self.compact_inspector_scroll[2],
                                self.compact_paths(cx),
                            )),
                    ),
            )
    }

    pub(super) fn compact_new_layer_button(&self) -> Stateful<Div> {
        compact_action(
            "compact-new-layer",
            "+",
            Box::new(NewLayer),
            self.pdf_job.is_none() && self.layer_command_enabled(super::layers::LayerCommand::New),
        )
        .h(px(22.))
        .text_size(px(16.))
        .tooltip(tooltip("New Layer…"))
    }

    fn compact_tool_options(&self, cx: &Context<Self>) -> Div {
        use NumericField as N;
        use Tool as T;
        // Keep the current tool heading, but no document controls or hints,
        // in an empty workspace. This gate covers every compact tool below.
        if self.active_tab().is_none() {
            return div();
        }
        let mut body = div().flex().flex_col().gap_2().p(px(5.));
        if self.preferences.tool == T::Text {
            body = body.child(compact_label(self.text_format_target()));
        }
        let fields: &[(&str, NumericField)] = match self.preferences.tool {
            T::Pencil => &[("Size", N::BrushSize), ("Opacity", N::BrushOpacity)],
            T::Brush | T::Eraser | T::Blur | T::Sharpen | T::CloneStamp => &[
                ("Size", N::BrushSize),
                (
                    if matches!(self.preferences.tool, T::Blur | T::Sharpen) {
                        "Strength"
                    } else {
                        "Opacity"
                    },
                    N::BrushOpacity,
                ),
            ],
            T::Crop => &[("Width", N::CropWidth), ("Height", N::CropHeight)],
            T::ScaleLayer => &[("Width", N::ScaleWidth), ("Height", N::ScaleHeight)],
            T::Rotate => &[("Angle °", N::RotateAngle)],
            T::MagicWand => &[("Tolerance", N::WandTolerance), ("Gap", N::WandGapRadius)],
            T::BucketFill if self.preferences.fill_mode == crate::document::FillMode::Connected => {
                &[("Tolerance", N::FillTolerance), ("Gap", N::FillGapRadius)]
            }
            T::Text => &[("Size", N::TextSize)],
            T::Path => &[("Stroke", N::PathStrokeSize)],
            _ => &[],
        };
        for &(label, field) in fields {
            if self.numeric_value(field).is_some() {
                body = body.child(self.compact_numeric(label, field, cx));
            }
        }
        match self.preferences.tool {
            T::Move => body = body.child(self.compact_move_controls(cx)),
            T::Brush | T::Eraser | T::Blur | T::Sharpen | T::CloneStamp => {
                use crate::paint::smoothing::Smoothing;
                let mode = self.brush_smoothing();
                let (label, next) = match mode {
                    Smoothing::Off => ("Off", Smoothing::Light),
                    Smoothing::Light => ("Light", Smoothing::Strong),
                    Smoothing::Strong => ("Strong", Smoothing::Off),
                };
                body = body.child(
                    compact_label("Smooth").child(
                        compact_button("compact-smoothing", label, mode != Smoothing::Off)
                            .tooltip(tooltip("Smoothing: Off → Light → Strong"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.set_brush_smoothing(next, cx)
                            })),
                    ),
                );
            }
            T::Crop | T::Select | T::OvalSelect => {
                let crop = self.preferences.tool == T::Crop;
                let ratio = if crop {
                    self.preferences.crop_ratio
                } else {
                    self.effective_selection_ratio()
                };
                let label = ratio.map_or_else(
                    || "Free".to_owned(),
                    |r| format!("{}:{}", r.width, r.height),
                );
                body = body.child(
                    compact_label("Ratio").child(
                        compact_button("compact-ratio", label.clone(), ratio.is_some())
                            .tooltip(tooltip(format!(
                                "Ratio {label}. Toggle Free / 1:1; expand for all aspect ratios"
                            )))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.set_aspect_ratio(
                                    crop,
                                    if ratio.is_some() {
                                        None
                                    } else {
                                        Some(crate::constraints::AspectRatio::SQUARE)
                                    },
                                    window,
                                    cx,
                                );
                            })),
                    ),
                );
                if !crop {
                    body = body.child(self.compact_feather(cx));
                }
            }
            T::FreeSelect => body = body.child(self.compact_feather(cx)),
            T::ScaleLayer if self.scale_drag.is_some() => {
                let locked = self.preferences.scale_lock;
                body = body.child(
                    compact_label("Aspect").child(
                        compact_button(
                            "compact-scale-lock",
                            if locked { "Locked" } else { "Free" },
                            locked,
                        )
                        .tooltip(tooltip("Lock aspect ratio"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.preferences.scale_lock = !locked;
                            cx.notify();
                        })),
                    ),
                );
            }
            T::Perspective if self.warp.is_some() => {
                let selected = self.selected_perspective_corner();
                body = body
                    .child(self.compact_numeric("X", N::PerspectiveX(selected as u8), cx))
                    .child(self.compact_numeric("Y", N::PerspectiveY(selected as u8), cx));
            }
            T::Flip if self.warp.is_some() => {
                let horizontal = self.warp_status().is_some_and(|s| s.horizontal);
                body = body.child(
                    compact_button("compact-mirror", if horizontal { "H" } else { "V" }, true)
                        .tooltip(tooltip("Toggle horizontal / vertical flip"))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.set_warp_mirror(
                                if horizontal {
                                    crate::transform::Mirror::Vertical
                                } else {
                                    crate::transform::Mirror::Horizontal
                                },
                                window,
                                cx,
                            );
                        })),
                );
            }
            T::Gradient => {
                use crate::paint::GradientBlend;
                let linear = self.preferences.gradient_shape == crate::paint::GradientShape::Linear;
                let blend = self.preferences.gradient_blend;
                let custom = self.preferences.gradient_use_custom;
                let (label, next, help) = if custom {
                    (
                        "Custom",
                        GradientBlend::ForegroundToBackground,
                        "Custom gradient. Click to switch to foreground → background.",
                    )
                } else if blend == GradientBlend::ForegroundToBackground {
                    (
                        "F→B",
                        GradientBlend::ForegroundToTransparent,
                        "Foreground → Background. Click for F→A (foreground → transparent).",
                    )
                } else {
                    (
                        "F→A",
                        GradientBlend::ForegroundToBackground,
                        "Foreground → Transparent (alpha). Click for F→B (foreground → background).",
                    )
                };
                body = body
                    .child(
                        compact_label("Colors").child(
                            compact_button("compact-gradient-colors", label, true)
                                .tooltip(tooltip(help))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.set_gradient_blend(next, window, cx);
                                })),
                        ),
                    )
                    .child(
                        compact_label("Shape").child(
                            compact_button(
                                "compact-gradient-shape",
                                if linear { "Linear" } else { "Radial" },
                                true,
                            )
                            .on_click(cx.listener(
                                move |this, _, window, cx| {
                                    this.set_gradient_shape(
                                        if linear {
                                            crate::paint::GradientShape::Radial
                                        } else {
                                            crate::paint::GradientShape::Linear
                                        },
                                        window,
                                        cx,
                                    );
                                },
                            )),
                        ),
                    )
                    .child(
                        compact_label("Reverse").child(
                            compact_button(
                                "compact-gradient-reverse",
                                if self.preferences.gradient_reverse {
                                    "On"
                                } else {
                                    "Off"
                                },
                                self.preferences.gradient_reverse,
                            )
                            .on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.reverse_gradient(window, cx)
                                }),
                            ),
                        ),
                    );
            }
            T::Text => {
                let color = self
                    .text_color_value()
                    .unwrap_or(self.color_memory.swatches[0]);
                body = body.child(
                    compact_label("Color").child(
                        compact_control(
                            div()
                                .id("compact-text-color")
                                .w_full()
                                .h(px(26.))
                                .p(px(4.))
                                .rounded_sm()
                                .border_1()
                                .border_color(rgb(BORDER))
                                .bg(rgb(CANVAS))
                                .cursor_pointer()
                                .hover(|button| button.bg(rgb(HOVER)))
                                .tooltip(tooltip(format!(
                                    "{} — click to change",
                                    if self.text_color_mixed() {
                                        "Mixed text colors".to_owned()
                                    } else {
                                        format!("Text color #{color:06X}")
                                    }
                                )))
                                .child(div().size_full().rounded_sm().bg(rgb(color)).when(
                                    self.text_color_mixed(),
                                    |swatch| {
                                        swatch
                                            .bg(rgb(CANVAS))
                                            .text_size(px(9.))
                                            .text_center()
                                            .whitespace_nowrap()
                                            .overflow_hidden()
                                            .child("Mixed")
                                    },
                                )),
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.prepare_text_format(window, cx);
                            let swatch = if this.text_draft.is_some() {
                                super::text_ui::TEXT_SWATCH
                            } else {
                                0
                            };
                            this.color_form(swatch, window, cx);
                        })),
                    ),
                );
            }
            T::Path => {
                let enabled = self.can_stroke_outline(super::stroke_ui::StrokeSource::Path)
                    && self.preferences.brushes.outline.opacity > 0;
                body = body.child(
                    compact_button("compact-stroke-path", "Stroke", false)
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .when_else(
                            enabled,
                            |button| {
                                button.on_click(cx.listener(|this, _, window, cx| {
                                    this.stroke_outline(
                                        super::stroke_ui::StrokeSource::Path,
                                        window,
                                        cx,
                                    );
                                }))
                            },
                            |button| button.opacity(0.4).tab_index(-1).cursor_default(),
                        ),
                );
            }
            T::Zoom => {
                body = body
                    .child(compact_action(
                        "compact-zoom-in",
                        "+",
                        Box::new(ZoomIn),
                        self.active_tab().is_some(),
                    ))
                    .child(compact_action(
                        "compact-zoom-out",
                        "−",
                        Box::new(ZoomOut),
                        self.active_tab().is_some(),
                    ))
            }
            T::Pan => {
                body = body.child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(MUTED))
                        .text_center()
                        .child("Drag to pan"),
                )
            }
            _ => {}
        }
        body
    }

    fn compact_numeric(&self, label: &'static str, field: NumericField, cx: &Context<Self>) -> Div {
        compact_label(label).child(self.compact_numeric_chip(field, cx))
    }

    fn compact_move_controls(&self, cx: &Context<Self>) -> Div {
        let enabled = self.geometry_enabled() && !self.active_move_locked();
        let choices = [
            (
                "align-h-center",
                "Center Horizontally",
                AlignLayer::Horizontal,
            ),
            ("align-v-center", "Center Vertically", AlignLayer::Vertical),
        ];
        compact_label("Align").children(choices.into_iter().map(|(symbol, label, action)| {
            compact_control(align_button(symbol, label, action, enabled, cx))
                .w_full()
                .min_w_0()
                .when(!enabled, |button| button.tab_index(-1))
        }))
    }

    fn compact_feather(&self, cx: &Context<Self>) -> Div {
        let enabled = self.preferences.selection_feather_enabled;
        compact_label("Feather").child(
            compact_button(
                "compact-feather",
                if enabled { "On" } else { "Off" },
                enabled,
            )
            .tooltip(tooltip(
                "Feather new selections; expand to change its radius",
            ))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.preferences.selection_feather_enabled = !enabled;
                cx.notify();
            })),
        )
    }
}

fn compact_header(symbol: Option<&'static str>, label: &'static str) -> Stateful<Div> {
    div()
        .id(gpui::SharedString::from(format!("compact-header-{label}")))
        .h(px(if symbol.is_some() { 36. } else { 22. }))
        .flex_shrink_0()
        .w_full()
        .min_w_0()
        .overflow_hidden()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(1.))
        .text_color(rgb(MUTED))
        .text_size(px(9.))
        .tooltip(tooltip(label))
        .when_some(symbol, |header, symbol| header.child(icon(symbol, 15.)))
        .child(
            div()
                .w(px(TOOLBOX_FOLDED_WIDTH - 8.))
                .min_w_0()
                .text_ellipsis()
                .text_center()
                .child(label),
        )
}

fn compact_label(label: &'static str) -> Div {
    div()
        .w_full()
        .min_w_0()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .gap(px(2.))
        .child(
            div()
                .text_size(px(9.))
                .text_color(rgb(MUTED))
                .text_center()
                .child(label),
        )
}

fn compact_button(
    id: &'static str,
    label: impl Into<gpui::SharedString>,
    selected: bool,
) -> Stateful<Div> {
    let label = label.into();
    compact_control(
        div()
            .id(id)
            .h(px(25.))
            .w_full()
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .rounded_sm()
            .text_size(px(10.))
            .bg(rgb(if selected { SELECTED } else { CANVAS }))
            .text_color(rgb(if selected { ACCENT } else { TEXT }))
            .hover(|button| button.bg(rgb(HOVER)))
            .cursor_pointer()
            .tooltip(tooltip(label.clone()))
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .text_center()
                    .text_ellipsis()
                    .child(label),
            ),
    )
}

/// GPUI clicks focused buttons on key-up. Block canvas bindings and Space pan
/// on key-down, including repeats; a physical release invokes the same handler
/// as a pointer click exactly once.
pub(super) fn compact_control(button: Stateful<Div>) -> Stateful<Div> {
    button
        .tab_index(0)
        .key_context("CompactControl")
        .border_1()
        .focus(|button| button.border_color(rgb(ACCENT)))
        .on_key_down(|event, window, cx| {
            if let Some(backwards) = compact_tab_direction(&event.keystroke) {
                focus_compact_neighbor(None, backwards, window);
                cx.stop_propagation();
            } else if crate::commands::activate_sidebar_button(event, || {}) {
                cx.stop_propagation();
            }
        })
}

pub(super) fn compact_tab_direction(key: &gpui::Keystroke) -> Option<bool> {
    (key.key == "tab" && !key.modifiers.control && !key.modifiers.platform && !key.modifiers.alt)
        .then_some(key.modifiers.shift)
}

pub(super) fn focus_compact_neighbor(
    origin: Option<gpui::FocusHandle>,
    backwards: bool,
    window: &mut gpui::Window,
) {
    if let Some(origin) = origin {
        origin.focus(window);
    }
    if backwards {
        window.focus_prev();
    } else {
        window.focus_next();
    }
}

fn compact_action(
    id: &'static str,
    label: &'static str,
    action: Box<dyn Action>,
    enabled: bool,
) -> Stateful<Div> {
    compact_button(id, label, false).when_else(
        enabled,
        |button| {
            button.on_click(move |_, window, cx| window.dispatch_action(action.boxed_clone(), cx))
        },
        |button| button.opacity(0.4).tab_index(-1).cursor_default(),
    )
}

fn compact_scroll(id: &'static str, handle: &ScrollHandle, content: Div) -> Div {
    let fades = handle.clone();
    div()
        .relative()
        .flex_1()
        .min_h_0()
        .overflow_hidden()
        .child(
            div()
                .id(id)
                .size_full()
                .overflow_y_scroll()
                .scrollbar_width(px(0.))
                .track_scroll(handle)
                .child(content),
        )
        .child(
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| {
                    let above = fades.offset().y < px(-0.5);
                    let below = -fades.offset().y + px(0.5) < fades.max_offset().height;
                    for edge in 0..6 {
                        let color = rgba((PANEL << 8) | ((6 - edge) * 22));
                        if above {
                            window.paint_quad(fill(
                                Bounds::new(
                                    point(bounds.left(), bounds.top() + px(edge as f32)),
                                    size(bounds.size.width, px(1.)),
                                ),
                                color,
                            ));
                        }
                        if below {
                            window.paint_quad(fill(
                                Bounds::new(
                                    point(bounds.left(), bounds.bottom() - px(edge as f32 + 1.)),
                                    size(bounds.size.width, px(1.)),
                                ),
                                color,
                            ));
                        }
                    }
                },
            )
            .absolute()
            .size_full(),
        )
}
