//! Every Ferrite primitive on one screen.
//!
//!     cargo run --example showcase
//!     FERRITE_FPS=120 cargo run --example showcase

// Release builds are GUI-subsystem on Windows: no console window behind the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ferrite_design::{
    Appearance, FerriteText, Scale, ascii,
    chrome::{self, title_bar},
    components::{cursor, empty_state, panel, power_on_in, progress_bar, rule, segmented, status_bar},
    dither::{self, dither},
    motion, palette,
    theme,
    tokens::{hsla, space, text},
};
use gpui::{
    App, AppContext as _, Context, IntoElement, ParentElement, Render, Styled, Subscription,
    Window, div, px, size, InteractiveElement as _,
    StatefulInteractiveElement as _,
};
use ferrite_design::components::{Button, switch, tag};

struct Showcase {
    /// When the demo started: progress and spinner run on elapsed time, so
    /// a faster refresh rate makes them smoother, never faster.
    started: std::time::Instant,
    /// Built once: the pattern comparison dithers it every frame from cache.
    sphere: dither::Picture,
    _appearance: Subscription,
}

impl Showcase {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Redraw the progress demo and spinner every second live frame
        // (`motion::frame`, which the refresh picker changes): ~8ms at the
        // default 240fps, 80ms at 25. Under reduced motion nothing ticks;
        // the screen renders its final state.
        if !motion::reduced(cx) {
            cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor().timer(motion::frame() * 2).await;
                    if this.update(cx, |_, cx| cx.notify()).is_err() {
                        break;
                    }
                }
            })
            .detach();
        }
        Self { started: std::time::Instant::now(), sphere: dither::Picture::from_fn(300, 200, sphere_scene), _appearance: theme::follow_system(window) }
    }

    /// A 4.8s loop, 0 → 1.
    fn progress(&self) -> f32 {
        const LOOP: f32 = 4.8;
        (self.started.elapsed().as_secs_f32() % LOOP) / LOOP
    }
}

impl Render for Showcase {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let value = self.progress();
        let is_dark = p.is_dark();
        let elapsed = self.started.elapsed();

        let swatch = |name: &'static str, hex: u32, window: &Window| {
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .child(div().w(px(32.)).h(px(16.)).bg(hsla(hex)).border_1().border_color(hsla(p.line)))
                .child(div().display(Scale::X1, window).w(px(184.)).child(name))
                .child(div().body(text::SM).text_color(hsla(p.fg_dim)).child(format!("#{hex:06X}")))
        };

        let dither_sample = |label: &'static str, field: dither::Field, window: &Window| {
            div()
                .flex()
                .flex_col()
                .gap_1()
                .flex_1()
                .child(dither(field).ink(hsla(p.fg_dim)).h(px(48.)).w_full())
                .child(div().display(Scale::X1, window).text_color(hsla(p.fg_dim)).child(label))
        };

        // One column of the pattern comparison: the same three sources.
        let pattern_column = |label: &'static str, note: &'static str, pattern: dither::Pattern, window: &Window| {
            let ink = hsla(p.accent);
            div()
                .flex()
                .flex_col()
                .flex_1()
                .gap_2()
                .child(div().display(Scale::X1, window).child(label))
                .child(dither(dither::horizontal(0.0, 1.0)).pattern(pattern).ink(ink).h(px(32.)).w_full())
                .child(dither(dither::radial(1.0, 0.0)).pattern(pattern).ink(ink).h(px(64.)).w_full())
                // Fixed 3:2, the picture's own aspect, so the sphere stays round.
                .child(dither(self.sphere.clone()).pattern(pattern).ink(ink).w(px(300.)).h(px(200.)))
                .child(div().body(text::SM).text_color(hsla(p.fg_dim)).child(note))
        };

        power_on_in("power-on", div()
            .flex()
            .flex_col()
            .size_full()
            .bg(hsla(p.bg))
            .text_color(hsla(p.fg))
            .body(text::BASE)
            .child(title_bar("Ferrite Showcase"))
            .child(
                div()
                    .id("showcase-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .p(space::ROW)
                            .gap(space::ROW)
                    // ── Type ─────────────────────────────────────────────────
                    .child(
                        panel("Type").meta("PxPlus IBM VGA 8x16 / JetBrains Mono").child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(
                                    div()
                                        .flex()
                                        .flex_row()
                                        .items_end()
                                        .gap_3()
                                        .child(div().display(Scale::X3, window).text_color(hsla(p.accent_text)).child("FERRITE"))
                                        .child(cursor("hero-cursor")),
                                )
                                .child(div().display(Scale::X2, window).child("DISPLAY X2 — 32PX CELL"))
                                .child(div().display(Scale::X1, window).child("DISPLAY X1 ─ ░▒▓█ ┌─┐ │ └─┘ ═║ ←↑→↓"))
                                .child(rule(Some("body"), window, cx))
                                .child(div().body(text::LG).child("Body 15 — JetBrains Mono for anything read at length."))
                                .child(div().body(text::BASE).child("Body 13 — the default UI size. Inputs, lists, logs, code."))
                                .child(div().body(text::SM).text_color(hsla(p.fg_dim)).child("Body 12 dim — secondary text, still WCAG AA.")),
                        ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(space::ROW)
                            // ── Palette ──────────────────────────────────────
                            .child(
                                panel(if is_dark { "Iron" } else { "Paper" })
                                    .meta("palette")
                                    .flex_1()
                                    .child(swatch("SUNKEN", p.sunken, window))
                                    .child(swatch("BG", p.bg, window))
                                    .child(swatch("SURFACE", p.surface, window))
                                    .child(swatch("RAISED", p.raised, window))
                                    .child(swatch("LINE", p.line, window))
                                    .child(swatch("LINE STRONG", p.line_strong, window))
                                    .child(swatch("FG", p.fg, window))
                                    .child(swatch("FG DIM", p.fg_dim, window))
                                    .child(swatch("FG FAINT", p.fg_faint, window))
                                    .child(swatch("ACCENT", p.accent, window))
                                    .child(swatch("ACCENT DIM", p.accent_dim, window))
                                    .child(swatch("DANGER", p.danger, window))
                                    .child(swatch("SUCCESS", p.success, window))
                                    .child(swatch("WARNING", p.warning, window)),
                            )
                            // ── Controls (native Ferrite components) ─────────
                            .child(
                                panel("Controls")
                                    .meta("native")
                                    .flex_1()
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .flex_wrap()
                                            .gap_2()
                                            .child(Button::new("run").label("RUN").primary())
                                            .child(Button::new("step").label("STEP"))
                                            .child(Button::new("ghost").label("GHOST").ghost())
                                            .child(Button::new("kill").label("KILL").danger()),
                                    )
                                    .child(
                                        switch("appearance")
                                            .checked(!is_dark)
                                            .label("Paper mode")
                                            .on_change(cx.listener(|_, checked: &bool, window, cx| {
                                                let pref = if *checked { Appearance::Light } else { Appearance::Dark };
                                                theme::set_appearance(pref, window, cx);
                                            })),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .gap_2()
                                            .child(tag("live").accent())
                                            .child(tag("idle"))
                                            .child(tag("fault").danger())
                                            .child(tag("ok").success()),
                                    )
                                    .child(rule(Some("refresh"), window, cx))
                                    .child({
                                        let rates = motion::RATES;
                                        let current = rates.iter().position(|&r| r == motion::fps()).unwrap_or(1);
                                        rates.iter().fold(segmented("fps"), |seg, r| seg.option(format!("{r}")))
                                            .selected(current)
                                            .on_select(move |i, _, cx| {
                                                motion::set_fps(rates[*i]);
                                                cx.refresh_windows();
                                            })
                                    })
                                    .child(rule(Some("progress"), window, cx))
                                    .child(progress_bar(value, px(16.), cx))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .gap_2()
                                            .child(div().display(Scale::X1, window).text_color(hsla(p.accent_text)).child(ascii::bar(value, 24)))
                                            .child(div().display(Scale::X1, window).child(format!("{:>3}%", (value * 100.0).round() as u32)))
                                            .child(div().display(Scale::X1, window).text_color(hsla(p.fg_dim)).child(ascii::spinner(elapsed, motion::FRAME * 3).to_string())),
                                    )
                                    .child(
                                        div()
                                            .body(text::LG)
                                            .text_color(hsla(p.fg_dim))
                                            .child(ascii::sparkline(&[3., 4., 2., 6., 8., 7., 9., 5., 4., 6., 8., 10., 7., 6., 9., 11., 8., 6., 4., 5.])),
                                    ),
                            ),
                    )
                    // ── Dither ───────────────────────────────────────────────
                    .child(
                        panel("Dither").meta("bayer 4x4 · device-pixel cells").child(
                            div()
                                .flex()
                                .flex_row()
                                .gap(space::ROW)
                                .child(dither_sample("░ 0.25", dither::flat(dither::level::LIGHT), window))
                                .child(dither_sample("▒ 0.50", dither::flat(dither::level::MEDIUM), window))
                                .child(dither_sample("▓ 0.75", dither::flat(dither::level::DARK), window))
                                .child(dither_sample("RAMP →", dither::horizontal(0.0, 1.0), window))
                                .child(dither_sample("RADIAL", dither::radial(1.0, 0.0), window)),
                        ),
                    )
                    // ── Dither patterns, side by side ───────────────────────
                    .child(
                        panel("Patterns").meta("ramp · radial · picture").child(
                            div()
                                .flex()
                                .flex_row()
                                .gap(space::ROW)
                                .child(pattern_column("BAYER 4×4", "The texture. Default for fields.", dither::Pattern::Bayer4, window))
                                .child(pattern_column("BLUE NOISE", "Ordered, grid-free grain.", dither::Pattern::BlueNoise, window))
                                .child(pattern_column("ATKINSON", "Pictures only. Default for pictures.", dither::Pattern::Atkinson, window)),
                        ),
                    )
                    // ── Empty state ──────────────────────────────────────────
                    .child(
                        panel("Empty state").h(px(220.)).child(empty_state(
                            "No signal",
                            "Nothing here yet. Structural dither frames the void.",
                            window,
                            cx,
                        )),
                    ),
                    )
            )
            .child(
                status_bar()
                    .left("READY")
                    .left(if is_dark { "IRON" } else { "PAPER" })
                    .left(ascii::bracket("native controls"))
                    .right(format!("SCALE {:.2}x", window.scale_factor()))
                    .right(format!("{}FPS · {:.1}MS/FRAME", motion::fps(), motion::frame().as_secs_f32() * 1000.)),
            ))
    }
}

/// A lit sphere on a floor, as levels (1 = full ink): enough tone range to
/// tell the patterns apart. The picture is 3:2, so `x` runs 0→1.5.
fn sphere_scene(u: f32, v: f32) -> f32 {
    let (x, y) = (u * 1.5, v);
    // Backdrop: a dim wall, a brighter floor below the horizon.
    let mut level = if y > 0.72 { 0.18 + (y - 0.72) * 0.8 } else { 0.10 + y * 0.12 };
    // A soft shadow, thrown down-right by the upper-left light.
    let (sx, sy) = ((x - 0.84) / 0.34, (y - 0.80) / 0.06);
    let shadow = (sx * sx + sy * sy).sqrt();
    if shadow < 1.0 {
        level *= 0.2 + 0.8 * shadow;
    }
    // The ball: Lambert diffuse plus a specular glint.
    let (nx, ny) = ((x - 0.75) / 0.3, (y - 0.44) / 0.3);
    let d2 = nx * nx + ny * ny;
    if d2 < 1.0 {
        let nz = (1.0 - d2).sqrt();
        let light = [-0.5f32, -0.6, 0.62];
        let len = (light[0] * light[0] + light[1] * light[1] + light[2] * light[2]).sqrt();
        let [lx, ly, lz] = light.map(|c| c / len);
        let diffuse = (nx * lx + ny * ly + nz * lz).max(0.0);
        let (hx, hy, hz) = (lx, ly, lz + 1.0);
        let hlen = (hx * hx + hy * hy + hz * hz).sqrt();
        let spec = ((nx * hx + ny * hy + nz * hz) / hlen).max(0.0).powf(28.0);
        level = 0.03 + 0.85 * diffuse + 0.6 * spec;
    }
    level
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        // Theme first, window second: the first frame is already Ferrite.
        // FERRITE_APPEARANCE=light|system overrides the dark default.
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

        let options = chrome::window_options("Ferrite Showcase", size(px(1080.), px(860.)), cx);
        cx.open_window(options, |window, cx| {
            chrome::square_corners(window);
            chrome::power_off_on_close(window, cx);
            cx.new(|cx| Showcase::new(window, cx))
        })
        .expect("failed to open the showcase window");
        cx.activate(true);
    });
}
