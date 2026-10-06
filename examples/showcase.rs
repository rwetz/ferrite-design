//! Every Ferrite primitive on one screen, next to stock gpui-component
//! widgets wearing the Ferrite theme.
//!
//!     cargo run --example showcase

use ferrite_design::{
    Appearance, FerriteText, Scale, ascii,
    chrome::{self, title_bar},
    components::{cursor, empty_state, panel, progress_bar, rule, status_bar},
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
use gpui_component::{
    Root,
    button::{Button, ButtonVariants as _},
    switch::Switch,
    tag::Tag,
};

struct Showcase {
    tick: u64,
    _appearance: Subscription,
}

impl Showcase {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Drive the progress demo and spinner at the stepped frame rate.
        // Under reduced motion nothing ticks; the screen renders its final state.
        if !motion::reduced(cx) {
            cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor().timer(motion::FRAME * 2).await;
                    if this.update(cx, |this, cx| {
                        this.tick += 1;
                        cx.notify();
                    }).is_err() {
                        break;
                    }
                }
            })
            .detach();
        }
        Self { tick: 0, _appearance: theme::follow_system(window) }
    }

    fn progress(&self) -> f32 {
        (self.tick % 120) as f32 / 119.0
    }
}

impl Render for Showcase {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let value = self.progress();
        let is_dark = p.is_dark();
        let elapsed = motion::FRAME * 2 * self.tick as u32;

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

        div()
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
                            // ── Controls (gpui-component, Ferrite-themed) ────
                            .child(
                                panel("Controls")
                                    .meta("gpui-component")
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
                                        Switch::new("appearance")
                                            .checked(!is_dark)
                                            .label("Paper mode")
                                            .on_click(cx.listener(|_, checked: &bool, window, cx| {
                                                let pref = if *checked { Appearance::Light } else { Appearance::Dark };
                                                theme::set_appearance(pref, window, cx);
                                            })),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .gap_2()
                                            .child(Tag::primary().child("LIVE"))
                                            .child(Tag::secondary().child("IDLE"))
                                            .child(Tag::danger().child("FAULT"))
                                            .child(Tag::success().child("OK")),
                                    )
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
                    .left(ascii::bracket("gpui-component 0.7.1"))
                    .right(format!("SCALE {:.2}x", window.scale_factor()))
                    .right(format!("{}", motion::FRAME.as_millis()) + "MS/FRAME"),
            )
    }
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

        let options = chrome::window_options("Ferrite Showcase", size(px(1080.), px(860.)), cx);
        cx.open_window(options, |window, cx| {
            chrome::square_corners(window);
            let view = cx.new(|cx| Showcase::new(window, cx));
            cx.new(|cx| Root::new(view, window, cx))
        })
        .expect("failed to open the showcase window");
        cx.activate(true);
    });
}
