//! # ferrite-design
//!
//! The design language for the Ferrite family of GPUI apps.
//!
//! Ferrite inherits the *rules* of Nexis (`rwetz/nexis-design`) — one accent,
//! a single source of truth for tokens, a shared motion vocabulary, custom
//! window chrome with native macOS controls, no flash on startup, tripwire
//! tests on the invariants — and deliberately none of its *look*. Where Nexis
//! is coral glass with rounded corners and springs, Ferrite is phosphor amber
//! on iron grey: hard corners, pixel display type, ordered dither instead of
//! gradients, and stepped motion.
//!
//! ```no_run
//! use gpui::App;
//!
//! fn main() {
//!     gpui_platform::application().run(|cx: &mut App| {
//!         ferrite_design::init(ferrite_design::Appearance::Dark, cx);
//!         // open windows with ferrite_design::chrome::window_options(..)
//!     });
//! }
//! ```
//!
//! See `docs/DESIGN_LANGUAGE.md` for the full language and
//! `examples/showcase.rs` for every primitive on one screen.

// The theme config is one large `json!` literal.
#![recursion_limit = "512"]

pub mod ascii;
pub mod chrome;
pub mod components;
pub mod dither;
pub mod fonts;
pub mod icon;
pub mod motion;
mod raster;
pub mod theme;
pub mod tokens;

pub use fonts::{FerriteText, Scale};
pub use icon::{Icon, icon};
pub use theme::{Appearance, palette};
pub use tokens::{IRON, PAPER, Palette};

/// Initialise gpui-component, register Ferrite's fonts and install its theme.
///
/// Call once at startup, **before opening any window**, so the first frame
/// is already Ferrite.
pub fn init(appearance: Appearance, cx: &mut gpui::App) {
    gpui_component::init(cx);
    if let Err(err) = fonts::register(cx) {
        // Text falls back to system faces; the app still works.
        eprintln!("ferrite-design: failed to register embedded fonts: {err:#}");
    }
    theme::install(appearance, cx);
}
