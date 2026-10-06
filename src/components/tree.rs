//! The tree: nested rows with real connector lines, like `tree(1)` output.
//!
//! ```text
//!  > src/
//!    ├─ v components/
//!    │  ├─   button.rs
//!    │  └─   menu.rs          ← selected: amber bar + wash
//!    └─   lib.rs
//!  > docs/
//! ```
//!
//! The guides are drawn as 1px lines, not box-drawing glyphs, so they join
//! across rows at any row height and scale.
//!
//! - **Selection** is controlled: `.selected(Some(id))`, new id in
//!   `on_select`. **Expansion** is the tree's own state (seed it with
//!   `.expanded([...])`), so apps only track what they care about.
//! - **Mouse:** clicking a row selects it; clicking a branch also toggles it.
//! - **Keyboard** (one tab stop): ↑/↓ move, → expands (or steps into the
//!   first child), ← collapses (or steps out to the parent), Enter/Space
//!   toggle a branch, Home/End jump.

use std::collections::HashSet;
use std::rc::Rc;

use gpui::{
    App, ElementId, FocusHandle, InteractiveElement, IntoElement, MouseButton, ParentElement,
    Pixels, RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder as _, px,
};

use crate::fonts::{Scale, display_size};
use crate::icon::{Icon, icon};
use crate::theme::palette;
use crate::tokens::{hsla, text};
use crate::fonts::FerriteText;

type SelectHandler = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// One node. Its `id` must be unique in the tree (a path works well).
#[derive(Clone)]
pub struct TreeNode {
    id: SharedString,
    label: SharedString,
    icon: Option<Icon>,
    meta: Option<SharedString>,
    children: Vec<TreeNode>,
}

pub fn tree_node(id: impl Into<SharedString>, label: impl Into<SharedString>) -> TreeNode {
    TreeNode { id: id.into(), label: label.into(), icon: None, meta: None, children: Vec::new() }
}

impl TreeNode {
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Dim right-aligned text (a size, a count, a status).
    pub fn meta(mut self, meta: impl Into<SharedString>) -> Self {
        self.meta = Some(meta.into());
        self
    }

    pub fn child(mut self, node: TreeNode) -> Self {
        self.children.push(node);
        self
    }

    pub fn children(mut self, nodes: impl IntoIterator<Item = TreeNode>) -> Self {
        self.children.extend(nodes);
        self
    }
}

#[derive(IntoElement)]
pub struct Tree {
    id: ElementId,
    nodes: Vec<TreeNode>,
    selected: Option<SharedString>,
    initially_expanded: Vec<SharedString>,
    on_select: Option<SelectHandler>,
}

pub fn tree(id: impl Into<ElementId>) -> Tree {
    Tree { id: id.into(), nodes: Vec::new(), selected: None, initially_expanded: Vec::new(), on_select: None }
}

impl Tree {
    pub fn node(mut self, node: TreeNode) -> Self {
        self.nodes.push(node);
        self
    }

    pub fn nodes(mut self, nodes: impl IntoIterator<Item = TreeNode>) -> Self {
        self.nodes.extend(nodes);
        self
    }

    pub fn selected(mut self, id: Option<impl Into<SharedString>>) -> Self {
        self.selected = id.map(Into::into);
        self
    }

    /// Branches open on first render. After that the tree owns expansion.
    pub fn expanded(mut self, ids: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.initially_expanded.extend(ids.into_iter().map(Into::into));
        self
    }

    pub fn on_select(mut self, handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

// ── Flattening + keys (pure, tested) ──────────────────────────────────────

/// A visible row.
#[derive(Clone)]
struct Flat<'a> {
    node: &'a TreeNode,
    depth: usize,
    /// For each ancestor column (depth 1..), whether its line continues
    /// below this row (that ancestor has later siblings).
    rails: Vec<bool>,
    last: bool,
    parent: Option<usize>,
    /// Position among its siblings, for the appear cascade.
    sibling: usize,
}

fn flatten<'a>(nodes: &'a [TreeNode], expanded: &HashSet<SharedString>) -> Vec<Flat<'a>> {
    fn walk<'a>(nodes: &'a [TreeNode], depth: usize, rails: &[bool], parent: Option<usize>, expanded: &HashSet<SharedString>, out: &mut Vec<Flat<'a>>) {
        for (i, node) in nodes.iter().enumerate() {
            let last = i + 1 == nodes.len();
            let index = out.len();
            out.push(Flat { node, depth, rails: rails.to_vec(), last, parent, sibling: i });
            if !node.children.is_empty() && expanded.contains(&node.id) {
                let mut child_rails = rails.to_vec();
                if depth > 0 {
                    child_rails.push(!last);
                }
                walk(&node.children, depth + 1, &child_rails, Some(index), expanded, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(nodes, 0, &[], None, expanded, &mut out);
    out
}

#[derive(Debug, PartialEq)]
enum Outcome {
    Select(SharedString),
    Toggle(SharedString),
    None,
}

fn key_outcome(rows: &[Flat], selected: Option<&SharedString>, expanded: &HashSet<SharedString>, key: &str) -> Outcome {
    if rows.is_empty() {
        return Outcome::None;
    }
    let at = selected.and_then(|s| rows.iter().position(|r| &r.node.id == s));
    let pick = |i: usize| Outcome::Select(rows[i].node.id.clone());
    let Some(at) = at else {
        return match key {
            "up" | "down" | "home" | "end" | "left" | "right" => pick(if key == "end" || key == "up" { rows.len() - 1 } else { 0 }),
            _ => Outcome::None,
        };
    };
    let row = &rows[at];
    let branch = !row.node.children.is_empty();
    let open = expanded.contains(&row.node.id);
    match key {
        "down" => pick((at + 1).min(rows.len() - 1)),
        "up" => pick(at.saturating_sub(1)),
        "home" => pick(0),
        "end" => pick(rows.len() - 1),
        "right" if branch && !open => Outcome::Toggle(row.node.id.clone()),
        "right" if branch => pick(at + 1),
        "left" if branch && open => Outcome::Toggle(row.node.id.clone()),
        "left" => row.parent.map_or(Outcome::None, pick),
        "enter" | "space" if branch => Outcome::Toggle(row.node.id.clone()),
        _ => Outcome::None,
    }
}

// ── Rendering ─────────────────────────────────────────────────────────────

const ROW: Pixels = px(28.);

struct TreeState {
    focus: FocusHandle,
    expanded: HashSet<SharedString>,
}

impl RenderOnce for Tree {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let seed = self.initially_expanded.clone();
        let state = window.use_keyed_state(self.id.clone(), cx, move |_, cx| TreeState { focus: cx.focus_handle(), expanded: seed.into_iter().collect() });
        let focus = state.read(cx).focus.clone();
        let expanded = state.read(cx).expanded.clone();
        let rows = flatten(&self.nodes, &expanded);
        let lead = display_size(Scale::X1, window);
        let guide = hsla(p.line_strong);
        // Indent columns are as wide as the chevron, and lines run down
        // their middle, so a child's guide drops straight out of its
        // parent's chevron.
        let indent = lead;
        let mid = (lead / 2.).floor();

        let toggle = {
            let state = state.clone();
            move |id: &SharedString, cx: &mut App| {
                state.update(cx, |s, cx| {
                    if !s.expanded.remove(id) {
                        s.expanded.insert(id.clone());
                    }
                    cx.notify();
                })
            }
        };

        let mut list = div().flex().flex_col().py_1();
        for (ri, row) in rows.iter().enumerate() {
            let node = row.node;
            let branch = !node.children.is_empty();
            let open = expanded.contains(&node.id);
            let active = self.selected.as_ref() == Some(&node.id);

            // Guides: one column per ancestor below the root level, then the
            // connector into this row.
            let mut guides = div().flex().flex_row().h_full().flex_none();
            for &rail in &row.rails {
                guides = guides.child(
                    div().relative().w(indent).h_full().when(rail, |el| {
                        el.child(div().absolute().left(mid).top_0().bottom_0().w(px(1.)).bg(guide))
                    }),
                );
            }
            if row.depth > 0 {
                guides = guides.child(
                    div()
                        .relative()
                        .w(indent)
                        .h_full()
                        // ├ or └: the vertical runs to the middle, and on if siblings follow.
                        .child(div().absolute().left(mid).top_0().w(px(1.)).h(if row.last { ROW / 2. } else { ROW }).bg(guide))
                        .child(div().absolute().left(mid).right_0().top(ROW / 2.).h(px(1.)).bg(guide)),
                );
            }

            let id = node.id.clone();
            let ink = if active { p.accent_text } else { p.fg_dim };
            list = list.child(
                div()
                    .id(("node", ri))
                    .role(Role::TreeItem)
                    .aria_label(node.label.clone())
                    .aria_level(row.depth + 1)
                    .aria_selected(active)
                    .when(branch, |el| el.aria_expanded(open))
                    .relative()
                    .flex()
                    .flex_row()
                    .items_center()
                    .h(ROW)
                    .pl_2()
                    .pr_3()
                    .children(super::selection(format!("{}-{}", self.id, node.id), active, window, cx))
                    .when(!active, |el| el.hover(|s| s.bg(hsla(p.raised))))
                    .child(guides)
                    .child(div().relative().size(lead).flex_none().map(|el| {
                        if branch {
                            el.child(icon(if open { Icon::ChevronDown } else { Icon::ChevronRight }).color(hsla(ink)))
                        } else if row.depth > 0 {
                            // A leaf's connector runs on through the empty chevron slot.
                            el.child(div().absolute().left_0().right(px(4.)).top(lead / 2.).h(px(1.)).bg(guide))
                        } else {
                            el
                        }
                    }))
                    .child(div().size(lead).flex_none().when_some(node.icon, |el, i| el.child(icon(i).color(hsla(ink)))))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .pl_1()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .body(text::BASE)
                            .text_color(hsla(p.fg))
                            .child({
                                let p = crate::animate::play_after(
                                    ElementId::Name(format!("{}-in-{}", self.id, node.id).into()),
                                    0u8,
                                    crate::animate::stagger(row.sibling),
                                    crate::motion::BASE,
                                    window,
                                    cx,
                                );
                                crate::animate::scramble(&node.label, p)
                            }),
                    )
                    .when_some(node.meta.clone(), |el, m| el.child(div().body(text::XS).text_color(hsla(p.fg_faint)).child(m)))
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .on_click({
                        let toggle = toggle.clone();
                        let handler = self.on_select.clone();
                        let focus = focus.clone();
                        move |_, window, cx| {
                            focus.focus(window, cx);
                            if branch {
                                toggle(&id, cx);
                            }
                            if let Some(h) = &handler {
                                h(&id, window, cx);
                            }
                        }
                    }),
            );
        }

        // Keys need the flattened rows; rebuild them from owned nodes.
        let nodes = Rc::new(self.nodes);
        let selected = self.selected.clone();
        div()
            .id(self.id.clone())
            .role(Role::Tree)
            .track_focus(&focus)
            .tab_stop(true)
            .border_1()
            .border_color(gpui::transparent_black())
            .focus_visible(|s| s.border_color(hsla(p.accent)))
            .on_key_down({
                let state = state.clone();
                let handler = self.on_select.clone();
                move |ev, window, cx| {
                    let expanded = state.read(cx).expanded.clone();
                    let rows = flatten(&nodes, &expanded);
                    match key_outcome(&rows, selected.as_ref(), &expanded, ev.keystroke.key.as_str()) {
                        Outcome::None => return,
                        Outcome::Toggle(id) => toggle(&id, cx),
                        Outcome::Select(id) => {
                            if let Some(h) = &handler {
                                h(&id, window, cx);
                            }
                        }
                    }
                    cx.stop_propagation();
                }
            })
            .child(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<TreeNode> {
        vec![
            tree_node("src", "src/")
                .child(tree_node("src/c", "components/").child(tree_node("src/c/b", "button.rs")).child(tree_node("src/c/m", "menu.rs")))
                .child(tree_node("src/lib", "lib.rs")),
            tree_node("docs", "docs/").child(tree_node("docs/a", "a.md")),
        ]
    }

    fn ids<'a>(rows: &'a [Flat<'a>]) -> Vec<&'a str> {
        rows.iter().map(|r| r.node.id.as_ref()).collect()
    }

    #[test]
    fn flatten_shows_only_expanded_branches() {
        let nodes = sample();
        let none = HashSet::new();
        assert_eq!(ids(&flatten(&nodes, &none)), ["src", "docs"]);
        let open: HashSet<SharedString> = ["src".into(), "src/c".into()].into_iter().collect();
        let rows = flatten(&nodes, &open);
        assert_eq!(ids(&rows), ["src", "src/c", "src/c/b", "src/c/m", "src/lib", "docs"]);
        // components/ is not last under src, so its children carry a rail.
        assert_eq!(rows[2].rails, vec![true]);
        assert!(rows[3].last && rows[4].last);
        assert_eq!(rows[2].parent, Some(1));
    }

    #[test]
    fn arrows_expand_collapse_and_step() {
        let nodes = sample();
        let mut open: HashSet<SharedString> = HashSet::new();
        let rows = flatten(&nodes, &open);
        let src: SharedString = "src".into();
        assert_eq!(key_outcome(&rows, Some(&src), &open, "right"), Outcome::Toggle(src.clone()));
        open.insert(src.clone());
        let rows = flatten(&nodes, &open);
        assert_eq!(key_outcome(&rows, Some(&src), &open, "right"), Outcome::Select("src/c".into()), "steps into first child");
        let lib: SharedString = "src/lib".into();
        assert_eq!(key_outcome(&rows, Some(&lib), &open, "left"), Outcome::Select(src.clone()), "steps out to the parent");
        assert_eq!(key_outcome(&rows, Some(&src), &open, "left"), Outcome::Toggle(src.clone()), "collapses");
        assert_eq!(key_outcome(&rows, None, &open, "down"), Outcome::Select(src), "starts at the top");
    }
}
