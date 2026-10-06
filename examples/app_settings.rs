//! Template: a settings / preferences app (or the settings window of any
//! app).
//!
//! Sections in a sidebar, labelled fields on the right, and a draft that is
//! only applied when saved: an accent alert appears while there are unsaved
//! changes, with Save and Discard. Appearance and refresh rate apply live so
//! the user can see them. "Reset to defaults" confirms in a dialog.
//!
//!     cargo run --example app_settings

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ferrite_design::prelude::*;
use gpui::{
    App, AppContext as _, Context, Entity, IntoElement, Render, SharedString, Subscription, Window, div, px, size,
};

const CHANNELS: [&str; 3] = ["Stable", "Beta", "Nightly"];

/// Everything the user can change. `Clone + PartialEq` is what makes the
/// draft/saved comparison (and so the "unsaved changes" bar) free.
#[derive(Clone, PartialEq)]
struct Prefs {
    scheme: usize,
    appearance: usize,
    fps: usize,
    font_size: f64,
    word_wrap: bool,
    line_numbers: bool,
    auto_update: bool,
    channel: usize,
    check_hours: f64,
    telemetry: bool,
    crash_reports: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs {
            scheme: 0,
            appearance: 0,
            fps: motion::RATES.iter().position(|r| *r == motion::DEFAULT_FPS).unwrap_or(5),
            font_size: 13.,
            word_wrap: true,
            line_numbers: true,
            auto_update: true,
            channel: 0,
            check_hours: 6.,
            telemetry: false,
            crash_reports: true,
        }
    }
}

struct Settings {
    section: SharedString,
    saved: Prefs,
    draft: Prefs,
    name: Entity<TextInput>,
    confirm_reset: bool,
    toaster: Entity<Toaster>,
    _appearance: Subscription,
}

impl Settings {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name = cx.new(|cx| {
            let mut input = TextInput::new(window, cx).placeholder("display name");
            input.set_value("ryan", cx);
            input
        });
        Self {
            section: "general".into(),
            saved: Prefs::default(),
            draft: Prefs::default(),
            name,
            confirm_reset: false,
            toaster: cx.new(|_| Toaster::new()),
            _appearance: theme::follow_system(window),
        }
    }

    /// A setter for one draft field, for any controlled component.
    fn set<T: 'static>(cx: &mut Context<Self>, f: fn(&mut Prefs, &T)) -> impl Fn(&T, &mut Window, &mut App) + 'static {
        let weak = cx.weak_entity();
        move |v, window, cx| {
            let _ = weak.update(cx, |this, cx| {
                f(&mut this.draft, v);
                this.apply_live(window, cx);
                cx.notify();
            });
        }
    }

    /// Settings the user should *see* change before saving.
    fn apply_live(&self, window: &mut Window, cx: &mut App) {
        let pref = [Appearance::Dark, Appearance::Light, Appearance::System][self.draft.appearance];
        theme::set_appearance(pref, window, cx);
        theme::set_scheme(&SCHEMES[self.draft.scheme], cx);
        motion::set_fps(motion::RATES[self.draft.fps]);
    }

    fn general(&self, cx: &mut Context<Self>) -> gpui::Div {
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(field("name", "Display name").hint("shown in the title bar and on shared items").child(self.name.clone()))
            .child(field("updates", "Auto-update").child(switch("auto").label("Install updates on restart").checked(self.draft.auto_update).on_change(Self::set(cx, |p, v: &bool| p.auto_update = *v))))
            .child(
                field("channel", "Channel")
                    .hint("Nightly builds may break")
                    .child(select("channel-select").options(CHANNELS).selected(Some(self.draft.channel)).disabled(!self.draft.auto_update).on_change(Self::set(cx, |p, i: &usize| p.channel = *i))),
            )
            .child(
                field("interval", "Check every").child(
                    number_input("hours").value(self.draft.check_hours).range(1., 48.).digits(2).suffix("h").disabled(!self.draft.auto_update).on_change(Self::set(cx, |p, v: &f64| p.check_hours = *v)),
                ),
            )
    }

    fn appearance(&self, cx: &mut Context<Self>) -> gpui::Div {
        let rates = motion::RATES;
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                field("scheme", "Color scheme").hint(SCHEMES[self.draft.scheme].about).child(
                    SCHEMES.iter().fold(select("scheme-select"), |s, sc| s.option(sc.name)).selected(Some(self.draft.scheme)).on_change(Self::set(cx, |p, i: &usize| p.scheme = *i)),
                ),
            )
            .child(field("theme", "Mode").hint("applies immediately").child(segmented("theme-seg").option("Iron").option("Paper").option("System").selected(self.draft.appearance).on_select(Self::set(cx, |p, i: &usize| p.appearance = *i))))
            .child(
                field("fps", "Refresh").hint("25 is the classic stepped look").child(
                    rates.iter().fold(segmented("fps-seg"), |s, r| s.option(format!("{r}"))).selected(self.draft.fps).on_select(Self::set(cx, |p, i: &usize| p.fps = *i)),
                ),
            )
            .child(field("font", "Font size").child(number_input("font-size").value(self.draft.font_size).range(10., 20.).digits(2).suffix("px").on_change(Self::set(cx, |p, v: &f64| p.font_size = *v))))
    }

    fn editor(&self, cx: &mut Context<Self>) -> gpui::Div {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(checkbox("wrap").label("Word wrap").checked(self.draft.word_wrap).on_change(Self::set(cx, |p, v: &bool| p.word_wrap = *v)))
            .child(checkbox("numbers").label("Line numbers").checked(self.draft.line_numbers).on_change(Self::set(cx, |p, v: &bool| p.line_numbers = *v)))
    }

    fn privacy(&self, cx: &mut Context<Self>) -> gpui::Div {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(alert("privacy-note", "Nothing leaves this machine by default").message("Telemetry is anonymous usage counts; crash reports include a stack trace."))
            .child(switch("tele").label("Usage telemetry").checked(self.draft.telemetry).on_change(Self::set(cx, |p, v: &bool| p.telemetry = *v)))
            .child(switch("crash").label("Crash reports").checked(self.draft.crash_reports).on_change(Self::set(cx, |p, v: &bool| p.crash_reports = *v)))
    }
}

impl Render for Settings {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let dirty = self.draft != self.saved;
        let (title, body) = match self.section.as_ref() {
            "appearance" => ("Appearance", self.appearance(cx)),
            "editor" => ("Editor", self.editor(cx)),
            "privacy" => ("Privacy", self.privacy(cx)),
            _ => ("General", self.general(cx)),
        };

        let nav = sidebar("sections")
            .width(px(200.))
            .section("Settings")
            .item("general", "General", Icon::Sliders)
            .item("appearance", "Appearance", Icon::Dot)
            .item("editor", "Editor", Icon::File)
            .item("privacy", "Privacy", Icon::Lock)
            .selected(self.section.clone())
            .footer(Button::new("reset").label("Reset to defaults").small().ghost().full_width().on_click(cx.listener(|this, _, _, cx| {
                this.confirm_reset = true;
                cx.notify();
            })))
            .on_select(cx.listener(|this, key: &SharedString, _, cx| {
                this.section = key.clone();
                cx.notify();
            }));

        let unsaved = dirty.then(|| {
            alert("unsaved", "Unsaved changes")
                .accent()
                .action(Button::new("save").label("Save").small().primary().on_click(cx.listener(|this, _, _, cx| {
                    this.saved = this.draft.clone();
                    this.toaster.update(cx, |t, cx| {
                        t.push(toast("Settings saved").success(), cx);
                    });
                    cx.notify();
                })))
                .action(Button::new("discard").label("Discard").small().on_click(cx.listener(|this, _, window, cx| {
                    this.draft = this.saved.clone();
                    this.apply_live(window, cx);
                    cx.notify();
                })))
        });

        let page = scroll_area("page").flex_1().min_h_0().child(
            div()
                .flex()
                .flex_col()
                .gap_4()
                .p_6()
                .max_w(px(720.))
                .child(div().display(Scale::X2, window).text_color(hsla(p.fg)).child(decrypt("title", title.to_uppercase())))
                .children(unsaved)
                .child(wipe_in("section", &self.section, body)),
        );

        let reset = dialog("reset-dialog")
            .open(self.confirm_reset)
            .title("Reset all settings?")
            .description("Every section goes back to its default. Your display name is kept.")
            .danger()
            .confirm("Reset", {
                let weak = cx.weak_entity();
                move |window, cx| {
                    let _ = weak.update(cx, |this, cx| {
                        this.draft = Prefs::default();
                        this.saved = Prefs::default();
                        this.apply_live(window, cx);
                        cx.notify();
                    });
                }
            })
            .on_close({
                let weak = cx.weak_entity();
                move |_, cx| {
                    let _ = weak.update(cx, |this, cx| {
                        this.confirm_reset = false;
                        cx.notify();
                    });
                }
            });

        window_frame().child(power_on_in(
            "power",
            div()
                .flex()
                .flex_col()
                .size_full()
                .bg(hsla(p.bg))
                .text_color(hsla(p.fg))
                .body(text::BASE)
                .child(self.toaster.clone())
                .child(title_bar("Settings"))
                .child(div().flex().flex_row().flex_1().min_h_0().child(nav).child(page))
                .child(reset)
                .child(status_bar().left(if dirty { "MODIFIED" } else { "SAVED" }).right(format!("{}FPS", motion::fps()))),
        ))
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        ferrite_design::init(Appearance::Dark, cx);
        // FERRITE_SCHEME / FERRITE_APPEARANCE / FERRITE_FPS, for trying other looks.
        theme::apply_env(cx);
        let options = chrome::window_options("Settings", size(px(960.), px(680.)), cx);
        cx.open_window(options, |window, cx| {
            chrome::square_corners(window);
            chrome::power_off_on_close(window, cx);
            cx.new(|cx| Settings::new(window, cx))
        })
        .expect("failed to open the window");
        cx.activate(true);
    });
}
