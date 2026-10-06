//! Ferrite's own structural components.
//!
//! Ferrite's whole component set (docs/COMPONENTS.md): controls, overlays,
//! the text input, scrolling and layout in the submodules, and the framing
//! (`Panel`, `StatusBar`, `rule`, …) here.

pub mod button;
pub mod calendar;
pub mod chart;
pub mod dialog;
pub mod disclosure;
pub mod display;
pub mod drawer;
pub mod feedback;
pub mod form;
pub mod fx;
pub mod input;
pub mod list;
pub mod menu;
pub mod nav;
pub mod overlay;
pub mod palette;
pub mod scroll;
pub mod segmented;
pub mod slider;
pub mod split;
pub mod table;
pub mod tabs;
pub mod tag;
pub mod ticker;
pub mod toast;
pub mod toggle;
pub mod tree;
pub mod tooltip;

pub use button::Button;
pub use calendar::{Calendar, Date, DatePicker, calendar, date_picker};
pub use chart::{BarChart, Heatmap, LineChart, Sparkline, bar_chart, heatmap, line_chart, sparkline};
pub use dialog::{Dialog, dialog};
pub use display::{Avatar, Presence, PropertyList, Stat, Timeline, TimelineEvent, avatar, event, property_list, stat, timeline};
pub use drawer::{Drawer, drawer};
pub use disclosure::{Accordion, AccordionSection, accordion, accordion_section};
pub use feedback::{Alert, Skeleton, alert, skeleton, skeleton_text};
pub use form::{Field, NumberInput, Select, field, number_input, select};
pub use fx::{
    afterglow, cascade_in, count_up, decrypt, develop, dissolve, flash, interlace_in, ping, power_on_in, scan, shake,
    tear, typewriter, unroll_in, wipe_in,
};
pub use input::{InputEvent, TextInput};
pub use list::{ListItem, list_item};
pub use menu::{Menu, MenuItem, Submenu, context_menu, dropdown_menu, menu_item, submenu};
pub use nav::{Breadcrumb, Pagination, Sidebar, Steps, Toolbar, breadcrumb, pagination, sidebar, steps, toolbar};
pub use overlay::{Align, Popover, popover};
pub use palette::{CommandPalette, PaletteCommand, TogglePalette, command};
pub use segmented::{Segmented, segmented};
pub use scroll::{ScrollArea, Scrollbar, VirtualList, scroll_area, scrollbar, virtual_list};
pub use slider::{Slider, slider};
pub use split::{Split, split};
pub use table::{Column, SortDir, Table, column, table};
pub use tabs::{Tabs, tabs};
pub use tag::{Meter, Tag, meter, tag};
pub use toggle::{Switch, Toggle, checkbox, radio, switch};
pub use ticker::{Cursor, Spinner, cursor, spinner, ticker};
pub use toast::{Toast, ToastId, ToastKind, Toaster, toast};
pub use tooltip::{Kbd, Tooltip, kbd, tooltip};
pub use tree::{Tree, TreeNode, tree, tree_node};

use gpui::{
    AnyElement, App, IntoElement, ParentElement, Pixels, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _, px, relative,
};

use crate::dither::{self, dither};
use crate::fonts::{FerriteText, Scale};
use crate::theme::palette;
use crate::tokens::{hsla, text};

// ── Panel ─────────────────────────────────────────────────────────────────

/// A framed region with a `[ TITLE ]` header and a dithered header fill —
/// the basic Ferrite container. Hard 1px frame, no radius, no shadow.
#[derive(IntoElement)]
pub struct Panel {
    title: SharedString,
    meta: Option<SharedString>,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

pub fn panel(title: impl Into<SharedString>) -> Panel {
    Panel { title: title.into(), meta: None, children: Vec::new(), style: StyleRefinement::default() }
}

impl Panel {
    /// Right-aligned header text (a count, a status, a shortcut).
    pub fn meta(mut self, meta: impl Into<SharedString>) -> Self {
        self.meta = Some(meta.into());
        self
    }
}

impl ParentElement for Panel {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Panel {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Panel {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        // Headers boot when a panel first appears.
        let boot = crate::animate::play(gpui::ElementId::Name(format!("panel-{}", self.title).into()), 0u8, crate::motion::SLOW, window, cx);
        // Caller's style (size, flex) first, the frame on top — assigning the
        // style afterwards would wipe the frame (PITFALLS §35).
        let mut root = div();
        *root.style() = self.style;
        root.flex()
            .flex_col()
            .bg(hsla(p.bg))
            .border_1()
            .border_color(hsla(p.line))
            .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .h(px(24.))
                .px_2()
                .bg(hsla(p.surface))
                .border_b_1()
                .border_color(hsla(p.line))
                .child(
                    div()
                        .display(Scale::X1, window)
                        .text_color(hsla(p.fg))
                        .child(crate::animate::scramble(&format!("[ {} ]", self.title.to_uppercase()), boot)),
                )
                // The rule draws on left→right as the title locks in.
                .child(div().flex_1().h(px(8.)).child(dither(dither::flat(dither::level::LIGHT)).ink(hsla(p.line_strong)).h_full().w(relative(boot.eased()))))
                .when_some(self.meta, |this, meta| {
                    this.child(div().display(Scale::X1, window).text_color(hsla(p.fg_dim)).child(meta))
                }),
        )
        .child(div().flex().flex_col().flex_1().min_h_0().p_3().gap_2().children(self.children))
    }
}

// ── Selection ─────────────────────────────────────────────────────────────

/// The selected-row treatment, animated: the 2px amber bar lands at once and
/// the `accent_dim` wash sweeps in left→right behind the content when a row
/// becomes selected. Add it as the row's *first* child (it's absolute, so
/// content paints over it). `key` must be unique among sibling rows.
pub(crate) fn selection(key: impl Into<SharedString>, selected: bool, window: &mut Window, cx: &mut App) -> Option<gpui::Div> {
    let p = palette(cx);
    let key: SharedString = key.into();
    let t = crate::animate::play_on_change(gpui::ElementId::Name(format!("sel-{key}").into()), selected, crate::motion::BASE, window, cx).eased();
    selected.then(|| {
        div()
            .absolute()
            .left_0()
            .top_0()
            .bottom_0()
            .w(relative(t))
            .bg(hsla(p.accent_dim))
            .child(div().absolute().left_0().top_0().bottom_0().w(px(2.)).bg(hsla(p.accent)))
    })
}

// ── Rule ──────────────────────────────────────────────────────────────────

/// A 1px horizontal divider, optionally labelled: `── LABEL ─────────`.
pub fn rule(label: Option<&str>, window: &Window, cx: &App) -> impl IntoElement {
    let p = palette(cx);
    let line = || div().h(px(1.)).bg(hsla(p.line));
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_2()
        .w_full()
        .child(line().w(px(16.)))
        .when_some(label.map(str::to_uppercase), |this, label| {
            this.child(div().display(Scale::X1, window).text_color(hsla(p.fg_dim)).child(label))
        })
        .child(line().flex_1())
}

// ── Status bar ────────────────────────────────────────────────────────────

/// The bottom bar. Segments are separated by `│` in the faint ink.
///
/// A segment decrypts into place when its text changes — right for status
/// that changes as *events* (`READY` → `SAVED`). For values that tick
/// continuously (a line count, a clock, a frame rate) use [`StatusBar::left_live`]
/// / [`StatusBar::right_live`]: they update instantly, so a busy counter
/// doesn't keep the window animating (DESIGN_LANGUAGE §6.3).
#[derive(IntoElement)]
pub struct StatusBar {
    left: Vec<(SharedString, bool)>,
    right: Vec<(SharedString, bool)>,
}

pub fn status_bar() -> StatusBar {
    StatusBar { left: Vec::new(), right: Vec::new() }
}

impl StatusBar {
    pub fn left(mut self, segment: impl Into<SharedString>) -> Self {
        self.left.push((segment.into(), true));
        self
    }

    pub fn right(mut self, segment: impl Into<SharedString>) -> Self {
        self.right.push((segment.into(), true));
        self
    }

    /// A segment for live data: no decrypt on change.
    pub fn left_live(mut self, segment: impl Into<SharedString>) -> Self {
        self.left.push((segment.into(), false));
        self
    }

    /// A segment for live data: no decrypt on change.
    pub fn right_live(mut self, segment: impl Into<SharedString>) -> Self {
        self.right.push((segment.into(), false));
        self
    }
}

impl RenderOnce for StatusBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        // Each segment decrypts into place whenever its text changes.
        let segments = |side: &str, items: Vec<(SharedString, bool)>, window: &mut Window, cx: &mut App| {
            let mut row = div().flex().flex_row().items_center().gap_2();
            for (i, (item, decrypts)) in items.into_iter().enumerate() {
                if i > 0 {
                    row = row.child(div().display(Scale::X1, window).text_color(hsla(p.fg_faint)).child("│"));
                }
                let shown = if decrypts {
                    let t = crate::animate::play_on_change(gpui::ElementId::Name(format!("status-{side}-{i}").into()), &item, crate::motion::BASE, window, cx);
                    crate::animate::scramble(&item, t)
                } else {
                    item.to_string()
                };
                row = row.child(div().display(Scale::X1, window).child(shown));
            }
            row
        };
        let left = segments("l", self.left, window, cx);
        let right = segments("r", self.right, window, cx);
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .flex_shrink_0()
            .h(px(24.))
            .px_2()
            .bg(hsla(p.surface))
            .border_t_1()
            .border_color(hsla(p.line))
            .text_color(hsla(p.fg_dim))
            .child(left)
            .child(right)
    }
}

// ── Progress ──────────────────────────────────────────────────────────────

/// A progress bar: a solid accent fill with a ▓▒░ dithered leading edge.
/// The edge is a fixed-size cached dither, so a moving bar costs two quads.
pub fn progress_bar(value: f32, height: Pixels, cx: &App) -> impl IntoElement {
    let p = palette(cx);
    let value = value.clamp(0.0, 1.0);
    div()
        .relative()
        .overflow_hidden()
        .w_full()
        .h(height)
        .bg(hsla(p.sunken))
        .border_1()
        .border_color(hsla(p.line))
        .child(div().absolute().top_0().bottom_0().left_0().w(relative(value)).bg(hsla(p.accent)))
        .child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(relative(value))
                .w(px(32.))
                .child(dither(dither::horizontal(1.0, 0.0)).ink(hsla(p.accent)).size_full()),
        )
}

// ── Empty state ───────────────────────────────────────────────────────────

/// A content-free area: radial dither behind a short display-face message.
pub fn empty_state(title: &str, hint: &str, window: &Window, cx: &App) -> impl IntoElement {
    let p = palette(cx);
    div()
        .relative()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_2()
        .size_full()
        .min_h(px(160.))
        .child(
            div().absolute().inset_0().child(
                dither(dither::radial(0.0, dither::level::MEDIUM)).ink(hsla(p.line)).size_full(),
            ),
        )
        .child(
            div()
                .px_2()
                .bg(hsla(p.bg))
                .display(Scale::X2, window)
                .text_color(hsla(p.fg))
                .child(title.to_uppercase()),
        )
        .child(div().px_2().bg(hsla(p.bg)).body(text::SM).text_color(hsla(p.fg_dim)).child(hint.to_string()))
}
