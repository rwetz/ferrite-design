//! Motion effects as drop-in elements, built on [`crate::animate`].
//!
//! Each is keyed: give it an id and a `key`, and it plays when it first
//! appears and again whenever `key` changes — so `decrypt("status", &msg)`
//! re-decrypts every time the message changes, and `shake("name", errors,
//! field)` jolts once per new error.
//!
//! | Effect | Use it for |
//! |---|---|
//! | [`decrypt`] | status lines, headings, results arriving |
//! | [`typewriter`] | prompts, onboarding, a log line being written |
//! | [`shake`] | rejected input, a failed action |
//! | [`dissolve`] | content swapping in (a page, a preview) |
//! | [`count_up`] | numbers changing (totals, timings) |
//! | [`unroll_in`] | panels and sections appearing |

use std::hash::Hash;
use std::time::Duration;

use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, point, px,
};

use crate::animate::{self, dissolve_level, nudge, scramble, shake_offset, type_on, unroll};
use crate::dither::{self, dither};
use crate::motion;
use crate::theme::palette;
use crate::tokens::hsla;

/// Text that decrypts into place: noise locking in left to right.
#[derive(IntoElement)]
pub struct Decrypt {
    id: ElementId,
    text: SharedString,
    duration: Option<Duration>,
}

pub fn decrypt(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Decrypt {
    Decrypt { id: id.into(), text: text.into(), duration: None }
}

impl Decrypt {
    pub fn duration(mut self, d: Duration) -> Self {
        self.duration = Some(d);
        self
    }
}

impl RenderOnce for Decrypt {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Longer text takes longer, within the contained range.
        let d = self.duration.unwrap_or_else(|| {
            let n = self.text.chars().count() as u64;
            Duration::from_millis((n * 24).clamp(160, 400))
        });
        let p = animate::play(self.id, &self.text, d, window, cx);
        div().child(scramble(&self.text, p))
    }
}

/// Text typed on with a block cursor.
#[derive(IntoElement)]
pub struct Typewriter {
    id: ElementId,
    text: SharedString,
}

pub fn typewriter(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Typewriter {
    Typewriter { id: id.into(), text: text.into() }
}

impl RenderOnce for Typewriter {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let n = self.text.chars().count() as u64;
        let d = Duration::from_millis((n * 28).clamp(200, 900));
        let p = animate::play(self.id, &self.text, d, window, cx);
        div().child(type_on(&self.text, p))
    }
}

/// Jolts its child sideways once whenever `key` changes (not on first
/// render). Layout never moves; only the paint does.
#[derive(IntoElement)]
pub struct Shake {
    id: ElementId,
    key: u64,
    child: AnyElement,
}

pub fn shake(id: impl Into<ElementId>, key: impl Hash, child: impl IntoElement) -> Shake {
    use std::hash::{DefaultHasher, Hasher};
    let mut h = DefaultHasher::new();
    key.hash(&mut h);
    Shake { id: id.into(), key: h.finish(), child: child.into_any_element() }
}

impl RenderOnce for Shake {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = animate::play_on_change(self.id, self.key, motion::SLOW, window, cx);
        nudge(self.child, point(shake_offset(p), px(0.)))
    }
}

/// Content that dissolves in through the Bayer ramp whenever `key` changes
/// (and when it first appears).
#[derive(IntoElement)]
pub struct Dissolve {
    id: ElementId,
    key: u64,
    child: AnyElement,
}

pub fn dissolve(id: impl Into<ElementId>, key: impl Hash, child: impl IntoElement) -> Dissolve {
    use std::hash::{DefaultHasher, Hasher};
    let mut h = DefaultHasher::new();
    key.hash(&mut h);
    Dissolve { id: id.into(), key: h.finish(), child: child.into_any_element() }
}

impl RenderOnce for Dissolve {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = animate::play(self.id, self.key, motion::BASE, window, cx);
        let level = dissolve_level(p);
        div()
            .relative()
            .child(self.child)
            .when_some((level > 0.).then_some(level), |el, level| {
                el.child(div().absolute().inset_0().child(dither(dither::flat(level)).ink(hsla(palette(cx).bg)).size_full()))
            })
    }
}

/// A number that rolls to its new value in steps. `format` prints it.
#[derive(IntoElement)]
pub struct CountUp {
    id: ElementId,
    value: f32,
    format: fn(f32) -> String,
}

pub fn count_up(id: impl Into<ElementId>, value: f32, format: fn(f32) -> String) -> CountUp {
    CountUp { id: id.into(), value, format }
}

struct Rolling {
    from: f32,
    to: f32,
}

impl RenderOnce for CountUp {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let value = self.value;
        // Remember where we're rolling from: the value on screen when the
        // target changed.
        let roll = window.use_keyed_state(ElementId::Name(format!("{id}-from").into()), cx, move |_, _| Rolling { from: value, to: value });
        let p = animate::play_on_change(id, value.to_bits(), motion::SLOW, window, cx);
        let (from, to) = {
            let r = roll.read(cx);
            (r.from, r.to)
        };
        if to != value {
            // Start from whatever was showing mid-roll.
            let showing = animate::count(from, to, p);
            roll.update(cx, |r, _| {
                r.from = if p.done { to } else { showing };
                r.to = value;
            });
        }
        let r = roll.read(cx);
        div().child((self.format)(animate::count(r.from, r.to, p)))
    }
}

/// Unrolls its child top-down with an amber scan edge when it first
/// appears and whenever `key` changes.
#[derive(IntoElement)]
pub struct UnrollIn {
    id: ElementId,
    key: u64,
    child: AnyElement,
    duration: Duration,
}

pub fn unroll_in(id: impl Into<ElementId>, key: impl Hash, child: impl IntoElement) -> UnrollIn {
    use std::hash::{DefaultHasher, Hasher};
    let mut h = DefaultHasher::new();
    key.hash(&mut h);
    UnrollIn { id: id.into(), key: h.finish(), child: child.into_any_element(), duration: motion::BASE }
}

impl UnrollIn {
    pub fn duration(mut self, d: Duration) -> Self {
        self.duration = d;
        self
    }
}

impl RenderOnce for UnrollIn {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = animate::play(self.id, self.key, self.duration, window, cx);
        unroll(self.child, p.eased()).edge(hsla(palette(cx).accent))
    }
}

use gpui::prelude::FluentBuilder as _;
