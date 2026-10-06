//! Ferrite's own structural components.
//!
//! Ferrite's whole component set (docs/COMPONENTS.md): controls, overlays,
//! the text input, scrolling and layout in the submodules, and the framing
//! (`Panel`, `StatusBar`, `rule`, …) here.

pub mod button;
pub mod dialog;
pub mod fx;
pub mod input;
pub mod list;
pub mod menu;
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
pub use dialog::{Dialog, dialog};
pub use fx::{count_up, decrypt, dissolve, shake, typewriter, unroll_in};
pub use input::{InputEvent, TextInput};
pub use list::{ListItem, list_item};
pub use menu::{Menu, MenuItem, Submenu, context_menu, dropdown_menu, menu_item, submenu};
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
                        .child(format!("[ {} ]", self.title.to_uppercase())),
                )
                .child(dither(dither::flat(dither::level::LIGHT)).ink(hsla(p.line_strong)).flex_1().h(px(8.)))
                .when_some(self.meta, |this, meta| {
                    this.child(div().display(Scale::X1, window).text_color(hsla(p.fg_dim)).child(meta))
                }),
        )
        .child(div().flex().flex_col().flex_1().min_h_0().p_3().gap_2().children(self.children))
    }
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
#[derive(IntoElement)]
pub struct StatusBar {
    left: Vec<SharedString>,
    right: Vec<SharedString>,
}

pub fn status_bar() -> StatusBar {
    StatusBar { left: Vec::new(), right: Vec::new() }
}

impl StatusBar {
    pub fn left(mut self, segment: impl Into<SharedString>) -> Self {
        self.left.push(segment.into());
        self
    }

    pub fn right(mut self, segment: impl Into<SharedString>) -> Self {
        self.right.push(segment.into());
        self
    }
}

impl RenderOnce for StatusBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let segments = |items: Vec<SharedString>, window: &Window| {
            let mut row = div().flex().flex_row().items_center().gap_2();
            for (i, item) in items.into_iter().enumerate() {
                if i > 0 {
                    row = row.child(div().display(Scale::X1, window).text_color(hsla(p.fg_faint)).child("│"));
                }
                row = row.child(div().display(Scale::X1, window).child(item));
            }
            row
        };
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
            .child(segments(self.left, window))
            .child(segments(self.right, window))
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
