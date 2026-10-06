//! The text input: a single-line field, written on gpui's own text-input
//! plumbing (`EntityInputHandler`), so it gets the platform IME, dead keys
//! and the character palette for free.
//!
//! ```text
//!  ┌──────────────────────────────────────┐
//!  │ > ferrite-design▌                    │   sunken well, amber caret
//!  └──────────────────────────────────────┘   frame turns amber when focused
//! ```
//!
//! - **Editing:** grapheme-aware ←/→, Ctrl+←/→ by word (⌥ on macOS),
//!   Home/End, Shift to extend any of them, Backspace/Delete (+Ctrl for a
//!   word), select all, cut/copy/paste, undo/redo (typing runs coalesce into
//!   one step).
//! - **Mouse:** click to place, drag to select, double-click a word,
//!   triple-click everything, Shift+click to extend.
//! - **Long text** scrolls horizontally to keep the caret in view.
//! - **Caret** is a 2px amber bar that blinks at `motion::BLINK` while
//!   focused (solid under reduced motion, and right after every keystroke).
//!   The blink timer exists only while the field has focus.
//! - **Password** fields (`.masked(true)`) draw `•` per character and refuse
//!   to copy or cut.
//! - **Events:** [`InputEvent::Change`] on every edit, [`InputEvent::Submit`]
//!   on Enter. ↑/↓, Escape and Tab are deliberately unbound, so they reach
//!   whatever contains the field (a palette's list, a dialog).
//!
//! It's an entity: `cx.new(|cx| TextInput::new(cx).placeholder("…"))`, then
//! render the entity as a child.

use std::ops::Range;
use std::time::Instant;

use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, Element, ElementId, ElementInputHandler,
    Entity, EntityInputHandler, EventEmitter, FocusHandle, Focusable, GlobalElementId,
    InteractiveElement, IntoElement, KeyBinding, LayoutId, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Point, Render, Role, ShapedLine,
    SharedString, StatefulInteractiveElement, Style, Styled, Subscription, Task, TextRun, UTF16Selection, UnderlineStyle,
    Window, actions, div, fill, point, prelude::FluentBuilder as _, px, relative, size,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::fonts::{FerriteText, Scale};
use crate::motion;
use crate::theme::palette;
use crate::tokens::{hsla, text};

actions!(
    ferrite_input,
    [
        Backspace,
        Delete,
        DeleteWordLeft,
        DeleteWordRight,
        Left,
        Right,
        WordLeft,
        WordRight,
        SelectLeft,
        SelectRight,
        SelectWordLeft,
        SelectWordRight,
        SelectAll,
        Home,
        End,
        SelectHome,
        SelectEnd,
        Paste,
        Cut,
        Copy,
        Undo,
        Redo,
        Submit,
        ShowCharacterPalette,
    ]
);

/// The key context the field's bindings live in.
pub const CONTEXT: &str = "FerriteInput";

/// Registers the field's bindings. Called by `ferrite_design::init`.
pub fn init(cx: &mut App) {
    let ctx = Some(CONTEXT);
    // Word motion is Alt on macOS, Ctrl elsewhere.
    let word = if cfg!(target_os = "macos") { "alt" } else { "ctrl" };
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, ctx),
        KeyBinding::new("shift-backspace", Backspace, ctx),
        KeyBinding::new("delete", Delete, ctx),
        KeyBinding::new(&format!("{word}-backspace"), DeleteWordLeft, ctx),
        KeyBinding::new(&format!("{word}-delete"), DeleteWordRight, ctx),
        KeyBinding::new("left", Left, ctx),
        KeyBinding::new("right", Right, ctx),
        KeyBinding::new(&format!("{word}-left"), WordLeft, ctx),
        KeyBinding::new(&format!("{word}-right"), WordRight, ctx),
        KeyBinding::new("shift-left", SelectLeft, ctx),
        KeyBinding::new("shift-right", SelectRight, ctx),
        KeyBinding::new(&format!("{word}-shift-left"), SelectWordLeft, ctx),
        KeyBinding::new(&format!("{word}-shift-right"), SelectWordRight, ctx),
        KeyBinding::new("home", Home, ctx),
        KeyBinding::new("end", End, ctx),
        KeyBinding::new("shift-home", SelectHome, ctx),
        KeyBinding::new("shift-end", SelectEnd, ctx),
        KeyBinding::new("secondary-a", SelectAll, ctx),
        KeyBinding::new("secondary-v", Paste, ctx),
        KeyBinding::new("secondary-c", Copy, ctx),
        KeyBinding::new("secondary-x", Cut, ctx),
        KeyBinding::new("secondary-z", Undo, ctx),
        KeyBinding::new("secondary-shift-z", Redo, ctx),
        KeyBinding::new("ctrl-y", Redo, ctx),
        KeyBinding::new("enter", Submit, ctx),
        KeyBinding::new("ctrl-cmd-space", ShowCharacterPalette, ctx),
    ]);
    if cfg!(target_os = "macos") {
        cx.bind_keys([
            KeyBinding::new("cmd-left", Home, ctx),
            KeyBinding::new("cmd-right", End, ctx),
            KeyBinding::new("cmd-shift-left", SelectHome, ctx),
            KeyBinding::new("cmd-shift-right", SelectEnd, ctx),
        ]);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    /// The text changed (typing, paste, cut, undo…).
    Change,
    /// Enter was pressed.
    Submit,
}

// ── The edit buffer (pure, tested) ────────────────────────────────────────

#[derive(Clone)]
struct Snapshot {
    text: String,
    selection: Range<usize>,
}

#[derive(Clone, Copy, PartialEq)]
enum EditKind {
    Typing,
    Other,
}

/// Text + selection + history. All offsets are UTF-8 byte offsets on
/// grapheme boundaries.
#[derive(Clone, Default)]
struct Buffer {
    text: String,
    selection: Range<usize>,
    reversed: bool,
    marked: Option<Range<usize>>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    last_edit: Option<EditKind>,
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

impl Buffer {
    fn cursor(&self) -> usize {
        if self.reversed { self.selection.start } else { self.selection.end }
    }

    fn prev_boundary(&self, offset: usize) -> usize {
        self.text.grapheme_indices(true).rev().find_map(|(i, _)| (i < offset).then_some(i)).unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.text.grapheme_indices(true).find_map(|(i, _)| (i > offset).then_some(i)).unwrap_or(self.text.len())
    }

    /// Start of the word at or before `offset` (skipping separators first).
    fn prev_word(&self, offset: usize) -> usize {
        let before = &self.text[..offset];
        let mut chars = before.char_indices().rev().peekable();
        while chars.peek().is_some_and(|&(_, c)| !is_word(c)) {
            chars.next();
        }
        let mut start = chars.peek().map_or(0, |&(i, _)| i);
        while let Some(&(i, c)) = chars.peek() {
            if !is_word(c) {
                break;
            }
            start = i;
            chars.next();
        }
        start
    }

    /// End of the word at or after `offset` (skipping separators first).
    fn next_word(&self, offset: usize) -> usize {
        let after = &self.text[offset..];
        let mut chars = after.char_indices().peekable();
        while chars.peek().is_some_and(|&(_, c)| !is_word(c)) {
            chars.next();
        }
        while chars.peek().is_some_and(|&(_, c)| is_word(c)) {
            chars.next();
        }
        offset + chars.peek().map_or(after.len(), |&(i, _)| i)
    }

    /// The word (or run of separators) around `offset`, for double-click.
    fn word_at(&self, offset: usize) -> Range<usize> {
        let here = self.text[offset..].chars().next().or_else(|| self.text[..offset].chars().next_back());
        let Some(here) = here else { return 0..0 };
        let same = |c: char| is_word(c) == is_word(here) && !c.is_whitespace() == !here.is_whitespace();
        let start = self.text[..offset].char_indices().rev().take_while(|&(_, c)| same(c)).last().map_or(offset, |(i, _)| i);
        let end = self.text[offset..].char_indices().find(|&(_, c)| !same(c)).map_or(self.text.len(), |(i, _)| offset + i);
        start..end
    }

    fn move_to(&mut self, offset: usize) {
        self.selection = offset..offset;
        self.reversed = false;
        self.last_edit = None;
    }

    fn select_to(&mut self, offset: usize) {
        if self.reversed {
            self.selection.start = offset;
        } else {
            self.selection.end = offset;
        }
        if self.selection.end < self.selection.start {
            self.reversed = !self.reversed;
            self.selection = self.selection.end..self.selection.start;
        }
        self.last_edit = None;
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot { text: self.text.clone(), selection: self.selection.clone() }
    }

    /// Replace `range` with `new`, recording history. Consecutive typing
    /// coalesces into one undo step; a space ends the run.
    fn replace(&mut self, range: Range<usize>, new: &str, kind: EditKind) {
        let coalesce = kind == EditKind::Typing && self.last_edit == Some(EditKind::Typing) && !new.contains(' ');
        if !coalesce {
            self.undo.push(self.snapshot());
        }
        self.redo.clear();
        self.text.replace_range(range.clone(), new);
        let at = range.start + new.len();
        self.selection = at..at;
        self.reversed = false;
        self.marked = None;
        self.last_edit = Some(kind);
    }

    fn undo(&mut self) -> bool {
        let Some(prev) = self.undo.pop() else { return false };
        self.redo.push(self.snapshot());
        self.text = prev.text;
        self.selection = prev.selection;
        self.reversed = false;
        self.marked = None;
        self.last_edit = None;
        true
    }

    fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else { return false };
        self.undo.push(self.snapshot());
        self.text = next.text;
        self.selection = next.selection;
        self.reversed = false;
        self.marked = None;
        self.last_edit = None;
        true
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        self.text[..offset.min(self.text.len())].encode_utf16().count()
    }

    fn utf16_to_offset(&self, offset: usize) -> usize {
        let mut utf16 = 0;
        for (i, c) in self.text.char_indices() {
            if utf16 >= offset {
                return i;
            }
            utf16 += c.len_utf16();
        }
        self.text.len()
    }
}

/// What a masked field draws for each character.
const MASK: char = '•';

/// Map a text offset to the masked display string's offset.
fn masked_offset(text: &str, offset: usize) -> usize {
    text[..offset].chars().count() * MASK.len_utf8()
}

/// Map a masked display offset back to a text offset.
fn unmasked_offset(text: &str, offset: usize) -> usize {
    let n = offset / MASK.len_utf8();
    text.char_indices().nth(n).map_or(text.len(), |(i, _)| i)
}

// ── The entity ────────────────────────────────────────────────────────────

pub struct TextInput {
    focus: FocusHandle,
    buf: Buffer,
    placeholder: SharedString,
    prompt: Option<SharedString>,
    masked: bool,
    disabled: bool,
    bordered: bool,
    // Layout from the last paint, for mouse hit-testing and IME bounds.
    line: Option<ShapedLine>,
    bounds: Option<Bounds<Pixels>>,
    scroll_x: Pixels,
    selecting: bool,
    // Caret blink: reset on every edit/move; ticks only while focused.
    blink_from: Instant,
    blink: Option<Task<()>>,
    _focus_subs: Vec<Subscription>,
}

impl EventEmitter<InputEvent> for TextInput {}

impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl TextInput {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        let subs = vec![
            cx.on_focus(&focus, window, |this, _, cx| this.start_blink(cx)),
            cx.on_blur(&focus, window, |this, _, cx| {
                this.blink = None;
                this.selecting = false;
                cx.notify();
            }),
        ];
        Self {
            focus,
            buf: Buffer::default(),
            placeholder: SharedString::default(),
            prompt: None,
            masked: false,
            disabled: false,
            bordered: true,
            line: None,
            bounds: None,
            scroll_x: px(0.),
            selecting: false,
            blink_from: Instant::now(),
            blink: None,
            _focus_subs: subs,
        }
    }

    pub fn placeholder(mut self, text: impl Into<SharedString>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// A display-face glyph before the text, e.g. `">"`.
    pub fn prompt(mut self, prompt: impl Into<SharedString>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Password field: draw `•`, refuse copy/cut.
    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    /// Without the frame and well — for fields inside another surface
    /// (the command palette).
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        self.disabled = disabled;
        cx.notify();
    }

    pub fn value(&self) -> SharedString {
        self.buf.text.clone().into()
    }

    /// Replace the whole value (not an undoable edit; clears history).
    pub fn set_value(&mut self, value: impl Into<SharedString>, cx: &mut Context<Self>) {
        let value: SharedString = value.into();
        self.buf = Buffer { text: value.to_string(), ..Buffer::default() };
        self.buf.move_to(self.buf.text.len());
        self.scroll_x = px(0.);
        cx.emit(InputEvent::Change);
        cx.notify();
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.focus.focus(window, cx);
    }

    fn start_blink(&mut self, cx: &mut Context<Self>) {
        self.blink_from = Instant::now();
        if motion::reduced(cx) {
            return;
        }
        let half = motion::BLINK / 2;
        self.blink = Some(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(half).await;
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    break;
                }
            }
        }));
        cx.notify();
    }

    fn caret_visible(&self, cx: &App) -> bool {
        if motion::reduced(cx) || self.blink.is_none() {
            return true;
        }
        let half = motion::BLINK.as_millis() / 2;
        (self.blink_from.elapsed().as_millis() / half).is_multiple_of(2)
    }

    /// Something moved the caret: show it solid, restart the blink phase.
    fn touched(&mut self, cx: &mut Context<Self>) {
        self.blink_from = Instant::now();
        cx.notify();
    }

    fn edit(&mut self, range: Range<usize>, new: &str, kind: EditKind, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let new = new.replace(['\n', '\r'], " ");
        self.buf.replace(range, &new, kind);
        self.touched(cx);
        cx.emit(InputEvent::Change);
    }

    fn delete_to(&mut self, offset: usize, window: &mut Window, cx: &mut Context<Self>) {
        let range = if self.buf.selection.is_empty() {
            let c = self.buf.cursor();
            c.min(offset)..c.max(offset)
        } else {
            self.buf.selection.clone()
        };
        if range.is_empty() {
            window.play_system_bell();
            return;
        }
        self.edit(range, "", EditKind::Other, cx);
    }

    fn mv(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.buf.move_to(offset);
        self.touched(cx);
    }

    fn sel(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.buf.select_to(offset);
        self.touched(cx);
    }

    // ── Actions ──────────────────────────────────────────────────────────

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        let to = self.buf.prev_boundary(self.buf.cursor());
        self.delete_to(to, window, cx);
    }
    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        let to = self.buf.next_boundary(self.buf.cursor());
        self.delete_to(to, window, cx);
    }
    fn delete_word_left(&mut self, _: &DeleteWordLeft, window: &mut Window, cx: &mut Context<Self>) {
        let to = self.buf.prev_word(self.buf.cursor());
        self.delete_to(to, window, cx);
    }
    fn delete_word_right(&mut self, _: &DeleteWordRight, window: &mut Window, cx: &mut Context<Self>) {
        let to = self.buf.next_word(self.buf.cursor());
        self.delete_to(to, window, cx);
    }
    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        let to = if self.buf.selection.is_empty() { self.buf.prev_boundary(self.buf.cursor()) } else { self.buf.selection.start };
        self.mv(to, cx);
    }
    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        let to = if self.buf.selection.is_empty() { self.buf.next_boundary(self.buf.cursor()) } else { self.buf.selection.end };
        self.mv(to, cx);
    }
    fn word_left(&mut self, _: &WordLeft, _: &mut Window, cx: &mut Context<Self>) {
        let to = self.buf.prev_word(self.buf.cursor());
        self.mv(to, cx);
    }
    fn word_right(&mut self, _: &WordRight, _: &mut Window, cx: &mut Context<Self>) {
        let to = self.buf.next_word(self.buf.cursor());
        self.mv(to, cx);
    }
    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        let to = self.buf.prev_boundary(self.buf.cursor());
        self.sel(to, cx);
    }
    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        let to = self.buf.next_boundary(self.buf.cursor());
        self.sel(to, cx);
    }
    fn select_word_left(&mut self, _: &SelectWordLeft, _: &mut Window, cx: &mut Context<Self>) {
        let to = self.buf.prev_word(self.buf.cursor());
        self.sel(to, cx);
    }
    fn select_word_right(&mut self, _: &SelectWordRight, _: &mut Window, cx: &mut Context<Self>) {
        let to = self.buf.next_word(self.buf.cursor());
        self.sel(to, cx);
    }
    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.buf.move_to(0);
        self.sel(self.buf.text.len(), cx);
    }
    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.mv(0, cx);
    }
    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.mv(self.buf.text.len(), cx);
    }
    fn select_home(&mut self, _: &SelectHome, _: &mut Window, cx: &mut Context<Self>) {
        self.sel(0, cx);
    }
    fn select_end(&mut self, _: &SelectEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.sel(self.buf.text.len(), cx);
    }
    fn paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            let range = self.buf.selection.clone();
            self.edit(range, &text, EditKind::Other, cx);
        }
    }
    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.masked && !self.buf.selection.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(self.buf.text[self.buf.selection.clone()].to_string()));
        }
    }
    fn cut(&mut self, _: &Cut, _: &mut Window, cx: &mut Context<Self>) {
        if !self.masked && !self.buf.selection.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(self.buf.text[self.buf.selection.clone()].to_string()));
            let range = self.buf.selection.clone();
            self.edit(range, "", EditKind::Other, cx);
        }
    }
    fn undo(&mut self, _: &Undo, _: &mut Window, cx: &mut Context<Self>) {
        if !self.disabled && self.buf.undo() {
            self.touched(cx);
            cx.emit(InputEvent::Change);
        }
    }
    fn redo(&mut self, _: &Redo, _: &mut Window, cx: &mut Context<Self>) {
        if !self.disabled && self.buf.redo() {
            self.touched(cx);
            cx.emit(InputEvent::Change);
        }
    }
    fn submit(&mut self, _: &Submit, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(InputEvent::Submit);
    }
    fn show_character_palette(&mut self, _: &ShowCharacterPalette, window: &mut Window, _: &mut Context<Self>) {
        window.show_character_palette();
    }

    // ── Mouse ────────────────────────────────────────────────────────────

    fn index_at(&self, position: Point<Pixels>) -> usize {
        let (Some(bounds), Some(line)) = (self.bounds, self.line.as_ref()) else { return 0 };
        if self.buf.text.is_empty() {
            return 0;
        }
        let i = line.closest_index_for_x(position.x - bounds.left() + self.scroll_x);
        if self.masked { unmasked_offset(&self.buf.text, i) } else { i }
    }

    fn on_mouse_down(&mut self, ev: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        self.focus.focus(window, cx);
        let at = self.index_at(ev.position);
        match ev.click_count {
            2 => {
                let w = if self.masked { 0..self.buf.text.len() } else { self.buf.word_at(at) };
                self.buf.move_to(w.start);
                self.sel(w.end, cx);
            }
            n if n >= 3 => {
                self.buf.move_to(0);
                self.sel(self.buf.text.len(), cx);
            }
            _ => {
                self.selecting = true;
                if ev.modifiers.shift { self.sel(at, cx) } else { self.mv(at, cx) }
            }
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.selecting = false;
    }

    fn on_mouse_move(&mut self, ev: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.selecting {
            let at = self.index_at(ev.position);
            self.sel(at, cx);
        }
    }

    /// The display string and the text→display offset map.
    fn display(&self) -> (SharedString, Box<dyn Fn(usize) -> usize + '_>) {
        if self.masked {
            let shown: String = std::iter::repeat_n(MASK, self.buf.text.chars().count()).collect();
            (shown.into(), Box::new(|o| masked_offset(&self.buf.text, o)))
        } else {
            (self.buf.text.clone().into(), Box::new(|o| o))
        }
    }
}

impl EntityInputHandler for TextInput {
    fn text_for_range(&mut self, range: Range<usize>, actual: &mut Option<Range<usize>>, _: &mut Window, _: &mut Context<Self>) -> Option<String> {
        let r = self.buf.utf16_to_offset(range.start)..self.buf.utf16_to_offset(range.end);
        actual.replace(self.buf.offset_to_utf16(r.start)..self.buf.offset_to_utf16(r.end));
        Some(self.buf.text[r].to_string())
    }

    fn selected_text_range(&mut self, _: bool, _: &mut Window, _: &mut Context<Self>) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.buf.offset_to_utf16(self.buf.selection.start)..self.buf.offset_to_utf16(self.buf.selection.end),
            reversed: self.buf.reversed,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.buf.marked.as_ref().map(|r| self.buf.offset_to_utf16(r.start)..self.buf.offset_to_utf16(r.end))
    }

    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {
        self.buf.marked = None;
    }

    fn replace_text_in_range(&mut self, range: Option<Range<usize>>, text: &str, _: &mut Window, cx: &mut Context<Self>) {
        let range = range
            .map(|r| self.buf.utf16_to_offset(r.start)..self.buf.utf16_to_offset(r.end))
            .or(self.buf.marked.clone())
            .unwrap_or(self.buf.selection.clone());
        self.edit(range, text, EditKind::Typing, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        new_selected: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        let range = range
            .map(|r| self.buf.utf16_to_offset(r.start)..self.buf.utf16_to_offset(r.end))
            .or(self.buf.marked.clone())
            .unwrap_or(self.buf.selection.clone());
        self.buf.text.replace_range(range.clone(), text);
        self.buf.marked = (!text.is_empty()).then(|| range.start..range.start + text.len());
        self.buf.selection = new_selected
            .map(|r| {
                // Selection inside the marked text is relative to its start, in UTF-16.
                let base = self.buf.offset_to_utf16(range.start);
                self.buf.utf16_to_offset(base + r.start)..self.buf.utf16_to_offset(base + r.end)
            })
            .unwrap_or(range.start + text.len()..range.start + text.len());
        self.touched(cx);
        cx.emit(InputEvent::Change);
    }

    fn bounds_for_range(&mut self, range: Range<usize>, bounds: Bounds<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<Bounds<Pixels>> {
        let line = self.line.as_ref()?;
        let (s, e) = (self.buf.utf16_to_offset(range.start), self.buf.utf16_to_offset(range.end));
        let (s, e) = if self.masked { (masked_offset(&self.buf.text, s), masked_offset(&self.buf.text, e)) } else { (s, e) };
        Some(Bounds::from_corners(
            point(bounds.left() + line.x_for_index(s) - self.scroll_x, bounds.top()),
            point(bounds.left() + line.x_for_index(e) - self.scroll_x, bounds.bottom()),
        ))
    }

    fn character_index_for_point(&mut self, point: Point<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        Some(self.buf.offset_to_utf16(self.index_at(point)))
    }
}

// ── The text element ──────────────────────────────────────────────────────

struct TextLine {
    input: Entity<TextInput>,
}

struct Prepainted {
    line: ShapedLine,
    caret: Option<gpui::PaintQuad>,
    selection: Option<gpui::PaintQuad>,
    scroll_x: Pixels,
}

impl IntoElement for TextLine {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for TextLine {
    type RequestLayoutState = ();
    type PrepaintState = Prepainted;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(&mut self, _: Option<&GlobalElementId>, _: Option<&gpui::InspectorElementId>, window: &mut Window, cx: &mut App) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = window.line_height().into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Prepainted {
        let p = palette(cx);
        let input = self.input.read(cx);
        let style = window.text_style();
        let (shown, map) = input.display();
        let empty = shown.is_empty();
        let text = if empty { input.placeholder.clone() } else { shown };
        let color = if empty { hsla(p.fg_faint) } else if input.disabled { hsla(p.fg_dim) } else { style.color };
        let run = TextRun { len: text.len(), font: style.font(), color, background_color: None, underline: None, strikethrough: None };
        let runs = match input.buf.marked.as_ref().filter(|_| !empty && !input.masked) {
            Some(m) => [
                TextRun { len: m.start, ..run.clone() },
                TextRun {
                    len: m.end - m.start,
                    underline: Some(UnderlineStyle { color: Some(hsla(p.accent)), thickness: px(1.), wavy: false }),
                    ..run.clone()
                },
                TextRun { len: text.len() - m.end, ..run },
            ]
            .into_iter()
            .filter(|r| r.len > 0)
            .collect(),
            None => vec![run],
        };
        let font_size = style.font_size.to_pixels(window.rem_size());
        let line = window.text_system().shape_line(text, font_size, &runs, None);

        let sel = input.buf.selection.clone();
        let cursor = if empty { 0 } else { map(input.buf.cursor()) };
        let caret_x = if empty { px(0.) } else { line.x_for_index(cursor) };

        // Keep the caret inside the visible width.
        let width = bounds.size.width - px(2.);
        let mut scroll_x = input.scroll_x;
        if caret_x - scroll_x > width {
            scroll_x = caret_x - width;
        } else if caret_x < scroll_x {
            scroll_x = caret_x;
        }
        scroll_x = scroll_x.min((line.width - width).max(px(0.))).max(px(0.));

        let focused = input.focus.is_focused(window);
        let selection = (!sel.is_empty() && !empty).then(|| {
            let (a, b) = (line.x_for_index(map(sel.start)), line.x_for_index(map(sel.end)));
            fill(
                Bounds::from_corners(point(bounds.left() + a - scroll_x, bounds.top()), point(bounds.left() + b - scroll_x, bounds.bottom())),
                hsla(p.accent).opacity(if focused { 0.35 } else { 0.18 }),
            )
        });
        let caret = (focused && sel.is_empty() && !input.disabled && input.caret_visible(cx)).then(|| {
            fill(Bounds::new(point(bounds.left() + caret_x - scroll_x, bounds.top()), size(px(2.), bounds.size.height)), hsla(p.accent))
        });
        Prepainted { line, caret, selection, scroll_x }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        pre: &mut Prepainted,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus = self.input.read(cx).focus.clone();
        window.handle_input(&focus, ElementInputHandler::new(bounds, self.input.clone()), cx);
        let mask = gpui::ContentMask { bounds };
        window.with_content_mask(Some(mask), |window| {
            if let Some(sel) = pre.selection.take() {
                window.paint_quad(sel);
            }
            let _ = pre.line.paint(point(bounds.left() - pre.scroll_x, bounds.top()), window.line_height(), gpui::TextAlign::Left, None, window, cx);
            if let Some(caret) = pre.caret.take() {
                window.paint_quad(caret);
            }
        });
        let line = pre.line.clone();
        let scroll_x = pre.scroll_x;
        self.input.update(cx, |input, _| {
            input.line = Some(line);
            input.bounds = Some(bounds);
            input.scroll_x = scroll_x;
        });
    }
}

impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let focused = self.focus.is_focused(window);
        let frame = if self.disabled { p.line } else if focused { p.accent } else { p.line_strong };
        div()
            .id("ferrite-input")
            .role(Role::TextInput)
            .aria_label(self.placeholder.clone())
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            .when(!self.disabled, |el| el.tab_stop(true))
            .cursor(if self.disabled { CursorStyle::Arrow } else { CursorStyle::IBeam })
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::delete_word_left))
            .on_action(cx.listener(Self::delete_word_right))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::word_left))
            .on_action(cx.listener(Self::word_right))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_word_left))
            .on_action(cx.listener(Self::select_word_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::select_home))
            .on_action(cx.listener(Self::select_end))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::undo))
            .on_action(cx.listener(Self::redo))
            .on_action(cx.listener(Self::submit))
            .on_action(cx.listener(Self::show_character_palette))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .w_full()
            .body(text::BASE)
            .line_height(px(20.))
            .text_color(hsla(p.fg))
            .when(self.bordered, |el| el.h(px(32.)).px_2().bg(hsla(p.sunken)).border_1().border_color(hsla(frame)))
            .when_some(self.prompt.clone(), |el, prompt| {
                el.child(div().flex_none().display(Scale::X1, window).text_color(hsla(if self.disabled { p.fg_faint } else { p.accent })).child(prompt))
            })
            .child(div().flex_1().min_w_0().overflow_hidden().child(TextLine { input: cx.entity() }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(text: &str, at: usize) -> Buffer {
        let mut b = Buffer { text: text.into(), ..Buffer::default() };
        b.move_to(at);
        b
    }

    #[test]
    fn graphemes_move_as_one() {
        let b = buf("ae\u{301}x", 1); // "aéx" with a combining accent
        assert_eq!(b.next_boundary(1), 4);
        assert_eq!(b.prev_boundary(4), 1);
    }

    #[test]
    fn word_motion_skips_separators_then_a_word() {
        let b = buf("git push --force", 0);
        assert_eq!(b.next_word(0), 3);
        assert_eq!(b.next_word(3), 8);
        assert_eq!(b.next_word(8), 16);
        assert_eq!(b.prev_word(16), 11);
        assert_eq!(b.prev_word(11), 4);
        assert_eq!(b.prev_word(4), 0);
    }

    #[test]
    fn double_click_word() {
        let b = buf("open ferrite_design now", 0);
        assert_eq!(b.word_at(7), 5..19);
        assert_eq!(b.word_at(0), 0..4);
    }

    #[test]
    fn selection_flips_when_dragged_backwards() {
        let mut b = buf("abcdef", 3);
        b.select_to(5);
        assert_eq!(b.selection, 3..5);
        b.select_to(1);
        assert_eq!(b.selection, 1..3);
        assert!(b.reversed);
        assert_eq!(b.cursor(), 1);
    }

    #[test]
    fn typing_coalesces_and_undo_redo_round_trip() {
        let mut b = buf("", 0);
        for (i, c) in "dep".chars().enumerate() {
            b.replace(i..i, &c.to_string(), EditKind::Typing);
        }
        b.replace(3..3, " ", EditKind::Typing); // a space starts a new step
        b.replace(4..4, "x", EditKind::Typing);
        assert_eq!(b.text, "dep x");
        assert!(b.undo());
        assert_eq!(b.text, "dep");
        assert!(b.undo());
        assert_eq!(b.text, "");
        assert!(!b.undo());
        assert!(b.redo());
        assert_eq!(b.text, "dep");
    }

    #[test]
    fn utf16_offsets_round_trip() {
        let b = buf("a😀b", 0);
        assert_eq!(b.offset_to_utf16(5), 3); // after the emoji: 1 + 2 UTF-16 units
        assert_eq!(b.utf16_to_offset(3), 5);
        assert_eq!(b.utf16_to_offset(99), b.text.len());
    }

    #[test]
    fn mask_offsets_map_per_character() {
        let t = "pa😀s";
        assert_eq!(masked_offset(t, 2), 2 * MASK.len_utf8());
        assert_eq!(unmasked_offset(t, 3 * MASK.len_utf8()), 6);
        assert_eq!(unmasked_offset(t, 99), t.len());
    }
}
