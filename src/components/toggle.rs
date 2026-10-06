//! Glyph toggles: checkbox `[x]`, radio `(•)`, and the square switch
//! (docs/COMPONENTS.md #3).
//!
//! All three are *controlled*: the app owns the value and passes it in with
//! `.checked(..)`; the click handler receives the **new** value. Same
//! contract as gpui-component's `Checkbox`/`Radio`/`Switch`.
//!
//! The mark is display-face text (`[ ]`, `[x]`, `[-]`, `( )`, `(•)`), so a
//! checkbox reads like a line from a config screen. The label is body type —
//! it's content, not chrome.

use std::rc::Rc;

use gpui::{
    App, ElementId, FocusHandle, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled, Toggled, Window, div,
    prelude::FluentBuilder as _, px,
};

use crate::dither::{self, dither};
use crate::fonts::{FerriteText, Scale, display_size};
use crate::theme::palette;
use crate::tokens::{hsla, text};

type ChangeHandler = Rc<dyn Fn(&bool, &mut Window, &mut App)>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Checkbox,
    Radio,
}

/// A checkbox or radio.
#[derive(IntoElement)]
pub struct Toggle {
    id: ElementId,
    kind: Kind,
    label: Option<SharedString>,
    checked: bool,
    indeterminate: bool,
    disabled: bool,
    on_change: Option<ChangeHandler>,
}

pub fn checkbox(id: impl Into<ElementId>) -> Toggle {
    Toggle::new(id, Kind::Checkbox)
}

/// A radio. Group several by giving them the same handler and checking the
/// one that matches your state; the radio never unchecks itself.
pub fn radio(id: impl Into<ElementId>) -> Toggle {
    Toggle::new(id, Kind::Radio)
}

impl Toggle {
    fn new(id: impl Into<ElementId>, kind: Kind) -> Self {
        Self { id: id.into(), kind, label: None, checked: false, indeterminate: false, disabled: false, on_change: None }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Checkbox only: the `[-]` mixed state (e.g. a "select all" over a
    /// partial selection). Clicking it reports `true`.
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.indeterminate = indeterminate;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Called with the new value.
    pub fn on_change(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Toggle {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let focus: FocusHandle = window.use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle()).read(cx).clone();
        let mixed = self.kind == Kind::Checkbox && self.indeterminate;
        let (open, close) = match self.kind {
            Kind::Checkbox => ("[", "]"),
            Kind::Radio => ("(", ")"),
        };
        let mark = match (self.kind, self.checked, mixed) {
            (Kind::Checkbox, _, true) => "-",
            (Kind::Checkbox, true, _) => "x",
            (Kind::Radio, true, _) => "•",
            _ => " ",
        };
        let on = self.checked || mixed;
        // The mark stamps in through two noise glyphs when it changes.
        let stamp = crate::animate::play_on_change(
            ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), "stamp".into()),
            (self.checked, mixed),
            crate::motion::FAST,
            window,
            cx,
        );
        let mark = if on { crate::animate::stamp(mark, stamp) } else { mark };
        let next = match self.kind {
            Kind::Checkbox => mixed || !self.checked,
            Kind::Radio => true,
        };
        let (frame_ink, mark_ink, label_ink) = if self.disabled {
            (p.fg_faint, p.fg_faint, p.fg_faint)
        } else {
            (p.fg_dim, p.accent, p.fg)
        };
        let group: SharedString = format!("ferrite-toggle-{}", self.id).into();

        div()
            .id(self.id.clone())
            .group(group.clone())
            .role(match self.kind {
                Kind::Checkbox => Role::CheckBox,
                Kind::Radio => Role::RadioButton,
            })
            .aria_toggled(if mixed { Toggled::Mixed } else if self.checked { Toggled::True } else { Toggled::False })
            .when_some(self.label.clone(), |el, l| el.aria_label(l))
            .track_focus(&focus)
            .when(!self.disabled, |el| el.tab_stop(true))
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .px_1()
            .border_1()
            .border_color(gpui::transparent_black())
            .when(!self.disabled, |el| el.focus_visible(|s| s.border_color(hsla(p.accent))))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .display(Scale::X1, window)
                    .text_color(hsla(frame_ink))
                    .when(!self.disabled, |el| el.group_hover(group.clone(), |s| s.text_color(hsla(p.fg))))
                    .child(open)
                    .child(div().text_color(hsla(if on { mark_ink } else { frame_ink })).child(mark))
                    .child(close),
            )
            .when_some(self.label, |el, l| el.child(div().body(text::BASE).text_color(hsla(label_ink)).child(l)))
            .when(!self.disabled, |el| el.on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default()))
            .when_some(self.on_change.filter(|_| !self.disabled), |el, handler| {
                el.on_click(move |_, window, cx| handler(&next, window, cx))
            })
    }
}

// ── Switch ────────────────────────────────────────────────────────────────

/// A square on/off switch. Off: a dithered `sunken` track with the thumb at
/// the left. On: an amber track with the thumb at the right. The thumb
/// jumps — no slide (Ferrite motion is instant or stepped).
#[derive(IntoElement)]
pub struct Switch {
    id: ElementId,
    label: Option<SharedString>,
    checked: bool,
    disabled: bool,
    on_change: Option<ChangeHandler>,
}

pub fn switch(id: impl Into<ElementId>) -> Switch {
    Switch { id: id.into(), label: None, checked: false, disabled: false, on_change: None }
}

impl Switch {
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Called with the new value.
    pub fn on_change(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Switch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let focus: FocusHandle = window.use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle()).read(cx).clone();
        let on = self.checked;
        let next = !on;
        let (track_border, thumb) = match (self.disabled, on) {
            (true, _) => (p.line, p.fg_faint),
            (false, true) => (p.accent, p.accent_fg),
            (false, false) => (p.line_strong, p.fg_dim),
        };

        // The thumb travels in eased steps rather than jumping: 2px (off) to
        // 20px (on) inside the 34px track.
        let travel = crate::animate::play_on_change((self.id.clone(), "thumb"), on, crate::motion::FAST, window, cx).eased();
        let (from, to) = if on { (2., 20.) } else { (20., 2.) };
        let thumb_x = (from + (to - from) * travel).round();
        let track = div()
            .relative()
            .w(px(36.))
            .h(px(18.))
            .flex_shrink_0()
            .border_1()
            .border_color(hsla(track_border))
            .bg(hsla(if on && !self.disabled { p.accent } else { p.sunken }))
            .when(!on, |el| {
                el.child(
                    div()
                        .absolute()
                        .inset_0()
                        .child(dither(dither::flat(dither::level::LIGHT)).ink(hsla(p.line)).size_full()),
                )
            })
            .child(
                div()
                    .absolute()
                    .top(px(2.))
                    .size(px(12.))
                    .bg(hsla(thumb))
                    .left(px(thumb_x)),
            );

        div()
            .id(self.id.clone())
            .role(Role::Switch)
            .aria_toggled(if on { Toggled::True } else { Toggled::False })
            .when_some(self.label.clone(), |el, l| el.aria_label(l))
            .track_focus(&focus)
            .when(!self.disabled, |el| el.tab_stop(true))
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .px_1()
            .border_1()
            .border_color(gpui::transparent_black())
            .when(!self.disabled, |el| el.focus_visible(|s| s.border_color(hsla(p.accent))))
            .child(track)
            .child(
                div()
                    .display(Scale::X1, window)
                    // Three display cells (a cell is half its height), so
                    // ON/OFF never reflows the label beside it.
                    .w(display_size(Scale::X1, window) * 1.5)
                    .text_color(hsla(if self.disabled { p.fg_faint } else if on { p.accent_text } else { p.fg_dim }))
                    .child({
                        let text = if on { "ON" } else { "OFF" };
                        let p = crate::animate::play_on_change(ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), "readout".into()), on, crate::motion::FAST, window, cx);
                        crate::animate::scramble(text, p)
                    }),
            )
            .when_some(self.label, |el, l| {
                el.child(div().body(text::BASE).text_color(hsla(if self.disabled { p.fg_faint } else { p.fg })).child(l))
            })
            .when(!self.disabled, |el| el.on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default()))
            .when_some(self.on_change.filter(|_| !self.disabled), |el, handler| {
                el.on_click(move |_, window, cx| handler(&next, window, cx))
            })
    }
}
