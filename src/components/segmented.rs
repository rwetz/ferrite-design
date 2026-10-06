//! The segmented control: one choice from a few, as a single framed strip.
//!
//! ```text
//!  ┌──────┬──────────┬──────────┐
//!  │ FAST │█BALANCED█│ THOROUGH │   selected segment is inverse video
//!  └──────┴──────────┴──────────┘
//! ```
//!
//! Controlled, like the toggles: pass `.selected(i)`, get the new index in
//! `on_select`. One tab stop for the whole strip; ←/→ (and Home/End) move the
//! selection, the way a radio group does.

use std::rc::Rc;

use gpui::{
    App, ElementId, FocusHandle, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder as _, px,
};

use crate::fonts::{FerriteText, Scale};
use crate::icon::{Icon, icon};
use crate::theme::palette;
use crate::tokens::hsla;

type SelectHandler = Rc<dyn Fn(&usize, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct Segmented {
    id: ElementId,
    options: Vec<(SharedString, Option<Icon>)>,
    selected: usize,
    disabled: bool,
    on_select: Option<SelectHandler>,
}

pub fn segmented(id: impl Into<ElementId>) -> Segmented {
    Segmented { id: id.into(), options: Vec::new(), selected: 0, disabled: false, on_select: None }
}

impl Segmented {
    pub fn option(mut self, label: impl Into<SharedString>) -> Self {
        self.options.push((label.into(), None));
        self
    }

    pub fn option_with_icon(mut self, label: impl Into<SharedString>, icon: Icon) -> Self {
        self.options.push((label.into(), Some(icon)));
        self
    }

    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Called with the newly selected index.
    pub fn on_select(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

/// The index a key moves the selection to, if it's a navigation key.
fn step(key: &str, current: usize, len: usize) -> Option<usize> {
    if len == 0 {
        return None;
    }
    Some(match key {
        "left" | "up" => (current + len - 1) % len,
        "right" | "down" => (current + 1) % len,
        "home" => 0,
        "end" => len - 1,
        _ => return None,
    })
}

impl RenderOnce for Segmented {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let focus: FocusHandle = window.use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle()).read(cx).clone();
        let len = self.options.len();
        let selected = self.selected.min(len.saturating_sub(1));
        let handler = self.on_select.filter(|_| !self.disabled);
        let grow = crate::animate::play_on_change(ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), "grow".into()), selected, crate::motion::FAST, window, cx).eased();

        let mut strip = div()
            .id(self.id.clone())
            .role(Role::RadioGroup)
            .track_focus(&focus)
            .when(!self.disabled, |el| el.tab_stop(true))
            .flex()
            .flex_row()
            .flex_none()
            .h(px(28.))
            .border_1()
            .border_color(hsla(if self.disabled { p.line } else { p.line_strong }))
            .when(!self.disabled, |el| el.focus_visible(|s| s.border_color(hsla(p.accent))))
            .when(!self.disabled, |el| {
                // Clicking takes focus (so the arrows work next) without
                // gpui's default focus handling.
                let focus = focus.clone();
                el.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    window.prevent_default();
                    focus.focus(window, cx);
                })
            })
            .when_some(handler.clone(), |el, handler| {
                el.on_key_down(move |ev, window, cx| {
                    if let Some(next) = step(ev.keystroke.key.as_str(), selected, len) {
                        cx.stop_propagation();
                        if next != selected {
                            handler(&next, window, cx);
                        }
                    }
                })
            });

        for (i, (label, glyph)) in self.options.into_iter().enumerate() {
            let on = i == selected;
            let (ink, bg) = match (self.disabled, on) {
                (true, true) => (p.fg_dim, Some(p.raised)),
                (true, false) => (p.fg_faint, None),
                (false, true) => (p.accent_fg, Some(p.accent)),
                (false, false) => (p.fg_dim, None),
            };
            strip = strip.child(
                div()
                    .id(("segment", i))
                    .role(Role::RadioButton)
                    .aria_label(label.clone())
                    .aria_toggled(if on { gpui::Toggled::True } else { gpui::Toggled::False })
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .h_full()
                    .px_3()
                    .when(i > 0, |el| el.border_l_1().border_color(hsla(p.line_strong)))
                    .relative()
                    .when_some(bg, |el, bg| {
                        // The selection block grows out from the segment's
                        // centre when it lands here.
                        let w = if on { grow } else { 1. };
                        el.child(div().absolute().top_0().bottom_0().left(gpui::relative((1. - w) / 2.)).w(gpui::relative(w)).bg(hsla(bg)))
                    })
                    .text_color(hsla(ink))
                    .when(!self.disabled && !on, |el| el.hover(|s| s.bg(hsla(p.raised)).text_color(hsla(p.fg))))
                    .when_some(glyph, |el, g| el.child(icon(g).fit(px(22.)).color(hsla(ink))))
                    .child(div().display(Scale::X1, window).child(label.to_uppercase()))
                    .when_some(handler.clone().filter(|_| !on), |el, handler| {
                        el.on_click(move |_, window, cx| handler(&i, window, cx))
                    }),
            );
        }
        strip
    }
}

#[cfg(test)]
mod tests {
    use super::step;

    #[test]
    fn arrows_wrap_and_jump() {
        assert_eq!(step("right", 2, 3), Some(0));
        assert_eq!(step("left", 0, 3), Some(2));
        assert_eq!(step("end", 0, 3), Some(2));
        assert_eq!(step("home", 2, 3), Some(0));
        assert_eq!(step("x", 1, 3), None);
        assert_eq!(step("right", 0, 0), None);
    }
}
