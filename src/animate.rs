//! Stepped animation: the engine behind Ferrite's motion, and the effects
//! built on it.
//!
//! Ferrite motion is **jerky yet smooth, bold yet contained**:
//!
//! - **Jerky:** every animation advances in whole frames at
//!   [`motion::FRAME`] (25fps). Nothing glides; you can count the steps.
//! - **Smooth:** the steps follow an ease-out ([`snap`]), so the first
//!   frames take big bites and the last ones settle — a steady cadence that
//!   reads as fluid, not broken.
//! - **Bold:** the effects are unmistakable — a dither dissolve, a CRT-style
//!   unroll with an amber scan edge, text that decrypts into place, a shake.
//! - **Contained:** every effect stays inside its element's box and is over
//!   in 120–320ms. A clip runs a timer only while it plays, then stops; idle
//!   windows cost nothing. Under reduced motion every clip is already done.
//!
//! The engine is [`play`] / [`play_on_change`]: keyed, timer-driven progress
//! that re-renders the calling view once per frame while running. The
//! effects are plain functions of that progress ([`scramble`], [`type_on`],
//! [`shake_offset`], [`dissolve_level`]) and two elements that need paint
//! access ([`Unroll`], [`Nudge`]).

use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::{Duration, Instant};

use gpui::{
    AnyElement, App, Bounds, ContentMask, ElementId, GlobalElementId, Hsla, InspectorElementId,
    IntoElement, LayoutId, Pixels, Point, Task, Window, fill, point, px, size,
};

use crate::motion;

// ── Curves ────────────────────────────────────────────────────────────────

/// Ferrite's one easing curve: a steep ease-out (cubic). Applied to stepped
/// progress it makes big early jumps and small late ones.
pub fn snap(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    1. - (1. - t).powi(3)
}

/// Number of whole frames in `duration` (at least one).
pub fn frames(duration: Duration) -> u32 {
    ((duration.as_secs_f32() / motion::FRAME.as_secs_f32()).round() as u32).max(1)
}

/// Linear progress quantised to whole frames: 0, 1/n, 2/n … 1.
pub fn quantise(elapsed: Duration, duration: Duration) -> f32 {
    let n = frames(duration);
    let frame = (elapsed.as_secs_f32() / motion::FRAME.as_secs_f32()).floor() as u32;
    (frame.min(n) as f32) / n as f32
}

// ── The engine ────────────────────────────────────────────────────────────

/// Where a clip is. `t` is linear and frame-quantised; use [`Progress::eased`]
/// for positions and sizes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Progress {
    pub t: f32,
    /// Which frame this is (0-based) — for effects that index tables.
    pub frame: u32,
    pub done: bool,
}

impl Progress {
    pub const DONE: Progress = Progress { t: 1., frame: u32::MAX, done: true };

    pub fn eased(&self) -> f32 {
        snap(self.t)
    }
}

struct Clip {
    key: u64,
    start: Instant,
    delay: Duration,
    duration: Duration,
    _task: Option<Task<()>>,
}

fn hash_of(key: impl Hash) -> u64 {
    let mut h = DefaultHasher::new();
    key.hash(&mut h);
    h.finish()
}

fn progress(clip: &Clip) -> Progress {
    let elapsed = clip.start.elapsed().saturating_sub(clip.delay);
    let t = quantise(elapsed, clip.duration);
    let frame = (elapsed.as_secs_f32() / motion::FRAME.as_secs_f32()).floor() as u32;
    Progress { t, frame, done: t >= 1. }
}

/// Drive `clip` one frame at a time until it's done, then stop.
fn run(cx: &mut gpui::Context<Clip>) -> Task<()> {
    cx.spawn(async move |this, cx| {
        loop {
            cx.background_executor().timer(motion::FRAME).await;
            let done = this.update(cx, |clip, cx| {
                cx.notify();
                // Done only once the delay has passed too.
                clip.start.elapsed() >= clip.delay && progress(clip).done
            });
            if !matches!(done, Ok(false)) {
                break;
            }
        }
    })
}

fn clip(id: impl Into<ElementId>, key: u64, delay: Duration, duration: Duration, play_now: bool, window: &mut Window, cx: &mut App) -> Progress {
    if motion::reduced(cx) {
        return Progress::DONE;
    }
    let state = window.use_keyed_state(id, cx, move |_, cx| {
        let start = if play_now { Instant::now() } else { Instant::now() - (delay + duration) * 2 };
        Clip { key, start, delay, duration, _task: play_now.then(|| run(cx)) }
    });
    if state.read(cx).key != key {
        state.update(cx, |c, cx| {
            c.key = key;
            c.start = Instant::now();
            c.delay = delay;
            c.duration = duration;
            c._task = Some(run(cx));
        });
    }
    progress(state.read(cx))
}

/// Progress of a clip that plays when the element first appears and again
/// whenever `key` changes. Call while rendering; the view re-renders each
/// frame until it finishes.
pub fn play(id: impl Into<ElementId>, key: impl Hash, duration: Duration, window: &mut Window, cx: &mut App) -> Progress {
    clip(id, hash_of(key), Duration::ZERO, duration, true, window, cx)
}

/// [`play`] after holding at the start for `delay` — for cascades, where
/// each item waits a frame or two longer than the one before it
/// (see [`stagger`]).
pub fn play_after(id: impl Into<ElementId>, key: impl Hash, delay: Duration, duration: Duration, window: &mut Window, cx: &mut App) -> Progress {
    clip(id, hash_of(key), delay, duration, true, window, cx)
}

/// The delay for the `i`th item of a cascade: one frame apart, capped so a
/// long list still finishes inside the contained window.
pub fn stagger(i: usize) -> Duration {
    motion::FRAME * (i.min(8) as u32)
}

/// Like [`play`], but starts finished: it only plays when `key` changes.
/// For state transitions that shouldn't animate on first render.
pub fn play_on_change(id: impl Into<ElementId>, key: impl Hash, duration: Duration, window: &mut Window, cx: &mut App) -> Progress {
    clip(id, hash_of(key), Duration::ZERO, duration, false, window, cx)
}

/// A glyph for "stamping" a mark into place: two frames of noise, then the
/// mark. For checkboxes, radios — anything that flips between two glyphs.
pub fn stamp(mark: &'static str, p: Progress) -> &'static str {
    const NOISE: [&str; 3] = ["#", "*", "+"];
    if p.done { mark } else { NOISE[(p.frame as usize).min(NOISE.len() - 1)] }
}

// ── Effects (pure) ────────────────────────────────────────────────────────

/// Glyphs text scrambles through. ASCII only, so they exist in every face.
const NOISE: &[u8] = b"#%&*+=<>/\\|$@?!";

fn noise(i: usize, frame: u32) -> char {
    let h = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (frame as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    NOISE[(h >> 33) as usize % NOISE.len()] as char
}

/// "Decrypt" text into place: characters lock in left to right as `t`
/// goes 0→1; the rest churn through noise each frame. Spaces stay spaces,
/// so word shapes read from the first frame.
pub fn scramble(text: &str, p: Progress) -> String {
    if p.done {
        return text.to_string();
    }
    let n = text.chars().count();
    let locked = (p.eased() * n as f32).floor() as usize;
    text.chars()
        .enumerate()
        .map(|(i, c)| if i < locked || c.is_whitespace() { c } else { noise(i, p.frame) })
        .collect()
}

/// Type text on like a terminal: the first `t·len` characters, plus a
/// block cursor while typing.
pub fn type_on(text: &str, p: Progress) -> String {
    if p.done {
        return text.to_string();
    }
    let n = text.chars().count();
    let shown = (p.t * n as f32).round() as usize;
    let mut s: String = text.chars().take(shown).collect();
    s.push('█');
    s
}

/// A shake: a decaying side-to-side jolt, one table step per frame. Bold,
/// but it never leaves a 6px band and always ends at rest.
pub fn shake_offset(p: Progress) -> Pixels {
    const STEPS: [f32; 8] = [0., 6., -5., 4., -3., 2., -1., 0.];
    if p.done {
        return px(0.);
    }
    px(STEPS[(p.frame as usize).min(STEPS.len() - 1)])
}

/// Dither level for a dissolve-in veil: full cover → clear, eased and
/// snapped to the 4×4 Bayer matrix's 16 steps so every frame is a real
/// pattern change.
pub fn dissolve_level(p: Progress) -> f32 {
    ((1. - p.eased()) * 16.).round() / 16.
}

/// Interpolate a number in steps (a counter rolling to its value).
pub fn count(from: f32, to: f32, p: Progress) -> f32 {
    from + (to - from) * p.eased()
}

// ── Elements ──────────────────────────────────────────────────────────────

/// Reveals its child top-to-bottom like a CRT drawing a frame: the child is
/// laid out at full size and clipped to `t` of its height, with an amber
/// scan line on the leading edge. Includes `bleed` beyond the box so a hard
/// drop shadow unrolls with it.
pub struct Unroll {
    child: AnyElement,
    t: f32,
    edge: Option<Hsla>,
    bleed: Pixels,
}

pub fn unroll(child: impl IntoElement, t: f32) -> Unroll {
    Unroll { child: child.into_any_element(), t: t.clamp(0., 1.), edge: None, bleed: px(8.) }
}

impl Unroll {
    /// Draw the scan line on the clip edge while unrolling.
    pub fn edge(mut self, color: Hsla) -> Self {
        self.edge = Some(color);
        self
    }
}

impl IntoElement for Unroll {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl gpui::Element for Unroll {
    type RequestLayoutState = ();
    type PrepaintState = Bounds<Pixels>;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, window: &mut Window, cx: &mut App) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, bounds: Bounds<Pixels>, _: &mut (), window: &mut Window, cx: &mut App) -> Bounds<Pixels> {
        let visible = Bounds::new(bounds.origin, size(bounds.size.width + self.bleed, (bounds.size.height + self.bleed) * self.t));
        window.with_content_mask(Some(ContentMask { bounds: visible }), |window| {
            self.child.prepaint(window, cx);
        });
        visible
    }

    fn paint(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, bounds: Bounds<Pixels>, _: &mut (), visible: &mut Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        window.with_content_mask(Some(ContentMask { bounds: *visible }), |window| {
            self.child.paint(window, cx);
        });
        if let Some(edge) = self.edge.filter(|_| self.t < 1.) {
            let y = (bounds.top() + bounds.size.height * self.t - px(1.)).max(bounds.top());
            window.paint_quad(fill(Bounds::new(point(bounds.left(), y), size(bounds.size.width, px(2.))), edge));
        }
    }
}

/// Shifts its child without affecting layout — for shakes and stepped
/// slide-ins. The child's hitboxes move with it.
pub struct Nudge {
    child: AnyElement,
    offset: Point<Pixels>,
}

pub fn nudge(child: impl IntoElement, offset: Point<Pixels>) -> Nudge {
    Nudge { child: child.into_any_element(), offset }
}

impl IntoElement for Nudge {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl gpui::Element for Nudge {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, window: &mut Window, cx: &mut App) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, _: Bounds<Pixels>, _: &mut (), window: &mut Window, cx: &mut App) {
        let offset = self.offset;
        window.with_element_offset(offset, |window| {
            self.child.prepaint(window, cx);
        });
    }

    fn paint(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, _: Bounds<Pixels>, _: &mut (), _: &mut (), window: &mut Window, cx: &mut App) {
        self.child.paint(window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(t: f32, frame: u32) -> Progress {
        Progress { t, frame, done: t >= 1. }
    }

    #[test]
    fn progress_is_whole_frames() {
        let d = Duration::from_millis(200); // 5 frames
        assert_eq!(frames(d), 5);
        assert_eq!(quantise(Duration::from_millis(0), d), 0.);
        assert_eq!(quantise(Duration::from_millis(39), d), 0.);
        assert_eq!(quantise(Duration::from_millis(41), d), 0.2);
        assert_eq!(quantise(Duration::from_millis(999), d), 1.);
    }

    #[test]
    fn snap_front_loads_the_motion() {
        // Half the time covers most of the distance; ends exactly.
        assert!(snap(0.5) > 0.85);
        assert_eq!(snap(0.), 0.);
        assert_eq!(snap(1.), 1.);
    }

    #[test]
    fn scramble_locks_left_to_right_and_keeps_spaces() {
        let s = scramble("git push", at(0., 0));
        assert_eq!(s.chars().nth(3), Some(' '));
        assert_ne!(s, "git push");
        assert_eq!(scramble("git push", Progress::DONE), "git push");
        let mid = scramble("abcdefgh", at(0.4, 2));
        assert!(mid.starts_with("abcd")); // snap(0.4)·8 ≈ 6.2 → six locked
    }

    #[test]
    fn type_on_shows_a_cursor_until_done() {
        assert_eq!(type_on("abcd", at(0.5, 1)), "ab█");
        assert_eq!(type_on("abcd", Progress::DONE), "abcd");
    }

    #[test]
    fn shake_is_bounded_and_ends_at_rest() {
        for f in 0..20 {
            assert!(f32::from(shake_offset(at(0.5, f))).abs() <= 6.);
        }
        assert_eq!(shake_offset(Progress::DONE), px(0.));
    }

    #[test]
    fn dissolve_lands_on_bayer_steps() {
        for i in 0..=10 {
            let l = dissolve_level(at(i as f32 / 10., i)) * 16.;
            assert_eq!(l, l.round());
        }
        assert_eq!(dissolve_level(at(1., 10)), 0.);
        assert_eq!(dissolve_level(at(0., 0)), 1.);
    }
}
