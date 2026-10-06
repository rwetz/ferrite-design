# Components — today and the plan

## The decision

**Everything is Ferrite's own.** Ferrite depends on gpui and nothing built on
top of it — no component library, no second theme system. It started on
[gpui-component](https://github.com/longbridge/gpui-kit) (Longbridge) as a
stop-gap and replaced it piece by piece; the last pieces (text input,
scrollbars, virtual list, split pane, window frame, the theme bridge and
`Root`) went in one pass.

Why it was worth it: a shadcn-shaped library's geometry (pills, rings,
padding, icon set) leaks through any theme, its theme schema can drift under
you (PITFALLS §4), and its release cadence pins your gpui (§1).

Behaviour that was hard-won in gpui-component (focus handling, keyboard
activation, overlay dismissal, the title bar's platform quirks) was ported,
not reinvented — see the module docs for what came from where.

## What's already native

See every one of them live: `cargo run --example components`.

| Component | Module | Notes |
|---|---|---|
| `TitleBar` + window controls | `chrome` | replaced gpui-component's `TitleBar` from the start |
| **`Button`** | `components::button` | primary / secondary / ghost `[ LABEL ]` / danger; small; glyph, shortcut, tooltip, loading, toggle (`selected`); disabled = dashed frame; pressed = inverse video |
| **`checkbox` / `radio`** | `components::toggle` | `[x]` `[ ]` `[-]` / `(•)` `( )` display-face marks, body labels |
| **`switch`** | `components::toggle` | square thumb, dithered off-track, `ON`/`OFF` readout |
| **`tag`** | `components::tag` | solid (inverse video) or outline, five tones |
| **`meter`** | `components::tag` | segmented LED bar, warning/danger zones |
| **`tabs`** | `components::tabs` | `│ TAB │` strip, amber top edge, optional count |
| **`list_item`** | `components::list` | selectable row: amber bar + `accent_dim` wash, glyph, meta |
| **`tooltip` / `kbd`** | `components::tooltip` | square tooltip with optional keycaps |
| **`popover`** | `components::overlay` | non-modal panel under a trigger, `[ TITLE ]` header, hard dithered drop shadow |
| **`dropdown_menu` / `context_menu`** | `components::menu` | icons, shortcuts, check toggles, section labels, separators, danger + disabled rows, nested `submenu`s (open on hover, → / ← / Esc per level, cascade snaps on screen as one); full keyboard nav |
| **`CommandPalette`** | `components::palette` | modal, fuzzy-ranked (`fuzzy`), grouped when empty, amber match highlights, screen-door scrim; wraps gpui-component's `Input` for the text field |
| **`Toaster` / `toast`** | `components::toast` | bottom-right stack of up to 4; info/success/warning/danger with pixel icon + `INFO OK WARN ERR` code; stepped 16-segment countdown, paused on hover; dither materialise on entry; optional action; sticky |
| **`dialog`** | `components::dialog` | controlled modal: `[ TITLE ]` header, description + any content, cancel (Esc) / confirm (Enter) footer, screen-door backdrop; `.danger()` for destructive confirms |
| **`segmented`** | `components::segmented` | one-of-few strip, selected segment in inverse video; ←/→ move |
| **`slider`** | `components::slider` | discrete LED-cell track snapped to `step`, tall thumb cell, fixed-width readout; drag with pointer capture, arrows/PageUp/Home/End |
| **`tree`** | `components::tree` | 1px connector guides (├ └ │), chevrons, owns its expansion; →/← expand, collapse, step in/out |
| **`table`** | `components::table` | display-face header with ▲/▼ sort marks, right-aligned numeric columns, row selection; for up to a few hundred rows |
| **`TextInput`** | `components::input` | single-line field on gpui's `EntityInputHandler` (IME, dead keys), grapheme/word motion, undo/redo, mouse select (double/triple click), horizontal scroll, `.masked()`, `.prompt(">")`; `Change` / `Submit` events |
| **`scrollbar` / `scroll_area`** | `components::scroll` | square thumb on a hairline track, drag (pointer captured) or click-to-jump; reads the offset at paint time |
| **`virtual_list`** | `components::scroll` | renders only visible rows (gpui `uniform_list`), with the Ferrite scrollbar |
| **`split`** | `components::split` | draggable 1px divider, fraction state, min sizes, double-click resets |
| **`window_frame`** | `chrome` | Linux client-decoration frame + resize edges; passthrough elsewhere |
| **`Icon`** | `icon` | 21 pixel icons on the type grid; `fit()` for fixed-height controls |
| **`spinner` / `cursor` / `ticker`** | `components::ticker` | timer-driven periodic state (no per-frame redraws) |
| `Panel`, `StatusBar`, `rule`, `progress_bar`, `empty_state` | `components` | framing |
| `Dither` | `dither` | the texture primitive |

Every interactive component carries the behaviour ported from
gpui-component: its own focus handle, Tab-order participation, Enter/Space
activation (gpui's keyboard click), no focus-steal on mouse down, a
keyboard-only focus frame (`focus_visible`), and an accessibility
role/label/toggled state.

**Known gaps:**
- gpui exposes no accessibility "disabled" state. Disabled controls drop out
  of the tab order and ignore input, but screen readers aren't told they're
  disabled.
- gpui has no focus scope, so a `dialog` can't trap Tab: it can walk out to
  elements behind the backdrop (which still can't be clicked).

## Replacement log

| # | gpui-component piece | Ferrite replacement |
|---|---|---|
| 1 | `window_border` (Linux CSD) | `chrome::window_frame` — square frame + resize edges (unverified on Linux) |
| 2 | `Button` | `Button` |
| 3 | `Switch` / `Checkbox` / `Radio` | `switch` / `checkbox` / `radio` |
| 4 | `Tag` / `Badge` | `tag` (+ `meter`) |
| 5 | `Tooltip` / `Popover` / menus | `tooltip`, `popover`, `dropdown_menu` / `context_menu` + `submenu` |
| 6 | `Tab` / `TabBar` | `tabs` (+ `list_item`) |
| 7 | `Input` | `TextInput` — gpui's own IME plumbing, undo, word motion, mask |
| 8 | `Dialog` / `Slider` / `Tree` / `Table` | `dialog`, `slider`, `tree`, `table` (+ `segmented`) |
| 9 | `Scrollbar` / `Scrollable` | `scrollbar`, `scroll_area` |
| 10 | `VirtualList` / virtual table | `virtual_list` (gpui's `uniform_list` + Ferrite scrollbar) |
| 11 | `Resizable` | `split` |
| 12 | `Notification` | `Toaster` / `toast` |
| 13 | `Root` + `ThemeConfig` | nothing — your view is the window root; `theme.rs` owns appearance |

Not replaced because Ferrite never used them: docking, charts, the code
editor, date pickers. Build them natively when an app needs one.

## Rules for writing a native component

1. **Colors from `palette(cx)` only.** No hex in components.
2. **Display text through `.display(Scale, window)`**, body through
   `.body(size)`. Never raw font sizes on the display face.
3. **0px radius, no shadow.** Depth via surface steps and 1px lines.
4. **Builder API** in the established shape (`Button::new(id).label(..)
   .primary()`), and if it takes `Styled`, apply the caller's style first
   (PITFALLS §35).
5. **Stepped or instant motion** from `motion::*`; honour `motion::reduced`.
6. **Interactive ⇒ stable `ElementId` + visible focus** (1px accent).
7. **Ship it in the showcase** in the same commit, next to the
   gpui-component version it replaces, so the difference is visible.
8. **Platform behaviour is copied, not re-invented.** When replacing a
   library component, port its platform handling (and the reasons, into
   PITFALLS.md) before restyling it — `chrome::TitleBar` is the model.

## Exit criteria for dropping gpui-component

- Items 1–6 above are native and used by every Ferrite app.
- Remaining library use is confined to `Input`, `List`/`Table`, `Dock`, and
  is behind thin Ferrite wrappers, so a future swap touches one crate.
- At that point gpui-component becomes an optional feature of this crate
  rather than a hard dependency.
