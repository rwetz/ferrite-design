//! Dates: a [`Date`] type with the civil-calendar arithmetic a picker needs,
//! the month-grid [`calendar`], and the [`date_picker`] field that opens it.
//!
//! ```text
//!  [<]   OCTOBER 2026   [>]
//!   MO TU WE TH FR SA SU
//!   28 29 30  1  2  3  4      days outside the month: faint
//!    5 [6] 7  8  9 10 11      today: amber frame
//!   12 13 ██ 15 16 17 18      selected: inverse video (amber block)
//!   …
//! ```
//!
//! No date-library dependency: Gregorian arithmetic only (days from the
//! civil epoch, Howard Hinnant's algorithms), which is all a picker needs.
//! [`Date::today`] is the date in the system's local time zone; anything
//! finer about time zones is the app's business.
//!
//! Keyboard (calendar focused): arrows move the selection by a day or a
//! week, PageUp/PageDown by a month, Home/End to the month's ends. The shown
//! month follows the selection.

use std::rc::Rc;

use gpui::{
    App, ElementId, FocusHandle, InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels, RenderOnce,
    Role, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder as _, px,
};

use super::overlay::{Align, OverlayState, below, reveal, surface};
use crate::fonts::{FerriteText, Scale};
use crate::icon::{Icon, icon};
use crate::theme::palette;
use crate::tokens::{hsla, text};

// ── Date ──────────────────────────────────────────────────────────────────

/// Days since 1970-01-01 on the local calendar at `unix_secs`, in a zone
/// `offset_secs` east of UTC.
fn days_at(unix_secs: i64, offset_secs: i64) -> i64 {
    (unix_secs + offset_secs).div_euclid(86_400)
}

/// The zone offset implied by a local and a UTC reading of the same moment,
/// each as (date, seconds into the day). The two readings can straddle a
/// second, so the result is rounded to the nearest quarter hour, the
/// granularity of every real zone.
fn offset_between(local: (Date, i64), utc: (Date, i64)) -> i64 {
    let at = |(date, secs): (Date, i64)| date.days() * 86_400 + secs;
    let raw = at(local) - at(utc);
    (raw as f64 / 900.).round() as i64 * 900
}

/// The local zone's offset from UTC in seconds (east positive) at `unix_secs`,
/// daylight saving included; 0 if the system can't say.
#[cfg(windows)]
fn local_offset(_unix_secs: i64) -> i64 {
    use windows::Win32::Foundation::SYSTEMTIME;
    use windows::Win32::System::SystemInformation::{GetLocalTime, GetSystemTime};
    // SAFETY: both calls just read the clock.
    let (local, utc) = unsafe { (GetLocalTime(), GetSystemTime()) };
    let reading = |t: SYSTEMTIME| {
        (Date { year: t.wYear as i32, month: t.wMonth as u32, day: t.wDay as u32 }, t.wHour as i64 * 3600 + t.wMinute as i64 * 60 + t.wSecond as i64)
    };
    offset_between(reading(local), reading(utc))
}

#[cfg(unix)]
fn local_offset(unix_secs: i64) -> i64 {
    let t = unix_secs as libc::time_t;
    // SAFETY: localtime_r only writes the `tm` it's given, and is thread-safe.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    if unsafe { libc::localtime_r(&t, &mut tm) }.is_null() {
        return 0;
    }
    tm.tm_gmtoff as i64
}

#[cfg(not(any(windows, unix)))]
fn local_offset(_unix_secs: i64) -> i64 {
    0
}

/// A calendar date (proleptic Gregorian). Ordered, hashable, `Copy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub year: i32,
    /// 1–12.
    pub month: u32,
    /// 1–31.
    pub day: u32,
}

pub const MONTHS: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December",
];

impl Date {
    /// A date, with `month` and `day` clamped into range (Feb 31 → Feb 28/29).
    pub fn new(year: i32, month: u32, day: u32) -> Self {
        let month = month.clamp(1, 12);
        Date { year, month, day: day.clamp(1, days_in_month(year, month)) }
    }

    /// Today in the system's local time zone.
    pub fn today() -> Self {
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
        Date::from_days(days_at(secs, local_offset(secs)))
    }

    /// Days since 1970-01-01.
    pub fn days(self) -> i64 {
        let (m, d) = (self.month as i64, self.day as i64);
        let y = self.year as i64 - if m <= 2 { 1 } else { 0 };
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// The date `days` after 1970-01-01.
    pub fn from_days(days: i64) -> Self {
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let year = (yoe + era * 400 + if month <= 2 { 1 } else { 0 }) as i32;
        Date { year, month, day }
    }

    pub fn add_days(self, n: i64) -> Self {
        Date::from_days(self.days() + n)
    }

    /// The same day `n` months later, clamped to the month's length.
    pub fn add_months(self, n: i32) -> Self {
        let index = self.year * 12 + self.month as i32 - 1 + n;
        Date::new(index.div_euclid(12), index.rem_euclid(12) as u32 + 1, self.day)
    }

    /// 0 = Monday … 6 = Sunday.
    pub fn weekday(self) -> u32 {
        // 1970-01-01 was a Thursday (3).
        (self.days() + 3).rem_euclid(7) as u32
    }

    pub fn first_of_month(self) -> Self {
        Date { day: 1, ..self }
    }

    /// `2026-10-06`.
    pub fn iso(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl std::fmt::Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.iso())
    }
}

pub fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// The 42 days (6 weeks) a month grid shows for the month containing
/// `date`, starting on Monday (or Sunday).
pub fn month_grid(date: Date, sunday_first: bool) -> [Date; 42] {
    let first = date.first_of_month();
    let offset = if sunday_first { (first.weekday() + 1) % 7 } else { first.weekday() };
    let start = first.add_days(-(offset as i64));
    std::array::from_fn(|i| start.add_days(i as i64))
}

// ── Calendar ──────────────────────────────────────────────────────────────

type DateHandler = Rc<dyn Fn(&Date, &mut Window, &mut App)>;

/// A month grid. The selection is controlled (`.selected(..)`,
/// `on_select`); which month is *shown* is the calendar's own state, seeded
/// from the selection (or today) and moved by the arrows.
#[derive(IntoElement)]
pub struct Calendar {
    id: ElementId,
    selected: Option<Date>,
    today: Date,
    min: Option<Date>,
    max: Option<Date>,
    sunday_first: bool,
    on_select: Option<DateHandler>,
}

pub fn calendar(id: impl Into<ElementId>) -> Calendar {
    Calendar { id: id.into(), selected: None, today: Date::today(), min: None, max: None, sunday_first: false, on_select: None }
}

impl Calendar {
    pub fn selected(mut self, date: Option<Date>) -> Self {
        self.selected = date;
        self
    }

    /// Override "today" (tests, screenshots, a time-zone-aware app).
    pub fn today(mut self, date: Date) -> Self {
        self.today = date;
        self
    }

    /// Dates outside `min..=max` are shown faint and can't be chosen.
    pub fn range(mut self, min: Option<Date>, max: Option<Date>) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    pub fn week_starts_sunday(mut self) -> Self {
        self.sunday_first = true;
        self
    }

    pub fn on_select(mut self, handler: impl Fn(&Date, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

struct CalendarState {
    focus: FocusHandle,
    shown: Date,
    /// The selection last rendered, so a new one moves the shown month.
    last: Option<Date>,
}

impl RenderOnce for Calendar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let seed = self.selected.unwrap_or(self.today).first_of_month();
        let selected = self.selected;
        let state = window.use_keyed_state(self.id.clone(), cx, move |_, cx| CalendarState { focus: cx.focus_handle(), shown: seed, last: selected });
        if state.read(cx).last != selected {
            state.update(cx, |s, _| {
                s.last = selected;
                if let Some(d) = selected {
                    s.shown = d.first_of_month();
                }
            });
        }
        let (focus, shown) = {
            let s = state.read(cx);
            (s.focus.clone(), s.shown)
        };
        let (min, max) = (self.min, self.max);
        let allowed = move |d: Date| min.is_none_or(|m| d >= m) && max.is_none_or(|m| d <= m);
        let handler = self.on_select.clone();

        let title = format!("{} {}", MONTHS[shown.month as usize - 1].to_uppercase(), shown.year);
        let flip = crate::animate::play_on_change(ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), "flip".into()), shown, crate::motion::BASE, window, cx);
        let nav = |id: &'static str, glyph: Icon, by: i32| {
            let state = state.clone();
            div()
                .id(id)
                .role(Role::Button)
                .aria_label(if by < 0 { "Previous month" } else { "Next month" })
                .flex()
                .items_center()
                .justify_center()
                .size(px(24.))
                .border_1()
                .border_color(hsla(p.line))
                .hover(|s| s.bg(hsla(p.raised)))
                .child(icon(glyph).fit(px(16.)).color(hsla(p.fg_dim)))
                .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                .on_click(move |_, _, cx| {
                    state.update(cx, |s, cx| {
                        s.shown = s.shown.add_months(by);
                        cx.notify();
                    })
                })
        };
        let header = div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .gap_2()
            .child(nav("prev", Icon::ChevronLeft, -1))
            .child(div().display(Scale::X1, window).text_color(hsla(p.fg)).child(crate::animate::scramble(&title, flip)))
            .child(nav("next", Icon::ChevronRight, 1));

        let names = if self.sunday_first { ["SU", "MO", "TU", "WE", "TH", "FR", "SA"] } else { ["MO", "TU", "WE", "TH", "FR", "SA", "SU"] };
        let weekdays = div().flex().flex_row().children(names.map(|n| {
            div().w(px(32.)).flex().justify_center().body(text::XS).text_color(hsla(p.fg_faint)).child(n)
        }));

        // Picking a day (click or arrow keys) lands with a flash and a ring.
        let pick = crate::animate::play_on_change(
            ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), "pick".into()),
            selected,
            crate::motion::BASE,
            window,
            cx,
        );
        let days = month_grid(shown, self.sunday_first);
        let mut grid = div().flex().flex_col();
        for week in days.chunks(7) {
            let mut row = div().flex().flex_row();
            for &d in week {
                let in_month = d.month == shown.month;
                let ok = allowed(d);
                let (is_sel, is_today) = (Some(d) == selected, d == self.today);
                let handler = handler.clone().filter(|_| ok);
                let ink = if is_sel {
                    p.accent_fg
                } else if !ok {
                    p.line_strong
                } else if !in_month {
                    p.fg_faint
                } else {
                    p.fg
                };
                row = row.child(
                    div()
                        .id(ElementId::Integer(d.days() as u64))
                        .role(Role::Cell)
                        .aria_selected(is_sel)
                        .aria_label(SharedString::from(d.iso()))
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(32.))
                        .border_1()
                        .border_color(hsla(if is_today && !is_sel { p.accent } else { p.bg }))
                        .when(!is_today || is_sel, |el| el.border_color(gpui::transparent_black()))
                        .when(is_sel, |el| el.bg(hsla(p.accent)))
                        // The menu-row wash: it reads on any ground, including
                        // the date picker's raised surface.
                        .when(ok && !is_sel, |el| el.hover(|s| s.bg(hsla(p.accent_dim)).text_color(hsla(p.fg))))
                        .body(text::SM)
                        .text_color(hsla(ink))
                        .child(format!("{}", d.day))
                        .when(is_sel, |el| el.relative().children(landing(pick, p)))
                        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                        .when_some(handler, |el, h| el.on_click(move |_, window, cx| h(&d, window, cx))),
                );
            }
            grid = grid.child(row);
        }

        div()
            .id(self.id.clone())
            .role(Role::Grid)
            .track_focus(&focus)
            .tab_stop(true)
            .flex()
            .flex_col()
            .gap_2()
            .w(px(32. * 7. + 2.))
            .p(px(1.))
            .border_1()
            .border_color(gpui::transparent_black())
            .focus_visible(|s| s.border_color(hsla(p.accent)))
            .on_mouse_down(MouseButton::Left, {
                let focus = focus.clone();
                move |_, window, cx| focus.focus(window, cx)
            })
            .when_some(self.on_select.clone(), |el, h| {
                el.on_key_down(move |ev, window, cx| {
                    let from = selected.unwrap_or(shown);
                    let to = match ev.keystroke.key.as_str() {
                        "left" => from.add_days(-1),
                        "right" => from.add_days(1),
                        "up" => from.add_days(-7),
                        "down" => from.add_days(7),
                        "pageup" => from.add_months(-1),
                        "pagedown" => from.add_months(1),
                        "home" => from.first_of_month(),
                        "end" => Date::new(from.year, from.month, 31),
                        _ => return,
                    };
                    cx.stop_propagation();
                    if allowed(to) {
                        h(&to, window, cx);
                    }
                })
            })
            .child(header)
            .child(weekdays)
            .child(grid)
    }
}

/// The picked day landing: its block floods with ink and dissolves back
/// (a flash) while a dither ring steps out around it (a ping).
fn landing(pick: crate::animate::Progress, p: &crate::tokens::Palette) -> Vec<gpui::AnyElement> {
    use crate::dither::{self, dither};
    let mut out = Vec::new();
    if let Some(level) = crate::animate::flash_level(pick) {
        out.push(div().absolute().inset_0().child(dither(dither::flat(level)).ink(hsla(p.accent_fg)).size_full()).into_any_element());
    }
    if let Some((spread, level)) = crate::animate::ping_ring(pick) {
        let edge = || dither(dither::flat(level)).ink(hsla(p.accent)).size_full();
        let t = px(2.);
        out.push(
            div()
                .absolute()
                .top(-spread)
                .left(-spread)
                .right(-spread)
                .bottom(-spread)
                .child(div().absolute().top_0().left_0().right_0().h(t).child(edge()))
                .child(div().absolute().bottom_0().left_0().right_0().h(t).child(edge()))
                .child(div().absolute().top_0().bottom_0().left_0().w(t).child(edge()))
                .child(div().absolute().top_0().bottom_0().right_0().w(t).child(edge()))
                .into_any_element(),
        );
    }
    out
}

// ── Date picker ───────────────────────────────────────────────────────────

/// A form field that shows a date (`2026-10-06`) and opens a [`calendar`]
/// under it. Choosing a day closes it. Controlled, like the calendar.
#[derive(IntoElement)]
pub struct DatePicker {
    id: ElementId,
    selected: Option<Date>,
    placeholder: SharedString,
    width: Pixels,
    min: Option<Date>,
    max: Option<Date>,
    on_select: Option<DateHandler>,
}

pub fn date_picker(id: impl Into<ElementId>) -> DatePicker {
    DatePicker { id: id.into(), selected: None, placeholder: "YYYY-MM-DD".into(), width: px(200.), min: None, max: None, on_select: None }
}

impl DatePicker {
    pub fn selected(mut self, date: Option<Date>) -> Self {
        self.selected = date;
        self
    }

    pub fn placeholder(mut self, text: impl Into<SharedString>) -> Self {
        self.placeholder = text.into();
        self
    }

    pub fn width(mut self, width: Pixels) -> Self {
        self.width = width;
        self
    }

    pub fn range(mut self, min: Option<Date>, max: Option<Date>) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    pub fn on_select(mut self, handler: impl Fn(&Date, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for DatePicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let state = window.use_keyed_state(self.id.clone(), cx, |_, cx| OverlayState::new(cx));
        let open = state.read(cx).open;
        let well_id = ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), "well".into());
        let focus: FocusHandle = window.use_keyed_state(well_id.clone(), cx, |_, cx| cx.focus_handle()).read(cx).clone();
        let t = crate::animate::play_on_change(ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), "pick".into()), self.selected, crate::motion::FAST, window, cx);
        let label = match self.selected {
            Some(d) => crate::animate::scramble(&d.iso(), t),
            None => self.placeholder.to_string(),
        };
        let toggle = {
            let state = state.clone();
            move |window: &mut Window, cx: &mut App| {
                state.update(cx, |s, cx| {
                    if s.open == open {
                        if open { s.close(window, cx) } else { s.show(None, None, window, cx) }
                    }
                })
            }
        };
        let well = div()
            .id(well_id)
            .role(Role::ComboBox)
            .aria_expanded(open)
            .track_focus(&focus)
            .tab_stop(true)
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .w(self.width)
            .h(px(28.))
            .px_2()
            .bg(hsla(p.sunken))
            .border_1()
            .border_color(hsla(if open { p.accent } else { p.line_strong }))
            .hover(|s| s.border_color(hsla(p.fg_faint)))
            .focus_visible(|s| s.border_color(hsla(p.accent)))
            .child(icon(Icon::Calendar).fit(px(16.)).color(hsla(p.fg_dim)))
            .child(div().flex_1().body(text::BASE).text_color(hsla(if self.selected.is_some() { p.fg } else { p.fg_faint })).child(label))
            .on_mouse_down(MouseButton::Left, {
                let toggle = toggle.clone();
                move |_, window, cx| {
                    cx.stop_propagation();
                    window.prevent_default();
                    toggle(window, cx);
                }
            })
            .on_key_down({
                let state = state.clone();
                move |ev, window, cx| {
                    if !open && matches!(ev.keystroke.key.as_str(), "enter" | "space" | "down") {
                        cx.stop_propagation();
                        state.update(cx, |s, cx| s.show(None, None, window, cx));
                    }
                }
            });

        let mut root = div().id(self.id.clone()).relative().flex_none().child(well);
        if open {
            let overlay_focus = state.read(cx).focus.clone();
            let handler = self.on_select.clone();
            let closer = state.clone();
            let cal = calendar(ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), "cal".into()))
                .selected(self.selected)
                .range(self.min, self.max)
                .on_select(move |d, window, cx| {
                    if let Some(h) = &handler {
                        h(d, window, cx);
                    }
                    // Let the pick land before the popup goes: the day
                    // flashes in place, then the well decrypts the date.
                    let closer = closer.clone();
                    window
                        .spawn(cx, async move |cx| {
                            cx.background_executor().timer(crate::motion::BASE).await;
                            let _ = cx.update(|window, cx| closer.update(cx, |s, cx| s.close(window, cx)));
                        })
                        .detach();
                });
            let panel = div()
                .id("date-surface")
                .role(Role::Dialog)
                .track_focus(&overlay_focus)
                .occlude()
                .on_key_down({
                    let state = state.clone();
                    move |ev, window, cx| {
                        if ev.keystroke.key == "escape" {
                            cx.stop_propagation();
                            state.update(cx, |s, cx| s.close(window, cx));
                        }
                    }
                })
                .on_mouse_down_out({
                    let state = state.clone();
                    move |_, window, cx| state.update(cx, |s, cx| s.close(window, cx))
                })
                .child(reveal(surface(div().p_2().child(cal), cx), crate::motion::FAST, window, cx));
            root = root.child(below(Align::Start, panel));
        }
        root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_round_trip_across_eras() {
        assert_eq!(Date::new(1970, 1, 1).days(), 0);
        assert_eq!(Date::from_days(0), Date::new(1970, 1, 1));
        for days in [-800_000i64, -1, 0, 1, 59, 60, 11_016, 20_367, 2_932_896] {
            assert_eq!(Date::from_days(days).days(), days, "{days}");
        }
        assert_eq!(Date::new(2000, 2, 29).add_days(1), Date::new(2000, 3, 1));
        assert_eq!(Date::new(2026, 12, 31).add_days(1), Date::new(2027, 1, 1));
    }

    #[test]
    fn weekdays_match_the_real_calendar() {
        assert_eq!(Date::new(1970, 1, 1).weekday(), 3, "Thursday");
        assert_eq!(Date::new(2026, 10, 6).weekday(), 1, "Tuesday");
        assert_eq!(Date::new(2000, 1, 1).weekday(), 5, "Saturday");
    }

    #[test]
    fn months_clamp_and_leap_years_are_right() {
        assert_eq!(Date::new(2024, 1, 31).add_months(1), Date::new(2024, 2, 29));
        assert_eq!(Date::new(2023, 1, 31).add_months(1), Date::new(2023, 2, 28));
        assert_eq!(Date::new(2026, 1, 15).add_months(-1), Date::new(2025, 12, 15));
        assert_eq!(Date::new(2026, 3, 15).add_months(-14), Date::new(2025, 1, 15));
        assert!(is_leap(2000) && !is_leap(1900) && is_leap(2024) && !is_leap(2026));
        assert_eq!(Date::new(2026, 2, 31), Date::new(2026, 2, 28));
    }

    #[test]
    fn grids_start_on_the_right_weekday_and_cover_the_month() {
        // October 2026 starts on a Thursday.
        let g = month_grid(Date::new(2026, 10, 20), false);
        assert_eq!(g[0], Date::new(2026, 9, 28), "Monday before the 1st");
        assert_eq!(g[3], Date::new(2026, 10, 1));
        assert!(g.contains(&Date::new(2026, 10, 31)));
        let g = month_grid(Date::new(2026, 10, 20), true);
        assert_eq!(g[0], Date::new(2026, 9, 27), "Sunday before the 1st");
        for w in g.windows(2) {
            assert_eq!(w[0].add_days(1), w[1]);
        }
    }

    #[test]
    fn iso_is_zero_padded() {
        assert_eq!(Date::new(2026, 3, 7).iso(), "2026-03-07");
    }

    /// Unix seconds for a UTC date and time.
    fn utc(date: Date, h: i64, m: i64) -> i64 {
        date.days() * 86_400 + h * 3600 + m * 60
    }

    #[test]
    fn today_is_the_local_date() {
        let (oct6, oct7) = (Date::new(2026, 10, 6), Date::new(2026, 10, 7));
        // 20:00 on the 6th in UTC-5 is already 01:00 on the 7th in UTC.
        assert_eq!(Date::from_days(days_at(utc(oct7, 1, 0), -5 * 3600)), oct6);
        // 03:00 on the 7th in UTC+9 is still 18:00 on the 6th in UTC.
        assert_eq!(Date::from_days(days_at(utc(oct6, 18, 0), 9 * 3600)), oct7);
        // At UTC it's plain division, including before the epoch.
        assert_eq!(days_at(utc(oct6, 12, 0), 0), oct6.days());
        assert_eq!(Date::from_days(days_at(-1, 0)), Date::new(1969, 12, 31));
    }

    #[test]
    fn offsets_round_to_the_quarter_hour() {
        let d = Date::new(2026, 10, 6);
        // 19:59:59 in UTC-5, read a second before UTC ticks over to 01:00:00.
        assert_eq!(offset_between((d, 19 * 3600 + 59 * 60 + 59), (d.add_days(1), 3600)), -5 * 3600);
        // Nepal, UTC+5:45.
        assert_eq!(offset_between((d, 5 * 3600 + 45 * 60), (d, 0)), 5 * 3600 + 45 * 60);
        assert_eq!(offset_between((d, 3600), (d, 3600)), 0);
    }
}
