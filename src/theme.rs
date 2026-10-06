//! Appearance: which palette is on screen, and keeping it in step with the
//! user's preference and the OS.
//!
//! Two choices make the palette: the **scheme** ([`set_scheme`]: Ferrite by
//! default, or any of [`crate::schemes::SCHEMES`]) and the **tone** (dark or
//! light, from the [`Appearance`] preference). Both are gpui globals;
//! [`palette`] reads them, every component paints from that, and changing
//! either refreshes every window. There is no second theme system to keep in
//! sync and nothing that can fall back to someone else's default colors.

use gpui::{App, Global, Subscription, Window, WindowAppearance};

use crate::schemes::{FERRITE, Scheme};
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

struct ActiveScheme(&'static Scheme);
impl Global for ActiveScheme {}

/// The last time the palette on screen changed at runtime: a counter and
/// the palette it changed *from*. `chrome::window_frame` keys its glitch
/// transition on it.
#[derive(Clone, Copy)]
struct LastShift {
    epoch: u64,
    from: &'static Palette,
}
impl Global for LastShift {}

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

/// The palette currently on screen: the active scheme in the active tone.
/// Use this, never a palette constant, anywhere you paint a token yourself.
pub fn palette(cx: &App) -> &'static Palette {
    scheme(cx).palette(tone(cx))
}

/// The scheme on screen (Ferrite unless the app chose another).
pub fn scheme(cx: &App) -> &'static Scheme {
    cx.try_global::<ActiveScheme>().map(|s| s.0).unwrap_or(&FERRITE)
}

/// Switch color scheme. Call it before opening the first window (after
/// [`crate::init`]) to start in a scheme with no flash, or at any time from
/// a settings screen; every window repaints.
///
/// ```ignore
/// ferrite_design::init(Appearance::Dark, cx);
/// theme::set_scheme(schemes::by_key("harbor").unwrap(), cx);
/// ```
pub fn set_scheme(scheme: &'static Scheme, cx: &mut App) {
    let from = palette(cx);
    cx.set_global(ActiveScheme(scheme));
    record_shift(from, cx);
    cx.refresh_windows();
}

/// The most recent runtime palette change, as `(epoch, from)`: the epoch
/// counts changes (0 = none yet) and `from` is the palette that was on
/// screen before it. For transitions; [`palette`] is always the new one.
pub fn last_shift(cx: &App) -> (u64, Option<&'static Palette>) {
    cx.try_global::<LastShift>().map(|s| (s.epoch, Some(s.from))).unwrap_or((0, None))
}

fn record_shift(from: &'static Palette, cx: &mut App) {
    if std::ptr::eq(from, palette(cx)) {
        return;
    }
    let epoch = last_shift(cx).0 + 1;
    cx.set_global(LastShift { epoch, from });
}

/// Developer overrides from the environment, for trying an app in another
/// look without a settings screen. Call right after [`crate::init`]:
///
/// - `FERRITE_SCHEME=harbor` — any key in [`crate::schemes::SCHEMES`]
/// - `FERRITE_APPEARANCE=light|dark|system`
/// - `FERRITE_FPS=25` — the refresh rate (12–240)
///
/// Unset or unrecognised values change nothing.
pub fn apply_env(cx: &mut App) {
    if let Some(scheme) = std::env::var("FERRITE_SCHEME").ok().and_then(|k| crate::schemes::by_key(&k)) {
        set_scheme(scheme, cx);
    }
    let pref = match std::env::var("FERRITE_APPEARANCE").as_deref() {
        Ok("light") => Some(Appearance::Light),
        Ok("dark") => Some(Appearance::Dark),
        Ok("system") => Some(Appearance::System),
        _ => None,
    };
    if let Some(pref) = pref {
        install(pref, cx);
    }
    if let Some(fps) = std::env::var("FERRITE_FPS").ok().and_then(|v| v.parse().ok()) {
        crate::motion::set_fps(fps);
    }
}

fn set_tone(tone: Tone, cx: &mut App) {
    let from = palette(cx);
    cx.set_global(ActiveTone(tone));
    record_shift(from, cx);
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
