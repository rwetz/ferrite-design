//! Appearance: which palette is on screen, and keeping it in step with the
//! user's preference and the OS.
//!
//! Ferrite owns this outright. The active [`Tone`] is a gpui global;
//! [`palette`] reads it, every component paints from that, and switching
//! tone refreshes every window. There is no second theme system to keep in
//! sync and nothing that can fall back to someone else's default colors.

use gpui::{App, Global, Subscription, Window, WindowAppearance};

use crate::tokens::{Palette, Tone};

/// The user's appearance preference. Ferrite is dark-first: the default is
/// [`Appearance::Dark`], not `System`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Appearance {
    #[default]
    Dark,
    Light,
    /// Follow the OS. Requires [`follow_system`] to be wired per window.
    System,
}

struct AppearancePref(Appearance);
impl Global for AppearancePref {}

struct ActiveTone(Tone);
impl Global for ActiveTone {}

/// Set the preference and resolve the first tone.
///
/// Call **before opening any window**, so the first frame is already the
/// right palette (Nexis's no-flash rule: a window that paints one frame in
/// the wrong mode and then switches looks broken).
pub fn install(pref: Appearance, cx: &mut App) {
    cx.set_global(AppearancePref(pref));
    cx.set_global(ActiveTone(resolve(pref, cx.window_appearance())));
}

/// Change the appearance preference at runtime (e.g. from a settings menu).
pub fn set_appearance(pref: Appearance, window: &mut Window, cx: &mut App) {
    cx.set_global(AppearancePref(pref));
    set_tone(resolve(pref, window.appearance()), cx);
}

/// The current preference.
pub fn appearance(cx: &App) -> Appearance {
    cx.try_global::<AppearancePref>().map(|p| p.0).unwrap_or_default()
}

/// The tone on screen.
pub fn tone(cx: &App) -> Tone {
    cx.try_global::<ActiveTone>().map(|t| t.0).unwrap_or(Tone::Dark)
}

/// The palette currently on screen. Use this, not `IRON`/`PAPER` directly,
/// anywhere you paint a token yourself.
pub fn palette(cx: &App) -> &'static Palette {
    Palette::for_tone(tone(cx))
}

fn set_tone(tone: Tone, cx: &mut App) {
    cx.set_global(ActiveTone(tone));
    cx.refresh_windows();
}

/// Re-resolve `System` whenever the OS appearance changes. Keep the returned
/// subscription alive for as long as the window (store it on the root view).
pub fn follow_system(window: &Window) -> Subscription {
    window.observe_window_appearance(|window, cx| {
        if appearance(cx) == Appearance::System {
            set_tone(resolve(Appearance::System, window.appearance()), cx);
        }
    })
}

fn resolve(pref: Appearance, os: WindowAppearance) -> Tone {
    match pref {
        Appearance::Dark => Tone::Dark,
        Appearance::Light => Tone::Light,
        Appearance::System => match os {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Tone::Dark,
            WindowAppearance::Light | WindowAppearance::VibrantLight => Tone::Light,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_preferences_ignore_the_os() {
        assert_eq!(resolve(Appearance::Dark, WindowAppearance::Light), Tone::Dark);
        assert_eq!(resolve(Appearance::Light, WindowAppearance::Dark), Tone::Light);
        assert_eq!(resolve(Appearance::System, WindowAppearance::VibrantLight), Tone::Light);
        assert_eq!(resolve(Appearance::System, WindowAppearance::Dark), Tone::Dark);
    }
}
