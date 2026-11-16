use std::sync::Arc;

use image::{Rgba, RgbaImage};

use crate::document::DocumentContent;

pub const TILE_FINE: u32 = 256;
// GPUI samples atlas textures with a linear filter, so a tile keeps one source
// pixel past each edge. Without it, a seam blends against the neighboring atlas tile.
pub const FILTER_APRON: u32 = 1;
const MAX_FIT_SHIFT: u32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelRect {
    pub x: i64,
    pub y: i64,
    pub width: u32,
    pub height: u32,
}

impl PixelRect {
    pub fn new(x: i64, y: i64, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(self) -> i64 {
        self.x.saturating_add(i64::from(self.width))
    }

    pub fn bottom(self) -> i64 {
        self.y.saturating_add(i64::from(self.height))
    }

    fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub fn overlaps(self, other: Self) -> bool {
        self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }

    fn expand(self, apron: u32) -> Self {
        let pad = i64::from(apron);
        Self {
            x: self.x.saturating_sub(pad),
            y: self.y.saturating_sub(pad),
            width: self.width.saturating_add(apron.saturating_mul(2)),
            height: self.height.saturating_add(apron.saturating_mul(2)),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TileUpdate {
    pub rebuilt: u32,
    pub retained: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dirty {
    None,
    // The document changed, but its pixels did not.
    Unpainted,
    // Layer metadata changed the composite, but the layer bitmaps did not change.
    Composite(PixelRect),
    Region(PixelRect),
    Regions(Vec<PixelRect>),
    Canvas,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileSpan {
    pub index: usize,
    pub content: PixelRect,
    pub stored: PixelRect,
}

// Coarsest power-of-two reduction that still covers the viewport.
// 8192 canvas pixels in a 1600 pixel window uses shift 2 (2048 source pixels).
pub fn fit_level_shift(canvas_edge: u32, viewport_edge: u32) -> u32 {
    if viewport_edge == 0 || canvas_edge <= viewport_edge {
        return 0;
    }
    let mut shift = 0;
    while shift < MAX_FIT_SHIFT && (canvas_edge >> (shift + 1)) >= viewport_edge {
        shift += 1;
    }
    shift
}

#[derive(Clone)]
pub struct TileCache {
    tile: u32,
    apron: u32,
    shift: u32,
    canvas_width: u32,
    canvas_height: u32,
    cols: u32,
    rows: u32,
    tiles: Vec<Option<Arc<RgbaImage>>>,
}

impl TileCache {
    pub fn new(width: u32, height: u32, tile: u32, apron: u32) -> Self {
        Self::leveled(width, height, tile, apron, 0)
    }

    // `shift` is the Fit reduction: each level pixel covers `1 << shift` canvas pixels.
    pub fn leveled(width: u32, height: u32, tile: u32, apron: u32, shift: u32) -> Self {
        let shift = shift.min(MAX_FIT_SHIFT);
        let scale = 1u32 << shift;
        let tile = tile.max(1);
        let cols = width.div_ceil(scale).max(1).div_ceil(tile).max(1);
        let rows = height.div_ceil(scale).max(1).div_ceil(tile).max(1);
        Self {
            tile,
            apron,
            shift,
            canvas_width: width,
            canvas_height: height,
            cols,
            rows,
            tiles: vec![None; (cols as usize).saturating_mul(rows as usize)],
        }
    }

    pub fn fill_from_composite(
        &mut self,
        pixels: &RgbaImage,
        check: &impl Fn() -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            pixels.dimensions() == (self.canvas_width, self.canvas_height),
            "Invalid prepared tile source"
        );
        for index in 0..self.tiles.len() {
            check()?;
            let rect = self.coverage(index as u32 % self.cols, index as u32 / self.cols);
            let region = image::imageops::crop_imm(
                pixels,
                rect.x as u32,
                rect.y as u32,
                rect.width,
                rect.height,
            )
            .to_image();
            self.tiles[index] = Some(Arc::new(if self.shift == 0 {
                region
            } else {
                box_downsample(&region, self.shift)
            }));
        }
        check()
    }

    pub fn shift(&self) -> u32 {
        self.shift
    }

    pub fn spans(&self) -> impl Iterator<Item = TileSpan> + '_ {
        (0..self.rows).flat_map(move |row| (0..self.cols).map(move |column| self.span(column, row)))
    }

    pub fn slot(&self, index: usize) -> Option<&Arc<RgbaImage>> {
        self.tiles.get(index).and_then(|tile| tile.as_ref())
    }

    // Rebuilds tiles whose filter coverage meets `dirty`. Untouched tiles keep their image.
    pub fn rebuild(&mut self, content: &DocumentContent, dirty: PixelRect) -> TileUpdate {
        self.rebuild_regions(content, &[dirty])
    }

    pub fn rebuild_regions(
        &mut self,
        content: &DocumentContent,
        dirty: &[PixelRect],
    ) -> TileUpdate {
        let stride = i64::from(self.tile) * i64::from(self.scale());
        let apron = self.apron.saturating_mul(self.scale());
        let mut affected = vec![false; self.tiles.len()];
        for &dirty in dirty.iter().filter(|rect| !rect.is_empty()) {
            let rect = clip_to_canvas(dirty.expand(apron), self.canvas_width, self.canvas_height);
            if rect.is_empty() {
                continue;
            }
            let columns = rect.x / stride..=(rect.right() - 1) / stride;
            for row in rect.y / stride..=(rect.bottom() - 1) / stride {
                let first = (row * i64::from(self.cols) + columns.start()) as usize;
                let last = (row * i64::from(self.cols) + columns.end()) as usize;
                affected[first..=last].fill(true);
            }
        }
        let mut rebuilt = 0;
        for (index, _) in affected
            .iter()
            .enumerate()
            .filter(|(_, affected)| **affected)
        {
            let column = index as u32 % self.cols;
            let row = index as u32 / self.cols;
            let rect = self.coverage(column, row);
            let image = content.composite_rect(rect.x, rect.y, rect.width, rect.height);
            self.tiles[index] = Some(Arc::new(box_downsample(&image, self.shift)));
            rebuilt += 1;
        }
        TileUpdate {
            rebuilt,
            retained: self.stored() - rebuilt,
        }
    }

    fn stored(&self) -> u32 {
        self.tiles.iter().filter(|tile| tile.is_some()).count() as u32
    }

    fn span(&self, column: u32, row: u32) -> TileSpan {
        TileSpan {
            index: (row * self.cols + column) as usize,
            content: self.content_rect(column, row),
            stored: self.stored_rect(column, row),
        }
    }

    fn scale(&self) -> u32 {
        1u32 << self.shift
    }

    fn content_rect(&self, column: u32, row: u32) -> PixelRect {
        let scale = u64::from(self.scale());
        let origin_x = u64::from(column) * u64::from(self.tile) * scale;
        let origin_y = u64::from(row) * u64::from(self.tile) * scale;
        let width = (u64::from(self.tile) * scale)
            .min(u64::from(self.canvas_width).saturating_sub(origin_x));
        let height = (u64::from(self.tile) * scale)
            .min(u64::from(self.canvas_height).saturating_sub(origin_y));
        PixelRect::new(
            origin_x as i64,
            origin_y as i64,
            width as u32,
            height as u32,
        )
    }

    fn coverage(&self, column: u32, row: u32) -> PixelRect {
        clip_to_canvas(
            self.content_rect(column, row)
                .expand(self.apron.saturating_mul(self.scale())),
            self.canvas_width,
            self.canvas_height,
        )
    }

    fn stored_rect(&self, column: u32, row: u32) -> PixelRect {
        let source = self.coverage(column, row);
        let scale = self.scale();
        // A partial block at the canvas edge still occupies one texel in the image.
        // Keep its drawn pitch consistent with the samples shared by adjacent tiles.
        PixelRect::new(
            source.x,
            source.y,
            source.width.div_ceil(scale) * scale,
            source.height.div_ceil(scale) * scale,
        )
    }
}

fn clip_to_canvas(rect: PixelRect, width: u32, height: u32) -> PixelRect {
    let right = rect.right().clamp(0, i64::from(width));
    let bottom = rect.bottom().clamp(0, i64::from(height));
    let x = rect.x.clamp(0, i64::from(width));
    let y = rect.y.clamp(0, i64::from(height));
    PixelRect::new(x, y, (right - x) as u32, (bottom - y) as u32)
}

// Averages each (1<<shift) square. Used for the Fit level chosen by `fit_level_shift`.
pub fn box_downsample(source: &RgbaImage, shift: u32) -> RgbaImage {
    if shift == 0 {
        return source.clone();
    }
    let factor = 1u32 << shift;
    let width = source.width().div_ceil(factor);
    let height = source.height().div_ceil(factor);
    RgbaImage::from_fn(width, height, |x, y| {
        let left = x * factor;
        let top = y * factor;
        let right = (left + factor).min(source.width());
        let bottom = (top + factor).min(source.height());
        let count = u64::from(right - left) * u64::from(bottom - top);
        let mut rgb = [0u64; 3];
        let mut alpha = 0u64;
        for sy in top..bottom {
            for sx in left..right {
                let [r, g, b, a] = source.get_pixel(sx, sy).0;
                let a = u64::from(a);
                alpha += a;
                for (sum, channel) in rgb.iter_mut().zip([r, g, b]) {
                    *sum += u64::from(channel) * a;
                }
            }
        }
        let [r, g, b] = rgb.map(|sum| (sum / alpha.max(1)) as u8);
        Rgba([r, g, b, (alpha / count) as u8])
    })
}
