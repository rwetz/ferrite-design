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
