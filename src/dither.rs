//! Ordered (Bayer) dithering, the Ferrite texture.
//!
//! A dither field maps every point of an element to a level in `0.0..=1.0`;
//! each cell is inked if its level beats the 4×4 Bayer threshold at that
//! cell. The result is the classic cross-hatched ramp from 1-bit graphics,
//! and it is how Ferrite does what other systems do with gradients, shadows
//! and translucency: there are no gradients in Ferrite, only dither.
//!
//! **Where it goes** (structural accents, not noise):
//! panel-header fills, empty states, progress edges, loading placeholders,
//! large backgrounds behind content-free areas. **Never** behind text a user
//! has to read, and never animated continuously.
//!
//! **Device pixels.** Cells are sized in *physical* pixels and the grid is
//! snapped to the device, so a 1-px dither stays a 1-px dither at 125% and
//! 150% instead of smearing into grey mush.
//!
//! **Cost.** A dither is rasterised once into an image at device resolution
//! and cached by (field, size, scale, colors); every later frame paints that
//! image as a single textured quad. Painting cells as individual quads looks
//! the same but costs tens of thousands of quads per element per frame —
//! enough to peg a core (see PITFALLS §17). Fields are therefore plain data
//! ([`Field`]), not closures, so they can be cache keys.

use gpui::{
    App, Bounds, Hsla, IntoElement, Pixels, RenderOnce, StyleRefinement, Styled, Window, canvas,
};

use crate::raster::{self, bgra};

/// The 4×4 Bayer matrix. Thresholds are `(m + 0.5) / 16`.
pub const BAYER4: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

/// The threshold a level must exceed for cell `(x, y)` to be inked.
pub fn threshold(x: u32, y: u32) -> f32 {
    (BAYER4[(y % 4) as usize][(x % 4) as usize] as f32 + 0.5) / 16.0
}

/// Whether cell `(x, y)` is inked at `level`.
pub fn ink(x: u32, y: u32, level: f32) -> bool {
    level > threshold(x, y)
}

/// The levels Ferrite uses. Pick from these rather than inventing values so
/// textures match across apps; they map to the CP437 shades `░ ▒ ▓`.
pub mod level {
    pub const LIGHT: f32 = 0.25; // ░
    pub const MEDIUM: f32 = 0.5; // ▒
    pub const DARK: f32 = 0.75; // ▓
}

/// What level each point of an element gets. Coordinates are normalised:
/// `u` runs 0→1 left to right, `v` 0→1 top to bottom.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Field {
    /// Uniform level.
    Flat(f32),
    /// Linear ramp left → right.
    Horizontal { from: f32, to: f32 },
    /// Linear ramp top → bottom.
    Vertical { from: f32, to: f32 },
    /// Falloff from the centre (`center`) to the corners (`edge`).
    Radial { center: f32, edge: f32 },
}

impl Field {
    pub fn level(&self, u: f32, v: f32) -> f32 {
        match *self {
            Field::Flat(l) => l,
            Field::Horizontal { from, to } => from + (to - from) * u,
            Field::Vertical { from, to } => from + (to - from) * v,
            Field::Radial { center, edge } => {
                let (dx, dy) = (u - 0.5, v - 0.5);
                let d = ((dx * dx + dy * dy).sqrt() / std::f32::consts::FRAC_1_SQRT_2).min(1.0);
                center + (edge - center) * d
            }
        }
    }

    fn key(&self) -> [u32; 3] {
        match *self {
            Field::Flat(l) => [0, l.to_bits(), 0],
            Field::Horizontal { from, to } => [1, from.to_bits(), to.to_bits()],
            Field::Vertical { from, to } => [2, from.to_bits(), to.to_bits()],
            Field::Radial { center, edge } => [3, center.to_bits(), edge.to_bits()],
        }
    }
}

pub fn flat(level: f32) -> Field {
    Field::Flat(level)
}

pub fn horizontal(from: f32, to: f32) -> Field {
    Field::Horizontal { from, to }
}

pub fn vertical(from: f32, to: f32) -> Field {
    Field::Vertical { from, to }
}

pub fn radial(center: f32, edge: f32) -> Field {
    Field::Radial { center, edge }
}

/// Rasterise `field` into a BGRA buffer `w`×`h` device pixels, with square
/// cells `cell` device pixels on a side. Pure, so it is testable.
pub fn raster(field: Field, w: u32, h: u32, cell: u32, ink_bgra: [u8; 4], paper_bgra: [u8; 4]) -> Vec<u8> {
    let cell = cell.max(1);
    let (cols, rows) = (w.div_ceil(cell), h.div_ceil(cell));
    let mut out = vec![0u8; (w * h * 4) as usize];
    let mut row_mask = vec![false; cols as usize];
    for row in 0..rows {
        let v = ((row as f32 + 0.5) * cell as f32) / h.max(1) as f32;
        for col in 0..cols {
            let u = ((col as f32 + 0.5) * cell as f32) / w.max(1) as f32;
            row_mask[col as usize] = ink(col, row, field.level(u, v));
        }
        let y0 = row * cell;
        for y in y0..(y0 + cell).min(h) {
            let line = &mut out[(y * w * 4) as usize..((y + 1) * w * 4) as usize];
            for (x, px) in line.chunks_exact_mut(4).enumerate() {
                px.copy_from_slice(if row_mask[x / cell as usize] { &ink_bgra } else { &paper_bgra });
            }
        }
    }
    out
}

// ── The element ──────────────────────────────────────────────────────────

/// A dithered fill. Size it like any element (`.size_full()`, `.h(px(16.))`).
#[derive(IntoElement)]
pub struct Dither {
    field: Field,
    ink: Option<Hsla>,
    paper: Option<Hsla>,
    cell: u32,
    style: StyleRefinement,
}

/// Start a dither over `field`. Ink defaults to the palette's `line_strong`.
pub fn dither(field: Field) -> Dither {
    Dither { field, ink: None, paper: None, cell: 2, style: StyleRefinement::default() }
}

impl Dither {
    /// The color of inked cells.
    pub fn ink(mut self, color: Hsla) -> Self {
        self.ink = Some(color);
        self
    }

    /// Fill uninked cells too (default: transparent).
    pub fn paper(mut self, color: Hsla) -> Self {
        self.paper = Some(color);
        self
    }

    /// Cell edge in **device** pixels. 1 = finest, 2 = default, 3–4 = chunky.
    pub fn cell(mut self, device_px: u32) -> Self {
        self.cell = device_px.max(1);
        self
    }
}

impl Styled for Dither {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// Larger than any sane element; guards against a runaway layout asking for
/// a gigapixel texture.
const MAX_SIDE: u32 = 8192;

impl RenderOnce for Dither {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ink_color = self
            .ink
            .unwrap_or_else(|| crate::tokens::hsla(crate::theme::palette(cx).line_strong));
        let ink = bgra(ink_color);
        let paper = self.paper.map(bgra).unwrap_or([0, 0, 0, 0]);
        let field = self.field;
        let cell = self.cell;

        let mut element = canvas(
            |_, _, _| {},
            move |bounds: Bounds<Pixels>, _, window: &mut Window, _cx| {
                let sf = window.scale_factor();
                // Snap to the device grid so image pixels map 1:1 onto
                // screen pixels (no resampling blur).
                let ox = (f32::from(bounds.origin.x) * sf).round();
                let oy = (f32::from(bounds.origin.y) * sf).round();
                let w = ((f32::from(bounds.size.width) * sf).round() as u32).min(MAX_SIDE);
                let h = ((f32::from(bounds.size.height) * sf).round() as u32).min(MAX_SIDE);
                if w == 0 || h == 0 {
                    return;
                }
                let key = raster::Key::Dither { field: field.key(), w, h, cell, ink, paper };
                let image = raster::image(key, window, || (w, h, raster(field, w, h, cell, ink, paper)));
                raster::paint(image, ox / sf, oy / sf, w, h, window);
            },
        );
        *element.style() = self.style;
        element
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_ink_the_expected_fraction() {
        for (level, expected) in [(0.0, 0), (level::LIGHT, 4), (level::MEDIUM, 8), (level::DARK, 12), (1.0, 16)] {
            let n = (0..4).flat_map(|y| (0..4).map(move |x| (x, y))).filter(|&(x, y)| ink(x, y, level)).count();
            assert_eq!(n, expected, "level {level}");
        }
    }

    #[test]
    fn raster_matches_the_cell_rule() {
        let (ink_px, paper_px) = ([1, 2, 3, 255], [0, 0, 0, 0]);
        // 8×8 device px, 2px cells → a 4×4 cell grid at 50%: half inked.
        let buf = raster(flat(level::MEDIUM), 8, 8, 2, ink_px, paper_px);
        let inked = buf.chunks_exact(4).filter(|p| *p == ink_px).count();
        assert_eq!(inked, 32);
        // Every 2×2 block is uniform.
        for cy in 0..4 {
            for cx in 0..4 {
                let at = |x: u32, y: u32| &buf[((y * 8 + x) * 4) as usize..][..4];
                let first = at(cx * 2, cy * 2);
                for (dx, dy) in [(1, 0), (0, 1), (1, 1)] {
                    assert_eq!(at(cx * 2 + dx, cy * 2 + dy), first);
                }
            }
        }
    }

    #[test]
    fn raster_handles_partial_edge_cells() {
        // Sizes that aren't a multiple of the cell must not panic or overrun.
        let buf = raster(horizontal(0.0, 1.0), 7, 5, 3, [9; 4], [0; 4]);
        assert_eq!(buf.len(), 7 * 5 * 4);
    }
}
