//! Template: a wizard — an installer, onboarding, a setup or export flow.
//!
//! A fixed-size window with a step strip, one page per step, Back/Next with
//! per-page validation (a rejected Next shakes the field), an install step
//! driven by a timer with a progress bar and a cascading log, and a final
//! page. Completed steps are clickable to go back.
//!
//!     cargo run --example app_wizard

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::time::Duration;

use ferrite_design::prelude::*;
use gpui::{App, AppContext as _, Context, Entity, IntoElement, Render, SharedString, Subscription, Window, div, px, size};

const STEPS: [&str; 4] = ["Account", "Workspace", "Install", "Done"];
const REGIONS: [&str; 3] = ["eu-west-1", "us-east-1", "ap-south-1"];
const TASKS: [&str; 6] = ["resolve dependencies", "fetch toolchain", "unpack fonts", "warm dither cache", "write config", "register service"];

struct Wizard {
    step: usize,
    name: Entity<TextInput>,
    name_error: Option<SharedString>,
    region: Option<usize>,
    features: [bool; 3],
    start: Option<Date>,
    progress: usize,
    _subs: Vec<Subscription>,
}

impl Wizard {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name = cx.new(|cx| TextInput::new(window, cx).placeholder("workspace name"));
        let sub = cx.subscribe(&name, |this: &mut Self, _, ev: &InputEvent, cx| match ev {
            InputEvent::Change => {
                this.name_error = None;
                cx.notify();
            }
            InputEvent::Submit => this.next(cx),
        });
        Self { step: 0, name, name_error: None, region: None, features: [true, true, false], start: None, progress: 0, _subs: vec![sub, theme::follow_system(window)] }
    }

    /// Validate the current page; advance only if it passes.
    fn next(&mut self, cx: &mut Context<Self>) {
        match self.step {
            0 => {
                let name = self.name.read(cx).value();
                if name.trim().len() < 3 {
                    self.name_error = Some("at least three characters".into());
                    cx.notify();
                    return;
                }
            }
            1 if self.region.is_none() => return,
            _ => {}
        }
        self.step = (self.step + 1).min(STEPS.len() - 1);
        if self.step == 2 {
            self.install(cx);
        }
        cx.notify();
    }

    /// The stand-in for real work: one task every 450ms.
    fn install(&mut self, cx: &mut Context<Self>) {
        self.progress = 0;
        cx.spawn(async move |this, cx| {
            for _ in 0..TASKS.len() {
                cx.background_executor().timer(Duration::from_millis(450)).await;
                if this.update(cx, |this, cx| {
                    this.progress += 1;
                    cx.notify();
                }).is_err() {
                    return;
                }
            }
        })
        .detach();
    }

    fn page(&mut self, window: &mut Window, cx: &mut Context<Self>) -> gpui::AnyElement {
        let p = palette(cx);
        match self.step {
            0 => div()
                .flex()
                .flex_col()
                .gap_4()
                .child(div().display(Scale::X2, window).child(typewriter("hello", "SET UP YOUR WORKSPACE")))
                .child(div().body(text::BASE).text_color(hsla(p.fg_dim)).child("A few questions, then Ferrite installs everything. You can change all of it later."))
                .child(field("name", "Name").required().hint("Enter to continue").error(self.name_error.clone()).child(self.name.clone()))
                .into_any_element(),
            1 => div()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    field("region", "Region").required().hint("where your workspace lives").child(select("region-select").options(REGIONS).selected(self.region).placeholder("Choose a region").on_change(cx.listener(|this, i: &usize, _, cx| {
                        this.region = Some(*i);
                        cx.notify();
                    }))),
                )
                .child(
                    field("features", "Features").child(
                        div().flex().flex_col().gap_2().children(["Command palette", "Crash reports", "Nightly updates"].into_iter().enumerate().map(|(i, label)| {
                            checkbox(("feature", i)).label(label).checked(self.features[i]).on_change(cx.listener(move |this, v: &bool, _, cx| {
                                this.features[i] = *v;
                                cx.notify();
                            }))
                        })),
                    ),
                )
                .child(field("start", "Start on").hint("optional").child(date_picker("start-date").selected(self.start).range(Some(Date::today()), None).on_select(cx.listener(|this, d: &Date, _, cx| {
                    this.start = Some(*d);
                    cx.notify();
                }))))
                .into_any_element(),
            2 => {
                let done = self.progress >= TASKS.len();
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(div().display(Scale::X1, window).text_color(hsla(if done { p.accent_text } else { p.fg })).child(decrypt("phase", if done { "INSTALLED" } else { "INSTALLING…" })))
                    .child(progress_bar(self.progress as f32 / TASKS.len() as f32, px(16.), cx))
                    .child(
                        cascade_in("tasks", self.progress).flex().flex_col().gap_1().children(TASKS.iter().take(self.progress).map(|t| {
                            div().flex().flex_row().items_center().gap_2().body(text::SM).text_color(hsla(p.fg_dim)).child(icon(Icon::Check).fit(px(16.)).color(hsla(p.success))).child(*t)
                        })),
                    )
                    .when(!done, |el| el.child(div().flex().flex_row().items_center().gap_2().child(spinner("busy").color(hsla(p.accent))).child(div().body(text::SM).child(TASKS[self.progress.min(TASKS.len() - 1)]))))
                    .into_any_element()
            }
            _ => div()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_4()
                .size_full()
                .child(power_on_in("ready", div().px_8().py_4().border_1().border_color(hsla(p.accent)).display(Scale::X3, window).text_color(hsla(p.accent_text)).child("READY.")))
                .child(
                    property_list()
                        .row("name", self.name.read(cx).value())
                        .row("region", self.region.map(|i| REGIONS[i]).unwrap_or("—"))
                        .row("start", self.start.map(|d| d.iso()).unwrap_or_else(|| "now".into())),
                )
                .into_any_element(),
        }
    }
}

impl Render for Wizard {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let step = self.step;
        let installing = step == 2 && self.progress < TASKS.len();
        let can_next = match step {
            1 => self.region.is_some(),
            2 => !installing,
            3 => false,
            _ => true,
        };
        let page = self.page(window, cx);

        let footer = div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .h(px(56.))
            .px_4()
            .bg(hsla(p.surface))
            .border_t_1()
            .border_color(hsla(p.line))
            .child(div().body(text::SM).text_color(hsla(p.fg_faint)).child(format!("step {} of {}", step + 1, STEPS.len())))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .child(Button::new("back").label("Back").disabled(step == 0 || installing || step == 3).on_click(cx.listener(|this, _, _, cx| {
                        this.step = this.step.saturating_sub(1);
                        cx.notify();
                    })))
                    .child(if step == 3 {
                        Button::new("finish").label("Finish").primary().on_click(|_, window, _| window.remove_window())
                    } else {
                        Button::new("next").label("Next").primary().disabled(!can_next).shortcut("Enter").on_click(cx.listener(|this, _, _, cx| this.next(cx)))
                    }),
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
                .child(title_bar("Setup"))
                .child(
                    div().px_6().py_4().border_b_1().border_color(hsla(p.line)).child(
                        STEPS.iter().fold(steps("steps"), |s, l| s.step(*l)).current(step).on_select(cx.listener(|this, i: &usize, _, cx| {
                            // Going back is always allowed, but not out of a running install.
                            if this.step != 2 || this.progress >= TASKS.len() {
                                this.step = *i;
                                cx.notify();
                            }
                        })),
                    ),
                )
                .child(div().flex_1().min_h_0().p_6().child(interlace_in("page", step, page)))
                .child(footer),
        ))
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        ferrite_design::init(Appearance::Dark, cx);
        if let Some(scheme) = std::env::var("FERRITE_SCHEME").ok().and_then(|k| ferrite_design::schemes::by_key(&k)) {
            ferrite_design::theme::set_scheme(scheme, cx);
        }
        let mut options = chrome::window_options("Setup", size(px(720.), px(560.)), cx);
        options.is_resizable = false;
        cx.open_window(options, |window, cx| {
            chrome::square_corners(window);
            chrome::power_off_on_close(window, cx);
            cx.new(|cx| Wizard::new(window, cx))
        })
        .expect("failed to open the window");
        cx.activate(true);
    });
}
