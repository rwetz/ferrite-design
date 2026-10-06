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
//! | [`develop`] | pictures and art arriving, speck by speck (blue noise) |
//! | [`afterglow`] | a value changing: the old text fades like phosphor |
//! | [`interlace_in`] | page and panel switches, heavier than unroll |
//! | [`tear`] | a system failure (shake is for *input* errors) |
//! | [`ping`] | a new item: one dither ring steps out from a marker |
//! | [`power_on_in`] | a window opening — and, opted in, closing |
//! | [`wipe_in`] | side panels and drawers arriving from an edge |
//! | [`scan`] | content that just refreshed in place: one line passes over it |
//! | [`flash`] | "look here": a value that needs attention floods and clears |
//! | [`cascade_in`] | a list or grid arriving item by item, one frame apart |

use std::hash::Hash;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    AnyElement, App, ElementId, Hsla, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    StyleRefinement, Window, div, point, prelude::FluentBuilder as _, px, relative,
};

use crate::animate::{
    self, Edge, band, develop_level, dissolve_level, flash_level, interlace, interlace_fields, nudge, ping_ring,
    power_on, power_on_band, scan_line, scramble, shake_offset, stagger, tear_bands, type_on, unroll, wipe,
};
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
    Shake { id: id.into(), key: key_of(key), child: child.into_any_element() }
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
    Dissolve { id: id.into(), key: key_of(key), child: child.into_any_element() }
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
    UnrollIn { id: id.into(), key: key_of(key), child: child.into_any_element(), duration: motion::BASE }
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

fn key_of(key: impl Hash) -> u64 {
    use std::hash::{DefaultHasher, Hasher};
    let mut h = DefaultHasher::new();
    key.hash(&mut h);
    h.finish()
}

/// Content that develops in through blue noise, speck by speck, when it
/// first appears and whenever `key` changes. The grain is organic where
/// [`dissolve`]'s Bayer veil is a grid; made for pictures and art.
#[derive(IntoElement)]
pub struct Develop {
    id: ElementId,
    key: u64,
    child: AnyElement,
}

pub fn develop(id: impl Into<ElementId>, key: impl Hash, child: impl IntoElement) -> Develop {
    Develop { id: id.into(), key: key_of(key), child: child.into_any_element() }
}

impl RenderOnce for Develop {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = animate::play(self.id, self.key, motion::SLOW, window, cx);
        let level = develop_level(p);
        div().relative().child(self.child).when(level > 0., |el| {
            let veil = dither(dither::flat(level)).pattern(dither::Pattern::BlueNoise).ink(hsla(palette(cx).bg));
            el.child(div().absolute().inset_0().child(veil.size_full()))
        })
    }
}

/// Text whose old value lingers as a fading amber ghost under the new one
/// for a few frames when it changes — phosphor afterglow. First render is
/// plain. Style the text by styling the parent.
#[derive(IntoElement)]
pub struct Afterglow {
    id: ElementId,
    text: SharedString,
}

pub fn afterglow(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Afterglow {
    Afterglow { id: id.into(), text: text.into() }
}

impl RenderOnce for Afterglow {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p_ = palette(cx);
        // (current, previous): previous is what glows.
        let memory_id = ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), "glow".into());
        let memory = window.use_keyed_state(memory_id, cx, |_, _| (self.text.clone(), None::<SharedString>));
        if memory.read(cx).0 != self.text {
            let text = self.text.clone();
            memory.update(cx, |m, _| *m = (text, Some(m.0.clone())));
        }
        let ghost = memory.read(cx).1.clone();
        let p = animate::play_on_change(self.id, &self.text, motion::FAST, window, cx);
        // Bright phosphor, then dim, then gone: one step per frame.
        let glow: Option<Hsla> = (!p.done).then(|| hsla(if p.frame == 0 { p_.accent } else { p_.accent_dim }));
        div()
            .relative()
            .when_some(ghost.zip(glow), |el, (ghost, color)| {
                el.child(div().absolute().top_0().left_0().whitespace_nowrap().text_color(color).child(ghost))
            })
            .child(div().relative().child(self.text))
    }
}

/// Content drawn one interlaced field at a time when it first appears and
/// whenever `key` changes: even rows sweep down, then the odd rows fill in.
/// Heavier than [`unroll_in`]; for page and panel switches. Unrevealed rows
/// are covered in the page `bg` unless [`InterlaceIn::veil`] says otherwise.
#[derive(IntoElement)]
pub struct InterlaceIn {
    id: ElementId,
    key: u64,
    child: AnyElement,
    veil: Option<Hsla>,
}

pub fn interlace_in(id: impl Into<ElementId>, key: impl Hash, child: impl IntoElement) -> InterlaceIn {
    InterlaceIn { id: id.into(), key: key_of(key), child: child.into_any_element(), veil: None }
}

impl InterlaceIn {
    /// The color behind the content (default: the palette's `bg`).
    pub fn veil(mut self, color: Hsla) -> Self {
        self.veil = Some(color);
        self
    }
}

impl RenderOnce for InterlaceIn {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = animate::play(self.id, self.key, motion::BASE, window, cx);
        let veil = self.veil.unwrap_or_else(|| hsla(palette(cx).bg));
        interlace(self.child, interlace_fields(p), veil)
    }
}

/// Tears its content for three frames whenever `key` changes: horizontal
/// bands jolt sideways and snap back. For *system* failures — a lost link, a
/// crashed job — where [`shake`] means "your input was wrong".
///
/// Takes a builder, because each band draws its own copy of the content.
/// Use it on static content (banners, labels, icons): while tearing, copies
/// of anything interactive would duplicate its hitboxes.
#[derive(IntoElement)]
pub struct Tear {
    id: ElementId,
    key: u64,
    build: Rc<dyn Fn() -> AnyElement>,
}

pub fn tear<E: IntoElement>(id: impl Into<ElementId>, key: impl Hash, build: impl Fn() -> E + 'static) -> Tear {
    Tear { id: id.into(), key: key_of(key), build: Rc::new(move || build().into_any_element()) }
}

impl RenderOnce for Tear {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = animate::play_on_change(self.id, self.key, motion::FAST, window, cx);
        let Some(bands) = tear_bands(p) else {
            return div().child((self.build)());
        };
        let [first, rest @ ..] = bands;
        // The first band lays out in flow; the others overlay it exactly.
        let mut el = div().relative().child(band((self.build)(), first.0, first.1, px(first.2)));
        for (from, to, dx) in rest {
            el = el.child(div().absolute().inset_0().child(band((self.build)(), from, to, px(dx))));
        }
        el
    }
}

/// Wraps a marker (a tag, a badge, an icon) and pings it whenever `key`
/// changes: a square dither ring steps out 8px and thins away. For "a new
/// item arrived here". The ring bleeds past the box, like a drop shadow.
#[derive(IntoElement)]
pub struct Ping {
    id: ElementId,
    key: u64,
    child: AnyElement,
    ink: Option<Hsla>,
}

pub fn ping(id: impl Into<ElementId>, key: impl Hash, child: impl IntoElement) -> Ping {
    Ping { id: id.into(), key: key_of(key), child: child.into_any_element(), ink: None }
}

impl Ping {
    /// Ring color (default: the accent).
    pub fn ink(mut self, color: Hsla) -> Self {
        self.ink = Some(color);
        self
    }
}

impl RenderOnce for Ping {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = animate::play_on_change(self.id, self.key, motion::FAST, window, cx);
        let ink = self.ink.unwrap_or_else(|| hsla(palette(cx).accent));
        div().relative().child(self.child).when_some(ping_ring(p), |el, (spread, level)| {
            let edge = || dither(dither::flat(level)).ink(ink);
            let t = px(2.);
            el.child(
                div()
                    .absolute()
                    .top(-spread)
                    .left(-spread)
                    .right(-spread)
                    .bottom(-spread)
                    .child(div().absolute().top_0().left_0().right_0().h(t).child(edge().size_full()))
                    .child(div().absolute().bottom_0().left_0().right_0().h(t).child(edge().size_full()))
                    .child(div().absolute().top_0().bottom_0().left_0().w(t).child(edge().size_full()))
                    .child(div().absolute().top_0().bottom_0().right_0().w(t).child(edge().size_full())),
            )
        })
    }
}

/// A window powering on, CRT style: an amber line draws out from the centre
/// and the picture opens vertically from it. Plays every time the window
/// opens. Wrap the window's root content in it.
///
/// With `chrome::power_off_on_close` registered, it also plays the
/// switch-off in reverse when the window is closed.
#[derive(IntoElement)]
pub struct PowerOnIn {
    id: ElementId,
    child: AnyElement,
}

pub fn power_on_in(id: impl Into<ElementId>, child: impl IntoElement) -> PowerOnIn {
    PowerOnIn { id: id.into(), child: child.into_any_element() }
}

impl RenderOnce for PowerOnIn {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let closing = crate::chrome::powering_off(window, cx);
        let off_id = ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), "off".into());
        let off = animate::play_on_change(off_id, closing, motion::SLOW, window, cx);
        let p = animate::play(self.id, 0u8, motion::SLOW, window, cx);
        let band = if !closing {
            power_on_band(p)
        } else if off.done {
            (0., 0.) // Switched off; dark until the window goes.
        } else {
            // Power-on run backwards: collapse to the line, shrink to a point.
            power_on_band(animate::Progress { t: 1. - off.t, frame: 0, done: false })
        };
        power_on(self.child, band, hsla(palette(cx).accent))
    }
}


/// Wipes its child in from one edge with an amber scan edge when it first
/// appears and whenever `key` changes — [`unroll_in`] on its side. For
/// sidebars, drawers and inspector panes.
#[derive(IntoElement)]
pub struct WipeIn {
    id: ElementId,
    key: u64,
    child: AnyElement,
    from: Edge,
    duration: Duration,
}

pub fn wipe_in(id: impl Into<ElementId>, key: impl Hash, child: impl IntoElement) -> WipeIn {
    WipeIn { id: id.into(), key: key_of(key), child: child.into_any_element(), from: Edge::Left, duration: motion::BASE }
}

impl WipeIn {
    /// Reveal from the right edge (a drawer on the right).
    pub fn from_right(mut self) -> Self {
        self.from = Edge::Right;
        self
    }

    pub fn duration(mut self, d: Duration) -> Self {
        self.duration = d;
        self
    }
}

impl RenderOnce for WipeIn {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = animate::play(self.id, self.key, self.duration, window, cx);
        wipe(self.child, p.eased(), self.from).edge(hsla(palette(cx).accent))
    }
}

/// Passes one amber scan line down over its child whenever `key` changes
/// (not on first render). Nothing is hidden: the content is already there,
/// the line just says "refreshed". For reloaded panes, re-run queries,
/// polled data that arrived as an event.
#[derive(IntoElement)]
pub struct Scan {
    id: ElementId,
    key: u64,
    child: AnyElement,
}

pub fn scan(id: impl Into<ElementId>, key: impl Hash, child: impl IntoElement) -> Scan {
    Scan { id: id.into(), key: key_of(key), child: child.into_any_element() }
}

impl RenderOnce for Scan {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = animate::play_on_change(self.id, self.key, motion::SLOW, window, cx);
        let accent = hsla(palette(cx).accent);
        div().relative().child(self.child).when_some(scan_line(p), |el, y| {
            el.child(div().absolute().left_0().right_0().top(relative(y)).h(px(2.)).bg(accent))
        })
    }
}

/// Floods its child with ink and dissolves back through the Bayer ramp
/// whenever `key` changes (not on first render) — the button's click
/// acknowledgement, for anything. Capped at ▓ so the content reads through.
#[derive(IntoElement)]
pub struct Flash {
    id: ElementId,
    key: u64,
    child: AnyElement,
    ink: Option<Hsla>,
}

pub fn flash(id: impl Into<ElementId>, key: impl Hash, child: impl IntoElement) -> Flash {
    Flash { id: id.into(), key: key_of(key), child: child.into_any_element(), ink: None }
}

impl Flash {
    /// Flood color (default: the accent).
    pub fn ink(mut self, color: Hsla) -> Self {
        self.ink = Some(color);
        self
    }
}

impl RenderOnce for Flash {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = animate::play_on_change(self.id, self.key, motion::FAST, window, cx);
        let ink = self.ink.unwrap_or_else(|| hsla(palette(cx).accent));
        div().relative().child(self.child).when_some(flash_level(p), |el, level| {
            el.child(div().absolute().inset_0().child(dither(dither::flat(level)).ink(ink).size_full()))
        })
    }
}

/// Lays out its children like a `div` (style it: `.flex().flex_col()`…)
/// and unrolls each one in a frame after the one before it (capped at
/// eight frames), when it first appears and whenever `key` changes.
/// Layout is final from the first frame; only the reveal is staggered.
#[derive(IntoElement)]
pub struct CascadeIn {
    id: ElementId,
    key: u64,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

pub fn cascade_in(id: impl Into<ElementId>, key: impl Hash) -> CascadeIn {
    CascadeIn { id: id.into(), key: key_of(key), children: Vec::new(), style: StyleRefinement::default() }
}

impl ParentElement for CascadeIn {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for CascadeIn {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for CascadeIn {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let accent = hsla(palette(cx).accent);
        let mut root = div();
        *root.style() = self.style;
        for (i, child) in self.children.into_iter().enumerate() {
            let id = ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), format!("c{i}").into());
            let p = animate::play_after(id, self.key, stagger(i), motion::FAST, window, cx);
            root = root.child(if p.done { child } else { unroll(child, p.eased()).edge(accent).into_any_element() });
        }
        root
    }
}
