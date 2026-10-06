//! Template: a records explorer — the shape of an admin panel, a CRM, an
//! issue tracker, a database browser.
//!
//! A toolbar with a live search field and a status filter, a sortable
//! table of the current page, pagination, an empty state when nothing
//! matches, and a details drawer for the selected record.
//!
//!     cargo run --example app_explorer
//!
//! Replace `records()` with your query; keep filtering, sorting and paging
//! in `visible()` so the view stays a pure function of the state.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ferrite_design::prelude::*;
use gpui::{
    App, AppContext as _, Context, Entity, IntoElement, Render, Subscription, Window, div, px, size,
};

const STATUSES: [&str; 4] = ["All", "Open", "Blocked", "Done"];
const PER_PAGE: usize = 12;

#[derive(Clone)]
struct Record {
    id: u32,
    title: String,
    owner: &'static str,
    status: &'static str,
    updated: Date,
}

/// Stand-in for your data: 200 deterministic records.
fn records() -> Vec<Record> {
    let words = ["dither", "palette", "chrome", "toast", "input", "raster", "motion", "tree", "table", "fonts"];
    let verbs = ["Fix", "Port", "Document", "Profile", "Refactor", "Test"];
    let owners = ["ryan", "ada", "grace", "linus", "barbara"];
    (0..200u32)
        .map(|i| Record {
            id: 1000 + i,
            title: format!("{} {} {}", verbs[i as usize % verbs.len()], words[(i as usize * 7) % words.len()], ["cache", "layout", "keys", "focus"][i as usize % 4]),
            owner: owners[(i as usize * 3) % owners.len()],
            status: ["Open", "Blocked", "Done", "Open"][(i as usize * 5) % 4],
            updated: Date::new(2026, 10, 6).add_days(-((i as i64 * 37) % 120)),
        })
        .collect()
}

struct Explorer {
    all: Vec<Record>,
    search: Entity<TextInput>,
    status: usize,
    sort: Option<(usize, SortDir)>,
    page: usize,
    selected: Option<u32>,
    _subs: Vec<Subscription>,
}

impl Explorer {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| TextInput::new(window, cx).placeholder("search title or owner…").prompt(">"));
        // Typing filters live and goes back to page one.
        let sub = cx.subscribe(&search, |this: &mut Self, _, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::Change) {
                this.page = 0;
                cx.notify();
            }
        });
        Self { all: records(), search, status: 0, sort: Some((4, SortDir::Desc)), page: 0, selected: None, _subs: vec![sub, theme::follow_system(window)] }
    }

    /// Filter, sort, and return (total matches, this page).
    fn visible(&self, cx: &App) -> (usize, Vec<Record>) {
        let query = self.search.read(cx).value().to_lowercase();
        let mut rows: Vec<Record> = self
            .all
            .iter()
            .filter(|r| self.status == 0 || r.status == STATUSES[self.status])
            .filter(|r| query.is_empty() || r.title.to_lowercase().contains(&query) || r.owner.contains(&query))
            .cloned()
            .collect();
        if let Some((col, dir)) = self.sort {
            rows.sort_by(|a, b| {
                let o = match col {
                    0 => a.id.cmp(&b.id),
                    1 => a.title.cmp(&b.title),
                    2 => a.owner.cmp(b.owner),
                    3 => a.status.cmp(b.status),
                    _ => a.updated.cmp(&b.updated),
                };
                if dir == SortDir::Desc { o.reverse() } else { o }
            });
        }
        let total = rows.len();
        let page = rows.into_iter().skip(self.page * PER_PAGE).take(PER_PAGE).collect();
        (total, page)
    }
}

impl Render for Explorer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let (total, rows) = self.visible(cx);
        let pages = total.div_ceil(PER_PAGE).max(1);
        let selected_row = self.selected.and_then(|id| rows.iter().position(|r| r.id == id));
        let detail = self.selected.and_then(|id| self.all.iter().find(|r| r.id == id)).cloned();

        let bar = toolbar()
            .child(div().w(px(320.)).child(self.search.clone()))
            .child(select("status").options(STATUSES).selected(Some(self.status)).width(px(140.)).on_change(cx.listener(|this, i: &usize, _, cx| {
                this.status = *i;
                this.page = 0;
                cx.notify();
            })))
            .spacer()
            .child(div().display(Scale::X1, window).text_color(hsla(p.fg_dim)).child(format!("{total} RECORDS")));

        let content = if rows.is_empty() {
            div().flex_1().child(empty_state("No matches", "Try a shorter search, or another status.", window, cx)).into_any_element()
        } else {
            let tone = |s: &str| match s {
                "Blocked" => Tone::Danger,
                "Done" => Tone::Success,
                _ => Tone::Accent,
            };
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    table("records")
                        .column(column("ID").width(px(72.)).align_right().sortable())
                        .column(column("Title").sortable())
                        .column(column("Owner").width(px(100.)).sortable())
                        .column(column("Status").width(px(96.)).sortable())
                        .column(column("Updated").width(px(112.)).align_right().sortable())
                        .rows(rows.iter().map(|r| vec![format!("#{}", r.id), r.title.clone(), r.owner.to_string(), r.status.to_string(), r.updated.iso()]))
                        .sort(self.sort)
                        .selected(selected_row)
                        .on_sort(cx.listener(|this, s: &(usize, SortDir), _, cx| {
                            this.sort = Some(*s);
                            cx.notify();
                        }))
                        .on_select({
                            let ids: Vec<u32> = rows.iter().map(|r| r.id).collect();
                            cx.listener(move |this, i: &usize, _, cx| {
                                this.selected = ids.get(*i).copied();
                                cx.notify();
                            })
                        }),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .child(div().flex().flex_row().gap_2().children(["Open", "Blocked", "Done"].map(|s| tag(s).tone(tone(s)).outline())))
                        .child(pagination("pages").total(pages).current(self.page.min(pages - 1)).on_change(cx.listener(|this, i: &usize, _, cx| {
                            this.page = *i;
                            cx.notify();
                        }))),
                )
                .into_any_element()
        };

        let details = drawer("record")
            .open(detail.is_some())
            .title(detail.as_ref().map(|r| format!("#{}", r.id)).unwrap_or_default())
            .on_close({
                let weak = cx.weak_entity();
                move |_, cx| {
                    let _ = weak.update(cx, |this, cx| {
                        this.selected = None;
                        cx.notify();
                    });
                }
            })
            .when_some(detail, |d, r| {
                d.child(div().body(text::LG).text_color(hsla(p.fg)).child(r.title.clone()))
                    .child(property_list().row_with("owner", div().flex().flex_row().items_center().gap_2().child(avatar(r.owner).size(px(20.))).child(r.owner)).row("status", r.status).row("updated", r.updated.iso()))
                    .child(rule(Some("activity"), window, cx))
                    .child(timeline("activity").time_width(px(80.)).event(event(r.updated.iso(), format!("status → {}", r.status)).tone(Tone::Accent)).event(event(r.updated.add_days(-3).iso(), "created")))
            })
            .footer(Button::new("close-record").label("Close").small().shortcut("Esc").on_click(cx.listener(|this, _, _, cx| {
                this.selected = None;
                cx.notify();
            })));

        window_frame().child(power_on_in(
            "power",
            div()
                .flex()
                .flex_col()
                .size_full()
                .bg(hsla(p.bg))
                .text_color(hsla(p.fg))
                .body(text::BASE)
                .child(title_bar("Explorer"))
                .child(bar)
                .child(scroll_area("body").flex_1().min_h_0().child(div().p(space::ROW).child(content)))
                .child(details)
                .child(status_bar().left(format!("PAGE {}/{}", self.page.min(pages - 1) + 1, pages)).right(format!("{total} OF {}", self.all.len()))),
        ))
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        ferrite_design::init(Appearance::Dark, cx);
        // FERRITE_SCHEME / FERRITE_APPEARANCE / FERRITE_FPS, for trying other looks.
        theme::apply_env(cx);
        let options = chrome::window_options("Explorer", size(px(1100.), px(760.)), cx);
        cx.open_window(options, |window, cx| {
            chrome::square_corners(window);
            chrome::power_off_on_close(window, cx);
            cx.new(|cx| Explorer::new(window, cx))
        })
        .expect("failed to open the window");
        cx.activate(true);
    });
}
