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
//! 150% instead of smearing into grey mush. This is the dither's equivalent
//! of the display font's pixel-snapping.

use std::rc::Rc;

use gpui::{
    App, Bounds, Hsla, IntoElement, Pixels, RenderOnce, StyleRefinement, Styled, Window, canvas,
    fill, point, px, size,
};

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

/// A level for every point: `(x, y, width, height)` in logical px → `0..=1`.
pub type Field = Rc<dyn Fn(f32, f32, f32, f32) -> f32>;

/// Uniform level.
pub fn flat(level: f32) -> Field {
    Rc::new(move |_, _, _, _| level)
}

/// Linear ramp left → right.
pub fn horizontal(from: f32, to: f32) -> Field {
    Rc::new(move |x, _, w, _| from + (to - from) * (x / w.max(1.0)))
}

/// Linear ramp top → bottom.
pub fn vertical(from: f32, to: f32) -> Field {
    Rc::new(move |_, y, _, h| from + (to - from) * (y / h.max(1.0)))
}

/// Radial falloff from the centre (`center` level) to the corners (`edge`).
pub fn radial(center: f32, edge: f32) -> Field {
    Rc::new(move |x, y, w, h| {
        let (dx, dy) = (x / w.max(1.0) - 0.5, y / h.max(1.0) - 0.5);
        let d = ((dx * dx + dy * dy).sqrt() / std::f32::consts::FRAC_1_SQRT_2).min(1.0);
        center + (edge - center) * d
    })
}

/// A progress fill: solid up to `value`, then a dithered falloff `ramp` px
/// wide — the ▓▒░ leading edge.
pub fn progress(value: f32, ramp: Pixels) -> Field {
    let ramp = f32::from(ramp);
    Rc::new(move |x, _, w, _| {
        let edge = value.clamp(0.0, 1.0) * w;
        if x <= edge - ramp {
            1.0
        } else if x >= edge {
            0.0
        } else {
            (edge - x) / ramp
        }
    })
}

/// Hard cap on painted quads per element, so a careless full-window dither at
/// 1px cells can't stall a frame. Past it, cells grow until it fits.
const MAX_CELLS: f32 = 160_000.0;

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

impl RenderOnce for Dither {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ink_color = self.ink.unwrap_or_else(|| crate::tokens::hsla(crate::theme::palette(cx).line_strong));
        let paper = self.paper;
        let field = self.field;
        let cell_device = self.cell;

        let mut element = canvas(
            |_, _, _| {},
            move |bounds: Bounds<Pixels>, _, window: &mut Window, _cx| {
                let sf = window.scale_factor();
                let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
                if w <= 0.0 || h <= 0.0 {
                    return;
                }
                if let Some(paper) = paper {
                    window.paint_quad(fill(bounds, paper));
                }

                // Cell size in logical px, grown if the element is huge.
                let mut cell = cell_device as f32 / sf;
                let cells = (w / cell) * (h / cell);
                if cells > MAX_CELLS {
                    cell *= (cells / MAX_CELLS).sqrt().ceil();
                }

                // Snap the grid origin to a device pixel so cells align.
                let ox = (f32::from(bounds.origin.x) * sf).round() / sf;
                let oy = (f32::from(bounds.origin.y) * sf).round() / sf;
                let cols = (w / cell).ceil() as u32;
                let rows = (h / cell).ceil() as u32;
                let right = ox + w;
                let bottom = oy + h;

                window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
                    for row in 0..rows {
                        let y = oy + row as f32 * cell;
                        let cy = (row as f32 + 0.5) * cell;
                        let row_h = cell.min(bottom - y);
                        // Merge horizontal runs of inked cells into one quad.
                        let mut run_start: Option<u32> = None;
                        for col in 0..=cols {
                            let on = col < cols && {
                                let cx_ = (col as f32 + 0.5) * cell;
                                ink(col, row, field(cx_, cy, w, h))
                            };
                            match (on, run_start) {
                                (true, None) => run_start = Some(col),
                                (false, Some(start)) => {
                                    let x0 = ox + start as f32 * cell;
                                    let x1 = (ox + col as f32 * cell).min(right);
                                    window.paint_quad(fill(
                                        Bounds::new(point(px(x0), px(y)), size(px(x1 - x0), px(row_h))),
                                        ink_color,
                                    ));
                                    run_start = None;
                                }
                                _ => {}
                            }
                        }
                    }
                });
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
    fn progress_field_is_solid_then_falls_off() {
        let f = progress(0.5, px(10.));
        assert_eq!(f(10.0, 0.0, 100.0, 4.0), 1.0);
        assert!(f(45.0, 0.0, 100.0, 4.0) > 0.0 && f(45.0, 0.0, 100.0, 4.0) < 1.0);
        assert_eq!(f(60.0, 0.0, 100.0, 4.0), 0.0);
    }
}
