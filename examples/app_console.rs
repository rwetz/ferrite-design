//! Template: a console — a log viewer, a REPL, a job runner, a serial
//! monitor.
//!
//! A live, virtualised log (only visible rows render, so it stays fast at
//! any length) with a level filter and follow-tail, a command prompt that
//! echoes and answers, a spinner while a command runs, and per-level
//! counts in the status bar.
//!
//!     cargo run --example app_console
//!
//! Replace the timer in `Console::new` with your process's output stream,
//! and `run()` with your command handler.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::time::Duration;

use ferrite_design::prelude::*;
use gpui::{
    App, AppContext as _, Context, Entity, Focusable as _, IntoElement, Render, ScrollStrategy, SharedString, Subscription,
    UniformListScrollHandle, Window, div, px, size,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Level {
    Info,
    Warn,
    Error,
    Command,
}

impl Level {
    fn label(self) -> &'static str {
        match self {
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERR ",
            Level::Command => "  > ",
        }
    }
}

#[derive(Clone)]
struct Line {
    at: u64,
    level: Level,
    text: SharedString,
}

const FILTERS: [&str; 4] = ["All", "Warn+", "Errors", "Commands"];

struct Console {
    lines: Vec<Line>,
    filter: usize,
    follow: bool,
    running: bool,
    clock: u64,
    prompt: Entity<TextInput>,
    scroll: UniformListScrollHandle,
    _subs: Vec<Subscription>,
}

impl Console {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let prompt = cx.new(|cx| TextInput::new(window, cx).placeholder("help, status, build, clear").prompt(">").bordered(false));
        let sub = cx.subscribe_in(&prompt, window, |this: &mut Self, input, ev: &InputEvent, window, cx| {
            if let InputEvent::Submit = ev {
                let cmd = input.read(cx).value().trim().to_string();
                input.update(cx, |i, cx| i.set_value("", cx));
                if !cmd.is_empty() {
                    this.run(cmd, window, cx);
                }
            }
        });
        // The stand-in output stream: a line every 400ms.
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_millis(400)).await;
                if this.update(cx, |this, cx| this.tick(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        Self {
            lines: Vec::new(),
            filter: 0,
            follow: true,
            running: false,
            clock: 0,
            prompt,
            scroll: UniformListScrollHandle::new(),
            _subs: vec![sub, theme::follow_system(window)],
        }
    }

    fn push(&mut self, level: Level, text: impl Into<SharedString>) {
        self.lines.push(Line { at: self.clock, level, text: text.into() });
    }

    fn tick(&mut self, cx: &mut Context<Self>) {
        self.clock += 1;
        let h = self.clock.wrapping_mul(2_654_435_761) % 100;
        let sys = ["net", "disk", "sched", "cache", "auth"][(h % 5) as usize];
        let (level, text) = match h {
            0..=3 => (Level::Error, format!("{sys}: request timed out after 30s")),
            4..=14 => (Level::Warn, format!("{sys}: queue depth {} above soft limit", 40 + h)),
            _ => (Level::Info, format!("{sys}: ok · {} ops · {}.{:02}ms", h * 13, h % 9, h % 97)),
        };
        self.push(level, text);
        cx.notify();
    }

    /// Your command handler. Long work: set `running`, spawn, push results.
    fn run(&mut self, cmd: String, _window: &mut Window, cx: &mut Context<Self>) {
        self.push(Level::Command, cmd.clone());
        match cmd.as_str() {
            "clear" => self.lines.clear(),
            "help" => self.push(Level::Info, "commands: help · status · build · clear"),
            "status" => {
                let errors = self.lines.iter().filter(|l| l.level == Level::Error).count();
                self.push(Level::Info, format!("{} lines · {errors} errors · following: {}", self.lines.len(), self.follow));
            }
            "build" => {
                self.running = true;
                cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(Duration::from_secs(2)).await;
                    let _ = this.update(cx, |this, cx| {
                        this.running = false;
                        this.push(Level::Info, "build: finished in 2.0s");
                        cx.notify();
                    });
                })
                .detach();
            }
            other => self.push(Level::Warn, format!("unknown command: {other}")),
        }
        self.follow = true;
        cx.notify();
    }

    fn visible(&self) -> Vec<Line> {
        self.lines
            .iter()
            .filter(|l| match self.filter {
                1 => matches!(l.level, Level::Warn | Level::Error),
                2 => l.level == Level::Error,
                3 => l.level == Level::Command,
                _ => true,
            })
            .cloned()
            .collect()
    }
}

impl Render for Console {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let rows = self.visible();
        let count = rows.len();
        if self.follow && count > 0 {
            self.scroll.scroll_to_item(count - 1, ScrollStrategy::Bottom);
        }
        let (warns, errors) = (
            self.lines.iter().filter(|l| l.level == Level::Warn).count(),
            self.lines.iter().filter(|l| l.level == Level::Error).count(),
        );

        let bar = toolbar()
            .child(FILTERS.iter().fold(segmented("filter"), |s, f| s.option(*f)).selected(self.filter).on_select(cx.listener(|this, i: &usize, _, cx| {
                this.filter = *i;
                cx.notify();
            })))
            .separator()
            .child(switch("follow").label("Follow").checked(self.follow).on_change(cx.listener(|this, v: &bool, _, cx| {
                this.follow = *v;
                cx.notify();
            })))
            .spacer()
            .child(Button::new("clear").label("Clear").icon(Icon::Trash).small().ghost().on_click(cx.listener(|this, _, _, cx| {
                this.lines.clear();
                cx.notify();
            })));

        let log = if count == 0 {
            div().size_full().child(empty_state("Quiet", "Nothing matches this filter yet.", window, cx)).into_any_element()
        } else {
            virtual_list("log", count, move |range, _, cx| {
                let p = palette(cx);
                range
                    .map(|i| {
                        let line = &rows[i];
                        let ink = match line.level {
                            Level::Error => p.danger,
                            Level::Warn => p.warning,
                            Level::Command => p.accent_text,
                            Level::Info => p.fg_dim,
                        };
                        div()
                            .h(px(20.))
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_3()
                            .px_3()
                            .body(text::SM)
                            .when(line.level == Level::Command, |el| el.bg(hsla(p.surface)))
                            .child(div().w(px(64.)).flex_none().text_color(hsla(p.fg_faint)).child(format!("{:>6.1}s", line.at as f32 * 0.4)))
                            .child(div().w(px(36.)).flex_none().text_color(hsla(ink)).child(line.level.label()))
                            .child(div().flex_1().min_w_0().whitespace_nowrap().text_color(hsla(if line.level == Level::Info { p.fg } else { ink })).child(line.text.clone()))
                            .into_any_element()
                    })
                    .collect()
            })
            .track_scroll(&self.scroll)
            .size_full()
            .into_any_element()
        };

        let prompt = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .h(px(36.))
            .px_2()
            .bg(hsla(p.sunken))
            .border_t_1()
            .border_color(hsla(p.line_strong))
            .child(div().flex_1().child(self.prompt.clone()))
            .when(self.running, |el| el.child(spinner("busy").color(hsla(p.accent))).child(div().body(text::SM).text_color(hsla(p.fg_dim)).child("running")));

        window_frame().child(power_on_in(
            "power",
            boot_screen("boot", div()
                .flex()
                .flex_col()
                .size_full()
                .bg(hsla(p.bg))
                .text_color(hsla(p.fg))
                .body(text::BASE)
                .child(title_bar("Console"))
                .child(bar)
                .child(div().flex_1().min_h_0().bg(hsla(p.sunken)).child(log))
                .child(prompt)
                .child(
                    status_bar()
                        .left(if self.follow { "FOLLOWING" } else { "PAUSED" })
                        // Counters tick constantly: live segments, no decrypt.
                        .left_live(format!("{} LINES", self.lines.len()))
                        .right_live(format!("{warns} WARN"))
                        .right_live(format!("{errors} ERR")),
                ),
            )
            .title("Console"),
        ))
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        ferrite_design::init(Appearance::Dark, cx);
        // FERRITE_SCHEME / FERRITE_APPEARANCE / FERRITE_FPS, for trying other looks.
        theme::apply_env(cx);
        let options = chrome::window_options("Console", size(px(1000.), px(680.)), cx);
        cx.open_window(options, |window, cx| {
            chrome::square_corners(window);
            chrome::power_off_on_close(window, cx);
            let view = cx.new(|cx| Console::new(window, cx));
            // Typing goes straight to the prompt.
            let prompt = view.read(cx).prompt.clone();
            let handle = prompt.read(cx).focus_handle(cx);
            handle.focus(window, cx);
            view
        })
        .expect("failed to open the window");
        cx.activate(true);
    });
}
