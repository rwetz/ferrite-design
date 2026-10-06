//! Every native Ferrite component, live and wired to state, plus the
//! gpui-component widgets they replace side by side.
//!
//!     cargo run --example components
//!     FERRITE_APPEARANCE=light cargo run --example components

// Release builds are GUI-subsystem on Windows: no console window behind the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::time::Duration;

use ferrite_design::{
    Appearance, FerriteText, Icon, Scale,
    chrome::{self, title_bar},
    components::{
        Align, Button, CommandPalette, TogglePalette, checkbox, command, context_menu, cursor, dropdown_menu, submenu, kbd, list_item, menu_item,
        meter, panel, popover, radio, rule, spinner, status_bar, switch, tabs, tag, toast, tooltip, Toast, Toaster,
    },
    icon::icon,
    motion, palette, theme,
    tokens::{hsla, space, text},
};
use gpui::{
    App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement, ParentElement, Render,
    SharedString, StatefulInteractiveElement as _, Styled, Subscription, Window, div, px, size,
};
use gpui_component::{
    Root,
    button::{Button as LibButton, ButtonVariants as _},
    switch::Switch as LibSwitch,
    tag::Tag as LibTag,
};

const FILES: [(Icon, &str, &str); 5] = [
    (Icon::Folder, "src/", "dir"),
    (Icon::File, "main.rs", "2.1 KB"),
    (Icon::File, "dither.rs", "9.8 KB"),
    (Icon::File, "tokens.rs", "7.4 KB"),
    (Icon::File, "Cargo.toml", "1.0 KB"),
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
    word_wrap: bool,
    show_hidden: bool,
    hide_merged: bool,
    only_mine: bool,
    compact: bool,
    palette: Entity<CommandPalette>,
    toaster: Entity<Toaster>,
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
            word_wrap: true,
            show_hidden: false,
            hide_merged: true,
            only_mine: false,
            compact: false,
            palette: Self::build_palette(window, cx),
            toaster: cx.new(|_| Toaster::new()),
            _appearance: theme::follow_system(window),
        }
    }

    /// Every command the app offers. Handlers reach the view through a weak
    /// handle; the palette defers them a tick, so updating anything is safe.
    fn build_palette(window: &mut Window, cx: &mut Context<Self>) -> Entity<CommandPalette> {
        let this = cx.weak_entity();
        let on = |f: fn(&mut Components, &mut Window, &mut Context<Components>)| {
            let this = this.clone();
            move |window: &mut Window, cx: &mut App| {
                let _ = this.update(cx, |v, cx| f(v, window, cx));
            }
        };
        let theme_cmd = |label: &'static str, pref: Appearance, icon: Icon| {
            command(label).group("Theme").icon(icon).keywords(["appearance", "mode", "color"]).on_run(move |window, cx| {
                theme::set_appearance(pref, window, cx);
            })
        };
        let commands = vec![
            command("New file").group("File").icon(Icon::Plus).shortcut("Ctrl+N").on_run(on(|v, _, cx| v.log("NEW FILE", cx))),
            command("Open folder").group("File").icon(Icon::Folder).shortcut("Ctrl+O").on_run(on(|v, _, cx| v.log("OPEN FOLDER", cx))),
            command("Duplicate file").group("File").icon(Icon::Copy).on_run(on(|v, _, cx| v.log("DUPLICATE", cx))),
            command("Delete file").group("File").icon(Icon::Trash).keywords(["remove", "trash"]).on_run(on(|v, _, cx| v.log("DELETE", cx))),
            command("Run task").group("Run").icon(Icon::Play).shortcut("Ctrl+R").on_run(on(|v, _, cx| {
                v.clicks += 1;
                let n = v.clicks;
                v.log(format!("RUN ×{n}"), cx);
            })),
            command("Deploy").group("Run").icon(Icon::Up).keywords(["ship", "release"]).on_run(on(|v, _, cx| v.deploy(cx))),
            command("Stop all").group("Run").icon(Icon::Stop).on_run(on(|v, _, cx| v.log("STOPPED", cx))),
            command("Toggle word wrap").group("View").icon(Icon::Menu).shortcut("Alt+Z").keywords(["soft wrap", "lines"]).on_run(on(|v, _, cx| {
                v.word_wrap = !v.word_wrap;
                let s = if v.word_wrap { "WRAP ON" } else { "WRAP OFF" };
                v.log(s, cx);
            })),
            command("Toggle hidden files").group("View").icon(Icon::File).on_run(on(|v, _, cx| {
                v.show_hidden = !v.show_hidden;
                let s = if v.show_hidden { "HIDDEN ON" } else { "HIDDEN OFF" };
                v.log(s, cx);
            })),
            command("Go to Files").group("View").icon(Icon::ChevronRight).on_run(on(|v, _, cx| { v.tab = 0; v.row = 0; v.log("TAB 1", cx); })),
            command("Go to Processes").group("View").icon(Icon::ChevronRight).on_run(on(|v, _, cx| { v.tab = 1; v.row = 0; v.log("TAB 2", cx); })),
            command("Go to Config").group("View").icon(Icon::ChevronRight).on_run(on(|v, _, cx| { v.tab = 2; v.row = 0; v.log("TAB 3", cx); })),
            theme_cmd("Theme: Iron", Appearance::Dark, Icon::Dot),
            theme_cmd("Theme: Paper", Appearance::Light, Icon::File),
            theme_cmd("Theme: Follow system", Appearance::System, Icon::Sliders),
            command("Git: Stage all").group("Git").icon(Icon::Plus).on_run(on(|v, _, cx| v.log("STAGED", cx))),
            command("Git: Commit").group("Git").icon(Icon::Check).shortcut("Ctrl+Enter").on_run(on(|v, _, cx| v.log("COMMITTED", cx))),
            command("Git: Push").group("Git").icon(Icon::Up).keywords(["upload", "publish"]).on_run(on(|v, _, cx| {
                v.log("PUSHED", cx);
                v.notify(toast("Pushed to origin/main").success().message("3 commits · 1ebd035..a41f9c2"), cx);
            })),
            command("Git: Pull").group("Git").icon(Icon::Down).keywords(["fetch", "sync"]).on_run(on(|v, _, cx| {
                v.log("PULL FAILED", cx);
                v.notify(toast("Pull failed").danger().message("merge conflict in src/tokens.rs"), cx);
            })),
            command("Search docs").group("Help").icon(Icon::Search).on_run(on(|v, _, cx| v.log("DOCS", cx))),
            command("Report an issue").group("Help").icon(Icon::Warning).keywords(["bug"]).on_run(on(|v, _, cx| v.log("REPORT", cx))),
        ];
        cx.new(|cx| {
            let mut palette = CommandPalette::new(window, cx);
            palette.set_commands(commands, cx);
            palette
        })
    }

    fn notify(&mut self, toast: Toast, cx: &mut Context<Self>) {
        self.toaster.update(cx, |t, cx| t.push(toast, cx));
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
                let undo = cx.weak_entity();
                this.notify(
                    toast("Deployed").success().message("staging · build 4412 · 2.0s").action("Undo", move |_, cx| {
                        let _ = undo.update(cx, |v, cx| v.log("ROLLED BACK", cx));
                    }),
                    cx,
                );
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
                            Button::new("run").label("Run").icon(Icon::Play).primary().shortcut("Ctrl+R").tooltip("Run the task")
                                .on_click(cx.listener(|this, _, _, cx| { this.clicks += 1; let n = this.clicks; this.log(format!("RUN ×{n}"), cx); })),
                        )
                        .child(Button::new("step").label("Step").glyph("»").on_click(cx.listener(|this, _, _, cx| this.log("STEP", cx))))
                        .child(Button::new("cancel").label("Cancel").ghost().on_click(cx.listener(|this, _, _, cx| this.log("CANCEL", cx))))
                        .child(Button::new("kill").label("Kill").icon(Icon::Close).danger().tooltip("Terminate the process").on_click(cx.listener(|this, _, _, cx| this.log("KILL", cx)))),
                )
                .child(section("states", window, cx))
                .child(
                    row()
                        .child(Button::new("deploy").label("Deploy").icon(Icon::Up).primary().loading(self.deploying)
                            .on_click(cx.listener(|this, _, _, cx| this.deploy(cx))))
                        .child(Button::new("pin").label(if self.pinned { "Pinned" } else { "Pin" }).icon(Icon::Stop).selected(self.pinned)
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
                        .child(Button::new("s4").icon(Icon::Refresh).small().tooltip("Reload"))
                        .child(Button::new("s5").icon(Icon::Plus).small().tooltip("New"))
                        .child(Button::new("s7").icon(Icon::Search).small().ghost().tooltip("Search"))
                        .child(Button::new("s8").icon(Icon::Trash).small().danger().tooltip("Delete"))
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
                list_item(("file", i), *name).icon(*g).meta(*meta).selected(self.row == i)
                    .on_click(cx.listener(move |this, _, _, cx| { this.row = i; this.log(format!("OPEN {}", FILES[i].1.to_uppercase()), cx); }))
                    .into_any_element()
            }).collect::<Vec<_>>(),
            1 => PROCS.iter().enumerate().map(|(i, (name, meta))| {
                list_item(("proc", i), *name).icon(Icon::Dot).meta(*meta).selected(self.row == i)
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

        // ── Overlays ─────────────────────────────────────────────────────
        // Menu handlers are plain `Fn(&mut Window, &mut App)`; reach the view
        // through its entity.
        let this = cx.entity();
        let act = |what: &'static str| {
            let this = this.clone();
            move |_: &mut Window, cx: &mut App| this.update(cx, |v, cx| v.log(what, cx))
        };
        let toggle = |f: fn(&mut Components) -> &mut bool, what: &'static str| {
            let this = this.clone();
            move |_: &mut Window, cx: &mut App| {
                this.update(cx, |v, cx| {
                    let flag = f(v);
                    *flag = !*flag;
                    let state = if *flag { "ON" } else { "OFF" };
                    v.log(format!("{what} {state}"), cx);
                })
            }
        };
        let file_menu = dropdown_menu("file-menu")
            .trigger(Button::new("file-btn").label("File").icon(Icon::ChevronDown))
            .item(menu_item("New").icon(Icon::Plus).shortcut("Ctrl+N").on_select(act("NEW")))
            .item(menu_item("Open folder").icon(Icon::Folder).shortcut("Ctrl+O").on_select(act("OPEN")))
            .submenu(
                submenu("Open recent")
                    .icon(Icon::File)
                    .item(menu_item("ferrite-design/").icon(Icon::Folder).on_select(act("OPEN FERRITE")))
                    .item(menu_item("nexis-design/").icon(Icon::Folder).on_select(act("OPEN NEXIS")))
                    .item(menu_item("logscope/").icon(Icon::Folder).on_select(act("OPEN LOGSCOPE")))
                    .separator()
                    .item(menu_item("Clear recent").on_select(act("CLEARED RECENT"))),
            )
            .item(menu_item("Duplicate").icon(Icon::Copy).shortcut("Ctrl+D").on_select(act("DUPLICATE")))
            .submenu(
                submenu("Export")
                    .icon(Icon::Up)
                    .label("Image")
                    .item(menu_item("PNG").on_select(act("EXPORT PNG")))
                    .submenu(
                        submenu("Dithered")
                            .item(menu_item("1-bit · Bayer 4x4").on_select(act("EXPORT 1-BIT")))
                            .item(menu_item("2-bit · Bayer 4x4").on_select(act("EXPORT 2-BIT"))),
                    )
                    .separator()
                    .item(menu_item("PDF").disabled(true)),
            )
            .separator()
            .label("View")
            .item(menu_item("Word wrap").checked(self.word_wrap).shortcut("Alt+Z").on_select(toggle(|v| &mut v.word_wrap, "WRAP")))
            .item(menu_item("Hidden files").checked(self.show_hidden).on_select(toggle(|v| &mut v.show_hidden, "HIDDEN")))
            .separator()
            .item(menu_item("Delete").icon(Icon::Trash).shortcut("Del").danger().on_select(act("DELETE")));
        let more_menu = dropdown_menu("more-menu")
            .align(Align::End)
            .width(px(200.))
            .trigger(Button::new("more-btn").icon(Icon::More).ghost().tooltip("More"))
            .item(menu_item("Refresh").icon(Icon::Refresh).shortcut("F5").on_select(act("REFRESH")))
            .item(menu_item("Settings").icon(Icon::Sliders).on_select(act("SETTINGS")))
            .item(menu_item("Report issue").icon(Icon::Warning).on_select(act("REPORT")));
        let (hide_merged, only_mine, compact) = (self.hide_merged, self.only_mine, self.compact);
        let filters = popover("filters")
            .title("Filters")
            .trigger(Button::new("filters-btn").label("Filters").icon(Icon::Sliders))
            .content({
                let this = this.clone();
                move |_, _| {
                    let set = |f: fn(&mut Components) -> &mut bool, what: &'static str| {
                        let this = this.clone();
                        move |v: &bool, _: &mut Window, cx: &mut App| {
                            let v = *v;
                            this.update(cx, |c, cx| {
                                *f(c) = v;
                                c.log(format!("{what} {}", if v { "ON" } else { "OFF" }), cx);
                            })
                        }
                    };
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(checkbox("f-merged").label("Hide merged").checked(hide_merged).on_change(set(|c| &mut c.hide_merged, "HIDE MERGED")))
                        .child(checkbox("f-mine").label("Only mine").checked(only_mine).on_change(set(|c| &mut c.only_mine, "ONLY MINE")))
                        .child(switch("f-compact").label("Compact rows").checked(compact).on_change(set(|c| &mut c.compact, "COMPACT")))
                        .into_any_element()
                }
            });
        let ctx_area = context_menu("ctx")
            .item(menu_item("Copy path").icon(Icon::Copy).shortcut("Ctrl+Shift+C").on_select(act("COPY PATH")))
            .item(menu_item("Reveal in folder").icon(Icon::Folder).on_select(act("REVEAL")))
            .item(menu_item("Search here").icon(Icon::Search).on_select(act("SEARCH HERE")))
            .submenu(
                submenu("Open with")
                    .icon(Icon::ChevronRight)
                    .item(menu_item("Editor").on_select(act("OPEN IN EDITOR")))
                    .item(menu_item("Terminal").on_select(act("OPEN IN TERMINAL")))
                    .item(menu_item("Hex view").on_select(act("OPEN HEX"))),
            )
            .separator()
            .item(menu_item("Move to trash").icon(Icon::Trash).danger().on_select(act("TRASH")))
            .child(
                div()
                    .h(px(96.))
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_1()
                    .border_dashed()
                    .border_color(hsla(p.line_strong))
                    .display(Scale::X1, window)
                    .text_color(hsla(p.fg_dim))
                    .child("RIGHT-CLICK HERE"),
            );
        let overlays = panel("Overlays").meta("popover · menu · palette · toast").child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(section("dropdown + popover", window, cx))
                .child(row().child(file_menu).child(filters).child(div().flex_1()).child(more_menu))
                .child(section("command palette", window, cx))
                .child(row().child(
                    Button::new("palette-btn").label("Commands").icon(Icon::Search).shortcut("Ctrl+Shift+P").on_click({
                        let palette = self.palette.clone();
                        move |_, window, cx| palette.update(cx, |p, cx| p.open(window, cx))
                    }),
                ))
                .child(section("toasts", window, cx))
                .child({
                    let this = cx.weak_entity();
                    let push = move |id: &'static str, label: &'static str, t: fn() -> Toast| {
                        let this = this.clone();
                        Button::new(id).label(label).small().secondary().on_click(move |_, _, cx| {
                            let _ = this.update(cx, |v, cx| v.notify(t(), cx));
                        })
                    };
                    row()
                        .child(push("toast-info", "Info", || toast("Indexing workspace").message("1,204 files · rust-analyzer")))
                        .child(push("toast-ok", "Success", || toast("Saved").success().message("tokens.rs · 7.4 KB")))
                        .child(push("toast-warn", "Warning", || toast("Disk almost full").warning().message("92% of C: used")))
                        .child(push("toast-err", "Error", || {
                            toast("Build failed").danger().message("error[E0308]: mismatched types").action("Retry", |_, _| {})
                        }))
                        .child(push("toast-sticky", "Sticky", || {
                            toast("Update ready").message("Ferrite 0.2 · restart to apply").sticky().action("Restart", |_, _| {})
                        }))
                })
                .child(section("context menu", window, cx))
                .child(ctx_area),
        );

        // ── Icons ────────────────────────────────────────────────────────
        let icons = panel("Icons").meta("16×16 · on the type grid · never fall back").child(
            div().flex().flex_row().flex_wrap().gap_3().children(Icon::ALL.iter().map(|&i| {
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    .w(px(112.))
                    .child(icon(i).scale(Scale::X2).color(hsla(p.accent)))
                    .child(div().body(text::XS).text_color(hsla(p.fg_dim)).child(i.name()))
            })),
        );

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(hsla(p.bg))
            .text_color(hsla(p.fg))
            .body(text::BASE)
            .on_action(cx.listener(|this, _: &TogglePalette, window, cx| {
                this.palette.update(cx, |p, cx| p.toggle(window, cx));
            }))
            .child(self.palette.clone())
            .child(self.toaster.clone())
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
                            .child(div().flex_1().child(versus)))
                        .child(div().flex().flex_row().gap(space::ROW)
                            .child(div().flex_1().child(overlays))
                            .child(div().flex_1().child(icons))),
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
        cx.bind_keys([gpui::KeyBinding::new("ctrl-shift-p", TogglePalette, None)]);
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
