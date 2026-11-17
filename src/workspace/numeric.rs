use gpui::{
    ClipboardItem, Context, CursorStyle, Div, Keystroke, MouseButton, Window, div, prelude::*, px,
    rgb,
};

use super::Workspace;
use super::paint_ui::BrushField;
use super::view::{ACCENT, BORDER, CANVAS, MUTED};
use crate::document::CropRect;
use crate::preview::PixelRect;
use crate::selection::Selection;
use crate::text::{MAX_LINE_SPACING, MAX_PIXEL_SIZE, MIN_LINE_SPACING, MIN_PIXEL_SIZE};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NumericField {
    SelectionRadius,
    SelectionWidth,
    SelectionHeight,
    CropX,
    CropY,
    CropWidth,
    CropHeight,
    TextSize,
    TextBoxWidth,
    TextBoxHeight,
    TextLineSpacing,
    TextLetterSpacing,
    WandTolerance,
    FillTolerance,
    WandGapRadius,
    FillGapRadius,
    BrushSize,
    BrushSpacing,
    BrushHardness,
    BrushOpacity,
    PathStrokeSize,
    ScaleWidth,
    ScaleHeight,
    RotateAngle,
    RotatePivotX,
    RotatePivotY,
    PerspectiveX(u8),
    PerspectiveY(u8),
    MoveX,
    MoveY,
    GridSpacing,
    SelectionX,
    SelectionY,
    SelectionRatioWidth,
    SelectionRatioHeight,
    CropRatioWidth,
    CropRatioHeight,
    BrushFlow,
    SelectionFeatherRadius,
    GradientStopPosition,
    GradientStopOpacity,
}

impl NumericField {
    fn id(self) -> usize {
        match self {
            Self::SelectionRadius => 0,
            Self::SelectionWidth => 1,
            Self::SelectionHeight => 2,
            Self::CropX => 3,
            Self::CropY => 4,
            Self::CropWidth => 5,
            Self::CropHeight => 6,
            Self::TextSize => 7,
            Self::TextBoxWidth => 45,
            Self::TextBoxHeight => 46,
            Self::TextLineSpacing => 8,
            Self::TextLetterSpacing => 9,
            Self::WandTolerance => 10,
            Self::FillTolerance => 32,
            Self::WandGapRadius => 33,
            Self::FillGapRadius => 34,
            Self::BrushSize => 11,
            Self::BrushSpacing => 12,
            Self::BrushHardness => 13,
            Self::BrushOpacity => 14,
            Self::PathStrokeSize => 15,
            Self::ScaleWidth => 16,
            Self::ScaleHeight => 17,
            Self::RotateAngle => 18,
            Self::RotatePivotX => 35,
            Self::RotatePivotY => 36,
            Self::PerspectiveX(index) => 37 + 2 * usize::from(index),
            Self::PerspectiveY(index) => 38 + 2 * usize::from(index),
            Self::MoveX => 19,
            Self::MoveY => 20,
            Self::GridSpacing => 21,
            Self::SelectionX => 22,
            Self::SelectionY => 23,
            Self::SelectionRatioWidth => 24,
            Self::SelectionRatioHeight => 25,
            Self::CropRatioWidth => 26,
            Self::CropRatioHeight => 27,
            Self::BrushFlow => 28,
            Self::SelectionFeatherRadius => 29,
            Self::GradientStopPosition => 30,
            Self::GradientStopOpacity => 31,
        }
    }

    fn is_crop(self) -> bool {
        matches!(
            self,
            Self::CropX | Self::CropY | Self::CropWidth | Self::CropHeight
        )
    }

    pub(super) fn is_scale(self) -> bool {
        matches!(self, Self::ScaleWidth | Self::ScaleHeight)
    }

    pub(super) fn is_move(self) -> bool {
        matches!(self, Self::MoveX | Self::MoveY)
    }

    pub(super) fn is_warp(self) -> bool {
        matches!(
            self,
            Self::RotateAngle
                | Self::RotatePivotX
                | Self::RotatePivotY
                | Self::PerspectiveX(_)
                | Self::PerspectiveY(_)
        )
    }

    fn is_warp_position(self) -> bool {
        matches!(
            self,
            Self::RotatePivotX | Self::RotatePivotY | Self::PerspectiveX(_) | Self::PerspectiveY(_)
        )
    }

    fn parse(self, text: &str) -> Option<i64> {
        if matches!(self, Self::GradientStopPosition | Self::GradientStopOpacity) {
            let value = text.parse::<f64>().ok()?;
            (value.is_finite() && (0.0..=100.0).contains(&value)).then(|| {
                let value = if self == Self::GradientStopPosition {
                    value * 10.
                } else {
                    value * 255. / 100.
                };
                value.round() as i64
            })
        } else if self.is_warp_position() {
            let value = text.parse::<f64>().ok()?;
            (value.is_finite() && value.abs() <= super::transform_ui::MAX_TRANSFORM_COORD)
                .then(|| (value * 10.).round() as i64)
        } else if self == Self::RotateAngle {
            let value = text.parse::<f64>().ok()?;
            (value.is_finite() && (-360.0..=360.0).contains(&value))
                .then(|| (value * 10.).round() as i64)
        } else {
            parse_numeric(text).filter(|&value| {
                (!self.is_scale()
                    || (0..=i64::from(crate::document::MAX_NEW_EDGE)).contains(&value))
                    && (!self.is_move() || i32::try_from(value).is_ok())
            })
        }
    }

    fn format(self, value: i64) -> String {
        if self == Self::GradientStopOpacity {
            format!("{:.1}", value as f64 * 100. / 255.)
        } else if self.is_warp_position()
            || matches!(self, Self::RotateAngle | Self::GradientStopPosition)
        {
            format!("{:.1}", value as f64 / 10.)
        } else {
            value.to_string()
        }
    }

    pub(super) fn is_selection(self) -> bool {
        matches!(
            self,
            Self::SelectionRadius
                | Self::SelectionWidth
                | Self::SelectionHeight
                | Self::SelectionX
                | Self::SelectionY
                | Self::SelectionRatioWidth
                | Self::SelectionRatioHeight
        )
    }

    fn is_selection_ratio(self) -> bool {
        matches!(self, Self::SelectionRatioWidth | Self::SelectionRatioHeight)
    }

    fn is_crop_ratio(self) -> bool {
        matches!(self, Self::CropRatioWidth | Self::CropRatioHeight)
    }

    fn accepts_character(self, ch: char) -> bool {
        ch.is_ascii_digit()
            || ((matches!(self, Self::TextLetterSpacing | Self::RotateAngle)
                || self.is_move()
                || self.is_warp_position())
                && ch == '-')
            || ((self.is_warp_position()
                || matches!(
                    self,
                    Self::RotateAngle | Self::GradientStopPosition | Self::GradientStopOpacity
                ))
                && ch == '.')
    }

    fn is_tool_option(self) -> bool {
        matches!(
            self,
            Self::TextBoxWidth
                | Self::TextBoxHeight
                | Self::TextLetterSpacing
                | Self::WandTolerance
                | Self::FillTolerance
                | Self::WandGapRadius
                | Self::FillGapRadius
                | Self::BrushSize
                | Self::BrushSpacing
                | Self::BrushHardness
                | Self::BrushOpacity
                | Self::BrushFlow
                | Self::PathStrokeSize
                | Self::RotateAngle
                | Self::RotatePivotX
                | Self::RotatePivotY
                | Self::PerspectiveX(_)
                | Self::PerspectiveY(_)
                | Self::GridSpacing
                | Self::SelectionRatioWidth
                | Self::SelectionRatioHeight
                | Self::CropRatioWidth
                | Self::CropRatioHeight
                | Self::SelectionFeatherRadius
                | Self::GradientStopPosition
                | Self::GradientStopOpacity
        )
    }

    fn constrain(self, value: i64) -> i64 {
        let (min, max) = match self {
            Self::WandTolerance | Self::FillTolerance => (0, 255),
            Self::WandGapRadius | Self::FillGapRadius => {
                (0, i64::from(crate::selection::MAX_GAP_RADIUS))
            }
            Self::TextBoxWidth | Self::TextBoxHeight => {
                (1, i64::from(crate::document::MAX_NEW_EDGE))
            }
            Self::GridSpacing => (1, 4096),
            Self::SelectionFeatherRadius => (0, 128),
            Self::GradientStopPosition => (0, 1000),
            Self::GradientStopOpacity => (0, 255),
            Self::SelectionRatioWidth
            | Self::SelectionRatioHeight
            | Self::CropRatioWidth
            | Self::CropRatioHeight => (1, 16384),
            Self::RotateAngle => (-3600, 3600),
            Self::RotatePivotX
            | Self::RotatePivotY
            | Self::PerspectiveX(_)
            | Self::PerspectiveY(_) => {
                let limit = (super::transform_ui::MAX_TRANSFORM_COORD * 10.).round() as i64;
                (-limit, limit)
            }
            Self::BrushSize | Self::PathStrokeSize => (1, 500),
            Self::BrushSpacing => (1, 100),
            Self::BrushHardness | Self::BrushOpacity | Self::BrushFlow => (0, 100),
            Self::TextLetterSpacing => (
                crate::text::MIN_LETTER_SPACING as i64,
                crate::text::MAX_LETTER_SPACING as i64,
            ),
            _ => (0, i64::from(u32::MAX)),
        };
        value.clamp(min, max)
    }
}

pub(super) struct NumericEdit {
    text_style: Option<super::text_ui::TextEdit>,
    pub(super) field: NumericField,
    text: String,
    cursor: usize,
    selected: bool,
    original: i64,
    geometry: NumericGeometry,
    compact_scroll: gpui::ScrollHandle,
    compact_focus: Option<gpui::FocusHandle>,
}

#[derive(Clone, Copy, Default)]
struct NumericGeometry {
    crop: Option<CropRect>,
    ratio: Option<(
        Option<crate::constraints::AspectRatio>,
        crate::constraints::AspectRatio,
    )>,
}

pub(super) enum NumericFollowup {
    CompactTab {
        origin: Option<gpui::FocusHandle>,
        backwards: bool,
    },
    PathVisibility {
        tab: usize,
        path: crate::paths::PathId,
        visible: bool,
    },
    Key(Keystroke),
    Save(super::files::SaveMode),
    Export,
    LayerCommand {
        layer: crate::document::LayerId,
        command: super::layers::LayerCommand,
    },
    TextOperation {
        along_path: bool,
        layer: crate::document::LayerId,
        path: Option<crate::paths::PathId>,
    },
    Stroke {
        source: super::stroke_ui::StrokeSource,
        dialog: bool,
        layer: crate::document::LayerId,
        path: Option<crate::paths::PathId>,
    },
    Focus(NumericField),
    Effect {
        reopen: bool,
    },
    FitLayer {
        layer: crate::document::LayerId,
    },
    LayerContextMenu {
        tab: usize,
        layer: crate::document::LayerId,
        position: gpui::Point<gpui::Pixels>,
    },
    LayerMetadata {
        tab: usize,
        layer: crate::document::LayerId,
        command: super::layers::LayerCommand,
    },
}

pub(super) struct PendingNumeric {
    tab: usize,
    generation: u64,
    followup: NumericFollowup,
}

impl PendingNumeric {
    fn accepts_completion(
        &self,
        tab: &super::Tab,
        id: usize,
        generation: u64,
        published: bool,
    ) -> bool {
        self.tab == id
            && tab.id == id
            && self.generation == generation
            && !tab.busy()
            && generation.checked_add(u64::from(published)) == Some(tab.history.generation())
            && match &self.followup {
                NumericFollowup::FitLayer { layer }
                | NumericFollowup::LayerCommand { layer, .. } => tab.active_layer == *layer,
                _ => true,
            }
    }
}

impl NumericEdit {
    fn new(field: NumericField, value: i64, crop: Option<CropRect>) -> Self {
        let text = field.format(value);
        Self {
            text_style: None,
            field,
            cursor: text.chars().count(),
            text,
            selected: true,
            original: value,
            geometry: NumericGeometry { crop, ratio: None },
            compact_scroll: gpui::ScrollHandle::new(),
            compact_focus: None,
        }
    }

    fn len(&self) -> usize {
        self.text.chars().count()
    }

    fn byte_at(&self, chars: usize) -> usize {
        self.text
            .char_indices()
            .nth(chars)
            .map(|(index, _)| index)
            .unwrap_or(self.text.len())
    }

    fn insert_digits(&mut self, text: &str) {
        let decimal = self.field.accepts_character('.');
        let digits: String = text
            .chars()
            .filter(|&ch| self.field.accepts_character(ch))
            .collect();
        if digits.is_empty() {
            return;
        }
        let mut next = if self.selected {
            String::new()
        } else {
            self.text.clone()
        };
        let at = if self.selected {
            0
        } else {
            self.byte_at(self.cursor)
        };
        next.insert_str(at, &digits);
        let unsigned = next.strip_prefix('-').unwrap_or(&next);
        if unsigned.len() > 10
            || !unsigned
                .chars()
                .all(|ch| ch.is_ascii_digit() || (decimal && ch == '.'))
            || unsigned.matches('.').count() > 1
        {
            return;
        }
        self.text = next;
        self.cursor = at + digits.len();
        self.selected = false;
    }

    fn backspace(&mut self) {
        if self.selected {
            self.text.clear();
            self.cursor = 0;
            self.selected = false;
        } else if self.cursor > 0 {
            self.cursor -= 1;
            let start = self.byte_at(self.cursor);
            let end = self.byte_at(self.cursor + 1);
            self.text.replace_range(start..end, "");
        }
    }

    fn delete(&mut self) {
        if self.selected {
            self.text.clear();
            self.cursor = 0;
            self.selected = false;
        } else if self.cursor < self.len() {
            let start = self.byte_at(self.cursor);
            let end = self.byte_at(self.cursor + 1);
            self.text.replace_range(start..end, "");
        }
    }

    fn move_left(&mut self) {
        self.cursor = if self.selected {
            0
        } else {
            self.cursor.saturating_sub(1)
        };
        self.selected = false;
    }

    fn move_right(&mut self) {
        self.cursor = if self.selected {
            self.len()
        } else {
            (self.cursor + 1).min(self.len())
        };
        self.selected = false;
    }

    pub(super) fn is_crop(&self) -> bool {
        self.field.is_crop()
    }
}

fn parse_numeric(text: &str) -> Option<i64> {
    (!text.is_empty())
        .then(|| text.parse::<i64>().ok())
        .flatten()
}

impl Workspace {
    pub(super) fn numeric_value(&self, field: NumericField) -> Option<i64> {
        let rect = self.selection_shape_rect();
        let crop = self.crop;
        match field {
            NumericField::SelectionX => rect.map(|rect| rect.x),
            NumericField::SelectionY => rect.map(|rect| rect.y),
            NumericField::SelectionRatioWidth => {
                Some(i64::from(self.preferences.selection_ratio_value.width))
            }
            NumericField::SelectionRatioHeight => {
                Some(i64::from(self.preferences.selection_ratio_value.height))
            }
            NumericField::CropRatioWidth => {
                Some(i64::from(self.preferences.crop_ratio_value.width))
            }
            NumericField::CropRatioHeight => {
                Some(i64::from(self.preferences.crop_ratio_value.height))
            }
            NumericField::SelectionRadius => Some(i64::from(self.active_selection_radius())),
            NumericField::SelectionWidth => rect.map(|rect| i64::from(rect.width)),
            NumericField::SelectionHeight => rect.map(|rect| i64::from(rect.height)),
            NumericField::CropX => crop.map(|crop| i64::from(crop.x)),
            NumericField::CropY => crop.map(|crop| i64::from(crop.y)),
            NumericField::CropWidth => crop.map(|crop| i64::from(crop.width)),
            NumericField::CropHeight => crop.map(|crop| i64::from(crop.height)),
            NumericField::TextSize => Some(self.text_size().round() as i64),
            NumericField::TextBoxWidth => self.text_layout().width.map(i64::from),
            NumericField::TextBoxHeight => self.text_layout().height.map(i64::from),
            NumericField::TextLineSpacing => {
                Some((self.text_line_spacing() * 100.0).round() as i64)
            }
            NumericField::TextLetterSpacing => Some(self.text_letter_spacing().round() as i64),
            NumericField::RotatePivotX | NumericField::RotatePivotY => {
                self.rotation_pivot().map(|pivot| {
                    ((if field == NumericField::RotatePivotX {
                        pivot.0
                    } else {
                        pivot.1
                    }) * 10.)
                        .round() as i64
                })
            }
            NumericField::PerspectiveX(index) | NumericField::PerspectiveY(index) => {
                self.perspective_point(index).map(|point| {
                    ((if matches!(field, NumericField::PerspectiveX(_)) {
                        point.0
                    } else {
                        point.1
                    }) * 10.)
                        .round() as i64
                })
            }
            NumericField::WandTolerance => Some(i64::from(self.preferences.wand_match.tolerance)),
            NumericField::FillTolerance => Some(i64::from(self.preferences.fill_match.tolerance)),
            NumericField::WandGapRadius => Some(i64::from(self.preferences.wand_match.gap_radius)),
            NumericField::FillGapRadius => Some(i64::from(self.preferences.fill_match.gap_radius)),
            NumericField::BrushSize => Some(i64::from(self.brush_value(BrushField::Size))),
            NumericField::BrushSpacing => Some(i64::from(self.brush_value(BrushField::Spacing))),
            NumericField::BrushHardness => Some(i64::from(self.brush_value(BrushField::Hardness))),
            NumericField::BrushOpacity => Some(i64::from(self.brush_value(BrushField::Opacity))),
            NumericField::BrushFlow => Some(i64::from(self.brush_value(BrushField::Flow))),
            NumericField::PathStrokeSize => {
                Some(i64::from(self.preferences.brushes.outline.diameter))
            }
            NumericField::ScaleWidth => self.scale_size().map(|size| i64::from(size.0)),
            NumericField::ScaleHeight => self.scale_size().map(|size| i64::from(size.1)),
            NumericField::RotateAngle => self
                .warp_status()?
                .degrees
                .map(|degrees| (degrees * 10.).round() as i64),
            NumericField::MoveX => self.move_offset().map(|offset| i64::from(offset.0)),
            NumericField::MoveY => self.move_offset().map(|offset| i64::from(offset.1)),
            NumericField::GridSpacing => Some(i64::from(self.preferences.grid_spacing)),
            NumericField::SelectionFeatherRadius => {
                Some(i64::from(self.preferences.selection_feather_radius))
            }
            NumericField::GradientStopPosition => self
                .selected_gradient_stop()
                .map(|stop| i64::from(stop.position)),
            NumericField::GradientStopOpacity => self
                .selected_gradient_stop()
                .map(|stop| i64::from(stop.color[3])),
        }
    }

    pub(super) fn begin_numeric_edit(
        &mut self,
        field: NumericField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.active_tab().is_none_or(|tab| tab.busy()) {
            return;
        }
        if field.is_warp() && self.warp_status().is_none_or(|status| status.applying) {
            return;
        }
        if let Some(edit) = self.numeric_edit.as_mut() {
            if edit.field == field {
                edit.selected = true;
                edit.cursor = edit.len();
                self.numeric_focus.focus(window);
                cx.notify();
                return;
            }
            if self.commit_numeric_with_followup(NumericFollowup::Focus(field), window, cx) {
                return;
            }
        }
        if field == NumericField::TextSize {
            self.prepare_text_format(window, cx);
        }
        let Some(value) = self.numeric_value(field) else {
            return;
        };
        let original_crop = if field.is_crop() || field.is_crop_ratio() {
            self.crop
        } else {
            None
        };
        if field.is_scale() && !self.begin_scale_numeric(window, cx) {
            return;
        }
        if field.is_move() && !self.begin_move_numeric(window, cx) {
            return;
        }
        let mut edit = NumericEdit::new(field, value, original_crop);
        if field == NumericField::TextSize {
            edit.text_style = self.text_draft.as_ref().map(|draft| draft.state());
        }
        edit.geometry.ratio = if field.is_selection_ratio() {
            Some((
                self.preferences.selection_ratio,
                self.preferences.selection_ratio_value,
            ))
        } else if field.is_crop_ratio() {
            Some((
                self.preferences.crop_ratio,
                self.preferences.crop_ratio_value,
            ))
        } else {
            None
        };
        if self.preferences.inspector_folded {
            edit.compact_focus = window.focused(cx);
        }
        self.numeric_edit = Some(edit);
        self.numeric_focus.focus(window);
        cx.notify();
    }

    pub(super) fn numeric_key(
        &mut self,
        key: &Keystroke,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.numeric_edit.is_none() {
            return false;
        }
        if self.preferences.inspector_folded
            && let Some(backwards) = super::compact_inspector::compact_tab_direction(key)
        {
            let origin = self
                .numeric_edit
                .as_ref()
                .and_then(|edit| edit.compact_focus.clone());
            if !self.commit_numeric_with_followup(
                NumericFollowup::CompactTab {
                    origin: origin.clone(),
                    backwards,
                },
                window,
                cx,
            ) {
                super::compact_inspector::focus_compact_neighbor(origin, backwards, window);
            }
            return true;
        }
        if key.key == "escape" {
            self.cancel_numeric_edit(window, cx);
            return true;
        }
        if key.key == "enter" {
            self.commit_numeric_edit(window, cx);
            self.focus.focus(window);
            return true;
        }
        if key.modifiers.secondary() {
            match key.key.as_str() {
                "a" => {
                    if let Some(edit) = self.numeric_edit.as_mut() {
                        edit.selected = true;
                        edit.cursor = edit.len();
                    }
                    cx.notify();
                }
                "c" => {
                    if let Some(edit) = self.numeric_edit.as_ref() {
                        cx.write_to_clipboard(ClipboardItem::new_string(edit.text.clone()));
                    }
                    cx.notify();
                }
                "x" => {
                    if let Some(edit) = self.numeric_edit.as_ref() {
                        cx.write_to_clipboard(ClipboardItem::new_string(edit.text.clone()));
                    }
                    if let Some(edit) = self.numeric_edit.as_mut() {
                        edit.text.clear();
                        edit.cursor = 0;
                        edit.selected = false;
                    }
                    self.apply_numeric_fields(window, cx);
                }
                "v" => {
                    let text = cx
                        .read_from_clipboard()
                        .and_then(|item| item.text())
                        .unwrap_or_default();
                    if let Some(edit) = self.numeric_edit.as_mut() {
                        edit.insert_digits(&text);
                    }
                    self.apply_numeric_fields(window, cx);
                }
                _ => return false,
            }
            return true;
        }
        if key.modifiers.alt || key.modifiers.control || key.modifiers.platform {
            return false;
        }
        let mut changed = false;
        {
            let Some(edit) = self.numeric_edit.as_mut() else {
                return false;
            };
            match key.key.as_str() {
                "backspace" => {
                    edit.backspace();
                    changed = true;
                }
                "delete" => {
                    edit.delete();
                    changed = true;
                }
                "left" => edit.move_left(),
                "right" => edit.move_right(),
                "home" => {
                    edit.cursor = 0;
                    edit.selected = false;
                }
                "end" => {
                    edit.cursor = edit.len();
                    edit.selected = false;
                }
                _ => {
                    if let Some(text) = key.key_char.as_deref()
                        && text.chars().any(|ch| edit.field.accepts_character(ch))
                    {
                        edit.insert_digits(text);
                        changed = true;
                    }
                }
            }
        }
        if changed {
            self.apply_numeric_fields(window, cx);
        } else {
            cx.notify();
        }
        true
    }

    pub(super) fn apply_numeric_fields(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((field, text, original, geometry)) = self
            .numeric_edit
            .as_ref()
            .map(|edit| (edit.field, edit.text.clone(), edit.original, edit.geometry))
        else {
            return;
        };
        let Some(value) = field.parse(&text) else {
            self.restore_numeric_values(field, original, geometry, window, cx);
            cx.notify();
            return;
        };
        if field.is_tool_option() {
            self.apply_tool_numeric(field, value, window, cx);
            return;
        }
        if field.is_scale() {
            if let Ok(value) = u32::try_from(value) {
                if value == 0 {
                    self.restore_scale_size(window, cx);
                } else {
                    self.preview_scale_size(field == NumericField::ScaleWidth, value, window, cx);
                }
            }
            return;
        }
        if field.is_move() {
            self.preview_move_position(field == NumericField::MoveX, value as i32, cx);
            return;
        }
        let value = value.clamp(0, i64::from(u32::MAX)) as u32;
        match field {
            NumericField::TextSize => {
                // Sizes below the minimum are mid-typing; wait for more digits.
                if (MIN_PIXEL_SIZE as u32..=MAX_PIXEL_SIZE as u32).contains(&value) {
                    self.set_text_size(value as f32, window, cx);
                }
            }
            NumericField::TextLineSpacing => {
                let line = value as f32 / 100.0;
                if (MIN_LINE_SPACING..=MAX_LINE_SPACING).contains(&line) {
                    self.set_text_line_spacing(line, window, cx);
                }
            }
            NumericField::SelectionRadius => {
                let rect = self.selection_shape_rect();
                let max = rect.map_or_else(
                    || {
                        self.active_tab().map_or(0, |tab| {
                            let (width, height) = tab.document.content.size();
                            width.max(height)
                        })
                    },
                    |rect| rect.width.min(rect.height) / 2,
                );
                self.preferences.selection_radius = value.min(max);
                if let Some(selection) = rect.and_then(|rect| {
                    self.selection_for_rect(rect, Some(self.preferences.selection_radius))
                }) {
                    self.selection_preview = Some(selection);
                }
            }
            NumericField::SelectionX | NumericField::SelectionY => {
                let Some(mut rect) = self.selection_shape_rect() else {
                    return;
                };
                let Some(tab) = self.active_tab() else {
                    return;
                };
                if field == NumericField::SelectionX {
                    rect.x = i64::from(value.min(tab.document.width().saturating_sub(rect.width)));
                } else {
                    rect.y =
                        i64::from(value.min(tab.document.height().saturating_sub(rect.height)));
                }
                self.selection_preview = self.selection_for_rect(rect, None);
            }
            NumericField::SelectionWidth | NumericField::SelectionHeight => {
                if value == 0 {
                    self.restore_numeric_values(field, original, geometry, window, cx);
                    cx.notify();
                    return;
                }
                let Some(mut rect) = self.selection_shape_rect() else {
                    return;
                };
                let Some(tab) = self.active_tab() else {
                    return;
                };
                let (width, height) = tab.document.content.size();
                let limits = (
                    width
                        .saturating_sub(u32::try_from(rect.x).unwrap_or(0))
                        .max(1),
                    height
                        .saturating_sub(u32::try_from(rect.y).unwrap_or(0))
                        .max(1),
                );
                if let Some(ratio) = self.effective_selection_ratio() {
                    (rect.width, rect.height) =
                        ratio.fit(field == NumericField::SelectionWidth, value, limits);
                } else if field == NumericField::SelectionWidth {
                    rect.width = value.min(limits.0);
                } else {
                    rect.height = value.min(limits.1);
                }
                if let Some(selection) = self.selection_for_rect(rect, None) {
                    self.selection_preview = Some(selection);
                }
            }
            NumericField::CropX | NumericField::CropY => {
                let Some(mut crop) = self.crop else {
                    return;
                };
                let Some(tab) = self.active_tab() else {
                    return;
                };
                let (width, height) = tab.document.content.size();
                if field == NumericField::CropX {
                    crop.x = value.min(width.saturating_sub(crop.width));
                } else {
                    crop.y = value.min(height.saturating_sub(crop.height));
                }
                self.crop = Some(crop);
            }
            NumericField::CropWidth | NumericField::CropHeight => {
                if value == 0 {
                    self.restore_numeric_values(field, original, geometry, window, cx);
                    cx.notify();
                    return;
                }
                let Some(mut crop) = self.crop else {
                    return;
                };
                let Some(tab) = self.active_tab() else {
                    return;
                };
                let (width, height) = tab.document.content.size();
                let limits = (
                    width.saturating_sub(crop.x).max(1),
                    height.saturating_sub(crop.y).max(1),
                );
                if let Some(ratio) = self.preferences.crop_ratio {
                    (crop.width, crop.height) =
                        ratio.fit(field == NumericField::CropWidth, value, limits);
                } else if field == NumericField::CropWidth {
                    crop.width = value.min(limits.0);
                } else {
                    crop.height = value.min(limits.1);
                }
                self.crop = Some(crop);
            }
            _ => unreachable!("tool options were handled before selection and crop fields"),
        }
        cx.notify();
    }

    pub(super) fn commit_numeric_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(edit) = self.numeric_edit.take() else {
            return;
        };
        if edit.field.parse(&edit.text).is_none() {
            self.restore_numeric_edit(&edit, window, cx);
            if edit.field.is_move() {
                self.cancel_move(window, cx);
            }
        } else if let Some(state) = edit.text_style {
            self.finish_text_format(state);
        } else if edit.field.is_move() {
            self.finish_move(window, cx);
        } else if edit.field.is_selection()
            && let Some(selection) = self.selection_preview.take()
        {
            self.commit_placed_selection(selection, window, cx);
        }
        cx.notify();
    }

    pub(super) fn commit_numeric_with_followup(
        &mut self,
        followup: NumericFollowup,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let before = self
            .active_tab()
            .filter(|tab| !tab.busy())
            .map(|tab| (tab.id, tab.history.generation()));
        self.commit_numeric_edit(window, cx);
        let Some((id, generation)) = before else {
            return false;
        };
        if self.active_tab().is_some_and(|tab| {
            tab.id == id && tab.editing && tab.history.generation() == generation
        }) {
            self.numeric_followup = Some(PendingNumeric {
                tab: id,
                generation,
                followup,
            });
            return true;
        }
        false
    }

    pub(super) fn finish_numeric_followup(
        &mut self,
        id: usize,
        generation: u64,
        published: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .numeric_followup
            .as_ref()
            .is_none_or(|pending| pending.tab != id)
        {
            return;
        }
        let pending = self.numeric_followup.take().unwrap();
        let Some(tab) = self
            .active_tab()
            .filter(|tab| pending.accepts_completion(tab, id, generation, published))
        else {
            return;
        };
        let settled_generation = tab.history.generation();
        let tool = self.preferences.tool;
        let entity = cx.weak_entity();
        // Menu availability and key contexts must render the settled edit first.
        // Dispatch outside the entity update so keystroke interceptors can borrow it.
        window.on_next_frame(move |window, cx| {
            let key = entity
                .update(cx, |this, cx| {
                    if this.form.is_some()
                        || this.dialog_open
                        || this.numeric_edit.is_some()
                        || this.preferences.tool != tool
                        || this.active_tab().is_none_or(|tab| {
                            tab.id != id
                                || tab.busy()
                                || tab.history.generation() != settled_generation
                        })
                    {
                        return None;
                    }
                    match pending.followup {
                        NumericFollowup::CompactTab { origin, backwards } => {
                            super::compact_inspector::focus_compact_neighbor(
                                origin, backwards, window,
                            );
                            None
                        }
                        NumericFollowup::PathVisibility { tab, path, visible } => {
                            this.edit_path_visibility(tab, path, visible, window, cx);
                            None
                        }
                        NumericFollowup::TextOperation {
                            along_path,
                            layer,
                            path,
                        } => {
                            if this.active_tab().is_some_and(|tab| {
                                tab.active_layer == layer
                                    && (!along_path
                                        || tab.document.content.active_path_id() == path)
                            }) {
                                if along_path {
                                    this.text_path_form(window, cx);
                                } else {
                                    this.alpha_to_selection(window, cx);
                                }
                            }
                            None
                        }
                        NumericFollowup::Key(key) => {
                            this.focus.focus(window);
                            Some(key)
                        }
                        NumericFollowup::Focus(field) => {
                            this.begin_numeric_edit(field, window, cx);
                            None
                        }
                        NumericFollowup::Effect { reopen } => {
                            this.run_last_effect(reopen, window, cx);
                            None
                        }
                        NumericFollowup::Stroke {
                            source,
                            dialog,
                            layer,
                            path,
                        } => {
                            if this.active_tab().is_some_and(|tab| {
                                tab.active_layer == layer
                                    && path.is_none_or(|id| {
                                        tab.document
                                            .content
                                            .active_path()
                                            .is_some_and(|active| active.id == id)
                                    })
                            }) {
                                if dialog {
                                    this.stroke_form(source, window, cx);
                                } else {
                                    this.stroke_outline(source, window, cx);
                                }
                            }
                            None
                        }
                        NumericFollowup::Export => {
                            this.export_form(window, cx);
                            None
                        }
                        NumericFollowup::Save(mode) => {
                            this.save(mode, window, cx);
                            None
                        }
                        NumericFollowup::LayerCommand { layer, command } => {
                            if this
                                .active_tab()
                                .is_some_and(|tab| tab.active_layer == layer)
                            {
                                this.run_layer_command(command, window, cx);
                            }
                            None
                        }
                        NumericFollowup::FitLayer { layer } => {
                            if this
                                .active_tab()
                                .is_some_and(|tab| tab.active_layer == layer)
                            {
                                this.fit_active_layer_to_canvas(window, cx);
                            }
                            None
                        }
                        NumericFollowup::LayerContextMenu {
                            tab,
                            layer,
                            position,
                        } => {
                            this.open_layer_context_menu(tab, layer, position, window, cx);
                            None
                        }
                        NumericFollowup::LayerMetadata {
                            tab,
                            layer,
                            command,
                        } => {
                            this.edit_layer_metadata(tab, layer, command, window, cx);
                            None
                        }
                    }
                })
                .ok()
                .flatten();
            if let Some(key) = key {
                window.dispatch_keystroke(key, cx);
            }
        });
    }

    pub(super) fn cancel_numeric_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(edit) = self.numeric_edit.take() else {
            return;
        };
        self.restore_numeric_edit(&edit, window, cx);
        if edit.field.is_move() {
            self.cancel_move(window, cx);
        }
        self.focus.focus(window);
        cx.notify();
    }

    fn restore_numeric_edit(
        &mut self,
        edit: &NumericEdit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(state) = &edit.text_style {
            self.restore_text_format(state.clone(), window, cx);
        } else {
            self.restore_numeric_values(edit.field, edit.original, edit.geometry, window, cx);
        }
    }

    fn restore_numeric_values(
        &mut self,
        field: NumericField,
        original: i64,
        geometry: NumericGeometry,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if field == NumericField::TextSize
            && let Some(state) = self
                .numeric_edit
                .as_ref()
                .and_then(|edit| edit.text_style.clone())
        {
            self.restore_text_format(state, window, cx);
            return;
        }
        if let Some((ratio, value)) = geometry.ratio {
            if field.is_crop_ratio() {
                self.preferences.crop_ratio = ratio;
                self.preferences.crop_ratio_value = value;
                self.crop = geometry.crop;
            } else {
                self.preferences.selection_ratio = ratio;
                self.preferences.selection_ratio_value = value;
                self.selection_preview = None;
            }
            return;
        }
        if field.is_tool_option() {
            self.apply_tool_numeric(field, original, window, cx);
            return;
        }
        if field.is_scale() {
            self.restore_scale_size(window, cx);
            return;
        }
        if field.is_move() {
            self.restore_move_position(cx);
            return;
        }
        match field {
            NumericField::TextSize => self.set_text_size(original as f32, window, cx),
            NumericField::TextLineSpacing => {
                self.set_text_line_spacing(original as f32 / 100.0, window, cx)
            }
            NumericField::SelectionRadius => {
                self.preferences.selection_radius = original as u32;
                self.selection_preview = None;
            }
            NumericField::SelectionWidth
            | NumericField::SelectionHeight
            | NumericField::SelectionX
            | NumericField::SelectionY => {
                self.selection_preview = None;
            }
            NumericField::CropX
            | NumericField::CropY
            | NumericField::CropWidth
            | NumericField::CropHeight => self.crop = geometry.crop,
            _ => unreachable!("tool options were restored before selection and crop fields"),
        }
    }

    fn apply_tool_numeric(
        &mut self,
        field: NumericField,
        value: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let value = field.constrain(value);
        match field {
            NumericField::WandTolerance => self.preferences.wand_match.tolerance = value as u8,
            NumericField::FillTolerance => self.preferences.fill_match.tolerance = value as u8,
            NumericField::WandGapRadius => self.preferences.wand_match.gap_radius = value as u8,
            NumericField::FillGapRadius => self.preferences.fill_match.gap_radius = value as u8,
            NumericField::GridSpacing => self.preferences.grid_spacing = value as u32,
            NumericField::SelectionFeatherRadius => {
                self.preferences.selection_feather_radius = value as u32
            }
            NumericField::GradientStopPosition => self.move_gradient_stop(value as u16, window, cx),
            NumericField::GradientStopOpacity => {
                self.set_gradient_stop_alpha(value as u8, window, cx)
            }
            NumericField::SelectionRatioWidth | NumericField::SelectionRatioHeight => {
                if field == NumericField::SelectionRatioWidth {
                    self.preferences.selection_ratio_value.width = value as u32;
                } else {
                    self.preferences.selection_ratio_value.height = value as u32;
                }
                let ratio = self.preferences.selection_ratio_value;
                self.preferences.selection_ratio = Some(ratio);
                self.selection_preview = self.active_tab().and_then(|tab| {
                    let selection = &tab.selection;
                    let rect = selection
                        .feather_geometry()
                        .map(|geometry| geometry.rect)
                        .or_else(|| match selection {
                            Selection::Rect(rect) => Some(*rect),
                            Selection::RoundedRect(shape) => Some(shape.rect()),
                            Selection::Ellipse(shape) => Some(shape.rect()),
                            _ => None,
                        })?;
                    super::selection::editable_selection_for_rect(
                        selection,
                        ratio.inscribe(rect),
                        None,
                        selection.is_antialiased(),
                        tab.document.content.size(),
                    )
                });
            }
            NumericField::CropRatioWidth | NumericField::CropRatioHeight => {
                if field == NumericField::CropRatioWidth {
                    self.preferences.crop_ratio_value.width = value as u32;
                } else {
                    self.preferences.crop_ratio_value.height = value as u32;
                }
                let ratio = self.preferences.crop_ratio_value;
                self.preferences.crop_ratio = Some(ratio);
                let baseline = self
                    .numeric_edit
                    .as_ref()
                    .and_then(|edit| edit.geometry.crop)
                    .or(self.crop);
                self.crop = baseline.map(|crop| super::selection::ratio_crop(crop, ratio));
            }
            NumericField::BrushSize => self.set_brush_value(BrushField::Size, value as u32, cx),
            NumericField::BrushSpacing => {
                self.set_brush_value(BrushField::Spacing, value as u32, cx)
            }
            NumericField::BrushHardness => {
                self.set_brush_value(BrushField::Hardness, value as u32, cx)
            }
            NumericField::BrushOpacity => {
                self.set_brush_value(BrushField::Opacity, value as u32, cx)
            }
            NumericField::BrushFlow => self.set_brush_value(BrushField::Flow, value as u32, cx),
            NumericField::PathStrokeSize => {
                self.preferences.brushes.outline.diameter = value as u32
            }
            NumericField::RotateAngle => self.set_warp_angle(value as f64 / 10., window, cx),
            NumericField::RotatePivotX | NumericField::RotatePivotY => {
                if let Some(mut pivot) = self.rotation_pivot() {
                    if field == NumericField::RotatePivotX {
                        pivot.0 = value as f64 / 10.;
                    } else {
                        pivot.1 = value as f64 / 10.;
                    }
                    self.set_rotation_pivot(pivot, window, cx);
                }
            }
            NumericField::PerspectiveX(index) | NumericField::PerspectiveY(index) => {
                if let Some(mut point) = self.perspective_point(index) {
                    if matches!(field, NumericField::PerspectiveX(_)) {
                        point.0 = value as f64 / 10.;
                    } else {
                        point.1 = value as f64 / 10.;
                    }
                    self.set_perspective_point(index, point, window, cx);
                }
            }
            NumericField::TextBoxWidth | NumericField::TextBoxHeight => {
                self.set_text_box_edge(
                    field == NumericField::TextBoxWidth,
                    value as u32,
                    window,
                    cx,
                );
            }
            NumericField::TextLetterSpacing => {
                self.set_text_letter_spacing(value as f32, window, cx)
            }
            _ => unreachable!("only tool settings use this numeric setter"),
        }
        cx.notify();
    }

    pub(super) fn numeric_stepper(
        &self,
        field: NumericField,
        down: gpui::Stateful<Div>,
        up: gpui::Stateful<Div>,
        cx: &Context<Self>,
    ) -> Div {
        div()
            .flex()
            .items_center()
            .gap_1()
            .child(down)
            .child(self.numeric_chip(field, None, cx))
            .child(up)
    }

    fn selection_for_rect(&self, rect: PixelRect, radius: Option<u32>) -> Option<Selection> {
        let tab = self.active_tab()?;
        let (width, height) = tab.document.content.size();
        let current = &tab.selection;
        super::selection::editable_selection_for_rect(
            current,
            rect,
            radius,
            self.preferences.selection_antialias,
            (width, height),
        )
    }

    pub(super) fn numeric_chip(
        &self,
        field: NumericField,
        label: Option<&'static str>,
        cx: &Context<Self>,
    ) -> gpui::Stateful<Div> {
        let value = self.numeric_text(field);
        let chip = self.numeric_chip_base(field, cx);
        if let Some(label) = label {
            chip.flex_1()
                .min_w_0()
                .justify_between()
                .child(
                    div()
                        .text_size(px(10.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(MUTED))
                        .child(label),
                )
                .child(value)
        } else {
            chip.min_w(px(48.)).justify_center().child(value)
        }
    }

    fn numeric_chip_base(&self, field: NumericField, cx: &Context<Self>) -> gpui::Stateful<Div> {
        let active = self
            .numeric_edit
            .as_ref()
            .is_some_and(|edit| edit.field == field);
        div()
            .id(("numeric-field", field.id()))
            .h(px(26.))
            .flex()
            .items_center()
            .gap_1()
            .px_2()
            .rounded_md()
            .bg(rgb(CANVAS))
            .border_1()
            .border_color(rgb(if active { ACCENT } else { BORDER }))
            .cursor(CursorStyle::IBeam)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    this.begin_numeric_edit(field, window, cx);
                }),
            )
    }

    pub(super) fn compact_numeric_chip(
        &self,
        field: NumericField,
        cx: &Context<Self>,
    ) -> gpui::Stateful<Div> {
        let edit = self
            .numeric_edit
            .as_ref()
            .filter(|edit| edit.field == field);
        let full_value = edit.map_or_else(
            || {
                if field == NumericField::TextSize && self.text_size_mixed() {
                    "Mixed".into()
                } else {
                    field.format(self.numeric_value(field).unwrap_or(0))
                }
            },
            |edit| edit.text.clone(),
        );
        let value = if let Some(edit) = edit {
            let scroll = &edit.compact_scroll;
            let text = div()
                .id(("compact-numeric-scroll", field.id()))
                .w_full()
                .min_w_0()
                .flex()
                .items_center()
                .text_size(px(12.))
                .overflow_x_scroll()
                .scrollbar_width(px(0.))
                .track_scroll(scroll);
            if edit.selected {
                scroll.set_offset(gpui::Point::default());
                text.child(
                    div()
                        .flex_shrink_0()
                        .whitespace_nowrap()
                        .bg(rgb(ACCENT))
                        .text_color(rgb(CANVAS))
                        .child(edit.text.clone()),
                )
            } else {
                scroll.scroll_to_item(1);
                let at = edit.byte_at(edit.cursor);
                text.child(
                    div()
                        .flex_shrink_0()
                        .whitespace_nowrap()
                        .child(edit.text[..at].to_owned()),
                )
                .child(div().w(px(1.)).h(px(14.)).flex_shrink_0().bg(rgb(ACCENT)))
                .child(
                    div()
                        .flex_shrink_0()
                        .whitespace_nowrap()
                        .child(edit.text[at..].to_owned()),
                )
            }
        } else {
            div()
                .id(("compact-numeric-scroll", field.id()))
                .w_full()
                .min_w_0()
                .text_size(px(12.))
                .text_center()
                .text_ellipsis()
                .child(full_value.clone())
        };
        super::compact_inspector::compact_control(self.numeric_chip_base(field, cx))
            .w_full()
            .when(field == NumericField::TextSize, |chip| {
                chip.w(px(super::view::TOOLBOX_FOLDED_WIDTH - 10.))
            })
            .min_w_0()
            .px(px(2.))
            // Pointer editing owns the shared numeric focus. Do not briefly
            // focus this chip and blur/commit an existing edit when reselecting it.
            .capture_any_mouse_down(|event, window, _| {
                if event.button == MouseButton::Left {
                    window.prevent_default();
                }
            })
            .tooltip(super::tooltip::tooltip(full_value))
            .on_click(
                cx.listener(move |this, event: &gpui::ClickEvent, window, cx| {
                    if matches!(event, gpui::ClickEvent::Keyboard(_)) {
                        this.begin_numeric_edit(field, window, cx);
                    }
                }),
            )
            .child(value)
    }

    fn numeric_text(&self, field: NumericField) -> Div {
        let Some(edit) = self
            .numeric_edit
            .as_ref()
            .filter(|edit| edit.field == field)
        else {
            let value = self.numeric_value(field).unwrap_or(0);
            return div().text_size(px(12.)).child(
                if field == NumericField::TextSize && self.text_size_mixed() {
                    "Mixed".into()
                } else {
                    field.format(value)
                },
            );
        };
        let text = div().flex().items_center().text_size(px(12.));
        if edit.selected && !edit.text.is_empty() {
            text.child(
                div()
                    .bg(rgb(ACCENT))
                    .text_color(rgb(CANVAS))
                    .child(edit.text.clone()),
            )
        } else {
            let at = edit.byte_at(edit.cursor);
            text.child(edit.text[..at].to_owned())
                .child(div().w(px(1.)).h(px(14.)).flex_shrink_0().bg(rgb(ACCENT)))
                .child(edit.text[at..].to_owned())
        }
    }
}
