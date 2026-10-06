//! The table: columns of text with a sortable display-face header.
//!
//! ```text
//!  NAME ▲              PID      CPU     STATE
//!  ─────────────────────────────────────────────
//!  cargo              9021    12.4%     run
//! ▌ferrite-atlas      4412    31.0%     run      ← selected
//!  rust-analyzer      3310     4.2%     idle
//! ```
//!
//! For tens to a few hundred rows — this one renders every row. For
//! thousands, render rows in a [`virtual_list`](super::scroll::virtual_list).
//!
//! - **Sorting** is controlled: pass `.sort(Some((column, dir)))`, sort your
//!   rows, and update in `on_sort`. Clicking a sortable header sorts by it
//!   ascending, clicking it again flips the direction.
//! - **Selection** is controlled too: `.selected(Some(row))`, `on_select`.
//!   One tab stop; ↑/↓, Home/End move the selection.
//! - Numbers go in `.align_right()` columns so digits line up.

use std::rc::Rc;

use gpui::{
    App, ElementId, FocusHandle, InteractiveElement, IntoElement, MouseButton, ParentElement,
    Pixels, RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder as _, px,
};

use crate::fonts::{FerriteText, Scale};
use crate::theme::palette;
use crate::tokens::{hsla, text};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDir {
    Asc,
    Desc,
}

impl SortDir {
    pub fn flip(self) -> Self {
        match self {
            SortDir::Asc => SortDir::Desc,
            SortDir::Desc => SortDir::Asc,
        }
    }
}

type SortHandler = Rc<dyn Fn(&(usize, SortDir), &mut Window, &mut App)>;
type SelectHandler = Rc<dyn Fn(&usize, &mut Window, &mut App)>;

/// A column. Build with [`column`].
#[derive(Clone)]
pub struct Column {
    title: SharedString,
    width: Option<Pixels>,
    right: bool,
    sortable: bool,
}

pub fn column(title: impl Into<SharedString>) -> Column {
    Column { title: title.into(), width: None, right: false, sortable: false }
}

impl Column {
    /// Fixed width. Columns without one share the remaining space.
    pub fn width(mut self, width: Pixels) -> Self {
        self.width = Some(width);
        self
    }

    pub fn align_right(mut self) -> Self {
        self.right = true;
        self
    }

    pub fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }
}

#[derive(IntoElement)]
pub struct Table {
    id: ElementId,
    columns: Vec<Column>,
    rows: Vec<Vec<SharedString>>,
    sort: Option<(usize, SortDir)>,
    selected: Option<usize>,
    on_sort: Option<SortHandler>,
    on_select: Option<SelectHandler>,
}

pub fn table(id: impl Into<ElementId>) -> Table {
    Table { id: id.into(), columns: Vec::new(), rows: Vec::new(), sort: None, selected: None, on_sort: None, on_select: None }
}

impl Table {
    pub fn column(mut self, column: Column) -> Self {
        self.columns.push(column);
        self
    }

    pub fn row(mut self, cells: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.rows.push(cells.into_iter().map(Into::into).collect());
        self
    }

    pub fn rows<R: IntoIterator<Item = impl Into<SharedString>>>(mut self, rows: impl IntoIterator<Item = R>) -> Self {
        for r in rows {
            self = self.row(r);
        }
        self
    }

    /// The column the rows are sorted by, and which way. Display only — you
    /// sort the rows.
    pub fn sort(mut self, sort: Option<(usize, SortDir)>) -> Self {
        self.sort = sort;
        self
    }

    pub fn selected(mut self, row: Option<usize>) -> Self {
        self.selected = row;
        self
    }

    /// Called with the requested `(column, direction)`.
    pub fn on_sort(mut self, handler: impl Fn(&(usize, SortDir), &mut Window, &mut App) + 'static) -> Self {
        self.on_sort = Some(Rc::new(handler));
        self
    }

    pub fn on_select(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

/// The sort a header click asks for.
pub fn next_sort(current: Option<(usize, SortDir)>, clicked: usize) -> (usize, SortDir) {
    match current {
        Some((c, dir)) if c == clicked => (c, dir.flip()),
        _ => (clicked, SortDir::Asc),
    }
}

fn key_row(key: &str, selected: Option<usize>, len: usize) -> Option<usize> {
    if len == 0 {
        return None;
    }
    Some(match (key, selected) {
        ("down", Some(s)) => (s + 1).min(len - 1),
        ("up", Some(s)) => s.saturating_sub(1),
        ("down" | "home", None) | ("home", _) => 0,
        ("up" | "end", None) | ("end", _) => len - 1,
        _ => return None,
    })
}

const ROW: Pixels = px(28.);

impl RenderOnce for Table {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let focus: FocusHandle = window.use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle()).read(cx).clone();
        let len = self.rows.len();
        let cols = Rc::new(self.columns);

        let cell = |col: &Column| {
            div()
                .px_3()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .flex()
                .flex_row()
                .when(col.right, |el| el.justify_end())
                .map(|el| match col.width {
                    Some(w) => el.w(w).flex_none(),
                    None => el.flex_1(),
                })
        };

        let mut header = div()
            .flex()
            .flex_row()
            .items_center()
            .h(ROW)
            .bg(hsla(p.surface))
            .border_b_1()
            .border_color(hsla(p.line_strong));
        for (ci, col) in cols.iter().enumerate() {
            let sorted = self.sort.filter(|(c, _)| *c == ci).map(|(_, d)| d);
            let arrow = match sorted {
                Some(SortDir::Asc) => " ▲",
                Some(SortDir::Desc) => " ▼",
                None => "",
            };
            let ink = if sorted.is_some() { p.fg } else { p.fg_dim };
            header = header.child(
                cell(col)
                    .id(("col", ci))
                    .role(Role::ColumnHeader)
                    .aria_label(col.title.clone())
                    .h_full()
                    .items_center()
                    .display(Scale::X1, window)
                    .text_color(hsla(ink))
                    .child(col.title.to_uppercase())
                    .child(div().text_color(hsla(p.accent_text)).child(arrow))
                    .when_some(self.on_sort.clone().filter(|_| col.sortable), |el, handler| {
                        let current = self.sort;
                        el.cursor_pointer()
                            .hover(|s| s.text_color(hsla(p.fg)).bg(hsla(p.raised)))
                            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                            .on_click(move |_, window, cx| handler(&next_sort(current, ci), window, cx))
                    }),
            );
        }

        let mut body = div().flex().flex_col();
        for (ri, cells) in self.rows.iter().enumerate() {
            let active = self.selected == Some(ri);
            let mut row = div()
                .id(("row", ri))
                .role(Role::Row)
                .aria_selected(active)
                .relative()
                .flex()
                .flex_row()
                .items_center()
                .h(ROW)
                .border_b_1()
                .border_color(hsla(p.line))
                .body(text::BASE)
                .text_color(hsla(p.fg))
                .children(super::selection(format!("{}-row-{ri}", self.id), active, window, cx))
                .when(!active, |el| el.hover(|s| s.bg(hsla(p.raised))));
            for (ci, col) in cols.iter().enumerate() {
                let text = cells.get(ci).cloned().unwrap_or_default();
                row = row.child(cell(col).text_color(hsla(if ci == 0 || active { p.fg } else { p.fg_dim })).child(text));
            }
            body = body.child(
                row.on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .when_some(self.on_select.clone(), |el, handler| {
                        let focus = focus.clone();
                        el.on_click(move |_, window, cx| {
                            focus.focus(window, cx);
                            handler(&ri, window, cx);
                        })
                    }),
            );
        }

        let selected = self.selected;
        div()
            .id(self.id.clone())
            .role(Role::Table)
            .aria_row_count(len)
            .aria_column_count(cols.len())
            .track_focus(&focus)
            .tab_stop(true)
            .flex()
            .flex_col()
            .border_1()
            .border_color(hsla(p.line))
            .focus_visible(|s| s.border_color(hsla(p.accent)))
            .when_some(self.on_select, |el, handler| {
                el.on_key_down(move |ev, window, cx| {
                    if let Some(next) = key_row(ev.keystroke.key.as_str(), selected, len) {
                        cx.stop_propagation();
                        if Some(next) != selected {
                            handler(&next, window, cx);
                        }
                    }
                })
            })
            .child(header)
            .child(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_click_sorts_then_flips() {
        assert_eq!(next_sort(None, 2), (2, SortDir::Asc));
        assert_eq!(next_sort(Some((2, SortDir::Asc)), 2), (2, SortDir::Desc));
        assert_eq!(next_sort(Some((2, SortDir::Desc)), 2), (2, SortDir::Asc));
        assert_eq!(next_sort(Some((2, SortDir::Desc)), 0), (0, SortDir::Asc));
    }

    #[test]
    fn keys_move_and_clamp() {
        assert_eq!(key_row("down", None, 3), Some(0));
        assert_eq!(key_row("up", None, 3), Some(2));
        assert_eq!(key_row("down", Some(2), 3), Some(2));
        assert_eq!(key_row("up", Some(0), 3), Some(0));
        assert_eq!(key_row("end", Some(0), 3), Some(2));
        assert_eq!(key_row("x", Some(0), 3), None);
        assert_eq!(key_row("down", None, 0), None);
    }
}
