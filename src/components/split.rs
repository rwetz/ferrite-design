//! The split pane: two regions with a draggable 1px divider between them.
//!
//! ```text
//!  ┌──────────┬───────────────────────┐
//!  │ files    │ editor                │   divider: 1px `line_strong`,
//!  │          ║                       │   amber while hovered or dragged,
//!  │          │                       │   7px grab zone, resize cursor
//!  └──────────┴───────────────────────┘
//! ```
//!
//! The split is the pane's own state (seed it with `.initial(..)`), kept as
//! a fraction so it survives window resizes, and clamped so neither side
//! gets smaller than `.min(..)`. Double-click the divider to reset it.

use gpui::{
    AnyElement, App, Bounds, CursorStyle, ElementId, HitboxBehavior, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, ParentElement, Pixels, RenderOnce,
    StyleRefinement, Styled, Window, canvas, div, prelude::FluentBuilder as _, px, relative,
};

use crate::theme::palette;
use crate::tokens::hsla;

/// Half the divider's grab zone.
const GRAB: f32 = 3.;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// Side by side; the divider is vertical.
    Horizontal,
    /// Stacked; the divider is horizontal.
    Vertical,
}

#[derive(IntoElement)]
pub struct Split {
    id: ElementId,
    axis: Axis,
    first: Option<AnyElement>,
    second: Option<AnyElement>,
    initial: f32,
    min: Pixels,
    style: StyleRefinement,
}

/// Side by side. Use [`Split::vertical`] to stack.
pub fn split(id: impl Into<ElementId>) -> Split {
    Split { id: id.into(), axis: Axis::Horizontal, first: None, second: None, initial: 0.3, min: px(120.), style: StyleRefinement::default() }
}

impl Split {
    pub fn vertical(mut self) -> Self {
        self.axis = Axis::Vertical;
        self
    }

    pub fn first(mut self, el: impl IntoElement) -> Self {
        self.first = Some(el.into_any_element());
        self
    }

    pub fn second(mut self, el: impl IntoElement) -> Self {
        self.second = Some(el.into_any_element());
        self
    }

    /// Starting share of the first pane, 0–1 (default 0.3).
    pub fn initial(mut self, fraction: f32) -> Self {
        self.initial = fraction.clamp(0., 1.);
        self
    }

    /// Smallest either pane may get (default 120px).
    pub fn min(mut self, min: Pixels) -> Self {
        self.min = min;
        self
    }
}

impl Styled for Split {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// Clamp a requested first-pane length so both panes keep `min`. Pure.
pub fn clamp_split(want: f32, total: f32, min: f32) -> f32 {
    if total <= 2. * min {
        return total / 2.;
    }
    want.clamp(min, total - min)
}

struct SplitState {
    fraction: f32,
    initial: f32,
    /// Length along the axis available to the panes, from the last layout.
    total: f32,
    /// While dragging: pointer position and first-pane length at the press.
    drag: Option<(f32, f32)>,
    hovered: bool,
}

impl RenderOnce for Split {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let initial = self.initial;
        let state = window.use_keyed_state(self.id.clone(), cx, move |_, _| SplitState { fraction: initial, initial, total: 0., drag: None, hovered: false });
        let (fraction, lit) = {
            let s = state.read(cx);
            (s.fraction, s.drag.is_some() || s.hovered)
        };
        let horizontal = self.axis == Axis::Horizontal;
        let min = f32::from(self.min);

        // Caller's style (size, border) first; the split's own layout on top.
        let mut root = div();
        *root.style() = self.style;
        let root = root.id(self.id.clone()).relative().flex().when(horizontal, |el| el.flex_row()).when(!horizontal, |el| el.flex_col());

        let first = div()
            .relative()
            .overflow_hidden()
            .flex_none()
            .map(|el| if horizontal { el.h_full().w(relative(fraction)) } else { el.w_full().h(relative(fraction)) })
            .children(self.first);
        let second = div().relative().overflow_hidden().flex_1().min_w_0().min_h_0().children(self.second);

        // The grab zone hangs off the divider itself, so it is always exactly
        // where the line is drawn; drags are measured from the press point.
        let zone = canvas(
            |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
            {
                let state = state.clone();
                move |_, hitbox, window, _| {
                    window.set_cursor_style(if horizontal { CursorStyle::ResizeLeftRight } else { CursorStyle::ResizeUpDown }, &hitbox);
                    let along = move |pt: gpui::Point<Pixels>| f32::from(if horizontal { pt.x } else { pt.y });
                    {
                        let state = state.clone();
                        let hitbox = hitbox.clone();
                        window.on_mouse_event(move |ev: &MouseDownEvent, phase, window, cx| {
                            if !phase.bubble() || ev.button != MouseButton::Left || !hitbox.is_hovered(window) {
                                return;
                            }
                            cx.stop_propagation();
                            window.prevent_default();
                            if ev.click_count == 2 {
                                state.update(cx, |s, _| s.fraction = s.initial);
                            } else {
                                window.capture_pointer(hitbox.id);
                                state.update(cx, |s, _| s.drag = Some((along(ev.position), s.fraction * s.total)));
                            }
                            window.refresh();
                        });
                    }
                    window.on_mouse_event(move |ev: &MouseMoveEvent, phase, window, cx| {
                        if !phase.bubble() {
                            return;
                        }
                        let over = hitbox.is_hovered(window);
                        let (drag, was_over, total) = {
                            let s = state.read(cx);
                            (s.drag, s.hovered, s.total)
                        };
                        if over != was_over {
                            state.update(cx, |s, _| s.hovered = over);
                            window.refresh();
                        }
                        let Some((from, len)) = drag else { return };
                        if ev.pressed_button != Some(MouseButton::Left) {
                            state.update(cx, |s, _| s.drag = None);
                            window.release_pointer();
                            window.refresh();
                            return;
                        }
                        let len = clamp_split(len + along(ev.position) - from, total, min);
                        state.update(cx, |s, _| s.fraction = len / total.max(1.));
                        window.refresh();
                    });
                }
            },
        )
        .absolute()
        .map(|el| {
            if horizontal {
                el.top_0().bottom_0().left(px(-GRAB)).w(px(GRAB * 2. + 1.))
            } else {
                el.left_0().right_0().top(px(-GRAB)).h(px(GRAB * 2. + 1.))
            }
        });
        let divider = div()
            .relative()
            .flex_none()
            .bg(hsla(if lit { p.accent } else { p.line_strong }))
            .map(|el| if horizontal { el.w(px(1.)).h_full() } else { el.h(px(1.)).w_full() })
            .child(zone);

        // Measures the space the panes share, for converting drags to fractions.
        let measure = canvas(
            move |bounds: Bounds<Pixels>, _, cx| {
                let total = f32::from(if horizontal { bounds.size.width } else { bounds.size.height }) - 1.;
                state.update(cx, |s, _| s.total = total);
            },
            |_, _, _, _| {},
        )
        .absolute()
        .inset_0();

        root.child(measure).child(first).child(divider).child(second)
    }
}

#[cfg(test)]
mod tests {
    use super::clamp_split;

    #[test]
    fn both_panes_keep_their_minimum() {
        assert_eq!(clamp_split(50., 1000., 120.), 120.);
        assert_eq!(clamp_split(990., 1000., 120.), 880.);
        assert_eq!(clamp_split(400., 1000., 120.), 400.);
        // Too small for both minimums: split evenly.
        assert_eq!(clamp_split(10., 200., 120.), 100.);
    }
}
