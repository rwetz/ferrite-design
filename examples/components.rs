//! Every native Ferrite component, live and wired to state, plus the
//! gpui-component widgets they replace side by side.
//!
//!     cargo run --example components
//!     FERRITE_APPEARANCE=light cargo run --example components

// Release builds are GUI-subsystem on Windows: no console window behind the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::time::Duration;

use ferrite_design::{
    Appearance, FerriteText, Scale,
    chrome::{self, title_bar},
    components::{
        Button, checkbox, cursor, kbd, list_item, meter, panel, radio, rule, spinner, status_bar,
        switch, tabs, tag, tooltip,
    },
    motion, palette, theme,
    tokens::{hsla, space, text},
};
use gpui::{
    App, AppContext as _, Context, InteractiveElement as _, IntoElement, ParentElement, Render,
    SharedString, StatefulInteractiveElement as _, Styled, Subscription, Window, div, px, size,
};
use gpui_component::{
    Root,
    button::{Button as LibButton, ButtonVariants as _},
    switch::Switch as LibSwitch,
    tag::Tag as LibTag,
};

const FILES: [(&str, &str, &str); 5] = [
    ("▸", "src/", "dir"),
    ("·", "main.rs", "2.1 KB"),
    ("·", "dither.rs", "9.8 KB"),
    ("·", "tokens.rs", "7.4 KB"),
    ("·", "Cargo.toml", "1.0 KB"),
];
const PROCS: [(&str, &str); 4] = [("ferrite-atlas", "pid 4412"), ("cargo", "pid 9021"), ("rust-analyzer", "pid 3310"), ("showcase", "pid 7777")];
const MODES: [&str; 3] = ["Fast", "Balanced", "Thorough"];

struct Components {
    clicks: u32,
    last: SharedString,
    deploying: bool,
    pinned: bool,
    checks: [bool; 3],
    mode: usize,
    telemetry: bool,
    tab: usize,
    row: usize,
    tick: u64,
    lib_switch: bool,
    _appearance: Subscription,
}

impl Components {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Meters update twice a second — the rate the data would.
        if !motion::reduced(cx) {
            cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor().timer(Duration::from_millis(500)).await;
                    if this.update(cx, |this, cx| { this.tick += 1; cx.notify(); }).is_err() {
                        break;
                    }
                }
            })
            .detach();
        }
        Self {
            clicks: 0,
            last: "READY".into(),
            deploying: false,
            pinned: false,
            checks: [true, false, true],
            mode: 1,
            telemetry: true,
            tab: 0,
            row: 2,
            tick: 0,
            lib_switch: false,
            _appearance: theme::follow_system(window),
        }
    }

    fn log(&mut self, what: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.last = what.into();
        cx.notify();
    }

    /// Deterministic wobbling load figures for the meters.
    fn load(&self, seed: u64) -> f32 {
        let t = self.tick as f32 * 0.6 + seed as f32 * 1.7;
        (0.5 + 0.35 * t.sin() + 0.12 * (t * 2.3).cos()).clamp(0.02, 1.0)
    }

    fn deploy(&mut self, cx: &mut Context<Self>) {
        self.deploying = true;
        self.log("DEPLOYING…", cx);
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(2)).await;
            let _ = this.update(cx, |this, cx| {
                this.deploying = false;
                this.log("DEPLOYED", cx);
            });
        })
        .detach();
    }
}

fn section(label: &'static str, window: &Window, cx: &App) -> impl IntoElement {
    rule(Some(label), window, cx)
}

fn row() -> gpui::Div {
    div().flex().flex_row().flex_wrap().items_center().gap_2()
}

impl Render for Components {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let is_dark = p.is_dark();
        let all = self.checks.iter().all(|c| *c);
        let any = self.checks.iter().any(|c| *c);

        // ── Buttons ──────────────────────────────────────────────────────
        let buttons = panel("Button").meta("native · replaces gpui-component #2").child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(section("variants", window, cx))
                .child(
                    row()
                        .child(
                            Button::new("run").label("Run").glyph("▶").primary().shortcut("Ctrl+R").tooltip("Run the task")
                                .on_click(cx.listener(|this, _, _, cx| { this.clicks += 1; let n = this.clicks; this.log(format!("RUN ×{n}"), cx); })),
                        )
                        .child(Button::new("step").label("Step").glyph("»").on_click(cx.listener(|this, _, _, cx| this.log("STEP", cx))))
                        .child(Button::new("cancel").label("Cancel").ghost().on_click(cx.listener(|this, _, _, cx| this.log("CANCEL", cx))))
                        .child(Button::new("kill").label("Kill").glyph("×").danger().tooltip("Terminate the process").on_click(cx.listener(|this, _, _, cx| this.log("KILL", cx)))),
                )
                .child(section("states", window, cx))
                .child(
                    row()
                        .child(Button::new("deploy").label("Deploy").glyph("↑").primary().loading(self.deploying)
                            .on_click(cx.listener(|this, _, _, cx| this.deploy(cx))))
                        .child(Button::new("pin").label(if self.pinned { "Pinned" } else { "Pin" }).glyph("■").selected(self.pinned)
                            .on_click(cx.listener(|this, _, _, cx| { this.pinned = !this.pinned; let s = if this.pinned { "PINNED" } else { "UNPINNED" }; this.log(s, cx); })))
                        .child(Button::new("d1").label("Primary").primary().disabled(true))
                        .child(Button::new("d2").label("Secondary").disabled(true))
                        .child(Button::new("d3").label("Ghost").ghost().disabled(true))
                        .child(Button::new("d4").label("Danger").danger().disabled(true)),
                )
                .child(section("small", window, cx))
                .child(
                    row()
                        .child(Button::new("s1").label("Save").primary().small())
                        .child(Button::new("s2").label("Diff").small())
                        .child(Button::new("s3").label("Undo").ghost().small())
                        .child(Button::new("s4").glyph("↻").small().tooltip("Reload"))
                        .child(Button::new("s5").glyph("+").small().tooltip("New"))
                        .child(Button::new("s6").label("Purge").danger().small()),
                ),
        );

        // ── Toggles ──────────────────────────────────────────────────────
        let toggles = panel("Toggles").meta("checkbox · radio · switch").flex_1().child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(section("checkbox", window, cx))
                .child(
                    checkbox("all").label("All targets").checked(all).indeterminate(any && !all)
                        .on_change(cx.listener(|this, v: &bool, _, cx| { this.checks = [*v; 3]; this.log("TARGETS: ALL", cx); })),
                )
                .children(["x86_64-pc-windows-msvc", "aarch64-apple-darwin", "x86_64-unknown-linux-gnu"].into_iter().enumerate().map(|(i, name)| {
                    div().pl_4().child(
                        checkbox(("target", i)).label(name).checked(self.checks[i])
                            .on_change(cx.listener(move |this, v: &bool, _, cx| { this.checks[i] = *v; this.log(format!("TARGET {i}: {}", if *v { "ON" } else { "OFF" }), cx); })),
                    )
                }))
                .child(checkbox("locked").label("Locked option").checked(true).disabled(true))
                .child(section("radio", window, cx))
                .children(MODES.into_iter().enumerate().map(|(i, m)| {
                    radio(("mode", i)).label(m).checked(self.mode == i)
                        .on_change(cx.listener(move |this, _: &bool, _, cx| { this.mode = i; this.log(format!("MODE: {}", MODES[i].to_uppercase()), cx); }))
                }))
                .child(section("switch", window, cx))
                .child(switch("telemetry").label("Telemetry").checked(self.telemetry)
                    .on_change(cx.listener(|this, v: &bool, _, cx| { this.telemetry = *v; this.log(if *v { "TELEMETRY ON" } else { "TELEMETRY OFF" }, cx); })))
                .child(switch("paper").label("Paper mode").checked(!is_dark)
                    .on_change(cx.listener(|_, v: &bool, window, cx| {
                        theme::set_appearance(if *v { Appearance::Light } else { Appearance::Dark }, window, cx);
                    })))
                .child(switch("sealed").label("Sealed (disabled)").checked(true).disabled(true)),
        );

        // ── Tags, keys, meters ───────────────────────────────────────────
        let status = panel("Status").meta("tag · kbd · meter · spinner").flex_1().child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(section("tag", window, cx))
                .child(row().child(tag("live").accent()).child(tag("idle")).child(tag("fault").danger()).child(tag("ok").success()).child(tag("degraded").warning()))
                .child(row().child(tag("live").accent().outline()).child(tag("idle").outline()).child(tag("fault").danger().outline()).child(tag("ok").success().outline()).child(tag("degraded").warning().outline()))
                .child(section("kbd", window, cx))
                .child(row().child(kbd("Ctrl+Shift+P")).child(div().body(text::SM).text_color(hsla(p.fg_dim)).child("command palette")))
                .child(row().child(kbd("Alt+F4")).child(div().body(text::SM).text_color(hsla(p.fg_dim)).child("quit")))
                .child(section("meter", window, cx))
                .child(meter(self.load(1)).label("cpu"))
                .child(meter(self.load(2) * 0.8).label("mem"))
                .child(meter(self.load(3) * 0.55).label("disk"))
                .child(meter(0.97).label("swap"))
                .child(section("live", window, cx))
                .child(
                    row()
                        .child(spinner("sp1").color(hsla(p.accent)))
                        .child(div().body(text::SM).child("indexing"))
                        .child(div().w(px(16.)))
                        .child(cursor("cur1"))
                        .child(div().body(text::SM).text_color(hsla(p.fg_dim)).child("awaiting input")),
                ),
        );

        // ── Tabs + list ──────────────────────────────────────────────────
        let tab = self.tab;
        let list_body = div().flex().flex_col().py_1().children(match tab {
            0 => FILES.iter().enumerate().map(|(i, (g, name, meta))| {
                list_item(("file", i), *name).glyph(*g).meta(*meta).selected(self.row == i)
                    .on_click(cx.listener(move |this, _, _, cx| { this.row = i; this.log(format!("OPEN {}", FILES[i].1.to_uppercase()), cx); }))
                    .into_any_element()
            }).collect::<Vec<_>>(),
            1 => PROCS.iter().enumerate().map(|(i, (name, meta))| {
                list_item(("proc", i), *name).glyph("●").meta(*meta).selected(self.row == i)
                    .on_click(cx.listener(move |this, _, _, cx| { this.row = i; this.log(format!("SELECT {}", PROCS[i].0.to_uppercase()), cx); }))
                    .into_any_element()
            }).collect(),
            _ => vec![
                list_item("cfg0", "theme = \"iron\"").glyph("=").meta("user").into_any_element(),
                list_item("cfg1", "dither.cell = 2").glyph("=").meta("default").into_any_element(),
                list_item("cfg2", "telemetry = false").glyph("=").meta("locked").disabled(true).into_any_element(),
            ],
        });
        let browser = div()
            .flex()
            .flex_col()
            .border_1()
            .border_color(hsla(p.line))
            .child(
                tabs("tabs")
                    .tab_with_meta("Files", "5")
                    .tab_with_meta("Processes", "4")
                    .tab("Config")
                    .selected(tab)
                    .on_select(cx.listener(|this, i: &usize, _, cx| { this.tab = *i; this.row = 0; this.log(format!("TAB {}", i + 1), cx); })),
            )
            .child(list_body);
        let navigation = panel("Tabs + List").meta("native · replaces #6").child(browser);

        // ── Versus ───────────────────────────────────────────────────────
        let versus = panel("Native vs gpui-component").meta("same theme, different geometry").child(
            div()
                .flex()
                .flex_row()
                .gap(space::X3)
                .child(
                    div().flex().flex_col().gap_2()
                        .child(div().display(Scale::X1, window).text_color(hsla(p.accent_text)).child("FERRITE"))
                        .child(row().child(Button::new("v1").label("Run").primary()).child(Button::new("v2").label("Step")))
                        .child(switch("v3").label("Switch").checked(self.lib_switch).on_change(cx.listener(|this, v: &bool, _, cx| { this.lib_switch = *v; cx.notify(); })))
                        .child(row().child(tag("live").accent()).child(tag("fault").danger())),
                )
                .child(
                    div().flex().flex_col().gap_2()
                        .child(div().display(Scale::X1, window).text_color(hsla(p.fg_dim)).child("GPUI-COMPONENT"))
                        .child(row().child(LibButton::new("l1").label("Run").primary()).child(LibButton::new("l2").label("Step")))
                        .child(LibSwitch::new("l3").label("Switch").checked(self.lib_switch).on_click(cx.listener(|this, v: &bool, _, cx| { this.lib_switch = *v; cx.notify(); })))
                        .child(row().child(LibTag::primary().child("LIVE")).child(LibTag::danger().child("FAULT"))),
                ),
        );

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(hsla(p.bg))
            .text_color(hsla(p.fg))
            .body(text::BASE)
            .child(title_bar("Ferrite Components"))
            .child(
                div().id("scroll").flex_1().min_h_0().overflow_y_scroll().child(
                    div()
                        .flex()
                        .flex_col()
                        .p(space::ROW)
                        .gap(space::ROW)
                        .child(buttons)
                        .child(div().flex().flex_row().gap(space::ROW).child(toggles).child(status))
                        .child(div().flex().flex_row().gap(space::ROW)
                            .child(div().flex_1().child(navigation))
                            .child(div().flex_1().child(versus))),
                ),
            )
            .child(
                div()
                    .id("status-tip")
                    .tooltip(tooltip("Last event").builder())
                    .child(
                        status_bar()
                            .left(self.last.clone())
                            .left(if is_dark { "IRON" } else { "PAPER" })
                            .left(format!("MODE {}", MODES[self.mode].to_uppercase()))
                            .right(format!("CLICKS {}", self.clicks))
                            .right(format!("SCALE {:.2}x", window.scale_factor())),
                    ),
            )
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let appearance = match std::env::var("FERRITE_APPEARANCE").as_deref() {
            Ok("light") => Appearance::Light,
            Ok("system") => Appearance::System,
            _ => Appearance::Dark,
        };
        ferrite_design::init(appearance, cx);
        let options = chrome::window_options("Ferrite Components", size(px(1180.), px(900.)), cx);
        cx.open_window(options, |window, cx| {
            chrome::square_corners(window);
            let view = cx.new(|cx| Components::new(window, cx));
            cx.new(|cx| Root::new(view, window, cx))
        })
        .expect("failed to open the components window");
        cx.activate(true);
    });
}
