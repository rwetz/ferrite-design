//! Template: a monitoring dashboard.
//!
//! Sidebar navigation, a toolbar with a time-range switch, KPI tiles, a live
//! line chart, a sortable service table with a details drawer, toasts and a
//! command palette. Data is synthetic and refreshes every two seconds — live
//! data updates instantly; only events animate.
//!
//!     cargo run --example app_dashboard
//!
//! To start an app from it: copy this file to `src/main.rs` of a new crate
//! (docs/SCAFFOLDING.md, or `scripts/new-app.sh my-app dashboard`) and
//! replace `Telemetry` with your data source.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::time::Duration;

use ferrite_design::prelude::*;
use gpui::{
    App, AppContext as _, Context, Entity, IntoElement, KeyBinding, Render, SharedString, Subscription, Window, div,
    px, size,
};

const RANGES: [&str; 3] = ["1H", "24H", "7D"];

/// Stand-in for your data source.
#[derive(Clone)]
struct Service {
    name: &'static str,
    region: &'static str,
    rps: f32,
    p95: f32,
    errors: f32,
}

struct Telemetry {
    tick: u64,
}

impl Telemetry {
    fn wave(&self, i: usize, k: f32) -> f32 {
        let t = (self.tick as f32 + i as f32) * 0.3;
        (50. + 25. * (t * k).sin() + 12. * (t * 0.41 + k).cos()).max(1.)
    }

    fn latency(&self) -> Vec<f32> {
        (0..60).map(|i| self.wave(i, 1.)).collect()
    }

    fn services(&self) -> Vec<Service> {
        let names = [("api", "eu-west-1"), ("auth", "eu-west-1"), ("billing", "us-east-1"), ("search", "us-east-1"), ("media", "ap-south-1"), ("queue", "eu-west-1")];
        names
            .iter()
            .enumerate()
            .map(|(i, (name, region))| Service { name, region, rps: self.wave(i * 7, 0.7) * 12., p95: self.wave(i * 3, 1.3), errors: self.wave(i * 5, 2.) / 80. })
            .collect()
    }
}

struct Dashboard {
    nav: SharedString,
    range: usize,
    data: Telemetry,
    sort: Option<(usize, SortDir)>,
    selected: Option<usize>,
    drawer: bool,
    palette: Entity<CommandPalette>,
    toaster: Entity<Toaster>,
    _appearance: Subscription,
}

impl Dashboard {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Live data: a timer, and a plain redraw — no animation.
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(2)).await;
                if this.update(cx, |this, cx| {
                    this.data.tick += 1;
                    cx.notify();
                }).is_err() {
                    break;
                }
            }
        })
        .detach();

        let weak = cx.weak_entity();
        let go = move |key: &'static str| {
            let weak = weak.clone();
            move |_: &mut Window, cx: &mut App| {
                let _ = weak.update(cx, |this, cx| {
                    this.nav = key.into();
                    cx.notify();
                });
            }
        };
        let palette = cx.new(|cx| {
            let mut p = CommandPalette::new(window, cx);
            p.set_commands(
                vec![
                    command("Go to overview").group("Navigate").icon(Icon::Home).on_run(go("overview")),
                    command("Go to services").group("Navigate").icon(Icon::Chart).on_run(go("services")),
                    command("Go to alerts").group("Navigate").icon(Icon::Bell).on_run(go("alerts")),
                    command("Dark theme").group("Theme").on_run(|window, cx| theme::set_appearance(Appearance::Dark, window, cx)),
                    command("Light theme").group("Theme").on_run(|window, cx| theme::set_appearance(Appearance::Light, window, cx)),
                ],
                cx,
            );
            p
        });
        Self {
            nav: "overview".into(),
            range: 1,
            data: Telemetry { tick: 0 },
            sort: Some((1, SortDir::Desc)),
            selected: None,
            drawer: false,
            palette,
            toaster: cx.new(|_| Toaster::new()),
            _appearance: theme::follow_system(window),
        }
    }

    fn sorted_services(&self) -> Vec<Service> {
        let mut rows = self.data.services();
        if let Some((col, dir)) = self.sort {
            rows.sort_by(|a, b| {
                let o = match col {
                    0 => a.name.cmp(b.name),
                    1 => a.rps.total_cmp(&b.rps),
                    2 => a.p95.total_cmp(&b.p95),
                    _ => a.errors.total_cmp(&b.errors),
                };
                if dir == SortDir::Desc { o.reverse() } else { o }
            });
        }
        rows
    }
}

impl Render for Dashboard {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let latency = self.data.latency();
        let services = self.sorted_services();
        let total_rps: f32 = services.iter().map(|s| s.rps).sum();

        // The title bar already carries the name; the sidebar doesn't repeat it.
        let nav = sidebar("nav")
            .section("Monitor")
            .item("overview", "Overview", Icon::Home)
            .item_with_meta("services", "Services", Icon::Chart, format!("{}", services.len()))
            .item_with_meta("alerts", "Alerts", Icon::Bell, "1")
            .section("Admin")
            .item("settings", "Settings", Icon::Sliders)
            .selected(self.nav.clone())
            .footer(div().flex().flex_row().items_center().gap_2().child(avatar("ops on call").size(px(24.)).presence(Presence::Online)).child(div().body(text::SM).text_color(hsla(p.fg_dim)).child("on call")))
            .on_select(cx.listener(|this, key: &SharedString, _, cx| {
                this.nav = key.clone();
                cx.notify();
            }));

        let bar = toolbar()
            .child(breadcrumb("crumbs").crumb("pulse").crumb(self.nav.clone()))
            .spacer()
            .child(
                RANGES.iter().fold(segmented("range"), |s, r| s.option(*r)).selected(self.range).on_select(cx.listener(|this, i: &usize, _, cx| {
                    this.range = *i;
                    cx.notify();
                })),
            )
            .separator()
            .child(Button::new("cmd").icon(Icon::Search).small().tooltip("Commands · Ctrl+Shift+P").on_click({
                let palette = self.palette.clone();
                move |_, window, cx| palette.update(cx, |p, cx| p.open(window, cx))
            }));

        let tiles = div()
            .flex()
            .flex_row()
            .gap(space::ROW)
            .child(div().flex_1().child(stat("rps", "Requests/s", format!("{total_rps:.0}")).delta(3.4).trend(latency.iter().rev().take(24).copied())))
            .child(div().flex_1().child(stat("p95", "p95 latency", format!("{:.0}MS", latency[59])).delta(-2.0).lower_is_better().trend(latency.iter().skip(36).copied())))
            .child(div().flex_1().child(stat("err", "Error rate", "0.31%").delta(0.05).lower_is_better()))
            .child(div().flex_1().child(stat("up", "Uptime", "99.99%").delta(0.0)));

        let chart = panel("Latency").meta(format!("last {}", RANGES[self.range])).child(
            line_chart("latency", latency)
                .labels((0..60).map(|i| format!("-{}m", 60 - i)))
                .format(|v| format!("{v:.0}MS"))
                .height(px(180.)),
        );

        let table_panel = panel("Services").meta("click a row for details").flex_1().child(
            table("services")
                .column(column("Service").sortable())
                .column(column("Req/s").width(px(90.)).align_right().sortable())
                .column(column("p95").width(px(80.)).align_right().sortable())
                .column(column("Errors").width(px(80.)).align_right().sortable())
                .rows(services.iter().map(|s| vec![s.name.to_string(), format!("{:.0}", s.rps), format!("{:.0}ms", s.p95), format!("{:.2}%", s.errors)]))
                .sort(self.sort)
                .selected(self.selected)
                .on_sort(cx.listener(|this, s: &(usize, SortDir), _, cx| {
                    this.sort = Some(*s);
                    cx.notify();
                }))
                .on_select(cx.listener(|this, i: &usize, _, cx| {
                    this.selected = Some(*i);
                    this.drawer = true;
                    cx.notify();
                })),
        );

        let incidents = panel("Incidents").meta("today").w(px(320.)).child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(alert("deg", "Search degraded").warning().message("p95 above 120ms in us-east-1").action(Button::new("ack").label("Acknowledge").small().on_click({
                    let toaster = self.toaster.clone();
                    move |_, _, cx| toaster.update(cx, |t, cx| {
                        t.push(toast("Acknowledged").success().message("search · paged to on-call"), cx);
                    })
                })))
                .child(
                    timeline("feed")
                        .event(event("12:04", "Deploy finished").detail("api · build 4412").tone(Tone::Success))
                        .event(event("11:58", "Latency alert").detail("search · us-east-1").tone(Tone::Warning))
                        .event(event("09:30", "Daily report sent")),
                ),
        );

        let details = self.selected.and_then(|i| services.get(i)).cloned();
        let drawer_el = drawer("service")
            .open(self.drawer && details.is_some())
            .title(details.as_ref().map(|s| s.name).unwrap_or("Service"))
            .on_close({
                let weak = cx.weak_entity();
                move |_, cx| {
                    let _ = weak.update(cx, |this, cx| {
                        this.drawer = false;
                        cx.notify();
                    });
                }
            })
            .when_some(details, |d, s| {
                d.child(
                    property_list()
                        .row("region", s.region)
                        .row("requests/s", format!("{:.0}", s.rps))
                        .row("p95", format!("{:.0}ms", s.p95))
                        .row_with("errors", meter(s.errors / 2.).id("drawer-err").segments(12)),
                )
            });

        let body = scroll_area("body").flex_1().min_h_0().child(
            div()
                .flex()
                .flex_col()
                .gap(space::ROW)
                .p(space::ROW)
                .child(tiles)
                .child(chart)
                .child(div().flex().flex_row().gap(space::ROW).child(table_panel).child(incidents)),
        );

        window_frame().child(power_on_in(
            "power",
            div()
                .flex()
                .flex_col()
                .size_full()
                .bg(hsla(p.bg))
                .text_color(hsla(p.fg))
                .body(text::BASE)
                .on_action(cx.listener(|this, _: &TogglePalette, window, cx| this.palette.update(cx, |p, cx| p.toggle(window, cx))))
                .child(self.palette.clone())
                .child(self.toaster.clone())
                .child(title_bar("Pulse"))
                .child(div().flex().flex_row().flex_1().min_h_0().child(nav).child(div().flex().flex_col().flex_1().min_w_0().child(bar).child(body)))
                .child(drawer_el)
                .child(status_bar().left("LIVE").left_live(format!("TICK {}", self.data.tick)).right(format!("{}FPS", motion::fps())).right(p.name.to_uppercase())),
        ))
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        ferrite_design::init(Appearance::Dark, cx);
        // FERRITE_SCHEME=harbor|mono|phosphor|… starts in another color scheme.
        if let Some(scheme) = std::env::var("FERRITE_SCHEME").ok().and_then(|k| ferrite_design::schemes::by_key(&k)) {
            ferrite_design::theme::set_scheme(scheme, cx);
        }
        cx.bind_keys([KeyBinding::new("ctrl-shift-p", TogglePalette, None)]);
        let options = chrome::window_options("Pulse", size(px(1280.), px(860.)), cx);
        cx.open_window(options, |window, cx| {
            chrome::square_corners(window);
            chrome::power_off_on_close(window, cx);
            cx.new(|cx| Dashboard::new(window, cx))
        })
        .expect("failed to open the window");
        cx.activate(true);
    });
}
