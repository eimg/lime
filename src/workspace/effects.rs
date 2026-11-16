use gpui::{Context, Div, Window, prelude::*};

use super::{Workspace, adjust_ui::AdjustKind, filter_ui::FilterKind};
use crate::adjustments::Adjustment;
use crate::commands::{ReopenLastEffect, RepeatLastEffect};
use crate::document::{DocumentContent, LayerId};
use crate::filters::FilterSpec;
use crate::preview::Dirty;
use crate::selection::Selection;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum Effect {
    Adjustment(Adjustment),
    Filter(FilterSpec),
}

impl Effect {
    fn supports_text(self) -> bool {
        matches!(self, Self::Filter(spec) if spec.supports_text())
    }

    fn valid(self) -> bool {
        match self {
            Self::Adjustment(value) => value.valid(),
            Self::Filter(FilterSpec::Blur { radius }) => radius <= crate::filters::MAX_RADIUS,
            Self::Filter(FilterSpec::Sharpen { radius, amount }) => {
                radius <= crate::filters::MAX_RADIUS && amount <= 100
            }
            Self::Filter(FilterSpec::Stroke { width, opacity, .. }) => {
                width <= crate::filters::MAX_STROKE_WIDTH && opacity <= 100
            }
            Self::Filter(FilterSpec::Shadow {
                radius,
                offset,
                opacity,
                ..
            }) => {
                radius <= crate::filters::MAX_RADIUS
                    && opacity <= 100
                    && offset.0.unsigned_abs() <= 16384
                    && offset.1.unsigned_abs() <= 16384
            }
        }
    }

    fn slot(self) -> usize {
        match self {
            Self::Adjustment(value) => AdjustKind::of(value) as usize,
            Self::Filter(value) => 7 + FilterKind::of(value) as usize,
        }
    }

    fn is_identity(self) -> bool {
        match self {
            Self::Adjustment(value) => value.is_identity(),
            Self::Filter(value) => value.is_identity(),
        }
    }

    fn fits(self, content: &DocumentContent, layer: LayerId) -> bool {
        match self {
            Self::Filter(spec) if spec.supports_text() && !spec.is_identity() => {
                content.layer_filter_fits(layer, spec)
            }
            _ => true,
        }
    }

    fn retained_spec(self) -> Option<crate::document::layer_effect::EffectSpec> {
        use crate::document::layer_effect::EffectSpec;
        match self {
            Self::Adjustment(value) => Some(EffectSpec::Adjustment(value)),
            Self::Filter(FilterSpec::Blur { radius }) => Some(EffectSpec::Blur { radius }),
            Self::Filter(FilterSpec::Sharpen { radius, amount }) => {
                Some(EffectSpec::Sharpen { radius, amount })
            }
            Self::Filter(FilterSpec::Shadow { .. } | FilterSpec::Stroke { .. }) => None,
        }
    }

    fn apply(self, content: &mut DocumentContent, layer: LayerId, selection: &Selection) -> Dirty {
        match self {
            Self::Adjustment(value) => content.apply_adjustment(layer, value, selection),
            Self::Filter(spec @ FilterSpec::Shadow { .. }) => content
                .place_drop_shadow(layer, spec)
                .map_or(Dirty::None, Dirty::Region),
            Self::Filter(spec @ FilterSpec::Stroke { .. }) => content
                .place_layer_filter(layer, spec)
                .map_or(Dirty::None, Dirty::Region),
            Self::Filter(value) => content.apply_filter(layer, value, selection),
        }
    }
}

#[derive(Clone, Copy)]
struct PendingEffect {
    tab: usize,
    generation: u64,
    order: u64,
    effect: Effect,
}

#[derive(Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Preferences {
    #[serde(deserialize_with = "deserialize_effect_slots")]
    values: [Option<Effect>; 11],
    last: Option<Effect>,
}

// Version-one settings used ten slots; preserve them when adding Stroke.
fn deserialize_effect_slots<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<[Option<Effect>; 11], D::Error> {
    let values = <Vec<Option<Effect>> as serde::Deserialize>::deserialize(deserializer)?;
    if !matches!(values.len(), 10 | 11) {
        return Err(serde::de::Error::custom(
            "Expected ten or eleven effect slots",
        ));
    }
    let mut slots = [None; 11];
    for (slot, value) in slots.iter_mut().zip(values) {
        *slot = value;
    }
    Ok(slots)
}

impl Preferences {
    pub(super) fn reconcile_last(&mut self) {
        // `last` identifies a slot. Its duplicated parameters must follow the
        // winning complete slot value when another instance updates that effect.
        self.last = self.last.and_then(|effect| self.values[effect.slot()]);
    }

    pub(super) fn valid(&self) -> bool {
        self.values.iter().enumerate().all(|(slot, effect)| {
            effect.is_none_or(|effect| {
                effect.valid() && effect.slot() == slot && !effect.is_identity()
            })
        }) && self
            .last
            .is_none_or(|effect| effect.valid() && self.values[effect.slot()] == Some(effect))
    }
}

#[derive(Default)]
pub(super) struct EffectMemory {
    // Keep only small parameter values, never document pixels or selections.
    values: [Option<(u64, Effect)>; 11],
    last: Option<(u64, Effect)>,
    order: u64,
    pending: Vec<PendingEffect>,
}

impl EffectMemory {
    pub(super) fn preferences(&self) -> Preferences {
        Preferences {
            values: self.values.map(|value| value.map(|(_, effect)| effect)),
            last: self.last.map(|(_, effect)| effect),
        }
    }

    pub(super) fn restore(&mut self, preferences: &Preferences) {
        for (current, saved) in self.values.iter_mut().zip(preferences.values) {
            if current.is_none() {
                *current = saved.map(|effect| (0, effect));
            }
        }
        if self.last.is_none() {
            self.last = preferences.last.map(|effect| (0, effect));
        }
    }

    pub(super) fn adjustment(&self, kind: AdjustKind) -> Adjustment {
        match self.values[kind as usize] {
            Some((_, Effect::Adjustment(value))) => value,
            _ => kind.defaults(),
        }
    }

    pub(super) fn filter(&self, kind: FilterKind) -> Option<FilterSpec> {
        match self.values[7 + kind as usize] {
            Some((_, Effect::Filter(value))) => Some(value),
            _ => None,
        }
    }

    fn queue(&mut self, tab: usize, generation: u64, effect: Effect) {
        self.order = self.order.saturating_add(1);
        self.pending.push(PendingEffect {
            tab,
            generation,
            order: self.order,
            effect,
        });
    }

    pub(super) fn finish(&mut self, tab: usize, generation: u64, published: bool) {
        let Some(index) = self
            .pending
            .iter()
            .position(|pending| pending.tab == tab && pending.generation == generation)
        else {
            return;
        };
        let pending = self.pending.remove(index);
        // No-op edits, rejected generations, and closed tabs must not become repeatable.
        if !published || pending.effect.is_identity() {
            return;
        }
        let slot = &mut self.values[pending.effect.slot()];
        if slot.is_none_or(|(order, _)| pending.order > order) {
            *slot = Some((pending.order, pending.effect));
        }
        if self.last.is_none_or(|(order, _)| pending.order > order) {
            self.last = Some((pending.order, pending.effect));
        }
    }
}

impl Workspace {
    pub(super) fn bind_effect_actions(&self, root: Div, cx: &Context<Self>) -> Div {
        if !self.last_effect_enabled() {
            return root;
        }
        root.on_action(cx.listener(|this, _: &RepeatLastEffect, window, cx| {
            this.run_last_effect(false, window, cx);
        }))
        .on_action(cx.listener(|this, _: &ReopenLastEffect, window, cx| {
            this.run_last_effect(true, window, cx);
        }))
    }

    fn last_effect_enabled(&self) -> bool {
        let Some((_, effect)) = self.effect_memory.last else {
            return false;
        };
        self.form.is_none()
            && !self.dialog_open
            && self.pdf_job.is_none()
            && self.stroke.is_none()
            && self.guide_drag.is_none()
            && self.path_gesture.is_none()
            && self.selection_drag.is_none()
            && self.opacity_drag.is_none()
            && (self.move_drag.is_none() || self.numeric_edit.is_some())
            && self
                .scale_drag
                .as_ref()
                .is_none_or(|scale| !scale.changed())
            && self.gradient.is_none()
            && self.warp.as_ref().is_none_or(|warp| !warp.changed())
            && self.crop.is_none()
            && self.free_points.is_empty()
            && self.active_tab().is_some_and(|tab| {
                !tab.busy()
                    && tab
                        .document
                        .content
                        .layer(tab.active_layer)
                        .is_some_and(|layer| {
                            layer.visible && (layer.text.is_none() || effect.supports_text())
                        })
            })
    }

    pub(super) fn run_last_effect(
        &mut self,
        reopen: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.last_effect_enabled() {
            return;
        }
        if self.commit_numeric_with_followup(
            super::numeric::NumericFollowup::Effect { reopen },
            window,
            cx,
        ) {
            return;
        }
        if self.finish_text(window, cx) {
            self.defer_after_text(move |this, window, cx| this.run_last_effect(reopen, window, cx));
            return;
        }
        // Resolve the target only after typed text/geometry has landed.
        if !self.last_effect_enabled() {
            return;
        }
        let Some((_, effect)) = self.effect_memory.last else {
            return;
        };
        if reopen {
            match effect {
                Effect::Adjustment(value) => {
                    self.adjustment_form(AdjustKind::of(value), window, cx)
                }
                Effect::Filter(value) => self.filter_form(FilterKind::of(value), window, cx),
            }
            return;
        }
        let Some(tab) = self.active_tab() else {
            return;
        };
        if !effect.fits(&tab.document.content, tab.active_layer) {
            self.notice(
                "The repeated effect does not fit in the layer stack or its offset is too large.",
                cx,
            );
            return;
        }
        let target = (tab.id, tab.history.generation());
        self.edit_effect(
            effect,
            target,
            tab.active_layer,
            tab.selection.clone(),
            false,
            window,
            cx,
        );
    }

    pub(super) fn edit_retained_layer_effect(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.form.is_some() || self.dialog_open || self.active_tab().is_none_or(|tab| tab.busy())
        {
            return;
        }
        let Some(effect) = self
            .active_tab()
            .and_then(|tab| tab.document.content.layer(tab.active_layer))
            .and_then(|layer| layer.effect.as_ref())
            .map(|effect| effect.spec)
        else {
            return;
        };
        self.effect_reediting = true;
        match effect {
            crate::document::layer_effect::EffectSpec::Adjustment(value) => {
                self.adjustment_form(AdjustKind::of(value), window, cx)
            }
            spec => self.filter_form(FilterKind::of(spec.filter().unwrap()), window, cx),
        }
        self.effect_reediting = false;
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn edit_effect(
        &mut self,
        effect: Effect,
        target: (usize, u64),
        layer: LayerId,
        selection: Selection,
        reediting: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if (effect.is_identity() && !reediting)
            || self.active_tab().is_none_or(|tab| {
                tab.id != target.0 || tab.history.generation() != target.1 || tab.busy()
            })
        {
            return;
        }
        if effect.retained_spec().is_some()
            && self.active_tab().is_some_and(|tab| {
                !tab.document
                    .content
                    .can_retain_effect(layer, &selection, reediting)
            })
        {
            self.notice("There is not enough layer memory to retain this effect. Bake an existing effect or use a smaller layer.", cx);
            return;
        }
        if self.form.as_ref().is_some_and(|form| {
            matches!(
                form.kind,
                super::dialogs::FormKind::Adjustment { .. }
                    | super::dialogs::FormKind::Filter { .. }
            )
        }) {
            self.applying_form = Some(target);
        }
        self.edit_content(
            move |content, _, _| {
                if let Some(spec) = effect.retained_spec() {
                    content.apply_retained_effect(layer, spec, &selection, reediting)
                } else {
                    effect.apply(content, layer, &selection)
                }
            },
            window,
            cx,
        );
        if self
            .active_tab()
            .is_some_and(|tab| tab.id == target.0 && tab.editing)
        {
            self.effect_memory.queue(target.0, target.1, effect);
        }
    }
}
