//! Stepped animation: the engine behind Ferrite's motion, and the effects
//! built on it.
//!
//! Ferrite motion is **jerky yet smooth, bold yet contained**:
//!
//! - **Jerky:** every animation advances in whole frames at
//!   [`motion::frame`] (240fps by default, display-smooth; apps lower it
//!   to 25 for the classic stepped look).
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
//! [`shake_offset`], [`dissolve_level`], [`develop_level`],
//! [`interlace_fields`], [`tear_bands`], [`ping_ring`], [`power_on_band`],
//! [`scan_line`], [`flash_level`]) and the elements that need paint access
//! ([`Unroll`], [`Wipe`], [`Nudge`], [`Interlace`], [`Band`], [`PowerOn`]).

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

/// Number of whole frames in `duration` (at least one), at the live rate.
pub fn frames(duration: Duration) -> u32 {
    frames_at(duration, motion::frame())
}

/// Linear progress quantised to whole frames at the live rate: 0, 1/n … 1.
pub fn quantise(elapsed: Duration, duration: Duration) -> f32 {
    quantise_at(elapsed, duration, motion::frame())
}

fn frames_at(duration: Duration, frame: Duration) -> u32 {
    ((duration.as_secs_f32() / frame.as_secs_f32()).round() as u32).max(1)
}

fn quantise_at(elapsed: Duration, duration: Duration, frame: Duration) -> f32 {
    let n = frames_at(duration, frame);
    let step = (elapsed.as_secs_f32() / frame.as_secs_f32()).floor() as u32;
    (step.min(n) as f32) / n as f32
}

// ── The engine ────────────────────────────────────────────────────────────

/// Where a clip is. `t` is linear and frame-quantised; use [`Progress::eased`]
/// for positions and sizes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Progress {
    pub t: f32,
    /// Which 40ms beat ([`motion::FRAME`]) this is, 0-based — for effects
    /// that index tables (shake, stamp, noise). Fixed at any refresh rate, so
    /// a table's timing never changes; only `t` gets finer.
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
            cx.background_executor().timer(motion::frame()).await;
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

/// Veil level for a develop: full cover → clear over the blue-noise tile, so
/// the content arrives speck by speck. Snapped to 64ths: finer than Bayer's
/// 16, but bounded, so a develop at 240fps can't flood the raster cache.
pub fn develop_level(p: Progress) -> f32 {
    ((1. - p.eased()) * 64.).round() / 64.
}

/// How far each interlaced field has been drawn, top to bottom, as
/// `(even, odd)`: the even rows in the first half, the odd rows in the second.
pub fn interlace_fields(p: Progress) -> (f32, f32) {
    if p.done {
        return (1., 1.);
    }
    if p.t < 0.5 { (snap(p.t * 2.), 0.) } else { (1., snap((p.t - 0.5) * 2.)) }
}

/// A tear: three horizontal bands of an element jolted sideways, one table
/// row per frame, then whole again. Each band is `(from, to, dx)`: fractions
/// of the height and a pixel offset. Never beyond 6px, like a shake.
pub fn tear_bands(p: Progress) -> Option<[(f32, f32, f32); 3]> {
    const TABLE: [[(f32, f32, f32); 3]; 3] = [
        [(0., 0.30, 0.), (0.30, 0.55, 6.), (0.55, 1., -3.)],
        [(0., 0.45, -4.), (0.45, 0.70, 0.), (0.70, 1., 5.)],
        [(0., 0.20, 0.), (0.20, 0.80, 2.), (0.80, 1., 0.)],
    ];
    if p.done { None } else { TABLE.get(p.frame as usize).copied() }
}

/// A ping: a square dither ring stepping outward from its element and
/// thinning ▓ → ▒ → ░, one step per frame. `(spread, level)`.
pub fn ping_ring(p: Progress) -> Option<(Pixels, f32)> {
    use crate::dither::level;
    const STEPS: [(f32, f32); 3] = [(2., level::DARK), (5., level::MEDIUM), (8., level::LIGHT)];
    if p.done {
        return None;
    }
    STEPS.get(p.frame as usize).map(|&(spread, level)| (px(spread), level))
}

/// Power-on: the visible band as `(width, height)` fractions. A line draws
/// out from the centre, then the picture opens vertically from it — a CRT
/// warming up.
pub fn power_on_band(p: Progress) -> (f32, f32) {
    const SPLIT: f32 = 0.35;
    if p.done {
        return (1., 1.);
    }
    if p.t < SPLIT { (snap(p.t / SPLIT), 0.) } else { (1., snap((p.t - SPLIT) / (1. - SPLIT))) }
}

/// A scan: one amber line passing top to bottom over content that is
/// already there — "this just refreshed". The line's position as a fraction
/// of the height, or `None` once it has passed.
pub fn scan_line(p: Progress) -> Option<f32> {
    (!p.done).then(|| p.eased())
}

/// A flash: the element floods with ink, then dissolves back through the
/// Bayer ramp. Capped at ▓ (75%) so whatever is underneath still reads.
/// `None` when there's nothing to draw.
pub fn flash_level(p: Progress) -> Option<f32> {
    if p.done {
        return None;
    }
    let level = (dissolve_level(p) * crate::dither::level::DARK * 16.).round() / 16.;
    (level > 0.).then_some(level)
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

/// Which edge a [`Wipe`] starts from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Edge {
    /// Reveal left → right (a sidebar, a drawer from the left).
    #[default]
    Left,
    /// Reveal right → left (a drawer from the right).
    Right,
}

/// [`Unroll`] turned on its side: reveals its child from one edge with an
/// amber scan line on the leading edge. Layout is the child's full size from
/// the first frame, so nothing around it moves. For side panels and drawers.
pub struct Wipe {
    child: AnyElement,
    t: f32,
    from: Edge,
    edge: Option<Hsla>,
    bleed: Pixels,
}

pub fn wipe(child: impl IntoElement, t: f32, from: Edge) -> Wipe {
    Wipe { child: child.into_any_element(), t: t.clamp(0., 1.), from, edge: None, bleed: px(8.) }
}

impl Wipe {
    /// Draw the scan line on the clip edge while wiping.
    pub fn edge(mut self, color: Hsla) -> Self {
        self.edge = Some(color);
        self
    }

    fn visible(&self, bounds: Bounds<Pixels>) -> Bounds<Pixels> {
        let w = (bounds.size.width + self.bleed) * self.t;
        let h = bounds.size.height + self.bleed;
        match self.from {
            Edge::Left => Bounds::new(bounds.origin, size(w, h)),
            Edge::Right => Bounds::new(point(bounds.right() - w + self.bleed, bounds.top()), size(w, h)),
        }
    }
}

impl IntoElement for Wipe {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl gpui::Element for Wipe {
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
        let visible = self.visible(bounds);
        window.with_content_mask(Some(ContentMask { bounds: visible }), |window| self.child.prepaint(window, cx));
        visible
    }

    fn paint(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, bounds: Bounds<Pixels>, _: &mut (), visible: &mut Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        window.with_content_mask(Some(ContentMask { bounds: *visible }), |window| self.child.paint(window, cx));
        if let Some(edge) = self.edge.filter(|_| self.t < 1.) {
            let x = match self.from {
                Edge::Left => (bounds.left() + bounds.size.width * self.t - px(1.)).max(bounds.left()),
                Edge::Right => (bounds.right() - bounds.size.width * self.t - px(1.)).min(bounds.right() - px(2.)),
            };
            window.paint_quad(fill(Bounds::new(point(x, bounds.top()), size(px(2.), bounds.size.height)), edge));
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

/// Draws its child one interlaced field at a time: the even rows sweep down
/// first, then the odd ones fill in. Unrevealed rows are covered with
/// `veil` (the background behind the child), so set that to match.
pub struct Interlace {
    child: AnyElement,
    even: f32,
    odd: f32,
    veil: Hsla,
}

pub fn interlace(child: impl IntoElement, (even, odd): (f32, f32), veil: Hsla) -> Interlace {
    Interlace { child: child.into_any_element(), even: even.clamp(0., 1.), odd: odd.clamp(0., 1.), veil }
}

impl IntoElement for Interlace {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl gpui::Element for Interlace {
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
        self.child.prepaint(window, cx);
    }

    fn paint(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, bounds: Bounds<Pixels>, _: &mut (), _: &mut (), window: &mut Window, cx: &mut App) {
        self.child.paint(window, cx);
        if self.even >= 1. && self.odd >= 1. {
            return;
        }
        // One scan line is one logical pixel snapped to whole device pixels.
        let sf = window.scale_factor();
        let line = sf.round().max(1.) / sf;
        let height = f32::from(bounds.size.height);
        let rows = (height / line).ceil() as usize;
        let hidden = |i: usize| {
            let y = i as f32 * line;
            y >= height * if i.is_multiple_of(2) { self.even } else { self.odd }
        };
        // Cover runs of hidden rows: below both sweeps that is one quad.
        let mut i = 0;
        while i < rows {
            if !hidden(i) {
                i += 1;
                continue;
            }
            let start = i;
            while i < rows && hidden(i) {
                i += 1;
            }
            let top = start as f32 * line;
            let bottom = (i as f32 * line).min(height);
            let quad = Bounds::new(point(bounds.left(), bounds.top() + px(top)), size(bounds.size.width, px(bottom - top)));
            window.paint_quad(fill(quad, self.veil));
        }
    }
}

/// One horizontal band of its child, shifted sideways: the child is clipped
/// to `from..to` of its height (fractions) and drawn `dx` across. Three
/// stacked bands make a tear.
pub struct Band {
    child: AnyElement,
    from: f32,
    to: f32,
    dx: Pixels,
}

pub fn band(child: impl IntoElement, from: f32, to: f32, dx: Pixels) -> Band {
    Band { child: child.into_any_element(), from, to, dx }
}

impl Band {
    fn mask(&self, bounds: Bounds<Pixels>) -> Bounds<Pixels> {
        let h = bounds.size.height;
        let slack = px(8.);
        Bounds::new(
            point(bounds.left() - slack, bounds.top() + h * self.from),
            size(bounds.size.width + slack * 2., h * (self.to - self.from)),
        )
    }
}

impl IntoElement for Band {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl gpui::Element for Band {
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
        let mask = self.mask(bounds);
        let offset = point(self.dx, px(0.));
        window.with_content_mask(Some(ContentMask { bounds: mask }), |window| {
            window.with_element_offset(offset, |window| self.child.prepaint(window, cx));
        });
        mask
    }

    fn paint(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, _: Bounds<Pixels>, _: &mut (), mask: &mut Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        window.with_content_mask(Some(ContentMask { bounds: *mask }), |window| self.child.paint(window, cx));
    }
}

/// Reveals its child like a CRT warming up: a bright `line` draws out from
/// the centre, then the picture opens vertically from it. `(w, h)` from
/// [`power_on_band`]. Outside the band nothing is painted.
pub struct PowerOn {
    child: AnyElement,
    w: f32,
    h: f32,
    line: Hsla,
}

pub fn power_on(child: impl IntoElement, (w, h): (f32, f32), line: Hsla) -> PowerOn {
    PowerOn { child: child.into_any_element(), w: w.clamp(0., 1.), h: h.clamp(0., 1.), line }
}

impl PowerOn {
    const LINE: Pixels = px(2.);

    fn band(&self, bounds: Bounds<Pixels>) -> Bounds<Pixels> {
        if self.w >= 1. && self.h >= 1. {
            return bounds;
        }
        let w = bounds.size.width * self.w;
        let h = (bounds.size.height * self.h).max(Self::LINE);
        Bounds::new(point(bounds.center().x - w / 2., bounds.center().y - h / 2.), size(w, h))
    }
}

impl IntoElement for PowerOn {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl gpui::Element for PowerOn {
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
        let band = self.band(bounds);
        window.with_content_mask(Some(ContentMask { bounds: band }), |window| self.child.prepaint(window, cx));
        band
    }

    fn paint(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, _: Bounds<Pixels>, _: &mut (), band: &mut Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        if self.h <= 0. {
            // Still drawing the line: only the line shows.
            window.paint_quad(fill(*band, self.line));
            return;
        }
        window.with_content_mask(Some(ContentMask { bounds: *band }), |window| self.child.paint(window, cx));
        if self.h < 1. {
            let edge = |y| Bounds::new(point(band.left(), y), size(band.size.width, Self::LINE));
            window.paint_quad(fill(edge(band.top()), self.line));
            window.paint_quad(fill(edge(band.bottom() - Self::LINE), self.line));
        }
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
        assert_eq!(frames_at(d, motion::FRAME), 5);
        assert_eq!(quantise_at(Duration::from_millis(0), d, motion::FRAME), 0.);
        assert_eq!(quantise_at(Duration::from_millis(39), d, motion::FRAME), 0.);
        assert_eq!(quantise_at(Duration::from_millis(41), d, motion::FRAME), 0.2);
        assert_eq!(quantise_at(Duration::from_millis(999), d, motion::FRAME), 1.);
        // At 240fps the same 200ms clip takes 48 finer steps, same length.
        let fast = Duration::from_micros(4_167);
        assert_eq!(frames_at(d, fast), 48);
        assert_eq!(quantise_at(Duration::from_millis(201), d, fast), 1.);
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

    #[test]
    fn new_effects_end_at_rest() {
        assert_eq!(develop_level(Progress::DONE), 0.);
        assert_eq!(develop_level(at(0., 0)), 1.);
        assert_eq!(interlace_fields(Progress::DONE), (1., 1.));
        assert_eq!(interlace_fields(at(0.25, 1)).1, 0., "odd rows wait for the even field");
        assert_eq!(interlace_fields(at(0.75, 3)).0, 1., "even field done before odd starts");
        assert!(tear_bands(Progress::DONE).is_none());
        assert!(tear_bands(at(0.9, 3)).is_none(), "a tear is three frames");
        assert!(ping_ring(Progress::DONE).is_none());
        assert_eq!(power_on_band(Progress::DONE), (1., 1.));
        assert_eq!(power_on_band(at(0.2, 2)).1, 0., "the line draws before it opens");
    }

    #[test]
    fn tears_stay_in_band_and_cover_the_element() {
        for f in 0..3 {
            let bands = tear_bands(at(0.1, f)).unwrap();
            assert_eq!(bands[0].0, 0.);
            assert_eq!(bands[2].1, 1.);
            for w in bands.windows(2) {
                assert_eq!(w[0].1, w[1].0, "bands tile the height");
            }
            assert!(bands.iter().all(|b| b.2.abs() <= 6.));
        }
    }

    #[test]
    fn scan_and_flash_end_at_rest() {
        assert!(scan_line(Progress::DONE).is_none());
        assert_eq!(scan_line(at(0., 0)), Some(0.));
        assert!(flash_level(Progress::DONE).is_none());
        let first = flash_level(at(0., 0)).unwrap();
        assert!(first <= crate::dither::level::DARK, "a flash never hides what's under it");
        assert_eq!((first * 16.).fract(), 0., "flash lands on Bayer steps");
    }

    #[test]
    fn stagger_is_capped() {
        assert_eq!(stagger(0), Duration::ZERO);
        assert_eq!(stagger(8), stagger(100), "a long list still finishes inside the window");
    }

    #[test]
    fn develop_is_bounded_to_64_levels() {
        for i in 0..=1000 {
            let level = develop_level(at(i as f32 / 1000., 0));
            assert_eq!((level * 64.).fract(), 0.);
        }
    }
}
