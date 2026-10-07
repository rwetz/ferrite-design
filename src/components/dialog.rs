//! The dialog: a modal panel over a screen-doored app.
//!
//! ```text
//!  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░   the app, screen-doored
//!  ░░ ┌──────────────────────────────────────┐ ░░
//!  ░░ │ ▲ [ DELETE 3 FILES? ] ░░░░░░░░░░░░░░ │ ░░   [ TITLE ] header
//!  ░░ ├──────────────────────────────────────┤ ░░
//!  ░░ │ This can't be undone.                │ ░░   description + content
//!  ░░ ├──────────────────────────────────────┤ ░░
//!  ░░ │           [ KEEP  ESC ] [ DELETE ⏎ ] │ ░░   cancel + confirm
//!  ░░ └──────────────────────────────────────┘▒░░
//! ```
//!
//! **Controlled:** the app owns `open`. Esc, a click on the backdrop, and
//! the cancel button all call `on_close`; the confirm button (or Enter while
//! the dialog itself has focus) calls `on_confirm`, then `on_close`. Close
//! it by setting your state to `false` in `on_close`.
//!
//! **Focus:** opening remembers what had focus and focuses the dialog;
//! closing gives focus back. Tab is *not* trapped — gpui has no focus scope
//! yet, so Tab can walk out to elements behind the backdrop (they still
//! can't be clicked). Known gap, see COMPONENTS.md.
//!
//! Render it anywhere in the window's tree (it draws nothing while closed
//! and lifts itself above everything while open). For "are you sure?",
//! [`Dialog::danger`] turns the header and confirm button red and the role
//! into an alert dialog.

use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FocusHandle, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Pixels, RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled,
    Window, anchored, deferred, div, point, prelude::FluentBuilder as _, px,
};

use super::button::Button;
use super::overlay::{reveal, scrim, surface};
use crate::dither::{self, dither};
use crate::fonts::{FerriteText, Scale};
use crate::icon::{Icon, icon};
use crate::theme::palette;
use crate::tokens::{hsla, text};

type Handler = Rc<dyn Fn(&mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct Dialog {
    id: ElementId,
    open: bool,
    title: SharedString,
    description: Option<SharedString>,
    children: Vec<AnyElement>,
    confirm: Option<(SharedString, Handler)>,
    cancel: SharedString,
    danger: bool,
    width: Pixels,
    on_close: Option<Handler>,
}

pub fn dialog(id: impl Into<ElementId>) -> Dialog {
    Dialog {
        id: id.into(),
        open: false,
        title: SharedString::default(),
        description: None,
        children: Vec::new(),
        confirm: None,
        cancel: "Cancel".into(),
        danger: false,
        width: px(440.),
        on_close: None,
    }
}

impl Dialog {
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// A line or two of body text under the header.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// The primary action. Without one the dialog shows only a close button.
    pub fn confirm(mut self, label: impl Into<SharedString>, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.confirm = Some((label.into(), Rc::new(handler)));
        self
    }

    /// Label for the cancel button (default "Cancel").
    pub fn cancel(mut self, label: impl Into<SharedString>) -> Self {
        self.cancel = label.into();
        self
    }

    /// A destructive confirmation: red header and confirm button, alert role.
    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }

    pub fn width(mut self, width: Pixels) -> Self {
        self.width = width;
        self
    }

    /// Esc, backdrop click, cancel — and after confirm. Set `open` to false here.
    pub fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for Dialog {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

struct DialogState {
    focus: FocusHandle,
    was_open: bool,
    previous: Option<FocusHandle>,
}

impl RenderOnce for Dialog {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let state = window.use_keyed_state(self.id.clone(), cx, |_, cx| DialogState { focus: cx.focus_handle(), was_open: false, previous: None });
        let focus = state.read(cx).focus.clone();
        let was_open = state.read(cx).was_open;

        // Open/close transitions: remember and restore focus. Focus moves on
        // the next tick — not in the middle of drawing.
        if self.open && !was_open {
            let previous = window.focused(cx);
            state.update(cx, |s, _| {
                s.was_open = true;
                s.previous = previous;
            });
            let focus = focus.clone();
            window.defer(cx, move |window, cx| focus.focus(window, cx));
        } else if !self.open && was_open {
            let previous = state.update(cx, |s, _| {
                s.was_open = false;
                s.previous.take()
            });
            if let Some(previous) = previous {
                window.defer(cx, move |window, cx| previous.focus(window, cx));
            }
        }

        let root = div().id(self.id.clone());
        if !self.open {
            return root;
        }

        let close = self.on_close.clone();
        let confirm = self.confirm.as_ref().map(|(_, h)| h.clone());
        let run_confirm: Option<Handler> = confirm.map(|confirm| {
            let close = close.clone();
            Rc::new(move |window: &mut Window, cx: &mut App| {
                confirm(window, cx);
                if let Some(close) = &close {
                    close(window, cx);
                }
            }) as Handler
        });

        let (title_ink, rule_ink) = if self.danger { (p.danger, p.danger) } else { (p.fg, p.line_strong) };
        let header = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .h(px(32.))
            .px_3()
            .bg(hsla(p.surface))
            .border_b_1()
            .border_color(hsla(p.line))
            .when(self.danger, |el| {
                // Three hard flashes, then solid.
                let flash = crate::animate::play("warn", 0u8, crate::motion::SLOW, window, cx);
                let lit = flash.done || (flash.frame / 2).is_multiple_of(2);
                el.child(div().size(px(20.)).when(lit, |el| el.child(icon(Icon::Warning).fit(px(20.)).color(hsla(p.danger)))))
            })
            .child(
                div()
                    .flex_none()
                    .display(Scale::X1, window)
                    .text_color(hsla(title_ink))
                    .child(format!("[ {} ]", self.title.to_uppercase())),
            )
            .child(dither(dither::flat(dither::level::LIGHT)).ink(hsla(rule_ink)).flex_1().h(px(8.)));

        let body = div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .when_some(self.description, |el, d| el.child(div().body(text::BASE).text_color(hsla(p.fg_dim)).child(d)))
            .children(self.children);

        let footer = div()
            .flex()
            .flex_row()
            .items_center()
            .justify_end()
            .gap_2()
            .h(px(48.))
            .px_3()
            .bg(hsla(p.surface))
            .border_t_1()
            .border_color(hsla(p.line))
            .child(
                Button::new("dialog-cancel")
                    .label(if self.confirm.is_some() { self.cancel.clone() } else { "Close".into() })
                    .secondary()
                    .shortcut("Esc")
                    .when_some(close.clone(), |b, close| b.on_click(move |_, window, cx| close(window, cx))),
            )
            .when_some(self.confirm.as_ref().map(|(l, _)| l.clone()).zip(run_confirm.clone()), |el, (label, run)| {
                let b = Button::new("dialog-confirm").label(label).shortcut("Enter");
                let b = if self.danger { b.danger() } else { b.primary() };
                el.child(b.on_click(move |_, window, cx| run(window, cx)))
            });

        let panel = div()
            .id("dialog")
            .role(if self.danger { Role::AlertDialog } else { Role::Dialog })
            .aria_label(self.title.clone())
            .track_focus(&focus)
            .occlude()
            .on_key_down({
                let focus = focus.clone();
                let close = close.clone();
                move |ev, window, cx| match ev.keystroke.key.as_str() {
                    "escape" => {
                        cx.stop_propagation();
                        if let Some(close) = &close {
                            close(window, cx);
                        }
                    }
                    // Only when the dialog itself is focused: a focused
                    // button handles its own Enter.
                    "enter" if focus.is_focused(window) => {
                        cx.stop_propagation();
                        if let Some(run) = &run_confirm {
                            run(window, cx);
                        }
                    }
                    _ => {}
                }
            })
            .child(reveal(surface(div().w(self.width).flex().flex_col().child(header).child(body).child(footer), cx), crate::motion::BASE, window, cx));

        let viewport = window.viewport_size();
        let open = crate::animate::play("open", 0u8, crate::motion::BASE, window, cx);
        root.child(
            deferred(
                anchored().position(point(px(0.), px(0.))).child(
                    div()
                        .id("dialog-layer")
                        .w(viewport.width)
                        .h(viewport.height)
                        .occlude()
                        .child(
                            div()
                                .id("dialog-scrim")
                                .absolute()
                                .inset_0()
                                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                    if let Some(close) = &close {
                                        close(window, cx);
                                    }
                                })
                                .child(scrim(open.eased(), cx)),
                        )
                        .child(
                            div()
                                .absolute()
                                .inset_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                // The layer itself ignores clicks; only the panel takes them.
                                .child(panel),
                        ),
                ),
            )
            .with_priority(super::overlay::layer::DIALOG),
        )
    }
}
