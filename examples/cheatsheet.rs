//! Compile-only guard for the API cheat sheet in AGENTS.md §3: every
//! builder chain there appears here. If a component's API changes, this
//! stops building and the cheat sheet gets fixed in the same commit.
//! It renders nonsense; there is nothing to run.

use ferrite_design::prelude::*;
use gpui::{App, Context, Entity, IntoElement, Render, SharedString, Window, div, px};
#[allow(dead_code)]
struct V { input: Entity<TextInput>, toaster: Entity<Toaster>, x: usize, b: bool, f: f64, v: f32 }
impl Render for V {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let w = cx.weak_entity();
        let h = { let w = w.clone(); move |_: &mut Window, cx: &mut App| { let _ = w.update(cx, |this, cx| { this.b = false; cx.notify(); }); } };
        let i = self.x; let b = self.b;
        let values = vec![1f32, 2., 3.];
        let picture = dither::Picture::from_fn(8, 8, |u, _| u);
        let rows_of_strings = vec![vec!["a".to_string(), "1".into()]];
        let _ = self.input.read(cx).value();
        self.toaster.update(cx, |t, cx| { t.push(toast("Saved").success().message("…").action("Undo", h.clone()), cx); });
        div()
            .child(Button::new("b").label("Run").icon(Icon::Play).primary().small().shortcut("Ctrl+R").tooltip("…").loading(b).selected(b).disabled(b).full_width().on_click(cx.listener(|this, _ev: &gpui::ClickEvent, _w, cx| { this.x += 1; cx.notify(); })))
            .child(checkbox("c").label("…").checked(b).indeterminate(b).on_change(cx.listener(|this, v: &bool, _, cx| { this.b = *v; cx.notify(); })))
            .child(radio("r").label("x").checked(b)).child(switch("s").label("x").checked(b))
            .child(segmented("sg").option("A").option_with_icon("B", Icon::Menu).selected(i).on_select(cx.listener(|this, v: &usize, _, cx| { this.x = *v; cx.notify(); })))
            .child(slider("sl").range(0., 100.).step(5.).value(self.v).format(|v| format!("{v:.0}%").into()).on_change(cx.listener(|this, v: &f32, _, _| this.v = *v)))
            .child(tag("live").accent().outline()).child(meter(0.6).label("cpu").id("cpu")).child(kbd("Ctrl+Shift+P")).child(spinner("sp")).child(cursor("cu")).child(progress_bar(0.4, px(16.), cx))
            .child(field("fi", "Name").required().hint("…").error(Some("…")).stacked().child(self.input.clone()))
            .child(select("se").options(["a", "b"]).selected(Some(i)).placeholder("Choose…").width(px(220.)).on_change(cx.listener(|this, v: &usize, _, _| this.x = *v)))
            .child(number_input("ni").value(self.f).range(1., 64.).step(1.).digits(3).decimals(0).suffix("ms").on_change(cx.listener(|this, v: &f64, _, _| this.f = *v)))
            .child(date_picker("dp").selected(Some(Date::new(2026, 10, 6))).range(Some(Date::today()), None).on_select(cx.listener(|_, d: &Date, _, _| { let _ = (d.add_days(1), d.add_months(1), d.iso(), d.weekday()); })))
            .child(calendar("ca").selected(None).range(None, None).on_select(|_, _, _| {}))
            .child(dropdown_menu("dm").trigger(Button::new("t")).item(menu_item("Open").icon(Icon::Folder).shortcut("Ctrl+O").on_select(h.clone())).submenu(submenu("Recent").item(menu_item("x"))).separator().label("View").item(menu_item("Wrap").checked(b)))
            .child(context_menu("cm").item(menu_item("x")).child(div()))
            .child(popover("po").title("Filters").trigger(Button::new("pt")).content(|_window, _cx| div().into_any_element()))
            .child(dialog("di").open(b).title("Delete?").description("…").danger().confirm("Delete", h.clone()).cancel("Keep").on_close(h.clone()))
            .child(drawer("dr").open(b).title("Details").left().width(px(380.)).child(div()).footer(Button::new("f")).on_close(h.clone()))
            .child(sidebar("sb").brand("App").section("Workspace").item("key", "Label", Icon::Home).item_with_meta("logs", "Logs", Icon::Terminal, "12").selected("key").collapsed(b).footer(div()).on_select(cx.listener(|_, k: &SharedString, _, _| { let _ = k; })))
            .child(toolbar().child(div()).separator().spacer().child(div()))
            .child(tabs("ta").tab("Logs").tab_with_meta("Jobs", "3").selected(i).on_select(|_, _, _| {}))
            .child(breadcrumb("bc").crumbs(["src", "app.rs"]).on_select(|_, _, _| {}))
            .child(pagination("pg").total(20).current(i).on_change(|_, _, _| {})).child(steps("st").step("Account").step("Done").current(i).on_select(|_, _, _| {}))
            .child(accordion("ac").open(["General"]).single().section(accordion_section("General").meta("3").child(div())))
            .child(list_item("li", "name.rs").icon(Icon::File).meta("2 KB").selected(b).on_click(|_, _, _| {}))
            .child(tree("tr").nodes([tree_node("src", "src/").icon(Icon::Folder).children([tree_node("src/a.rs", "a.rs")])]).expanded(["src"]).selected(Some("src")).on_select(|_, _, _| {}))
            .child(table("tb").column(column("Name").sortable()).column(column("CPU").width(px(80.)).align_right().sortable()).rows(rows_of_strings).sort(Some((0, SortDir::Asc))).selected(Some(i)).on_sort(|_, _, _| {}).on_select(|_, _, _| {}))
            .child(virtual_list("vl", 10, move |range, _window, _cx| range.map(|i| div().child(format!("{i}")).into_any_element()).collect()).size_full())
            .child(scroll_area("sa").flex_1().min_h_0().child(div())).child(split("sp").initial(0.25).min(px(180.)).first(div()).second(div()))
            .child(responsive("rs", |room, _, _| div().w(room.width / whole_scale(room, gpui::size(px(400.), px(100.)), 4) as f32).into_any_element()).flex_1())
            .child(property_list().row("pid", "4412").row_with("state", tag("run").accent()))
            .child(stat("st", "Requests", "18.2K").delta(4.2).lower_is_better().trend(values.clone()).caption("vs last week"))
            .child(avatar("Ada Lovelace").initials().size(px(32.)).presence(Presence::Online))
            .child(timeline("tl").time_width(px(80.)).event(event("12:04", "Deployed").detail("…").tone(Tone::Success)))
            .child(alert("al", "Disk almost full").warning().message("…").action(Button::new("a")).on_close(h.clone()))
            .child(skeleton("sk").h(px(64.)).w_full()).child(skeleton_text("skt", 3, px(10.))).child(empty_state("No matches", "hint", window, cx))
            .child(line_chart("lc", values.clone()).title("Latency").compare(values.clone()).labels(["a", "b", "c"]).format(|v| format!("{v:.0}MS")).height(px(180.)))
            .child(bar_chart("bc2").bars([("Mon", 12.), ("Tue", 19.)]).highlight(Some(1))).child(sparkline(values.clone()).size(px(120.), px(20.))).child(heatmap(vec![vec![0.1, 0.5]]).row_labels(["Mon"]))
            .child(panel("Title").meta("12 items").flex_1().child(div())).child(rule(Some("section"), window, cx))
            .child(div().display(Scale::X1, window).text_color(hsla(p.fg_dim)).child("LABEL")).child(div().body(text::SM).child("x"))
            .child(icon(Icon::Search).fit(px(16.)).color(hsla(p.accent))).child(dither(dither::flat(dither::level::LIGHT)).ink(hsla(p.line)).size_full())
            .child(decrypt("d1", "x")).child(typewriter("d2", "x")).child(count_up("d3", 1., |v| format!("{v:.0}"))).child(afterglow("d4", "x"))
            .child(shake("e1", i, div())).child(tear("e2", i, || div())).child(flash("e3", i, div())).child(ping("e4", i, div())).child(scan("e5", i, div()))
            .child(unroll_in("e6", i, div())).child(wipe_in("e7", i, div()).from_right()).child(interlace_in("e8", i, div())).child(dissolve("e9", i, div()))
            .child(develop("e10", i, dither(picture))).child(cascade_in("e11", i).flex().flex_col().children([div()])).child(power_on_in("e12", div()))
            .child(Button::new("sch").on_click(|_, window, cx| { theme::set_scheme(&SCHEMES[1], cx); let _ = schemes::by_key("mono"); theme::set_appearance(Appearance::Light, window, cx); motion::set_fps(25); }))
            .child(Button::new("ic").icon(Icon::ChevronUp)).child(icon(Icon::Lock))
            .child(ascii_box().title("System").double().ink(hsla(p.accent)).child(div())).child(ascii_rule(Some("logs")).double())
            .child(banner("bn", "FERRITE")).child(ascii_art(dither::Picture::from_fn(4, 4, |u, _| u)).cols(48).ramp(&ascii::BUBBLES)).child(ascii_gauge(0.42).label("cpu").cells(12))
            .child(spinner("sp2").frames(ascii::spinners::DOTS)).child(mark()).child(ascii::gauge(0.4, 10)).child(ascii::box_edge(ascii::SINGLE, 20, true, Some("t")))
            .child(ascii_box().style(ascii::DOUBLE_H).shadow().draw_on("ab", 0).child(banner("bn2", "F").shadow()))
            .child(ascii_table().header(["name", "pid"]).row(["cargo", "9021"]).selected(Some(0))).child(ascii_tree().item(0, "src/").item(1, "lib.rs"))
            .child(ascii_plot(vec![1., 2., 3.]).size(48, 8).format(|v| format!("{v:.0}MS"))).child(ascii_bars().bar("mon", 12.).cells(24)).child(ascii_cal(2026, 10).today(Some(6)))
            .child(marquee("mq", "NOW PLAYING").cells(32)).child(ascii_button("ok", "Ok").primary().on_click(|_, _, _| {})).child(ascii_list("al").item("Quick").selected(Some(0)).on_select(|_, _, _| {}))
            .child(ascii::table(&["a"], &[vec!["1".into()]], ascii::PLAIN).join("
")).child(ascii::tree(&[(0, "src/")]).len().to_string()).child(ascii::plot(&[1., 2.], 8, 4).join("
"))
            .child(ascii::cal(2026, 10, false).join("
"))
            .child(ascii_film("film", vec![dither::Picture::from_fn(4, 4, |u, _| u)]).cols(64).charset(ascii::Charset::Full).fps(12))
            .child(ascii_art(dither::Picture::from_fn(4, 4, |u, _| u)).cols(120).charset(ascii::Charset::Full).fit(ascii::Fit::Shape).contrast(1.5).invert(false).diffuse(false)).child(ascii::frame(&["x"], ascii::PLAIN, Some("t")).join("
"))
    }
}
fn main() {
    eprintln!("cheatsheet is a compile check; see AGENTS.md");
}
