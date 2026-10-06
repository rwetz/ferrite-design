//! Form building blocks: the labelled [`field`] row, the [`number_input`]
//! stepper and the [`select`] value picker.
//!
//! ```text
//!  NAME            ┌────────────────────────────┐
//!                  │ ferrite-atlas              │   any control
//!                  └────────────────────────────┘
//!                  lowercase, dashes allowed       hint (dim) — or the
//!                  ▲ name is taken                 error, in danger
//!
//!  WORKERS         [-]  008  [+]                   number_input
//!  REGION          [ eu-west-1             v ]     select
//! ```
//!
//! All three are controlled, like the toggles: the app owns the value.

use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FocusHandle, InteractiveElement, IntoElement, MouseButton, ParentElement,
    Pixels, RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder as _, px,
};

use super::menu::{dropdown_menu, menu_item};
use crate::fonts::{FerriteText, Scale, display_size};
use crate::icon::{Icon, icon};
use crate::theme::palette;
use crate::tokens::{hsla, text};

// ── Field ─────────────────────────────────────────────────────────────────

/// A form row: a display-face label, the control, and a hint or an error
/// under it. When the error changes to a new message the row shakes once
/// (`shake` is the language's "your input was wrong").
#[derive(IntoElement)]
pub struct Field {
    id: ElementId,
    label: SharedString,
    hint: Option<SharedString>,
    error: Option<SharedString>,
    required: bool,
    stacked: bool,
    label_width: Pixels,
    children: Vec<AnyElement>,
}

pub fn field(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Field {
    Field {
        id: id.into(),
        label: label.into(),
        hint: None,
        error: None,
        required: false,
        stacked: false,
        label_width: px(128.),
        children: Vec::new(),
    }
}

impl Field {
    /// Dim help text under the control. Hidden while there's an error.
    pub fn hint(mut self, hint: impl Into<SharedString>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// An error under the control, in danger, with a warning icon.
    pub fn error(mut self, error: Option<impl Into<SharedString>>) -> Self {
        self.error = error.map(Into::into);
        self
    }

    /// Mark the label with an amber `*`.
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Label above the control instead of beside it (narrow forms).
    pub fn stacked(mut self) -> Self {
        self.stacked = true;
        self
    }

    /// Width of the label column in side-by-side layout (default 128px).
    /// Give every field in a form the same width so controls line up.
    pub fn label_width(mut self, width: Pixels) -> Self {
        self.label_width = width;
        self
    }
}

impl ParentElement for Field {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Field {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let lead = display_size(Scale::X1, window);
        let label = div()
            .flex()
            .flex_row()
            .flex_none()
            .gap_1()
            // Level with the first line of a 28px control.
            .min_h(px(28.))
            .items_center()
            .when(!self.stacked, |el| el.w(self.label_width))
            .display(Scale::X1, window)
            .text_color(hsla(if self.error.is_some() { p.danger } else { p.fg_dim }))
            .child(self.label.to_uppercase())
            .when(self.required, |el| el.child(div().text_color(hsla(p.accent_text)).child("*")));
        let note = match (&self.error, &self.hint) {
            (Some(e), _) => Some(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .text_color(hsla(p.danger))
                    .child(icon(Icon::Warning).fit(lead.min(px(16.))).color(hsla(p.danger)))
                    .child(e.clone()),
            ),
            (None, Some(h)) => Some(div().text_color(hsla(p.fg_dim)).child(h.clone())),
            _ => None,
        };
        let control = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .gap_1()
            .children(self.children)
            .when_some(note, |el, n| el.child(n.body(text::SM)));
        let row = div()
            .id(self.id.clone())
            .role(Role::Group)
            .aria_label(self.label.clone())
            .flex()
            .gap_2()
            .map(|el| if self.stacked { el.flex_col() } else { el.flex_row().items_start() })
            .child(label)
            .child(control);
        // Shake on each *new* error; clearing one is not an event.
        let last = window.use_keyed_state(ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), "err".into()), cx, |_, _| (0u32, None::<SharedString>));
        if self.error.is_some() && last.read(cx).1 != self.error {
            let error = self.error.clone();
            last.update(cx, |(n, e), _| {
                *n += 1;
                *e = error;
            });
        }
        let errors = last.read(cx).0;
        super::fx::shake(ElementId::NamedChild(std::sync::Arc::new(self.id), "shake".into()), errors, row)
    }
}

// ── Number input ──────────────────────────────────────────────────────────

type ValueHandler = Rc<dyn Fn(&f64, &mut Window, &mut App)>;

/// A numeric stepper: `[-]  008  [+]`. The readout is display-face and
/// fixed-width; the value is clamped to the range and snapped to `step`.
/// One tab stop: ↑/→ and ↓/← step, PageUp/PageDown step ×10, Home/End jump
/// to the ends. A new value rolls in (`count`) when it changes.
#[derive(IntoElement)]
pub struct NumberInput {
    id: ElementId,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    digits: usize,
    decimals: usize,
    suffix: Option<SharedString>,
    disabled: bool,
    on_change: Option<ValueHandler>,
}

pub fn number_input(id: impl Into<ElementId>) -> NumberInput {
    NumberInput {
        id: id.into(),
        value: 0.,
        min: f64::MIN,
        max: f64::MAX,
        step: 1.,
        digits: 3,
        decimals: 0,
        suffix: None,
        disabled: false,
        on_change: None,
    }
}

impl NumberInput {
    pub fn value(mut self, value: f64) -> Self {
        self.value = value;
        self
    }

    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = min.min(max);
        self.max = max.max(min);
        self
    }

    pub fn step(mut self, step: f64) -> Self {
        self.step = step.abs().max(f64::EPSILON);
        self
    }

    /// Zero-padded integer digits in the readout (default 3: `008`).
    pub fn digits(mut self, digits: usize) -> Self {
        self.digits = digits.max(1);
        self
    }

    /// Digits after the decimal point (default 0).
    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    /// A dim unit after the readout: `MS`, `%`, `GB`.
    pub fn suffix(mut self, suffix: impl Into<SharedString>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Called with the new (clamped, snapped) value.
    pub fn on_change(mut self, handler: impl Fn(&f64, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

/// `value` moved by `steps` steps, snapped to the step grid from `min`
/// and clamped to the range.
pub fn step_value(value: f64, steps: f64, min: f64, max: f64, step: f64) -> f64 {
    let base = if min.is_finite() { min } else { 0. };
    let raw = value + steps * step;
    let snapped = base + ((raw - base) / step).round() * step;
    snapped.clamp(min, max)
}

/// The readout: zero-padded to `digits` integer digits, `decimals` after
/// the point, sign kept.
pub fn format_number(value: f64, digits: usize, decimals: usize) -> String {
    let width = digits + if decimals > 0 { decimals + 1 } else { 0 };
    if value < 0. {
        format!("-{:0width$.decimals$}", -value, width = width, decimals = decimals)
    } else {
        format!("{:0width$.decimals$}", value, width = width, decimals = decimals)
    }
}

impl RenderOnce for NumberInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let focus: FocusHandle = window.use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle()).read(cx).clone();
        let (value, min, max, step) = (self.value.clamp(self.min, self.max), self.min, self.max, self.step);
        let handler = self.on_change.filter(|_| !self.disabled);
        let set = {
            let handler = handler.clone();
            move |steps: f64, window: &mut Window, cx: &mut App| {
                let next = step_value(value, steps, min, max, step);
                if let Some(h) = &handler
                    && next != value
                {
                    h(&next, window, cx);
                }
            }
        };
        let set = Rc::new(set);
        let readout = format_number(value, self.digits, self.decimals);
        let cell = display_size(Scale::X1, window) / 2.;
        let width = cell * (readout.chars().count() as f32 + 2.);
        let t = crate::animate::play_on_change(ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), "roll".into()), value.to_bits(), crate::motion::FAST, window, cx);
        let shown = crate::animate::scramble(&readout, t);
        let ink = if self.disabled { p.fg_faint } else { p.fg };

        let stepper = |id: &'static str, glyph: Icon, steps: f64, enabled: bool| {
            let set = set.clone();
            div()
                .id(id)
                .flex()
                .items_center()
                .justify_center()
                .size(px(28.))
                .flex_none()
                .border_1()
                .border_color(hsla(if enabled { p.line_strong } else { p.line }))
                .bg(hsla(p.raised))
                .child(icon(glyph).fit(px(16.)).color(hsla(if enabled { p.fg } else { p.fg_faint })))
                .when(enabled, |el| {
                    el.hover(|s| s.bg(hsla(p.line)))
                        .active(|s| s.bg(hsla(p.fg)))
                        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                        .on_click(move |_, window, cx| set(steps, window, cx))
                })
        };
        let interactive = !self.disabled && handler.is_some();

        div()
            .id(self.id.clone())
            .role(Role::SpinButton)
            .aria_label(readout.clone())
            .track_focus(&focus)
            .when(interactive, |el| el.tab_stop(true))
            .flex()
            .flex_row()
            .flex_none()
            .items_center()
            .gap_1()
            .p(px(1.))
            .border_1()
            .border_color(gpui::transparent_black())
            .when(interactive, |el| el.focus_visible(|s| s.border_color(hsla(p.accent))))
            .when(interactive, |el| {
                let set = set.clone();
                el.on_key_down(move |ev, window, cx| {
                    let steps = match ev.keystroke.key.as_str() {
                        "up" | "right" => 1.,
                        "down" | "left" => -1.,
                        "pageup" => 10.,
                        "pagedown" => -10.,
                        "home" if min.is_finite() => f64::MIN / 4.,
                        "end" if max.is_finite() => f64::MAX / 4.,
                        _ => return,
                    };
                    cx.stop_propagation();
                    set(steps, window, cx);
                })
            })
            .child(stepper("dec", Icon::Minus, -1., interactive && value > min))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_center()
                    .h(px(28.))
                    .w(width)
                    .bg(hsla(p.sunken))
                    .border_1()
                    .border_color(hsla(p.line_strong))
                    .display(Scale::X1, window)
                    .text_color(hsla(ink))
                    .child(shown),
            )
            .child(stepper("inc", Icon::Plus, 1., interactive && value < max))
            .when_some(self.suffix, |el, s| el.child(div().pl_1().display(Scale::X1, window).text_color(hsla(p.fg_dim)).child(s.to_uppercase())))
    }
}

// ── Select ────────────────────────────────────────────────────────────────

type SelectHandler = Rc<dyn Fn(&usize, &mut Window, &mut App)>;

/// A form value picker: a sunken well showing the current choice and a
/// chevron, opening a menu of options with a check on the current one.
/// Everything a dropdown menu does (keyboard, unroll, dismissal) it does.
#[derive(IntoElement)]
pub struct Select {
    id: ElementId,
    options: Vec<(SharedString, Option<Icon>)>,
    selected: Option<usize>,
    placeholder: SharedString,
    width: Pixels,
    disabled: bool,
    on_change: Option<SelectHandler>,
}

pub fn select(id: impl Into<ElementId>) -> Select {
    Select {
        id: id.into(),
        options: Vec::new(),
        selected: None,
        placeholder: "Choose…".into(),
        width: px(240.),
        disabled: false,
        on_change: None,
    }
}

impl Select {
    pub fn option(mut self, label: impl Into<SharedString>) -> Self {
        self.options.push((label.into(), None));
        self
    }

    pub fn option_with_icon(mut self, label: impl Into<SharedString>, icon: Icon) -> Self {
        self.options.push((label.into(), Some(icon)));
        self
    }

    pub fn options(mut self, labels: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.options.extend(labels.into_iter().map(|l| (l.into(), None)));
        self
    }

    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    /// Shown, dim, while nothing is selected.
    pub fn placeholder(mut self, text: impl Into<SharedString>) -> Self {
        self.placeholder = text.into();
        self
    }

    pub fn width(mut self, width: Pixels) -> Self {
        self.width = width;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Called with the chosen index (also when it's the current one).
    pub fn on_change(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Select {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let trigger_id = ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), "well".into());
        let focus: FocusHandle = window.use_keyed_state(trigger_id.clone(), cx, |_, cx| cx.focus_handle()).read(cx).clone();
        let current = self.selected.and_then(|i| self.options.get(i)).cloned();
        let t = crate::animate::play_on_change(ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), "pick".into()), self.selected, crate::motion::FAST, window, cx);
        let (text_ink, label) = match &current {
            Some((l, _)) => (if self.disabled { p.fg_faint } else { p.fg }, crate::animate::scramble(l, t)),
            None => (p.fg_faint, self.placeholder.to_string()),
        };
        let well = div()
            .id(trigger_id)
            .role(Role::ComboBox)
            .aria_label(current.as_ref().map(|c| c.0.clone()).unwrap_or(self.placeholder.clone()))
            .track_focus(&focus)
            .when(!self.disabled, |el| el.tab_stop(true))
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .w(self.width)
            .h(px(28.))
            .pl_2()
            .pr_1()
            .bg(hsla(p.sunken))
            .border_1()
            .map(|el| if self.disabled { el.border_dashed().border_color(hsla(p.line_strong)) } else { el.border_color(hsla(p.line_strong)) })
            .when(!self.disabled, |el| el.hover(|s| s.border_color(hsla(p.fg_faint))).focus_visible(|s| s.border_color(hsla(p.accent))))
            .when_some(current.as_ref().and_then(|c| c.1), |el, i| el.child(icon(i).fit(px(16.)).color(hsla(p.fg_dim))))
            .child(div().flex_1().min_w_0().overflow_hidden().whitespace_nowrap().text_ellipsis().body(text::BASE).text_color(hsla(text_ink)).child(label))
            .child(icon(Icon::ChevronDown).fit(px(16.)).color(hsla(if self.disabled { p.fg_faint } else { p.fg_dim })));

        if self.disabled {
            return div().id(self.id).child(well).into_any_element();
        }
        let mut menu = dropdown_menu(self.id.clone()).width(self.width).trigger(well);
        for (i, (label, glyph)) in self.options.into_iter().enumerate() {
            let handler = self.on_change.clone();
            let mut item = menu_item(label).checked(self.selected == Some(i));
            if let Some(g) = glyph {
                item = item.icon(g);
            }
            menu = menu.item(item.on_select(move |window, cx| {
                if let Some(h) = &handler {
                    h(&i, window, cx);
                }
            }));
        }
        menu.into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_snap_and_clamp() {
        assert_eq!(step_value(5., 1., 0., 10., 1.), 6.);
        assert_eq!(step_value(10., 1., 0., 10., 1.), 10.);
        assert_eq!(step_value(0., -1., 0., 10., 1.), 0.);
        // Off-grid values land back on the grid from `min`.
        assert_eq!(step_value(0.33, 1., 0., 1., 0.25), 0.5); // 0.58 → 0.5
        assert_eq!(step_value(0.9, 1., 0., 1., 0.25), 1.); // clamped
        assert_eq!(step_value(3., 10., 0., 100., 5.), 55.);
        // Home/End sentinels pin to the ends.
        assert_eq!(step_value(42., f64::MIN / 4., 0., 100., 1.), 0.);
        assert_eq!(step_value(42., f64::MAX / 4., 0., 100., 1.), 100.);
    }

    #[test]
    fn readout_is_fixed_width() {
        assert_eq!(format_number(8., 3, 0), "008");
        assert_eq!(format_number(12.5, 2, 1), "12.5");
        assert_eq!(format_number(1.5, 3, 2), "001.50");
        assert_eq!(format_number(-4., 2, 0), "-04");
    }
}
