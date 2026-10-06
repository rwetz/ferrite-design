//! Text-mode primitives: shade ramps, bars, spinners, sparklines, brackets.
//!
//! These return plain strings so they compose with any element. Mind which
//! face renders them (see fonts.rs): the display face has CP437/WGL4 only,
//! so [`sparkline`] must be set in BODY.

use std::time::Duration;

/// CP437 shade ramp, empty → full. Safe in the display face.
pub const RAMP: [char; 5] = [' ', '░', '▒', '▓', '█'];

/// Eighth-height blocks for sparklines. **BODY face only.**
pub const SPARK: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// The classic line spinner. Safe in the display face.
pub const SPINNER: [char; 4] = ['|', '/', '-', '\\'];

/// Box-drawing set for frames drawn in text (display face).
pub mod boxes {
    pub const H: char = '─';
    pub const V: char = '│';
    pub const TL: char = '┌';
    pub const TR: char = '┐';
    pub const BL: char = '└';
    pub const BR: char = '┘';
    pub const H2: char = '═';
    pub const V2: char = '║';
}

/// The shade glyph for a level in `0..=1`.
pub fn shade(level: f32) -> char {
    let i = (level.clamp(0.0, 1.0) * (RAMP.len() - 1) as f32).round() as usize;
    RAMP[i]
}

/// A text progress bar `width` cells wide: full blocks, then a shaded
/// partial cell, then `·` for the remainder. `bar(0.42, 10)` → `████▒·····`.
pub fn bar(value: f32, width: usize) -> String {
    let filled = value.clamp(0.0, 1.0) * width as f32;
    let full = filled.floor() as usize;
    let mut out = String::with_capacity(width * 3);
    out.extend(std::iter::repeat_n('█', full));
    if full < width {
        let frac = filled - full as f32;
        out.push(if frac > 0.0 { shade(frac.max(0.25)) } else { '·' });
        out.extend(std::iter::repeat_n('·', width - full - 1));
    }
    out
}

/// The spinner frame for `elapsed` time at one frame per `frame`.
pub fn spinner(elapsed: Duration, frame: Duration) -> char {
    let i = (elapsed.as_millis() / frame.as_millis().max(1)) as usize;
    SPINNER[i % SPINNER.len()]
}

/// A one-line chart of `values`, scaled to their own min..max. **BODY face.**
pub fn sparkline(values: &[f32]) -> String {
    let (lo, hi) = values.iter().fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    let span = (hi - lo).max(f32::EPSILON);
    values
        .iter()
        .map(|&v| SPARK[(((v - lo) / span) * (SPARK.len() - 1) as f32).round() as usize])
        .collect()
}

/// `[ LABEL ]` — the Ferrite label grammar: uppercase, bracketed.
pub fn bracket(label: &str) -> String {
    format!("[ {} ]", label.to_uppercase())
}

/// `── label ──────` padded with rules to `width` cells.
pub fn rule(label: &str, width: usize) -> String {
    let head = format!("{h}{h} {label} ", h = boxes::H);
    let used = head.chars().count();
    let mut out = head;
    out.extend(std::iter::repeat_n(boxes::H, width.saturating_sub(used)));
    out
}


// ── Box drawing ───────────────────────────────────────────────────────────

/// The glyphs of one box-drawing style. All of them are in CP437, so the
/// display face draws them on its cell grid and they join seamlessly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoxStyle {
    pub h: char,
    pub v: char,
    pub tl: char,
    pub tr: char,
    pub bl: char,
    pub br: char,
    /// Junctions, for grids: `┬` `┴` `├` `┤` `┼`.
    pub down: char,
    pub up: char,
    pub right: char,
    pub left: char,
    pub cross: char,
}

/// `┌─┐ │ └─┘` — the default.
pub const SINGLE: BoxStyle =
    BoxStyle { h: '─', v: '│', tl: '┌', tr: '┐', bl: '└', br: '┘', down: '┬', up: '┴', right: '├', left: '┤', cross: '┼' };
/// `╔═╗ ║ ╚═╝` — emphasis: the focused or primary box, a modal.
pub const DOUBLE: BoxStyle =
    BoxStyle { h: '═', v: '║', tl: '╔', tr: '╗', bl: '╚', br: '╝', down: '╦', up: '╩', right: '╠', left: '╣', cross: '╬' };
/// `╒═╕ │ ╘═╛` — double rules, single sides: a header block, a ledger.
pub const DOUBLE_H: BoxStyle =
    BoxStyle { h: '═', v: '│', tl: '╒', tr: '╕', bl: '╘', br: '╛', down: '╤', up: '╧', right: '╞', left: '╡', cross: '╪' };
/// `╓─╖ ║ ╙─╜` — single rules, double sides: a column, a sidebar.
pub const DOUBLE_V: BoxStyle =
    BoxStyle { h: '─', v: '║', tl: '╓', tr: '╖', bl: '╙', br: '╜', down: '╥', up: '╨', right: '╟', left: '╢', cross: '╫' };
/// `+-+ | +-+` — plain ASCII, for text that leaves the app: logs, the
/// clipboard, a terminal.
pub const PLAIN: BoxStyle =
    BoxStyle { h: '-', v: '|', tl: '+', tr: '+', bl: '+', br: '+', down: '+', up: '+', right: '+', left: '+', cross: '+' };

/// Every box style, for pickers and demos.
pub const STYLES: [(&str, BoxStyle); 5] =
    [("single", SINGLE), ("double", DOUBLE), ("double_h", DOUBLE_H), ("double_v", DOUBLE_V), ("plain", PLAIN)];

/// One line of a box `cols` cells wide: `┌──────┐`, optionally with a
/// `[ TITLE ]` set into it after two rule cells. Exactly `cols` chars.
pub fn box_edge(style: BoxStyle, cols: usize, top: bool, title: Option<&str>) -> String {
    let (l, r) = if top { (style.tl, style.tr) } else { (style.bl, style.br) };
    if cols < 2 {
        return l.to_string().chars().take(cols).collect();
    }
    let inner = cols - 2;
    let mut mid: String = match title {
        Some(t) if inner >= 6 => {
            let label: String = bracket(t).chars().take(inner.saturating_sub(2)).collect();
            format!("{h}{label}", h = style.h)
        }
        _ => String::new(),
    };
    let used = mid.chars().count();
    mid.extend(std::iter::repeat_n(style.h, inner.saturating_sub(used)));
    let mid: String = mid.chars().take(inner).collect();
    format!("{l}{mid}{r}")
}

// ── Banner: big block letters ─────────────────────────────────────────────

/// The 5×5 block font: `(width, rows)`, each row's bits read left to right.
/// Uppercase letters, digits and `- . ! ? : / _` and space.
pub fn block_glyph(c: char) -> Option<(u32, [u8; 5])> {
    match c.to_ascii_uppercase() {
        'A' => Some((5, [0b01110,0b10001,0b11111,0b10001,0b10001])),
        'B' => Some((5, [0b11110,0b10001,0b11110,0b10001,0b11110])),
        'C' => Some((5, [0b01111,0b10000,0b10000,0b10000,0b01111])),
        'D' => Some((5, [0b11110,0b10001,0b10001,0b10001,0b11110])),
        'E' => Some((5, [0b11111,0b10000,0b11110,0b10000,0b11111])),
        'F' => Some((5, [0b11111,0b10000,0b11110,0b10000,0b10000])),
        'G' => Some((5, [0b01111,0b10000,0b10011,0b10001,0b01110])),
        'H' => Some((5, [0b10001,0b10001,0b11111,0b10001,0b10001])),
        'I' => Some((5, [0b11111,0b00100,0b00100,0b00100,0b11111])),
        'J' => Some((5, [0b00111,0b00001,0b00001,0b10001,0b01110])),
        'K' => Some((5, [0b10001,0b10010,0b11100,0b10010,0b10001])),
        'L' => Some((5, [0b10000,0b10000,0b10000,0b10000,0b11111])),
        'M' => Some((5, [0b10001,0b11011,0b10101,0b10001,0b10001])),
        'N' => Some((5, [0b10001,0b11001,0b10101,0b10011,0b10001])),
        'O' => Some((5, [0b01110,0b10001,0b10001,0b10001,0b01110])),
        'P' => Some((5, [0b11110,0b10001,0b11110,0b10000,0b10000])),
        'Q' => Some((5, [0b01110,0b10001,0b10101,0b10010,0b01101])),
        'R' => Some((5, [0b11110,0b10001,0b11110,0b10010,0b10001])),
        'S' => Some((5, [0b01111,0b10000,0b01110,0b00001,0b11110])),
        'T' => Some((5, [0b11111,0b00100,0b00100,0b00100,0b00100])),
        'U' => Some((5, [0b10001,0b10001,0b10001,0b10001,0b01110])),
        'V' => Some((5, [0b10001,0b10001,0b10001,0b01010,0b00100])),
        'W' => Some((5, [0b10001,0b10001,0b10101,0b11011,0b10001])),
        'X' => Some((5, [0b10001,0b01010,0b00100,0b01010,0b10001])),
        'Y' => Some((5, [0b10001,0b01010,0b00100,0b00100,0b00100])),
        'Z' => Some((5, [0b11111,0b00010,0b00100,0b01000,0b11111])),
        '0' => Some((5, [0b01110,0b10011,0b10101,0b11001,0b01110])),
        '1' => Some((5, [0b00100,0b01100,0b00100,0b00100,0b01110])),
        '2' => Some((5, [0b01110,0b10001,0b00110,0b01000,0b11111])),
        '3' => Some((5, [0b11110,0b00001,0b01110,0b00001,0b11110])),
        '4' => Some((5, [0b10010,0b10010,0b11111,0b00010,0b00010])),
        '5' => Some((5, [0b11111,0b10000,0b11110,0b00001,0b11110])),
        '6' => Some((5, [0b01110,0b10000,0b11110,0b10001,0b01110])),
        '7' => Some((5, [0b11111,0b00001,0b00010,0b00100,0b00100])),
        '8' => Some((5, [0b01110,0b10001,0b01110,0b10001,0b01110])),
        '9' => Some((5, [0b01110,0b10001,0b01111,0b00001,0b01110])),
        '-' => Some((5, [0b00000,0b00000,0b01110,0b00000,0b00000])),
        '.' => Some((5, [0b00000,0b00000,0b00000,0b00000,0b00100])),
        '!' => Some((5, [0b00100,0b00100,0b00100,0b00000,0b00100])),
        '?' => Some((5, [0b01110,0b10001,0b00110,0b00000,0b00100])),
        ':' => Some((5, [0b00000,0b00100,0b00000,0b00100,0b00000])),
        '/' => Some((5, [0b00001,0b00010,0b00100,0b01000,0b10000])),
        '_' => Some((5, [0b00000,0b00000,0b00000,0b00000,0b11111])),
        ' ' => Some((3, [0b000,0b000,0b000,0b000,0b000])),
        _ => None,
    }
}

/// `text` in the 5×5 block font as **three** text rows, two pixel rows per
/// character cell (`▀` top, `▄` bottom, `█` both) — the 8×16 cell is twice
/// as tall as wide, so this keeps the letters square. Unknown characters
/// are skipped; letters are one column apart. Set it in the display face.
pub fn banner_rows(text: &str) -> [String; 3] {
    let mut px: [Vec<bool>; 5] = Default::default();
    for (i, c) in text.chars().filter_map(block_glyph).enumerate() {
        let (w, rows) = c;
        for (y, row) in rows.iter().enumerate() {
            if i > 0 {
                px[y].push(false);
            }
            for x in 0..w {
                px[y].push(row & (1 << (w - 1 - x)) != 0);
            }
        }
    }
    let cell = |top: bool, bottom: bool| match (top, bottom) {
        (true, true) => '█',
        (true, false) => '▀',
        (false, true) => '▄',
        (false, false) => ' ',
    };
    let width = px[0].len();
    let row = |a: usize, b: Option<usize>| (0..width).map(|x| cell(px[a][x], b.is_some_and(|b| px[b][x]))).collect::<String>();
    [row(0, Some(1)), row(2, Some(3)), row(4, None)]
}

// ── ASCII art ─────────────────────────────────────────────────────────────

/// The classic ten-step ASCII art ramp, light to dark ink.
pub const CLASSIC: [char; 10] = [' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];

/// A five-step ramp that reads as shapes rather than tone.
pub const BUBBLES: [char; 5] = [' ', '.', 'o', 'O', '@'];

/// Render `level(u, v)` (0 = paper, 1 = ink; u, v in 0..1) as `cols` columns
/// of characters from `ramp`. Rows follow from `aspect` (height / width of
/// the source) and the 1:2 shape of a text cell, so circles stay round.
pub fn art(level: impl Fn(f32, f32) -> f32, cols: usize, aspect: f32, ramp: &[char]) -> Vec<String> {
    let cols = cols.max(1);
    let rows = ((cols as f32 * aspect) / 2.).round().max(1.) as usize;
    let steps = ramp.len().max(1);
    (0..rows)
        .map(|r| {
            (0..cols)
                .map(|c| {
                    let (u, v) = ((c as f32 + 0.5) / cols as f32, (r as f32 + 0.5) / rows as f32);
                    // Floor, not round: the faintest tenth of tone stays blank,
                    // so a near-empty background doesn't fill with dots.
                    ramp[((level(u, v).clamp(0., 1.) * steps as f32) as usize).min(steps - 1)]
                })
                .collect()
        })
        .collect()
}

// ── Gauges and spinners ───────────────────────────────────────────────────

/// `[████▒·····] 42%` — a bracketed text gauge with a fixed-width readout.
pub fn gauge(value: f32, width: usize) -> String {
    format!("[{}] {:>3}%", bar(value, width), (value.clamp(0., 1.) * 100.).round() as u32)
}

/// Spinner frame sets, all in the display face. `LINE` is [`SPINNER`].
pub mod spinners {
    pub const LINE: &[&str] = &["|", "/", "-", "\\"];
    /// A block pulsing through the shade ramp.
    pub const SHADE: &[&str] = &["░", "▒", "▓", "█", "▓", "▒"];
    /// Three dots filling and draining.
    pub const DOTS: &[&str] = &[".  ", ".. ", "...", " ..", "  .", "   "];
    /// A dot swelling and shrinking.
    pub const PULSE: &[&str] = &["·", "•", "●", "•"];
    /// A block bouncing along a four-cell track.
    pub const BOUNCE: &[&str] = &["█···", "·█··", "··█·", "···█", "··█·", "·█··"];
    /// An arrow going round.
    pub const ARROWS: &[&str] = &["←", "↑", "→", "↓"];
    /// A bar filling and draining in a bracket.
    pub const BAR: &[&str] = &["[    ]", "[=   ]", "[==  ]", "[=== ]", "[ ===]", "[  ==]", "[   =]"];
    /// A dot swelling and shrinking.
    pub const GROW: &[&str] = &[".", "o", "O", "@", "O", "o"];
    /// A diamond flipping between suits.
    pub const SUITS: &[&str] = &["♠", "♣", "♥", "♦"];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_is_always_exactly_width_cells() {
        for v in [0.0, 0.01, 0.42, 0.5, 0.99, 1.0, 1.5] {
            assert_eq!(bar(v, 10).chars().count(), 10, "value {v}");
        }
        assert_eq!(bar(1.0, 4), "████");
        assert_eq!(bar(0.0, 4), "····");
    }

    #[test]
    fn sparkline_spans_the_ramp() {
        let s: Vec<char> = sparkline(&[0.0, 5.0, 10.0]).chars().collect();
        assert_eq!(s, vec!['▁', '▅', '█']);
    }

    #[test]
    fn box_edges_are_exactly_cols_wide() {
        for cols in [2, 3, 8, 9, 40] {
            for title in [None, Some("logs"), Some("a very long title that cannot fit")] {
                let top = box_edge(SINGLE, cols, true, title);
                assert_eq!(top.chars().count(), cols, "{cols} {title:?}: {top}");
                assert!(top.starts_with('┌') && top.ends_with('┐'));
            }
        }
        assert_eq!(box_edge(DOUBLE, 12, true, Some("io")), "╔═[ IO ]═══╗");
        assert_eq!(box_edge(SINGLE, 5, false, None), "└───┘");
    }

    #[test]
    fn banner_rows_line_up_and_cover_the_font() {
        for c in "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-.!?:/_ ".chars() {
            assert!(block_glyph(c).is_some(), "missing glyph {c:?}");
        }
        let rows = banner_rows("HI");
        let w = rows[0].chars().count();
        assert!(rows.iter().all(|r| r.chars().count() == w), "rows are one width");
        assert_eq!(w, 5 + 1 + 5);
        // H's left stem: full blocks top to bottom, bottom row ▀ (pixel row 4).
        assert_eq!(rows[0].chars().next(), Some('█'));
        assert_eq!(rows[2].chars().next(), Some('▀'));
        assert_eq!(banner_rows("é~")[0], "", "unknown characters are skipped");
    }

    #[test]
    fn art_keeps_circles_round() {
        let rows = art(|u, v| if (u - 0.5).hypot(v - 0.5) < 0.4 { 1. } else { 0. }, 20, 1.0, &CLASSIC);
        assert_eq!(rows.len(), 10, "a square source is half as many rows as columns");
        assert!(rows.iter().all(|r| r.chars().count() == 20));
        assert_eq!(rows[5].chars().nth(10), Some('@'));
        assert_eq!(rows[0].chars().next(), Some(' '));
    }

    #[test]
    fn gauges_are_fixed_width() {
        assert_eq!(gauge(0.42, 10).chars().count(), gauge(1.0, 10).chars().count());
        assert!(gauge(1.0, 4).ends_with("100%"));
        for set in [spinners::LINE, spinners::SHADE, spinners::DOTS, spinners::PULSE, spinners::BOUNCE] {
            let w = set[0].chars().count();
            assert!(set.iter().all(|f| f.chars().count() == w), "spinner frames keep their width");
        }
    }

    #[test]
    fn rule_pads_to_width() {
        assert_eq!(rule("io", 12).chars().count(), 12);
    }
}

// ── Grids: tables, trees, plots, calendars ────────────────────────────────

/// Pad or cut `text` to exactly `width` characters, right-aligned if `right`.
fn fit(text: &str, width: usize, right: bool) -> String {
    let t: String = text.chars().take(width).collect();
    let pad = width - t.chars().count();
    if right { format!("{}{t}", " ".repeat(pad)) } else { format!("{t}{}", " ".repeat(pad)) }
}

/// Numbers (and readings like `12.4%`, `188 MB`) right-align in a column.
fn numeric(cell: &str) -> bool {
    cell.trim_start().starts_with(|c: char| c.is_ascii_digit() || c == '-' || c == '+' || c == '.')
}

/// A text table: header, a rule, rows, all boxed in `style` with junctions.
/// Columns size to their widest cell; numbers right-align. Every line is the
/// same width. `table(&["NAME", "PID"], &[vec!["cargo".into(), "9021".into()]], SINGLE)`:
///
/// ```text
/// ┌───────┬──────┐
/// │ NAME  │  PID │
/// ├───────┼──────┤
/// │ cargo │ 9021 │
/// └───────┴──────┘
/// ```
pub fn table(headers: &[&str], rows: &[Vec<String>], style: BoxStyle) -> Vec<String> {
    let cols = headers.len().max(rows.iter().map(Vec::len).max().unwrap_or(0));
    let widths: Vec<usize> = (0..cols)
        .map(|c| {
            let head = headers.get(c).map_or(0, |h| h.chars().count());
            rows.iter().filter_map(|r| r.get(c)).map(|x| x.chars().count()).max().unwrap_or(0).max(head)
        })
        .collect();
    let right: Vec<bool> = (0..cols)
        .map(|c| {
            let cells: Vec<&String> = rows.iter().filter_map(|r| r.get(c)).collect();
            !cells.is_empty() && cells.iter().all(|x| numeric(x))
        })
        .collect();
    let rule = |l: char, j: char, r: char| {
        let mid: Vec<String> = widths.iter().map(|w| style.h.to_string().repeat(w + 2)).collect();
        format!("{l}{}{r}", mid.join(&j.to_string()))
    };
    let line = |cells: &[String]| {
        let mid: Vec<String> = (0..cols).map(|c| format!(" {} ", fit(cells.get(c).map_or("", |x| x), widths[c], right[c]))).collect();
        format!("{v}{}{v}", mid.join(&style.v.to_string()), v = style.v)
    };
    let head: Vec<String> = headers.iter().map(|h| h.to_uppercase()).collect();
    let mut out = vec![rule(style.tl, style.down, style.tr), line(&head), rule(style.right, style.cross, style.left)];
    out.extend(rows.iter().map(|r| line(r)));
    out.push(rule(style.bl, style.up, style.br));
    out
}

/// The guide for each line of an indented outline, `tree`-command style.
/// `items` are `(depth, label)` in display order; returns `(guide, label)`
/// pairs where the guide is `├── `, `└── ` and `│   ` runs. Depth 0 has no
/// guide.
pub fn tree<'a>(items: &[(usize, &'a str)]) -> Vec<(String, &'a str)> {
    // Whether the item at `i` is the last of its siblings: no later item at
    // the same depth before the outline climbs above it.
    let last = |i: usize| {
        let d = items[i].0;
        !items[i + 1..].iter().take_while(|(dd, _)| *dd >= d).any(|(dd, _)| *dd == d)
    };
    let mut open: Vec<bool> = Vec::new(); // per depth: a sibling still follows
    items
        .iter()
        .enumerate()
        .map(|(i, &(depth, label))| {
            open.truncate(depth);
            let mut guide = String::new();
            for d in 1..depth {
                guide.push_str(if open.get(d).copied().unwrap_or(false) { "│   " } else { "    " });
            }
            let is_last = last(i);
            if depth > 0 {
                guide.push_str(if is_last { "└── " } else { "├── " });
            }
            open.resize(depth + 1, false);
            open[depth] = !is_last;
            (guide, label)
        })
        .collect()
}

/// A text plot of `values`, `cols` × `rows` cells, scaled to their own
/// min..max: `*` on each point, `:` joining a point to the next one when it
/// jumps more than a row, so the line reads continuous. Rows top to bottom;
/// no axes (the component draws those).
pub fn plot(values: &[f32], cols: usize, rows: usize) -> Vec<String> {
    let (cols, rows) = (cols.max(2), rows.max(2));
    let mut grid = vec![vec![' '; cols]; rows];
    if values.is_empty() {
        return grid.into_iter().map(|r| r.into_iter().collect()).collect();
    }
    let (lo, hi) = values.iter().fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    let span = (hi - lo).max(f32::EPSILON);
    let sample = |c: usize| {
        let x = c as f32 / (cols - 1) as f32 * (values.len() - 1) as f32;
        let (i, f) = (x.floor() as usize, x.fract());
        let next = values[(i + 1).min(values.len() - 1)];
        values[i] + (next - values[i]) * f
    };
    let row_of = |v: f32| ((1. - (v - lo) / span) * (rows - 1) as f32).round() as usize;
    let ys: Vec<usize> = (0..cols).map(|c| row_of(sample(c))).collect();
    for c in 0..cols {
        if c + 1 < cols {
            let (a, b) = (ys[c].min(ys[c + 1]), ys[c].max(ys[c + 1]));
            for row in grid.iter_mut().take(b).skip(a + 1) {
                row[c] = ':';
            }
        }
        grid[ys[c]][c] = '*';
    }
    grid.into_iter().map(|r| r.into_iter().collect()).collect()
}

/// A month as `cal` prints it: a title line, weekday initials, then weeks of
/// right-aligned day numbers, each line 20 characters (7 cells of 3, minus
/// the trailing space). Weeks start on Monday unless `sunday_first`.
pub fn cal(year: i32, month: u32, sunday_first: bool) -> Vec<String> {
    use crate::components::calendar::{Date, days_in_month};
    const MONTHS: [&str; 12] =
        ["JANUARY", "FEBRUARY", "MARCH", "APRIL", "MAY", "JUNE", "JULY", "AUGUST", "SEPTEMBER", "OCTOBER", "NOVEMBER", "DECEMBER"];
    let first = Date::new(year, month, 1);
    let title = format!("{} {year}", MONTHS[(first.month - 1) as usize]);
    let mut out = vec![format!("{:^20}", title).trim_end().to_string()];
    out.push(if sunday_first { "Su Mo Tu We Th Fr Sa" } else { "Mo Tu We Th Fr Sa Su" }.to_string());
    let lead = if sunday_first { (first.weekday() + 1) % 7 } else { first.weekday() } as usize;
    let mut cells: Vec<String> = vec!["  ".into(); lead];
    cells.extend((1..=days_in_month(first.year, first.month)).map(|d| format!("{d:>2}")));
    for week in cells.chunks(7) {
        out.push(week.join(" "));
    }
    out
}

/// The `width`-character window of a scrolling `text` at `step`: the text
/// loops with a three-space gap, moving one character per step.
pub fn marquee(text: &str, width: usize, step: usize) -> String {
    let looped: Vec<char> = text.chars().chain("   ".chars()).collect();
    if looped.is_empty() {
        return " ".repeat(width);
    }
    (0..width).map(|i| looped[(step + i) % looped.len()]).collect()
}

/// `lines` boxed in `style`, optionally titled — the whole frame as strings,
/// for text that leaves the app (use [`PLAIN`] for the clipboard).
pub fn frame(lines: &[&str], style: BoxStyle, title: Option<&str>) -> Vec<String> {
    let inner = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0).max(title.map_or(0, |t| t.chars().count() + 6));
    let cols = inner + 4;
    let mut out = vec![box_edge(style, cols, true, title)];
    out.extend(lines.iter().map(|l| format!("{v} {} {v}", fit(l, inner, false), v = style.v)));
    out.push(box_edge(style, cols, false, None));
    out
}

#[cfg(test)]
mod grid_tests {
    use super::*;

    #[test]
    fn tables_are_rectangular_and_align_numbers() {
        let rows = vec![vec!["cargo".to_string(), "9021".to_string()], vec!["sshd".to_string(), "612".to_string()]];
        for (_, style) in STYLES {
            let t = table(&["name", "pid"], &rows, style);
            assert_eq!(t.len(), 6);
            let w = t[0].chars().count();
            assert!(t.iter().all(|l| l.chars().count() == w), "{t:#?}");
        }
        let t = table(&["name", "pid"], &rows, SINGLE);
        assert_eq!(t[0], "┌───────┬──────┐");
        assert_eq!(t[1], "│ NAME  │  PID │");
        assert_eq!(t[4], "│ sshd  │  612 │");
        assert_eq!(t[5], "└───────┴──────┘");
    }

    #[test]
    fn tree_guides_follow_siblings() {
        let items = [(0, "src/"), (1, "components/"), (2, "menu.rs"), (2, "tree.rs"), (1, "lib.rs"), (0, "Cargo.toml")];
        let lines: Vec<String> = tree(&items).into_iter().map(|(g, l)| format!("{g}{l}")).collect();
        assert_eq!(lines, ["src/", "├── components/", "│   ├── menu.rs", "│   └── tree.rs", "└── lib.rs", "Cargo.toml"]);
    }

    #[test]
    fn plots_mark_every_column_and_join_jumps() {
        let p = plot(&[0., 10., 0.], 5, 4);
        assert_eq!(p.len(), 4);
        assert!(p.iter().all(|r| r.chars().count() == 5));
        for c in 0..5 {
            assert_eq!(p.iter().filter(|r| r.chars().nth(c) == Some('*')).count(), 1, "one point per column");
        }
        assert!(p.iter().any(|r| r.contains(':')), "a jump is joined");
        assert!(p[0].contains('*') && p[3].contains('*'), "spans the height");
    }

    #[test]
    fn cal_matches_the_unix_layout() {
        let c = cal(2026, 10, false);
        assert_eq!(c[1], "Mo Tu We Th Fr Sa Su");
        // 1 October 2026 is a Thursday.
        assert_eq!(c[2], "          1  2  3  4");
        assert!(c.iter().skip(2).all(|l| l.chars().count() <= 20));
        assert!(c.last().unwrap().trim_end().ends_with("31"));
        assert_eq!(cal(2026, 10, true)[2], "             1  2  3");
    }

    #[test]
    fn marquee_loops_and_frames_box() {
        assert_eq!(marquee("ABC", 4, 0), "ABC ");
        assert_eq!(marquee("ABC", 4, 3), "   A");
        assert_eq!(marquee("ABC", 4, 6), "ABC ");
        let f = frame(&["hello", "hi"], PLAIN, None);
        assert_eq!(f, ["+-------+", "| hello |", "| hi    |", "+-------+"].map(String::from));
    }
}

// ── ASCII art, fitted to the face ─────────────────────────────────────────

/// Which characters a picture may be drawn with. Every set is real glyphs
/// of the display face; the art engine knows each one's pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Charset {
    /// ` .:-=+*#%@` — the classic ramp.
    Classic,
    /// Every ASCII punctuation mark, nothing else.
    Punctuation,
    /// `/` and `\` only: pictures as hatching.
    Slashes,
    /// `/ \ | - _`: line art.
    Lines,
    /// Accented letters (`é â ü Ñ Å …`) and the bare accents.
    Accents,
    /// A–Z and a–z.
    Letters,
    /// 0–9.
    Digits,
    /// `0` and `1`.
    Binary,
    /// CP437's Greek: `α ß Γ π Σ σ µ τ Φ Θ Ω δ φ ε`.
    Greek,
    /// Box-drawing pieces, single and double.
    Box,
    /// Shade and half blocks: `░ ▒ ▓ █ ▀ ▄ ▌ ▐`.
    Blocks,
    /// CP437's dingbats: `☺ ♥ ♦ ♣ ♠ ♪ ☼ ► ↕ ▲ …`.
    Symbols,
    /// The best character for every cell, from every text glyph the face
    /// has (~200: ASCII, accents, Greek, maths, box drawing). Shade blocks
    /// and dingbats are left out: blocks would win every cell on tone and
    /// turn the picture back into pixels ([`Charset::Blocks`]), and dingbats
    /// read as icons ([`Charset::Symbols`]).
    Full,
}

impl Charset {
    pub const ALL: [Charset; 13] = [
        Charset::Classic,
        Charset::Punctuation,
        Charset::Slashes,
        Charset::Lines,
        Charset::Accents,
        Charset::Letters,
        Charset::Digits,
        Charset::Binary,
        Charset::Greek,
        Charset::Box,
        Charset::Blocks,
        Charset::Symbols,
        Charset::Full,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Charset::Classic => "classic",
            Charset::Punctuation => "punctuation",
            Charset::Slashes => "slashes",
            Charset::Lines => "lines",
            Charset::Accents => "accents",
            Charset::Letters => "letters",
            Charset::Digits => "digits",
            Charset::Binary => "binary",
            Charset::Greek => "greek",
            Charset::Box => "box drawing",
            Charset::Blocks => "blocks",
            Charset::Symbols => "symbols",
            Charset::Full => "best character",
        }
    }

    /// The characters, space first. `Full` is every glyph the engine knows.
    pub fn chars(self) -> String {
        match self {
            Charset::Classic => " .:-=+*#%@".into(),
            Charset::Punctuation => " !\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~".into(),
            Charset::Slashes => " /\\".into(),
            Charset::Lines => " /\\|-_".into(),
            Charset::Accents => " `^~ÇüéâäàåçêëèïîìÄÅÉôöòûùÿÖÜáíóúñÑ".into(),
            Charset::Letters => std::iter::once(' ').chain('A'..='Z').chain('a'..='z').collect(),
            Charset::Digits => " 0123456789".into(),
            Charset::Binary => " 01".into(),
            Charset::Greek => " αßΓπΣσµτΦΘΩδφε".into(),
            Charset::Box => " ─│┌┐└┘├┤┬┴┼═║╔╗╚╝╠╣╦╩╬╒╕╘╛╓╖╙╜╞╡╤╧╟╢╥╨╪╫".into(),
            Charset::Blocks => " ░▒▓█▀▄▌▐".into(),
            Charset::Symbols => " ☺☻♥♦♣♠•◘○◙♂♀♪♫☼►◄↕‼¶§▬↨↑↓→←∟↔▲▼⌂".into(),
            Charset::Full => {
                // Text glyphs only: no shade blocks (they'd turn the picture
                // back into pixels) and no dingbats (☺ ♥ ◙ read as icons).
                let skip = Charset::Blocks.chars() + &Charset::Symbols.chars() + &Charset::Box.chars();
                crate::glyphs::GLYPHS.iter().map(|g| g.0).filter(|c| *c == ' ' || !skip.contains(*c)).collect()
            }
        }
    }

    /// How this set reads best: tone for ramps, shape for everything that
    /// has geometry to match.
    pub fn default_fit(self) -> Fit {
        match self {
            Charset::Classic | Charset::Digits | Charset::Binary => Fit::Tone,
            _ => Fit::Shape,
        }
    }
}

/// How a cell picks its character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fit {
    /// By ink: the glyph whose coverage is closest to the cell's darkness.
    /// The ramp is sorted from the glyphs' real pixels, so any set works.
    Tone,
    /// By shape: the glyph whose 4×8 coverage best matches the picture
    /// under the cell, so edges become `/`, `|`, `_`, `▄`… where they fall.
    Shape,
}

/// Everything that decides how a picture becomes characters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArtStyle {
    pub charset: Charset,
    pub fit: Fit,
    /// Gain around mid-grey before fitting: 1 = as is, 2 = punchy.
    pub contrast: f32,
    /// Swap ink and paper (light pictures on a dark ground read better).
    pub invert: bool,
    /// Spread each cell's tone error to its neighbours (Floyd–Steinberg),
    /// so a short ramp or a sparse set still carries smooth gradients — as
    /// density. Shape fits keep choosing by shape; only the ink carries over.
    pub diffuse: bool,
}

impl ArtStyle {
    /// The style that suits `charset`: its default fit, and diffusion for
    /// the sparse sets (slashes, lines, binary), whose few glyphs can only
    /// carry tone as density.
    pub fn new(charset: Charset) -> Self {
        let diffuse = matches!(charset, Charset::Slashes | Charset::Lines | Charset::Binary);
        ArtStyle { charset, fit: charset.default_fit(), contrast: 1., invert: false, diffuse }
    }

    fn key(&self) -> (Charset, Fit, u32, bool, bool) {
        (self.charset, self.fit, self.contrast.to_bits(), self.invert, self.diffuse)
    }
}

impl Default for ArtStyle {
    fn default() -> Self {
        ArtStyle::new(Charset::Classic)
    }
}

/// Sub-samples per cell: the 8×16 glyph in 2×2-pixel blocks.
const SUB_W: usize = 4;
const SUB_H: usize = 8;
const SUBS: usize = SUB_W * SUB_H;

/// How much a cell's overall ink counts against its shape in a shape fit.
const TONE: f32 = 1.5;

/// A 3×3 box blur over a cell's 4×8 coverage, edges clamped.
fn blur(c: &[f32; SUBS]) -> [f32; SUBS] {
    let mut out = [0.; SUBS];
    for y in 0..SUB_H {
        for x in 0..SUB_W {
            let (mut sum, mut n) = (0., 0.);
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let (xx, yy) = (x as i32 + dx, y as i32 + dy);
                    if (0..SUB_W as i32).contains(&xx) && (0..SUB_H as i32).contains(&yy) {
                        sum += c[yy as usize * SUB_W + xx as usize];
                        n += 1.;
                    }
                }
            }
            out[y * SUB_W + x] = sum / n;
        }
    }
    out
}

/// A glyph's ink in each 2×2-pixel block of its 8×16 cell, 0–1.
fn coverage(rows: &[u8; 16]) -> [f32; SUBS] {
    let mut out = [0.; SUBS];
    for (y, row) in rows.iter().enumerate() {
        for x in 0..8 {
            if row & (0x80 >> x) != 0 {
                out[(y / 2) * SUB_W + x / 2] += 0.25;
            }
        }
    }
    out
}

struct Shape {
    ch: char,
    cover: [f32; SUBS],
    ink: f32,
}

fn shapes(charset: Charset) -> Vec<Shape> {
    let wanted = charset.chars();
    crate::glyphs::GLYPHS
        .iter()
        .filter(|(c, _)| wanted.contains(*c))
        .map(|(ch, rows)| {
            let cover = coverage(rows);
            Shape { ch: *ch, ink: cover.iter().sum::<f32>() / SUBS as f32, cover }
        })
        .collect()
}

/// `level(u, v)` (0 = paper, 1 = ink) drawn as `cols` columns of `style`'s
/// characters, rows following `aspect` (height / width) and the 1:2 cell.
/// Unlike [`art`], it knows each glyph's pixels: [`Fit::Shape`] picks the
/// character that best matches the picture under every cell. Costs
/// `cols × rows × glyphs × 32` — cache the result (the `ascii_art`
/// component does).
pub fn art_fit(level: impl Fn(f32, f32) -> f32, cols: usize, aspect: f32, style: ArtStyle) -> Vec<String> {
    let cols = cols.max(1);
    let rows = ((cols as f32 * aspect) / 2.).round().max(1.) as usize;
    let glyphs = shapes(style.charset);
    if glyphs.is_empty() {
        return vec![" ".repeat(cols); rows];
    }
    // Map picture ink onto what the set can actually draw: the densest
    // glyph stands for full ink.
    let densest = glyphs.iter().map(|g| g.ink).fold(0., f32::max).max(f32::EPSILON);
    let adjust = |v: f32| {
        let v = if style.invert { 1. - v } else { v };
        ((v - 0.5) * style.contrast + 0.5).clamp(0., 1.)
    };
    let sample = |c: usize, r: usize| {
        let mut s = [0.; SUBS];
        // Each 2×2-pixel block is the mean of four samples inside it, so
        // strokes thinner than a block still register.
        for j in 0..SUB_H {
            for i in 0..SUB_W {
                let mut sum = 0.;
                for (di, dj) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
                    let u = (c as f32 + (i as f32 + di) / SUB_W as f32) / cols as f32;
                    let v = (r as f32 + (j as f32 + dj) / SUB_H as f32) / rows as f32;
                    sum += adjust(level(u, v));
                }
                s[j * SUB_W + i] = sum / 4.;
            }
        }
        s
    };
    match style.fit {
        Fit::Tone => {
            let mut grid: Vec<Vec<f32>> = (0..rows)
                .map(|r| (0..cols).map(|c| sample(c, r).iter().sum::<f32>() / SUBS as f32 * densest).collect())
                .collect();
            let mut out = Vec::with_capacity(rows);
            for r in 0..rows {
                let mut line = String::with_capacity(cols);
                for c in 0..cols {
                    let want = grid[r][c];
                    let g = glyphs.iter().min_by(|a, b| (a.ink - want).abs().total_cmp(&(b.ink - want).abs())).unwrap();
                    line.push(g.ch);
                    if style.diffuse {
                        let err = want - g.ink;
                        let mut spread = |r: usize, c: usize, w: f32| {
                            if let Some(cell) = grid.get_mut(r).and_then(|row| row.get_mut(c)) {
                                *cell += err * w;
                            }
                        };
                        spread(r, c + 1, 7. / 16.);
                        if c > 0 {
                            spread(r + 1, c - 1, 3. / 16.);
                        }
                        spread(r + 1, c, 5. / 16.);
                        spread(r + 1, c + 1, 1. / 16.);
                    }
                }
                out.push(line);
            }
            out
        }
        Fit::Shape => {
            // Compare what the eye sees: both the glyph and the picture,
            // softened, so a thin stroke counts as the mid-tone it reads as;
            // plus a tone term that keeps each cell's ink right. The picture
            // is scaled toward the set's densest glyph, so full ink maps to
            // it, but never below half (line art keeps its thin strokes).
            let scale = densest.max(0.5);
            let soft: Vec<[f32; SUBS]> = glyphs.iter().map(|g| blur(&g.cover)).collect();
            let mut carry = vec![vec![0f32; cols + 2]; rows + 1];
            let mut out = Vec::with_capacity(rows);
            for r in 0..rows {
                let mut line = String::with_capacity(cols);
                for c in 0..cols {
                    let raw = sample(c, r);
                    let s = raw.map(|v| v * scale);
                    let bs = blur(&s);
                    let mean = raw.iter().sum::<f32>() / SUBS as f32;
                    // Diffusing, ink is density, so it aims at the set's own
                    // range (full ink = the densest glyph) plus the carried
                    // error; otherwise at the picture as the shape sees it.
                    let m = if style.diffuse { mean * densest + carry[r][c + 1] } else { mean * scale };
                    let cost = |i: usize| {
                        let shape: f32 = soft[i].iter().zip(&bs).map(|(g, s)| (g - s) * (g - s)).sum();
                        shape + TONE * SUBS as f32 * (glyphs[i].ink - m).powi(2)
                    };
                    let best = (0..glyphs.len()).min_by(|&a, &b| cost(a).total_cmp(&cost(b))).unwrap();
                    line.push(glyphs[best].ch);
                    if style.diffuse {
                        let err = m - glyphs[best].ink;
                        carry[r][c + 2] += err * 7. / 16.;
                        carry[r + 1][c] += err * 3. / 16.;
                        carry[r + 1][c + 1] += err * 5. / 16.;
                        carry[r + 1][c + 2] += err / 16.;
                    }
                }
                out.push(line);
            }
            out
        }
    }
}

/// Fitted art by (picture id, columns, style).
type ArtCache = std::collections::HashMap<(u64, usize, (Charset, Fit, u32, bool, bool)), std::rc::Rc<Vec<String>>>;

thread_local! {
    static ART_CACHE: std::cell::RefCell<ArtCache> = Default::default();
}

/// [`art_fit`] for a [`crate::dither::Picture`], cached by the picture's
/// content, the width and the style: fitting is expensive, re-rendering a
/// view is not.
pub fn picture_art(picture: &crate::dither::Picture, cols: usize, style: ArtStyle) -> std::rc::Rc<Vec<String>> {
    let key = (picture.id(), cols, style.key());
    if let Some(hit) = ART_CACHE.with_borrow(|c| c.get(&key).cloned()) {
        return hit;
    }
    let (w, h) = picture.size();
    let lines = std::rc::Rc::new(art_fit(|u, v| picture.sample(u, v), cols, h as f32 / w.max(1) as f32, style));
    ART_CACHE.with_borrow_mut(|c| {
        if c.len() >= 96 {
            c.clear();
        }
        c.insert(key, lines.clone());
    });
    lines
}

#[cfg(test)]
mod art_tests {
    use super::*;

    #[test]
    fn every_charset_is_real_glyphs() {
        let known: String = crate::glyphs::GLYPHS.iter().map(|g| g.0).collect();
        for set in Charset::ALL {
            let missing: String = set.chars().chars().filter(|c| !known.contains(*c)).collect();
            assert!(missing.is_empty(), "{set:?} uses glyphs the table lacks: {missing:?}");
            assert!(set.chars().starts_with(' '), "{set:?} needs paper");
        }
        assert!(crate::glyphs::GLYPHS.len() > 200);
        for (c, _) in crate::glyphs::GLYPHS {
            assert!(crate::fonts::display_has(*c) || *c == ' ', "{c:?} not in the display face");
        }
    }

    #[test]
    fn shape_fit_draws_edges_with_their_glyphs() {
        // A diagonal edge, ink below-right of it: slashes should pick '/'.
        let lines = art_fit(|u, v| if v > 1. - u { 1. } else { 0. }, 8, 1., ArtStyle { diffuse: false, ..ArtStyle::new(Charset::Slashes) });
        // The edge itself is '/' on every row; the solid ink behind it hatches.
        for line in &lines {
            assert_eq!(line.trim_start().chars().next(), Some('/'), "{lines:?}");
        }
        // A vertical edge in line art becomes bars.
        let lines = art_fit(|u, _| if (u - 0.5625).abs() < 0.02 { 1. } else { 0. }, 8, 1., ArtStyle { diffuse: false, ..ArtStyle::new(Charset::Lines) });
        assert!(lines.concat().contains('|'), "{lines:?}");
    }

    #[test]
    fn tone_fit_orders_ink_and_paper() {
        let style = ArtStyle::new(Charset::Classic);
        let lines = art_fit(|u, _| u, 20, 0.1, style);
        let row: Vec<char> = lines[0].chars().collect();
        assert_eq!(row[0], ' ');
        // The densest glyph of the set (in the VGA face that's '#').
        let densest = *row.last().unwrap();
        assert!(densest == '#' || densest == '@', "{row:?}");
        let inverted = art_fit(|u, _| u, 20, 0.1, ArtStyle { invert: true, ..style });
        assert_eq!(inverted[0].chars().next(), Some(densest));
        // Diffusion keeps a mid-grey's average while using only two glyphs.
        let flat = art_fit(|_, _| 0.5, 40, 0.5, ArtStyle { diffuse: true, ..ArtStyle::new(Charset::Binary) });
        let chars: String = flat.concat();
        assert!(chars.contains(' ') && (chars.contains('0') || chars.contains('1')), "{chars}");
    }
}
