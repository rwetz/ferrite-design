//! The command palette: a modal list of everything an app can do, filtered
//! as you type.
//!
//! ```text
//!  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░   ← the app, screen-doored
//!  ░░ ┌──────────────────────────────────────┐ ░
//!  ░░ │ > dep█                         3/24 │ ░   prompt + input + count
//!  ░░ ├──────────────────────────────────────┤ ░
//!  ░░ ▌ ↑ Run: DEPloy              Ctrl+D    │ ░   matches highlighted amber
//!  ░░ │   Delete paper                       │ ░
//!  ░░ ├──────────────────────────────────────┤ ░
//!  ░░ │ [↑][↓] move  [ENTER] run  [ESC] close│ ░
//!  ░░ └──────────────────────────────────────┘▒░   hard dithered shadow
//! ```
//!
//! - **Empty query:** every command, grouped under display-face headings, in
//!   the order you registered them.
//! - **Typing:** a flat list ranked by [`crate::fuzzy`], best first, with
//!   the matched characters in amber and the group as dim meta.
//! - **Keys:** ↑/↓ move (wrapping), Enter runs, Escape closes. These are
//!   *key bindings* in the `FerritePalette` context, not key listeners:
//!   gpui dispatches bindings before listeners, so a listener would never
//!   see the arrows the focused input already binds (PITFALLS §25).
//! - **Focus:** opening remembers the focused element and focuses the
//!   input; closing gives focus back.
//! - **Running** a command closes the palette first, then calls the handler
//!   on the next tick (`window.defer`), so a handler may freely update any
//!   entity — including the palette.
//!
//! The text field is Ferrite's own [`TextInput`](super::input::TextInput),
//! drawn borderless. It binds Enter (→ `Submit`) but leaves ↑/↓ and Escape
//! unbound, so they fall through to the palette's own bindings.

use std::rc::Rc;

use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, HighlightStyle, InteractiveElement,
    IntoElement, KeyBinding, MouseButton, ParentElement, Render, Role, ScrollHandle, SharedString,
    StatefulInteractiveElement, Styled, StyledText, Subscription, Window, actions, anchored,
    deferred, div, point, prelude::FluentBuilder as _, px,
};
use super::input::{InputEvent, TextInput};

use super::overlay::{reveal, scrim, surface};
use super::tooltip::kbd;
use crate::dither::{self, dither};
use crate::fonts::{FerriteText, Scale, display_size};
use crate::fuzzy::fuzzy_match;
use crate::icon::{Icon, icon};
use crate::theme::palette;
use crate::tokens::{hsla, text};

actions!(ferrite_palette, [SelectPrev, SelectNext, Confirm, Dismiss, TogglePalette]);

/// The key context the palette's bindings live in.
pub const CONTEXT: &str = "FerritePalette";

/// Registers the palette's navigation bindings. Called by `ferrite_design::init`.
/// Apps bind [`TogglePalette`] themselves, e.g.
/// `KeyBinding::new("ctrl-shift-p", TogglePalette, None)`.
pub fn init(cx: &mut App) {
    let ctx = Some(CONTEXT);
    cx.bind_keys([
        KeyBinding::new("up", SelectPrev, ctx),
        KeyBinding::new("down", SelectNext, ctx),
        KeyBinding::new("enter", Confirm, ctx),
        KeyBinding::new("escape", Dismiss, ctx),
    ]);
}

type RunHandler = Rc<dyn Fn(&mut Window, &mut App)>;

/// One command. Build with [`command`].
#[derive(Clone)]
pub struct PaletteCommand {
    label: SharedString,
    group: Option<SharedString>,
    icon: Option<Icon>,
    shortcut: Option<SharedString>,
    keywords: Vec<SharedString>,
    run: Option<RunHandler>,
}

pub fn command(label: impl Into<SharedString>) -> PaletteCommand {
    PaletteCommand { label: label.into(), group: None, icon: None, shortcut: None, keywords: Vec::new(), run: None }
}

impl PaletteCommand {
    /// Section heading when the query is empty; dim meta when filtering.
    pub fn group(mut self, group: impl Into<SharedString>) -> Self {
        self.group = Some(group.into());
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Display-only shortcut text.
    pub fn shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Extra words that also match (scored lower than the label, never
    /// highlighted): synonyms, old names.
    pub fn keywords(mut self, words: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.keywords.extend(words.into_iter().map(Into::into));
        self
    }

    pub fn on_run(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.run = Some(Rc::new(handler));
        self
    }
}

#[derive(Clone)]
enum Row {
    Heading(SharedString),
    /// Command index + matched label positions (chars).
    Item(usize, Vec<usize>),
}

/// Rank `commands` against `query` into display rows. Pure, so it's tested.
fn build_rows(commands: &[PaletteCommand], query: &str) -> Vec<Row> {
    if query.trim().is_empty() {
        // Grouped, in registration order of each group's first command.
        let mut groups: Vec<Option<SharedString>> = Vec::new();
        for c in commands {
            if !groups.contains(&c.group) {
                groups.push(c.group.clone());
            }
        }
        let mut rows = Vec::new();
        for g in groups {
            if let Some(name) = &g {
                rows.push(Row::Heading(name.clone()));
            }
            rows.extend(commands.iter().enumerate().filter(|(_, c)| c.group == g).map(|(i, _)| Row::Item(i, Vec::new())));
        }
        return rows;
    }
    let mut scored: Vec<(i32, usize, Vec<usize>)> = commands
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            let label = fuzzy_match(query, &c.label).map(|m| (m.score, m.positions));
            // Keywords count, but less than the label, and highlight nothing.
            let keyword = c
                .keywords
                .iter()
                .filter_map(|k| fuzzy_match(query, k))
                .map(|m| (m.score * 3 / 4, Vec::new()))
                .max_by_key(|(s, _)| *s);
            match (label, keyword) {
                (Some(l), Some(k)) => Some(if l.0 >= k.0 { l } else { k }),
                (l, k) => l.or(k),
            }
            .map(|(s, p)| (s, i, p))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(commands[a.1].label.len().cmp(&commands[b.1].label.len())).then(a.1.cmp(&b.1)));
    scored.into_iter().map(|(_, i, p)| Row::Item(i, p)).collect()
}

/// The palette. Create once per window, render it as a child of the root
/// view (it draws nothing while closed), and open it from an action.
pub struct CommandPalette {
    input: Entity<TextInput>,
    commands: Vec<PaletteCommand>,
    rows: Vec<Row>,
    /// Index into `rows` of the highlighted item.
    highlight: Option<usize>,
    open: bool,
    previous_focus: Option<FocusHandle>,
    /// A focus point that always exists inside the app's tree (the
    /// palette's own idle element). See [`CommandPalette::new`].
    home: FocusHandle,
    scroll: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl CommandPalette {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| TextInput::new(window, cx).placeholder("Type a command…").bordered(false));
        let subscriptions = vec![cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| match event {
            InputEvent::Change => this.refilter(cx),
            // The field binds Enter itself, so the palette's Confirm binding
            // never sees it; run the command from the field's Submit instead.
            InputEvent::Submit => this.confirm(&Confirm, window, cx),
        })];
        // With nothing focused, gpui sends keystrokes only to the window's
        // root view — not to the app's own root, where `TogglePalette` is
        // handled — so Ctrl+Shift+P would do nothing. Park focus on an
        // element inside the app until something else takes it, and come
        // back here when the palette closes with nowhere else to go.
        let home = cx.focus_handle();
        if window.focused(cx).is_none() {
            home.focus(window, cx);
        }
        Self {
            input,
            commands: Vec::new(),
            rows: Vec::new(),
            highlight: None,
            open: false,
            previous_focus: None,
            home,
            scroll: ScrollHandle::new(),
            _subscriptions: subscriptions,
        }
    }

    pub fn set_commands(&mut self, commands: Vec<PaletteCommand>, cx: &mut Context<Self>) {
        self.commands = commands;
        self.refilter(cx);
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            return;
        }
        self.previous_focus = window.focused(cx);
        self.open = true;
        self.input.update(cx, |input, cx| {
            input.set_value("", cx);
            input.focus(window, cx);
        });
        self.refilter(cx);
        cx.notify();
    }

    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        self.open = false;
        let back = self.previous_focus.take().unwrap_or_else(|| self.home.clone());
        back.focus(window, cx);
        cx.notify();
    }

    pub fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open { self.close(window, cx) } else { self.open(window, cx) }
    }

    fn refilter(&mut self, cx: &mut Context<Self>) {
        let query = self.input.read(cx).value();
        self.rows = build_rows(&self.commands, &query);
        self.highlight = self.items().first().copied();
        self.scroll.scroll_to_item(0);
        cx.notify();
    }

    /// Row indices that are items (the keyboard's stops).
    fn items(&self) -> Vec<usize> {
        self.rows.iter().enumerate().filter_map(|(i, r)| matches!(r, Row::Item(..)).then_some(i)).collect()
    }

    fn step(&mut self, delta: isize, cx: &mut Context<Self>) {
        let items = self.items();
        if items.is_empty() {
            return;
        }
        let pos = self.highlight.and_then(|h| items.iter().position(|&i| i == h)).unwrap_or(0) as isize;
        let next = items[(pos + delta).rem_euclid(items.len() as isize) as usize];
        self.highlight = Some(next);
        self.scroll.scroll_to_item(next);
        cx.notify();
    }

    fn select_prev(&mut self, _: &SelectPrev, _: &mut Window, cx: &mut Context<Self>) {
        self.step(-1, cx);
    }

    fn select_next(&mut self, _: &SelectNext, _: &mut Window, cx: &mut Context<Self>) {
        self.step(1, cx);
    }

    fn dismiss(&mut self, _: &Dismiss, window: &mut Window, cx: &mut Context<Self>) {
        self.close(window, cx);
    }

    fn confirm(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        if let Some(Row::Item(i, _)) = self.highlight.and_then(|h| self.rows.get(h)) {
            self.run(*i, window, cx);
        }
    }

    fn run(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let handler = self.commands.get(index).and_then(|c| c.run.clone());
        self.close(window, cx);
        if let Some(handler) = handler {
            // Next tick: the handler may update this palette, which is
            // mid-update right now.
            window.defer(cx, move |window, cx| handler(window, cx));
        }
    }
}

/// Byte ranges for the matched chars of `label`, for `StyledText`.
fn highlight_ranges(label: &str, positions: &[usize], style: HighlightStyle) -> Vec<(std::ops::Range<usize>, HighlightStyle)> {
    label
        .char_indices()
        .enumerate()
        .filter(|(ci, _)| positions.contains(ci))
        .map(|(_, (byte, ch))| (byte..byte + ch.len_utf8(), style))
        .collect()
}

impl Render for CommandPalette {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Always rendered, open or not: it's the home focus point.
        let home = div().id("palette-home").track_focus(&self.home);
        if !self.open {
            return home.into_any_element();
        }
        let p = palette(cx);
        let open = crate::animate::play("open", 0u8, crate::motion::BASE, window, cx);
        let viewport = window.viewport_size();
        let total = self.commands.len();
        let shown = self.items().len();
        let filtering = !self.input.read(cx).value().trim().is_empty();
        let accent = HighlightStyle { color: Some(hsla(p.accent_text)), ..Default::default() };
        let lead = display_size(Scale::X1, window);

        let mut list = div().id("palette-list").track_scroll(&self.scroll).overflow_y_scroll().max_h(px(28. * 11.)).py_1();
        for (ri, row) in self.rows.iter().enumerate() {
            list = list.child(match row {
                Row::Heading(name) => div()
                    .px_3()
                    .pt_1()
                    .display(Scale::X1, window)
                    .text_color(hsla(p.fg_faint))
                    .child(name.to_uppercase())
                    .into_any_element(),
                Row::Item(ci, positions) => {
                    let c = &self.commands[*ci];
                    let active = self.highlight == Some(ri);
                    let label = StyledText::new(c.label.clone()).with_highlights(highlight_ranges(&c.label, positions, accent));
                    let index = *ci;
                    div()
                        .id(("cmd", ri))
                        .role(Role::ListBoxOption)
                        .aria_selected(active)
                        .aria_label(c.label.clone())
                        .relative()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .h(px(28.))
                        .px_3()
                        .body(text::BASE)
                        .text_color(hsla(p.fg))
                        .when(active, |el| {
                            el.bg(hsla(p.accent_dim))
                                .child(div().absolute().left_0().top_0().bottom_0().w(px(2.)).bg(hsla(p.accent)))
                        })
                        .child(div().size(lead).flex_shrink_0().when_some(c.icon, |el, i| {
                            el.child(icon(i).color(hsla(if active { p.accent_text } else { p.fg_dim })))
                        }))
                        .child(div().flex_1().min_w_0().overflow_hidden().whitespace_nowrap().child(label))
                        .when_some(c.group.clone().filter(|_| filtering), |el, g| {
                            el.child(div().body(text::XS).text_color(hsla(p.fg_faint)).child(g.to_uppercase()))
                        })
                        .when_some(c.shortcut.clone(), |el, k| el.child(div().body(text::SM).text_color(hsla(p.fg_dim)).child(k)))
                        .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                            if *hovered && this.highlight != Some(ri) {
                                this.highlight = Some(ri);
                                cx.notify();
                            }
                        }))
                        .on_mouse_down(MouseButton::Left, |_, window, cx| {
                            window.prevent_default();
                            cx.stop_propagation();
                        })
                        .on_click(cx.listener(move |this, _, window, cx| this.run(index, window, cx)))
                        .into_any_element()
                }
            });
        }
        if shown == 0 {
            list = list.child(
                div()
                    .relative()
                    .h(px(96.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(div().absolute().inset_0().child(dither(dither::radial(0.0, dither::level::LIGHT)).ink(hsla(p.line)).size_full()))
                    .child(div().px_2().bg(hsla(p.raised)).display(Scale::X1, window).text_color(hsla(p.fg_dim)).child("NO MATCH")),
            );
        }

        let hint = |keys: &[&str], what: &'static str| {
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_1()
                .children(keys.iter().map(|k| kbd(k)))
                .child(div().body(text::XS).text_color(hsla(p.fg_dim)).child(what))
        };

        let body = div()
            .w(px(640.))
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .h(px(44.))
                    .px_3()
                    .child(div().display(Scale::X1, window).text_color(hsla(p.accent)).child(">"))
                    .child(div().flex_1().body(text::LG).child(self.input.clone()))
                    .child(div().display(Scale::X1, window).text_color(hsla(p.fg_dim)).child(format!("{shown}/{total}"))),
            )
            .child(div().h(px(1.)).bg(hsla(p.line_strong)))
            .child(list)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_4()
                    .h(px(30.))
                    .px_3()
                    .bg(hsla(p.surface))
                    .border_t_1()
                    .border_color(hsla(p.line))
                    .child(hint(&["Up", "Down"], "move"))
                    .child(hint(&["Enter"], "run"))
                    .child(hint(&["Esc"], "close")),
            );

        // Full-window layer: a screen-door scrim (the app ~69% dithered into
        // the page color), then the panel near the top.
        home.child(deferred(
            anchored().position(point(px(0.), px(0.))).child(
                div()
                    .id("palette-layer")
                    .w(viewport.width)
                    .h(viewport.height)
                    .occlude()
                    .key_context(CONTEXT)
                    .on_action(cx.listener(Self::select_prev))
                    .on_action(cx.listener(Self::select_next))
                    .on_action(cx.listener(Self::confirm))
                    .on_action(cx.listener(Self::dismiss))
                    .child(
                        div()
                            .id("palette-scrim")
                            .absolute()
                            .inset_0()
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.close(window, cx)))
                            .child(scrim(open.eased(), cx)),
                    )
                    .child(
                        div()
                            .absolute()
                            .top(viewport.height * 0.12)
                            .left_0()
                            .right_0()
                            .flex()
                            .justify_center()
                            .child(div().id("palette").role(Role::Dialog).aria_label("Command palette").occlude().child(reveal(surface(body, cx), crate::motion::BASE, window, cx))),
                    ),
            ),
        )
        .with_priority(2))
        .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmds() -> Vec<PaletteCommand> {
        vec![
            command("New file").group("File"),
            command("Open folder").group("File"),
            command("Push").group("Git").keywords(["upload", "publish"]),
            command("Deploy").group("Run"),
            command("Stage all").group("Git"),
        ]
    }

    fn labels(rows: &[Row], cmds: &[PaletteCommand]) -> Vec<String> {
        rows.iter()
            .map(|r| match r {
                Row::Heading(h) => format!("# {h}"),
                Row::Item(i, _) => cmds[*i].label.to_string(),
            })
            .collect()
    }

    #[test]
    fn empty_query_groups_in_first_seen_order() {
        let c = cmds();
        assert_eq!(
            labels(&build_rows(&c, ""), &c),
            ["# File", "New file", "Open folder", "# Git", "Push", "Stage all", "# Run", "Deploy"]
        );
    }

    #[test]
    fn query_ranks_flat_without_headings() {
        let c = cmds();
        let rows = build_rows(&c, "op");
        assert_eq!(labels(&rows, &c)[0], "Open folder");
        assert!(rows.iter().all(|r| matches!(r, Row::Item(..))));
    }

    #[test]
    fn keywords_match_but_dont_highlight() {
        let c = cmds();
        let rows = build_rows(&c, "publish");
        assert_eq!(labels(&rows, &c), ["Push"]);
        assert!(matches!(&rows[0], Row::Item(_, p) if p.is_empty()));
    }

    #[test]
    fn highlight_ranges_are_byte_ranges() {
        let s = HighlightStyle::default();
        let r = highlight_ranges("añb", &[1, 2], s);
        assert_eq!(r.iter().map(|(r, _)| r.clone()).collect::<Vec<_>>(), vec![1..3, 3..4]);
    }
}
