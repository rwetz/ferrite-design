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

| Component | Module | Notes |
|---|---|---|
| `TitleBar` + window controls | `chrome` | replaced gpui-component's `TitleBar` from the start |
| `Panel` | `components` | `[ TITLE ]` header + dither fill |
| `StatusBar` | `components` | `│`-separated display-face segments |
| `rule` | `components` | labelled 1px divider |
| `cursor` | `components` | square-wave blinking block |
| `progress_bar` | `components` | dithered leading edge (replaces `Progress`) |
| `empty_state` | `components` | radial dither + message |
| `Dither` | `dither` | the texture primitive everything else uses |

## Replacement order

Ranked by how much the library default fights Ferrite, times how often apps
use it.

| # | gpui-component piece | Why first | Ferrite version |
|---|---|---|---|
| 1 | `window_border` (Linux CSD) | rounds corners; we need square resize edges | `chrome::WindowFrame` |
| 2 | `Button` | most-used; pill/ring geometry, icon set | `[ LABEL ]` text-mode buttons, accent fill for primary, inverted on press |
| 3 | `Switch` / `Checkbox` / `Radio` | rounded thumbs are off-language | `[x]` / `[ ]` / `(•)` glyph toggles |
| 4 | `Tag` / `Badge` | rounded pills | bracketed inverse-video labels |
| 5 | `Tooltip` / `Popover` / menus | shadows, radius | square, 1px `line_strong`, `raised` fill |
| 6 | `Tab` / `TabBar` | | `│ TAB │` with accent underline |
| 7 | `Input` | hardest (IME, selection, undo) — **last**, if ever | block caret, `sunken` well |
| — | `List`, `Table`, `VirtualList`, `Dock`, `Resizable` | behaviour-heavy, theme well already | keep wrapping; revisit only with a concrete reason |

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
