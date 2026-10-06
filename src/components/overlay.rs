//! Floating surfaces: the shared open/close + focus lifecycle, the Ferrite
//! surface (square, 1px frame, hard dithered drop shadow), and [`Popover`].
//! Menus build on the same state (`menu.rs`).
//!
//! The lifecycle is ported from gpui-base's popover:
//! - open state lives in keyed element state, toggled on mouse **down** (so
//!   it is decided before the click-outside handler runs);
//! - opening remembers the focused element and moves focus into the
//!   surface, so Escape and arrow keys reach it; closing gives focus back —
//!   but only if focus is still inside the surface;
//! - Escape and a click outside both close it;
//! - a click on the trigger while open is seen by *both* the trigger and the
//!   click-outside handler, so the trigger only acts if the state still
//!   matches what it rendered with (otherwise it would close then reopen).

use std::rc::Rc;

use gpui::{
    AnyElement, App, Context, ElementId, FocusHandle, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Pixels, Point, RenderOnce, Role, SharedString,
    StatefulInteractiveElement, Styled, Window, anchored, deferred, div, point,
    prelude::FluentBuilder as _, px, relative,
};

use crate::dither::{self, dither};
use crate::fonts::{FerriteText, Scale};
use crate::theme::palette;
use crate::tokens::hsla;

// ── State ─────────────────────────────────────────────────────────────────

pub(crate) struct OverlayState {
    pub(crate) open: bool,
    pub(crate) focus: FocusHandle,
    previous: Option<FocusHandle>,
    /// Highlighted entry (menus).
    pub(crate) highlight: Option<usize>,
    /// Highlights of open submenus, one per level below the root (menus).
    pub(crate) sub: Vec<Option<usize>>,
    /// Window position to open at (context menus); `None` = under the trigger.
    pub(crate) at: Option<Point<Pixels>>,
}

impl OverlayState {
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
        Self { open: false, focus: cx.focus_handle(), previous: None, highlight: None, sub: Vec::new(), at: None }
    }

    pub(crate) fn show(&mut self, at: Option<Point<Pixels>>, highlight: Option<usize>, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            self.previous = window.focused(cx);
        }
        self.open = true;
        self.at = at;
        self.highlight = highlight;
        self.sub.clear();
        self.focus.focus(window, cx);
        cx.notify();
    }

    pub(crate) fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        self.open = false;
        self.highlight = None;
        self.sub.clear();
        if let Some(previous) = self.previous.take()
            && self.focus.contains_focused(window, cx)
        {
            previous.focus(window, cx);
        }
        cx.notify();
    }
}

// ── Surface ───────────────────────────────────────────────────────────────

/// How far the hard shadow is offset, right and down.
const SHADOW: Pixels = px(4.);

/// The Ferrite floating surface: `raised` fill, 1px `line_strong` frame,
/// 0px corners, and instead of a soft shadow a hard-offset dithered one —
/// the drop shadow of a text-mode window.
pub(crate) fn surface(body: impl IntoElement, cx: &App) -> gpui::Div {
    let p = palette(cx);
    // Black dither vanishes on iron, so in dark mode the shadow is drawn in
    // the hairline grey: it reads as a stippled offset edge, not a glow.
    let shadow_ink = hsla(p.line_strong);
    div()
        .relative()
        .child(
            div()
                .absolute()
                .top(SHADOW)
                .left(SHADOW)
                .size_full()
                .child(dither(dither::flat(dither::level::MEDIUM)).ink(shadow_ink).size_full()),
        )
        .child(
            div()
                .relative()
                .bg(hsla(p.raised))
                .border_1()
                .border_color(hsla(p.line_strong))
                .text_color(hsla(p.fg))
                .child(body),
        )
}

/// Modal scrim density: 11 of every 16 cells inked in the page color
/// (~69%), so the app behind reads as present but clearly out of play.
pub(crate) const SCRIM: f32 = 0.6875;

/// The modal backdrop: the app behind screen-doored into the page color.
/// Never a blur, never a tint.
pub(crate) fn scrim(cx: &App) -> impl IntoElement {
    dither(dither::flat(SCRIM)).ink(hsla(palette(cx).bg)).size_full()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    /// Surface's left edge under the trigger's left edge.
    #[default]
    Start,
    /// Surface's right edge under the trigger's right edge.
    End,
}

/// Place `content` floating below the element this is a child of. The
/// caller must be `relative()`.
pub(crate) fn below(align: Align, content: impl IntoElement) -> impl IntoElement {
    let anchor = match align {
        Align::Start => gpui::Anchor::TopLeft,
        Align::End => gpui::Anchor::TopRight,
    };
    div()
        .absolute()
        .top(relative(1.))
        .map(|el| match align {
            Align::Start => el.left_0(),
            Align::End => el.right_0(),
        })
        .child(
            deferred(
                anchored()
                    .anchor(anchor)
                    .offset(point(px(0.), px(4.)))
                    .snap_to_window_with_margin(px(8.))
                    .child(content),
            )
            .with_priority(1),
        )
}

/// Place `content` floating at a window position (context menus).
pub(crate) fn at_point(position: Point<Pixels>, content: impl IntoElement) -> impl IntoElement {
    deferred(anchored().position(position).snap_to_window_with_margin(px(8.)).child(content)).with_priority(1)
}

// ── Popover ───────────────────────────────────────────────────────────────

type ContentBuilder = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;

/// A non-modal floating panel opened by a trigger: filters, quick settings,
/// details. Takes focus while open; Escape or a click outside closes it.
///
/// ```ignore
/// popover("filters")
///     .trigger(Button::new("f").icon(Icon::Sliders).label("Filters"))
///     .title("Filters")
///     .content(|window, cx| div().child(checkbox("x").label("Hide merged")).into_any_element())
/// ```
#[derive(IntoElement)]
pub struct Popover {
    id: ElementId,
    trigger: Option<AnyElement>,
    content: Option<ContentBuilder>,
    title: Option<SharedString>,
    align: Align,
    width: Pixels,
}

pub fn popover(id: impl Into<ElementId>) -> Popover {
    Popover { id: id.into(), trigger: None, content: None, title: None, align: Align::Start, width: px(280.) }
}

impl Popover {
    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    /// Built only while open, on every render.
    pub fn content(mut self, content: impl Fn(&mut Window, &mut App) -> AnyElement + 'static) -> Self {
        self.content = Some(Rc::new(content));
        self
    }

    /// A `[ TITLE ]` header row with the dithered fill, like a panel.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    pub fn width(mut self, width: Pixels) -> Self {
        self.width = width;
        self
    }
}

impl RenderOnce for Popover {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let state = window.use_keyed_state(self.id.clone(), cx, |_, cx| OverlayState::new(cx));
        let open = state.read(cx).open;

        let trigger = div()
            .id("trigger")
            .on_mouse_down(MouseButton::Left, {
                let state = state.clone();
                move |_, window, cx| {
                    cx.stop_propagation();
                    state.update(cx, |s, cx| {
                        if s.open == open {
                            if open { s.close(window, cx) } else { s.show(None, None, window, cx) }
                        }
                    });
                }
            })
            .on_key_down({
                let state = state.clone();
                move |ev, window, cx| {
                    if !open && matches!(ev.keystroke.key.as_str(), "enter" | "space" | "down") {
                        cx.stop_propagation();
                        state.update(cx, |s, cx| s.show(None, None, window, cx));
                    }
                }
            })
            .children(self.trigger);

        let mut root = div().id(self.id.clone()).relative().flex_none().child(trigger);

        if open {
            let focus = state.read(cx).focus.clone();
            let body = div()
                .w(self.width)
                .flex()
                .flex_col()
                .when_some(self.title, |el, title| {
                    el.child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .h(px(24.))
                            .px_2()
                            .bg(hsla(p.surface))
                            .border_b_1()
                            .border_color(hsla(p.line))
                            .child(div().display(Scale::X1, window).child(format!("[ {} ]", title.to_uppercase())))
                            .child(dither(dither::flat(dither::level::LIGHT)).ink(hsla(p.line_strong)).flex_1().h(px(8.))),
                    )
                })
                .child(
                    div()
                        .p_3()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .when_some(self.content, |el, content| el.child(content(window, cx))),
                );
            let panel = div()
                .id("popover-surface")
                .role(Role::Dialog)
                .track_focus(&focus)
                .occlude()
                .on_key_down({
                    let state = state.clone();
                    move |ev, window, cx| {
                        if ev.keystroke.key == "escape" {
                            cx.stop_propagation();
                            state.update(cx, |s, cx| s.close(window, cx));
                        }
                    }
                })
                .on_mouse_down_out({
                    let state = state.clone();
                    move |_, window, cx| state.update(cx, |s, cx| s.close(window, cx))
                })
                .child(surface(body, cx));
            root = root.child(below(self.align, panel));
        }
        root
    }
}
