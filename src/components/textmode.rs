//! Text-mode elements: the ASCII side of the language, drawn with real
//! glyphs on the display face's cell grid.
//!
//! ```text
//!  ╔═[ SYSTEM ]═══════════════╗    ascii_box: single or double frame, any
//!  ║  cpu  [██████▒·····]  52% ║    size; the title is set into the edge
//!  ║  mem  [███▒········]  27% ║    ascii_gauge
//!  ╚══════════════════════════╝
//!  ══ LOGS ═════════════════════    ascii_rule
//!
//!  █▀▀ █▀▀ █▀▄ █▀▄ ▀█▀ ▀█▀ █▀▀       banner: 5×5 block letters in half blocks
//!
//!       .:-=+**#%%@@%%#*+=-:.        ascii_art: any Picture as characters
//! ```
//!
//! Frames are painted as shaped glyphs, not CSS borders, so they are the
//! pixel font's own strokes: they join at the corners exactly as DOS boxes
//! do. A box can be any size: edges are clipped to it, so a fractional last
//! cell never shows a seam. Use them where the language wants a terminal
//! feel — a status block, a boot screen, an about box, a retro mode — and
//! keep `panel` as the everyday frame: one framing style per screen.

use gpui::{
    AnyElement, App, Bounds, ContentMask, ElementId, Hsla, IntoElement, ParentElement, Pixels, RenderOnce,
    SharedString, StyleRefinement, Styled, TextAlign, TextRun, Window, canvas, div, font, point,
    prelude::FluentBuilder as _, px, size,
};

use crate::ascii::{self, BoxStyle};
use crate::dither::{self, Picture, dither};
use crate::fonts::{DISPLAY, FerriteText, Scale, display_size};
use crate::theme::palette;
use crate::tokens::hsla;

/// Paint `text` in the display face at `origin`, clipped to `clip`.
fn paint_glyphs(text: &str, origin: gpui::Point<Pixels>, clip: Bounds<Pixels>, size: Pixels, color: Hsla, window: &mut Window, cx: &mut App) {
    if text.is_empty() || clip.size.width <= px(0.) || clip.size.height <= px(0.) {
        return;
    }
    let run = TextRun { len: text.len(), font: font(DISPLAY), color, background_color: None, underline: None, strikethrough: None };
    let line = window.text_system().shape_line(SharedString::from(text.to_string()), size, &[run], None);
    window.with_content_mask(Some(ContentMask { bounds: clip }), |window| {
        let _ = line.paint(origin, size, TextAlign::Left, None, window, cx);
    });
}

/// Snap a logical coordinate to the device grid.
fn snap(v: Pixels, sf: f32) -> Pixels {
    px((f32::from(v) * sf).round() / sf)
}

// ── Box ───────────────────────────────────────────────────────────────────

/// A text-mode frame around any content: `┌─[ TITLE ]──┐` (single) or
/// `╔═[ TITLE ]══╗` (double, for emphasis). Style it like a `div`; the
/// content gets one cell of padding inside the frame.
#[derive(IntoElement)]
pub struct AsciiBox {
    title: Option<SharedString>,
    style: BoxStyle,
    ink: Option<Hsla>,
    children: Vec<AnyElement>,
    refinement: StyleRefinement,
}

pub fn ascii_box() -> AsciiBox {
    AsciiBox { title: None, style: ascii::SINGLE, ink: None, children: Vec::new(), refinement: StyleRefinement::default() }
}

impl AsciiBox {
    /// A `[ TITLE ]` set into the top edge.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// `╔═╗` instead of `┌─┐`.
    pub fn double(mut self) -> Self {
        self.style = ascii::DOUBLE;
        self
    }

    /// Frame color (default `line_strong`; pass the accent for the active box).
    pub fn ink(mut self, color: Hsla) -> Self {
        self.ink = Some(color);
        self
    }
}

impl ParentElement for AsciiBox {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for AsciiBox {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.refinement
    }
}

impl RenderOnce for AsciiBox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let ink = self.ink.unwrap_or_else(|| hsla(p.line_strong));
        let title_ink = hsla(p.fg);
        let cell_h = display_size(Scale::X1, window);
        let cell_w = cell_h / 2.;
        let (style, title) = (self.style, self.title);
        let frame = canvas(
            |_, _, _| {},
            move |bounds: Bounds<Pixels>, _, window: &mut Window, cx: &mut App| {
                let sf = window.scale_factor();
                let (l, t) = (snap(bounds.left(), sf), snap(bounds.top(), sf));
                let (r, b) = (snap(bounds.right(), sf), snap(bounds.bottom(), sf));
                let cols = (f32::from(r - l) / f32::from(cell_w)).ceil() as usize;
                let rows = (f32::from(b - t) / f32::from(cell_h)).ceil() as usize;
                if cols < 2 || rows < 2 {
                    return;
                }
                let full = Bounds::new(point(l, t), size(r - l, b - t));
                // Edges: everything but the right-hand corner, clipped short
                // of it, then the corner at the exact right edge.
                for (top, y) in [(true, t), (false, b - cell_h)] {
                    let edge = ascii::box_edge(style, cols, top, if top { title.as_deref() } else { None });
                    let clip = Bounds::new(point(l, y), size(r - cell_w - l, cell_h));
                    paint_glyphs(&edge, point(l, y), clip, cell_h, ink, window, cx);
                    let corner = if top { style.tr } else { style.br };
                    paint_glyphs(&corner.to_string(), point(r - cell_w, y), full, cell_h, ink, window, cx);
                }
                // The title reads in fg, over the frame-colored copy.
                if let Some(title) = &title {
                    let label = ascii::bracket(title);
                    if cols >= label.chars().count() + 4 {
                        let x = l + cell_w * 2.;
                        paint_glyphs(&label, point(x, t), Bounds::new(point(x, t), size(r - cell_w - x, cell_h)), cell_h, title_ink, window, cx);
                    }
                }
                // Sides, row by row between the edges.
                let side = style.v.to_string();
                let inner = Bounds::new(point(l, t + cell_h), size(r - l, b - t - cell_h * 2.));
                for row in 1..rows {
                    let y = t + cell_h * row as f32;
                    paint_glyphs(&side, point(l, y), inner, cell_h, ink, window, cx);
                    paint_glyphs(&side, point(r - cell_w, y), inner, cell_h, ink, window, cx);
                }
            },
        )
        .absolute()
        .inset_0();
        let mut root = div();
        *root.style() = self.refinement;
        root.relative()
            .flex()
            .flex_col()
            .min_w(cell_w * 6.)
            .min_h(cell_h * 3.)
            .pt(cell_h)
            .pb(cell_h)
            .px(cell_w * 2.)
            .child(div().flex().flex_col().gap_1().children(self.children))
            .child(frame)
    }
}

// ── Rule ──────────────────────────────────────────────────────────────────

/// A text-mode divider across the full width: `══ LOGS ═══════` (double) or
/// `── LOGS ───────` (single).
#[derive(IntoElement)]
pub struct AsciiRule {
    label: Option<SharedString>,
    style: BoxStyle,
}

pub fn ascii_rule(label: Option<&str>) -> AsciiRule {
    AsciiRule { label: label.map(|l| SharedString::from(l.to_uppercase())), style: ascii::SINGLE }
}

impl AsciiRule {
    pub fn double(mut self) -> Self {
        self.style = ascii::DOUBLE;
        self
    }
}

impl RenderOnce for AsciiRule {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let (ink, text_ink) = (hsla(p.line_strong), hsla(p.fg_dim));
        let cell_h = display_size(Scale::X1, window);
        let cell_w = cell_h / 2.;
        let (h, label) = (self.style.h, self.label);
        div().w_full().h(cell_h).child(
            canvas(
                |_, _, _| {},
                move |bounds: Bounds<Pixels>, _, window: &mut Window, cx: &mut App| {
                    let sf = window.scale_factor();
                    let (l, t) = (snap(bounds.left(), sf), snap(bounds.top(), sf));
                    let cols = (f32::from(bounds.size.width) / f32::from(cell_w)).ceil() as usize;
                    let line: String = std::iter::repeat_n(h, cols).collect();
                    paint_glyphs(&line, point(l, t), bounds, cell_h, ink, window, cx);
                    if let Some(label) = &label {
                        // Over the rule: ` LABEL ` after two rule cells, on a
                        // cleared background so the rule stops at the text.
                        let text = format!(" {label} ");
                        let x = l + cell_w * 2.;
                        let w = cell_w * text.chars().count() as f32;
                        window.paint_quad(gpui::fill(Bounds::new(point(x, t), size(w, cell_h)), hsla(palette(cx).bg)));
                        paint_glyphs(&text, point(x, t), bounds, cell_h, text_ink, window, cx);
                    }
                },
            )
            .size_full(),
        )
    }
}

// ── Banner ────────────────────────────────────────────────────────────────

/// Big block letters for a title screen, an about box, an empty state:
/// the built-in 5×5 font (`ascii::block_glyph`), each font pixel half a
/// display cell, so a line is 2½ display rows tall at `Scale::X1` (40px). Draws on left→right when it first
/// appears and whenever the text changes.
#[derive(IntoElement)]
pub struct Banner {
    id: ElementId,
    text: SharedString,
    scale: Scale,
    color: Option<Hsla>,
}

pub fn banner(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Banner {
    Banner { id: id.into(), text: text.into(), scale: Scale::X1, color: None }
}

impl Banner {
    /// Pixel scale: X1 is 40px tall, X2 80px.
    pub fn scale(mut self, scale: Scale) -> Self {
        self.scale = scale;
        self
    }

    /// Ink (default: the accent as text).
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

/// The lit pixels of `text` in the 5×5 block font, as (column, row) on a
/// grid `width` columns wide (letters one column apart).
pub fn banner_pixels(text: &str) -> (usize, Vec<(usize, usize)>) {
    let mut x0 = 0usize;
    let mut lit = Vec::new();
    for (i, (w, rows)) in text.chars().filter_map(ascii::block_glyph).enumerate() {
        if i > 0 {
            x0 += 1;
        }
        for (y, row) in rows.iter().enumerate() {
            for x in 0..w as usize {
                if row & (1 << (w as usize - 1 - x)) != 0 {
                    lit.push((x0 + x, y));
                }
            }
        }
        x0 += w as usize;
    }
    (x0, lit)
}

impl RenderOnce for Banner {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        // One font pixel = half a display cell: square, on the type's grid.
        // Painted as quads, not ▀▄█ glyphs: text anti-aliasing (subpixel on
        // Linux) would fringe and seam dense glyphs (PITFALLS 47).
        let pixel = display_size(self.scale, window) / 2.;
        let (cols, lit) = banner_pixels(&self.text);
        let ink = self.color.unwrap_or_else(|| hsla(p.accent_text));
        let t = crate::animate::play(self.id.clone(), &self.text, crate::motion::SLOW, window, cx);
        let body = div().flex_none().w(pixel * cols as f32).h(pixel * 5.).child(
            canvas(
                |_, _, _| {},
                move |bounds: Bounds<Pixels>, _, window: &mut Window, _| {
                    let sf = window.scale_factor();
                    for &(x, y) in &lit {
                        let x0 = snap(bounds.left() + pixel * x as f32, sf);
                        let y0 = snap(bounds.top() + pixel * y as f32, sf);
                        let x1 = snap(bounds.left() + pixel * (x + 1) as f32, sf);
                        let y1 = snap(bounds.top() + pixel * (y + 1) as f32, sf);
                        window.paint_quad(gpui::fill(Bounds::new(point(x0, y0), size(x1 - x0, y1 - y0)), ink));
                    }
                },
            )
            .size_full(),
        );
        if t.done {
            body.into_any_element()
        } else {
            crate::animate::wipe(body, t.eased(), crate::animate::Edge::Left).edge(hsla(p.accent)).into_any_element()
        }
    }
}

// ── ASCII art ─────────────────────────────────────────────────────────────

/// A grayscale [`Picture`] drawn as characters, `cols` wide, rows following
/// the picture's aspect and the cell's 1:2 shape. Printable ASCII only:
/// for block-shade pictures use `dither(picture)`, which is crisper. Text, so it's selectable
/// in spirit and cheap: one shaped line per row. Pair with [`super::fx::develop`]
/// or `decrypt`-style reveals for an arrival.
#[derive(IntoElement)]
pub struct AsciiArt {
    picture: Picture,
    cols: usize,
    ramp: &'static [char],
    color: Option<Hsla>,
}

pub fn ascii_art(picture: Picture) -> AsciiArt {
    AsciiArt { picture, cols: 48, ramp: &ascii::CLASSIC, color: None }
}

impl AsciiArt {
    pub fn cols(mut self, cols: usize) -> Self {
        self.cols = cols.max(1);
        self
    }

    /// Characters from light to dark ink (default `ascii::CLASSIC`,
    /// ` .:-=+*#%@`; `ascii::BUBBLES` is ` .oO@`). Short ramps read as
    /// shapes; long ones as tone, but turn faint backgrounds into noise.
    pub fn ramp(mut self, ramp: &'static [char]) -> Self {
        if !ramp.is_empty() {
            self.ramp = ramp;
        }
        self
    }

    /// Ink (default `fg`; the accent for a hero image).
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

impl RenderOnce for AsciiArt {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let (w, h) = self.picture.size();
        let ramp = self.ramp;
        let picture = self.picture;
        let lines = ascii::art(|u, v| picture.sample(u, v), self.cols, h as f32 / w.max(1) as f32, ramp);
        div()
            .flex()
            .flex_col()
            .flex_none()
            .text_color(self.color.unwrap_or_else(|| hsla(p.fg)))
            .children(lines.into_iter().map(|l| div().display(Scale::X1, window).whitespace_nowrap().child(l)))
    }
}

// ── Gauge ─────────────────────────────────────────────────────────────────

/// `CPU [██████▒·····]  52%` — a text-mode gauge: brackets and track in
/// the faint ink, the fill in the accent, a fixed-width readout. Live data
/// friendly: it never animates.
#[derive(IntoElement)]
pub struct AsciiGauge {
    value: f32,
    cells: usize,
    label: Option<SharedString>,
}

pub fn ascii_gauge(value: f32) -> AsciiGauge {
    AsciiGauge { value, cells: 12, label: None }
}

impl AsciiGauge {
    /// Track width in cells (default 12).
    pub fn cells(mut self, cells: usize) -> Self {
        self.cells = cells.max(1);
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl RenderOnce for AsciiGauge {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let cell = display_size(Scale::X1, window) / 2.;
        let value = self.value.clamp(0., 1.);
        let filled = value * self.cells as f32;
        let (full, frac) = (filled.floor() as usize, filled.fract());
        // The fill is quads and a dithered partial cell rather than █▒
        // glyphs, which fringe under subpixel text AA (PITFALLS 47); the
        // track and brackets stay text.
        let track: String = std::iter::repeat_n('·', self.cells).collect();
        let bar = div()
            .relative()
            .flex_none()
            .w(cell * self.cells as f32)
            .text_color(hsla(p.fg_faint))
            .child(track)
            .child(div().absolute().top_0().bottom_0().left_0().w(cell * full as f32).bg(hsla(p.accent)))
            .when(full < self.cells && frac > 0., |el| {
                let level = ((frac.max(0.25) * 4.).round() / 4.).min(dither::level::DARK);
                el.child(
                    div()
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left(cell * full as f32)
                        .w(cell)
                        .bg(hsla(p.bg))
                        .child(dither(dither::flat(level)).ink(hsla(p.accent)).size_full()),
                )
            });
        div()
            .flex()
            .flex_row()
            .flex_none()
            .display(Scale::X1, window)
            .whitespace_nowrap()
            .when_some(self.label, |el, l| el.child(div().w(cell * 6.).text_color(hsla(p.fg_dim)).child(l.to_uppercase())))
            .child(div().text_color(hsla(p.fg_faint)).child("["))
            .child(bar)
            .child(div().text_color(hsla(p.fg_faint)).child("]"))
            .child(div().pl(cell).text_color(hsla(p.fg)).child(format!("{:>3}%", (value * 100.).round() as u32)))
    }
}

// ── Mark ──────────────────────────────────────────────────────────────────

/// The Ferrite mark: `▓▒░` in the accent, three display cells of real
/// dither (75%, 50%, 25%) instead of shade glyphs, so it is crisp under any
/// text anti-aliasing. `.shown(n)` reveals only the first `n` cells (the
/// title bar types it on at boot).
#[derive(IntoElement)]
pub struct Mark {
    shown: usize,
}

pub fn mark() -> Mark {
    Mark { shown: 3 }
}

impl Mark {
    pub fn shown(mut self, cells: usize) -> Self {
        self.shown = cells.min(3);
        self
    }
}

impl RenderOnce for Mark {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let accent = hsla(palette(cx).accent);
        let h = display_size(Scale::X1, window);
        let levels = [dither::level::DARK, dither::level::MEDIUM, dither::level::LIGHT];
        div().flex().flex_row().flex_none().h(h).children(levels.into_iter().enumerate().map(move |(i, l)| {
            div().w(h / 2.).h_full().when(i < self.shown, |el| el.child(dither(dither::flat(l)).ink(accent).size_full()))
        }))
    }
}
