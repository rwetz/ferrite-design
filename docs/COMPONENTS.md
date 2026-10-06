# Components — today and the plan

## The decision

**Short term:** Ferrite apps use [gpui-component](https://github.com/longbridge/gpui-kit)
(Longbridge, Apache-2.0) for widgets, re-skinned by Ferrite's theme. It gives
60+ working desktop components on day one: inputs with IME, lists, virtual
tables, menus, dialogs, docking.

**Long term:** Ferrite owns its components. gpui-component is a
shadcn-shaped library; its geometry (pills, rings, padding, icon set) leaks
through any theme, and its release cadence pins our gpui version (PITFALLS §1).
Native components remove both constraints.

This is a migration, not a rewrite: replace one component at a time, starting
where the library fights the language hardest, and keep apps working at every
step.

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
| **`dropdown_menu` / `context_menu`** | `components::menu` | icons, shortcuts, check toggles, section labels, separators, danger + disabled rows; full keyboard nav |
| **`Icon`** | `icon` | 21 pixel icons on the type grid; `fit()` for fixed-height controls |
| **`spinner` / `cursor` / `ticker`** | `components::ticker` | timer-driven periodic state (no per-frame redraws) |
| `Panel`, `StatusBar`, `rule`, `progress_bar`, `empty_state` | `components` | framing |
| `Dither` | `dither` | the texture primitive |

Every interactive component carries the behaviour ported from
gpui-component: its own focus handle, Tab-order participation, Enter/Space
activation (gpui's keyboard click), no focus-steal on mouse down, a
keyboard-only focus frame (`focus_visible`), and an accessibility
role/label/toggled state.

**Known gap:** gpui exposes no accessibility "disabled" state. Disabled
controls drop out of the tab order and ignore input, but screen readers
aren't told they're disabled.

## Replacement order

| # | gpui-component piece | Status |
|---|---|---|
| 1 | `window_border` (Linux CSD) | **open** — needs verifying on Linux; rounds corners today |
| 2 | `Button` | ✅ native |
| 3 | `Switch` / `Checkbox` / `Radio` | ✅ native |
| 4 | `Tag` / `Badge` | ✅ native (+ `meter`) |
| 5 | `Tooltip` / `Popover` / menus | ✅ native — tooltip, popover, dropdown + context menu (no submenus yet) |
| 6 | `Tab` / `TabBar` | ✅ native (+ `list_item`) |
| 7 | `Input` | **last, if ever** — IME, selection, undo |
| — | `List`, `Table`, `VirtualList`, `Dock`, `Resizable` | keep wrapping; put `list_item` rows inside the library's virtual list |

Next up: **`window_border`** once there's a Linux machine to verify it on,
then **submenus** and a **command palette** (a menu + filter input — the
first piece that needs the library's `Input`).

Inputs, virtualised lists and docking are where a component library earns its
keep. Replacing them is not a goal in itself.

## Rules for writing a native component

1. **Colors from `palette(cx)` only.** No hex in components.
2. **Display text through `.display(Scale, window)`**, body through
   `.body(size)`. Never raw font sizes on the display face.
3. **0px radius, no shadow.** Depth via surface steps and 1px lines.
4. **Builder API** matching gpui-component's shape where one exists
   (`Button::new(id).label(..).primary()`), so swapping an import is the
   whole migration for app code.
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
