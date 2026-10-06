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
    fn rule_pads_to_width() {
        assert_eq!(rule("io", 12).chars().count(), 12);
    }
}
