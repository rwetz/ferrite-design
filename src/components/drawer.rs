//! The drawer: a modal panel pinned to one side of the window, over a
//! screen-doored app — details, an inspector, a long form, settings.
//!
//! ```text
//!  ░░░░░░░░░░░░░░░░░░░░░░░░░┃ [ DETAILS ] ░░░░░░░ [x] │
//!  ░░ the app, screen-doored ┃ name     ferrite-atlas   │
//!  ░░░░░░░░░░░░░░░░░░░░░░░░░┃ pid      4412            │
//!  ░░░░░░░░░░░░░░░░░░░░░░░░░┃                          │
//!  ░░░░░░░░░░░░░░░░░░░░░░░░░┃            [ CLOSE ESC ] │
//! ```
//!
//! Controlled like [`dialog`](super::dialog): the app owns `open`; Esc, a
//! click on the backdrop, and the close button call `on_close`. Opening
//! remembers focus and focuses the drawer; closing restores it. It wipes in
//! from its edge with an amber scan line (exits are instant).

use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FocusHandle, InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels,
    RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled, Window, anchored, deferred, div, point,
    prelude::FluentBuilder as _, px,
};

use super::overlay::scrim;
use crate::animate::{self, Edge};
use crate::dither::{self, dither};
use crate::fonts::{FerriteText, Scale};
use crate::icon::{Icon, icon};
use crate::theme::palette;
use crate::tokens::hsla;

type Handler = Rc<dyn Fn(&mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct Drawer {
    id: ElementId,
    open: bool,
    title: SharedString,
    side: Edge,
    width: Pixels,
    children: Vec<AnyElement>,
    footer: Vec<AnyElement>,
    on_close: Option<Handler>,
}

pub fn drawer(id: impl Into<ElementId>) -> Drawer {
    Drawer {
        id: id.into(),
        open: false,
        title: "Details".into(),
        side: Edge::Right,
        width: px(380.),
        children: Vec::new(),
        footer: Vec::new(),
        on_close: None,
    }
}

impl Drawer {
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Pin to the left edge instead of the right.
    pub fn left(mut self) -> Self {
        self.side = Edge::Left;
        self
    }

    pub fn width(mut self, width: Pixels) -> Self {
        self.width = width;
        self
    }

    /// Something for the footer bar (buttons, right-aligned).
    pub fn footer(mut self, el: impl IntoElement) -> Self {
        self.footer.push(el.into_any_element());
        self
    }

    pub fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for Drawer {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

struct DrawerState {
    focus: FocusHandle,
    was_open: bool,
    previous: Option<FocusHandle>,
}

impl RenderOnce for Drawer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let state = window.use_keyed_state(self.id.clone(), cx, |_, cx| DrawerState { focus: cx.focus_handle(), was_open: false, previous: None });
        let focus = state.read(cx).focus.clone();
        let was_open = state.read(cx).was_open;
        // Remember and restore focus across open/close, on the next tick.
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
        let run_close = {
            let close = close.clone();
            move |window: &mut Window, cx: &mut App| {
                if let Some(c) = &close {
                    c(window, cx);
                }
            }
        };

        let header = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .h(px(40.))
            .px_3()
            .bg(hsla(p.surface))
            .border_b_1()
            .border_color(hsla(p.line))
            .child(div().flex_none().display(Scale::X1, window).text_color(hsla(p.fg)).child(format!("[ {} ]", self.title.to_uppercase())))
            .child(dither(dither::flat(dither::level::LIGHT)).ink(hsla(p.line_strong)).flex_1().h(px(8.)))
            .child(
                div()
                    .id("drawer-close")
                    .role(Role::Button)
                    .aria_label("Close")
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(24.))
                    .hover(|s| s.bg(hsla(p.line)))
                    .child(icon(Icon::Close).fit(px(16.)).color(hsla(p.fg_dim)))
                    .on_click({
                        let run_close = run_close.clone();
                        move |_, window, cx| run_close(window, cx)
                    }),
            );
        let panel = div()
            .id("drawer")
            .role(Role::Dialog)
            .aria_label(self.title.clone())
            .track_focus(&focus)
            .occlude()
            .flex()
            .flex_col()
            .w(self.width)
            .h_full()
            .bg(hsla(p.raised))
            .border_1()
            .border_color(hsla(p.line_strong))
            .on_key_down({
                let run_close = run_close.clone();
                move |ev, window, cx| {
                    if ev.keystroke.key == "escape" {
                        cx.stop_propagation();
                        run_close(window, cx);
                    }
                }
            })
            .child(header)
            .child(div().id("drawer-body").flex().flex_col().flex_1().min_h_0().overflow_y_scroll().gap_3().p_4().children(self.children))
            .when(!self.footer.is_empty(), |el| {
                el.child(
                    div()
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
                        .children(self.footer),
                )
            });

        let open = animate::play("open", 0u8, crate::motion::BASE, window, cx);
        let side = self.side;
        let panel = if open.done { panel.into_any_element() } else { animate::wipe(panel, open.eased(), side).edge(hsla(p.accent)).into_any_element() };
        let viewport = window.viewport_size();
        root.child(
            deferred(
                anchored().position(point(px(0.), px(0.))).child(
                    div()
                        .id("drawer-layer")
                        .w(viewport.width)
                        .h(viewport.height)
                        .occlude()
                        .child(
                            div()
                                .id("drawer-scrim")
                                .absolute()
                                .inset_0()
                                .on_mouse_down(MouseButton::Left, move |_, window, cx| run_close(window, cx))
                                .child(scrim(open.eased(), cx)),
                        )
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .bottom_0()
                                .map(|el| match side {
                                    Edge::Left => el.left_0(),
                                    Edge::Right => el.right_0(),
                                })
                                .child(panel),
                        ),
                ),
            )
            .with_priority(2),
        )
    }
}
