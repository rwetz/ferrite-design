//! The Ferrite palette, type scale and spacing grid: the single source of truth.
//!
//! Every color an app paints comes from a [`Palette`]. Components never name
//! a hex value; they ask the palette for a role (`bg`, `line`, `accent`, …).
//! `theme.rs` picks which palette is on screen and every component reads it
//! through `palette(cx)`, so a value changed here recolors everything.
//!
//! The rules:
//! - **One accent.** Phosphor amber carries all identity. It marks the primary
//!   action, the active/selected state, focus, the caret and live progress.
//!   Never decoration, never a second "brand" hue beside it.
//! - **Neutrals are near-pure greys** with a hair of warmth in the whites, so
//!   the amber reads hot against cold iron rather than muddy.
//! - **Status colors are semantic only** (danger, success, warning), never
//!   used to decorate.

use gpui::{Hsla, Pixels, px};

/// Which of the two shipped palettes a value belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Dark,
    Light,
}

/// Colors as `0xRRGGBB`. Use [`hsla`] (or the `Palette` accessors) to paint.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub tone: Tone,
    pub name: &'static str,

    // ── Iron: the neutral ramp, darkest to lightest in dark mode ──────────
    /// Wells, inputs, terminal/log bodies — sits *below* the page.
    pub sunken: u32,
    /// The page.
    pub bg: u32,
    /// Chrome: title bar, status bar, panel headers, sidebars.
    pub surface: u32,
    /// Floating and hovered: popovers, menus, hovered rows, default buttons.
    pub raised: u32,
    /// Hairlines and borders.
    pub line: u32,
    /// Emphasised borders: inputs, the window frame, pressed states.
    pub line_strong: u32,

    // ── Ink ────────────────────────────────────────────────────────────────
    pub fg: u32,
    /// Secondary text. Must still pass WCAG AA on `bg`.
    pub fg_dim: u32,
    /// Disabled text, separators drawn as glyphs, dither "off" ink. Not for
    /// anything a user has to read.
    pub fg_faint: u32,

    // ── Phosphor: the one accent ─────────────────────────────────────────
    /// Accent *fill* (primary buttons, progress, selection marker).
    pub accent: u32,
    pub accent_hover: u32,
    pub accent_active: u32,
    /// Text drawn *on* an accent fill.
    pub accent_fg: u32,
    /// Accent used *as* text on `bg` (links, the active tab label). In dark
    /// mode this is the fill itself; on paper it has to be darker to stay
    /// legible.
    pub accent_text: u32,
    /// A low-energy accent tint for selected-row backgrounds.
    pub accent_dim: u32,

    // ── Status (semantic only) ───────────────────────────────────────────
    pub danger: u32,
    pub danger_fg: u32,
    pub success: u32,
    pub warning: u32,
}

/// Dark — "iron". The identity; the default.
pub const IRON: Palette = Palette {
    tone: Tone::Dark,
    name: "Ferrite Iron",
    sunken: 0x070708,
    bg: 0x0B0B0C,
    surface: 0x111113,
    raised: 0x18181B,
    line: 0x2A2A2E,
    line_strong: 0x3D3D42,
    fg: 0xE8E8E6,
    fg_dim: 0x8E8E92,
    fg_faint: 0x55555A,
    accent: 0xF2A93B,
    accent_hover: 0xFFBA57,
    accent_active: 0xD99021,
    accent_fg: 0x0B0B0C,
    accent_text: 0xF2A93B,
    accent_dim: 0x3A2A12,
    danger: 0xF0503C,
    danger_fg: 0x0B0B0C,
    success: 0x9BD37E,
    warning: 0xF2D23B,
};

/// Light — "paper". A printout, not a white web page: warm off-white stock,
/// ink-black type, the same dither.
pub const PAPER: Palette = Palette {
    tone: Tone::Light,
    name: "Ferrite Paper",
    sunken: 0xE2DFD8,
    bg: 0xEDEBE6,
    surface: 0xE6E3DC,
    raised: 0xF6F4F0,
    line: 0xC7C3BA,
    line_strong: 0xA9A49A,
    fg: 0x151515,
    fg_dim: 0x55534E,
    fg_faint: 0x9A968E,
    accent: 0xE39A22,
    accent_hover: 0xF0A936,
    accent_active: 0xC9841A,
    accent_fg: 0x151515,
    accent_text: 0x8A5000,
    accent_dim: 0xF1DDB6,
    danger: 0xC0301D,
    danger_fg: 0xF6F4F0,
    success: 0x3B7526,
    warning: 0x735A00,
};

impl Palette {
    pub fn for_tone(tone: Tone) -> &'static Palette {
        match tone {
            Tone::Dark => &IRON,
            Tone::Light => &PAPER,
        }
    }

    pub fn is_dark(&self) -> bool {
        self.tone == Tone::Dark
    }
}

/// `0xRRGGBB` → gpui color.
pub fn hsla(hex: u32) -> Hsla {
    gpui::rgb(hex).into()
}

/// `0xRRGGBB` + alpha (0.0–1.0) → gpui color.
pub fn hsla_a(hex: u32, alpha: f32) -> Hsla {
    hsla(hex).opacity(alpha)
}

/// `0xRRGGBB` → `"#rrggbb"`, for exporting tokens to CSS or docs.
pub fn css(hex: u32) -> String {
    format!("#{hex:06x}")
}

/// `0xRRGGBB` + alpha byte → `"#rrggbbaa"`.
pub fn css_a(hex: u32, alpha: u8) -> String {
    format!("#{hex:06x}{alpha:02x}")
}

// ── Type ────────────────────────────────────────────────────────────────────
//
// Two faces, strictly split (see fonts.rs):
// - DISPLAY: the IBM VGA 8×16 pixel face, only at whole multiples of its
//   16px cell, and pixel-snapped to the device (fonts::display_size).
// - BODY: JetBrains Mono, at any size on this scale.

/// Body text sizes (JetBrains Mono). 13 is the default UI size.
pub mod text {
    use super::*;
    pub const XS: Pixels = px(11.);
    pub const SM: Pixels = px(12.);
    pub const BASE: Pixels = px(13.);
    pub const LG: Pixels = px(15.);
}

// ── Space ───────────────────────────────────────────────────────────────────
//
// The grid is the display font's character cell: 8px wide, 16px tall. Every
// gap and padding is a multiple of 4 (half a cell), and anything that
// carries display type is a multiple of 16 tall.

pub mod space {
    use super::*;
    pub const HALF: Pixels = px(4.);
    pub const CELL: Pixels = px(8.);
    pub const ROW: Pixels = px(16.);
    pub const X2: Pixels = px(24.);
    pub const X3: Pixels = px(32.);
}

/// Hard corners everywhere. This is a rule, not a default: there is no
/// "slightly rounded" variant.
pub const RADIUS: Pixels = px(0.);

#[cfg(test)]
mod tests {
    //! Tripwires. If a palette edit breaks legibility, these fail before a
    //! user ever sees it.
    use super::*;

    fn luminance(hex: u32) -> f64 {
        let ch = |shift: u32| {
            let c = ((hex >> shift) & 0xFF) as f64 / 255.0;
            if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * ch(16) + 0.7152 * ch(8) + 0.0722 * ch(0)
    }

    fn contrast(a: u32, b: u32) -> f64 {
        let (la, lb) = (luminance(a), luminance(b));
        let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
        (hi + 0.05) / (lo + 0.05)
    }

    fn check(p: &Palette) {
        let n = p.name;
        // Body text: AAA on every surface it sits on.
        for (bg, label) in [(p.bg, "bg"), (p.surface, "surface"), (p.raised, "raised"), (p.sunken, "sunken")] {
            assert!(contrast(p.fg, bg) >= 7.0, "{n}: fg on {label} = {:.2}", contrast(p.fg, bg));
            assert!(contrast(p.fg_dim, bg) >= 4.5, "{n}: fg_dim on {label} = {:.2}", contrast(p.fg_dim, bg));
        }
        assert!(contrast(p.accent_text, p.bg) >= 4.5, "{n}: accent_text on bg = {:.2}", contrast(p.accent_text, p.bg));
        assert!(contrast(p.accent_fg, p.accent) >= 4.5, "{n}: accent_fg on accent = {:.2}", contrast(p.accent_fg, p.accent));
        assert!(contrast(p.accent_fg, p.accent_hover) >= 4.5, "{n}: accent_fg on hover");
        assert!(contrast(p.danger_fg, p.danger) >= 4.5, "{n}: danger_fg on danger = {:.2}", contrast(p.danger_fg, p.danger));
        assert!(contrast(p.fg, p.accent_dim) >= 7.0, "{n}: fg on accent_dim (selected row)");
        for (c, label) in [(p.danger, "danger"), (p.success, "success"), (p.warning, "warning")] {
            assert!(contrast(c, p.bg) >= 4.5, "{n}: {label} as text on bg = {:.2}", contrast(c, p.bg));
            // theme.rs also uses them as button fills with `bg` as the label.
            assert!(contrast(p.bg, c) >= 4.5, "{n}: bg on {label} fill = {:.2}", contrast(p.bg, c));
        }
        // Hairlines must be visible but quiet.
        let l = contrast(p.line, p.bg);
        assert!((1.2..3.0).contains(&l), "{n}: line on bg = {l:.2}");
    }

    #[test]
    fn iron_is_legible() {
        check(&IRON);
    }

    #[test]
    fn paper_is_legible() {
        check(&PAPER);
    }

    #[test]
    fn accent_differs_from_nexis_coral() {
        // Nexis's brand is coral ≈ #F27A5E. Ferrite must not drift toward it.
        let coral = 0xF27A5Eu32;
        let hue_gap = |a: u32| {
            let h = hsla(a).h * 360.;
            let c = hsla(coral).h * 360.;
            (h - c).abs().min(360. - (h - c).abs())
        };
        assert!(hue_gap(IRON.accent) > 20., "accent hue too close to Nexis coral");
    }
}
