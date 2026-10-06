//! The bridge from Ferrite tokens to gpui-component's theme.
//!
//! gpui-component keeps one `ThemeConfig` per mode and reloads the active one
//! whenever the mode changes. So Ferrite does not poke colors into the live
//! theme (they would be overwritten on the next light/dark switch); it builds
//! a full `ThemeConfig` for each palette and installs both, which makes the
//! component library's own mode switching keep the Ferrite look.
//!
//! The configs are built from `tokens.rs` at runtime rather than shipped as
//! JSON files, so the palette has exactly one definition and cannot drift.

use std::rc::Rc;

use gpui::{App, Global, Subscription, Window, WindowAppearance};
use gpui_component::{Theme, ThemeConfig, ThemeMode};
use serde_json::json;

use crate::fonts;
use crate::tokens::{Palette, Tone, css, css_a, IRON, PAPER};

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

/// Install both Ferrite palettes into gpui-component and apply `pref`.
///
/// Call after `gpui_component::init` and **before opening any window**, so
/// the first frame is already Ferrite. (That is the GPUI version of Nexis's
/// no-flash rule: a window that paints one frame of the library default and
/// then switches looks broken.)
pub fn install(pref: Appearance, cx: &mut App) {
    let dark = Rc::new(config(&IRON));
    let light = Rc::new(config(&PAPER));
    Theme::update(cx, |theme| {
        theme.dark_theme = dark;
        theme.light_theme = light;
    });
    cx.set_global(AppearancePref(pref));
    let tone = resolve(pref, cx.window_appearance());
    Theme::change(mode_for(tone), None, cx);
}

/// Change the appearance preference at runtime (e.g. from a settings menu).
pub fn set_appearance(pref: Appearance, window: &mut Window, cx: &mut App) {
    cx.set_global(AppearancePref(pref));
    let tone = resolve(pref, window.appearance());
    Theme::change(mode_for(tone), Some(window), cx);
}

/// The current preference.
pub fn appearance(cx: &App) -> Appearance {
    cx.try_global::<AppearancePref>().map(|p| p.0).unwrap_or_default()
}

/// The palette currently on screen. Use this, not `IRON`/`PAPER` directly,
/// anywhere you paint a token yourself.
pub fn palette(cx: &App) -> &'static Palette {
    let tone = if Theme::global(cx).mode.is_dark() { Tone::Dark } else { Tone::Light };
    Palette::for_tone(tone)
}

/// Re-resolve `System` whenever the OS appearance changes. Keep the returned
/// subscription alive for as long as the window (store it on the root view).
pub fn follow_system(window: &Window) -> Subscription {
    window.observe_window_appearance(|window, cx| {
        if appearance(cx) == Appearance::System {
            let tone = resolve(Appearance::System, window.appearance());
            Theme::change(mode_for(tone), Some(window), cx);
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

fn mode_for(tone: Tone) -> ThemeMode {
    match tone {
        Tone::Dark => ThemeMode::Dark,
        Tone::Light => ThemeMode::Light,
    }
}

/// Project a Ferrite palette onto gpui-component's theme schema.
///
/// Every key is set explicitly, even where the library would derive a value,
/// so no library default (and no shadcn blue) can leak through.
pub fn config(p: &Palette) -> ThemeConfig {
    serde_json::from_value(config_json(p))
        .expect("Ferrite theme config must match gpui-component's schema")
}

fn config_json(p: &Palette) -> serde_json::Value {
    let c = css;
    let selection = css_a(p.accent, if p.is_dark() { 0x55 } else { 0x66 });
    let drop = css_a(p.accent, 0x33);
    let overlay = if p.is_dark() { css_a(0x000000, 0xB0) } else { css_a(p.fg, 0x55) };
    let transparent = css_a(0x000000, 0x00);
    // The chart ramp is amber stepping toward grey: one hue, read by lightness.
    let chart = if p.is_dark() {
        ["#f2a93b", "#c4873a", "#8e6a3a", "#6b6258", "#4a4a4e"]
    } else {
        ["#c9841a", "#a8732e", "#86663f", "#7a756c", "#a9a49a"]
    };

    json!({
        "name": p.name,
        "mode": if p.is_dark() { "dark" } else { "light" },
        "font.family": fonts::BODY,
        "font.size": 13.0,
        "mono_font.family": fonts::MONO,
        "mono_font.size": 13.0,
        "radius": 0,
        "radius.lg": 0,
        "shadow": false,
        "colors": {
            "background": c(p.bg),
            "foreground": c(p.fg),
            "border": c(p.line),
            "input.border": c(p.line_strong),
            "ring": c(p.accent),
            "caret": c(p.accent),
            "selection.background": selection,
            "overlay": overlay,
            "window.border": c(p.line_strong),

            "accent.background": c(p.raised),
            "accent.foreground": c(p.fg),
            "muted.background": c(p.surface),
            "muted.foreground": c(p.fg_dim),
            "popover.background": c(p.raised),
            "popover.foreground": c(p.fg),
            "accordion.background": c(p.bg),
            "group_box.background": c(p.surface),
            "group_box.foreground": c(p.fg),
            "group_box.title.foreground": c(p.fg_dim),
            "description_list.label.background": c(p.surface),
            "description_list.label.foreground": c(p.fg_dim),

            "primary.background": c(p.accent),
            "primary.hover.background": c(p.accent_hover),
            "primary.active.background": c(p.accent_active),
            "primary.foreground": c(p.accent_fg),
            "secondary.background": c(p.raised),
            "secondary.hover.background": c(p.line),
            "secondary.active.background": c(p.line_strong),
            "secondary.foreground": c(p.fg),

            "button.background": c(p.raised),
            "button.hover.background": c(p.line),
            "button.active.background": c(p.line_strong),
            "button.foreground": c(p.fg),
            "button.primary.background": c(p.accent),
            "button.primary.hover.background": c(p.accent_hover),
            "button.primary.active.background": c(p.accent_active),
            "button.primary.foreground": c(p.accent_fg),
            "button.secondary.background": c(p.raised),
            "button.secondary.hover.background": c(p.line),
            "button.secondary.active.background": c(p.line_strong),
            "button.secondary.foreground": c(p.fg),
            "button.danger.background": c(p.danger),
            "button.danger.hover.background": c(p.danger),
            "button.danger.active.background": c(p.danger),
            "button.danger.foreground": c(p.danger_fg),
            "button.success.background": c(p.success),
            "button.success.hover.background": c(p.success),
            "button.success.active.background": c(p.success),
            "button.success.foreground": c(p.bg),
            "button.warning.background": c(p.warning),
            "button.warning.hover.background": c(p.warning),
            "button.warning.active.background": c(p.warning),
            "button.warning.foreground": c(p.bg),
            // No blue "info" in Ferrite: info is just neutral emphasis.
            "button.info.background": c(p.raised),
            "button.info.hover.background": c(p.line),
            "button.info.active.background": c(p.line_strong),
            "button.info.foreground": c(p.fg),

            "danger.background": c(p.danger),
            "danger.hover.background": c(p.danger),
            "danger.active.background": c(p.danger),
            "danger.foreground": c(p.danger_fg),
            "success.background": c(p.success),
            "success.hover.background": c(p.success),
            "success.active.background": c(p.success),
            "success.foreground": c(p.bg),
            "warning.background": c(p.warning),
            "warning.hover.background": c(p.warning),
            "warning.active.background": c(p.warning),
            "warning.foreground": c(p.bg),
            "info.background": c(p.raised),
            "info.hover.background": c(p.line),
            "info.active.background": c(p.line_strong),
            "info.foreground": c(p.fg),

            "link": c(p.accent_text),
            "link.hover": c(p.accent_hover),
            "link.active": c(p.accent_active),

            "list.background": c(p.bg),
            "list.even.background": c(p.bg),
            "list.head.background": c(p.surface),
            "list.hover.background": c(p.raised),
            "list.active.background": c(p.accent_dim),
            "list.active.border": c(p.accent),
            "table.background": c(p.bg),
            "table.even.background": c(p.surface),
            "table.head.background": c(p.surface),
            "table.head.foreground": c(p.fg_dim),
            "table.foot.background": c(p.surface),
            "table.foot.foreground": c(p.fg_dim),
            "table.hover.background": c(p.raised),
            "table.active.background": c(p.accent_dim),
            "table.active.border": c(p.accent),
            "table.row.border": c(p.line),

            "tab_bar.background": c(p.surface),
            "tab_bar.segmented.background": c(p.raised),
            "tab.background": c(p.surface),
            "tab.foreground": c(p.fg_dim),
            "tab.active.background": c(p.bg),
            "tab.active.foreground": c(p.fg),

            "title_bar.background": c(p.surface),
            "title_bar.border": c(p.line),
            "status_bar.background": c(p.surface),
            "status_bar.border": c(p.line),
            "sidebar.background": c(p.surface),
            "sidebar.foreground": c(p.fg),
            "sidebar.border": c(p.line),
            "sidebar.accent.background": c(p.raised),
            "sidebar.accent.foreground": c(p.fg),
            "sidebar.primary.background": c(p.accent),
            "sidebar.primary.foreground": c(p.accent_fg),

            "scrollbar.background": transparent,
            "scrollbar.thumb.background": c(p.line_strong),
            "scrollbar.thumb.hover.background": c(p.fg_faint),
            "switch.background": c(p.line_strong),
            "switch.thumb.background": c(p.fg),
            "slider.background": c(p.accent),
            "slider.thumb.background": c(p.fg),
            "progress.bar.background": c(p.accent),
            "skeleton.background": c(p.raised),
            "drag.border": c(p.accent),
            "drop_target.background": drop,

            "chart.1": chart[0],
            "chart.2": chart[1],
            "chart.3": chart[2],
            "chart.4": chart[3],
            "chart.5": chart[4],
            "chart.bullish": c(p.success),
            "chart.bearish": c(p.danger),
            "chart.grid": c(p.line),

            // Base hues exist for syntax/ANSI-style uses only. Desaturated so
            // nothing competes with the amber.
            "base.red": c(p.danger),
            "base.red.light": css_a(p.danger, 0x33),
            "base.green": c(p.success),
            "base.green.light": css_a(p.success, 0x33),
            "base.yellow": c(p.warning),
            "base.yellow.light": css_a(p.warning, 0x33),
            "base.blue": if p.is_dark() { "#7f9cc0" } else { "#3d5f8a" },
            "base.blue.light": if p.is_dark() { "#7f9cc033" } else { "#3d5f8a33" },
            "base.cyan": if p.is_dark() { "#77b8b4" } else { "#2f6f6b" },
            "base.cyan.light": if p.is_dark() { "#77b8b433" } else { "#2f6f6b33" },
            "base.magenta": if p.is_dark() { "#b98aae" } else { "#7d4a72" },
            "base.magenta.light": if p.is_dark() { "#b98aae33" } else { "#7d4a7233" },
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configs_parse_against_the_library_schema() {
        let dark = config(&IRON);
        let light = config(&PAPER);
        assert!(dark.mode.is_dark());
        assert!(!light.mode.is_dark());
        assert_eq!(dark.radius, Some(0));
        assert_eq!(dark.shadow, Some(false));
    }

    #[test]
    fn every_color_key_survives_the_round_trip() {
        // serde ignores unknown keys, so a key renamed in a gpui-component
        // bump would silently fall back to the library default (shadcn blue).
        // Round-trip through the real struct and demand every key came back.
        for p in [&IRON, &PAPER] {
            let sent = config_json(p);
            let back = serde_json::to_value(config(p)).unwrap();
            for key in sent["colors"].as_object().unwrap().keys() {
                assert!(
                    !back["colors"][key].is_null(),
                    "{}: color key `{key}` was dropped by gpui-component's schema",
                    p.name
                );
            }
        }
    }
}
