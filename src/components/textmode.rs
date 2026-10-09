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
//! More of the same family: [`ascii_table`], [`ascii_tree`], [`ascii_plot`],
//! [`ascii_bars`], [`ascii_cal`], [`marquee`], and two controls,
//! [`ascii_button`] (`< OK >`) and [`ascii_list`] (`► item`). Boxes come in
//! five styles (`ascii::STYLES`), cast a dithered DOS shadow, and can draw
//! themselves on, tracing the frame clockwise.
//!
//! Frames are painted as shaped glyphs, not CSS borders, so they are the
//! pixel font's own strokes: they join at the corners exactly as DOS boxes
//! do. A box can be any size: edges are clipped to it, so a fractional last
//! cell never shows a seam. Use them where the language wants a terminal
//! feel — a status block, a boot screen, an about box, a retro mode — and
//! keep `panel` as the everyday frame: one framing style per screen.

use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, ClickEvent, ContentMask, ElementId, HighlightStyle, Hsla, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Pixels, RenderOnce, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, TextAlign,
    StyledText, TextRun, Window, canvas, div, font, point, prelude::FluentBuilder as _, px, size,
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
    shadow: bool,
    draw_on: Option<(ElementId, u64)>,
    children: Vec<AnyElement>,
    refinement: StyleRefinement,
}

pub fn ascii_box() -> AsciiBox {
    AsciiBox {
        title: None,
        style: ascii::SINGLE,
        ink: None,
        shadow: false,
        draw_on: None,
        children: Vec::new(),
        refinement: StyleRefinement::default(),
    }
}

/// How much of each side of a `w`×`h` frame a draw-on has traced when
/// `d` of its perimeter is drawn, clockwise from the top-left corner:
/// `[top →, right ↓, bottom ←, left ↑]`, each in pixels.
pub fn perimeter(w: f32, h: f32, d: f32) -> [f32; 4] {
    let side = |start: f32, len: f32| (d - start).clamp(0., len);
    [side(0., w), side(w, h), side(w + h, w), side(2. * w + h, h)]
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

    /// Any box style: `ascii::SINGLE`, `DOUBLE`, `DOUBLE_H` (`╒═╕`),
    /// `DOUBLE_V` (`╓─╖`) or `PLAIN` (`+-+`).
    pub fn style(mut self, style: BoxStyle) -> Self {
        self.style = style;
        self
    }

    /// A DOS drop shadow: one dithered cell to the right and below, outside
    /// the box. For dialogs and anything floating.
    pub fn shadow(mut self) -> Self {
        self.shadow = true;
        self
    }

    /// Trace the frame on, clockwise from the top-left corner behind an
    /// amber cursor, when the box first appears and whenever `key` changes;
    /// the content unrolls with it.
    pub fn draw_on(mut self, id: impl Into<ElementId>, key: impl std::hash::Hash) -> Self {
        use std::hash::{DefaultHasher, Hasher};
        let mut h = DefaultHasher::new();
        key.hash(&mut h);
        self.draw_on = Some((id.into(), h.finish()));
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
        let traced = self
            .draw_on
            .map(|(id, key)| crate::animate::play(id, key, crate::motion::SLOW, window, cx))
            .filter(|p| !p.done)
            .map(|p| p.eased());
        let cursor = hsla(p.accent);
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
                let Some(traced) = traced else {
                    paint_frame(style, title.as_deref(), (l, t, r, b), (cols, rows), (cell_w, cell_h), (ink, title_ink), window, cx);
                    return;
                };
                // Drawing on: the whole frame, seen through four strips that
                // grow clockwise round the perimeter, and a cursor cell at
                // the head.
                let (w, h) = (f32::from(r - l), f32::from(b - t));
                let [top, right, bottom, left] = perimeter(w, h, traced * 2. * (w + h));
                let strips = [
                    Bounds::new(point(l, t), size(px(top), cell_h)),
                    Bounds::new(point(r - cell_w, t), size(cell_w, px(right))),
                    Bounds::new(point(r - px(bottom), b - cell_h), size(px(bottom), cell_h)),
                    Bounds::new(point(l, b - px(left)), size(cell_w, px(left))),
                ];
                for strip in strips {
                    window.with_content_mask(Some(ContentMask { bounds: strip }), |window| {
                        paint_frame(style, title.as_deref(), (l, t, r, b), (cols, rows), (cell_w, cell_h), (ink, title_ink), window, cx);
                    });
                }
                let head = if left > 0. {
                    point(l, b - px(left))
                } else if bottom > 0. {
                    point(r - px(bottom), b - cell_h)
                } else if right > 0. {
                    point(r - cell_w, t + px(right))
                } else {
                    point(l + px(top), t)
                };
                let head = point(head.x.clamp(l, r - cell_w), head.y.clamp(t, b - cell_h));
                window.paint_quad(gpui::fill(Bounds::new(head, size(cell_w, cell_h)), cursor));
                let _ = full;
            },
        )
        .absolute()
        .inset_0();
        // The shadow lands once the frame has finished drawing.
        let shadow = (self.shadow && traced.is_none()).then(|| {
            let ink = hsla(p.line_strong);
            let strip = || dither(dither::flat(dither::level::MEDIUM)).ink(ink).size_full();
            [
                div().absolute().top(cell_h).right(-cell_w).w(cell_w).bottom(-cell_h).child(strip()),
                div().absolute().left(cell_w).right_0().bottom(-cell_h).h(cell_h).child(strip()),
            ]
        });
        let content = div().flex().flex_col().gap_1().children(self.children);
        let content = match traced {
            Some(t) => crate::animate::unroll(content, t).into_any_element(),
            None => content.into_any_element(),
        };
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
            .child(content)
            .child(frame)
            .children(shadow.into_iter().flatten())
    }
}

/// Paint a box frame over `(l, t, r, b)`: edges, the title, the sides.
#[allow(clippy::too_many_arguments)]
fn paint_frame(
    style: BoxStyle,
    title: Option<&str>,
    (l, t, r, b): (Pixels, Pixels, Pixels, Pixels),
    (cols, rows): (usize, usize),
    (cell_w, cell_h): (Pixels, Pixels),
    (ink, title_ink): (Hsla, Hsla),
    window: &mut Window,
    cx: &mut App,
) {
                let full = Bounds::new(point(l, t), size(r - l, b - t));
                // Edges: everything but the right-hand corner, clipped short
                // of it, then the corner at the exact right edge.
                for (top, y) in [(true, t), (false, b - cell_h)] {
                    let edge = ascii::box_edge(style, cols, top, if top { title } else { None });
                    let clip = Bounds::new(point(l, y), size(r - cell_w - l, cell_h));
                    paint_glyphs(&edge, point(l, y), clip, cell_h, ink, window, cx);
                    let corner = if top { style.tr } else { style.br };
                    paint_glyphs(&corner.to_string(), point(r - cell_w, y), full, cell_h, ink, window, cx);
                }
                // The title reads in fg, over the frame-colored copy.
                if let Some(title) = title {
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
    shadow: bool,
}

pub fn banner(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Banner {
    Banner { id: id.into(), text: text.into(), scale: Scale::X1, color: None, shadow: false }
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

    /// A hard dithered drop shadow one font pixel down and right.
    pub fn shadow(mut self) -> Self {
        self.shadow = true;
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
        let shadow = self.shadow.then(|| hsla(p.line_strong));
        let extra = if self.shadow { 1. } else { 0. };
        let body = div().flex_none().w(pixel * (cols as f32 + extra)).h(pixel * (5. + extra)).child(
            canvas(
                |_, _, _| {},
                move |bounds: Bounds<Pixels>, _, window: &mut Window, _| {
                    let sf = window.scale_factor();
                    if let Some(ink) = shadow {
                        for &(x, y) in &lit {
                            let o = point(snap(bounds.left() + pixel * (x + 1) as f32, sf), snap(bounds.top() + pixel * (y + 1) as f32, sf));
                            let cell = Bounds::new(o, size(pixel, pixel));
                            dither::paint_field(dither::flat(dither::level::MEDIUM), cell, ink, window);
                        }
                    }
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
    charset: Option<ascii::Charset>,
    fit: Option<ascii::Fit>,
    contrast: f32,
    invert: bool,
    diffuse: bool,
}

pub fn ascii_art(picture: Picture) -> AsciiArt {
    AsciiArt { picture, cols: 48, ramp: &ascii::CLASSIC, color: None, charset: None, fit: None, contrast: 1., invert: false, diffuse: false }
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

    /// Draw with a character set the engine knows glyph by glyph
    /// (`Charset::Punctuation`, `Slashes`, `Accents`, `Box`, `Full` for the
    /// best character…), fitted by tone or by shape. Overrides `.ramp`.
    pub fn charset(mut self, charset: ascii::Charset) -> Self {
        self.charset = Some(charset);
        self
    }

    /// Tone or shape (default: what suits the charset).
    pub fn fit(mut self, fit: ascii::Fit) -> Self {
        self.fit = Some(fit);
        self
    }

    /// Gain around mid-grey before fitting (1 = as is).
    pub fn contrast(mut self, contrast: f32) -> Self {
        self.contrast = contrast.max(0.);
        self
    }

    /// Swap ink and paper.
    pub fn invert(mut self, invert: bool) -> Self {
        self.invert = invert;
        self
    }

    /// Tone fit: error-diffuse between cells for smooth gradients.
    pub fn diffuse(mut self, diffuse: bool) -> Self {
        self.diffuse = diffuse;
        self
    }
}

impl RenderOnce for AsciiArt {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let (w, h) = self.picture.size();
        let fitted = self.charset.is_some() || self.fit.is_some() || self.invert || self.diffuse || self.contrast != 1.;
        let lines: Vec<String> = if fitted {
            let charset = self.charset.unwrap_or(ascii::Charset::Classic);
            let style = ascii::ArtStyle {
                charset,
                fit: self.fit.unwrap_or(charset.default_fit()),
                contrast: self.contrast,
                invert: self.invert,
                diffuse: self.diffuse,
            };
            ascii::picture_art(&self.picture, self.cols, style).as_ref().clone()
        } else {
            let picture = self.picture;
            ascii::art(|u, v| picture.sample(u, v), self.cols, h as f32 / w.max(1) as f32, self.ramp)
        };
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

// ── Shared bits for the grid components ──────────────────────────────────

/// One display-face line with no wrapping.
fn line(text: impl Into<SharedString>, window: &Window) -> gpui::Div {
    div().display(Scale::X1, window).whitespace_nowrap().child(text.into())
}

/// A ruled line (`│ a │ b │`) with the rule glyph in `frame` ink and the
/// cells in `text` ink.
fn ruled(text: &str, v: char, frame: Hsla, ink: Hsla, window: &Window) -> gpui::Div {
    let mut row = div().flex().flex_row().flex_none().display(Scale::X1, window).whitespace_nowrap();
    for (i, cell) in text.split(v).enumerate() {
        if i > 0 {
            row = row.child(div().text_color(frame).child(v.to_string()));
        }
        if !cell.is_empty() {
            row = row.child(div().text_color(ink).child(cell.to_string()));
        }
    }
    row
}

// ── Table ─────────────────────────────────────────────────────────────────

/// A text-mode table: a boxed grid with junctions, the header in `fg`,
/// rules in `line_strong`, numbers right-aligned. For readouts, reports and
/// about boxes; `table` stays the interactive, sortable one.
///
/// ```text
/// ┌───────┬──────┐
/// │ NAME  │  PID │
/// ├───────┼──────┤
/// │ cargo │ 9021 │
/// └───────┴──────┘
/// ```
#[derive(IntoElement)]
pub struct AsciiTable {
    headers: Vec<SharedString>,
    rows: Vec<Vec<String>>,
    style: BoxStyle,
    selected: Option<usize>,
}

pub fn ascii_table() -> AsciiTable {
    AsciiTable { headers: Vec::new(), rows: Vec::new(), style: ascii::SINGLE, selected: None }
}

impl AsciiTable {
    pub fn header<S: Into<SharedString>>(mut self, headers: impl IntoIterator<Item = S>) -> Self {
        self.headers = headers.into_iter().map(Into::into).collect();
        self
    }

    pub fn row<S: ToString>(mut self, cells: impl IntoIterator<Item = S>) -> Self {
        self.rows.push(cells.into_iter().map(|c| c.to_string()).collect());
        self
    }

    pub fn style(mut self, style: BoxStyle) -> Self {
        self.style = style;
        self
    }

    /// Highlight one data row in inverse amber.
    pub fn selected(mut self, row: Option<usize>) -> Self {
        self.selected = row;
        self
    }
}

impl RenderOnce for AsciiTable {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let headers: Vec<&str> = self.headers.iter().map(|h| h.as_ref()).collect();
        let lines = ascii::table(&headers, &self.rows, self.style);
        let (frame, last) = (hsla(p.line_strong), lines.len() - 1);
        let v = self.style.v;
        div().flex().flex_col().flex_none().items_start().children(lines.into_iter().enumerate().map(|(i, l)| match i {
            0 | 2 => line(l, window).text_color(frame),
            i if i == last => line(l, window).text_color(frame),
            1 => ruled(&l, v, frame, hsla(p.fg), window),
            i if self.selected == Some(i - 3) => ruled(&l, v, frame, hsla(p.accent_fg), window).bg(hsla(p.accent)),
            _ => ruled(&l, v, frame, hsla(p.fg_dim), window),
        }))
    }
}

// ── Tree ──────────────────────────────────────────────────────────────────

/// An outline drawn like the `tree` command: `├──`, `└──` and `│` guides in
/// the faint ink, labels in `fg`. Items are `(depth, label)` in order;
/// folders end in `/` by convention and read in the accent.
#[derive(IntoElement)]
pub struct AsciiTree {
    items: Vec<(usize, SharedString)>,
}

pub fn ascii_tree() -> AsciiTree {
    AsciiTree { items: Vec::new() }
}

impl AsciiTree {
    pub fn item(mut self, depth: usize, label: impl Into<SharedString>) -> Self {
        self.items.push((depth, label.into()));
        self
    }
}

impl RenderOnce for AsciiTree {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let items: Vec<(usize, &str)> = self.items.iter().map(|(d, l)| (*d, l.as_ref())).collect();
        div().flex().flex_col().flex_none().children(ascii::tree(&items).into_iter().map(|(guide, label)| {
            let ink = if label.ends_with('/') { p.accent_text } else { p.fg };
            div()
                .flex()
                .flex_row()
                .display(Scale::X1, window)
                .whitespace_nowrap()
                .child(div().text_color(hsla(p.fg_faint)).child(guide))
                .child(div().text_color(hsla(ink)).child(label.to_string()))
        }))
    }
}

// ── Plot ──────────────────────────────────────────────────────────────────

/// A text plot: `*` points joined by `:`, a left axis labelled with the
/// max and min, a baseline. `cols` × `rows` cells of plot area.
#[derive(IntoElement)]
pub struct AsciiPlot {
    values: Vec<f32>,
    cols: usize,
    rows: usize,
    format: fn(f32) -> String,
}

pub fn ascii_plot(values: impl Into<Vec<f32>>) -> AsciiPlot {
    AsciiPlot { values: values.into(), cols: 48, rows: 8, format: |v| format!("{v:.0}") }
}

impl AsciiPlot {
    /// Plot area in cells (default 48 × 8).
    pub fn size(mut self, cols: usize, rows: usize) -> Self {
        self.cols = cols.max(2);
        self.rows = rows.max(2);
        self
    }

    /// How the axis prints a value (default: whole numbers).
    pub fn format(mut self, format: fn(f32) -> String) -> Self {
        self.format = format;
        self
    }
}

impl RenderOnce for AsciiPlot {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let (lo, hi) = self.values.iter().fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
        let (lo, hi) = if self.values.is_empty() { (0., 0.) } else { (lo, hi) };
        let labels = [(self.format)(hi), (self.format)(lo)];
        let lw = labels.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        let body = ascii::plot(&self.values, self.cols, self.rows);
        let last = body.len() - 1;
        let axis = |text: String, tick: char| {
            div().flex().flex_row().child(div().text_color(hsla(p.fg_dim)).child(text)).child(div().text_color(hsla(p.fg_faint)).child(tick.to_string()))
        };
        let mut col = div().flex().flex_col().flex_none().display(Scale::X1, window).whitespace_nowrap();
        for (r, row) in body.into_iter().enumerate() {
            let (label, tick) = match r {
                0 => (format!("{:>lw$} ", labels[0]), '┤'),
                r if r == last => (format!("{:>lw$} ", labels[1]), '┤'),
                _ => (" ".repeat(lw + 1), '│'),
            };
            col = col.child(div().flex().flex_row().child(axis(label, tick)).child(div().text_color(hsla(p.accent_text)).child(row)));
        }
        col.child(div().text_color(hsla(p.fg_faint)).child(format!("{}└{}", " ".repeat(lw + 1), "─".repeat(self.cols))))
    }
}

// ── Bars ──────────────────────────────────────────────────────────────────

/// A text bar chart: one labelled row per value, the bar a gauge fill
/// scaled to the largest value (or `.max(..)`), a right-aligned readout.
#[derive(IntoElement)]
pub struct AsciiBars {
    bars: Vec<(SharedString, f32)>,
    max: Option<f32>,
    cells: usize,
    format: fn(f32) -> String,
}

pub fn ascii_bars() -> AsciiBars {
    AsciiBars { bars: Vec::new(), max: None, cells: 24, format: |v| format!("{v:.0}") }
}

impl AsciiBars {
    pub fn bar(mut self, label: impl Into<SharedString>, value: f32) -> Self {
        self.bars.push((label.into(), value));
        self
    }

    /// The value a full bar stands for (default: the largest bar).
    pub fn max(mut self, max: f32) -> Self {
        self.max = Some(max);
        self
    }

    /// Bar width in cells (default 24).
    pub fn cells(mut self, cells: usize) -> Self {
        self.cells = cells.max(1);
        self
    }

    pub fn format(mut self, format: fn(f32) -> String) -> Self {
        self.format = format;
        self
    }
}

/// A gauge fill `cells` wide: `·` track, accent quads, a dithered partial
/// cell (quads, not `█▒` glyphs: PITFALLS §47).
fn fill_cells(value: f32, cells: usize, inset: Pixels, window: &Window, cx: &App) -> gpui::Div {
    let p = palette(cx);
    let cell = display_size(Scale::X1, window) / 2.;
    let filled = value.clamp(0., 1.) * cells as f32;
    let (full, frac) = (filled.floor() as usize, filled.fract());
    let track: String = std::iter::repeat_n('·', cells).collect();
    div()
        .relative()
        .flex_none()
        .w(cell * cells as f32)
        .text_color(hsla(p.fg_faint))
        .child(track)
        .child(div().absolute().top(inset).bottom(inset).left_0().w(cell * full as f32).bg(hsla(p.accent)))
        .when(full < cells && frac > 0., |el| {
            let level = ((frac.max(0.25) * 4.).round() / 4.).min(dither::level::DARK);
            el.child(
                div()
                    .absolute()
                    .top(inset)
                    .bottom(inset)
                    .left(cell * full as f32)
                    .w(cell)
                    .bg(hsla(p.bg))
                    .child(dither(dither::flat(level)).ink(hsla(p.accent)).size_full()),
            )
        })
}

impl RenderOnce for AsciiBars {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let max = self.max.unwrap_or_else(|| self.bars.iter().map(|b| b.1).fold(0., f32::max)).max(f32::EPSILON);
        let lw = self.bars.iter().map(|b| b.0.chars().count()).max().unwrap_or(0);
        let readouts: Vec<String> = self.bars.iter().map(|b| (self.format)(b.1)).collect();
        let rw = readouts.iter().map(|r| r.chars().count()).max().unwrap_or(0);
        div().flex().flex_col().flex_none().children(self.bars.iter().zip(readouts).map(|((label, v), readout)| {
            div()
                .flex()
                .flex_row()
                .flex_none()
                .display(Scale::X1, window)
                .whitespace_nowrap()
                .child(div().text_color(hsla(p.fg_dim)).child(format!("{:<lw$} ", label.to_uppercase())))
                .child(fill_cells(v / max, self.cells, px(3.), window, cx))
                .child(div().text_color(hsla(p.fg)).child(format!(" {readout:>rw$}")))
        }))
    }
}

// ── Calendar ──────────────────────────────────────────────────────────────

/// A month as `cal` prints it, in the display face: weekends dim, today in
/// inverse amber. Read-only; `calendar` is the interactive one.
#[derive(IntoElement)]
pub struct AsciiCal {
    year: i32,
    month: u32,
    today: Option<u32>,
    sunday_first: bool,
}

pub fn ascii_cal(year: i32, month: u32) -> AsciiCal {
    AsciiCal { year, month, today: None, sunday_first: false }
}

impl AsciiCal {
    /// Mark this day of the month.
    pub fn today(mut self, day: Option<u32>) -> Self {
        self.today = day;
        self
    }

    pub fn sunday_first(mut self) -> Self {
        self.sunday_first = true;
        self
    }
}

impl RenderOnce for AsciiCal {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let lines = ascii::cal(self.year, self.month, self.sunday_first);
        let weekend = |i: usize| if self.sunday_first { i == 0 || i == 6 } else { i >= 5 };
        let mut col = div()
            .flex()
            .flex_col()
            .flex_none()
            .child(line(lines[0].clone(), window).text_color(hsla(p.fg)))
            .child(line(lines[1].clone(), window).text_color(hsla(p.fg_faint)));
        for week in &lines[2..] {
            let chars: Vec<char> = week.chars().collect();
            let mut row = div().flex().flex_row().display(Scale::X1, window).whitespace_nowrap();
            for (i, cell) in chars.chunks(3).enumerate() {
                let day: String = cell.iter().take(2).collect();
                let is_today = day.trim().parse::<u32>().ok().is_some_and(|d| Some(d) == self.today);
                let ink = if is_today { p.accent_fg } else if weekend(i) { p.fg_dim } else { p.fg };
                row = row
                    .child(div().text_color(hsla(ink)).when(is_today, |el| el.bg(hsla(p.accent))).child(day))
                    .when(cell.len() > 2, |el| el.child(" "));
            }
            col = col.child(row);
        }
        col
    }
}

// ── Marquee ───────────────────────────────────────────────────────────────

/// Text scrolling through a fixed window, one cell per 120ms, looping. A
/// classic for a status ticker or an attract screen; it redraws its view
/// on a timer while shown, like a spinner, so keep it to one per screen.
/// Under reduced motion it holds still.
#[derive(IntoElement)]
pub struct Marquee {
    id: ElementId,
    text: SharedString,
    cells: usize,
    color: Option<Hsla>,
}

pub fn marquee(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Marquee {
    Marquee { id: id.into(), text: text.into(), cells: 32, color: None }
}

impl Marquee {
    /// Window width in cells (default 32).
    pub fn cells(mut self, cells: usize) -> Self {
        self.cells = cells.max(1);
        self
    }

    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

impl RenderOnce for Marquee {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let step = super::ticker::ticker(self.id, crate::motion::FRAME * 3, window, cx);
        line(ascii::marquee(&self.text, self.cells, step as usize), window).text_color(self.color.unwrap_or_else(|| hsla(p.accent_text)))
    }
}

// ── Controls ──────────────────────────────────────────────────────────────

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// A text-mode button: `< OK >`. Hover lights it amber-dim; `.primary()`
/// is inverse amber, the one default action of a text-mode dialog.
#[derive(IntoElement)]
pub struct AsciiButton {
    id: ElementId,
    label: SharedString,
    primary: bool,
    on_click: Option<ClickHandler>,
}

pub fn ascii_button(id: impl Into<ElementId>, label: impl Into<SharedString>) -> AsciiButton {
    AsciiButton { id: id.into(), label: label.into(), primary: false, on_click: None }
}

impl AsciiButton {
    pub fn primary(mut self) -> Self {
        self.primary = true;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for AsciiButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let (ink, bg) = if self.primary { (p.accent_fg, Some(p.accent)) } else { (p.fg, None) };
        div()
            .id(self.id)
            .role(gpui::Role::Button)
            .aria_label(self.label.clone())
            .flex_none()
            .display(Scale::X1, window)
            .whitespace_nowrap()
            .cursor_pointer()
            .text_color(hsla(ink))
            .when_some(bg, |el, bg| el.bg(hsla(bg)))
            .when(!self.primary, |el| el.hover(|s| s.bg(hsla(p.accent_dim))))
            .active(|s| s.bg(hsla(p.fg)).text_color(hsla(p.bg)))
            .child(format!("< {} >", self.label.to_uppercase()))
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .when_some(self.on_click, |el, h| el.on_click(move |ev, window, cx| h(ev, window, cx)))
    }
}

type SelectHandler = Rc<dyn Fn(&usize, &mut Window, &mut App)>;

/// A text-mode pick list: `► ITEM` on the selected row in inverse amber,
/// the others indented, hover in amber-dim. Click to select.
#[derive(IntoElement)]
pub struct AsciiList {
    id: ElementId,
    items: Vec<SharedString>,
    selected: Option<usize>,
    on_select: Option<SelectHandler>,
}

pub fn ascii_list(id: impl Into<ElementId>) -> AsciiList {
    AsciiList { id: id.into(), items: Vec::new(), selected: None, on_select: None }
}

impl AsciiList {
    pub fn item(mut self, label: impl Into<SharedString>) -> Self {
        self.items.push(label.into());
        self
    }

    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    pub fn on_select(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for AsciiList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let width = self.items.iter().map(|l| l.chars().count()).max().unwrap_or(0) + 3;
        div().id(self.id).role(gpui::Role::List).flex().flex_col().flex_none().items_start().children(self.items.into_iter().enumerate().map(|(i, label)| {
            let on = self.selected == Some(i);
            let text = format!("{}{:<w$}", if on { "► " } else { "  " }, label.to_uppercase(), w = width - 2);
            let handler = self.on_select.clone();
            div()
                .id(("ascii-item", i))
                .role(gpui::Role::ListItem)
                .aria_selected(on)
                .display(Scale::X1, window)
                .whitespace_nowrap()
                .cursor_pointer()
                .text_color(hsla(if on { p.accent_fg } else { p.fg }))
                .when(on, |el| el.bg(hsla(p.accent)))
                .when(!on, |el| el.hover(|s| s.bg(hsla(p.accent_dim))))
                .child(text)
                .when_some(handler, |el, h| el.on_click(move |_, window, cx| h(&i, window, cx)))
        }))
    }
}

// ── Film ──────────────────────────────────────────────────────────────────

/// A loop of pictures played as ASCII art: a spinning logo, a turning
/// planet, an attract screen. Every frame is fitted once (by
/// `ascii::picture_art`, cached) and then just shown, so after the first
/// loop playback costs a text re-render per frame. Plays on a timer while
/// shown, like a spinner — continuous motion, so one per screen, for
/// splash and idle states, never behind work. Holds the first frame under
/// reduced motion.
#[derive(IntoElement)]
pub struct AsciiFilm {
    id: ElementId,
    frames: Rc<[Picture]>,
    cols: usize,
    style: ascii::ArtStyle,
    fit: Option<ascii::Fit>,
    fps: u32,
    color: Option<Hsla>,
    masks: Rc<[Picture]>,
    inks: Vec<Hsla>,
}

/// Play `frames` in a loop as ASCII art (default: 64 columns, classic
/// ramp, 12fps).
pub fn ascii_film(id: impl Into<ElementId>, frames: impl Into<Rc<[Picture]>>) -> AsciiFilm {
    AsciiFilm {
        id: id.into(),
        frames: frames.into(),
        cols: 64,
        style: ascii::ArtStyle::default(),
        fit: None,
        fps: 12,
        color: None,
        masks: Rc::from(Vec::new()),
        inks: Vec::new(),
    }
}

impl AsciiFilm {
    pub fn cols(mut self, cols: usize) -> Self {
        self.cols = cols.max(1);
        self
    }

    /// Any `ascii::Charset`; its default fit and diffusion come with it.
    pub fn charset(mut self, charset: ascii::Charset) -> Self {
        let ascii::ArtStyle { contrast, invert, .. } = self.style;
        self.style = ascii::ArtStyle { contrast, invert, ..ascii::ArtStyle::new(charset) };
        self
    }

    pub fn fit(mut self, fit: ascii::Fit) -> Self {
        self.fit = Some(fit);
        self
    }

    pub fn contrast(mut self, contrast: f32) -> Self {
        self.style.contrast = contrast.max(0.);
        self
    }

    pub fn invert(mut self, invert: bool) -> Self {
        self.style.invert = invert;
        self
    }

    /// Playback rate (default 12; clamped to 1–25 — this is a flip book,
    /// not video).
    pub fn fps(mut self, fps: u32) -> Self {
        self.fps = fps.clamp(1, 25);
        self
    }

    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    /// Colour parts of the picture: `masks[n]` goes with frame `n` (fewer
    /// masks than frames repeat), and a mask's raw value at a sample is an
    /// ink index: 0 keeps the film's own `color`, `k` paints with
    /// `inks[k - 1]`. Each character cell takes the ink most of it is
    /// (`ascii::cell_inks`). For illustrations — a sky, a map — not for
    /// status: take the inks from `palette(cx)` where one fits.
    pub fn inks(mut self, masks: impl Into<Rc<[Picture]>>, inks: impl IntoIterator<Item = Hsla>) -> Self {
        self.masks = masks.into();
        self.inks = inks.into_iter().collect();
        self
    }
}

impl RenderOnce for AsciiFilm {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        if self.frames.is_empty() {
            return div();
        }
        let n = super::ticker::ticker(self.id, std::time::Duration::from_millis(1000 / self.fps as u64), window, cx);
        let picture = &self.frames[n as usize % self.frames.len()];
        let style = ascii::ArtStyle { fit: self.fit.unwrap_or(self.style.fit), ..self.style };
        let lines = ascii::picture_art(picture, self.cols, style);
        let cells = (!self.masks.is_empty() && !self.inks.is_empty())
            .then(|| ascii::cell_inks(&self.masks[n as usize % self.frames.len() % self.masks.len()], self.cols, lines.len()));
        let inks = self.inks;
        div()
            .flex()
            .flex_col()
            .flex_none()
            .text_color(self.color.unwrap_or_else(|| hsla(p.fg)))
            .children(lines.iter().enumerate().map(|(r, l)| {
                let row = div().display(Scale::X1, window).whitespace_nowrap();
                match cells.as_ref().and_then(|c| c.get(r)) {
                    Some(cells) => row.child(StyledText::new(l.clone()).with_highlights(ink_runs(l, cells, &inks))),
                    None => row.child(l.clone()),
                }
            }))
    }
}

/// Byte ranges of `line` to paint in each ink, adjacent cells merged.
/// Index 0, out-of-range indices and blank cells keep the base color.
fn ink_runs(line: &str, cells: &[u8], inks: &[Hsla]) -> Vec<(std::ops::Range<usize>, HighlightStyle)> {
    let mut runs: Vec<(std::ops::Range<usize>, u8)> = Vec::new();
    for (c, (at, ch)) in line.char_indices().enumerate() {
        let k = cells.get(c).copied().unwrap_or(0);
        if k == 0 || k as usize > inks.len() || ch == ' ' {
            continue;
        }
        let end = at + ch.len_utf8();
        match runs.last_mut() {
            Some((range, last)) if *last == k && range.end == at => range.end = end,
            _ => runs.push((at..end, k)),
        }
    }
    runs.into_iter().map(|(range, k)| (range, HighlightStyle { color: Some(inks[k as usize - 1]), ..Default::default() })).collect()
}

#[cfg(test)]
mod tests {
    use super::{ink_runs, perimeter};

    #[test]
    fn ink_runs_merge_and_skip_blanks() {
        let (a, b) = (gpui::red(), gpui::blue());
        let runs: Vec<_> = ink_runs("ab cd·e", &[1, 1, 1, 1, 2, 2, 0], &[a, b]).into_iter().map(|(r, h)| (r, h.color)).collect();
        // The blank breaks the run; '·' is two bytes; index 0 stays plain.
        assert_eq!(runs, vec![(0..2, Some(a)), (3..4, Some(a)), (4..7, Some(b))]);
        assert!(ink_runs("ab", &[3, 3], &[a]).is_empty(), "out-of-range ink keeps the base color");
    }


    #[test]
    fn draw_on_traces_clockwise() {
        assert_eq!(perimeter(10., 4., 0.), [0., 0., 0., 0.]);
        assert_eq!(perimeter(10., 4., 12.), [10., 2., 0., 0.]);
        assert_eq!(perimeter(10., 4., 20.), [10., 4., 6., 0.]);
        assert_eq!(perimeter(10., 4., 28.), [10., 4., 10., 4.]);
        assert_eq!(perimeter(10., 4., 99.), [10., 4., 10., 4.]);
    }
}

