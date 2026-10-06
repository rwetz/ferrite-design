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

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use gpui::App;

/// The classic stepped frame, ~25fps: chunkier than the display, the CRT
/// look (`set_fps(25)`). Also the fixed beat for *table* effects (shake,
/// stamp, toast step-in, decrypt churn), which keep their timing at any rate.
pub const FRAME: Duration = Duration::from_millis(40);

/// The default refresh rate: as smooth as any common display shows (120Hz
/// ProMotion included). Apps lower it with [`set_fps`] for the stepped look.
pub const DEFAULT_FPS: u32 = 240;

/// The refresh rates an app can offer, in frames per second. 25 is the
/// classic stepped look, 240 the default; above the display's own refresh,
/// extra frames are never seen.
pub const RATES: [u32; 6] = [12, 25, 30, 60, 120, 240];

/// The fastest frame [`set_frame`] accepts (240fps).
pub const MIN_FRAME: Duration = Duration::from_micros(4_167);
/// The slowest frame [`set_frame`] accepts (~12fps).
pub const MAX_FRAME: Duration = Duration::from_micros(83_334);

static FRAME_MICROS: AtomicU64 = AtomicU64::new(1_000_000 / DEFAULT_FPS as u64);

/// The live frame: how often clips, loading dither and other *smooth*
/// motion (unroll, dissolve, travel, grow, develop…) take a step. Faster
/// means more, smaller steps over the same duration — never a faster
/// animation. Costs CPU only while something is playing.
pub fn frame() -> Duration {
    Duration::from_micros(FRAME_MICROS.load(Ordering::Relaxed))
}

/// Set the live frame, clamped to `MIN_FRAME..=MAX_FRAME`. Process-wide;
/// clips already playing pick it up on their next step.
pub fn set_frame(frame: Duration) {
    FRAME_MICROS.store(clamp_frame(frame).as_micros() as u64, Ordering::Relaxed);
}

/// [`set_frame`] in frames per second.
pub fn set_fps(fps: u32) {
    set_frame(frame_for(fps));
}

/// The live frame rate, rounded to whole frames per second.
pub fn fps() -> u32 {
    fps_for(frame())
}

fn clamp_frame(frame: Duration) -> Duration {
    frame.clamp(MIN_FRAME, MAX_FRAME)
}

fn frame_for(fps: u32) -> Duration {
    Duration::from_micros(1_000_000 / fps.max(1) as u64)
}

fn fps_for(frame: Duration) -> u32 {
    (1_000_000.0 / frame.as_micros() as f64).round() as u32
}
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
    fn frame_rate_is_clamped_and_round_trips() {
        // Pure helpers only: the live frame is process-wide, and other
        // tests read it in parallel.
        assert_eq!(clamp_frame(frame_for(1000)), MIN_FRAME);
        assert_eq!(clamp_frame(frame_for(1)), MAX_FRAME);
        for fps in RATES {
            assert_eq!(fps_for(clamp_frame(frame_for(fps))), fps);
        }
        assert_eq!(fps_for(FRAME), 25);
        assert_eq!(fps_for(Duration::from_micros(1_000_000 / DEFAULT_FPS as u64)), DEFAULT_FPS);
    }

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
