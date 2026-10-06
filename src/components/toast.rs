//! Toasts: short, non-modal notifications stacked in the bottom-right corner.
//!
//! ```text
//!                         ┌──────────────────────────────────┐
//!                         ▌ ✓  OK  Deployed            [x] │   tone bar, icon, kind
//!                         │      staging · build 4412        │   message
//!                         │      [ UNDO ]                    │   optional action
//!                         ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀────────────────┘▒  countdown, stepped
//!                          ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒   hard dithered shadow
//! ```
//!
//! - **Kinds:** `info` (amber), `success`, `warning`, `danger` — each with a
//!   pixel icon and a display-face code (`INFO OK WARN ERR`).
//! - **Lifetime:** 4s (info, success), 6s (warning), 8s (danger), or
//!   `.sticky()`. The countdown drains in 16 discrete steps along the bottom
//!   edge. Pointing at the stack pauses every toast in it.
//! - **Entry:** materialises through three dither frames (▓ ▒ ░), instant
//!   under reduced motion. Dismissal is instant.
//! - **Stack:** newest nearest the corner, at most [`MAX_VISIBLE`]; older
//!   ones are dropped.
//! - **Time** is driven by one timer that runs only while a toast is on
//!   screen and redraws only when something visible changes (PITFALLS §17).
//! - **Actions** close the toast, then run on the next tick, like palette
//!   commands (PITFALLS §26).
//!
//! One `Toaster` per window, rendered as a child of the root view:
//!
//! ```ignore
//! let toaster = cx.new(|_| Toaster::new());
//! toaster.update(cx, |t, cx| t.push(toast("Deployed").success().message("staging"), cx));
//! ```

use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{
    App, Context, Hsla, InteractiveElement, IntoElement, ParentElement, Pixels, Render, Role,
    SharedString, StatefulInteractiveElement, Styled, Window, anchored, deferred, div, point,
    prelude::FluentBuilder as _, px, relative,
};

use super::button::Button;
use super::overlay::surface;
use crate::dither::{self, dither};
use crate::fonts::{FerriteText, Scale};
use crate::icon::{Icon, icon};
use crate::motion;
use crate::theme::palette;
use crate::tokens::{Palette, hsla, text};

/// The most toasts on screen at once.
pub const MAX_VISIBLE: usize = 4;
/// Countdown resolution.
pub const SEGMENTS: u32 = 16;
/// Entry frames: dither levels laid over the toast in its own fill.
const ENTRY: [f32; 3] = [dither::level::DARK, dither::level::MEDIUM, dither::level::LIGHT];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToastKind {
    #[default]
    Info,
    Success,
    Warning,
    Danger,
}

impl ToastKind {
    fn code(self) -> &'static str {
        match self {
            ToastKind::Info => "INFO",
            ToastKind::Success => "OK",
            ToastKind::Warning => "WARN",
            ToastKind::Danger => "ERR",
        }
    }

    fn icon(self) -> Icon {
        match self {
            ToastKind::Info => Icon::Dot,
            ToastKind::Success => Icon::Check,
            ToastKind::Warning => Icon::Warning,
            ToastKind::Danger => Icon::Close,
        }
    }

    /// (tone for bar/icon/countdown, tone for text on `raised`).
    fn tones(self, p: &Palette) -> (Hsla, Hsla) {
        match self {
            ToastKind::Info => (hsla(p.accent), hsla(p.accent_text)),
            ToastKind::Success => (hsla(p.success), hsla(p.success)),
            ToastKind::Warning => (hsla(p.warning), hsla(p.warning)),
            ToastKind::Danger => (hsla(p.danger), hsla(p.danger)),
        }
    }

    pub fn default_duration(self) -> Duration {
        Duration::from_secs(match self {
            ToastKind::Info | ToastKind::Success => 4,
            ToastKind::Warning => 6,
            ToastKind::Danger => 8,
        })
    }
}

type ActionHandler = Rc<dyn Fn(&mut Window, &mut App)>;

/// A notification. Build with [`toast`], show with [`Toaster::push`].
#[derive(Clone)]
pub struct Toast {
    title: SharedString,
    message: Option<SharedString>,
    kind: ToastKind,
    duration: Option<Duration>,
    sticky: bool,
    action: Option<(SharedString, ActionHandler)>,
}

pub fn toast(title: impl Into<SharedString>) -> Toast {
    Toast { title: title.into(), message: None, kind: ToastKind::Info, duration: None, sticky: false, action: None }
}

impl Toast {
    pub fn kind(mut self, kind: ToastKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn success(self) -> Self {
        self.kind(ToastKind::Success)
    }

    pub fn warning(self) -> Self {
        self.kind(ToastKind::Warning)
    }

    pub fn danger(self) -> Self {
        self.kind(ToastKind::Danger)
    }

    /// A second, dimmer line of detail.
    pub fn message(mut self, message: impl Into<SharedString>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Override the kind's default lifetime.
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Stay until dismissed. For things the user must act on.
    pub fn sticky(mut self) -> Self {
        self.sticky = true;
        self
    }

    /// One button (Undo, Retry, Open). Clicking it dismisses the toast.
    pub fn action(mut self, label: impl Into<SharedString>, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.action = Some((label.into(), Rc::new(handler)));
        self
    }

    fn lifetime(&self) -> Duration {
        self.duration.unwrap_or_else(|| self.kind.default_duration())
    }
}

/// Identifies a pushed toast, for [`Toaster::dismiss`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ToastId(u64);

struct Live {
    id: ToastId,
    toast: Toast,
    remaining: Duration,
    born: Instant,
}

impl Live {
    fn segments(&self) -> u32 {
        countdown_segments(self.remaining, self.toast.lifetime())
    }
}

/// Lit countdown segments for `remaining` of `total`: full at the start,
/// stepping down, and never 0 while time is left.
pub fn countdown_segments(remaining: Duration, total: Duration) -> u32 {
    if total.is_zero() || remaining.is_zero() {
        return 0;
    }
    let f = remaining.as_secs_f32() / total.as_secs_f32();
    ((f * SEGMENTS as f32).ceil() as u32).clamp(1, SEGMENTS)
}

/// The entry overlay for a toast `age` old, or `None` once it's fully in.
fn entry_frame(age: Duration) -> Option<usize> {
    let frame = (age.as_millis() / motion::FRAME.as_millis()) as usize;
    (frame < ENTRY.len()).then_some(frame)
}

/// The per-window toast stack. Draws nothing while empty.
pub struct Toaster {
    toasts: Vec<Live>,
    next_id: u64,
    paused: bool,
    ticking: bool,
    last_tick: Instant,
    bottom: Pixels,
}

impl Default for Toaster {
    fn default() -> Self {
        Self::new()
    }
}

impl Toaster {
    pub fn new() -> Self {
        Self { toasts: Vec::new(), next_id: 0, paused: false, ticking: false, last_tick: Instant::now(), bottom: px(36.) }
    }

    /// Distance from the window's bottom edge (clear a status bar). Default 36px.
    pub fn bottom_inset(mut self, inset: Pixels) -> Self {
        self.bottom = inset;
        self
    }

    pub fn push(&mut self, toast: Toast, cx: &mut Context<Self>) -> ToastId {
        let id = ToastId(self.next_id);
        self.next_id += 1;
        let remaining = toast.lifetime();
        self.toasts.push(Live { id, toast, remaining, born: Instant::now() });
        if self.toasts.len() > MAX_VISIBLE {
            let excess = self.toasts.len() - MAX_VISIBLE;
            self.toasts.drain(..excess);
        }
        self.start_ticking(cx);
        cx.notify();
        id
    }

    pub fn dismiss(&mut self, id: ToastId, cx: &mut Context<Self>) {
        let before = self.toasts.len();
        self.toasts.retain(|t| t.id != id);
        if self.toasts.len() != before {
            cx.notify();
        }
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.toasts.clear();
        cx.notify();
    }

    pub fn len(&self) -> usize {
        self.toasts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.toasts.is_empty()
    }

    fn start_ticking(&mut self, cx: &mut Context<Self>) {
        if self.ticking {
            return;
        }
        self.ticking = true;
        self.last_tick = Instant::now();
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(motion::FRAME).await;
                match this.update(cx, |t, cx| t.tick(cx)) {
                    Ok(true) => {}
                    _ => break,
                }
            }
        })
        .detach();
    }

    /// Advance time; returns whether to keep ticking.
    fn tick(&mut self, cx: &mut Context<Self>) -> bool {
        let now = Instant::now();
        let dt = now - self.last_tick;
        self.last_tick = now;
        let mut changed = false;
        for t in &mut self.toasts {
            // Entry frames redraw on their own schedule.
            let age = now - t.born;
            changed |= entry_frame(age) != entry_frame(age.saturating_sub(dt));
            if self.paused || t.toast.sticky {
                continue;
            }
            let before = t.segments();
            t.remaining = t.remaining.saturating_sub(dt);
            changed |= t.segments() != before;
        }
        let before = self.toasts.len();
        self.toasts.retain(|t| t.toast.sticky || !t.remaining.is_zero());
        changed |= self.toasts.len() != before;
        if changed {
            cx.notify();
        }
        self.ticking = !self.toasts.is_empty();
        self.ticking
    }

    fn act(&mut self, id: ToastId, window: &mut Window, cx: &mut Context<Self>) {
        let handler = self.toasts.iter().find(|t| t.id == id).and_then(|t| t.toast.action.as_ref().map(|a| a.1.clone()));
        self.dismiss(id, cx);
        if let Some(handler) = handler {
            window.defer(cx, move |window, cx| handler(window, cx));
        }
    }
}

impl Render for Toaster {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.toasts.is_empty() {
            return div().into_any_element();
        }
        let p = palette(cx);
        let viewport = window.viewport_size();
        let reduced = motion::reduced(cx);
        let this = cx.entity().downgrade();
        let lead = crate::fonts::display_size(Scale::X1, window);

        let mut stack = div()
            .id("toasts")
            .flex()
            .flex_col()
            .items_end()
            .gap_2()
            // Occlude here, not on each toast: a child's occlusion would hide
            // the pointer from this element's hover, and pausing needs it.
            .occlude()
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                this.paused = *hovered;
                cx.notify();
            }));

        for live in &self.toasts {
            let t = &live.toast;
            let id = live.id;
            let (tone, tone_text) = t.kind.tones(p);
            let lit = if t.sticky { 0 } else { live.segments() };

            let body = div()
                .relative()
                .w(px(360.))
                .flex()
                .flex_col()
                // Tone bar down the left edge.
                .child(div().absolute().left_0().top_0().bottom_0().w(px(2.)).bg(tone))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_start()
                        .gap_2()
                        .pl_3()
                        .pr_1()
                        .py_2()
                        .child(div().size(lead).flex_shrink_0().child(icon(t.kind.icon()).color(tone)))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(
                                    div()
                                        .flex()
                                        .flex_row()
                                        .items_center()
                                        .gap_2()
                                        .min_h(lead)
                                        .child(div().display(Scale::X1, window).text_color(tone_text).child(t.kind.code()))
                                        .child(div().flex_1().min_w_0().body(text::BASE).text_color(hsla(p.fg)).child(t.title.clone())),
                                )
                                .when_some(t.message.clone(), |el, m| {
                                    el.child(div().body(text::SM).text_color(hsla(p.fg_dim)).child(m))
                                })
                                .when_some(t.action.as_ref().map(|a| a.0.clone()), |el, label| {
                                    el.child(
                                        div().pt_1().flex().flex_row().child(
                                            Button::new(("toast-action", id.0)).label(label).secondary().small().on_click({
                                                let this = this.clone();
                                                move |_, window, cx| {
                                                    let _ = this.update(cx, |t, cx| t.act(id, window, cx));
                                                }
                                            }),
                                        ),
                                    )
                                }),
                        )
                        .child(Button::new(("toast-close", id.0)).icon(Icon::Close).ghost().small().tooltip("Dismiss").on_click({
                            let this = this.clone();
                            move |_, _, cx| {
                                let _ = this.update(cx, |t, cx| t.dismiss(id, cx));
                            }
                        })),
                )
                // Countdown: 16 discrete steps, draining right to left.
                .when(!t.sticky, |el| {
                    el.child(
                        div()
                            .flex()
                            .flex_row()
                            .h(px(2.))
                            .bg(hsla(p.line))
                            .child(div().h_full().w(relative(lit as f32 / SEGMENTS as f32)).bg(tone)),
                    )
                });

            // Entry: the toast's own fill dithered over it, thinning out.
            let entry = if reduced { None } else { entry_frame(live.born.elapsed()) };
            let card = div()
                .relative()
                .child(surface(body, cx))
                .when_some(entry, |el, frame| {
                    el.child(div().absolute().inset_0().child(dither(dither::flat(ENTRY[frame])).ink(hsla(p.raised)).size_full()))
                });

            let label: SharedString = match &t.message {
                Some(m) => format!("{} {}: {}", t.kind.code(), t.title, m).into(),
                None => format!("{} {}", t.kind.code(), t.title).into(),
            };
            stack = stack.child(
                div()
                    .id(("toast", id.0))
                    .role(if matches!(t.kind, ToastKind::Warning | ToastKind::Danger) { Role::Alert } else { Role::Status })
                    .aria_label(label)
                    .child(card),
            );
        }

        // Shadow is drawn 4px right/down of each card; leave room for it.
        let corner = point(viewport.width - px(16.), viewport.height - self.bottom);
        deferred(anchored().anchor(gpui::Anchor::BottomRight).position(corner).child(stack))
            .with_priority(3)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn countdown_steps_down_and_ends_at_zero() {
        let total = Duration::from_secs(4);
        assert_eq!(countdown_segments(total, total), SEGMENTS);
        assert_eq!(countdown_segments(Duration::from_millis(2000), total), SEGMENTS / 2);
        assert_eq!(countdown_segments(Duration::from_millis(1), total), 1);
        assert_eq!(countdown_segments(Duration::ZERO, total), 0);
        // Monotonic.
        let mut last = SEGMENTS;
        for ms in (0..=4000).rev().step_by(50) {
            let s = countdown_segments(Duration::from_millis(ms), total);
            assert!(s <= last);
            last = s;
        }
    }

    #[test]
    fn entry_is_three_frames() {
        assert_eq!(entry_frame(Duration::ZERO), Some(0));
        assert_eq!(entry_frame(motion::FRAME * 2), Some(2));
        assert_eq!(entry_frame(motion::FRAME * 3), None);
    }

    #[test]
    fn kinds_have_distinct_codes_and_rising_lifetimes() {
        let kinds = [ToastKind::Info, ToastKind::Success, ToastKind::Warning, ToastKind::Danger];
        let codes: std::collections::HashSet<_> = kinds.iter().map(|k| k.code()).collect();
        assert_eq!(codes.len(), kinds.len());
        assert!(ToastKind::Danger.default_duration() > ToastKind::Warning.default_duration());
        assert!(ToastKind::Warning.default_duration() > ToastKind::Success.default_duration());
    }
}
