//! Menus: a dropdown opened from a trigger, and a context menu opened by
//! right-click at the pointer (docs/COMPONENTS.md #5).
//!
//! Rows are body type with an optional leading icon (or a check for
//! toggleable entries) and a right-aligned shortcut. The highlighted row
//! gets the list-row treatment — 2px amber bar + `accent_dim` wash; danger
//! rows highlight in danger. Section labels are display-face, separators are
//! 1px hairlines.
//!
//! Keyboard: ↑/↓ move (skipping disabled rows, wrapping), Home/End jump,
//! Enter/Space choose, Escape closes. Opening from the keyboard highlights
//! the first row; opening with the mouse highlights nothing until hover.
//! Choosing closes the menu and returns focus *before* the handler runs, so
//! a handler that opens a dialog or moves focus wins.

use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, Entity, InteractiveElement, IntoElement, MouseButton,
    ParentElement, RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled, Window,
    div, prelude::FluentBuilder as _, px,
};

use super::overlay::{Align, OverlayState, at_point, below, surface};
use crate::fonts::{FerriteText, Scale};
use crate::icon::{Icon, icon};
use crate::theme::palette;
use crate::tokens::{hsla, text};

type SelectHandler = Rc<dyn Fn(&mut Window, &mut App)>;

/// One choosable row.
#[derive(Clone)]
pub struct MenuItem {
    label: SharedString,
    icon: Option<Icon>,
    shortcut: Option<SharedString>,
    checked: Option<bool>,
    danger: bool,
    disabled: bool,
    on_select: Option<SelectHandler>,
}

pub fn menu_item(label: impl Into<SharedString>) -> MenuItem {
    MenuItem { label: label.into(), icon: None, shortcut: None, checked: None, danger: false, disabled: false, on_select: None }
}

impl MenuItem {
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Display-only shortcut text, e.g. `"Ctrl+S"`.
    pub fn shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// A toggle entry: shows a check when `true`, reserves the space when
    /// `false`.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_select(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

#[derive(Clone)]
enum Entry {
    Item(MenuItem),
    Separator,
    Label(SharedString),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Dropdown,
    Context,
}

/// A dropdown or context menu. Build with [`dropdown_menu`] or
/// [`context_menu`].
#[derive(IntoElement)]
pub struct Menu {
    id: ElementId,
    kind: Kind,
    trigger: Option<AnyElement>,
    children: Vec<AnyElement>,
    entries: Vec<Entry>,
    align: Align,
    width: gpui::Pixels,
}

/// A menu that drops down from `trigger` (usually a `Button`).
pub fn dropdown_menu(id: impl Into<ElementId>) -> Menu {
    Menu { id: id.into(), kind: Kind::Dropdown, trigger: None, children: Vec::new(), entries: Vec::new(), align: Align::Start, width: px(240.) }
}

/// A menu that opens at the pointer when its children are right-clicked.
pub fn context_menu(id: impl Into<ElementId>) -> Menu {
    Menu { kind: Kind::Context, ..dropdown_menu(id) }
}

impl Menu {
    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    pub fn item(mut self, item: MenuItem) -> Self {
        self.entries.push(Entry::Item(item));
        self
    }

    pub fn separator(mut self) -> Self {
        self.entries.push(Entry::Separator);
        self
    }

    /// A section heading.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.entries.push(Entry::Label(label.into()));
        self
    }

    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    pub fn width(mut self, width: gpui::Pixels) -> Self {
        self.width = width;
        self
    }
}

impl ParentElement for Menu {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

/// Indices of entries the keyboard can land on.
fn choosable(entries: &[Entry]) -> Vec<usize> {
    entries
        .iter()
        .enumerate()
        .filter_map(|(i, e)| matches!(e, Entry::Item(item) if !item.disabled).then_some(i))
        .collect()
}

/// The next highlight for a navigation key, or `None` if the key isn't one.
fn navigate(key: &str, current: Option<usize>, choosable: &[usize]) -> Option<Option<usize>> {
    if choosable.is_empty() {
        return None;
    }
    let pos = current.and_then(|c| choosable.iter().position(|&i| i == c));
    let n = choosable.len();
    let next = match key {
        "down" => pos.map_or(0, |p| (p + 1) % n),
        "up" => pos.map_or(n - 1, |p| (p + n - 1) % n),
        "home" => 0,
        "end" => n - 1,
        _ => return None,
    };
    Some(Some(choosable[next]))
}

fn choose(state: &Entity<OverlayState>, item: &MenuItem, window: &mut Window, cx: &mut App) {
    state.update(cx, |s, cx| s.close(window, cx));
    if let Some(handler) = &item.on_select {
        handler(window, cx);
    }
}

impl RenderOnce for Menu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let state = window.use_keyed_state(self.id.clone(), cx, |_, cx| OverlayState::new(cx));
        let (open, highlight, at) = {
            let s = state.read(cx);
            (s.open, s.highlight, s.at)
        };
        let entries = Rc::new(self.entries);
        let keyboard_targets = choosable(&entries);
        let first = keyboard_targets.first().copied();

        // ── The opener ──────────────────────────────────────────────────
        let opener = match self.kind {
            Kind::Dropdown => div()
                .id("trigger")
                .on_mouse_down(MouseButton::Left, {
                    let state = state.clone();
                    move |_, window, cx| {
                        cx.stop_propagation();
                        state.update(cx, |s, cx| {
                            if s.open == open {
                                if open { s.close(window, cx) } else { s.show(None, None, window, cx) }
                            }
                        });
                    }
                })
                .on_key_down({
                    let state = state.clone();
                    move |ev, window, cx| {
                        if !open && matches!(ev.keystroke.key.as_str(), "enter" | "space" | "down") {
                            cx.stop_propagation();
                            state.update(cx, |s, cx| s.show(None, first, window, cx));
                        }
                    }
                })
                .children(self.trigger),
            Kind::Context => div()
                .id("area")
                .on_mouse_down(MouseButton::Right, {
                    let state = state.clone();
                    move |ev, window, cx| {
                        cx.stop_propagation();
                        state.update(cx, |s, cx| s.show(Some(ev.position), None, window, cx));
                    }
                })
                .children(self.children),
        };

        let mut root = div().id(self.id.clone()).relative().child(opener);
        if !open {
            return root;
        }

        // ── The surface ─────────────────────────────────────────────────
        let has_lead = entries.iter().any(|e| matches!(e, Entry::Item(i) if i.icon.is_some() || i.checked.is_some()));
        let focus = state.read(cx).focus.clone();
        let mut list = div().w(self.width).flex().flex_col().py_1();
        for (i, entry) in entries.iter().enumerate() {
            list = list.child(match entry {
                Entry::Separator => div().h(px(1.)).my_1().mx_2().bg(hsla(p.line)).into_any_element(),
                Entry::Label(label) => div()
                    .px_3()
                    .pt_1()
                    .display(Scale::X1, window)
                    .text_color(hsla(p.fg_faint))
                    .child(label.to_uppercase())
                    .into_any_element(),
                Entry::Item(item) => {
                    let active = highlight == Some(i) && !item.disabled;
                    let (ink, bg) = match (item.disabled, item.danger, active) {
                        (true, _, _) => (p.fg_faint, None),
                        (false, true, true) => (p.danger_fg, Some(p.danger)),
                        (false, true, false) => (p.danger, None),
                        (false, false, true) => (p.fg, Some(p.accent_dim)),
                        (false, false, false) => (p.fg, None),
                    };
                    let lead_ink = if active && !item.danger { p.accent_text } else { ink };
                    div()
                        .id(("item", i))
                        .role(Role::MenuItem)
                        .aria_label(item.label.clone())
                        .when_some(item.checked, |el, c| el.aria_toggled(if c { gpui::Toggled::True } else { gpui::Toggled::False }))
                        .relative()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .h(px(28.))
                        .px_3()
                        .body(text::BASE)
                        .text_color(hsla(ink))
                        .when_some(bg, |el, bg| el.bg(hsla(bg)))
                        .when(active && !item.danger, |el| {
                            el.child(div().absolute().left_0().top_0().bottom_0().w(px(2.)).bg(hsla(p.accent)))
                        })
                        .when(has_lead, |el| {
                            let lead = match (item.checked, item.icon) {
                                (Some(true), _) => Some(Icon::Check),
                                (Some(false), _) => None,
                                (None, i) => i,
                            };
                            // Checks are amber, like a checkbox's `[x]`.
                            let ink = if item.checked == Some(true) && !item.disabled { p.accent_text } else { lead_ink };
                            el.child(
                                div()
                                    .size(crate::fonts::display_size(Scale::X1, window))
                                    .flex_shrink_0()
                                    .when_some(lead, |el, l| el.child(icon(l).color(hsla(ink)))),
                            )
                        })
                        .child(div().flex_1().min_w_0().overflow_hidden().whitespace_nowrap().child(item.label.clone()))
                        .when_some(item.shortcut.clone(), |el, k| {
                            el.child(
                                div()
                                    .body(text::SM)
                                    .text_color(hsla(if active && item.danger { p.danger_fg } else { p.fg_dim }))
                                    .child(k),
                            )
                        })
                        .when(!item.disabled, |el| {
                            let hover_state = state.clone();
                            let click_state = state.clone();
                            let item = item.clone();
                            el.on_hover(move |hovered, _, cx| {
                                if *hovered {
                                    hover_state.update(cx, |s, cx| {
                                        if s.highlight != Some(i) {
                                            s.highlight = Some(i);
                                            cx.notify();
                                        }
                                    });
                                }
                            })
                            .on_mouse_down(MouseButton::Left, |_, window, cx| {
                                window.prevent_default();
                                cx.stop_propagation();
                            })
                            .on_click(move |_, window, cx| choose(&click_state, &item, window, cx))
                        })
                        .into_any_element()
                }
            });
        }

        let panel = div()
            .id("menu-surface")
            .role(Role::Menu)
            .track_focus(&focus)
            .occlude()
            .on_key_down({
                let state = state.clone();
                let entries = entries.clone();
                move |ev, window, cx| {
                    let key = ev.keystroke.key.as_str();
                    if key == "escape" {
                        cx.stop_propagation();
                        state.update(cx, |s, cx| s.close(window, cx));
                    } else if matches!(key, "enter" | "space") {
                        cx.stop_propagation();
                        if let Some(Entry::Item(item)) = highlight.and_then(|h| entries.get(h)) {
                            choose(&state, item, window, cx);
                        }
                    } else if let Some(next) = navigate(key, highlight, &keyboard_targets) {
                        cx.stop_propagation();
                        state.update(cx, |s, cx| {
                            s.highlight = next;
                            cx.notify();
                        });
                    }
                }
            })
            .on_mouse_down_out({
                let state = state.clone();
                move |_, window, cx| state.update(cx, |s, cx| s.close(window, cx))
            })
            .child(surface(list, cx));

        root = match (self.kind, at) {
            (Kind::Context, Some(position)) => root.child(at_point(position, panel)),
            _ => root.child(below(self.align, panel)),
        };
        root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries() -> Vec<Entry> {
        vec![
            Entry::Label("file".into()),
            Entry::Item(menu_item("open")),
            Entry::Item(menu_item("locked").disabled(true)),
            Entry::Separator,
            Entry::Item(menu_item("save")),
            Entry::Item(menu_item("quit")),
        ]
    }

    #[test]
    fn keyboard_skips_labels_separators_and_disabled_rows() {
        assert_eq!(choosable(&entries()), vec![1, 4, 5]);
    }

    #[test]
    fn arrows_wrap_and_start_sensibly() {
        let c = choosable(&entries());
        assert_eq!(navigate("down", None, &c), Some(Some(1)));
        assert_eq!(navigate("up", None, &c), Some(Some(5)));
        assert_eq!(navigate("down", Some(1), &c), Some(Some(4)), "skips disabled + separator");
        assert_eq!(navigate("down", Some(5), &c), Some(Some(1)), "wraps");
        assert_eq!(navigate("up", Some(1), &c), Some(Some(5)), "wraps");
        assert_eq!(navigate("end", Some(1), &c), Some(Some(5)));
        assert_eq!(navigate("x", Some(1), &c), None);
    }
}
