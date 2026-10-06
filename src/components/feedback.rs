//! Inline feedback: the [`alert`] callout and [`skeleton`] loading
//! placeholders.
//!
//! ```text
//!  ▌ ▲ [ DISK ALMOST FULL ]                         [x]   alert: tone bar,
//!  ▌   92% of C: used. Old builds can be pruned.          icon, title,
//!  ▌   [ PRUNE ] [ DETAILS ]                              message, actions
//!
//!  ░░░░░░░░░░░░▒▒▒▒░░░░░░░░░░░░░░░░░░                    skeleton: a light
//!  ░░░░░░░░░░░░░░░░░░░░░                                 dither block with a
//!                                                        stepped scan band
//! ```
//!
//! An alert is the persistent sibling of a toast: it lives in the layout,
//! next to what it's about, until the condition clears or it's dismissed.

use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, Pixels, RenderOnce, Role,
    SharedString, StatefulInteractiveElement, Styled, StyleRefinement, Window, div, prelude::FluentBuilder as _, px,
    relative,
};

use super::ticker::ticker;
use crate::dither::{self, dither};
use crate::fonts::{FerriteText, Scale};
use crate::icon::{Icon, icon};
use crate::motion;
use crate::theme::palette;
use crate::tokens::{hsla, text};

pub use super::tag::Tone;

// ── Alert ─────────────────────────────────────────────────────────────────

type Handler = Rc<dyn Fn(&mut Window, &mut App)>;

/// An inline callout. Tones: neutral (info — there is no blue), accent
/// (something live or new), success, warning, danger. The title decrypts
/// in when the alert appears and whenever it changes.
#[derive(IntoElement)]
pub struct Alert {
    id: ElementId,
    tone: Tone,
    title: SharedString,
    message: Option<SharedString>,
    actions: Vec<AnyElement>,
    on_close: Option<Handler>,
}

pub fn alert(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Alert {
    Alert { id: id.into(), tone: Tone::Neutral, title: title.into(), message: None, actions: Vec::new(), on_close: None }
}

impl Alert {
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    pub fn accent(self) -> Self {
        self.tone(Tone::Accent)
    }

    pub fn success(self) -> Self {
        self.tone(Tone::Success)
    }

    pub fn warning(self) -> Self {
        self.tone(Tone::Warning)
    }

    pub fn danger(self) -> Self {
        self.tone(Tone::Danger)
    }

    pub fn message(mut self, message: impl Into<SharedString>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// A button (or anything) in the action row under the message.
    pub fn action(mut self, el: impl IntoElement) -> Self {
        self.actions.push(el.into_any_element());
        self
    }

    /// Show a close button; the app removes the alert in the handler.
    pub fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Alert {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let (bar, ink, glyph) = match self.tone {
            Tone::Neutral => (p.fg_dim, p.fg, Icon::Info),
            Tone::Accent => (p.accent, p.accent_text, Icon::Bell),
            Tone::Success => (p.success, p.success, Icon::Check),
            Tone::Warning => (p.warning, p.warning, Icon::Warning),
            Tone::Danger => (p.danger, p.danger, Icon::Warning),
        };
        let t = crate::animate::play(ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), "title".into()), &self.title, motion::BASE, window, cx);
        let title = crate::animate::scramble(&format!("[ {} ]", self.title.to_uppercase()), t);
        div()
            .id(self.id.clone())
            .role(if matches!(self.tone, Tone::Danger | Tone::Warning) { Role::Alert } else { Role::Status })
            .aria_label(self.title.clone())
            .relative()
            .flex()
            .flex_row()
            .items_start()
            .gap_2()
            .py_2()
            .pl(px(14.))
            .pr_2()
            .bg(hsla(p.raised))
            .border_1()
            .border_color(hsla(p.line))
            // The tone bar: gpui has no per-side border colors (PITFALLS §19).
            .child(div().absolute().left_0().top_0().bottom_0().w(px(2.)).bg(hsla(bar)))
            .child(div().pt(px(4.)).child(icon(glyph).fit(px(16.)).color(hsla(bar))))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap_1()
                    .child(div().display(Scale::X1, window).text_color(hsla(ink)).child(title))
                    .when_some(self.message, |el, m| el.child(div().body(text::SM).text_color(hsla(p.fg_dim)).child(m)))
                    .when(!self.actions.is_empty(), |el| el.child(div().flex().flex_row().flex_wrap().gap_2().pt_1().children(self.actions))),
            )
            .when_some(self.on_close, |el, close| {
                el.child(
                    div()
                        .id("close")
                        .role(Role::Button)
                        .aria_label("Dismiss")
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(24.))
                        .hover(|s| s.bg(hsla(p.line)))
                        .child(icon(Icon::Close).fit(px(16.)).color(hsla(p.fg_dim)))
                        .on_click(move |_, window, cx| close(window, cx)),
                )
            })
    }
}

// ── Skeleton ──────────────────────────────────────────────────────────────

/// Where the scan band is at `step` of a sweep of `steps`, as the band's
/// left edge in fractions of the width (it starts and ends off the edge).
pub fn sweep_position(step: u64, steps: u64, band: f32) -> f32 {
    let steps = steps.max(1);
    let t = (step % steps) as f32 / steps as f32;
    -band + t * (1. + band)
}

/// A loading placeholder in the shape of the content to come: a `sunken`
/// block under a light dither, with a medium-dither band stepping across it
/// a few times a second (a timer, not per-frame redraws; still under
/// reduced motion). Size it like any element. For a paragraph, use
/// [`skeleton_text`].
#[derive(IntoElement)]
pub struct Skeleton {
    id: ElementId,
    style: StyleRefinement,
}

pub fn skeleton(id: impl Into<ElementId>) -> Skeleton {
    Skeleton { id: id.into(), style: StyleRefinement::default() }
}

impl Styled for Skeleton {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// The scan band steps 20 times per sweep, at 12.5Hz: a 1.6s cycle.
const SWEEP_STEPS: u64 = 20;
const BAND: f32 = 0.25;

impl RenderOnce for Skeleton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let still = motion::reduced(cx);
        let step = ticker(self.id.clone(), motion::FRAME * 2, window, cx);
        let mut root = div();
        *root.style() = self.style;
        root.relative()
            .overflow_hidden()
            .min_h(px(12.))
            .bg(hsla(p.sunken))
            .child(div().absolute().inset_0().child(dither(dither::flat(dither::level::LIGHT)).ink(hsla(p.line)).size_full()))
            .when(!still, |el| {
                el.child(
                    div()
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left(relative(sweep_position(step, SWEEP_STEPS, BAND)))
                        .w(relative(BAND))
                        .child(dither(dither::horizontal(dither::level::LIGHT, dither::level::MEDIUM)).ink(hsla(p.line_strong)).size_full()),
                )
            })
    }
}

/// `lines` skeleton bars shaped like a paragraph: full width, the last one
/// shorter. All share one sweep.
pub fn skeleton_text(id: impl Into<ElementId>, lines: usize, line_height: Pixels) -> impl IntoElement {
    let id: ElementId = id.into();
    div().flex().flex_col().gap_2().w_full().children((0..lines.max(1)).map(move |i| {
        let last = i + 1 == lines.max(1);
        skeleton(ElementId::NamedChild(std::sync::Arc::new(id.clone()), format!("l{i}").into()))
            .h(line_height)
            .w(relative(if last && lines > 1 { 0.6 } else { 1. }))
    }))
}

#[cfg(test)]
mod tests {
    use super::sweep_position;

    #[test]
    fn the_band_sweeps_from_off_left_to_the_far_edge() {
        assert_eq!(sweep_position(0, 20, 0.25), -0.25);
        assert!(sweep_position(19, 20, 0.25) < 1.0);
        assert_eq!(sweep_position(20, 20, 0.25), -0.25, "wraps");
        assert!(sweep_position(10, 20, 0.25) > 0.);
    }
}
