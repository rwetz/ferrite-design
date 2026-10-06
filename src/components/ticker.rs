//! Timer-driven periodic state, and the components built on it.
//!
//! Anything that changes on a schedule (a blinking caret, a spinner) uses a
//! [`ticker`] instead of a repeating animation: the window redraws only when
//! the content actually changes, at the content's own rate (PITFALLS §17).

use std::time::Duration;

use gpui::{
    App, ElementId, Hsla, IntoElement, ParentElement, RenderOnce, Styled, Task, Window, div,
    prelude::FluentBuilder as _,
};

use crate::ascii;
use crate::fonts::{FerriteText, Scale};
use crate::motion;
use crate::theme::palette;
use crate::tokens::hsla;

struct Ticker {
    count: u64,
    _task: Task<()>,
}

/// A counter keyed to `id` that increments every `period` and re-renders the
/// view that called it. Lives as long as the element is rendered in
/// consecutive frames. Returns 0 forever under reduced motion.
///
/// Must be called while an element is rendering (inside `RenderOnce::render`
/// or a view's `render`).
pub fn ticker(id: impl Into<ElementId>, period: Duration, window: &mut Window, cx: &mut App) -> u64 {
    if motion::reduced(cx) {
        return 0;
    }
    let state = window.use_keyed_state(id, cx, |_, cx| Ticker {
        count: 0,
        _task: cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(period).await;
                let alive = this.update(cx, |t: &mut Ticker, cx| {
                    t.count += 1;
                    cx.notify();
                });
                if alive.is_err() {
                    break;
                }
            }
        }),
    });
    state.read(cx).count
}

// ── Cursor ────────────────────────────────────────────────────────────────

/// A blinking block caret, the "live" marker. Square-wave, not a fade, and
/// solid under reduced motion. Redraws 2×/s.
#[derive(IntoElement)]
pub struct Cursor {
    id: ElementId,
}

pub fn cursor(id: impl Into<ElementId>) -> Cursor {
    Cursor { id: id.into() }
}

impl RenderOnce for Cursor {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let on = ticker(self.id, motion::BLINK / 2, window, cx).is_multiple_of(2);
        div()
            .display(Scale::X1, window)
            .text_color(hsla(p.accent))
            .when(!on, |el| el.opacity(0.))
            .child("█")
    }
}

// ── Spinner ───────────────────────────────────────────────────────────────

/// The text-mode `| / - \` spinner. Static `|` under reduced motion.
#[derive(IntoElement)]
pub struct Spinner {
    id: ElementId,
    color: Option<Hsla>,
}

pub fn spinner(id: impl Into<ElementId>) -> Spinner {
    Spinner { id: id.into(), color: None }
}

impl Spinner {
    /// Ink color (default: `fg_dim`).
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

impl RenderOnce for Spinner {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let n = ticker(self.id, motion::FRAME * 3, window, cx);
        div()
            .display(Scale::X1, window)
            .text_color(self.color.unwrap_or_else(|| hsla(p.fg_dim)))
            .child(ascii::SPINNER[n as usize % ascii::SPINNER.len()].to_string())
    }
}
