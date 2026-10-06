# Components

**Everything is Ferrite's own.** Ferrite depends on gpui and nothing built
on top of it — no component library, no second theme system. See every
component live with `cargo run --example components` (one page per family,
with a scheme picker), and whole apps built from them in `examples/app_*.rs`.

`use ferrite_design::prelude::*;` imports all of them.

## Catalogue

### Controls

| Component | Module | Notes |
|---|---|---|
| **`Button`** | `button` | primary / secondary / ghost `[ LABEL ]` / danger; small; icon, glyph, shortcut, tooltip, loading, toggle (`selected`); disabled = dashed frame; pressed = inverse video; click flash |
| **`checkbox` / `radio`** | `toggle` | `[x]` `[ ]` `[-]` / `(•)` `( )` display-face marks, stamped in |
| **`switch`** | `toggle` | square thumb that travels, dithered off-track, `ON`/`OFF` readout |
| **`segmented`** | `segmented` | one-of-few strip, selected segment inverse video; ←/→ |
| **`slider`** | `slider` | LED-cell track snapped to `step`; drag (pointer captured), arrows/PageUp/Home/End |
| **`tag`** | `tag` | solid (inverse video) or outline, five tones |
| **`meter`** | `tag` | segmented LED bar, warning/danger zones, `.roll(true)` for events, `.id()` |
| **`kbd` / `tooltip`** | `tooltip` | keycaps; square tooltip that unrolls |
| **`spinner` / `cursor` / `ticker`** | `ticker` | timer-driven periodic state (no per-frame redraws) |

### Forms

| Component | Module | Notes |
|---|---|---|
| **`TextInput`** | `input` | single-line field on gpui's `EntityInputHandler` (IME, dead keys), grapheme/word motion, undo/redo, mouse select, `.masked()`, `.prompt(">")`; `Change` / `Submit` events |
| **`field`** | `form` | label + control + hint or error; required mark; shakes once per *new* error; side-by-side or `.stacked()` |
| **`select`** | `form` | value picker: sunken well + chevron, opens a menu with a check on the current option |
| **`number_input`** | `form` | `[-] 008 [+]` stepper; range, step, digits, decimals, suffix; arrows/PageUp/Home/End |
| **`calendar`** | `calendar` | month grid, today framed, selection inverse, min/max range, keyboard; `Date` type with civil arithmetic (no date-crate dependency) |
| **`date_picker`** | `calendar` | field that opens a calendar; choosing closes it |

### Overlays

| Component | Module | Notes |
|---|---|---|
| **`popover`** | `overlay` | non-modal panel under a trigger, `[ TITLE ]` header, hard dithered shadow |
| **`dropdown_menu` / `context_menu`** | `menu` | icons, shortcuts, checks, labels, separators, danger + disabled rows, nested `submenu`s, full keyboard nav |
| **`dialog`** | `dialog` | controlled modal over a screen-doored app; Esc/Enter; `.danger()` |
| **`drawer`** | `drawer` | controlled modal side sheet; wipes in from its edge; header, scrolling body, footer |
| **`CommandPalette`** | `palette` | modal, fuzzy-ranked (`fuzzy`), grouped when empty, match highlights; built on `TextInput` |
| **`Toaster` / `toast`** | `toast` | bottom-right stack; four kinds; stepped countdown paused on hover; actions |

### Navigation

| Component | Module | Notes |
|---|---|---|
| **`sidebar`** | `nav` | brand, sections, items with icons and counts, footer; `.collapsed(true)` = icon rail with tooltips |
| **`toolbar`** | `nav` | control row on `surface`; `.separator()`, `.spacer()` |
| **`tabs`** | `tabs` | `│ TAB │` strip; the amber edge *seeks* between tabs |
| **`breadcrumb`** | `nav` | `src / components / menu.rs`; every crumb but the last is a link |
| **`pagination`** | `nav` | `[<] 1 … 4 [5] 6 … 20 [>]`, constant width; ←/→ |
| **`steps`** | `nav` | wizard progress; done steps checked and clickable, connector draws on |
| **`accordion`** | `disclosure` | stacked sections, own open state, `.single()`; content unrolls in |

### Data display

| Component | Module | Notes |
|---|---|---|
| **`list_item`** | `list` | selectable row: amber bar + swept-in wash, icon, meta |
| **`tree`** | `tree` | 1px connector guides, owns expansion, cascade on open; full keyboard |
| **`table`** | `table` | sortable display-face header, right-aligned numbers, row selection |
| **`virtual_list`** | `scroll` | renders only visible rows (gpui `uniform_list`); rows are full-width click targets |
| **`property_list`** | `display` | key/value rows for inspectors; values can be any element |
| **`stat`** | `display` | KPI tile: label, value that decrypts in, ▲/▼ delta (`.lower_is_better()`), sparkline |
| **`avatar`** | `display` | 5×5 mirror identicon from the name, or initials; presence square |
| **`timeline`** | `display` | activity feed on a 1px rail; cascades in, pings the newest marker |
| **`alert`** | `feedback` | inline callout: tone bar, icon, decrypting title, message, actions, dismiss |
| **`skeleton` / `skeleton_text`** | `feedback` | loading placeholder: light dither + a stepped scan band (timer, not per-frame) |

### Charts

| Component | Module | Notes |
|---|---|---|
| **`line_chart`** | `chart` | stepped line on the pixel grid, dithered area, dim comparison series, axis labels, hover crosshair + readout; draws on once |
| **`bar_chart`** | `chart` | labelled bars that grow in; hover inverts and shows the value; `.highlight(i)` |
| **`sparkline`** | `chart` | inline bars, the latest in amber |
| **`heatmap`** | `chart` | grid whose dither *density* is the value; row labels; `LESS ░▒▓█ MORE` legend |

### Text mode (ASCII)

| Component | Module | Notes |
|---|---|---|
| **`ascii_box`** | `textmode` | real box-glyph frame, single or `.double()`, title in the edge, any size (edges clipped to the box) |
| **`ascii_rule`** | `textmode` | `── LABEL ───` across the width |
| **`banner`** | `textmode` | 5×5 block letters as quads, wiped on |
| **`ascii_art`** | `textmode` | a `Picture` as characters, any ramp (`ascii::CLASSIC`, `BUBBLES`) |
| **`ascii_gauge`** | `textmode` | `[████▒·····] 42%`; live-data safe |
| **`mark`** | `textmode` | the `▓▒░` mark as real dither |
| `spinner(..).frames(..)` | `ticker` | `ascii::spinners::{LINE, SHADE, DOTS, PULSE, BOUNCE}` |

### Layout and framing

| Component | Module | Notes |
|---|---|---|
| **`scroll_area` / `scrollbar`** | `scroll` | square thumb on a hairline track; drag or click-to-jump |
| **`split`** | `split` | draggable 1px divider, fraction state, min sizes, double-click resets |
| **`TitleBar` / `window_frame`** | `chrome` | self-drawn square chrome on Windows/Linux, native lights on macOS; Linux resize edges |
| `Panel`, `StatusBar`, `rule`, `progress_bar`, `empty_state` | `components` | framing; status-bar segments decrypt on change, `*_live` segments don't |
| `Icon` | `icon` | 32 pixel icons on the type grid; `fit()` for fixed-height controls |
| `Dither` | `dither` | the texture primitive |

### Motion (drop-in effects, `fx`)

`decrypt`, `typewriter`, `shake`, `dissolve`, `count_up`, `unroll_in`,
`develop`, `afterglow`, `interlace_in`, `tear`, `ping`, `power_on_in`,
`wipe_in`, `scan`, `flash`, `cascade_in`, and `boot_screen` (a
text-mode POST at launch, opt-in). The theme glitch needs no call:
`window_frame` plays it on every scheme or tone switch. What each is for:
DESIGN_LANGUAGE §6.1. The engine underneath is `animate`.

## Behaviour every interactive component carries

Its own focus handle in keyed state, Tab-order participation, Enter/Space
activation (gpui's keyboard click), no focus-steal on mouse down (composite
controls take focus on click instead, PITFALLS §32), a keyboard-only focus
frame (`focus_visible`), and an accessibility role, label and state.
All are **controlled** where they hold a value (the app owns it, the
handler receives the new one); components own only incidental state
(which tree branches or accordion sections are open, the shown month).

## Known gaps

- gpui exposes no accessibility "disabled" state. Disabled controls leave
  the tab order and ignore input, but screen readers aren't told.
- gpui has no focus scope, so `dialog` and `drawer` can't trap Tab.
- No multi-line text editor yet; see [ROADMAP.md](ROADMAP.md).

## Rules for writing a native component

1. **Colors from `palette(cx)` only.** No hex in components; they must look
   right in every scheme (`schemes::SCHEMES`), so check a new one in at
   least Ferrite, Mono and one wild scheme, dark and light.
2. **Display text through `.display(Scale, window)`**, body through
   `.body(size)`. Any new display-face glyph goes in the
   `fonts::tests` tripwire.
3. **0px radius, no shadow.** Depth via surface steps and 1px lines.
4. **Builder API** in the established shape (`Button::new(id).label(..)
   .primary()`); if it takes `Styled`, apply the caller's style first
   (PITFALLS §35).
5. **Motion from `animate`/`motion` only**, entrances not exits, events not
   live data, keyed clips namespaced by the component's id.
6. **Interactive ⇒ stable `ElementId` + visible focus** (1px accent).
7. **Pure logic in pure functions with tests** (ranges, paging, dates,
   snapping) — the GUI can't be unit-tested, the arithmetic can.
8. **Ship it in the components gallery** in the same commit.

## History: leaving gpui-component

Ferrite started on [gpui-component](https://github.com/longbridge/gpui-component)
and replaced it piece by piece (window border, button, toggles, tags,
tooltip/popover/menus, tabs, input, dialog/slider/tree/table, scrolling,
virtual list, resizable, notifications, `Root` + theme). Behaviour that was
hard-won there (focus handling, keyboard activation, overlay dismissal, the
title bar's platform quirks) was ported, not reinvented; the module docs say
what came from where. The dependency is gone.
