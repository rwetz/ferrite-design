//! Data display: [`avatar`], [`stat`], [`property_list`] and [`timeline`].
//!
//! ```text
//!  ▓░▓       [ CPU ]              ▌ name      ferrite-atlas      ● 12:04  deployed
//!  ░▓░       31.0%  ▲ 2.1%        ▌ pid       4412               │        staging · 4412
//!  ▓▓▓       ▁▂▃▅▃▆█              ▌ state     run                ○ 11:58  build failed
//!  avatar    stat                 property_list                 timeline
//! ```

use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, Pixels, RenderOnce, SharedString, Styled, Window,
    div, prelude::FluentBuilder as _, px,
};

use super::chart::sparkline;
use super::tag::Tone;
use crate::fonts::{FerriteText, Scale, display_size};
use crate::icon::{Icon, icon};
use crate::theme::palette;
use crate::tokens::{hsla, text};

// ── Avatar ────────────────────────────────────────────────────────────────

/// A 5×5 mirror-symmetric identicon for `seed`, row-major: the same name
/// always draws the same face, and no two neighbours in a list look alike.
/// At least a few cells are always inked.
pub fn identicon(seed: &str) -> [bool; 25] {
    // FNV-1a: stable across runs and platforms, unlike DefaultHasher.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in seed.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    let mut cells = [false; 25];
    // Fifteen bits fill the left three columns; the right two mirror them.
    for (bit, (row, col)) in (0..5).flat_map(|r| (0..3).map(move |c| (r, c))).enumerate() {
        let on = (h >> bit) & 1 == 1;
        cells[row * 5 + col] = on;
        cells[row * 5 + (4 - col)] = on;
    }
    if cells.iter().filter(|c| **c).count() < 5 {
        // Too sparse to read as a face: fill the spine.
        for row in 0..5 {
            cells[row * 5 + 2] = true;
        }
    }
    cells
}

/// Presence, as a small square in the corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presence {
    Online,
    Away,
    Busy,
    Offline,
}

/// A user mark: a pixel identicon from the name (default) or the initials
/// in the display face. Square, like everything.
#[derive(IntoElement)]
pub struct Avatar {
    name: SharedString,
    size: Pixels,
    initials: bool,
    presence: Option<Presence>,
}

pub fn avatar(name: impl Into<SharedString>) -> Avatar {
    Avatar { name: name.into(), size: px(32.), initials: false, presence: None }
}

impl Avatar {
    /// Edge length (default 32px). Identicon cells snap to whole pixels.
    pub fn size(mut self, size: Pixels) -> Self {
        self.size = size;
        self
    }

    /// Show up to two initials instead of the identicon.
    pub fn initials(mut self) -> Self {
        self.initials = true;
        self
    }

    pub fn presence(mut self, presence: Presence) -> Self {
        self.presence = Some(presence);
        self
    }
}

/// Up to two initials: first letters of the first two words, uppercase.
pub fn initials_of(name: &str) -> String {
    let mut words = name.split(|c: char| c.is_whitespace() || c == '-' || c == '_' || c == '.').filter(|w| !w.is_empty());
    let first = words.next().and_then(|w| w.chars().next());
    let second = words.next().and_then(|w| w.chars().next());
    first.into_iter().chain(second).flat_map(char::to_uppercase).collect()
}

impl RenderOnce for Avatar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let edge = f32::from(self.size);
        // Whole-pixel cells with a one-cell margin: 7 cells across.
        let cell = px((edge / 7.).floor().max(1.));
        let face = div()
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .flex_none()
            .size(self.size)
            .bg(hsla(p.raised))
            .border_1()
            .border_color(hsla(p.line_strong))
            .map(|el| {
                if self.initials {
                    el.display(Scale::X1, window).text_color(hsla(p.fg)).child(initials_of(&self.name))
                } else {
                    let cells = identicon(&self.name);
                    el.child(div().flex().flex_col().children((0..5).map(|r| {
                        div().flex().flex_row().children((0..5).map(move |c| {
                            div().size(cell).when(cells[r * 5 + c], |el| el.bg(hsla(p.fg_dim)))
                        }))
                    })))
                }
            });
        div().relative().flex_none().child(face).when_some(self.presence, |el, presence| {
            let color = match presence {
                Presence::Online => p.success,
                Presence::Away => p.warning,
                Presence::Busy => p.danger,
                Presence::Offline => p.fg_faint,
            };
            let dot = (self.size * 0.3).max(px(6.)).min(px(10.));
            el.child(div().absolute().right(-px(2.)).bottom(-px(2.)).size(dot).bg(hsla(color)).border_1().border_color(hsla(p.bg)))
        })
    }
}

// ── Stat ──────────────────────────────────────────────────────────────────

/// A KPI tile: a label, a big value that decrypts in when it changes, an
/// optional delta (▲ green / ▼ red, or the reverse with `.lower_is_better()`)
/// and an optional sparkline trend.
#[derive(IntoElement)]
pub struct Stat {
    id: ElementId,
    label: SharedString,
    value: SharedString,
    delta: Option<f32>,
    delta_suffix: SharedString,
    lower_is_better: bool,
    trend: Option<Vec<f32>>,
    caption: Option<SharedString>,
}

pub fn stat(id: impl Into<ElementId>, label: impl Into<SharedString>, value: impl Into<SharedString>) -> Stat {
    Stat {
        id: id.into(),
        label: label.into(),
        value: value.into(),
        delta: None,
        delta_suffix: "%".into(),
        lower_is_better: false,
        trend: None,
        caption: None,
    }
}

impl Stat {
    /// Change since the last period, e.g. `2.1` for +2.1%.
    pub fn delta(mut self, delta: f32) -> Self {
        self.delta = Some(delta);
        self
    }

    /// Unit after the delta (default `%`).
    pub fn delta_suffix(mut self, suffix: impl Into<SharedString>) -> Self {
        self.delta_suffix = suffix.into();
        self
    }

    /// Down is good (latency, errors, cost): flips the delta's colors.
    pub fn lower_is_better(mut self) -> Self {
        self.lower_is_better = true;
        self
    }

    pub fn trend(mut self, values: impl IntoIterator<Item = f32>) -> Self {
        self.trend = Some(values.into_iter().collect());
        self
    }

    /// A dim line under everything ("vs last week").
    pub fn caption(mut self, caption: impl Into<SharedString>) -> Self {
        self.caption = Some(caption.into());
        self
    }
}

impl RenderOnce for Stat {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let t = crate::animate::play(ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), "value".into()), &self.value, crate::motion::BASE, window, cx);
        let delta = self.delta.map(|d| {
            let good = (d >= 0.) != self.lower_is_better;
            let tone = if d == 0. { p.fg_dim } else if good { p.success } else { p.danger };
            let glyph = if d > 0. { Icon::Up } else if d < 0. { Icon::Down } else { Icon::Minus };
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_1()
                .body(text::SM)
                .text_color(hsla(tone))
                .child(icon(glyph).fit(px(16.)).color(hsla(tone)))
                .child(format!("{:.1}{}", d.abs(), self.delta_suffix))
        });
        div()
            .flex()
            .flex_col()
            .h_full()
            .gap_1()
            .p_3()
            .bg(hsla(p.surface))
            .border_1()
            .border_color(hsla(p.line))
            .child(div().display(Scale::X1, window).text_color(hsla(p.fg_dim)).child(format!("[ {} ]", self.label.to_uppercase())))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_end()
                    .gap_3()
                    .child(div().display(Scale::X2, window).text_color(hsla(p.fg)).child(crate::animate::scramble(&self.value, t)))
                    .children(delta),
            )
            .when_some(self.trend, |el, values| el.child(sparkline(values).size(px(120.), px(20.))))
            .when_some(self.caption, |el, c| el.child(div().body(text::XS).text_color(hsla(p.fg_faint)).child(c)))
    }
}

// ── Property list ─────────────────────────────────────────────────────────

/// Key/value rows for an inspector or a details pane: dim keys in a fixed
/// column, values in body type, optionally toned or monospaced-aligned.
#[derive(IntoElement)]
pub struct PropertyList {
    rows: Vec<(SharedString, AnyElement)>,
    key_width: Pixels,
}

pub fn property_list() -> PropertyList {
    PropertyList { rows: Vec::new(), key_width: px(112.) }
}

impl PropertyList {
    /// A row with a text value.
    pub fn row(mut self, key: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        let value: SharedString = value.into();
        self.rows.push((key.into(), div().child(value).into_any_element()));
        self
    }

    /// A row whose value is any element (a tag, a meter, a link).
    pub fn row_with(mut self, key: impl Into<SharedString>, value: impl IntoElement) -> Self {
        self.rows.push((key.into(), value.into_any_element()));
        self
    }

    pub fn key_width(mut self, width: Pixels) -> Self {
        self.key_width = width;
        self
    }
}

impl RenderOnce for PropertyList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let key_width = self.key_width;
        div().flex().flex_col().children(self.rows.into_iter().enumerate().map(move |(i, (k, v))| {
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_3()
                .min_h(px(26.))
                .when(i > 0, |el| el.border_t_1().border_color(hsla(p.line)))
                .child(div().w(key_width).flex_none().body(text::SM).text_color(hsla(p.fg_dim)).child(k))
                // A flex row, so a tag or a meter keeps its own width.
                .child(div().flex().flex_row().items_center().flex_1().min_w_0().body(text::SM).text_color(hsla(p.fg)).child(v))
        }))
    }
}

// ── Timeline ──────────────────────────────────────────────────────────────

/// One event: a time, a title, optional detail, and a tone for its marker.
pub struct TimelineEvent {
    time: SharedString,
    title: SharedString,
    detail: Option<SharedString>,
    tone: Tone,
}

pub fn event(time: impl Into<SharedString>, title: impl Into<SharedString>) -> TimelineEvent {
    TimelineEvent { time: time.into(), title: title.into(), detail: None, tone: Tone::Neutral }
}

impl TimelineEvent {
    pub fn detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }
}

/// An activity feed: square markers on a 1px rail, newest first. Events
/// cascade in, one frame apart, when the feed first appears; a new event at
/// the top pings its marker.
#[derive(IntoElement)]
pub struct Timeline {
    id: ElementId,
    events: Vec<TimelineEvent>,
    time_width: Pixels,
}

pub fn timeline(id: impl Into<ElementId>) -> Timeline {
    Timeline { id: id.into(), events: Vec::new(), time_width: px(48.) }
}

impl Timeline {
    /// Width of the time column (default 48px, enough for `12:04`). Widen it
    /// for dates: `px(80.)` fits `2026-10-05`.
    pub fn time_width(mut self, width: Pixels) -> Self {
        self.time_width = width;
        self
    }

    pub fn event(mut self, event: TimelineEvent) -> Self {
        self.events.push(event);
        self
    }

    pub fn events(mut self, events: impl IntoIterator<Item = TimelineEvent>) -> Self {
        self.events.extend(events);
        self
    }
}

impl RenderOnce for Timeline {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let lead = display_size(Scale::X1, window);
        let n = self.events.len();
        let newest = self.events.first().map(|e| format!("{}{}", e.time, e.title)).unwrap_or_default();
        let mut feed = super::fx::cascade_in(self.id.clone(), 0u8).flex().flex_col();
        for (i, e) in self.events.into_iter().enumerate() {
            let marker_ink = match e.tone {
                Tone::Neutral => p.fg_dim,
                Tone::Accent => p.accent,
                Tone::Success => p.success,
                Tone::Warning => p.warning,
                Tone::Danger => p.danger,
            };
            let marker = div().size(px(8.)).bg(hsla(marker_ink));
            let marker = if i == 0 {
                super::fx::ping(ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), "ping".into()), &newest, marker).ink(hsla(marker_ink)).into_any_element()
            } else {
                marker.into_any_element()
            };
            feed = feed.child(
                div()
                    .flex()
                    .flex_row()
                    .gap_3()
                    .child(div().w(self.time_width).flex_none().pt(px(2.)).whitespace_nowrap().body(text::XS).text_color(hsla(p.fg_faint)).child(e.time))
                    .child(
                        // The rail: a marker, then a line down to the next one.
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .w(px(8.))
                            .flex_none()
                            .pt(px(5.))
                            .child(marker)
                            .when(i + 1 < n, |el| el.child(div().flex_1().w(px(1.)).bg(hsla(p.line_strong)))),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .pb_3()
                            .min_h(lead + px(8.))
                            .child(div().body(text::SM).text_color(hsla(p.fg)).child(e.title))
                            .when_some(e.detail, |el, d| el.child(div().body(text::XS).text_color(hsla(p.fg_dim)).child(d))),
                    ),
            );
        }
        feed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identicons_are_stable_symmetric_and_never_blank() {
        for name in ["ryan", "ferrite-atlas", "", "a", "zz-top"] {
            let face = identicon(name);
            assert_eq!(face, identicon(name), "stable");
            for r in 0..5 {
                for c in 0..2 {
                    assert_eq!(face[r * 5 + c], face[r * 5 + 4 - c], "mirror-symmetric");
                }
            }
            assert!(face.iter().filter(|c| **c).count() >= 5, "{name:?} too sparse");
        }
        assert_ne!(identicon("alice"), identicon("bob"));
    }

    #[test]
    fn initials_take_the_first_two_words() {
        assert_eq!(initials_of("Ryan Wetzstein"), "RW");
        assert_eq!(initials_of("ferrite-atlas"), "FA");
        assert_eq!(initials_of("solo"), "S");
        assert_eq!(initials_of("  "), "");
    }
}
