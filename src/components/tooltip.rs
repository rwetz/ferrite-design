//! Tooltips and keycaps.

use gpui::{
    AnyView, App, AppContext as _, Context, IntoElement, ParentElement, Render, RenderOnce,
    SharedString, Styled, Window, div, prelude::FluentBuilder as _, px,
};

use crate::fonts::FerriteText;
use crate::theme::palette;
use crate::tokens::{hsla, text};

// ── Kbd ───────────────────────────────────────────────────────────────────

/// Keycaps for a shortcut: `kbd("Ctrl+Shift+P")` → `[CTRL][SHIFT][P]`.
/// Square caps, 1px frame, a 2px bottom edge for the key's depth.
#[derive(IntoElement)]
pub struct Kbd {
    keys: Vec<SharedString>,
}

pub fn kbd(shortcut: &str) -> Kbd {
    Kbd {
        keys: shortcut
            .split('+')
            .map(|k| SharedString::from(k.trim().to_uppercase()))
            .filter(|k| !k.is_empty())
            .collect(),
    }
}

impl RenderOnce for Kbd {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        div().flex().flex_row().gap_1().children(self.keys.into_iter().map(|key| {
            div()
                .px_1()
                .min_w(px(18.))
                .flex()
                .justify_center()
                .bg(hsla(p.raised))
                .border_1()
                .border_b_2()
                .border_color(hsla(p.line_strong))
                .text_color(hsla(p.fg_dim))
                .body(text::XS)
                .child(key)
        }))
    }
}

// ── Tooltip ───────────────────────────────────────────────────────────────

/// A square tooltip: raised fill, 1px strong frame, body text, optional
/// shortcut. Attach with gpui's `.tooltip(..)`:
///
/// ```ignore
/// div().id("x").tooltip(ferrite_design::components::tooltip("Run task").kbd("Ctrl+R").builder())
/// ```
#[derive(Clone)]
pub struct Tooltip {
    text: SharedString,
    kbd: Option<SharedString>,
}

pub fn tooltip(text: impl Into<SharedString>) -> Tooltip {
    Tooltip { text: text.into(), kbd: None }
}

impl Tooltip {
    pub fn kbd(mut self, shortcut: impl Into<SharedString>) -> Self {
        self.kbd = Some(shortcut.into());
        self
    }

    /// The closure gpui's `.tooltip()` wants.
    pub fn builder(self) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
        move |_, cx| cx.new(|_| self.clone()).into()
    }
}

impl Render for Tooltip {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .bg(hsla(p.raised))
            .border_1()
            .border_color(hsla(p.line_strong))
            .text_color(hsla(p.fg))
            .body(text::SM)
            .child(self.text.clone())
            .when_some(self.kbd.clone(), |el, k| el.child(kbd(&k)))
    }
}
