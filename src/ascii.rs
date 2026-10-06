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
}

/// `┌─┐ │ └─┘` — the default.
pub const SINGLE: BoxStyle = BoxStyle { h: '─', v: '│', tl: '┌', tr: '┐', bl: '└', br: '┘' };
/// `╔═╗ ║ ╚═╝` — emphasis: the focused or primary box, a modal.
pub const DOUBLE: BoxStyle = BoxStyle { h: '═', v: '║', tl: '╔', tr: '╗', bl: '╚', br: '╝' };

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
