//! One-shot curved lettering. Shape the complete line before rotating clusters;
//! the result is raster content, with no persistent dependency on its guide.
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result, ensure};
use cosmic_text::{CacheKeyFlags, Command};
use image::RgbaImage;
use resvg::tiny_skia::{FillRule, Paint, Path, PathBuilder, Pixmap, Transform};

use super::{TextAlign, TextLayout, TextSpec};
use crate::document::{MAX_IMAGE_BYTES, MAX_NEW_EDGE};
use crate::paths::VectorPath;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Options {
    pub align: TextAlign,
    pub offset: f64,
    // Positive is below the baseline in the direction of travel.
    pub baseline: f64,
    pub reverse: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            align: TextAlign::Left,
            offset: 0.,
            baseline: 0.,
            reverse: false,
        }
    }
}

pub struct Prepared {
    paths: Vec<(Path, [u8; 4])>,
    pub offset: (i32, i32),
    pub size: (u32, u32),
    pub overflow: bool,
}

struct Guide {
    points: Vec<(f64, f64)>,
    distances: Vec<f64>,
    length: f64,
}

impl Guide {
    fn new(path: &VectorPath, reverse: bool) -> Result<Self> {
        ensure!(
            path.anchors.len() >= 2
                && path.anchors.len() <= crate::paths::MAX_ANCHORS
                && path.anchors.iter().all(|a| a.finite()),
            "Choose a usable guide path."
        );
        let mut flattened = crate::paths::stroke(path);
        if reverse {
            flattened.reverse();
        }
        let mut points = Vec::with_capacity(flattened.len());
        let mut distances = vec![0.];
        let mut length = 0.;
        for point in flattened {
            let Some(previous) = points.last().copied() else {
                points.push(point);
                continue;
            };
            let next = length + (point.0 - previous.0).hypot(point.1 - previous.1);
            if next - length <= 1e-6 {
                continue;
            }
            length = next;
            ensure!(
                length.is_finite() && length <= 1e9,
                "The guide path is too large."
            );
            distances.push(length);
            points.push(point);
        }
        ensure!(length > 1e-6, "The guide path has no length.");
        Ok(Self {
            points,
            distances,
            length,
        })
    }

    fn sample(&self, distance: f64) -> ((f64, f64), (f64, f64)) {
        let distance = distance.clamp(0., self.length);
        let index = self
            .distances
            .partition_point(|value| *value <= distance)
            .clamp(1, self.points.len() - 1);
        let (a, b) = (self.points[index - 1], self.points[index]);
        let length = self.distances[index] - self.distances[index - 1];
        let tangent = ((b.0 - a.0) / length, (b.1 - a.1) / length);
        let along = distance - self.distances[index - 1];
        ((a.0 + tangent.0 * along, a.1 + tangent.1 * along), tangent)
    }
}

pub fn prepare(
    spec: &TextSpec,
    path: &VectorPath,
    options: Options,
    canceled: &AtomicBool,
) -> Result<Prepared> {
    ensure!(
        super::fields_allowed(
            &spec.source,
            &spec.family,
            &spec.family,
            spec.size,
            spec.spacing
        ) && spec.layout.allowed(),
        "The text settings are out of range."
    );
    ensure!(
        !spec.source.contains(['\n', '\r']),
        "Text Along Path supports one line. Remove line breaks and try again."
    );
    ensure!(
        options.offset.is_finite()
            && options.baseline.is_finite()
            && options.offset.abs() <= 1_000_000.
            && options.baseline.abs() <= 1_000_000.,
        "Offsets must be between −1000000 and 1000000 pixels."
    );
    ensure!(
        super::runs::allowed(&spec.source, &spec.runs),
        "Invalid styled text ranges."
    );
    ensure!(
        spec.styles_in(0..spec.source.len())
            .iter()
            .any(|style| style.color[3] != 0),
        "The text is fully transparent."
    );
    let guide = Guide::new(path, options.reverse)?;
    super::with_fonts(|fonts, cache| {
        // Fixed boxes and their clipping do not constrain a guide's single line.
        let mut path_spec = spec.clone();
        path_spec.layout = TextLayout::default();
        let buffer = super::layout(fonts, &path_spec);
        let run = buffer
            .layout_runs()
            .next()
            .context("There is no text to place.")?;
        let width = f64::from(run.line_w);
        ensure!(
            width.is_finite() && width > 0.,
            "There is no text to place."
        );
        let start = options.offset
            + (guide.length - width)
                * match options.align {
                    TextAlign::Left => 0.,
                    TextAlign::Center => 0.5,
                    TextAlign::Right => 1.,
                };
        // A cluster can contain a base and several marks, or a ligature. All its
        // glyphs share one rotation and retain the shaper's relative positions.
        let mut clusters = BTreeMap::<(usize, usize), (f64, f64)>::new();
        for glyph in run.glyphs {
            let left = f64::from(glyph.x);
            let right = left + f64::from(glyph.w);
            let entry = clusters
                .entry((glyph.start, glyph.end))
                .or_insert((left, right));
            entry.0 = entry.0.min(left);
            entry.1 = entry.1.max(right);
        }
        let mut paths = Vec::new();
        // Keep outline storage local to this request. Font/size changes must not
        // grow the process-wide raster cache's optional outline cache forever.
        let mut outlines = HashMap::new();
        let mut overflow = start < -1e-6 || start + width > guide.length + 1e-6;
        let mut commands_count = 0usize;
        for glyph in run.glyphs {
            ensure!(
                !canceled.load(Ordering::Relaxed),
                "Text placement canceled."
            );
            let (left, right) = clusters[&(glyph.start, glyph.end)];
            // Omit complete overflowing clusters; never wrap a closed guide or
            // slice a combining sequence through the end of an open guide.
            if start + left < -1e-6 || start + right > guide.length + 1e-6 {
                overflow = true;
                continue;
            }
            let center = (left + right) * 0.5;
            let (position, tangent) = guide.sample(start + center);
            let mut key = glyph.physical((0., 0.), 1.).cache_key;
            key.flags |= CacheKeyFlags::DISABLE_HINTING;
            let commands = outlines
                .entry(key)
                .or_insert_with(|| cache.get_outline_commands_uncached(fonts, key));
            let Some(commands) = commands.as_deref() else {
                let source = &spec.source[glyph.start..glyph.end];
                ensure!(
                    source.chars().all(char::is_whitespace),
                    "A glyph has no scalable outline. Bitmap-only fonts and emoji are not supported by Text Along Path."
                );
                continue;
            };
            commands_count += commands.len();
            ensure!(
                commands_count <= 1_000_000,
                "The text outlines are too complex."
            );
            let map = |px: f32, py: f32| {
                let x =
                    f64::from(glyph.x + glyph.font_size * glyph.x_offset) + f64::from(px) - center;
                let y = f64::from(glyph.y - glyph.font_size * glyph.y_offset) - f64::from(py)
                    + options.baseline;
                (
                    (position.0 + tangent.0 * x - tangent.1 * y) as f32,
                    (position.1 + tangent.1 * x + tangent.0 * y) as f32,
                )
            };
            let mut builder = PathBuilder::new();
            for command in commands {
                match *command {
                    Command::MoveTo(p) => {
                        let (x, y) = map(p.x, p.y);
                        builder.move_to(x, y);
                    }
                    Command::LineTo(p) => {
                        let (x, y) = map(p.x, p.y);
                        builder.line_to(x, y);
                    }
                    Command::QuadTo(c, p) => {
                        let (cx, cy) = map(c.x, c.y);
                        let (x, y) = map(p.x, p.y);
                        builder.quad_to(cx, cy, x, y);
                    }
                    Command::CurveTo(a, b, p) => {
                        let (ax, ay) = map(a.x, a.y);
                        let (bx, by) = map(b.x, b.y);
                        let (x, y) = map(p.x, p.y);
                        builder.cubic_to(ax, ay, bx, by, x, y);
                    }
                    Command::Close => builder.close(),
                }
            }
            if let Some(path) = builder.finish() {
                paths.push((path, spec.style_at(glyph.start).color));
            }
        }
        ensure!(
            !paths.is_empty(),
            "No letters fit on the guide. Reduce the font size, adjust the offset, or use a longer path."
        );
        let bounds = paths.iter().map(|(path, _)| path.bounds()).fold(
            [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ],
            |b, r| {
                [
                    b[0].min(f64::from(r.left())),
                    b[1].min(f64::from(r.top())),
                    b[2].max(f64::from(r.right())),
                    b[3].max(f64::from(r.bottom())),
                ]
            },
        );
        let left = bounds[0].floor() - 1.;
        let top = bounds[1].floor() - 1.;
        let width = bounds[2].ceil() + 1. - left;
        let height = bounds[3].ceil() + 1. - top;
        ensure!(
            left >= f64::from(i32::MIN)
                && top >= f64::from(i32::MIN)
                && left + width <= f64::from(i32::MAX)
                && top + height <= f64::from(i32::MAX)
                && width >= 1.
                && height >= 1.
                && width <= f64::from(MAX_NEW_EDGE)
                && height <= f64::from(MAX_NEW_EDGE)
                && width * height <= (MAX_IMAGE_BYTES / 4) as f64,
            "The curved text exceeds the layer size limit."
        );
        Ok(Prepared {
            paths,
            offset: (left as i32, top as i32),
            size: (width as u32, height as u32),
            overflow,
        })
    })
}

impl Prepared {
    fn raster(&self, size: (u32, u32), transform: Transform) -> Result<RgbaImage> {
        let mut pixmap = Pixmap::new(size.0, size.1).context("The text bitmap is too large.")?;
        let mut paint = Paint::default();
        for (path, color) in &self.paths {
            paint.set_color_rgba8(color[0], color[1], color[2], color[3]);
            pixmap.fill_path(path, &paint, FillRule::Winding, transform, None);
        }
        let mut data = pixmap.take();
        for pixel in data.as_chunks_mut::<4>().0 {
            let alpha = u32::from(pixel[3]);
            for channel in &mut pixel[..3] {
                *channel = (u32::from(*channel) * 255 + alpha / 2)
                    .checked_div(alpha)
                    .unwrap_or(0)
                    .min(255) as u8;
            }
        }
        RgbaImage::from_raw(size.0, size.1, data).context("Invalid text bitmap.")
    }

    pub fn render(&self) -> Result<RgbaImage> {
        self.raster(
            self.size,
            Transform::from_translate(-(self.offset.0 as f32), -(self.offset.1 as f32)),
        )
    }

    pub fn preview(&self, canvas: (u32, u32), edge: u32) -> Result<RgbaImage> {
        let size = crate::adjustments::preview_size(canvas.0, canvas.1, edge.clamp(1, 1024));
        self.raster(
            size,
            Transform::from_scale(
                size.0 as f32 / canvas.0 as f32,
                size.1 as f32 / canvas.1 as f32,
            ),
        )
    }
}
