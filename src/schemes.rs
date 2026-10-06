//! Color schemes: named pairs of palettes (a dark one and a light one) that
//! an app can switch between at runtime with [`crate::theme::set_scheme`].
//!
//! **Ferrite** (Iron / Paper) is the signature and the default. The others
//! keep every rule of the language and change only the colors:
//!
//! - **One accent per scheme.** A scheme that pairs two hues (Harbor's
//!   orange and blue) puts the second one in the *neutral ramp*, never in a
//!   second accent — "active" always means exactly one color.
//! - **Same lightness ladder.** Every scheme's `sunken → bg → surface →
//!   raised → line` steps follow Iron's and Paper's (measured in OKLCH), so
//!   depth reads the same in all of them.
//! - **Same tripwires.** `tokens::tests` checks every palette here against
//!   the contrast rules Iron and Paper pass. A scheme that fails doesn't build.
//!
//! The neutral schemes (Mono, Graphite, Slate, Concrete) are for apps that
//! want less personality; the wild ones (Harbor, Cyanotype, Phosphor,
//! Verdigris, Bruise) are for apps that want a different one. The palettes
//! were generated from a small OKLCH model (neutral hue and chroma, one
//! accent) and contrast-corrected; tweak values by hand, then `cargo test`.

use crate::tokens::{IRON, PAPER, Palette, Tone};

/// Which shelf a scheme sits on in a picker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemeKind {
    /// Ferrite's own amber on iron.
    Signature,
    /// Black, white and greys, with a restrained accent.
    Neutral,
    /// A different personality: tinted neutrals and a bolder accent.
    Wild,
}

/// A named pair of palettes. The appearance preference (dark, light,
/// system) picks which one is on screen.
#[derive(Debug, Clone, Copy)]
pub struct Scheme {
    /// Stable identifier, for settings files and `FERRITE_SCHEME`.
    pub key: &'static str,
    pub name: &'static str,
    pub kind: SchemeKind,
    /// One line for a picker.
    pub about: &'static str,
    pub dark: &'static Palette,
    pub light: &'static Palette,
}

impl Scheme {
    pub fn palette(&self, tone: Tone) -> &'static Palette {
        match tone {
            Tone::Dark => self.dark,
            Tone::Light => self.light,
        }
    }
}

/// The signature scheme: phosphor amber on iron grey, and its paper.
pub const FERRITE: Scheme = Scheme {
    key: "ferrite",
    name: "Ferrite",
    kind: SchemeKind::Signature,
    about: "phosphor amber on iron grey; the default",
    dark: &IRON,
    light: &PAPER,
};

/// Every shipped scheme, signature first, then neutral, then wild.
pub const SCHEMES: &[Scheme] = &[
    FERRITE,
    Scheme { key: "mono", name: "Mono", kind: SchemeKind::Neutral, about: "pure black and white; the accent is inverse video", dark: &MONO_DARK, light: &MONO_LIGHT },
    Scheme { key: "graphite", name: "Graphite", kind: SchemeKind::Neutral, about: "soft mid-greys, a silver accent; easy on the eyes for long sessions", dark: &GRAPHITE_DARK, light: &GRAPHITE_LIGHT },
    Scheme { key: "slate", name: "Slate", kind: SchemeKind::Neutral, about: "cool blue-grey neutrals, pale steel accent", dark: &SLATE_DARK, light: &SLATE_LIGHT },
    Scheme { key: "concrete", name: "Concrete", kind: SchemeKind::Neutral, about: "warm greys, a bone accent; the quietest scheme", dark: &CONCRETE_DARK, light: &CONCRETE_LIGHT },
    Scheme { key: "harbor", name: "Harbor", kind: SchemeKind::Wild, about: "muted orange on blue-slate iron: the orange/blue pairing, one accent", dark: &HARBOR_DARK, light: &HARBOR_LIGHT },
    Scheme { key: "cyanotype", name: "Cyanotype", kind: SchemeKind::Wild, about: "blueprint: deep prussian blue with a chalk-yellow accent", dark: &CYANOTYPE_DARK, light: &CYANOTYPE_LIGHT },
    Scheme { key: "phosphor", name: "Phosphor", kind: SchemeKind::Wild, about: "P1 green CRT, Ferrite's amber's older sibling", dark: &PHOSPHOR_DARK, light: &PHOSPHOR_LIGHT },
    Scheme { key: "verdigris", name: "Verdigris", kind: SchemeKind::Wild, about: "oxidised copper: brown iron with a patina-teal accent", dark: &VERDIGRIS_DARK, light: &VERDIGRIS_LIGHT },
    Scheme { key: "bruise", name: "Bruise", kind: SchemeKind::Wild, about: "aubergine greys with an acid-lime accent; the loud one", dark: &BRUISE_DARK, light: &BRUISE_LIGHT },
];

/// A scheme by its key (`"harbor"`), e.g. from a settings file.
pub fn by_key(key: &str) -> Option<&'static Scheme> {
    SCHEMES.iter().find(|s| s.key.eq_ignore_ascii_case(key))
}

// ── The palettes ────────────────────────────────────────────────────────────

pub const MONO_DARK: Palette = Palette {
    tone: Tone::Dark,
    name: "Mono Dark",
    sunken: 0x000000,
    bg: 0x000000,
    surface: 0x0F0F0F,
    raised: 0x1B1B1B,
    line: 0x383838,
    line_strong: 0x585858,
    fg: 0xFFFFFF,
    fg_dim: 0xA8A8A8,
    fg_faint: 0x696969,
    accent: 0xFFFFFF,
    accent_hover: 0xEBEBEB,
    accent_active: 0xD7D7D7,
    accent_fg: 0x000000,
    accent_text: 0xFFFFFF,
    accent_dim: 0x2E2E2E,
    danger: 0xF0584B,
    danger_fg: 0x000000,
    success: 0x90D281,
    warning: 0xE8D34F,
};

pub const MONO_LIGHT: Palette = Palette {
    tone: Tone::Light,
    name: "Mono Light",
    sunken: 0xEEEEEE,
    bg: 0xFFFFFF,
    surface: 0xF3F3F3,
    raised: 0xFFFFFF,
    line: 0xCACACA,
    line_strong: 0x929292,
    fg: 0x000000,
    fg_dim: 0x505050,
    fg_faint: 0xA4A4A4,
    accent: 0x000000,
    accent_hover: 0x060606,
    accent_active: 0x161616,
    accent_fg: 0xFFFFFF,
    accent_text: 0x000000,
    accent_dim: 0xD1D1D1,
    danger: 0xAF2C1F,
    danger_fg: 0xFFFFFF,
    success: 0x3A732C,
    warning: 0x776100,
};

pub const GRAPHITE_DARK: Palette = Palette {
    tone: Tone::Dark,
    name: "Graphite Dark",
    sunken: 0x191919,
    bg: 0x1F1F1F,
    surface: 0x262626,
    raised: 0x2F2F2F,
    line: 0x444444,
    line_strong: 0x585858,
    fg: 0xF2F2F2,
    fg_dim: 0xB7B7B7,
    fg_faint: 0x747474,
    accent: 0xD5D0C8,
    accent_hover: 0xE5E0D8,
    accent_active: 0xC2BDB5,
    accent_fg: 0x1F1F1F,
    accent_text: 0xD5D0C8,
    accent_dim: 0x4B4948,
    danger: 0xF0584B,
    danger_fg: 0x1F1F1F,
    success: 0x90D281,
    warning: 0xE8D34F,
};

pub const GRAPHITE_LIGHT: Palette = Palette {
    tone: Tone::Light,
    name: "Graphite Light",
    sunken: 0xCACACA,
    bg: 0xD6D6D6,
    surface: 0xCFCFCF,
    raised: 0xE1E1E1,
    line: 0xAEAEAE,
    line_strong: 0x8C8C8C,
    fg: 0x0F0F0F,
    fg_dim: 0x404040,
    fg_faint: 0x808080,
    accent: 0x504D47,
    accent_hover: 0x43403A,
    accent_active: 0x383530,
    accent_fg: 0xE1E1E1,
    accent_text: 0x504D47,
    accent_dim: 0xABAAA8,
    danger: 0xAC281B,
    danger_fg: 0xD6D6D6,
    success: 0x2E6720,
    warning: 0x6E5900,
};

pub const SLATE_DARK: Palette = Palette {
    tone: Tone::Dark,
    name: "Slate Dark",
    sunken: 0x04070C,
    bg: 0x070C11,
    surface: 0x0D1217,
    raised: 0x13191F,
    line: 0x252B31,
    line_strong: 0x383E45,
    fg: 0xE5E8EB,
    fg_dim: 0x888F97,
    fg_faint: 0x50565D,
    accent: 0xA6C1D9,
    accent_hover: 0xB6D1E9,
    accent_active: 0x93AEC6,
    accent_fg: 0x070C11,
    accent_text: 0xA6C1D9,
    accent_dim: 0x2D3741,
    danger: 0xF0584B,
    danger_fg: 0x070C11,
    success: 0x90D281,
    warning: 0xE8D34F,
};

pub const SLATE_LIGHT: Palette = Palette {
    tone: Tone::Light,
    name: "Slate Light",
    sunken: 0xDBE0E4,
    bg: 0xE7ECF0,
    surface: 0xDFE4E8,
    raised: 0xF0F5FA,
    line: 0xC0C4C8,
    line_strong: 0xA1A5A9,
    fg: 0x141516,
    fg_dim: 0x505357,
    fg_faint: 0x93979B,
    accent: 0x456783,
    accent_hover: 0x375974,
    accent_active: 0x2C4E68,
    accent_fg: 0xF0F5FA,
    accent_text: 0x456783,
    accent_dim: 0xB3C1CD,
    danger: 0xAF2C1F,
    danger_fg: 0xE7ECF0,
    success: 0x3A732C,
    warning: 0x776100,
};

pub const CONCRETE_DARK: Palette = Palette {
    tone: Tone::Dark,
    name: "Concrete Dark",
    sunken: 0x090704,
    bg: 0x0E0B07,
    surface: 0x14110D,
    raised: 0x1B1813,
    line: 0x2E2A25,
    line_strong: 0x413D38,
    fg: 0xE9E7E5,
    fg_dim: 0x928E88,
    fg_faint: 0x595550,
    accent: 0xDFD3BB,
    accent_hover: 0xF0E3CB,
    accent_active: 0xCBC0A8,
    accent_fg: 0x0E0B07,
    accent_text: 0xDFD3BB,
    accent_dim: 0x403B32,
    danger: 0xF0584B,
    danger_fg: 0x0E0B07,
    success: 0x90D281,
    warning: 0xE8D34F,
};

pub const CONCRETE_LIGHT: Palette = Palette {
    tone: Tone::Light,
    name: "Concrete Light",
    sunken: 0xE3DED8,
    bg: 0xEFEAE4,
    surface: 0xE7E2DC,
    raised: 0xF8F4ED,
    line: 0xC7C3BC,
    line_strong: 0xA8A49E,
    fg: 0x161513,
    fg_dim: 0x56524D,
    fg_faint: 0x9A9690,
    accent: 0x776246,
    accent_hover: 0x685438,
    accent_active: 0x5D492E,
    accent_fg: 0xF8F4ED,
    accent_text: 0x776246,
    accent_dim: 0xC9BEB1,
    danger: 0xAF2C1F,
    danger_fg: 0xEFEAE4,
    success: 0x3A732C,
    warning: 0x776100,
};

pub const HARBOR_DARK: Palette = Palette {
    tone: Tone::Dark,
    name: "Harbor Dark",
    sunken: 0x000714,
    bg: 0x020C19,
    surface: 0x051220,
    raised: 0x0B1928,
    line: 0x1D2C3B,
    line_strong: 0x303F50,
    fg: 0xE2E9F0,
    fg_dim: 0x7F91A4,
    fg_faint: 0x475769,
    accent: 0xE48B53,
    accent_hover: 0xF59B63,
    accent_active: 0xD07840,
    accent_fg: 0x020C19,
    accent_text: 0xE48B53,
    accent_dim: 0x382A27,
    danger: 0xF0584B,
    danger_fg: 0x020C19,
    success: 0x90D281,
    warning: 0xE8D34F,
};

pub const HARBOR_LIGHT: Palette = Palette {
    tone: Tone::Light,
    name: "Harbor Light",
    sunken: 0xDAE0E6,
    bg: 0xE6ECF2,
    surface: 0xDEE4EA,
    raised: 0xEFF5FB,
    line: 0xBEC4CA,
    line_strong: 0xA0A5AB,
    fg: 0x141517,
    fg_dim: 0x4F5458,
    fg_faint: 0x92979C,
    accent: 0xDC7B40,
    accent_hover: 0xED8B50,
    accent_active: 0xC8692C,
    accent_fg: 0x141517,
    accent_text: 0xA94D00,
    accent_dim: 0xE3C8B9,
    danger: 0xAF2C1F,
    danger_fg: 0xE6ECF2,
    success: 0x3A732C,
    warning: 0x776100,
};

pub const CYANOTYPE_DARK: Palette = Palette {
    tone: Tone::Dark,
    name: "Cyanotype Dark",
    sunken: 0x000B2F,
    bg: 0x031337,
    surface: 0x091C41,
    raised: 0x11254B,
    line: 0x273C64,
    line_strong: 0x3B527C,
    fg: 0xE5EFFF,
    fg_dim: 0x91ABDB,
    fg_faint: 0x59729E,
    accent: 0xEFDF91,
    accent_hover: 0xFFF0A1,
    accent_active: 0xDBCB7E,
    accent_fg: 0x031337,
    accent_text: 0xEFDF91,
    accent_dim: 0x3C444D,
    danger: 0xF0584B,
    danger_fg: 0x031337,
    success: 0x90D281,
    warning: 0xFFBB5C,
};

pub const CYANOTYPE_LIGHT: Palette = Palette {
    tone: Tone::Light,
    name: "Cyanotype Light",
    sunken: 0xD7E1E7,
    bg: 0xE3EDF3,
    surface: 0xDBE5EB,
    raised: 0xECF6FD,
    line: 0xBBC5CB,
    line_strong: 0x9DA6AC,
    fg: 0x131617,
    fg_dim: 0x4C545A,
    fg_faint: 0x8F989E,
    accent: 0xC3972A,
    accent_hover: 0xD3A73E,
    accent_active: 0xB08405,
    accent_fg: 0x131617,
    accent_text: 0x8B6100,
    accent_dim: 0xD9D1B3,
    danger: 0xAF2C1F,
    danger_fg: 0xE3EDF3,
    success: 0x3A732C,
    warning: 0x8D5406,
};

pub const PHOSPHOR_DARK: Palette = Palette {
    tone: Tone::Dark,
    name: "Phosphor Dark",
    sunken: 0x030904,
    bg: 0x060E07,
    surface: 0x0B140D,
    raised: 0x121B13,
    line: 0x242E25,
    line_strong: 0x364138,
    fg: 0xE5E9E5,
    fg_dim: 0x869288,
    fg_faint: 0x4E5950,
    accent: 0x88E48C,
    accent_hover: 0x98F59C,
    accent_active: 0x75D079,
    accent_fg: 0x060E07,
    accent_text: 0x88E48C,
    accent_dim: 0x254127,
    danger: 0xF0584B,
    danger_fg: 0x060E07,
    success: 0x2FD8D0,
    warning: 0xE8D34F,
};

pub const PHOSPHOR_LIGHT: Palette = Palette {
    tone: Tone::Light,
    name: "Phosphor Light",
    sunken: 0xDBE1DC,
    bg: 0xE7EDE8,
    surface: 0xDFE5E0,
    raised: 0xF0F6F1,
    line: 0xBFC5C0,
    line_strong: 0xA0A6A1,
    fg: 0x141614,
    fg_dim: 0x4F5550,
    fg_faint: 0x929893,
    accent: 0x4EA954,
    accent_hover: 0x5EB963,
    accent_active: 0x3A9642,
    accent_fg: 0x141614,
    accent_text: 0x117823,
    accent_dim: 0xB6D7B9,
    danger: 0xAF2C1F,
    danger_fg: 0xE7EDE8,
    success: 0x00756F,
    warning: 0x776100,
};

pub const VERDIGRIS_DARK: Palette = Palette {
    tone: Tone::Dark,
    name: "Verdigris Dark",
    sunken: 0x0E0502,
    bg: 0x130804,
    surface: 0x1A0E09,
    raised: 0x211510,
    line: 0x342722,
    line_strong: 0x483A34,
    fg: 0xEDE6E4,
    fg_dim: 0x9B8B84,
    fg_faint: 0x61524C,
    accent: 0x5AC6BD,
    accent_hover: 0x6BD6CD,
    accent_active: 0x44B3AA,
    accent_fg: 0x130804,
    accent_text: 0x5AC6BD,
    accent_dim: 0x243630,
    danger: 0xF0584B,
    danger_fg: 0x130804,
    success: 0x90D281,
    warning: 0xE8D34F,
};

pub const VERDIGRIS_LIGHT: Palette = Palette {
    tone: Tone::Light,
    name: "Verdigris Light",
    sunken: 0xE4DED9,
    bg: 0xF0EAE5,
    surface: 0xE8E2DD,
    raised: 0xFAF3EE,
    line: 0xC9C2BD,
    line_strong: 0xA9A39E,
    fg: 0x161514,
    fg_dim: 0x57524E,
    fg_faint: 0x9B9590,
    accent: 0x229991,
    accent_hover: 0x38A9A0,
    accent_active: 0x00877F,
    accent_fg: 0x161514,
    accent_text: 0x00756E,
    accent_dim: 0xAED0CA,
    danger: 0xAF2C1F,
    danger_fg: 0xF0EAE5,
    success: 0x3A732C,
    warning: 0x776100,
};

pub const BRUISE_DARK: Palette = Palette {
    tone: Tone::Dark,
    name: "Bruise Dark",
    sunken: 0x0D030F,
    bg: 0x120614,
    surface: 0x180C1B,
    raised: 0x1F1322,
    line: 0x322535,
    line_strong: 0x463849,
    fg: 0xECE5ED,
    fg_dim: 0x98889C,
    fg_faint: 0x5E4F62,
    accent: 0xC0E160,
    accent_hover: 0xD0F271,
    accent_active: 0xADCD4B,
    accent_fg: 0x120614,
    accent_text: 0xC0E160,
    accent_dim: 0x3C3B26,
    danger: 0xF0584B,
    danger_fg: 0x120614,
    success: 0x54D8B1,
    warning: 0xE8D34F,
};

pub const BRUISE_LIGHT: Palette = Palette {
    tone: Tone::Light,
    name: "Bruise Light",
    sunken: 0xE2DDE3,
    bg: 0xEEE9EF,
    surface: 0xE6E1E7,
    raised: 0xF8F2F9,
    line: 0xC6C1C7,
    line_strong: 0xA7A3A8,
    fg: 0x161516,
    fg_dim: 0x565157,
    fg_faint: 0x99949A,
    accent: 0x8EAD20,
    accent_hover: 0x9DBD37,
    accent_active: 0x7C9A00,
    accent_fg: 0x161516,
    accent_text: 0x577200,
    accent_dim: 0xCFD6AD,
    danger: 0xAF2C1F,
    danger_fg: 0xEEE9EF,
    success: 0x007556,
    warning: 0x776100,
};
