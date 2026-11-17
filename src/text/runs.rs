//! Canonical character styling. Ranges use UTF-8 offsets at grapheme boundaries.
use super::{FontFace, Spacing, TextSpec, fields_allowed};
use serde::{Deserialize, Serialize};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterStyle {
    pub family: String,
    pub face: FontFace,
    pub size: f32,
    pub color: [u8; 4],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StyleRun {
    pub start: usize,
    pub end: usize,
    pub style: CharacterStyle,
}

pub fn allowed(source: &str, runs: &[StyleRun]) -> bool {
    if runs.is_empty() {
        return true;
    }
    if runs.len() > super::MAX_TEXT_CHARS {
        return false;
    }
    let mut boundaries: Vec<_> = source.grapheme_indices(true).map(|(i, _)| i).collect();
    boundaries.push(source.len());
    let mut end = 0;
    for run in runs {
        if run.start != end
            || run.end <= run.start
            || boundaries.binary_search(&run.end).is_err()
            || !fields_allowed(
                "",
                &run.style.family,
                "",
                run.style.size,
                Spacing::default(),
            )
        {
            return false;
        }
        end = run.end;
    }
    end == source.len()
}

impl TextSpec {
    pub fn base_style(&self) -> CharacterStyle {
        CharacterStyle {
            family: self.family.clone(),
            face: self.face,
            size: self.size,
            color: self.color.0,
        }
    }

    /// At a boundary use the following character; at end use the last one.
    pub fn style_at(&self, byte: usize) -> CharacterStyle {
        let index = self.runs.partition_point(|run| run.end <= byte);
        self.runs
            .get(index)
            .or_else(|| self.runs.last().filter(|_| byte == self.source.len()))
            .map_or_else(|| self.base_style(), |run| run.style.clone())
    }

    pub fn styles_in(&self, range: Range<usize>) -> Vec<CharacterStyle> {
        if self.runs.is_empty() {
            return vec![self.base_style()];
        }
        self.runs
            .iter()
            .filter(|run| run.start < range.end && run.end > range.start)
            .map(|run| run.style.clone())
            .collect()
    }

    pub fn format_range(&mut self, range: Range<usize>, change: impl Fn(&mut CharacterStyle)) {
        let runs = self
            .source
            .grapheme_indices(true)
            .map(|(start, grapheme)| {
                let mut style = self.style_at(start);
                if start >= range.start && start < range.end {
                    change(&mut style);
                }
                StyleRun {
                    start,
                    end: start + grapheme.len(),
                    style,
                }
            })
            .collect();
        self.set_runs(runs);
    }

    pub fn replace_styled(
        &mut self,
        range: Range<usize>,
        replacement: &str,
        style: &CharacterStyle,
        inserted: &[StyleRun],
    ) {
        let mut runs = Vec::new();
        for (start, ch) in self.source.char_indices().filter(|(i, _)| *i < range.start) {
            runs.push(StyleRun {
                start,
                end: start + ch.len_utf8(),
                style: self.style_at(start),
            });
        }
        for (offset, ch) in replacement.char_indices() {
            let style = inserted
                .iter()
                .find(|r| r.start <= offset && offset < r.end)
                .map_or(style, |r| &r.style)
                .clone();
            let start = range.start + offset;
            runs.push(StyleRun {
                start,
                end: start + ch.len_utf8(),
                style,
            });
        }
        for (offset, ch) in self.source.char_indices().filter(|(i, _)| *i >= range.end) {
            let start = range.start + replacement.len() + offset - range.end;
            runs.push(StyleRun {
                start,
                end: start + ch.len_utf8(),
                style: self.style_at(offset),
            });
        }
        self.source.replace_range(range, replacement);
        // An inserted combining mark can join the preceding grapheme. Its base
        // character owns that cluster's style, never split a shaping cluster.
        let aligned = self
            .source
            .grapheme_indices(true)
            .map(|(start, g)| {
                let index = runs.partition_point(|r| r.end <= start);
                StyleRun {
                    start,
                    end: start + g.len(),
                    style: runs[index].style.clone(),
                }
            })
            .collect();
        self.set_runs(aligned);
    }

    fn set_runs(&mut self, runs: Vec<StyleRun>) {
        let mut merged: Vec<StyleRun> = Vec::new();
        for run in runs {
            if let Some(last) = merged.last_mut().filter(|last| last.style == run.style) {
                last.end = run.end;
            } else {
                merged.push(run);
            }
        }
        if merged.len() == 1 && merged[0].style == self.base_style() {
            merged.clear();
        }
        self.runs = merged;
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn sample(source: &str) -> TextSpec {
        TextSpec {
            source: source.into(),
            runs: Vec::new(),
            family: "sans-serif".into(),
            face: FontFace::default(),
            size: 24.,
            color: image::Rgba([20, 30, 40, 255]),
            spacing: Spacing::default(),
            layout: Default::default(),
        }
    }

    #[test]
    fn styled_edits_preserve_neighbors_and_joined_graphemes() {
        let mut spec = sample("aကောင်း👩‍💻z");
        let start = 1;
        let end = spec.source.find('👩').unwrap();
        spec.format_range(start..end, |style| {
            style.face.bold = true;
            style.color = [255, 0, 0, 128];
        });
        assert!(allowed(&spec.source, &spec.runs));
        assert!(!spec.style_at(0).face.bold);
        assert!(spec.style_at(start).face.bold);
        assert!(!spec.style_at(end).face.bold);
        let inherited = spec.style_at(start);
        spec.replace_styled(start..end, "e", &inherited, &[]);
        spec.replace_styled(2..2, "\u{301}", &spec.base_style(), &[]);
        assert!(spec.source.starts_with("ae\u{301}"));
        assert!(allowed(&spec.source, &spec.runs));
        assert!(spec.style_at(1).face.bold);
        assert!(spec.style_at(2).face.bold);
        let emoji = spec.source.find('👩').unwrap();
        spec.replace_styled(emoji..spec.source.len() - 1, "", &inherited, &[]);
        assert_eq!(spec.source, "ae\u{301}z");
        assert!(!spec.style_at(spec.source.len()).face.bold);
        // Independent properties survive a later whole-range color change.
        spec.format_range(0..spec.source.len(), |style| style.color = [0, 0, 255, 255]);
        assert!(spec.style_at(1).face.bold);
        assert!(!spec.style_at(0).face.bold);
        let mut broken = spec.runs.clone();
        broken[0].end = 2; // splits the next grapheme and overlaps its run
        assert!(!allowed(&spec.source, &broken));
    }

    #[test]
    fn mixed_rendering_preserves_alpha_and_variable_line_carets() {
        let mut spec = sample("AA\nBB");
        spec.format_range(0..2, |style| {
            style.color = [255, 0, 0, 90];
            style.size = 16.;
        });
        spec.format_range(3..5, |style| {
            style.color = [0, 0, 255, 255];
            style.size = 48.;
            style.face.bold = true;
        });
        let rendered = super::super::render_spec(&spec).unwrap();
        assert_eq!(rendered.style.spec(), spec);
        assert!(rendered.image.pixels().any(|p| p[0] == 255 && p[3] > 0));
        assert!(
            rendered
                .image
                .pixels()
                .filter(|p| p[0] == 255)
                .all(|p| p[3] <= 90)
        );
        assert!(rendered.image.pixels().any(|p| p[2] == 255 && p[3] > 90));
        let carets = super::super::caret_map(&spec).unwrap();
        let first = carets.at(0).unwrap();
        let second = carets.at(3).unwrap();
        assert!(second.height > first.height);
        assert_eq!(
            carets
                .hit_position(second.x, second.top + second.height / 2.)
                .unwrap()
                .0,
            3
        );
        assert!(
            carets
                .selection_rects(3..5)
                .iter()
                .all(|rect| rect.3 == second.height)
        );
        let mut path_spec = spec.clone();
        path_spec.source.replace_range(2..3, " ");
        let guide = crate::paths::VectorPath {
            id: crate::paths::PathId(1),
            visible: true,
            closed: false,
            anchors: vec![
                crate::paths::Anchor::corner(crate::paths::AnchorId(1), (0., 80.)),
                crate::paths::Anchor::corner(crate::paths::AnchorId(2), (400., 80.)),
            ],
        };
        let curved = super::super::path::prepare(
            &path_spec,
            &guide,
            Default::default(),
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap()
        .render()
        .unwrap();
        assert!(curved.pixels().any(|p| p[0] > 200 && p[3] > 0));
        assert!(curved.pixels().any(|p| p[2] > 200 && p[3] > 90));
        assert!(curved.pixels().filter(|p| p[0] > 200).all(|p| p[3] <= 90));
        spec.layout.width = Some(65);
        spec.layout.height = Some(130);
        assert!(super::super::render_spec(&spec).is_ok());
        assert!(super::super::caret_map(&spec).unwrap().stops.len() >= 6);
    }
}
