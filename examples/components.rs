//! Every Ferrite component, live and wired to state, one page per family:
//! controls, forms, data & input, navigation, charts, layout, motion.
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
        InputEvent, SortDir, TextInput, Date, Presence, accordion, accordion_section, alert, avatar, bar_chart, breadcrumb,
        calendar, cascade_in, date_picker, drawer, event, field, flash, heatmap, line_chart, number_input, pagination,
        property_list, scan, select, sidebar, skeleton, skeleton_text, sparkline, stat, steps, timeline, toolbar, wipe_in, afterglow, column, count_up, decrypt, develop, dissolve, interlace_in, ping, power_on_in, shake, tear, typewriter, unroll_in, dialog, scroll_area, segmented, slider, split, table, tree, tree_node, virtual_list,
    },
    dither::{self, dither},
    icon::icon,
    motion, palette, theme,
    tokens::{hsla, space, text},
    components::tag::Tone,
};
use gpui::{
    App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement, ParentElement, Render,
    SharedString, StatefulInteractiveElement as _, Styled, Subscription, Window, div, prelude::FluentBuilder as _, px, size,
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
const PAGES: [&str; 7] = ["Controls", "Forms", "Data & input", "Navigation", "Charts", "Layout", "Motion"];
const REGIONS: [&str; 4] = ["eu-west-1", "us-east-1", "ap-south-1", "sa-east-1"];
const LOG_LINES: usize = 10_000;
const PROC_TABLE: [(&str, u32, f32, u32, &str); 6] = [
    ("ferrite-atlas", 4412, 31.0, 412, "run"),
    ("cargo", 9021, 12.4, 188, "run"),
    ("rust-analyzer", 3310, 4.2, 1630, "idle"),
    ("showcase", 7777, 0.8, 96, "idle"),
    ("dither-bake", 5120, 57.3, 64, "run"),
    ("sshd", 612, 0.0, 12, "sleep"),
];

#[derive(Clone, Copy)]
struct Proc {
    name: &'static str,
    pid: u32,
    cpu: f32,
    mem: u32,
    state: &'static str,
}

#[derive(Clone, Copy, PartialEq)]
enum Modal {
    Delete,
    Rename,
    Motion,
}

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
    inputs: Vec<Entity<TextInput>>,
    name_errors: u32,
    replay: [u32; 24],
    // Forms
    region: Option<usize>,
    workers: f64,
    timeout: f64,
    launch: Option<Date>,
    handle_error: Option<SharedString>,
    // Navigation
    nav: SharedString,
    rail: bool,
    pager: usize,
    wizard: usize,
    drawer: bool,
    alerts: [bool; 3],
    // Charts
    series_seed: u64,
    /// Built once: the develop demo's picture.
    orb: dither::Picture,
    _input_subs: Vec<Subscription>,
    word_wrap: bool,
    show_hidden: bool,
    hide_merged: bool,
    only_mine: bool,
    compact: bool,
    page: usize,
    density: usize,
    volume: f32,
    gamma: f32,
    tree_sel: Option<SharedString>,
    procs: Vec<Proc>,
    sort: Option<(usize, SortDir)>,
    proc_sel: Option<usize>,
    dialog: Option<Modal>,
    palette: Entity<CommandPalette>,
    toaster: Entity<Toaster>,
    _appearance: Subscription,
}

impl Components {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let inputs: Vec<Entity<TextInput>> = vec![
            cx.new(|cx| TextInput::new(window, cx).placeholder("project name")),
            cx.new(|cx| TextInput::new(window, cx).placeholder("filter files…").prompt(">")),
            cx.new(|cx| TextInput::new(window, cx).placeholder("api token").masked(true)),
            cx.new(|cx| {
                let mut t = TextInput::new(window, cx).placeholder("read-only");
                t.set_value("ferrite-design", cx);
                t.set_disabled(true, cx);
                t
            }),
        ];
        let input_subs = inputs
            .iter()
            .enumerate()
            .map(|(i, input)| {
                cx.subscribe(input, move |this: &mut Components, input, ev: &InputEvent, cx| {
                    let name = ["NAME", "FILTER", "TOKEN", "LOCKED"][i];
                    match ev {
                        InputEvent::Change => cx.notify(),
                        InputEvent::Submit if i == 0 && input.read(cx).value().trim().is_empty() => {
                            this.name_errors += 1;
                            this.notify(toast("Name can't be empty").warning(), cx);
                            this.log("NAME REJECTED", cx);
                        }
                        InputEvent::Submit => {
                            let v = input.read(cx).value();
                            let shown = if i == 2 { "•".repeat(v.chars().count()) } else { v.to_string() };
                            this.log(format!("{name} = {shown}"), cx);
                        }
                    }
                })
            })
            .collect();
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
            inputs,
            name_errors: 0,
            replay: [0; 24],
            region: Some(0),
            workers: 8.,
            timeout: 2.5,
            launch: Some(Date::new(2026, 10, 14)),
            handle_error: None,
            nav: "overview".into(),
            rail: false,
            pager: 4,
            wizard: 1,
            drawer: false,
            alerts: [true; 3],
            series_seed: 0,
            orb: dither::Picture::from_fn(240, 160, orb_scene),
            _input_subs: input_subs,
            word_wrap: true,
            show_hidden: false,
            hide_merged: true,
            only_mine: false,
            compact: false,
            page: 0,
            density: 1,
            volume: 65.,
            gamma: 1.0,
            tree_sel: Some("src/components/menu.rs".into()),
            procs: PROC_TABLE.iter().map(|&(name, pid, cpu, mem, state)| Proc { name, pid, cpu, mem, state }).collect(),
            sort: None,
            proc_sel: Some(1),
            dialog: None,
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

impl Components {
    fn sort_procs(&mut self) {
        let Some((col, dir)) = self.sort else { return };
        let selected = self.proc_sel.map(|i| self.procs[i].pid);
        self.procs.sort_by(|a, b| {
            let o = match col {
                0 => a.name.cmp(b.name),
                1 => a.pid.cmp(&b.pid),
                2 => a.cpu.total_cmp(&b.cpu),
                3 => a.mem.cmp(&b.mem),
                _ => a.state.cmp(b.state),
            };
            if dir == SortDir::Desc { o.reverse() } else { o }
        });
        // Selection follows the row, not the index.
        self.proc_sel = selected.and_then(|pid| self.procs.iter().position(|p| p.pid == pid));
    }

    fn motion_page(&mut self, window: &mut Window, cx: &mut Context<Self>) -> gpui::Div {
        let p = palette(cx);
        let this = cx.weak_entity();
        let replay = |i: usize| {
            let this = this.clone();
            Button::new(("replay", i)).label("Replay").icon(Icon::Refresh).small().ghost().on_click(move |_, _, cx| {
                let _ = this.update(cx, |v, cx| {
                    v.replay[i] += 1;
                    if i == 21 {
                        v.notify(toast("Stepped in").message("dither + 40→14→4→0px"), cx);
                    }
                    cx.notify();
                });
            })
        };
        let tile = |i: usize, title: &'static str, note: &'static str, body: gpui::AnyElement| {
            panel(title).meta(note).flex_1().child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(div().h(px(150.)).flex().items_center().justify_center().child(body))
                    .child(div().flex().flex_row().justify_end().child(replay(i))),
            )
        };
        let r = self.replay;
        let big = |t: &'static str| div().display(Scale::X2, window).text_color(hsla(p.fg)).child(t);

        let unroll_demo = unroll_in(("m-unroll", r[0] as usize), r[0], {
            div()
                .w(px(300.))
                .flex()
                .flex_col()
                .border_1()
                .border_color(hsla(p.line_strong))
                .bg(hsla(p.raised))
                .children(["boot sequence", "mount /dev/fe0", "load palette IRON", "dither cache warm", "ready"].iter().map(|l| {
                    div().h(px(24.)).px_3().flex().items_center().body(text::SM).text_color(hsla(p.fg_dim)).child(*l)
                }))
        })
        .duration(motion::SLOW);
        let dissolve_demo = dissolve(("m-dissolve", r[1] as usize), r[1], big("FERRITE"));
        let decrypt_demo = div()
            .display(Scale::X1, window)
            .text_color(hsla(p.accent_text))
            .child(decrypt(("m-decrypt", r[2] as usize), "ACCESS GRANTED · 0x7F3A"));
        let type_demo = div()
            .body(text::LG)
            .text_color(hsla(p.fg))
            .child(typewriter(("m-type", r[3] as usize), "> cargo run --release"));
        let shake_demo = shake(
            "m-shake",
            r[4],
            div()
                .px_4()
                .py_2()
                .border_1()
                .border_color(hsla(p.danger))
                .display(Scale::X1, window)
                .text_color(hsla(p.danger))
                .child("ACCESS DENIED"),
        );
        let target = if r[5].is_multiple_of(2) { 98.6 } else { 12.5 };
        let count_demo = div()
            .display(Scale::X2, window)
            .text_color(hsla(p.accent_text))
            .child(count_up("m-count", target, |v| format!("{v:05.1}%")));

        let develop_demo = develop(("m-develop", r[6] as usize), r[6], dither(self.orb.clone()).ink(hsla(p.accent)).w(px(240.)).h(px(160.)));
        let latency = 12 + (r[7] * 37) % 180;
        let glow_demo = div()
            .display(Scale::X2, window)
            .text_color(hsla(p.fg))
            .child(afterglow("m-glow", format!("{latency:>3}MS")));
        let interlace_demo = interlace_in(("m-interlace", r[8] as usize), r[8], {
            div()
                .w(px(300.))
                .flex()
                .flex_col()
                .border_1()
                .border_color(hsla(p.line_strong))
                .bg(hsla(p.raised))
                .children(["channel 3", "signal locked", "field 1 · even", "field 2 · odd", "picture ok"].iter().map(|l| {
                    div().h(px(24.)).px_3().flex().items_center().body(text::SM).text_color(hsla(p.fg_dim)).child(*l)
                }))
        })
        .veil(hsla(p.surface));
        let tear_demo = {
            let (danger, ink, scale) = (p.danger, p.bg, fonts_x1(window));
            tear("m-tear", r[9], move || {
                div()
                    .px_4()
                    .py_2()
                    .bg(hsla(danger))
                    .text_color(hsla(ink))
                    .text_size(scale)
                    .font_family(ferrite_design::fonts::DISPLAY)
                    .child("LINK LOST · RETRYING")
            })
        };
        let ping_demo = ping("m-ping", r[10], tag(format!("{} new", r[10])).accent());
        let seek_demo = tabs("m-seek").tab("Logs").tab("Metrics").tab("Config").tab("Env").selected((r[11] % 4) as usize);
        let power_demo = power_on_in(("m-power", r[12] as usize), {
            div()
                .w(px(260.))
                .h(px(130.))
                .flex()
                .items_center()
                .justify_center()
                .bg(hsla(p.raised))
                .border_1()
                .border_color(hsla(p.line_strong))
                .display(Scale::X2, window)
                .text_color(hsla(p.accent_text))
                .child("READY.")
        });

        let lines = |items: &[&'static str]| {
            div()
                .w(px(260.))
                .flex()
                .flex_col()
                .border_1()
                .border_color(hsla(p.line_strong))
                .bg(hsla(p.raised))
                .children(items.iter().map(|l| div().h(px(24.)).px_3().flex().items_center().body(text::SM).text_color(hsla(p.fg_dim)).child(*l)))
        };
        let wipe_demo = wipe_in(("m-wipe", r[13] as usize), r[13], lines(&["inspector", "name   ferrite-atlas", "pid    4412", "state  run"])).from_right();
        let scan_demo = scan("m-scan", r[14], lines(&["query: errors last 1h", "rows: 42", "took 18ms"]));
        let flash_demo = flash("m-flash", r[15], div().px_4().py_2().border_1().border_color(hsla(p.accent)).display(Scale::X2, window).text_color(hsla(p.accent_text)).child("3 NEW"));
        let cascade_demo = cascade_in(("m-cascade", r[16] as usize), r[16])
            .w(px(260.))
            .flex()
            .flex_col()
            .gap_1()
            .children(["mount /dev/fe0", "load palette", "warm dither cache", "bind keys", "ready"].iter().map(|l| {
                div().h(px(20.)).px_2().flex().items_center().bg(hsla(p.raised)).body(text::SM).text_color(hsla(p.fg_dim)).child(*l)
            }));
        let boot_demo = div().w(px(260.)).child(panel(format!("Uplink {}", r[19] + 1)).meta("ok").child(div().body(text::SM).text_color(hsla(p.fg_dim)).child("title decrypts, rule draws on")));
        let on = r[17].is_multiple_of(2);
        let toggles_demo = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(checkbox("m-check").label("Stamped").checked(!on))
            .child(switch("m-switch").label("Travels").checked(!on))
            .child(segmented("m-seg").option("One").option("Two").option("Three").selected((r[17] % 3) as usize));
        let sweep_demo = div().w(px(240.)).flex().flex_col().children((0..3usize).map(|i| list_item(("m-sweep", i), ["alpha.rs", "beta.rs", "gamma.rs"][i]).icon(Icon::File).selected(r[18] as usize % 3 == i)));
        let modal_demo = div()
            .flex()
            .flex_col()
            .gap_2()
            .items_center()
            .child(Button::new("m-toast").label("Push a toast").small().on_click(cx.listener(|this, _, _, cx| this.notify(toast("Stepped in").message("dither + 40→14→4→0px"), cx))))
            .child(Button::new("m-dialog").label("Open a dialog").small().on_click(cx.listener(|this, _, _, cx| {
                this.dialog = Some(Modal::Motion);
                cx.notify();
            })))
            .child(
                dialog("motion-dialog")
                    .open(self.dialog == Some(Modal::Motion))
                    .title("Screen door")
                    .description("The backdrop steps up the Bayer ramp to 69% while the panel unrolls.")
                    .on_close({
                        let this = cx.weak_entity();
                        move |_, cx| {
                            let _ = this.update(cx, |v, cx| {
                                v.dialog = None;
                                cx.notify();
                            });
                        }
                    }),
            )
            .child(div().body(text::XS).text_color(hsla(p.fg_faint)).child("the replay button does nothing here"));

        let rates = motion::RATES;
        let current = rates.iter().position(|&r| r == motion::fps()).unwrap_or(1);
        let rate_picker = rates.iter().fold(segmented("fps"), |seg, r| seg.option(format!("{r}")))
            .selected(current)
            .on_select(move |i, _, cx| {
                motion::set_fps(rates[*i]);
                cx.refresh_windows();
            });

        div()
            .flex()
            .flex_col()
            .gap(space::ROW)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .child(div().display(Scale::X1, window).child("REFRESH"))
                    .child(rate_picker)
                    .child(div().display(Scale::X1, window).text_color(hsla(p.fg_dim)).child("FPS"))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .body(text::SM)
                            .text_color(hsla(p.fg_dim))
                            .child("Smooth effects take finer steps at higher rates; timed ones (shake, tear, ping, stamp) keep their beat. Costs CPU only while something plays."),
                    ),
            )
            .child(
                div()
                    .body(text::SM)
                    .text_color(hsla(p.fg_dim))
                    .child(format!("Running at {}fps, eased out: big bites first, then it settles. Drop to 25 for the classic stepped look. Every effect stays in its box and is over in a third of a second.", motion::fps())),
            )
            .child(
                div().flex().flex_row().gap(space::ROW)
                    .child(tile(0, "Unroll", "panels · menus · pages", unroll_demo.into_any_element()))
                    .child(tile(1, "Dissolve", "Bayer ramp, 16 levels", dissolve_demo.into_any_element()))
                    .child(tile(2, "Decrypt", "text locks in left → right", decrypt_demo.into_any_element())),
            )
            .child(
                div().flex().flex_row().gap(space::ROW)
                    .child(tile(3, "Typewriter", "terminal print", type_demo.into_any_element()))
                    .child(tile(4, "Shake", "rejection · 6px max", shake_demo.into_any_element()))
                    .child(tile(5, "Count", "numbers roll in steps", count_demo.into_any_element())),
            )
            .child(
                div().flex().flex_row().gap(space::ROW)
                    .child(tile(6, "Develop", "blue noise, speck by speck", develop_demo.into_any_element()))
                    .child(tile(7, "Afterglow", "old value fades like phosphor", glow_demo.into_any_element()))
                    .child(tile(8, "Interlace", "even field, then odd", interlace_demo.into_any_element())),
            )
            .child(
                div().flex().flex_row().gap(space::ROW)
                    .child(tile(9, "Tear", "system failure · 6px max", tear_demo.into_any_element()))
                    .child(tile(10, "Ping", "new item · one ring", ping_demo.into_any_element()))
                    .child(tile(11, "Seek", "the tab edge travels", seek_demo.into_any_element())),
            )
            .child(
                div().flex().flex_row().gap(space::ROW)
                    .child(tile(12, "Power-on", "launch · once per window", power_demo.into_any_element()))
                    .child(tile(13, "Wipe", "drawers · sidebars", wipe_demo.into_any_element()))
                    .child(tile(14, "Scan", "refreshed in place", scan_demo.into_any_element())),
            )
            .child(
                div().flex().flex_row().gap(space::ROW)
                    .child(tile(15, "Flash", "look here · any element", flash_demo.into_any_element()))
                    .child(tile(16, "Cascade", "items one frame apart", cascade_demo.into_any_element()))
                    .child(tile(19, "Boot", "panel headers", boot_demo.into_any_element())),
            )
            .child(rule(Some("motion inside the components"), window, cx))
            .child(
                div().flex().flex_row().gap(space::ROW)
                    .child(tile(17, "Stamp · travel · grow", "checkbox · switch · segmented", toggles_demo.into_any_element()))
                    .child(tile(18, "Sweep", "selection fills in", sweep_demo.into_any_element()))
                    .child(tile(21, "Step-in · screen door", "toasts · modals", modal_demo.into_any_element())),
            )
    }

    fn layout_page(&mut self, window: &mut Window, cx: &mut Context<Self>) -> gpui::Div {
        let p = palette(cx);
        // A deterministic fake log: level, subsystem, message.
        let line = |i: usize| -> (&'static str, u32, String) {
            let h = (i as u64).wrapping_mul(2654435761) % 1000;
            let (level, ink) = match h {
                0..=11 => ("ERR ", p.danger),
                12..=59 => ("WARN", p.warning),
                _ => ("INFO", p.fg_dim),
            };
            let sys = ["dither", "raster", "input", "palette", "chrome", "toast"][(h % 6) as usize];
            (level, ink, format!("{sys}: frame {} ok in {}.{:02}ms", i * 3, h % 9, h % 97))
        };
        let log = virtual_list("log", LOG_LINES, move |range, _, _| {
            range
                .map(|i| {
                    let (level, ink, msg) = line(i);
                    div()
                        .h(px(22.))
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_3()
                        .px_3()
                        .body(text::SM)
                        .child(div().w(px(56.)).flex_none().text_color(hsla(p.fg_faint)).child(format!("{:>5}", i + 1)))
                        .child(div().w(px(40.)).flex_none().text_color(hsla(ink)).child(level))
                        .child(div().flex_1().min_w_0().overflow_hidden().whitespace_nowrap().text_color(hsla(p.fg)).child(msg))
                        .into_any_element()
                })
                .collect()
        })
        .size_full();

        let files = scroll_area("files-scroll").size_full().child(
            div().flex().flex_col().py_1().children((0..48usize).map(|i| {
                let name = format!("module_{i:02}.rs");
                list_item(("file", i), name).icon(Icon::File).meta(format!("{}.{} KB", 1 + i % 9, i % 10)).selected(i == 3)
            })),
        );

        let pane = |title: &'static str, meta: String, body: gpui::AnyElement| {
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .h(px(28.))
                        .px_3()
                        .bg(hsla(p.surface))
                        .border_b_1()
                        .border_color(hsla(p.line))
                        .child(div().display(Scale::X1, window).text_color(hsla(p.fg_dim)).child(title))
                        .child(div().body(text::XS).text_color(hsla(p.fg_faint)).child(meta)),
                )
                .child(div().flex_1().min_h_0().child(body))
        };

        div().flex().flex_col().gap(space::ROW).child(
            panel("Split · scroll area · virtual list").meta("drag the divider · double-click resets").child(
                split("layout-split")
                    .initial(0.28)
                    .min(px(180.))
                    .h(px(560.))
                    .border_1()
                    .border_color(hsla(p.line))
                    .first(pane("FILES", "48 items".into(), files.into_any_element()))
                    .second(pane("LOG", format!("{LOG_LINES} lines · only visible rows render"), log.into_any_element())),
            ),
        )
    }

    fn data_page(&mut self, window: &mut Window, cx: &mut Context<Self>) -> gpui::Div {
        let p = palette(cx);
        let this = cx.weak_entity();
        let set = |f: fn(&mut Components, &mut Context<Components>)| {
            let this = this.clone();
            move |_: &mut Window, cx: &mut App| {
                let _ = this.update(cx, |v, cx| {
                    f(v, cx);
                    cx.notify();
                });
            }
        };

        // ── Input ────────────────────────────────────────────────────────
        let label = |t: &'static str| div().w(px(96.)).body(text::SM).text_color(hsla(p.fg_dim)).child(t);
        let input = panel("Input").meta("segmented · slider").flex_1().child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(section("segmented", window, cx))
                .child(
                    row().child(
                        segmented("density")
                            .option("Compact")
                            .option("Cozy")
                            .option("Roomy")
                            .selected(self.density)
                            .on_select({
                                let this = this.clone();
                                move |i, _, cx| {
                                    let i = *i;
                                    let _ = this.update(cx, |v, cx| {
                                        v.density = i;
                                        v.log(format!("DENSITY {}", ["COMPACT", "COZY", "ROOMY"][i]), cx);
                                    });
                                }
                            }),
                    ),
                )
                .child(
                    row().child(
                        segmented("view")
                            .option_with_icon("List", Icon::Menu)
                            .option_with_icon("Files", Icon::Folder)
                            .selected(0)
                            .disabled(true),
                    ),
                )
                .child(section("slider", window, cx))
                .child(
                    row().child(label("Volume")).child(
                        slider("volume")
                            .label("Volume")
                            .range(0., 100.)
                            .step(5.)
                            .value(self.volume)
                            .format(|v| format!("{v:.0}%").into())
                            .on_change({
                                let this = this.clone();
                                move |v, _, cx| {
                                    let v = *v;
                                    let _ = this.update(cx, |c, cx| {
                                        c.volume = v;
                                        c.log(format!("VOLUME {v:.0}"), cx);
                                    });
                                }
                            }),
                    ),
                )
                .child(
                    row().child(label("Gamma")).child(
                        slider("gamma")
                            .label("Gamma")
                            .range(0.5, 2.5)
                            .step(0.25)
                            .value(self.gamma)
                            .width(px(160.))
                            .on_change({
                                let this = this.clone();
                                move |v, _, cx| {
                                    let v = *v;
                                    let _ = this.update(cx, |c, cx| {
                                        c.gamma = v;
                                        c.log(format!("GAMMA {v:.2}"), cx);
                                    });
                                }
                            }),
                    ),
                )
                .child(row().child(label("Locked")).child(slider("locked").label("Locked").value(30.).disabled(true))),
        );

        // ── Dialog ───────────────────────────────────────────────────────
        let open_modal = |m: Modal| {
            let this = this.clone();
            move |_: &gpui::ClickEvent, _: &mut Window, cx: &mut App| {
                let _ = this.update(cx, |v, cx| {
                    v.dialog = Some(m);
                    cx.notify();
                });
            }
        };
        let close = set(|v, _| v.dialog = None);
        let dialogs = panel("Dialog").meta("modal · screen-door backdrop").flex_1().child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(section("confirm", window, cx))
                .child(
                    row()
                        .child(Button::new("open-delete").label("Delete files").icon(Icon::Trash).danger().on_click(open_modal(Modal::Delete)))
                        .child(Button::new("open-rename").label("Rename").icon(Icon::File).on_click(open_modal(Modal::Rename))),
                )
                .child(
                    div()
                        .body(text::SM)
                        .text_color(hsla(p.fg_dim))
                        .child("Esc, a click on the backdrop, or Cancel close it. Enter confirms."),
                )
                .child(
                    dialog("delete-dialog")
                        .open(self.dialog == Some(Modal::Delete))
                        .title("Delete 3 files?")
                        .description("dither.rs, tokens.rs and Cargo.toml will be removed from disk. This can't be undone.")
                        .danger()
                        .confirm("Delete", set(|v, cx| v.log("DELETED 3 FILES", cx)))
                        .cancel("Keep")
                        .on_close(close.clone()),
                )
                .child(
                    dialog("rename-dialog")
                        .open(self.dialog == Some(Modal::Rename))
                        .title("Rename")
                        .description("Applies to the selected file and every import of it.")
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(
                                    div()
                                        .flex()
                                        .flex_row()
                                        .items_center()
                                        .gap_2()
                                        .px_2()
                                        .h(px(32.))
                                        .bg(hsla(p.sunken))
                                        .border_1()
                                        .border_color(hsla(p.line_strong))
                                        .child(div().display(Scale::X1, window).text_color(hsla(p.accent)).child(">"))
                                        .child(div().body(text::BASE).child("menu.rs"))
                                        .child(cursor("rename-cursor")),
                                )
                                .child(checkbox("rename-imports").label("Update imports").checked(true)),
                        )
                        .confirm("Rename", set(|v, cx| v.log("RENAMED", cx)))
                        .on_close(close),
                ),
        );

        // ── Tree ─────────────────────────────────────────────────────────
        let leaf = |path: &'static str, name: &'static str, size: &'static str| tree_node(path, name).icon(Icon::File).meta(size);
        let files = tree("files")
            .expanded(["src", "src/components", "docs"])
            .selected(self.tree_sel.clone())
            .node(
                tree_node("src", "src/")
                    .icon(Icon::Folder)
                    .child(
                        tree_node("src/components", "components/")
                            .icon(Icon::Folder)
                            .child(leaf("src/components/button.rs", "button.rs", "11.2 KB"))
                            .child(leaf("src/components/dialog.rs", "dialog.rs", "8.9 KB"))
                            .child(leaf("src/components/menu.rs", "menu.rs", "21.4 KB"))
                            .child(leaf("src/components/tree.rs", "tree.rs", "12.0 KB")),
                    )
                    .child(leaf("src/dither.rs", "dither.rs", "9.8 KB"))
                    .child(leaf("src/lib.rs", "lib.rs", "1.6 KB")),
            )
            .node(
                tree_node("docs", "docs/")
                    .icon(Icon::Folder)
                    .child(leaf("docs/DESIGN_LANGUAGE.md", "DESIGN_LANGUAGE.md", "18.1 KB"))
                    .child(tree_node("docs/img", "img/").icon(Icon::Folder).child(leaf("docs/img/palette.png", "palette.png", "212 KB"))),
            )
            .node(leaf("Cargo.toml", "Cargo.toml", "1.0 KB"))
            .on_select({
                let this = this.clone();
                move |id, _, cx| {
                    let id = id.clone();
                    let _ = this.update(cx, |v, cx| {
                        v.log(format!("SELECT {}", id.to_uppercase()), cx);
                        v.tree_sel = Some(id);
                    });
                }
            });
        let tree_panel = panel("Tree").meta("expand · collapse with arrows").flex_1().child(files);

        // ── Table ────────────────────────────────────────────────────────
        let procs = table("procs")
            .column(column("Name").sortable())
            .column(column("PID").width(px(80.)).align_right().sortable())
            .column(column("CPU").width(px(88.)).align_right().sortable())
            .column(column("Mem").width(px(96.)).align_right().sortable())
            .column(column("State").width(px(88.)).sortable())
            .rows(self.procs.iter().map(|p| {
                [p.name.to_string(), p.pid.to_string(), format!("{:.1}%", p.cpu), format!("{} MB", p.mem), p.state.to_string()]
            }))
            .sort(self.sort)
            .selected(self.proc_sel)
            .on_sort({
                let this = this.clone();
                move |&(col, dir), _, cx| {
                    let _ = this.update(cx, |v, cx| {
                        v.sort = Some((col, dir));
                        v.sort_procs();
                        let arrow = if dir == SortDir::Asc { "ASC" } else { "DESC" };
                        v.log(format!("SORT {} {arrow}", ["NAME", "PID", "CPU", "MEM", "STATE"][col]), cx);
                    });
                }
            })
            .on_select({
                let this = this.clone();
                move |&i, _, cx| {
                    let _ = this.update(cx, |v, cx| {
                        v.proc_sel = Some(i);
                        let name = v.procs[i].name.to_uppercase();
                        v.log(format!("PROC {name}"), cx);
                    });
                }
            });
        let table_panel = panel("Table").meta("click a header to sort").flex_1().child(procs);

        div()
            .flex()
            .flex_col()
            .gap(space::ROW)
            .child(div().flex().flex_row().gap(space::ROW).child(input).child(dialogs))
            .child(div().flex().flex_row().gap(space::ROW).child(tree_panel).child(table_panel))
    }
}

/// The pages added with forms, navigation and charts.
impl Components {
    /// A handler for a controlled component: writes the new value into the
    /// view through `f`, then redraws.
    fn upd<T: 'static>(cx: &mut Context<Self>, f: fn(&mut Components, &T)) -> impl Fn(&T, &mut Window, &mut App) + 'static {
        let this = cx.weak_entity();
        move |v, _, cx| {
            let _ = this.update(cx, |c, cx| {
                f(c, v);
                cx.notify();
            });
        }
    }

    fn forms_page(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> gpui::Div {
        let p = palette(cx);
        let handle = self.inputs[0].read(cx).value();
        let form = panel("Field · select · number · date").meta("controlled · Tab walks the form").flex_1().child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    field("f-handle", "Handle")
                        .required()
                        .hint("lowercase, dashes allowed — Enter checks it")
                        .error(self.handle_error.clone())
                        .child(self.inputs[0].clone()),
                )
                .child(
                    field("f-region", "Region").hint("where new workers start").child(
                        select("region")
                            .options(REGIONS)
                            .selected(self.region)
                            .width(px(220.))
                            .on_change(Self::upd(cx, |c, i: &usize| {
                                c.region = Some(*i);
                                c.last = format!("REGION {}", REGIONS[*i].to_uppercase()).into();
                            })),
                    ),
                )
                .child(field("f-disabled", "Tier").child(select("tier").options(["Enterprise"]).selected(Some(0)).disabled(true)))
                .child(
                    field("f-workers", "Workers").child(
                        number_input("workers")
                            .value(self.workers)
                            .range(1., 64.)
                            .on_change(Self::upd(cx, |c, v: &f64| c.workers = *v)),
                    ),
                )
                .child(
                    field("f-timeout", "Timeout").child(
                        number_input("timeout")
                            .value(self.timeout)
                            .range(0.5, 30.)
                            .step(0.5)
                            .digits(2)
                            .decimals(1)
                            .suffix("s")
                            .on_change(Self::upd(cx, |c, v: &f64| c.timeout = *v)),
                    ),
                )
                .child(
                    field("f-launch", "Launch").hint("opens a calendar; arrows move, PgUp/PgDn page").child(
                        date_picker("launch")
                            .selected(self.launch)
                            .on_select(Self::upd(cx, |c, d: &Date| {
                                c.launch = Some(*d);
                                c.last = format!("LAUNCH {d}").into();
                            })),
                    ),
                )
                .child(
                    div().flex().flex_row().justify_end().gap_2().pt_2().border_t_1().border_color(hsla(p.line)).child(Button::new("f-reset").label("Reset").ghost().on_click(
                        cx.listener(|this, _, _, cx| {
                            this.handle_error = None;
                            this.workers = 8.;
                            this.timeout = 2.5;
                            this.region = Some(0);
                            cx.notify();
                        }),
                    ))
                    .child(Button::new("f-save").label("Save").primary().on_click(cx.listener(move |this, _, _, cx| {
                        let name = this.inputs[0].read(cx).value();
                        this.handle_error = if name.trim().is_empty() {
                            Some("a handle is required".into())
                        } else if name.chars().any(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')) {
                            Some("only a–z, 0–9 and dashes".into())
                        } else {
                            None
                        };
                        let ok = this.handle_error.is_none();
                        this.log(if ok { "SAVED" } else { "REJECTED" }, cx);
                    }))),
                )
                .child(div().body(text::XS).text_color(hsla(p.fg_faint)).child(format!("handle: {handle:?}"))),
        );

        let today = Date::new(2026, 10, 6);
        let cal = panel("Calendar").meta("inline month grid").child(
            calendar("inline-cal")
                .today(today)
                .selected(self.launch)
                .range(Some(today), None)
                .on_select(Self::upd(cx, |c, d: &Date| c.launch = Some(*d))),
        );

        let settings = panel("Accordion").meta("settings, grouped").flex_1().child(
            accordion("settings")
                .open(["General"])
                .section(
                    accordion_section("General")
                        .meta("3")
                        .child(switch("acc-tele").label("Telemetry").checked(self.telemetry).on_change(Self::upd(cx, |c, v: &bool| c.telemetry = *v)))
                        .child(checkbox("acc-wrap").label("Word wrap").checked(self.word_wrap).on_change(Self::upd(cx, |c, v: &bool| c.word_wrap = *v)))
                        .child(checkbox("acc-hidden").label("Show hidden files").checked(self.show_hidden).on_change(Self::upd(cx, |c, v: &bool| c.show_hidden = *v))),
                )
                .section(
                    accordion_section("Appearance").meta("2").child(
                        segmented("acc-density").option("Compact").option("Cozy").option("Roomy").selected(self.density).on_select(Self::upd(cx, |c, i: &usize| c.density = *i)),
                    ),
                )
                .section(
                    accordion_section("Danger zone").child(alert("acc-danger", "Irreversible").danger().message("Deleting a workspace removes its history.")).child(
                        Button::new("acc-del").label("Delete workspace").icon(Icon::Trash).danger().small(),
                    ),
                ),
        );

        div()
            .flex()
            .flex_col()
            .gap(space::ROW)
            .child(div().flex().flex_row().gap(space::ROW).child(form).child(cal))
            .child(settings)
    }

    fn nav_page(&mut self, window: &mut Window, cx: &mut Context<Self>) -> gpui::Div {
        let p = palette(cx);
        let content_for = |key: &str| match key {
            "overview" => "Everything at a glance.",
            "logs" => "10,000 lines, virtualised.",
            "jobs" => "Queued, running, done.",
            "alerts" => "What needs you now.",
            _ => "Settings, by section.",
        };
        let nav = self.nav.clone();
        let shell = panel("Sidebar · toolbar · breadcrumb").meta("an app shell").child(
            div()
                .flex()
                .flex_row()
                .h(px(340.))
                .border_1()
                .border_color(hsla(p.line))
                .child(
                    sidebar("nav")
                        .brand("Atlas")
                        .collapsed(self.rail)
                        .section("Workspace")
                        .item_with_meta("overview", "Overview", Icon::Home, "")
                        .item_with_meta("logs", "Logs", Icon::Terminal, "10k")
                        .item_with_meta("jobs", "Jobs", Icon::Play, "3")
                        .section("Account")
                        .item_with_meta("alerts", "Alerts", Icon::Bell, "2")
                        .item("settings", "Settings", Icon::Sliders)
                        .selected(nav.clone())
                        .footer(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_2()
                                .child(avatar("Ryan Wetzstein").size(px(24.)).presence(Presence::Online))
                                .when(!self.rail, |el| el.child(div().body(text::SM).text_color(hsla(p.fg_dim)).child("ryan"))),
                        )
                        .on_select(Self::upd(cx, |c, k: &SharedString| c.nav = k.clone())),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .child(
                            toolbar()
                                .child(Button::new("rail").icon(Icon::Menu).small().ghost().tooltip("Collapse sidebar").on_click(cx.listener(|this, _, _, cx| {
                                    this.rail = !this.rail;
                                    cx.notify();
                                })))
                                .separator()
                                .child(breadcrumb("crumbs").crumbs(["atlas", "workspace"]).crumb(nav.clone()).on_select(Self::upd(cx, |c, _: &usize| c.nav = "overview".into())))
                                .spacer()
                                .child(Button::new("tb-refresh").icon(Icon::Refresh).small().tooltip("Refresh").on_click(cx.listener(|this, _, _, cx| {
                                    this.replay[20] += 1;
                                    cx.notify();
                                })))
                                .child(Button::new("tb-details").label("Details").small().on_click(cx.listener(|this, _, _, cx| {
                                    this.drawer = true;
                                    cx.notify();
                                }))),
                        )
                        .child(
                            scan(
                                "nav-scan",
                                self.replay[20],
                                wipe_in("nav-page", &nav, {
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_3()
                                        .p_4()
                                        .child(div().display(Scale::X2, window).text_color(hsla(p.fg)).child(nav.to_uppercase()))
                                        .child(div().body(text::BASE).text_color(hsla(p.fg_dim)).child(content_for(&nav)))
                                        .child(skeleton_text("nav-skel", 3, px(10.)))
                                }),
                            ),
                        ),
                ),
        );

        let paging = panel("Pagination · steps").meta("controlled").flex_1().child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(section("pagination", window, cx))
                .child(pagination("pager").total(20).current(self.pager).on_change(Self::upd(cx, |c, i: &usize| c.pager = *i)))
                .child(div().body(text::SM).text_color(hsla(p.fg_dim)).child(format!("page {} of 20 · ←/→ when focused", self.pager + 1)))
                .child(section("steps", window, cx))
                .child(steps("wizard").step("Account").step("Workspace").step("Invite").step("Done").current(self.wizard).on_select(Self::upd(cx, |c, i: &usize| c.wizard = *i)))
                .child(
                    row()
                        .child(Button::new("w-back").label("Back").small().disabled(self.wizard == 0).on_click(cx.listener(|this, _, _, cx| {
                            this.wizard = this.wizard.saturating_sub(1);
                            cx.notify();
                        })))
                        .child(Button::new("w-next").label("Next").small().primary().disabled(self.wizard >= 4).on_click(cx.listener(|this, _, _, cx| {
                            this.wizard = (this.wizard + 1).min(4);
                            cx.notify();
                        }))),
                ),
        );

        let dismiss = |i: usize| {
            let this = cx.weak_entity();
            move |_: &mut Window, cx: &mut App| {
                let _ = this.update(cx, |c, cx| {
                    c.alerts[i] = false;
                    cx.notify();
                });
            }
        };
        let alerts = panel("Alert · avatar · timeline").meta("inline feedback").flex_1().child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .when(self.alerts[0], |el| el.child(alert("a-info", "Indexing").message("1,204 files · results may be partial").on_close(dismiss(0))))
                .when(self.alerts[1], |el| {
                    el.child(
                        alert("a-warn", "Disk almost full")
                            .warning()
                            .message("92% of C: used. Old builds can be pruned.")
                            .action(Button::new("prune").label("Prune").small())
                            .on_close(dismiss(1)),
                    )
                })
                .when(self.alerts[2], |el| el.child(alert("a-ok", "Deployed").success().message("staging · build 4412").on_close(dismiss(2))))
                .when(!self.alerts.iter().any(|a| *a), |el| {
                    el.child(Button::new("a-reset").label("Bring them back").small().on_click(cx.listener(|this, _, _, cx| {
                        this.alerts = [true; 3];
                        cx.notify();
                    })))
                })
                .child(section("avatar", window, cx))
                .child(
                    row()
                        .child(avatar("ryan").presence(Presence::Online))
                        .child(avatar("ferrite-atlas").presence(Presence::Busy))
                        .child(avatar("nexis").presence(Presence::Away))
                        .child(avatar("Ada Lovelace").initials())
                        .child(avatar("Grace Hopper").initials().size(px(48.)).presence(Presence::Offline)),
                )
                .child(section("timeline", window, cx))
                .child(
                    timeline("feed")
                        .event(event("12:04", "Deployed to staging").detail("build 4412 · 2.0s").tone(Tone::Success))
                        .event(event("11:58", "Build failed").detail("error[E0308]: mismatched types").tone(Tone::Danger))
                        .event(event("11:41", "Pushed 3 commits").detail("main · ryan").tone(Tone::Accent))
                        .event(event("09:12", "Workspace created")),
                ),
        );

        let details = drawer("details")
            .open(self.drawer)
            .title("Details")
            .on_close({
                let this = cx.weak_entity();
                move |_, cx| {
                    let _ = this.update(cx, |c, cx| {
                        c.drawer = false;
                        cx.notify();
                    });
                }
            })
            .child(
                property_list()
                    .row("name", "ferrite-atlas")
                    .row("pid", "4412")
                    .row_with("state", tag("run").accent())
                    .row_with("cpu", meter(0.31).id("drawer-cpu").segments(12))
                    .row("started", "2026-10-06 09:12")
                    .row("command", "atlas --serve --port 7070"),
            )
            .child(section("recent", window, cx))
            .child(timeline("drawer-feed").event(event("12:04", "healthy").tone(Tone::Success)).event(event("11:58", "restarted").tone(Tone::Warning)))
            .footer(Button::new("drawer-kill").label("Kill").danger().small())
            .footer(Button::new("drawer-close-btn").label("Close").small().shortcut("Esc").on_click(cx.listener(|this, _, _, cx| {
                this.drawer = false;
                cx.notify();
            })));

        div()
            .flex()
            .flex_col()
            .gap(space::ROW)
            .child(shell)
            .child(div().flex().flex_row().gap(space::ROW).child(paging).child(alerts))
            .child(details)
    }

    fn charts_page(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> gpui::Div {
        let p = palette(cx);
        let seed = self.series_seed;
        let wave = move |i: usize, k: f32| {
            let t = i as f32 * 0.35 + seed as f32 * 1.3;
            (40. + 30. * (t * k).sin() + 18. * (t * 0.37 + k).cos() + (i % 5) as f32 * 4.).max(0.)
        };
        let latency: Vec<f32> = (0..48).map(|i| wave(i, 1.)).collect();
        let baseline: Vec<f32> = (0..48).map(|i| wave(i, 0.6) * 0.8).collect();
        let hours: Vec<String> = (0..48).map(|i| format!("{:02}:{:02}", 9 + i / 4, (i % 4) * 15)).collect();

        let tiles = div()
            .flex()
            .flex_row()
            .gap(space::ROW)
            .child(div().flex_1().child(stat("s-req", "Requests", "18.2K").delta(4.2).trend((0..24).map(|i| wave(i, 0.8))).caption("vs last week")))
            .child(div().flex_1().child(stat("s-lat", "p95 latency", format!("{:.0}MS", latency[47])).delta(-6.5).lower_is_better().trend(latency.iter().copied().skip(24))))
            .child(div().flex_1().child(stat("s-err", "Errors", "0.42%").delta(0.1).lower_is_better().trend((0..24).map(|i| wave(i, 2.1) * 0.1))))
            .child(div().flex_1().child(stat("s-up", "Uptime", "99.98%").delta(0.0)));

        let line = panel("Line chart").meta("stepped · dithered area · hover for values").child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(line_chart(("lat-chart", seed as usize), latency.clone()).title("Latency").compare(baseline).labels(hours).format(|v| format!("{v:.0}MS")))
                .child(
                    row()
                        .child(Button::new("reseed").label("New data").icon(Icon::Refresh).small().on_click(cx.listener(|this, _, _, cx| {
                            this.series_seed += 1;
                            cx.notify();
                        })))
                        .child(div().body(text::SM).text_color(hsla(p.fg_dim)).child("Amber: this week. Dim: last week. The chart draws on once; new data lands instantly.")),
                ),
        );

        let bars = panel("Bar chart").meta("hover a bar").flex_1().child(
            bar_chart("deploys")
                .bars([("Mon", 12.), ("Tue", 19.), ("Wed", 7.), ("Thu", 23.), ("Fri", 16.), ("Sat", 3.), ("Sun", 1.)])
                .highlight(Some(3)),
        );

        let heat_rows: Vec<Vec<f32>> = (0..7)
            .map(|d| {
                (0..24)
                    .map(|h| {
                        let work = if (8..19).contains(&h) && d < 5 { 0.6 } else { 0.08 };
                        let n = ((d * 31 + h * 17 + seed as usize * 7) % 13) as f32 / 13.;
                        (work + n * 0.45 - 0.15).clamp(0., 1.)
                    })
                    .collect()
            })
            .collect();
        let heat = panel("Heatmap").meta("weekday × hour · dither density").flex_1().child(heatmap(heat_rows).row_labels(["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]));

        let inline = panel("Sparkline").meta("inline, in rows").child(
            div().flex().flex_col().children(PROC_TABLE.iter().enumerate().map(|(i, (name, _, cpu, _, _))| {
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .h(px(26.))
                    .child(div().w(px(140.)).body(text::SM).child(*name))
                    .child(sparkline((0..20).map(|k| wave(k + i * 7, 1.3) * cpu / 40.)))
                    .child(div().body(text::SM).text_color(hsla(p.fg_dim)).child(format!("{cpu:>5.1}%")))
            })),
        );

        let loading = panel("Skeleton").meta("loading placeholders").flex_1().child(
            div()
                .flex()
                .flex_row()
                .gap_3()
                .child(skeleton("sk-thumb").size(px(64.)))
                .child(div().flex_1().child(skeleton_text("sk-text", 3, px(10.)))),
        );

        div()
            .flex()
            .flex_col()
            .gap(space::ROW)
            .child(tiles)
            .child(line)
            .child(div().flex().flex_row().gap(space::ROW).child(bars).child(heat))
            .child(div().flex().flex_row().gap(space::ROW).child(div().flex_1().child(inline)).child(loading))
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
        let buttons = panel("Button").meta("primary · secondary · ghost · danger").child(
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
        let navigation = panel("Tabs + List").meta("tabs · list_item").child(browser);

        // ── Versus ───────────────────────────────────────────────────────
        let field = |label: &'static str, input: &Entity<TextInput>| {
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_3()
                .child(div().w(px(72.)).flex_none().body(text::SM).text_color(hsla(p.fg_dim)).child(label))
                .child(div().flex_1().child(input.clone()))
        };
        let name = self.inputs[0].read(cx).value();
        let versus = panel("Text input").meta("native · IME · undo · Enter submits").child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(shake("name-shake", self.name_errors, field("Name", &self.inputs[0])))
                .child(field("Filter", &self.inputs[1]))
                .child(field("Token", &self.inputs[2]))
                .child(field("Locked", &self.inputs[3]))
                .child(
                    div()
                        .body(text::SM)
                        .text_color(hsla(p.fg_faint))
                        .child(if name.is_empty() { "name: (empty)".to_string() } else { format!("name: {name}") }),
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

        let forms_page = self.forms_page(window, cx);
        let nav_page = self.nav_page(window, cx);
        let charts_page = self.charts_page(window, cx);
        let data_page = self.data_page(window, cx);
        let layout_page = self.layout_page(window, cx);
        let motion_page = self.motion_page(window, cx);

        chrome::window_frame().child(power_on_in("power-on", div()
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
                scroll_area("scroll").flex_1().min_h_0().child(
                    div()
                        .flex()
                        .flex_col()
                        .p(space::ROW)
                        .gap(space::ROW)
                        .child(
                            tabs("pages")
                                .tab(PAGES[0])
                                .tab(PAGES[1])
                                .tab(PAGES[2])
                                .tab(PAGES[3])
                                .tab(PAGES[4])
                                .tab(PAGES[5])
                                .tab(PAGES[6])
                                .selected(self.page)
                                .on_select({
                                    let this = cx.weak_entity();
                                    move |i: &usize, _, cx| {
                                        let i = *i;
                                        let _ = this.update(cx, |v, cx| {
                                            v.page = i;
                                            cx.notify();
                                        });
                                    }
                                }),
                        )
                        .when(self.page == 1, |el| el.child(unroll_in("page-1", 1, forms_page)))
                        .when(self.page == 2, |el| el.child(unroll_in("page-2", 2, data_page)))
                        .when(self.page == 3, |el| el.child(unroll_in("page-3", 3, nav_page)))
                        .when(self.page == 4, |el| el.child(unroll_in("page-4", 4, charts_page)))
                        .when(self.page == 5, |el| el.child(unroll_in("page-5", 5, layout_page)))
                        .when(self.page == 6, |el| el.child(unroll_in("page-6", 6, motion_page)))
                        .when(self.page == 0, |el| el.child(unroll_in("page-0", 0, div().flex().flex_col().gap(space::ROW)
                        .child(buttons)
                        .child(div().flex().flex_row().gap(space::ROW).child(toggles).child(status))
                        .child(div().flex().flex_row().gap(space::ROW)
                            .child(div().flex_1().child(navigation))
                            .child(div().flex_1().child(versus)))
                        .child(div().flex().flex_row().gap(space::ROW)
                            .child(div().flex_1().child(overlays))
                            .child(div().flex_1().child(icons)))))),
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
                            .right(format!("{}FPS", motion::fps()))
                            .right(format!("SCALE {:.2}x", window.scale_factor())),
                    ),
            )))
    }
}

/// The display face at its crisp 1× size, for elements styled by hand.
fn fonts_x1(window: &Window) -> gpui::Pixels {
    ferrite_design::fonts::display_size(Scale::X1, window)
}

/// A lit orb, as levels: the develop demo's picture (3:2).
fn orb_scene(u: f32, v: f32) -> f32 {
    let (x, y) = (u * 1.5, v);
    let (nx, ny) = ((x - 0.75) / 0.36, (y - 0.5) / 0.36);
    let d2 = nx * nx + ny * ny;
    if d2 >= 1. {
        return 0.06 + 0.1 * v;
    }
    let nz = (1. - d2).sqrt();
    let diffuse = (-0.5 * nx - 0.6 * ny + 0.62 * nz).max(0.) / 0.94;
    0.04 + 0.9 * diffuse
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let appearance = match std::env::var("FERRITE_APPEARANCE").as_deref() {
            Ok("light") => Appearance::Light,
            Ok("system") => Appearance::System,
            _ => Appearance::Dark,
        };
        ferrite_design::init(appearance, cx);
        // FERRITE_FPS=60|120|240… starts at that refresh rate (default 240).
        if let Some(fps) = std::env::var("FERRITE_FPS").ok().and_then(|v| v.parse().ok()) {
            motion::set_fps(fps);
        }
        cx.bind_keys([gpui::KeyBinding::new("ctrl-shift-p", TogglePalette, None)]);
        let options = chrome::window_options("Ferrite Components", size(px(1180.), px(900.)), cx);
        cx.open_window(options, |window, cx| {
            chrome::square_corners(window);
            chrome::power_off_on_close(window, cx);
            cx.new(|cx| Components::new(window, cx))
        })
        .expect("failed to open the components window");
        cx.activate(true);
    });
}
