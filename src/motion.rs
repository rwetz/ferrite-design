//! The motion vocabulary. Same rule as Nexis — one shared set of timings, no
//! component invents its own — but a different language: Nexis glides on
//! springs; Ferrite *steps*.
//!
//! Machines don't ease. Ferrite motion is either instant or quantised into a
//! few discrete frames, like a CRT redrawing or a progress bar filling in
//! character cells. Springs and long ease-out curves are off-language.
//!
//! - **Instant** is the default for state changes (hover, select, toggle).
//! - **Stepped** ([`steps`]) for things that should visibly *happen*: panels
//!   opening, a dither fading in, a counter ticking.
//! - **Blink** for the caret and "live" markers, at the classic text-mode rate.
//!
//! Reduced motion: check [`reduced`] and render the final state. Blinks
//! become solid, stepped reveals become instant.
//!
//! The engine that plays stepped clips, and the effects built on it, live
//! in [`crate::animate`] (and as drop-in elements in `components::fx`).

use std::time::Duration;

use gpui::App;

/// One stepped frame. ~25fps: deliberately chunkier than the display rate.
pub const FRAME: Duration = Duration::from_millis(40);
/// A short stepped transition (3 frames).
pub const FAST: Duration = Duration::from_millis(120);
/// The default stepped transition (5 frames).
pub const BASE: Duration = Duration::from_millis(200);
/// Large reveals (8 frames).
pub const SLOW: Duration = Duration::from_millis(320);
/// A full caret blink cycle: on for half, off for half (VGA text mode is
/// ~1.9Hz; 1060ms is the familiar terminal feel).
pub const BLINK: Duration = Duration::from_millis(1060);

/// Quantise linear progress into `n` equal steps. Use as an easing function:
/// `Animation::new(motion::BASE).with_easing(motion::steps(5))`.
pub fn steps(n: u32) -> impl Fn(f32) -> f32 + Clone + 'static {
    let n = n.max(1) as f32;
    move |t: f32| ((t * n).floor() / n).min(1.0)
}

/// Square-wave on/off for blink cycles: 1.0 for the first half, 0.0 after.
pub fn blink(t: f32) -> f32 {
    if t < 0.5 { 1.0 } else { 0.0 }
}

/// Whether the user asked the OS for reduced motion.
pub fn reduced(cx: &App) -> bool {
    cx.reduce_motion()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_quantise_and_finish() {
        let s = steps(4);
        assert_eq!(s(0.0), 0.0);
        assert_eq!(s(0.24), 0.0);
        assert_eq!(s(0.26), 0.25);
        assert_eq!(s(0.99), 0.75);
        assert_eq!(s(1.0), 1.0);
    }
}
