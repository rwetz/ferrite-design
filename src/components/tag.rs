//! Tags and the segmented meter (docs/COMPONENTS.md #4).

use gpui::{
    App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
    prelude::FluentBuilder as _, px,
};

use crate::fonts::FerriteText;
use crate::theme::palette;
use crate::tokens::{hsla, text};

// ── Tag ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tone {
    /// Amber: live, active, selected.
    Accent,
    #[default]
    Neutral,
    Danger,
    Success,
    Warning,
}

/// A square status label. Solid tags are inverse video (colored block, `bg`
/// ink); outline tags are a 1px frame in the tone. Uppercase body type —
/// tags sit inside dense rows where display type would be too tall.
#[derive(IntoElement)]
pub struct Tag {
    label: SharedString,
    tone: Tone,
    outline: bool,
}

pub fn tag(label: impl Into<SharedString>) -> Tag {
    Tag { label: label.into(), tone: Tone::Neutral, outline: false }
}

impl Tag {
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    pub fn accent(self) -> Self {
        self.tone(Tone::Accent)
    }

    pub fn danger(self) -> Self {
        self.tone(Tone::Danger)
    }

    pub fn success(self) -> Self {
        self.tone(Tone::Success)
    }

    pub fn warning(self) -> Self {
        self.tone(Tone::Warning)
    }

    pub fn outline(mut self) -> Self {
        self.outline = true;
        self
    }
}

impl RenderOnce for Tag {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let (fill, on_fill, as_text) = match self.tone {
            Tone::Accent => (p.accent, p.accent_fg, p.accent_text),
            Tone::Neutral => (p.line_strong, p.fg, p.fg_dim),
            Tone::Danger => (p.danger, p.danger_fg, p.danger),
            Tone::Success => (p.success, p.bg, p.success),
            Tone::Warning => (p.warning, p.bg, p.warning),
        };
        div()
            .flex_shrink_0()
            .px(px(6.))
            .border_1()
            .body(text::XS)
            .font_weight(gpui::FontWeight::BOLD)
            .map(|el| {
                if self.outline {
                    el.border_color(hsla(as_text)).text_color(hsla(as_text))
                } else {
                    el.border_color(hsla(fill)).bg(hsla(fill)).text_color(hsla(on_fill))
                }
            })
            .child(self.label.to_uppercase())
    }
}

// ── Meter ─────────────────────────────────────────────────────────────────

/// A segmented LED meter: `▮▮▮▮▮▮▯▯▯▯ 62%`. Segments past `warn` light in
/// the warning color and past `critical` in danger, like a VU meter.
/// Cheap: one quad per segment, no dither, no state.
#[derive(IntoElement)]
pub struct Meter {
    value: f32,
    segments: u32,
    warn: f32,
    critical: f32,
    label: Option<SharedString>,
    show_value: bool,
    roll: bool,
}

pub fn meter(value: f32) -> Meter {
    Meter { value, segments: 20, warn: 0.7, critical: 0.9, label: None, show_value: true, roll: false }
}

impl Meter {
    pub fn segments(mut self, n: u32) -> Self {
        self.segments = n.max(1);
        self
    }

    /// Fractions where segments turn warning / danger. `thresholds(1.1, 1.1)`
    /// keeps the meter all-amber.
    pub fn thresholds(mut self, warn: f32, critical: f32) -> Self {
        self.warn = warn;
        self.critical = critical;
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn hide_value(mut self) -> Self {
        self.show_value = false;
        self
    }

    /// Roll to new values segment by segment instead of jumping. For values
    /// that change as *events* (a job finishing, a quota used). Leave it off
    /// for live feeds that update several times a second: continuous data
    /// doesn't animate (DESIGN_LANGUAGE §6.2), and a meter that's always
    /// rolling keeps the window redrawing at 25fps.
    pub fn roll(mut self, roll: bool) -> Self {
        self.roll = roll;
        self
    }
}

/// How many of `segments` are lit for `value`. Rounds, so a meter never
/// shows empty for a nonzero value that rounds up, nor full below 100%.
pub fn lit_segments(value: f32, segments: u32) -> u32 {
    let v = value.clamp(0.0, 1.0);
    let lit = (v * segments as f32).round() as u32;
    if v > 0.0 && lit == 0 {
        1
    } else if v < 1.0 && lit == segments {
        segments - 1
    } else {
        lit
    }
}

/// Where a meter is rolling from / to.
struct Roll {
    from: f32,
    to: f32,
}

impl RenderOnce for Meter {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        // Roll to the new value segment by segment (LED-style) instead of
        // jumping. Keyed by label: give meters in one view distinct labels.
        let key = format!("meter-{}", self.label.clone().unwrap_or_default());
        let value = self.value.clamp(0., 1.);
        let roll = window.use_keyed_state(gpui::ElementId::Name(format!("{key}-roll").into()), cx, move |_, _| Roll { from: value, to: value });
        // Without `roll`, the clip never changes key, so it never runs.
        let clip_key = if self.roll { value.to_bits() } else { 0 };
        let t = crate::animate::play_on_change(gpui::ElementId::Name(key.into()), clip_key, crate::motion::SLOW, window, cx);
        if roll.read(cx).to != value {
            let (from, to) = (roll.read(cx).from, roll.read(cx).to);
            let showing = crate::animate::count(from, to, t);
            roll.update(cx, |r, _| {
                r.from = if t.done { to } else { showing };
                r.to = value;
            });
        }
        let shown = if self.roll {
            let r = roll.read(cx);
            crate::animate::count(r.from, r.to, t)
        } else {
            value
        };
        let lit = lit_segments(shown, self.segments);
        let n = self.segments;
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .when_some(self.label, |el, l| {
                el.child(div().body(text::SM).w(px(48.)).text_color(hsla(p.fg_dim)).child(l.to_uppercase()))
            })
            .child(div().flex().flex_row().gap(px(2.)).children((0..n).map(|i| {
                let at = (i as f32 + 1.0) / n as f32;
                let color = if i >= lit {
                    p.line
                } else if at > self.critical {
                    p.danger
                } else if at > self.warn {
                    p.warning
                } else {
                    p.accent
                };
                div().w(px(6.)).h(px(14.)).bg(hsla(color))
            })))
            .when(self.show_value, |el| {
                el.child(
                    div()
                        .body(text::SM)
                        .text_color(hsla(p.fg))
                        .child(format!("{:>3}%", (shown * 100.0).round() as u32)),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::lit_segments;

    #[test]
    fn meter_never_lies_at_the_ends() {
        assert_eq!(lit_segments(0.0, 10), 0);
        assert_eq!(lit_segments(0.01, 10), 1, "nonzero must show something");
        assert_eq!(lit_segments(0.99, 10), 9, "below 100% must not look full");
        assert_eq!(lit_segments(1.0, 10), 10);
        assert_eq!(lit_segments(0.5, 10), 5);
    }
}
