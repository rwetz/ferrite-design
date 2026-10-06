//! A boot screen: a text-mode POST that runs once when a window opens, then
//! hands the window to the app.
//!
//! ```text
//!  FERRITE
//!  BIOS v0.3 · GPUI
//!  ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒
//!  MEMORY TEST ......... 65536K OK
//!  DISPLAY ............. 240FPS
//!  PALETTE ............. FERRITE
//!  …
//!  BOOT CONSOLE█
//! ```
//!
//! It waits for the power-on, prints one line every three beats (a dot
//! leader types on, the status decrypts), types the boot line, then
//! dissolves into the app. About 1.4s, and any key or click skips straight
//! to the dissolve. The app renders underneath the whole time, so its
//! state, focus and shortcuts are live from the first frame. Reduced
//! motion skips it entirely.
//!
//! Wrap the root inside `power_on_in`:
//!
//! ```ignore
//! window_frame().child(power_on_in("power", boot_screen("boot", root).title("Console")))
//! ```
//!
//! A boot screen is a *launch* effect for apps that want the retro
//! ceremony (consoles, terminal-styled tools). It is opt-in and plays once
//! per window.

use std::time::Duration;

use gpui::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, prelude::FluentBuilder as _, px,
};

use crate::animate::{self, Progress};
use crate::dither::{self, dither};
use crate::fonts::{FerriteText, Scale};
use crate::motion;
use crate::theme::{palette, scheme, tone};
use crate::tokens::{Tone, hsla};

/// One line of the POST: a label, a dot leader and a status.
#[derive(Clone)]
struct Line {
    label: SharedString,
    status: Status,
}

#[derive(Clone)]
enum Status {
    Text(SharedString),
    /// A memory test: counts up to `n`K, then OK.
    Count(u32),
}

#[derive(IntoElement)]
pub struct BootScreen {
    id: ElementId,
    child: AnyElement,
    title: SharedString,
    lines: Vec<Line>,
}

/// A boot screen over `child` (the window's root content).
pub fn boot_screen(id: impl Into<ElementId>, child: impl IntoElement) -> BootScreen {
    BootScreen { id: id.into(), child: child.into_any_element(), title: "SYSTEM".into(), lines: Vec::new() }
}

impl BootScreen {
    /// The app's name, typed on the last line: `BOOT <TITLE>`.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Add a POST line. Any lines replace the default ones (memory,
    /// display, palette, tone, dither, fonts). Keep labels and statuses
    /// short, in capitals; up to eight lines keeps the boot contained.
    pub fn line(mut self, label: impl Into<SharedString>, status: impl Into<SharedString>) -> Self {
        self.lines.push(Line { label: label.into(), status: Status::Text(status.into()) });
        self
    }
}

// ── Timeline (pure) ───────────────────────────────────────────────────────

/// Each POST line takes three beats.
const LINE: Duration = Duration::from_millis(120);
/// The boot line types on, then holds a beat.
const HOLD: Duration = Duration::from_millis(240);
/// Dot leaders pad every line to this many characters before the status.
const LEADER: usize = 22;

/// How long the POST runs before the hand-off, for `n` lines: the power-on,
/// the lines, the boot line.
fn length(n: usize) -> Duration {
    motion::SLOW + LINE * n as u32 + HOLD
}

/// How far line `i` has printed at `elapsed`, 0→1, or `None` before it starts.
fn line_t(elapsed: Duration, i: usize) -> Option<f32> {
    let start = motion::SLOW + LINE * i as u32;
    let into = elapsed.checked_sub(start)?;
    Some((into.as_secs_f32() / LINE.as_secs_f32()).min(1.))
}

/// How far the boot line has typed, 0→1, or `None` before it starts.
fn boot_t(elapsed: Duration, n: usize) -> Option<f32> {
    let start = motion::SLOW + LINE * n as u32;
    let into = elapsed.checked_sub(start)?;
    // Typed over the first two thirds; the rest is the hold.
    Some((into.as_secs_f32() / (HOLD.as_secs_f32() * 0.66)).min(1.))
}

/// The label, padded with a dot leader that types on over the first half of
/// the line.
fn leader(label: &str, t: f32) -> String {
    let dots = LEADER.saturating_sub(label.chars().count() + 1).max(3);
    let shown = ((t * 2.).min(1.) * dots as f32).round() as usize;
    format!("{label} {}", ".".repeat(shown))
}

/// The status over the second half of the line: decrypting, or a memory
/// count rolling up. Empty in the first half.
fn status_text(status: &Status, t: f32, frame: u32) -> String {
    if t < 0.5 {
        return String::new();
    }
    let p = Progress { t: (t - 0.5) * 2., frame, done: t >= 1. };
    match status {
        Status::Text(s) => animate::scramble(s, p),
        Status::Count(n) if p.done => format!("{n}K OK"),
        Status::Count(n) => format!("{}K", animate::count(0., *n as f32, p).round() as u32),
    }
}

// ── Rendering ─────────────────────────────────────────────────────────────

fn default_lines(cx: &App) -> Vec<Line> {
    let text = |label: &'static str, status: String| Line { label: label.into(), status: Status::Text(status.into()) };
    vec![
        Line { label: "MEMORY TEST".into(), status: Status::Count(65536) },
        text("DISPLAY", format!("{}FPS", motion::fps())),
        text("PALETTE", scheme(cx).name.to_uppercase()),
        text("TONE", if tone(cx) == Tone::Dark { "DARK" } else { "LIGHT" }.into()),
        text("DITHER", "BAYER 4X4 OK".into()),
        text("FONTS", "VGA 8X16 OK".into()),
    ]
}

impl RenderOnce for BootScreen {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let lines = if self.lines.is_empty() { default_lines(cx) } else { self.lines };
        let child = |id: &ElementId, name: &'static str| ElementId::NamedChild(std::sync::Arc::new(id.clone()), name.into());

        let skip = window.use_keyed_state(child(&self.id, "skip"), cx, |_, _| false);
        let post = animate::play(child(&self.id, "post"), 0u8, length(lines.len()), window, cx);
        let handed_off = post.done || *skip.read(cx);
        let handoff = animate::play_on_change(child(&self.id, "handoff"), handed_off, motion::BASE, window, cx);

        let root = div().size_full().flex().flex_col().relative().child(self.child);
        if handed_off && handoff.done {
            return root;
        }
        if handed_off {
            // The POST is gone; the app dissolves in from under the ground.
            let level = animate::dissolve_level(handoff);
            return root.when(level > 0., |el| {
                el.child(div().absolute().inset_0().child(dither(dither::flat(level)).ink(hsla(p.bg)).size_full()))
            });
        }

        let elapsed = length(lines.len()).mul_f32(post.t);
        let row = || div().display(Scale::X1, window).h(px(18.)).flex().flex_row().gap_2();
        let mut screen = div()
            .id(child(&self.id, "screen"))
            .absolute()
            .inset_0()
            .occlude()
            .bg(hsla(p.bg))
            .p(px(32.))
            .flex()
            .flex_col()
            .gap_1()
            .child(div().display(Scale::X2, window).text_color(hsla(p.accent_text)).child("FERRITE"))
            .child(
                div()
                    .display(Scale::X1, window)
                    .text_color(hsla(p.fg_dim))
                    .child(format!("BIOS v{} · GPUI", env!("CARGO_PKG_VERSION"))),
            )
            .child(div().w(px(LEADER as f32 * 10. + 120.)).h(px(4.)).my_2().child(dither(dither::flat(dither::level::MEDIUM)).ink(hsla(p.line_strong)).size_full()));
        for (i, line) in lines.iter().enumerate() {
            let Some(t) = line_t(elapsed, i) else { break };
            screen = screen.child(
                row()
                    .child(div().text_color(hsla(p.fg_dim)).child(leader(&line.label, t)))
                    .child(div().text_color(hsla(if t >= 1. { p.fg } else { p.accent_text })).child(status_text(&line.status, t, post.frame))),
            );
        }
        if let Some(t) = boot_t(elapsed, lines.len()) {
            let boot = format!("BOOT {}", self.title.to_uppercase());
            // Keep the cursor through the hold: typing, then a solid block.
            let typed = animate::type_on(&boot, Progress { t, frame: post.frame, done: false });
            screen = screen.child(row().mt_2().text_color(hsla(p.accent_text)).child(typed));
        }

        // Any key or click skips to the hand-off. Window-level listeners:
        // at launch nothing may have focus yet.
        let skip_listen = skip.clone();
        screen = screen.child(
            gpui::canvas(
                |_, _, _| (),
                move |_, _, window, _| {
                    let s = skip_listen.clone();
                    window.on_key_event(move |_: &gpui::KeyDownEvent, phase, _, cx| {
                        if phase.bubble() {
                            s.update(cx, |v, cx| {
                                *v = true;
                                cx.notify();
                            });
                        }
                    });
                    let s = skip_listen.clone();
                    window.on_mouse_event(move |_: &gpui::MouseDownEvent, phase, _, cx| {
                        if phase.bubble() {
                            s.update(cx, |v, cx| {
                                *v = true;
                                cx.notify();
                            });
                        }
                    });
                },
            )
            .absolute()
            .size_full(),
        );
        root.child(screen)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_print_in_order_after_the_power_on() {
        assert_eq!(line_t(Duration::from_millis(100), 0), None, "waits for the power-on");
        assert_eq!(line_t(motion::SLOW, 0), Some(0.));
        assert_eq!(line_t(motion::SLOW + LINE, 0), Some(1.));
        assert_eq!(line_t(motion::SLOW + LINE, 1), Some(0.));
        assert_eq!(line_t(motion::SLOW + LINE, 2), None);
        assert!(boot_t(length(6) - HOLD, 6) == Some(0.));
        assert_eq!(boot_t(length(6), 6), Some(1.));
    }

    #[test]
    fn the_whole_post_stays_short() {
        assert!(length(6) <= Duration::from_millis(1300));
        assert!(length(8) + motion::BASE <= Duration::from_millis(1800));
    }

    #[test]
    fn leaders_type_on_and_line_up() {
        assert_eq!(leader("FONTS", 0.), "FONTS ");
        let full = leader("FONTS", 1.);
        assert_eq!(full.chars().count(), LEADER);
        assert_eq!(leader("MEMORY TEST", 0.5).chars().count(), LEADER);
        // A label longer than the leader still gets three dots.
        assert!(leader("A VERY LONG LABEL INDEED", 1.).ends_with("..."));
    }

    #[test]
    fn statuses_decrypt_and_count_in_the_second_half() {
        let ok = Status::Text("OK".into());
        assert_eq!(status_text(&ok, 0.3, 0), "");
        assert_eq!(status_text(&ok, 1., 9), "OK");
        let mem = Status::Count(65536);
        assert_eq!(status_text(&mem, 1., 9), "65536K OK");
        let mid = status_text(&mem, 0.75, 2);
        assert!(mid.ends_with('K') && mid != "65536K", "{mid}");
    }
}
