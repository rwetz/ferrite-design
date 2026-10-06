//! The slider: a value in a range, set by dragging along a row of cells.
//!
//! ```text
//!  ▮▮▮▮▮▮▮▮▮▮█▯▯▯▯▯▯▯▯▯▯▯▯▯   045
//!  └ lit ──┘ │ └ unlit ─────┘    readout (display face, fixed width)
//!          thumb
//! ```
//!
//! Like the meter, the track is discrete LED cells, not a smooth bar: the
//! value snaps to `step`, and the lit run moves a whole cell at a time. The
//! thumb is the one tall cell in the foreground color.
//!
//! Controlled: pass `.value(v)`, get the new value in `on_change`. Click or
//! drag anywhere on the track (the drag keeps tracking outside it — the
//! pointer is captured); ←/→ (and ↓/↑) step, PageUp/PageDown take 10 steps,
//! Home/End jump to the ends.

use std::rc::Rc;

use gpui::{
    App, Bounds, ElementId, FocusHandle, HitboxBehavior, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, MouseMoveEvent, ParentElement, Pixels, RenderOnce, Role,
    SharedString, StatefulInteractiveElement, Styled, Window, canvas, div,
    prelude::FluentBuilder as _, px,
};

use crate::fonts::{FerriteText, Scale};
use crate::theme::palette;
use crate::tokens::hsla;

type ChangeHandler = Rc<dyn Fn(&f32, &mut Window, &mut App)>;
type Format = Rc<dyn Fn(f32) -> SharedString>;

/// Cell pitch: a 6px cell and a 2px gap.
const PITCH: f32 = 8.;

#[derive(IntoElement)]
pub struct Slider {
    id: ElementId,
    label: Option<SharedString>,
    min: f32,
    max: f32,
    step: f32,
    value: f32,
    width: Pixels,
    disabled: bool,
    format: Option<Format>,
    on_change: Option<ChangeHandler>,
}

pub fn slider(id: impl Into<ElementId>) -> Slider {
    Slider {
        id: id.into(),
        label: None,
        min: 0.,
        max: 100.,
        step: 1.,
        value: 0.,
        width: px(240.),
        disabled: false,
        format: None,
        on_change: None,
    }
}

impl Slider {
    /// Accessible name (shown nowhere; put a visible label beside it).
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.min = min.min(max);
        self.max = max.max(min);
        self
    }

    pub fn step(mut self, step: f32) -> Self {
        self.step = step.abs().max(f32::EPSILON);
        self
    }

    pub fn value(mut self, value: f32) -> Self {
        self.value = value;
        self
    }

    /// Track width. The cell count follows (one cell per 8px).
    pub fn width(mut self, width: Pixels) -> Self {
        self.width = width;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// How the readout prints the value, e.g. `|v| format!("{v:.0}%").into()`.
    pub fn format(mut self, format: impl Fn(f32) -> SharedString + 'static) -> Self {
        self.format = Some(Rc::new(format));
        self
    }

    /// Called with the new (snapped) value, only when it changes.
    pub fn on_change(mut self, handler: impl Fn(&f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

/// `value` snapped to the step grid from `min` and clamped into range.
pub fn snap(value: f32, min: f32, max: f32, step: f32) -> f32 {
    let steps = ((value - min) / step).round();
    let v = min + steps * step;
    // Clamp, and land exactly on `max` when the grid doesn't divide the range.
    if v >= max - step * 0.5 && value >= max - step * 0.5 { max } else { v.clamp(min, max) }
}

/// Fraction of the range `value` sits at.
fn fraction(value: f32, min: f32, max: f32) -> f32 {
    if max <= min { 0. } else { ((value - min) / (max - min)).clamp(0., 1.) }
}

/// The value a key moves to, if it's a slider key.
fn key_value(key: &str, value: f32, min: f32, max: f32, step: f32) -> Option<f32> {
    let v = match key {
        "left" | "down" => value - step,
        "right" | "up" => value + step,
        "pageup" => value + step * 10.,
        "pagedown" => value - step * 10.,
        "home" => min,
        "end" => max,
        _ => return None,
    };
    Some(snap(v, min, max, step))
}

fn default_format(step: f32) -> Format {
    // As many decimals as the step needs (0.25 → 2, 0.5 → 1, 5 → 0).
    let decimals = (0..4).find(|&d| (step * 10f32.powi(d)).fract().abs() < 1e-4).unwrap_or(4) as usize;
    Rc::new(move |v| format!("{v:.decimals$}").into())
}

struct SliderState {
    focus: FocusHandle,
    dragging: bool,
}

impl RenderOnce for Slider {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let state = window.use_keyed_state(self.id.clone(), cx, |_, cx| SliderState { focus: cx.focus_handle(), dragging: false });
        let focus = state.read(cx).focus.clone();
        let (min, max, step) = (self.min, self.max, self.step);
        let value = snap(self.value, min, max, step);
        let frac = fraction(value, min, max);
        let format = self.format.unwrap_or_else(|| default_format(step));
        let handler = self.on_change.filter(|_| !self.disabled);

        let cells = ((f32::from(self.width) / PITCH).floor() as usize).max(2);
        let thumb = (frac * (cells - 1) as f32).round() as usize;
        let (lit_ink, thumb_ink, unlit_ink) = if self.disabled {
            (p.line_strong, p.fg_faint, p.line)
        } else {
            (p.accent, p.fg, p.line_strong)
        };

        // Track: the cells, plus a canvas over them that owns the pointer.
        let mut track = div().relative().flex().flex_row().items_center().gap(px(PITCH - 6.)).h(px(20.)).w(px(cells as f32 * PITCH - 2.));
        for i in 0..cells {
            let (h, ink) = if i == thumb {
                (px(20.), thumb_ink)
            } else if i < thumb {
                (px(12.), lit_ink)
            } else {
                (px(12.), unlit_ink)
            };
            track = track.child(div().w(px(6.)).h(h).flex_none().bg(hsla(ink)));
        }
        if let Some(handler) = handler.clone() {
            let state = state.clone();
            let focus = focus.clone();
            track = track.child(
                canvas(
                    |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
                    move |bounds: Bounds<Pixels>, hitbox, window, _| {
                        let at = move |x: Pixels| {
                            let f = f32::from(x - bounds.origin.x) / f32::from(bounds.size.width).max(1.);
                            snap(min + f.clamp(0., 1.) * (max - min), min, max, step)
                        };
                        {
                            let state = state.clone();
                            let handler = handler.clone();
                            let hitbox = hitbox.clone();
                            window.on_mouse_event(move |ev: &MouseDownEvent, phase, window, cx| {
                                if phase.bubble() && ev.button == MouseButton::Left && hitbox.is_hovered(window) {
                                    window.capture_pointer(hitbox.id);
                                    focus.focus(window, cx);
                                    state.update(cx, |s, _| s.dragging = true);
                                    let v = at(ev.position.x);
                                    if v != value {
                                        handler(&v, window, cx);
                                    }
                                }
                            });
                        }
                        window.on_mouse_event(move |ev: &MouseMoveEvent, phase, window, cx| {
                            if !phase.bubble() || !state.read(cx).dragging {
                                return;
                            }
                            if ev.pressed_button != Some(MouseButton::Left) {
                                state.update(cx, |s, _| s.dragging = false);
                                window.release_pointer();
                                return;
                            }
                            let v = at(ev.position.x);
                            if v != value {
                                handler(&v, window, cx);
                            }
                        });
                    },
                )
                .absolute()
                .inset_0(),
            );
        }

        // Fixed-width readout so the layout never shifts as digits change.
        let readout_chars = [min, max, min + step].iter().map(|&v| format(v).chars().count()).max().unwrap_or(3);
        let readout = format(value);

        div()
            .id(self.id.clone())
            .role(Role::Slider)
            .when_some(self.label, |el, l| el.aria_label(l))
            .aria_numeric_value(value as f64)
            .aria_min_numeric_value(min as f64)
            .aria_max_numeric_value(max as f64)
            .aria_numeric_value_step(step as f64)
            .aria_value(readout.clone())
            .track_focus(&focus)
            .when(!self.disabled, |el| el.tab_stop(true))
            .flex()
            .flex_row()
            .flex_none()
            .items_center()
            .gap_3()
            .p_1()
            .border_1()
            .border_color(gpui::transparent_black())
            .when(!self.disabled, |el| el.focus_visible(|s| s.border_color(hsla(p.accent))))
            .when_some(handler, |el, handler| {
                el.on_key_down(move |ev, window, cx| {
                    if let Some(v) = key_value(ev.keystroke.key.as_str(), value, min, max, step) {
                        cx.stop_propagation();
                        if v != value {
                            handler(&v, window, cx);
                        }
                    }
                })
            })
            .child(track)
            .child(
                div()
                    .display(Scale::X1, window)
                    .text_color(hsla(if self.disabled { p.fg_faint } else { p.fg }))
                    .child(format!("{readout:>readout_chars$}")),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_rounds_to_the_grid_and_clamps() {
        assert_eq!(snap(42.4, 0., 100., 5.), 40.);
        assert_eq!(snap(42.6, 0., 100., 5.), 45.);
        assert_eq!(snap(-3., 0., 100., 5.), 0.);
        assert_eq!(snap(130., 0., 100., 5.), 100.);
        // A grid that doesn't divide the range still reaches max.
        assert_eq!(snap(9.9, 0., 10., 3.), 10.);
        assert_eq!(snap(7., 0., 10., 3.), 6.);
    }

    #[test]
    fn keys_step_page_and_jump() {
        assert_eq!(key_value("right", 40., 0., 100., 5.), Some(45.));
        assert_eq!(key_value("left", 0., 0., 100., 5.), Some(0.));
        assert_eq!(key_value("pageup", 40., 0., 100., 5.), Some(90.));
        assert_eq!(key_value("end", 40., 0., 100., 5.), Some(100.));
        assert_eq!(key_value("x", 40., 0., 100., 5.), None);
    }

    #[test]
    fn readout_decimals_follow_the_step() {
        assert_eq!(default_format(5.)(45.), "45");
        assert_eq!(default_format(0.5)(1.5), "1.5");
        assert_eq!(default_format(0.25)(0.75), "0.75");
    }
}
