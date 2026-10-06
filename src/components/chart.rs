//! Charts, in the Ferrite way: stepped lines, square bars, dither for
//! area and density. No curves, no gradients, no anti-aliased strokes.
//!
//! ```text
//!  [ LATENCY ]                              42.0 MS   line_chart: header shows
//!  180 ┤      ┌─┐                                     the hovered (or latest)
//!      │   ┌──┘░└─┐       ┌──                         value; the area under the
//!      │ ──┘░░░░░░└───────┘░░                         line is ░ dither in amber
//!    0 ┤░░░░░░░░░░░░░░░░░░░░░
//!       09:00               12:00
//!
//!  ▁▂▅▃▇█▆▂ sparkline          ███ ▆▆▆ ▃▃▃  bar_chart     ░▒▓█ heatmap
//! ```
//!
//! - [`sparkline`]: inline bars, last one in amber. For stat tiles and
//!   table cells.
//! - [`line_chart`]: one stepped series with a dithered area, an optional
//!   dim comparison series, min/max axis labels, and a hover readout.
//! - [`bar_chart`]: labelled categories; hover shows the value.
//! - [`heatmap`]: a grid of cells whose dither density is the value (a
//!   contribution graph, an hour×weekday load map).
//!
//! Charts follow the motion rules: they *draw on* (wipe left → right) when
//! they first appear, and live data updates instantly — continuous data
//! never animates (DESIGN_LANGUAGE §6.3).

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    App, Bounds, ElementId, Hsla, InteractiveElement, IntoElement, ParentElement, Pixels, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, canvas, div, fill, point, prelude::FluentBuilder as _,
    px, relative, size,
};

use crate::animate::{self, Edge};
use crate::dither::{self, dither};
use crate::fonts::{FerriteText, Scale};
use crate::motion;
use crate::theme::palette;
use crate::tokens::{hsla, text};

/// The range a chart spans: from `min(0, data)` to the data's max, so bars
/// and areas grow from zero. A flat series gets a unit range.
pub fn value_range(values: &[f32]) -> (f32, f32) {
    let lo = values.iter().copied().filter(|v| v.is_finite()).fold(0f32, f32::min);
    let hi = values.iter().copied().filter(|v| v.is_finite()).fold(lo, f32::max);
    if (hi - lo).abs() < f32::EPSILON { (lo, lo + 1.) } else { (lo, hi) }
}

/// `v` as a fraction of `(lo, hi)`, clamped.
pub fn fraction(v: f32, (lo, hi): (f32, f32)) -> f32 {
    if !v.is_finite() { 0. } else { ((v - lo) / (hi - lo)).clamp(0., 1.) }
}

/// Snap a logical coordinate to the device grid.
fn snap(v: Pixels, sf: f32) -> Pixels {
    px((f32::from(v) * sf).round() / sf)
}

// ── Sparkline ─────────────────────────────────────────────────────────────

/// Inline bars, one per value, the last in amber and the rest dim. Size it
/// like any element (default 64×16).
#[derive(IntoElement)]
pub struct Sparkline {
    values: Vec<f32>,
    width: Pixels,
    height: Pixels,
    color: Option<Hsla>,
}

pub fn sparkline(values: impl IntoIterator<Item = f32>) -> Sparkline {
    Sparkline { values: values.into_iter().collect(), width: px(64.), height: px(16.), color: None }
}

impl Sparkline {
    pub fn size(mut self, width: Pixels, height: Pixels) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Ink for every bar but the last (default `fg_dim`).
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

impl RenderOnce for Sparkline {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let (dim, last) = (self.color.unwrap_or_else(|| hsla(p.fg_dim)), hsla(p.accent));
        let values = self.values;
        div().flex_none().w(self.width).h(self.height).child(
            canvas(
                |_, _, _| {},
                move |bounds: Bounds<Pixels>, _, window: &mut Window, _| {
                    let n = values.len();
                    if n == 0 {
                        return;
                    }
                    let sf = window.scale_factor();
                    let range = value_range(&values);
                    let slot = bounds.size.width / n as f32;
                    // A 1px gap between bars once there's room for it.
                    let gap = if f32::from(slot) >= 3. { px(1.) } else { px(0.) };
                    for (i, v) in values.iter().enumerate() {
                        let h = (bounds.size.height * fraction(*v, range)).max(px(1.));
                        let x0 = snap(bounds.left() + slot * i as f32, sf);
                        let x1 = snap(bounds.left() + slot * (i + 1) as f32 - gap, sf);
                        let y0 = snap(bounds.bottom() - h, sf);
                        let bar = Bounds::new(point(x0, y0), size((x1 - x0).max(px(1.)), bounds.bottom() - y0));
                        window.paint_quad(fill(bar, if i + 1 == n { last } else { dim }));
                    }
                },
            )
            .size_full(),
        )
    }
}

// ── Line chart ────────────────────────────────────────────────────────────

type Format = Rc<dyn Fn(f32) -> String>;

/// A stepped line over a dithered area: each value holds for its column,
/// so the line is all horizontal and vertical 2px runs on the pixel grid.
/// Hovering a column shows its value (and label) in the header; otherwise
/// the header shows the latest value.
#[derive(IntoElement)]
pub struct LineChart {
    id: ElementId,
    title: Option<SharedString>,
    values: Vec<f32>,
    compare: Option<Vec<f32>>,
    labels: Vec<SharedString>,
    height: Pixels,
    area: bool,
    format: Format,
}

pub fn line_chart(id: impl Into<ElementId>, values: impl IntoIterator<Item = f32>) -> LineChart {
    LineChart {
        id: id.into(),
        title: None,
        values: values.into_iter().collect(),
        compare: None,
        labels: Vec::new(),
        height: px(160.),
        area: true,
        format: Rc::new(|v| format!("{v:.1}")),
    }
}

impl LineChart {
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// A second series, drawn as a dim line without area (last week, a
    /// baseline, a target).
    pub fn compare(mut self, values: impl IntoIterator<Item = f32>) -> Self {
        self.compare = Some(values.into_iter().collect());
        self
    }

    /// One label per value (times, dates). The first and last are drawn
    /// under the chart; the hovered one goes in the header.
    pub fn labels(mut self, labels: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.labels = labels.into_iter().map(Into::into).collect();
        self
    }

    /// Plot height (default 160px).
    pub fn height(mut self, height: Pixels) -> Self {
        self.height = height;
        self
    }

    /// Turn the dithered area under the line off.
    pub fn no_area(mut self) -> Self {
        self.area = false;
        self
    }

    /// How values print in the header and the axis (default one decimal).
    pub fn format(mut self, format: impl Fn(f32) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }
}

/// Which of `n` equal columns across `bounds` the x coordinate falls in.
pub fn column_at(x: f32, left: f32, width: f32, n: usize) -> Option<usize> {
    if n == 0 || width <= 0. || x < left || x >= left + width {
        return None;
    }
    Some((((x - left) / width) * n as f32).floor() as usize).map(|i| i.min(n - 1))
}

impl RenderOnce for LineChart {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let hover = window.use_keyed_state(ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), "hover".into()), cx, |_, _| None::<usize>);
        let hovered = (*hover.read(cx)).filter(|i| *i < self.values.len());
        let plot_bounds: Rc<Cell<Option<Bounds<Pixels>>>> = Rc::default();
        let draw = animate::play(ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), "draw".into()), 0u8, motion::SLOW, window, cx);

        let mut range = value_range(&self.values);
        if let Some(c) = &self.compare {
            let r2 = value_range(c);
            range = (range.0.min(r2.0), range.1.max(r2.1));
        }
        let shown = hovered.or(self.values.len().checked_sub(1));
        let readout = shown.map(|i| (self.format)(self.values[i])).unwrap_or_default();
        let readout_label = shown.and_then(|i| self.labels.get(i).cloned());

        let (values, compare, area, n) = (self.values.clone(), self.compare.clone(), self.area, self.values.len());
        let (accent, dim, grid, cursor) = (hsla(p.accent), hsla(p.fg_dim), hsla(p.line), hsla(p.fg_faint));
        let record = plot_bounds.clone();
        let plot = canvas(
            move |bounds, _, _| record.set(Some(bounds)),
            move |bounds: Bounds<Pixels>, _, window: &mut Window, _| {
                if n == 0 {
                    return;
                }
                let sf = window.scale_factor();
                let col = bounds.size.width / n as f32;
                let y_of = |v: f32| snap(bounds.bottom() - bounds.size.height * fraction(v, range), sf);
                let x_of = |i: usize| snap(bounds.left() + col * i as f32, sf);
                // Gridlines at the top, middle and baseline.
                for f in [0., 0.5, 1.] {
                    let y = snap(bounds.top() + (bounds.size.height - px(1.)) * f, sf);
                    window.paint_quad(fill(Bounds::new(point(bounds.left(), y), size(bounds.size.width, px(1.))), grid));
                }
                if let Some(i) = hovered {
                    let x = snap(bounds.left() + col * (i as f32 + 0.5), sf);
                    window.paint_quad(fill(Bounds::new(point(x, bounds.top()), size(px(1.), bounds.size.height)), cursor));
                }
                let stepped = |values: &[f32], ink: Hsla, with_area: bool, window: &mut Window| {
                    for (i, v) in values.iter().enumerate().take(n) {
                        let (x0, x1, y) = (x_of(i), x_of(i + 1), y_of(*v));
                        if with_area {
                            let column = Bounds::new(point(x0, y), size(x1 - x0, bounds.bottom() - y));
                            window.with_content_mask(Some(gpui::ContentMask { bounds: column }), |window| {
                                dither::paint_field(dither::flat(dither::level::LIGHT), bounds, ink, window);
                            });
                        }
                        window.paint_quad(fill(Bounds::new(point(x0, y - px(1.)), size(x1 - x0, px(2.))), ink));
                        if let Some(next) = values.get(i + 1) {
                            let y1 = y_of(*next);
                            let (top, bottom) = if y1 < y { (y1, y) } else { (y, y1) };
                            window.paint_quad(fill(Bounds::new(point(x1 - px(1.), top - px(1.)), size(px(2.), bottom - top + px(2.))), ink));
                        }
                    }
                };
                if let Some(c) = &compare {
                    stepped(c, dim, false, window);
                }
                stepped(&values, accent, area, window);
            },
        )
        .size_full();

        let fmt = self.format.clone();
        let axis = div()
            .flex()
            .flex_col()
            .justify_between()
            .items_end()
            .h(self.height)
            .flex_none()
            .body(text::XS)
            .text_color(hsla(p.fg_faint))
            .child(fmt(range.1))
            .child(fmt(range.0));
        let plot_area = div()
            .id(ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), "plot".into()))
            .flex_1()
            .h(self.height)
            .on_mouse_move({
                let hover = hover.clone();
                move |ev, _, cx| {
                    let Some(b) = plot_bounds.get() else { return };
                    let at = column_at(f32::from(ev.position.x), f32::from(b.left()), f32::from(b.size.width), n);
                    if *hover.read(cx) != at {
                        hover.update(cx, |h, cx| {
                            *h = at;
                            cx.notify();
                        });
                    }
                }
            })
            .on_hover({
                let hover = hover.clone();
                move |inside, _, cx| {
                    if !*inside {
                        hover.update(cx, |h, cx| {
                            *h = None;
                            cx.notify();
                        });
                    }
                }
            })
            .child(if draw.done { plot.into_any_element() } else { animate::wipe(plot, draw.eased(), Edge::Left).edge(hsla(p.accent)).into_any_element() });

        let first_label = self.labels.first().cloned();
        let last_label = if self.labels.len() > 1 { self.labels.last().cloned() } else { None };
        div()
            .id(self.id.clone())
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .display(Scale::X1, window)
                    .when_some(self.title, |el, t| el.child(div().text_color(hsla(p.fg_dim)).child(format!("[ {} ]", t.to_uppercase()))))
                    .child(div().flex_1())
                    .when_some(readout_label, |el, l| el.child(div().text_color(hsla(p.fg_faint)).child(l.to_uppercase())))
                    .child(div().text_color(hsla(if hovered.is_some() { p.fg } else { p.accent_text })).child(readout)),
            )
            .child(div().flex().flex_row().gap_2().child(axis).child(plot_area))
            .when(first_label.is_some(), |el| {
                el.child(
                    div()
                        .flex()
                        .flex_row()
                        .justify_between()
                        .body(text::XS)
                        .text_color(hsla(p.fg_faint))
                        // Under the plot, not the axis.
                        .pl(px(40.))
                        .children(first_label)
                        .children(last_label),
                )
            })
    }
}

// ── Bar chart ─────────────────────────────────────────────────────────────

/// Labelled vertical bars in amber, growing from the baseline when the
/// chart first appears. Hovering a bar inverts it and shows its value.
/// `.highlight(i)` keeps one bar called out (the current period).
#[derive(IntoElement)]
pub struct BarChart {
    id: ElementId,
    bars: Vec<(SharedString, f32)>,
    height: Pixels,
    highlight: Option<usize>,
    format: Format,
}

pub fn bar_chart(id: impl Into<ElementId>) -> BarChart {
    BarChart { id: id.into(), bars: Vec::new(), height: px(140.), highlight: None, format: Rc::new(|v| format!("{v:.0}")) }
}

impl BarChart {
    pub fn bar(mut self, label: impl Into<SharedString>, value: f32) -> Self {
        self.bars.push((label.into(), value));
        self
    }

    pub fn bars(mut self, bars: impl IntoIterator<Item = (impl Into<SharedString>, f32)>) -> Self {
        self.bars.extend(bars.into_iter().map(|(l, v)| (l.into(), v)));
        self
    }

    pub fn height(mut self, height: Pixels) -> Self {
        self.height = height;
        self
    }

    pub fn highlight(mut self, index: Option<usize>) -> Self {
        self.highlight = index;
        self
    }

    pub fn format(mut self, format: impl Fn(f32) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }
}

impl RenderOnce for BarChart {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let values: Vec<f32> = self.bars.iter().map(|b| b.1).collect();
        let range = value_range(&values);
        let grow = animate::play(ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), "grow".into()), 0u8, motion::SLOW, window, cx);
        let mut row = div().id(self.id.clone()).flex().flex_row().items_end().gap_2().h(self.height + px(40.));
        for (i, (label, v)) in self.bars.into_iter().enumerate() {
            let group: SharedString = format!("{}-bar-{i}", self.id).into();
            let called_out = self.highlight == Some(i);
            // Bars grow one frame apart, like a cascade.
            let f = fraction(v, range) * animate::snap(((grow.t * 1.6) - i as f32 * 0.04).clamp(0., 1.));
            let f = if grow.done { fraction(v, range) } else { f };
            row = row.child(
                div()
                    .id(("bar", i))
                    .group(group.clone())
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_end()
                    .flex_1()
                    .min_w(px(12.))
                    .h_full()
                    .gap_1()
                    .child(
                        div()
                            .display(Scale::X1, window)
                            .text_color(if called_out { hsla(p.accent_text) } else { gpui::transparent_black() })
                            .group_hover(group.clone(), |s| s.text_color(hsla(p.fg)))
                            .child((self.format)(v)),
                    )
                    .child(
                        div().w_full().h(self.height).flex().flex_col().justify_end().child(
                            div()
                                .w_full()
                                .h(relative(f))
                                .min_h(px(1.))
                                .bg(hsla(if called_out { p.accent_hover } else { p.accent }))
                                .group_hover(group.clone(), |s| s.bg(hsla(p.fg))),
                        ),
                    )
                    .child(
                        div()
                            .w_full()
                            .h(px(16.))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_center()
                            .body(text::XS)
                            .text_color(hsla(if called_out { p.fg } else { p.fg_dim }))
                            .child(label),
                    ),
            );
        }
        div().flex().flex_col().child(row).child(div().h(px(1.)).bg(hsla(p.line_strong)))
    }
}

// ── Heatmap ───────────────────────────────────────────────────────────────

/// The dither level a heatmap cell gets: `0` stays empty and anything
/// nonzero shows at least ░, in five steps — so a single event is visible
/// and equal counts always look equal.
pub fn heat_level(v: f32) -> f32 {
    let v = if v.is_finite() { v.clamp(0., 1.) } else { 0. };
    if v <= 0. { 0. } else { ((v * 4.).ceil() / 4.).max(dither::level::LIGHT) }
}

/// A grid of square cells whose amber dither density is the value (0–1):
/// a contribution graph, a weekday×hour load map. Cells are cached dither
/// tiles, so a year of days is cheap. Optional row labels on the left and a
/// `LESS ░▒▓█ MORE` legend.
#[derive(IntoElement)]
pub struct Heatmap {
    rows: Vec<Vec<f32>>,
    row_labels: Vec<SharedString>,
    cell: Pixels,
    legend: bool,
}

/// `rows` of values in 0–1 (normalise counts yourself, so you choose the
/// scale).
pub fn heatmap(rows: impl IntoIterator<Item = Vec<f32>>) -> Heatmap {
    Heatmap { rows: rows.into_iter().collect(), row_labels: Vec::new(), cell: px(12.), legend: true }
}

impl Heatmap {
    pub fn row_labels(mut self, labels: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.row_labels = labels.into_iter().map(Into::into).collect();
        self
    }

    /// Cell edge (default 12px).
    pub fn cell(mut self, cell: Pixels) -> Self {
        self.cell = cell;
        self
    }

    pub fn no_legend(mut self) -> Self {
        self.legend = false;
        self
    }
}

impl RenderOnce for Heatmap {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let cell = self.cell;
        let tile = move |level: f32| {
            div()
                .size(cell)
                .flex_none()
                .bg(hsla(p.sunken))
                .border_1()
                .border_color(hsla(if level > 0. { p.accent_dim } else { p.line }))
                .when(level > 0., |el| el.child(dither(dither::flat(level)).ink(hsla(p.accent)).size_full()))
        };
        let has_labels = !self.row_labels.is_empty();
        let mut grid = div().flex().flex_col().gap(px(2.));
        for (r, row) in self.rows.into_iter().enumerate() {
            let label = self.row_labels.get(r).cloned();
            grid = grid.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(2.))
                    .when(has_labels, |el| {
                        el.child(div().w(px(32.)).flex_none().body(text::XS).text_color(hsla(p.fg_faint)).children(label))
                    })
                    .children(row.into_iter().map(|v| tile(heat_level(v)))),
            );
        }
        div().flex().flex_col().gap_2().child(grid).when(self.legend, |el| {
            el.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(2.))
                    .when(has_labels, |el| el.pl(px(34.)))
                    .body(text::XS)
                    .text_color(hsla(p.fg_faint))
                    .child(div().pr_1().child("less"))
                    .children([0., 0.25, 0.5, 0.75, 1.].map(tile))
                    .child(div().pl_1().child("more")),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_start_at_zero_and_never_collapse() {
        assert_eq!(value_range(&[3., 7., 5.]), (0., 7.));
        assert_eq!(value_range(&[-2., 4.]), (-2., 4.));
        assert_eq!(value_range(&[0., 0.]), (0., 1.));
        assert_eq!(value_range(&[]), (0., 1.));
        assert_eq!(value_range(&[f32::NAN, 2.]), (0., 2.));
        assert_eq!(fraction(7., (0., 7.)), 1.);
        assert_eq!(fraction(f32::NAN, (0., 7.)), 0.);
    }

    #[test]
    fn columns_cover_the_width_exactly() {
        assert_eq!(column_at(0., 0., 100., 4), Some(0));
        assert_eq!(column_at(99.9, 0., 100., 4), Some(3));
        assert_eq!(column_at(100., 0., 100., 4), None);
        assert_eq!(column_at(-1., 0., 100., 4), None);
        assert_eq!(column_at(50., 0., 100., 0), None);
    }

    #[test]
    fn heat_levels_are_five_steps_and_never_hide_an_event() {
        assert_eq!(heat_level(0.), 0.);
        assert_eq!(heat_level(0.01), 0.25);
        assert_eq!(heat_level(0.5), 0.5);
        assert_eq!(heat_level(0.51), 0.75);
        assert_eq!(heat_level(2.), 1.);
        for i in 0..=100 {
            let l = heat_level(i as f32 / 100.);
            assert_eq!((l * 4.).fract(), 0.);
        }
    }
}
