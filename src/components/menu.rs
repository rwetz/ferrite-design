//! Menus: a dropdown opened from a trigger, and a context menu opened by
//! right-click at the pointer (docs/COMPONENTS.md #5), with nested
//! submenus.
//!
//! Rows are body type with an optional leading icon (or a check for
//! toggleable entries) and a right-aligned shortcut. The highlighted row
//! gets the list-row treatment — 2px amber bar + `accent_dim` wash; danger
//! rows highlight in danger. Section labels are display-face, separators are
//! 1px hairlines.
//!
//! **Submenus** open to the right, their first row level with the row that
//! opened them, like panes of a text-mode menu bar:
//!
//! ```text
//!  ┌──────────────────┐
//!  │  New file        │
//!  ▌▸ Open recent    > ┌────────────────────┐
//!  │  Export         >│ ferrite-design/    │
//!  └──────────────────┘ nexis-design/      │
//!                     └────────────────────┘▒
//! ```
//!
//! They open instantly on hover (Ferrite has no hover delays) and the
//! parent row stays highlighted while its submenu is open. All levels are
//! laid out in one row inside one anchored layer, so the whole cascade
//! snaps inside the window together and a click anywhere in it is
//! "inside" (PITFALLS §30).
//!
//! Keyboard (acts on the deepest open level): ↑/↓ move (skipping disabled
//! rows, wrapping), Home/End jump, → or Enter/Space opens a submenu with its
//! first row highlighted, ← or Escape closes one level, Escape at the root
//! closes the menu, Enter/Space chooses. Opening from the keyboard
//! highlights the first row; opening with the mouse highlights nothing
//! until hover. Choosing closes the whole menu and returns focus *before*
//! the handler runs, so a handler that opens a dialog or moves focus wins.

use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, Entity, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Pixels, RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled,
    Window, div, prelude::FluentBuilder as _, px,
};

use super::overlay::{Align, OverlayState, at_point, below, reveal, surface};
use crate::fonts::{FerriteText, Scale, display_size};
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

/// A row that opens a nested menu. Build with [`submenu`], fill it like a
/// [`Menu`], and add it with [`Menu::submenu`] (or [`Submenu::submenu`] to
/// nest deeper — but keep it to two levels where you can).
#[derive(Clone)]
pub struct Submenu {
    label: SharedString,
    icon: Option<Icon>,
    disabled: bool,
    entries: Vec<Entry>,
}

pub fn submenu(label: impl Into<SharedString>) -> Submenu {
    Submenu { label: label.into(), icon: None, disabled: false, entries: Vec::new() }
}

impl Submenu {
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn item(mut self, item: MenuItem) -> Self {
        self.entries.push(Entry::Item(item));
        self
    }

    pub fn submenu(mut self, submenu: Submenu) -> Self {
        self.entries.push(Entry::Submenu(submenu));
        self
    }

    pub fn separator(mut self) -> Self {
        self.entries.push(Entry::Separator);
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.entries.push(Entry::Label(label.into()));
        self
    }
}

#[derive(Clone)]
enum Entry {
    Item(MenuItem),
    Submenu(Submenu),
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
    width: Pixels,
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

    pub fn submenu(mut self, submenu: Submenu) -> Self {
        self.entries.push(Entry::Submenu(submenu));
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

    /// Width of every level.
    pub fn width(mut self, width: Pixels) -> Self {
        self.width = width;
        self
    }
}

impl ParentElement for Menu {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

// ── Navigation (pure, tested) ─────────────────────────────────────────────
//
// The open cascade is a *path*: `path[0]` is the root level's highlight,
// `path[k]` the highlight in the submenu opened from `path[k - 1]`. Its
// length is the number of open levels.

/// Indices of entries the keyboard can land on.
fn choosable(entries: &[Entry]) -> Vec<usize> {
    entries
        .iter()
        .enumerate()
        .filter_map(|(i, e)| match e {
            Entry::Item(item) if !item.disabled => Some(i),
            Entry::Submenu(sub) if !sub.disabled => Some(i),
            _ => None,
        })
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

/// The entries shown at `level` of the cascade described by `path`.
fn level_entries<'a>(root: &'a [Entry], path: &[Option<usize>], level: usize) -> Option<&'a [Entry]> {
    let mut entries = root;
    for j in 0..level {
        match path.get(j).copied().flatten().and_then(|i| entries.get(i)) {
            Some(Entry::Submenu(sub)) => entries = &sub.entries,
            _ => return None,
        }
    }
    Some(entries)
}

/// The path after the pointer enters row `i` of `level`: that row is
/// highlighted, anything deeper closes, and a submenu row opens its
/// submenu. Re-entering an already-open submenu's row changes nothing.
fn hover_path(path: &[Option<usize>], level: usize, i: usize, opens: bool) -> Vec<Option<usize>> {
    if opens && path.get(level) == Some(&Some(i)) && path.len() > level + 1 {
        return path.to_vec();
    }
    let mut next = path[..=level.min(path.len().saturating_sub(1))].to_vec();
    next.resize(level + 1, None);
    next[level] = Some(i);
    if opens {
        next.push(None);
    }
    next
}

#[derive(Debug, PartialEq)]
enum KeyOutcome {
    /// Move to this path.
    Path(Vec<Option<usize>>),
    /// Choose row `index` of `level`.
    Choose { level: usize, index: usize },
    /// Close the whole menu.
    Close,
    Ignore,
}

/// What a key does to the cascade. Acts on the deepest open level.
fn key_outcome(root: &[Entry], path: &[Option<usize>], key: &str) -> KeyOutcome {
    let depth = path.len().saturating_sub(1);
    let Some(entries) = level_entries(root, path, depth) else {
        return KeyOutcome::Ignore;
    };
    let current = path.get(depth).copied().flatten();
    let pop = || KeyOutcome::Path(path[..depth].to_vec());
    match key {
        "escape" if depth > 0 => pop(),
        "escape" => KeyOutcome::Close,
        "left" if depth > 0 => pop(),
        "right" | "enter" | "space" => match current.and_then(|i| entries.get(i).map(|e| (i, e))) {
            Some((_, Entry::Submenu(sub))) if !sub.disabled => {
                let mut next = path.to_vec();
                next.push(choosable(&sub.entries).first().copied());
                KeyOutcome::Path(next)
            }
            Some((i, Entry::Item(item))) if !item.disabled && key != "right" => KeyOutcome::Choose { level: depth, index: i },
            _ => KeyOutcome::Ignore,
        },
        _ => match navigate(key, current, &choosable(entries)) {
            Some(next) => {
                let mut p = path[..=depth].to_vec();
                p[depth] = next;
                KeyOutcome::Path(p)
            }
            None => KeyOutcome::Ignore,
        },
    }
}

/// Row heights, so a submenu's first row can line up with its parent row.
/// Item rows follow `theme::row_height`.
const SEPARATOR: Pixels = px(9.); // 1px line + 4px margin each side

fn entry_height(entry: &Entry, label: Pixels, row: Pixels) -> Pixels {
    match entry {
        Entry::Item(_) | Entry::Submenu(_) => row,
        Entry::Separator => SEPARATOR,
        Entry::Label(_) => label,
    }
}

// ── State plumbing ────────────────────────────────────────────────────────

fn read_path(state: &Entity<OverlayState>, cx: &App) -> Vec<Option<usize>> {
    let s = state.read(cx);
    std::iter::once(s.highlight).chain(s.sub.iter().copied()).collect()
}

fn write_path(state: &Entity<OverlayState>, path: Vec<Option<usize>>, cx: &mut App) {
    state.update(cx, |s, cx| {
        let highlight = path.first().copied().flatten();
        let sub = path.get(1..).map(<[_]>::to_vec).unwrap_or_default();
        if s.highlight != highlight || s.sub != sub {
            s.highlight = highlight;
            s.sub = sub;
            cx.notify();
        }
    });
}

fn choose(state: &Entity<OverlayState>, item: &MenuItem, window: &mut Window, cx: &mut App) {
    state.update(cx, |s, cx| s.close(window, cx));
    if let Some(handler) = &item.on_select {
        handler(window, cx);
    }
}

// ── Rendering ─────────────────────────────────────────────────────────────

/// How far a row's label sits left of home while it cascades in: one step
/// per beat, then at rest.
fn row_slide(p: crate::animate::Progress) -> Pixels {
    const STEPS: [f32; 3] = [-6., -2., 0.];
    if p.done {
        return px(0.);
    }
    px(STEPS[(p.frame as usize).min(STEPS.len() - 1)])
}

/// One level's panel.
fn render_level(
    entries: &[Entry],
    level: usize,
    path: &[Option<usize>],
    width: Pixels,
    state: &Entity<OverlayState>,
    window: &mut Window,
    cx: &mut App,
) -> gpui::Stateful<gpui::Div> {
    let p = palette(cx);
    let lead = display_size(Scale::X1, window);
    let highlight = path.get(level).copied().flatten();
    let has_lead = entries.iter().any(|e| match e {
        Entry::Item(i) => i.icon.is_some() || i.checked.is_some(),
        Entry::Submenu(s) => s.icon.is_some(),
        _ => false,
    });

    // Rows cascade in under the unroll: each decrypts a beat after the one
    // above it. Keyed on the parent row, so a different submenu replays.
    let parent = if level == 0 { None } else { path.get(level - 1).copied().flatten() };
    let mut list = div().w(width).flex().flex_col().py_1();
    for (i, entry) in entries.iter().enumerate() {
        let cascade = crate::animate::play_after(
            ElementId::Name(format!("menu-row-{level}-{i}").into()),
            parent,
            crate::animate::stagger(i),
            crate::motion::FAST,
            window,
            cx,
        );
        // Each row churns its own noise, so the menu doesn't read as columns.
        let churn = crate::animate::Progress { frame: cascade.frame + i as u32 * 5, ..cascade };
        list = list.child(match entry {
            Entry::Separator => div().h(px(1.)).my_1().mx_2().bg(hsla(p.line)).into_any_element(),
            Entry::Label(label) => div()
                .h(lead + px(4.))
                .px_3()
                .pt_1()
                .display(Scale::X1, window)
                .text_color(hsla(p.fg_faint))
                .child(crate::animate::scramble(&label.to_uppercase(), churn))
                .into_any_element(),
            Entry::Item(_) | Entry::Submenu(_) => {
                let (label, row_icon, shortcut, checked, danger, disabled, opens) = match entry {
                    Entry::Item(it) => (it.label.clone(), it.icon, it.shortcut.clone(), it.checked, it.danger, it.disabled, false),
                    Entry::Submenu(s) => (s.label.clone(), s.icon, None, None, false, s.disabled, true),
                    _ => unreachable!(),
                };
                let active = highlight == Some(i) && !disabled;
                let (ink, bg) = match (disabled, danger, active) {
                    (true, _, _) => (p.fg_faint, None),
                    (false, true, true) => (p.danger_fg, Some(p.danger)),
                    (false, true, false) => (p.danger, None),
                    (false, false, true) => (p.fg, Some(p.accent_dim)),
                    (false, false, false) => (p.fg, None),
                };
                let lead_ink = if active && !danger { p.accent_text } else { ink };
                div()
                    .id(("item", level * 1000 + i))
                    .role(Role::MenuItem)
                    .aria_label(label.clone())
                    .when(opens, |el| el.aria_expanded(path.len() > level + 1 && active))
                    .when_some(checked, |el, c| el.aria_toggled(if c { gpui::Toggled::True } else { gpui::Toggled::False }))
                    .relative()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .h(crate::theme::row_height(cx))
                    .px_3()
                    .body(text::BASE)
                    .text_color(hsla(ink))
                    .when_some(bg, |el, bg| el.bg(hsla(bg)))
                    .when(active && !danger, |el| {
                        el.child(div().absolute().left_0().top_0().bottom_0().w(px(2.)).bg(hsla(p.accent)))
                    })
                    .when(has_lead, |el| {
                        let shown = match (checked, row_icon) {
                            (Some(true), _) => Some(Icon::Check),
                            (Some(false), _) => None,
                            (None, ic) => ic,
                        };
                        // Checks are amber, like a checkbox's `[x]`.
                        let ink = if checked == Some(true) && !disabled { p.accent_text } else { lead_ink };
                        el.child(div().size(lead).flex_shrink_0().when_some(shown, |el, l| el.child(icon(l).color(hsla(ink)))))
                    })
                    .child(crate::animate::nudge(
                        div().flex_1().min_w_0().overflow_hidden().whitespace_nowrap().child(crate::animate::scramble(&label, churn)),
                        gpui::point(row_slide(cascade), px(0.)),
                    ))
                    .when_some(shortcut, |el, k| {
                        el.child(
                            div()
                                .body(text::SM)
                                .text_color(hsla(if active && danger { p.danger_fg } else { p.fg_dim }))
                                .child(crate::animate::scramble(&k, churn)),
                        )
                    })
                    .when(opens, |el| {
                        el.child(div().size(lead).flex_shrink_0().child(icon(Icon::ChevronRight).color(hsla(if active { p.accent_text } else { p.fg_dim }))))
                    })
                    .when(!disabled, |el| {
                        let hover_state = state.clone();
                        let click_state = state.clone();
                        let item = match entry {
                            Entry::Item(it) => Some(it.clone()),
                            _ => None,
                        };
                        el.on_hover(move |hovered, _, cx| {
                            if *hovered {
                                let path = read_path(&hover_state, cx);
                                write_path(&hover_state, hover_path(&path, level, i, opens), cx);
                            }
                        })
                        .on_mouse_down(MouseButton::Left, |_, window, cx| {
                            window.prevent_default();
                            cx.stop_propagation();
                        })
                        .on_click(move |_, window, cx| match &item {
                            Some(item) => choose(&click_state, item, window, cx),
                            // A click on a submenu row opens it (if hover
                            // hasn't already); it never closes it.
                            None => {
                                let path = read_path(&click_state, cx);
                                write_path(&click_state, hover_path(&path, level, i, true), cx);
                            }
                        })
                    })
                    .into_any_element()
            }
        });
    }
    let panel = surface(list, cx);
    div().id(("menu-level", level)).role(Role::Menu).occlude().child(reveal(panel, crate::motion::FAST, window, cx))
}

impl RenderOnce for Menu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |_, cx| OverlayState::new(cx));
        let (open, at) = {
            let s = state.read(cx);
            (s.open, s.at)
        };
        let entries = Rc::new(self.entries);
        let first = choosable(&entries).first().copied();

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

        // ── The cascade ─────────────────────────────────────────────────
        // Every open level side by side in one flex row; each submenu is
        // pushed down so its first row is level with the row that opened it.
        let path = read_path(&state, cx);
        let label_h = display_size(Scale::X1, window) + px(4.);
        let mut levels = Vec::new();
        let mut top = px(0.);
        for level in 0..path.len() {
            let Some(level_items) = level_entries(&entries, &path, level) else { break };
            if level > 0 {
                let parent = level_entries(&entries, &path, level - 1).unwrap_or_default();
                let row = path[level - 1].unwrap_or(0);
                top += parent[..row].iter().map(|e| entry_height(e, label_h, crate::theme::row_height(cx))).fold(px(0.), |a, b| a + b);
            }
            levels.push(div().mt(top).child(render_level(level_items, level, &path, self.width, &state, window, cx)));
        }

        let focus = state.read(cx).focus.clone();
        let cascade = div()
            .id("menu-surface")
            .track_focus(&focus)
            .flex()
            .flex_row()
            .items_start()
            .on_key_down({
                let state = state.clone();
                let entries = entries.clone();
                move |ev, window, cx| {
                    let path = read_path(&state, cx);
                    match key_outcome(&entries, &path, ev.keystroke.key.as_str()) {
                        KeyOutcome::Ignore => return,
                        KeyOutcome::Close => state.update(cx, |s, cx| s.close(window, cx)),
                        KeyOutcome::Path(next) => write_path(&state, next, cx),
                        KeyOutcome::Choose { level, index } => {
                            if let Some(Entry::Item(item)) = level_entries(&entries, &path, level).and_then(|e| e.get(index)) {
                                choose(&state, &item.clone(), window, cx);
                            }
                        }
                    }
                    cx.stop_propagation();
                }
            })
            .on_mouse_down_out({
                let state = state.clone();
                move |_, window, cx| state.update(cx, |s, cx| s.close(window, cx))
            })
            .children(levels);

        root = match (self.kind, at) {
            (Kind::Context, Some(position)) => root.child(at_point(position, cascade)),
            _ => root.child(below(self.align, cascade)),
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

    /// root: [new, recent ▸ [a, b ▸ [x]], (sep), quit]
    fn nested() -> Vec<Entry> {
        vec![
            Entry::Item(menu_item("new")),
            Entry::Submenu(submenu("recent").item(menu_item("a")).submenu(submenu("b").item(menu_item("x")))),
            Entry::Separator,
            Entry::Item(menu_item("quit")),
        ]
    }

    #[test]
    fn keyboard_skips_labels_separators_and_disabled_rows() {
        assert_eq!(choosable(&entries()), vec![1, 4, 5]);
        assert_eq!(choosable(&nested()), vec![0, 1, 3], "submenu rows are stops");
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

    #[test]
    fn right_opens_left_closes_escape_steps_out() {
        let root = nested();
        let open = key_outcome(&root, &[Some(1)], "right");
        assert_eq!(open, KeyOutcome::Path(vec![Some(1), Some(0)]), "opens with first row highlighted");
        assert_eq!(key_outcome(&root, &[Some(1), Some(0)], "down"), KeyOutcome::Path(vec![Some(1), Some(1)]));
        assert_eq!(key_outcome(&root, &[Some(1), Some(1)], "enter"), KeyOutcome::Path(vec![Some(1), Some(1), Some(0)]));
        assert_eq!(key_outcome(&root, &[Some(1), Some(1), Some(0)], "left"), KeyOutcome::Path(vec![Some(1), Some(1)]));
        assert_eq!(key_outcome(&root, &[Some(1), Some(1)], "escape"), KeyOutcome::Path(vec![Some(1)]));
        assert_eq!(key_outcome(&root, &[Some(1)], "escape"), KeyOutcome::Close);
        assert_eq!(key_outcome(&root, &[Some(1)], "left"), KeyOutcome::Ignore);
        assert_eq!(key_outcome(&root, &[Some(0)], "right"), KeyOutcome::Ignore, "right on an item does nothing");
    }

    #[test]
    fn enter_chooses_at_the_deepest_level() {
        let root = nested();
        assert_eq!(key_outcome(&root, &[Some(1), Some(1), Some(0)], "enter"), KeyOutcome::Choose { level: 2, index: 0 });
        assert_eq!(key_outcome(&root, &[Some(3)], "space"), KeyOutcome::Choose { level: 0, index: 3 });
    }

    #[test]
    fn hover_opens_truncates_and_is_stable() {
        let opened = hover_path(&[None], 0, 1, true);
        assert_eq!(opened, vec![Some(1), None]);
        let deeper = vec![Some(1), Some(0)];
        assert_eq!(hover_path(&deeper, 0, 1, true), deeper, "re-entering the open row keeps its submenu state");
        assert_eq!(hover_path(&deeper, 0, 0, false), vec![Some(0)], "another row closes the submenu");
        assert_eq!(hover_path(&deeper, 1, 1, true), vec![Some(1), Some(1), None]);
    }

    #[test]
    fn rows_slide_in_from_the_left_and_rest() {
        use crate::animate::Progress;
        let at = |frame| Progress { t: 0.5, frame, done: false };
        assert_eq!(row_slide(at(0)), px(-6.));
        assert_eq!(row_slide(at(9)), px(0.));
        assert_eq!(row_slide(Progress::DONE), px(0.));
    }

    #[test]
    fn level_entries_follows_the_path() {
        let root = nested();
        assert_eq!(level_entries(&root, &[Some(1), Some(1)], 2).map(<[_]>::len), Some(1));
        assert!(level_entries(&root, &[Some(0)], 1).is_none(), "an item opens nothing");
    }
}
