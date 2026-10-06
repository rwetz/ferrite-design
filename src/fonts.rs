//! The two faces, embedded, and the pixel-snapping that keeps the display face
//! crisp.
//!
//! **DISPLAY: PxPlus IBM VGA 8x16** (VileR, CC BY-SA 4.0). The DOS text-mode
//! font. It is a *pixel* font drawn as outlines: every glyph is built from
//! square 1/16-em pixels, so it is only sharp when one font pixel lands on a
//! whole number of device pixels. At 16px on a 125% display it lands on 1.25
//! device pixels and smears. Hence the rules:
//!
//! 1. Display type is only set through [`display_size`], which picks the
//!    nearest size whose *physical* height is a whole multiple of 16.
//! 2. Display type is for titles, headers, labels, status, numbers and ASCII
//!    art — short strings. Never paragraphs, never input text.
//! 3. Its coverage is CP437 + WGL4: box drawing (`─│┌┐└┘├┤┬┴┼═║`), shades
//!    (`░▒▓█▀▄▌▐`), arrows, Latin/Greek/Cyrillic. It has **no** eighth blocks
//!    (`▁▂▃▅▆▇`) or quadrants (`▖▘▝▗`); set those in BODY.
//!
//! **BODY / MONO: JetBrains Mono** (OFL-1.1). Everything a user reads at
//! length or types: body copy, inputs, lists, code, logs. Any size on the
//! `tokens::text` scale.

use std::borrow::Cow;

use gpui::{App, Pixels, SharedString, Styled, Window, px};

pub const DISPLAY: &str = "PxPlus IBM VGA 8x16";
pub const BODY: &str = "JetBrains Mono";
pub const MONO: &str = BODY;

/// The display font's native character cell, in font pixels.
pub const DISPLAY_CELL_W: f32 = 8.;
pub const DISPLAY_CELL_H: f32 = 16.;

static PXPLUS_VGA: &[u8] = include_bytes!("../assets/fonts/PxPlus_IBM_VGA_8x16.ttf");
static JBM_REGULAR: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");
static JBM_BOLD: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf");

/// Register the embedded faces with gpui's text system. Called by
/// [`crate::init`]; apps never ship or install font files themselves.
pub fn register(cx: &App) -> anyhow::Result<()> {
    cx.text_system().add_fonts(vec![
        Cow::Borrowed(PXPLUS_VGA),
        Cow::Borrowed(JBM_REGULAR),
        Cow::Borrowed(JBM_BOLD),
    ])
}

/// Whether the display face has a glyph for `c`. Anything it lacks is drawn
/// by gpui in a fallback system font — wrong weight, size and baseline.
/// Use an [`crate::Icon`] instead of an out-of-coverage symbol.
pub fn display_has(c: char) -> bool {
    use std::sync::OnceLock;
    static COVERAGE: OnceLock<Vec<(u32, u32, i32, u32)>> = OnceLock::new();
    let segments = COVERAGE.get_or_init(|| cmap4_segments(PXPLUS_VGA).unwrap_or_default());
    let code = c as u32;
    segments.iter().any(|&(start, end, delta, range_offset)| {
        // Format-4 segments with an idRangeOffset map through a glyph array;
        // PxPlus uses plain deltas, and any segment that covers the code
        // point and isn't the 0xFFFF sentinel maps to a real glyph.
        start <= code && code <= end && code != 0xFFFF && (range_offset != 0 || (code as i32 + delta) & 0xFFFF != 0)
    })
}

/// Parse the (start, end, idDelta, idRangeOffset) segments of a TrueType
/// `cmap` format-4 subtable — just enough to answer "is this code point
/// covered".
fn cmap4_segments(font: &[u8]) -> Option<Vec<(u32, u32, i32, u32)>> {
    let u16_at = |o: usize| -> Option<u32> { Some(u16::from_be_bytes([*font.get(o)?, *font.get(o + 1)?]) as u32) };
    let u32_at = |o: usize| -> Option<usize> {
        Some(u32::from_be_bytes([*font.get(o)?, *font.get(o + 1)?, *font.get(o + 2)?, *font.get(o + 3)?]) as usize)
    };
    let tables = u16_at(4)? as usize;
    let cmap = (0..tables).find_map(|i| {
        let rec = 12 + 16 * i;
        (font.get(rec..rec + 4)? == b"cmap").then(|| u32_at(rec + 8)).flatten()
    })?;
    let subtables = u16_at(cmap + 2)? as usize;
    let sub = (0..subtables).find_map(|i| {
        let rec = cmap + 4 + 8 * i;
        let (platform, encoding) = (u16_at(rec)?, u16_at(rec + 2)?);
        let off = cmap + u32_at(rec + 4)?;
        (u16_at(off)? == 4 && (platform == 3 && encoding == 1 || platform == 0)).then_some(off)
    })?;
    let seg_count = (u16_at(sub + 6)? / 2) as usize;
    let ends = sub + 14;
    let starts = ends + 2 * seg_count + 2;
    let deltas = starts + 2 * seg_count;
    let offsets = deltas + 2 * seg_count;
    (0..seg_count)
        .map(|i| {
            Some((
                u16_at(starts + 2 * i)?,
                u16_at(ends + 2 * i)?,
                u16_at(deltas + 2 * i)? as u16 as i16 as i32,
                u16_at(offsets + 2 * i)?,
            ))
        })
        .collect()
}

/// How big a piece of display type is, in cells: 1× = 16px, 2× = 32px, …
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scale {
    X1 = 1,
    X2 = 2,
    X3 = 3,
}

/// The logical size to set display type at so its physical height is an
/// exact multiple of the 16px cell on *this* window's display.
///
/// At 100% this is 16/32/48. At 150%, `X1` becomes 21.33 logical px = 32
/// physical (2 font px per font pixel) rather than 24 physical (1.5, blurry).
/// At 125%, `X1` stays 16 physical = 12.8 logical: smaller than requested, but
/// crisp. Crisp wins; that is the point of the face.
pub fn display_size(scale: Scale, window: &Window) -> Pixels {
    let sf = window.scale_factor();
    let multiple = (scale as u32 as f32 * sf).round().max(1.0);
    px(DISPLAY_CELL_H * multiple / sf)
}

/// Styling helpers for the two faces.
pub trait FerriteText: Styled + Sized {
    /// Pixel display type, snapped to the device. Line height equals the
    /// size: the 8×16 cell already contains its own leading.
    fn display(self, scale: Scale, window: &Window) -> Self {
        let size = display_size(scale, window);
        self.font_family(SharedString::from(DISPLAY))
            .text_size(size)
            .line_height(size)
    }

    /// Body type at a size from `tokens::text`.
    fn body(self, size: Pixels) -> Self {
        self.font_family(SharedString::from(BODY)).text_size(size)
    }
}

impl<T: Styled + Sized> FerriteText for T {}

#[cfg(test)]
mod tests {
    use super::display_has;

    #[test]
    fn coverage_parser_sees_the_basics() {
        for c in "AZaz09[]()".chars() {
            assert!(display_has(c), "{c:?} should be covered");
        }
        // Known holes: these must be icons, not text.
        for c in ['↻', '▶', '▸', '⚙'] {
            assert!(!display_has(c), "{c:?} unexpectedly covered");
        }
    }

    #[test]
    fn every_glyph_ferrite_sets_in_the_display_face_exists() {
        // Every character Ferrite's own components render in DISPLAY. If you
        // add one, add it here; if this fails, use an Icon instead.
        let used = concat!(
            "[ ]x-•()",          // checkbox / radio marks
            "█",                 // cursor
            "▓▒░",               // title-bar mark
            "│─",                // status bar, rules
            "_□▫x",              // window controls
            "|/-\\",           // spinner
            " ░▒▓█",             // ascii::RAMP
            "─│┌┐└┘═║",          // ascii::boxes
            "·",                 // ascii::bar
            "»×■●↑",             // glyphs used in the examples
        );
        let missing: String = used.chars().filter(|c| !display_has(*c)).collect();
        assert!(missing.is_empty(), "display face lacks: {missing:?}");
    }
}
