//! Navigation: [`sidebar`], [`toolbar`], [`breadcrumb`], [`pagination`] and
//! [`steps`].
//!
//! ```text
//!  ┌────────────────┐
//!  │ ▓▒░ ATLAS      │  sidebar: brand, sections, items with icons and
//!  │ ── WORKSPACE   │  counts; the active item gets the list-row treatment
//!  ▌ ■ Overview  12 │  (2px amber bar + wash). Collapsed: icons only, with
//!  │ ■ Logs        │  tooltips.
//!  └────────────────┘
//!
//!  src / components / menu.rs                     breadcrumb
//!  [<] 1 … 4 [5] 6 … 20 [>]                       pagination
//!  [✓ ACCOUNT]───[2 WORKSPACE]───[3 INVITE]       steps
//! ```
//!
//! All controlled: the app owns the selection, page or step.

use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FocusHandle, InteractiveElement, IntoElement, MouseButton, ParentElement,
    Pixels, RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled, StyleRefinement, Window, div,
    prelude::FluentBuilder as _, px,
};

use super::tooltip::tooltip;
use crate::dither::{self, dither};
use crate::fonts::{FerriteText, Scale};
use crate::icon::{Icon, icon};
use crate::theme::palette;
use crate::tokens::{hsla, text};

type KeyHandler = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;
type IndexHandler = Rc<dyn Fn(&usize, &mut Window, &mut App)>;

// ── Sidebar ───────────────────────────────────────────────────────────────

enum Entry {
    Section(SharedString),
    Item { key: SharedString, label: SharedString, icon: Icon, meta: Option<SharedString> },
}

/// The app's primary navigation: a column on `surface` with a brand
/// header, section labels and items. `.collapsed(true)` narrows it to an
/// icon rail with tooltips. Put it first in a flex row, beside the content.
#[derive(IntoElement)]
pub struct Sidebar {
    id: ElementId,
    brand: Option<SharedString>,
    entries: Vec<Entry>,
    selected: Option<SharedString>,
    collapsed: bool,
    width: Pixels,
    footer: Vec<AnyElement>,
    on_select: Option<KeyHandler>,
}

pub fn sidebar(id: impl Into<ElementId>) -> Sidebar {
    Sidebar { id: id.into(), brand: None, entries: Vec::new(), selected: None, collapsed: false, width: px(220.), footer: Vec::new(), on_select: None }
}

impl Sidebar {
    /// The app name, in the header with the `▓▒░` mark.
    pub fn brand(mut self, name: impl Into<SharedString>) -> Self {
        self.brand = Some(name.into());
        self
    }

    /// A section label above the items that follow.
    pub fn section(mut self, label: impl Into<SharedString>) -> Self {
        self.entries.push(Entry::Section(label.into()));
        self
    }

    /// An item, keyed by `key` (what `on_select` receives).
    pub fn item(mut self, key: impl Into<SharedString>, label: impl Into<SharedString>, icon: Icon) -> Self {
        self.entries.push(Entry::Item { key: key.into(), label: label.into(), icon, meta: None });
        self
    }

    /// An item with a dim count or status after the label.
    pub fn item_with_meta(mut self, key: impl Into<SharedString>, label: impl Into<SharedString>, icon: Icon, meta: impl Into<SharedString>) -> Self {
        self.entries.push(Entry::Item { key: key.into(), label: label.into(), icon, meta: Some(meta.into()) });
        self
    }

    pub fn selected(mut self, key: impl Into<SharedString>) -> Self {
        self.selected = Some(key.into());
        self
    }

    /// Icons only, 48px wide; labels move into tooltips.
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// Expanded width (default 220px).
    pub fn width(mut self, width: Pixels) -> Self {
        self.width = width;
        self
    }

    /// Pinned to the bottom: an account row, a settings button, a version.
    pub fn footer(mut self, el: impl IntoElement) -> Self {
        self.footer.push(el.into_any_element());
        self
    }

    pub fn on_select(mut self, handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Sidebar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let collapsed = self.collapsed;
        let mut column = div()
            .id(self.id.clone())
            .role(Role::Navigation)
            .flex()
            .flex_col()
            .flex_none()
            .h_full()
            .w(if collapsed { px(48.) } else { self.width })
            .bg(hsla(p.surface))
            .border_r_1()
            .border_color(hsla(p.line));

        if let Some(brand) = self.brand {
            column = column.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .h(px(40.))
                    .px_3()
                    .border_b_1()
                    .border_color(hsla(p.line))
                    .display(Scale::X1, window)
                    .child(super::textmode::mark())
                    .when(!collapsed, |el| el.child(div().text_color(hsla(p.fg)).child(brand.to_uppercase()))),
            );
        }

        let mut items = div().flex().flex_col().flex_1().min_h_0().py_2().gap(px(2.));
        for (i, entry) in self.entries.into_iter().enumerate() {
            match entry {
                Entry::Section(label) => {
                    items = items.child(if collapsed {
                        div().mx_3().my_1().h(px(1.)).bg(hsla(p.line))
                    } else {
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .px_3()
                            .pt(if i > 0 { px(8.) } else { px(0.) })
                            .display(Scale::X1, window)
                            .text_color(hsla(p.fg_faint))
                            .child(label.to_uppercase())
                            .child(dither(dither::flat(dither::level::LIGHT)).ink(hsla(p.line)).flex_1().h(px(6.)))
                    });
                }
                Entry::Item { key, label, icon: glyph, meta } => {
                    let active = self.selected.as_ref() == Some(&key);
                    let id = ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), key.clone());
                    let focus: FocusHandle = window.use_keyed_state(id.clone(), cx, |_, cx| cx.focus_handle()).read(cx).clone();
                    let handler = self.on_select.clone();
                    let sel_key = format!("{}-{}", self.id, key);
                    items = items.child(
                        div()
                            .id(id)
                            .role(Role::Link)
                            .aria_selected(active)
                            .aria_label(label.clone())
                            .track_focus(&focus)
                            .tab_stop(true)
                            .relative()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .h(px(30.))
                            .map(|el| if collapsed { el.justify_center() } else { el.pl(px(12.)).pr_2() })
                            .border_1()
                            .border_color(gpui::transparent_black())
                            .children(super::selection(sel_key, active, window, cx))
                            .when(!active, |el| el.hover(|s| s.bg(hsla(p.raised))))
                            .focus_visible(|s| s.border_color(hsla(p.accent)))
                            .child(icon(glyph).fit(px(16.)).color(hsla(if active { p.accent_text } else { p.fg_dim })))
                            .when(!collapsed, |el| {
                                el.child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_ellipsis()
                                        .body(text::BASE)
                                        .text_color(hsla(if active { p.fg } else { p.fg_dim }))
                                        .child(label.clone()),
                                )
                                .when_some(meta.clone(), |el, m| el.child(div().body(text::XS).text_color(hsla(p.fg_faint)).child(m)))
                            })
                            .when(collapsed, |el| el.tooltip(tooltip(label.clone()).builder()))
                            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                            .when_some(handler, |el, h| el.on_click(move |_, window, cx| h(&key, window, cx))),
                    );
                }
            }
        }
        column
            .child(items)
            .when(!self.footer.is_empty(), |el| {
                el.child(div().flex().flex_col().gap_2().p_2().border_t_1().border_color(hsla(p.line)).children(self.footer))
            })
    }
}

// ── Toolbar ───────────────────────────────────────────────────────────────

/// A row of controls on `surface` with a hairline under it: buttons,
/// segmented controls, a select, a search field. `.separator()` drops a
/// 1px divider between groups; `.spacer()` pushes the rest to the right.
#[derive(IntoElement)]
pub struct Toolbar {
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

pub fn toolbar() -> Toolbar {
    Toolbar { children: Vec::new(), style: StyleRefinement::default() }
}

impl Toolbar {
    pub fn separator(mut self) -> Self {
        self.children.push(ToolbarPart::Separator.into_any_element());
        self
    }

    pub fn spacer(mut self) -> Self {
        self.children.push(div().flex_1().into_any_element());
        self
    }
}

#[derive(IntoElement)]
enum ToolbarPart {
    Separator,
}

impl RenderOnce for ToolbarPart {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div().mx_1().w(px(1.)).h(px(20.)).bg(hsla(palette(cx).line))
    }
}

impl ParentElement for Toolbar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Toolbar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Toolbar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let mut root = div();
        *root.style() = self.style;
        root.id("ferrite-toolbar")
            .role(Role::Toolbar)
            .flex()
            .flex_row()
            .flex_none()
            .items_center()
            .gap_2()
            .h(px(40.))
            .px_2()
            .bg(hsla(p.surface))
            .border_b_1()
            .border_color(hsla(p.line))
            .children(self.children)
    }
}

// ── Breadcrumb ────────────────────────────────────────────────────────────

/// A path: `src / components / menu.rs`. Every crumb but the last is a
/// link (amber on hover); the last is where you are. Body type, because
/// crumbs are names and names keep their case.
#[derive(IntoElement)]
pub struct Breadcrumb {
    id: ElementId,
    crumbs: Vec<SharedString>,
    on_select: Option<IndexHandler>,
}

pub fn breadcrumb(id: impl Into<ElementId>) -> Breadcrumb {
    Breadcrumb { id: id.into(), crumbs: Vec::new(), on_select: None }
}

impl Breadcrumb {
    pub fn crumb(mut self, label: impl Into<SharedString>) -> Self {
        self.crumbs.push(label.into());
        self
    }

    pub fn crumbs(mut self, labels: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.crumbs.extend(labels.into_iter().map(Into::into));
        self
    }

    /// Called with the index of the crumb clicked (never the last).
    pub fn on_select(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Breadcrumb {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let last = self.crumbs.len().saturating_sub(1);
        let mut row = div().id(self.id.clone()).role(Role::Navigation).flex().flex_row().items_center().gap_2().body(text::SM);
        for (i, crumb) in self.crumbs.into_iter().enumerate() {
            if i > 0 {
                row = row.child(div().text_color(hsla(p.fg_faint)).child("/"));
            }
            if i == last {
                row = row.child(div().text_color(hsla(p.fg)).child(crumb));
            } else {
                let handler = self.on_select.clone();
                row = row.child(
                    div()
                        .id(("crumb", i))
                        .role(Role::Link)
                        .text_color(hsla(p.fg_dim))
                        .hover(|s| s.text_color(hsla(p.accent_text)))
                        .when_some(handler, |el, h| el.on_click(move |_, window, cx| h(&i, window, cx)))
                        .child(crumb),
                );
            }
        }
        row
    }
}

// ── Pagination ────────────────────────────────────────────────────────────

/// The pages to show around `current` (0-based) out of `total`: the first,
/// the last, `siblings` either side of the current one, and `None` for each
/// gap. Always the same length once `total` is large, so the strip never
/// changes width as you page.
pub fn page_window(current: usize, total: usize, siblings: usize) -> Vec<Option<usize>> {
    let slots = 2 * siblings + 5; // first, last, current, two gaps
    if total <= slots {
        return (0..total).map(Some).collect();
    }
    let current = current.min(total - 1);
    let left = current.saturating_sub(siblings).max(1);
    let right = (current + siblings).min(total - 2);
    let near_start = left <= 2;
    let near_end = right >= total - 3;
    let mut out = vec![Some(0)];
    if near_start {
        out.extend((1..slots - 2).map(Some));
        out.push(None);
    } else if near_end {
        out.push(None);
        out.extend((total - (slots - 2)..total - 1).map(Some));
    } else {
        out.push(None);
        out.extend((left..=right).map(Some));
        out.push(None);
    }
    out.push(Some(total - 1));
    out
}

/// A pager: `[<] 1 … 4 [5] 6 … 20 [>]`. The current page is inverse video;
/// ←/→ (with the strip focused) page. Pages are shown 1-based, handled
/// 0-based.
#[derive(IntoElement)]
pub struct Pagination {
    id: ElementId,
    current: usize,
    total: usize,
    siblings: usize,
    on_change: Option<IndexHandler>,
}

pub fn pagination(id: impl Into<ElementId>) -> Pagination {
    Pagination { id: id.into(), current: 0, total: 1, siblings: 1, on_change: None }
}

impl Pagination {
    pub fn current(mut self, page: usize) -> Self {
        self.current = page;
        self
    }

    pub fn total(mut self, pages: usize) -> Self {
        self.total = pages.max(1);
        self
    }

    /// Pages shown either side of the current one (default 1).
    pub fn siblings(mut self, n: usize) -> Self {
        self.siblings = n;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Pagination {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let focus: FocusHandle = window.use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle()).read(cx).clone();
        let (current, total) = (self.current.min(self.total - 1), self.total);
        let handler = self.on_change.clone();
        let cell = |id: ElementId, enabled: bool, on: bool| {
            div()
                .id(id)
                .flex()
                .items_center()
                .justify_center()
                .h(px(24.))
                .min_w(px(24.))
                .px_1()
                .border_1()
                .border_color(hsla(if on { p.accent } else { p.line }))
                .when(on, |el| el.bg(hsla(p.accent)).text_color(hsla(p.accent_fg)))
                .when(!on && enabled, |el| el.text_color(hsla(p.fg_dim)).hover(|s| s.bg(hsla(p.raised)).text_color(hsla(p.fg))))
                .when(!enabled, |el| el.text_color(hsla(p.fg_faint)))
        };
        let go = |to: usize| {
            let handler = handler.clone();
            move |_: &gpui::ClickEvent, window: &mut Window, cx: &mut App| {
                if let Some(h) = &handler {
                    h(&to, window, cx);
                }
            }
        };
        let mut row = div()
            .id(self.id.clone())
            .role(Role::Navigation)
            .track_focus(&focus)
            .tab_stop(true)
            .flex()
            .flex_row()
            .items_center()
            .gap_1()
            .p(px(1.))
            .border_1()
            .border_color(gpui::transparent_black())
            .focus_visible(|s| s.border_color(hsla(p.accent)))
            .display(Scale::X1, window)
            .on_mouse_down(MouseButton::Left, {
                let focus = focus.clone();
                move |_, window, cx| {
                    window.prevent_default();
                    focus.focus(window, cx);
                }
            })
            .when_some(handler.clone(), |el, h| {
                el.on_key_down(move |ev, window, cx| {
                    let to = match ev.keystroke.key.as_str() {
                        "left" if current > 0 => current - 1,
                        "right" if current + 1 < total => current + 1,
                        "home" => 0,
                        "end" => total - 1,
                        _ => return,
                    };
                    cx.stop_propagation();
                    h(&to, window, cx);
                })
            });
        row = row.child(
            cell("prev".into(), current > 0, false)
                .child(icon(Icon::ChevronLeft).fit(px(16.)).color(hsla(if current > 0 { p.fg_dim } else { p.fg_faint })))
                .when(current > 0, |el| el.on_click(go(current - 1))),
        );
        for (slot, page) in page_window(current, total, self.siblings).into_iter().enumerate() {
            row = row.child(match page {
                Some(n) => cell(ElementId::Integer(n as u64), true, n == current)
                    .aria_label(SharedString::from(format!("page {}", n + 1)))
                    .child(format!("{}", n + 1))
                    .when(n != current, |el| el.on_click(go(n))),
                None => cell(("gap", slot).into(), false, false).border_color(gpui::transparent_black()).child("…"),
            });
        }
        row.child(
            cell("next".into(), current + 1 < total, false)
                .child(icon(Icon::ChevronRight).fit(px(16.)).color(hsla(if current + 1 < total { p.fg_dim } else { p.fg_faint })))
                .when(current + 1 < total, |el| el.on_click(go(current + 1))),
        )
    }
}

// ── Steps ─────────────────────────────────────────────────────────────────

/// Progress through a sequence — a wizard, an installer, a checkout:
/// `[✓ ACCOUNT]───[2 WORKSPACE]───[3 INVITE]`. Done steps carry an amber
/// check and an amber connector; the current one is framed in amber; the
/// rest are dim. With `on_select`, done steps are clickable (go back).
#[derive(IntoElement)]
pub struct Steps {
    id: ElementId,
    labels: Vec<SharedString>,
    current: usize,
    on_select: Option<IndexHandler>,
}

pub fn steps(id: impl Into<ElementId>) -> Steps {
    Steps { id: id.into(), labels: Vec::new(), current: 0, on_select: None }
}

impl Steps {
    pub fn step(mut self, label: impl Into<SharedString>) -> Self {
        self.labels.push(label.into());
        self
    }

    /// The step in progress (0-based). Past the last = all done.
    pub fn current(mut self, step: usize) -> Self {
        self.current = step;
        self
    }

    /// Called with a done step's index when it's clicked.
    pub fn on_select(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Steps {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let current = self.current;
        // The connector into the current step draws on when it advances.
        let t = crate::animate::play_on_change(ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), "advance".into()), current, crate::motion::BASE, window, cx).eased();
        let mut row = div().id(self.id.clone()).role(Role::List).flex().flex_row().items_center().display(Scale::X1, window);
        for (i, label) in self.labels.into_iter().enumerate() {
            let (done, here) = (i < current, i == current);
            if i > 0 {
                let lit = if i < current { 1. } else if here { t } else { 0. };
                row = row.child(
                    div()
                        .relative()
                        .flex_1()
                        .min_w(px(24.))
                        .h(px(1.))
                        .bg(hsla(p.line_strong))
                        .child(div().absolute().left_0().top_0().bottom_0().w(gpui::relative(lit)).bg(hsla(p.accent))),
                );
            }
            let handler = self.on_select.clone().filter(|_| done);
            row = row.child(
                div()
                    .id(("step", i))
                    .role(Role::ListItem)
                    .aria_label(label.clone())
                    .flex()
                    .flex_row()
                    .flex_none()
                    .items_center()
                    .gap_2()
                    .h(px(28.))
                    .px_2()
                    .border_1()
                    .border_color(hsla(if here { p.accent } else if done { p.line_strong } else { p.line }))
                    .text_color(hsla(if here { p.accent_text } else if done { p.fg } else { p.fg_faint }))
                    .map(|el| {
                        if done {
                            el.child(icon(Icon::Check).fit(px(16.)).color(hsla(p.accent)))
                        } else {
                            el.child(format!("{}", i + 1))
                        }
                    })
                    .child(label.to_uppercase())
                    .when_some(handler, |el, h| el.hover(|s| s.bg(hsla(p.raised))).on_click(move |_, window, cx| h(&i, window, cx))),
            );
        }
        row
    }
}

#[cfg(test)]
mod tests {
    use super::page_window;

    #[test]
    fn small_totals_show_every_page() {
        assert_eq!(page_window(0, 1, 1), vec![Some(0)]);
        assert_eq!(page_window(2, 7, 1).len(), 7);
    }

    #[test]
    fn the_window_keeps_its_width_and_ends() {
        for current in 0..30 {
            let w = page_window(current, 30, 1);
            assert_eq!(w.len(), 7, "page {current}: {w:?}");
            assert_eq!(w.first(), Some(&Some(0)));
            assert_eq!(w.last(), Some(&Some(29)));
            assert!(w.contains(&Some(current)), "page {current} is visible: {w:?}");
        }
        assert_eq!(page_window(15, 30, 1), vec![Some(0), None, Some(14), Some(15), Some(16), None, Some(29)]);
        assert_eq!(page_window(0, 30, 1), vec![Some(0), Some(1), Some(2), Some(3), Some(4), None, Some(29)]);
        assert_eq!(page_window(29, 30, 1), vec![Some(0), None, Some(25), Some(26), Some(27), Some(28), Some(29)]);
    }
}
