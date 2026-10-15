//! Scrolling: the Ferrite scrollbar, a scroll area that carries one, and a
//! virtual list for long data.
//!
//! ```text
//!  │ row                │▐│   track: `surface` with a 1px hairline
//!  │ row                │█│   thumb: square `line_strong` block —
//!  │ row                │█│          `fg_faint` on hover, amber while dragged
//!  │ row                │▐│
//! ```
//!
//! gpui scrolls but draws no scrollbar. These read the scroll position at
//! paint time (so they track wheel scrolling without re-rendering the view),
//! and support dragging the thumb (pointer captured) or clicking the track
//! to jump there.
//!
//! - [`scroll_area`]: a vertically scrolling box with a scrollbar. Use for
//!   anything up to a few thousand elements, and for every window body that
//!   isn't one fixed-size picture: `.fill()` lays the children out in a
//!   column at least as tall as the window (so `flex_1` children take the
//!   spare room), `.centered()` also centres them, and either scrolls from
//!   the top once the window is shorter than the content.
//! - [`virtual_list`]: renders only the visible rows of a uniform-height
//!   list (gpui's `uniform_list`), with the same scrollbar. Use for long
//!   data — logs, file lists, tables of thousands of rows.
//! - [`scrollbar`]: the bar alone, for a scroll container you built.

use std::ops::Range;
use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, ElementId, HitboxBehavior, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, MouseMoveEvent, ParentElement, Pixels, RenderOnce, ScrollHandle,
    StatefulInteractiveElement, StyleRefinement, Styled, UniformListScrollHandle, Window, canvas,
    div, fill, point, prelude::FluentBuilder as _, px, size, uniform_list,
};

use crate::theme::palette;
use crate::tokens::hsla;

/// Track width.
pub const WIDTH: Pixels = px(10.);
/// The thumb never gets shorter than this.
const MIN_THUMB: f32 = 24.;

/// Thumb position and length along a track, from scroll metrics. Pure.
/// Returns `None` when there's nothing to scroll.
pub fn thumb(track: f32, viewport: f32, max_offset: f32, offset: f32) -> Option<(f32, f32)> {
    if max_offset <= 0.5 || track <= 0. {
        return None;
    }
    let len = (track * viewport / (viewport + max_offset)).clamp(MIN_THUMB.min(track), track);
    let t = (offset / max_offset).clamp(0., 1.);
    Some(((track - len) * t, len))
}

/// The scroll offset that puts the thumb's top at `top` along the track.
fn offset_for(top: f32, track: f32, len: f32, max_offset: f32) -> f32 {
    let travel = (track - len).max(1.);
    (top / travel).clamp(0., 1.) * max_offset
}

#[derive(Default)]
struct BarState {
    /// Grab point within the thumb while dragging.
    drag: Option<f32>,
    hovered: bool,
}

/// A vertical scrollbar for `handle`. Place it over the right edge of the
/// scroll container's parent (`relative()`); it positions itself.
#[derive(IntoElement)]
pub struct Scrollbar {
    id: ElementId,
    handle: ScrollHandle,
}

pub fn scrollbar(id: impl Into<ElementId>, handle: &ScrollHandle) -> Scrollbar {
    Scrollbar { id: id.into(), handle: handle.clone() }
}

impl RenderOnce for Scrollbar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| BarState::default());
        let handle = self.handle;
        div().id(self.id).absolute().top_0().right_0().bottom_0().w(WIDTH).child(
            canvas(
                |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
                move |bounds: Bounds<Pixels>, hitbox, window, cx| {
                    let p = palette(cx);
                    let track = f32::from(bounds.size.height);
                    let viewport = f32::from(handle.bounds().size.height);
                    let max = f32::from(handle.max_offset().y);
                    let Some((top, len)) = thumb(track, viewport, max, -f32::from(handle.offset().y)) else {
                        return;
                    };
                    let (dragging, hovered) = {
                        let s = state.read(cx);
                        (s.drag.is_some(), s.hovered)
                    };

                    // Track and thumb.
                    window.paint_quad(fill(bounds, hsla(p.surface)));
                    window.paint_quad(fill(Bounds::new(bounds.origin, size(px(1.), bounds.size.height)), hsla(p.line)));
                    let ink = if dragging { p.accent } else if hovered { p.fg_faint } else { p.line_strong };
                    window.paint_quad(fill(
                        Bounds::new(point(bounds.left() + px(2.), bounds.top() + px(top)), size(bounds.size.width - px(3.), px(len))),
                        hsla(ink),
                    ));

                    let apply = {
                        let handle = handle.clone();
                        move |y: Pixels, grab: f32, window: &mut Window| {
                            let at = f32::from(y - bounds.top()) - grab;
                            let off = offset_for(at, track, len, max);
                            handle.set_offset(point(handle.offset().x, px(-off)));
                            window.refresh();
                        }
                    };
                    {
                        let state = state.clone();
                        let hitbox = hitbox.clone();
                        let apply = apply.clone();
                        window.on_mouse_event(move |ev: &MouseDownEvent, phase, window, cx| {
                            if !phase.bubble() || ev.button != MouseButton::Left || !hitbox.is_hovered(window) {
                                return;
                            }
                            cx.stop_propagation();
                            window.prevent_default();
                            window.capture_pointer(hitbox.id);
                            let y = f32::from(ev.position.y - bounds.top());
                            // On the thumb: grab where clicked. On the track: centre the thumb there.
                            let grab = if (top..top + len).contains(&y) { y - top } else { len / 2. };
                            state.update(cx, |s, _| s.drag = Some(grab));
                            apply(ev.position.y, grab, window);
                        });
                    }
                    window.on_mouse_event(move |ev: &MouseMoveEvent, phase, window, cx| {
                        if !phase.bubble() {
                            return;
                        }
                        let over = hitbox.is_hovered(window);
                        let (drag, was_over) = {
                            let s = state.read(cx);
                            (s.drag, s.hovered)
                        };
                        if over != was_over {
                            state.update(cx, |s, _| s.hovered = over);
                            window.refresh();
                        }
                        let Some(grab) = drag else { return };
                        if ev.pressed_button != Some(MouseButton::Left) {
                            state.update(cx, |s, _| s.drag = None);
                            window.release_pointer();
                            window.refresh();
                            return;
                        }
                        apply(ev.position.y, grab, window);
                    });
                },
            )
            .size_full(),
        )
    }
}

// ── Scroll area ───────────────────────────────────────────────────────────

/// A vertically scrolling box with a Ferrite scrollbar. Size it like any
/// element (`.flex_1()`, `.h(px(300.))`); children scroll inside.
///
/// For a window body, use `.fill()` or `.centered()` so the content uses a
/// tall window and scrolls in a short one:
///
/// ```ignore
/// scroll_area("body").flex_1().min_h_0().centered().inset(space::ROW).spacing(space::ROW)
///     .child(readout)
///     .child(panel("Scope").flex_1().min_h(px(160.)))   // grows; never below 160
/// ```
///
/// Don't hand-roll this with `justify_center()` on an `overflow_y_scroll()`
/// box: when the content is taller than the box, centring pushes its top
/// above the scroll origin, out of reach (PITFALLS §52).
#[derive(IntoElement)]
pub struct ScrollArea {
    id: ElementId,
    children: Vec<AnyElement>,
    style: StyleRefinement,
    layout: Layout,
    inset: Option<Pixels>,
    spacing: Option<Pixels>,
}

/// How a [`ScrollArea`] lays out its children.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Layout {
    /// As given: the children are the scrolled content.
    Plain,
    /// A column at least as tall as the area.
    Fill,
    /// A column at least as tall as the area, centred in it.
    Centered,
}

pub fn scroll_area(id: impl Into<ElementId>) -> ScrollArea {
    ScrollArea {
        id: id.into(),
        children: Vec::new(),
        style: StyleRefinement::default(),
        layout: Layout::Plain,
        inset: None,
        spacing: None,
    }
}

impl ScrollArea {
    /// Lay the children out in a column that is at least as tall as the
    /// area: `flex_1` children share the spare height, and once the content
    /// is taller than the area it scrolls from the top. Give a growing child
    /// a `min_h(..)` floor so a short window scrolls instead of crushing it.
    pub fn fill(mut self) -> Self {
        self.layout = Layout::Fill;
        self
    }

    /// Like [`fill`](Self::fill), and the column is centred vertically when
    /// it is shorter than the area. Safe at any height: it never centres
    /// content out of reach.
    pub fn centered(mut self) -> Self {
        self.layout = Layout::Centered;
        self
    }

    /// Padding inside the scrolled content (with `fill` or `centered`), so
    /// content scrolls to the window's edge rather than to a margin.
    pub fn inset(mut self, padding: Pixels) -> Self {
        self.inset = Some(padding);
        self.layout = self.layout.max_fill();
        self
    }

    /// Space between the children (with `fill` or `centered`).
    pub fn spacing(mut self, gap: Pixels) -> Self {
        self.spacing = Some(gap);
        self.layout = self.layout.max_fill();
        self
    }
}

impl Layout {
    /// Padding and spacing need the column; asking for them implies `fill`.
    fn max_fill(self) -> Self {
        if self == Layout::Plain { Layout::Fill } else { self }
    }
}

impl ParentElement for ScrollArea {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for ScrollArea {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for ScrollArea {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let handle = window.use_keyed_state(self.id.clone(), cx, |_, _| ScrollHandle::new()).read(cx).clone();
        let mut outer = div();
        *outer.style() = self.style;
        outer
            .relative()
            .overflow_hidden()
            .child(
                div()
                    .id(self.id.clone())
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(&handle)
                    // Room for the bar so it never covers content.
                    .pr(WIDTH)
                    .map(|scroller| match self.layout {
                        Layout::Plain => scroller.children(self.children),
                        layout => scroller.child(
                            div()
                                .flex()
                                .flex_col()
                                // At least the area's height (the scroller's
                                // is definite), so flex_1 children grow; taller
                                // content just makes the column taller.
                                .min_h_full()
                                .when(layout == Layout::Centered, |col| col.justify_center())
                                .when_some(self.inset, |col, p| col.p(p))
                                .when_some(self.spacing, |col, g| col.gap(g))
                                .children(self.children),
                        ),
                    }),
            )
            .child(scrollbar("scrollbar", &handle))
    }
}

// ── Virtual list ──────────────────────────────────────────────────────────

type RowBuilder = Rc<dyn Fn(Range<usize>, &mut Window, &mut App) -> Vec<AnyElement>>;

/// A list that renders only the rows in view. Every row must be the same
/// height (the first row is measured). Give it a size; it fills it.
///
/// ```ignore
/// virtual_list("log", lines.len(), move |range, _, _| {
///     range.map(|i| div().h(px(24.)).child(lines[i].clone()).into_any_element()).collect()
/// })
/// .h(px(320.))
/// ```
#[derive(IntoElement)]
pub struct VirtualList {
    id: ElementId,
    count: usize,
    rows: RowBuilder,
    handle: Option<UniformListScrollHandle>,
    style: StyleRefinement,
}

pub fn virtual_list(
    id: impl Into<ElementId>,
    count: usize,
    rows: impl Fn(Range<usize>, &mut Window, &mut App) -> Vec<AnyElement> + 'static,
) -> VirtualList {
    VirtualList { id: id.into(), count, rows: Rc::new(rows), handle: None, style: StyleRefinement::default() }
}

impl VirtualList {
    /// Drive it yourself (e.g. `scroll_to_item` to follow a selection).
    pub fn track_scroll(mut self, handle: &UniformListScrollHandle) -> Self {
        self.handle = Some(handle.clone());
        self
    }
}

impl Styled for VirtualList {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for VirtualList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let handle = match self.handle {
            Some(h) => h,
            None => window.use_keyed_state(self.id.clone(), cx, |_, _| UniformListScrollHandle::new()).read(cx).clone(),
        };
        let base = handle.0.borrow().base_handle.clone();
        let rows = self.rows;
        let mut outer = div();
        *outer.style() = self.style;
        outer
            .relative()
            .overflow_hidden()
            .child(
                // uniform_list lays each row out as a root: without a width
                // a row's hitbox shrinks to its content and clicks past the
                // text miss it (PITFALLS 45). A full-width box fixes that for
                // every caller.
                uniform_list(self.id, self.count, move |range, window, cx| {
                    rows(range, window, cx).into_iter().map(|row| div().w_full().child(row).into_any_element()).collect()
                })
                    .track_scroll(&handle)
                    .size_full()
                    .pr(WIDTH),
            )
            .child(scrollbar("scrollbar", &base))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padding_and_spacing_imply_a_column() {
        assert_eq!(scroll_area("a").layout, Layout::Plain);
        assert_eq!(scroll_area("a").inset(px(8.)).layout, Layout::Fill);
        assert_eq!(scroll_area("a").centered().spacing(px(8.)).layout, Layout::Centered);
        assert_eq!(scroll_area("a").spacing(px(8.)).centered().layout, Layout::Centered);
    }

    #[test]
    fn thumb_is_proportional_and_travels_the_track() {
        // 300px track over a viewport showing a quarter of the content.
        let (top, len) = thumb(300., 300., 900., 0.).unwrap();
        assert_eq!((top, len), (0., 75.));
        let (top, _) = thumb(300., 300., 900., 900.).unwrap();
        assert_eq!(top, 225.);
        let (top, _) = thumb(300., 300., 900., 450.).unwrap();
        assert_eq!(top, 112.5);
    }

    #[test]
    fn huge_content_keeps_a_grabbable_thumb() {
        let (_, len) = thumb(300., 300., 1_000_000., 0.).unwrap();
        assert_eq!(len, MIN_THUMB);
    }

    #[test]
    fn nothing_to_scroll_means_no_thumb() {
        assert!(thumb(300., 300., 0., 0.).is_none());
    }

    #[test]
    fn dragging_maps_back_to_offsets() {
        assert_eq!(offset_for(0., 300., 75., 900.), 0.);
        assert_eq!(offset_for(225., 300., 75., 900.), 900.);
        assert_eq!(offset_for(1e6, 300., 75., 900.), 900.);
    }
}
