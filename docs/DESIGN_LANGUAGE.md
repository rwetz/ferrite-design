# Ferrite — Design Language

> The canonical description of how a Ferrite app looks, moves and behaves.
> Every value here is defined in `src/` and enforced by tests where it can be;
> this document explains the *why*. When the two disagree, the code is right
> and this document has a bug.
>
> **Scope:** native desktop apps built on **GPUI** (Rust, GPU-rendered, no
> webview). Tauri/React webview apps use the separate
> [Nexis design system](https://github.com/rwetz/nexis-design).

---

## 0. One line

**Phosphor amber on iron grey: hard corners, pixel display type, ordered
dither instead of gradients, stepped motion, and a machine's honesty about
what it is doing.**

Ferrite should feel like a well-made instrument: a logic analyser, a BIOS
setup screen, a terminal on a good day. Gritty in *texture*, never in
*legibility*. The grit lives in the frame; the content stays clean.

---

## 1. Inherited from Nexis vs. new to Ferrite

Ferrite deliberately keeps the Nexis *rules* and replaces the Nexis *look*.

| Rule (kept) | Nexis expression | Ferrite expression |
|---|---|---|
| One accent carries identity | coral `oklch(0.72 0.15 35)` | phosphor amber `#F2A93B` |
| Neutral base palette | cool OKLCH greys, glass | near-pure iron greys, opaque |
| Tokens have one source of truth | `globals.css` + theme engine | `src/tokens.rs` → projected into gpui-component |
| Theme applied before first paint | `index.html` anti-flash script | `theme::install` before `open_window` |
| Shared motion vocabulary | springs (`snappy/smooth/gentle`) | steps (`FAST/BASE/SLOW`, `steps(n)`, `BLINK`) |
| Respect reduced motion | `MotionConfig reducedMotion="user"` | `motion::reduced(cx)` → render final state |
| Self-drawn chrome on Win/Linux, native on macOS | 12px rounded glass frame | square frame, `_ □ x` text-mode controls |
| Self-hosted, subsetted fonts | Inter + JetBrains Mono | PxPlus IBM VGA 8×16 + JetBrains Mono, embedded |
| Tripwire tests on invariants | Nexis's test suite | contrast, schema round-trip, dither, ASCII tests |
| Corners | radius scale, pill buttons | **0px everywhere**, no scale |
| Depth | shadows, `backdrop-blur` | none — depth is 1px lines and surface steps |
| Gradients / glow | `.brand-glow`, `.aurora-border` | **none** — ordered dither only |
| Bespoke cursor set | 29 PNGs via CSS | *not carried*: GPUI only exposes system cursors |

---

## 2. Color

All colors live in `src/tokens.rs` as two `Palette`s. Components ask for a
**role**, never a hex value.

### 2.1 The roles

| Role | Iron (dark) | Paper (light) | Use |
|---|---|---|---|
| `sunken` | `#070708` | `#E2DFD8` | wells: inputs, logs, progress tracks |
| `bg` | `#0B0B0C` | `#EDEBE6` | the page |
| `surface` | `#111113` | `#E6E3DC` | chrome: title bar, status bar, panel headers |
| `raised` | `#18181B` | `#F6F4F0` | popovers, hovered rows, default buttons |
| `line` | `#2A2A2E` | `#C7C3BA` | hairlines |
| `line_strong` | `#3D3D42` | `#A9A49A` | input borders, window frame, pressed |
| `fg` | `#E8E8E6` | `#151515` | text |
| `fg_dim` | `#8E8E92` | `#55534E` | secondary text (AA on every surface) |
| `fg_faint` | `#55555A` | `#9A968E` | disabled, glyph separators — never body text |
| `accent` | `#F2A93B` | `#E39A22` | **the** accent fill |
| `accent_text` | `#F2A93B` | `#8A5000` | accent used as text |
| `accent_fg` | `#0B0B0C` | `#151515` | text on an accent fill |
| `accent_dim` | `#3A2A12` | `#F1DDB6` | selected-row background |
| `danger` / `success` / `warning` | `#F0503C` / `#9BD37E` / `#F2D23B` | `#C0301D` / `#3B7526` / `#735A00` | semantic status only |

### 2.2 The rules

- **One accent.** Amber marks: the primary action, the active/selected item,
  focus rings, the caret, live progress, and the `▓▒░` title-bar mark. It is
  never decoration and never sits beside a second "brand" hue.
- **No blue.** There is no info-blue in Ferrite. "Info" is neutral emphasis
  (`raised` + `fg`). Hues outside amber appear only as semantic status or as
  desaturated syntax/ANSI colors.
- **Usable greys.** Iron greys have no hue; the whites carry a hair of warmth
  so amber reads hot against cold metal instead of muddy.
- **Paper is a printout, not a web page.** Warm off-white stock, ink type,
  the same dither. The accent darkens for text (`accent_text`) because pale
  amber on paper is illegible.
- **Contrast is enforced.** `tokens::tests` fails the build if `fg` drops
  below 7:1, `fg_dim` / `accent_text` / status colors below 4.5:1, or a line
  becomes invisible (or loud). Change a value, run `cargo test`.

### 2.3 Painting colors yourself

```rust
let p = ferrite_design::palette(cx);     // the palette on screen right now
div().bg(hsla(p.surface)).border_color(hsla(p.line))
```

Never reference `IRON`/`PAPER` directly in view code: `palette(cx)` follows
the current mode.

---

## 3. Typography

Two faces, strictly split. Both are embedded in the crate (`assets/fonts/`)
and registered by `ferrite_design::init`.

| Face | Use | Sizes |
|---|---|---|
| **PxPlus IBM VGA 8×16** (`fonts::DISPLAY`) | titles, panel headers, labels, status bar, numbers, ASCII art — *short strings* | `Scale::X1/X2/X3` only (16/32/48 cells) |
| **JetBrains Mono** (`fonts::BODY`) | body copy, inputs, lists, tables, code, logs — anything read at length or typed | `tokens::text::{XS 11, SM 12, BASE 13, LG 15}` |

### 3.1 Pixel snapping (the important part)

The display face is a pixel font drawn as outlines. It is only sharp when
each font pixel lands on a whole number of device pixels. `fonts::display_size`
picks the logical size whose **physical** height is an exact multiple of 16
for the current window's scale factor:

| OS scale | `X1` logical | `X1` physical |
|---|---|---|
| 100% | 16px | 16 |
| 125% | 12.8px | 16 (smaller than asked, but crisp) |
| 150% | 21.33px | 32 (larger than asked, but crisp) |
| 200% | 16px | 32 |

Crisp wins over exact. Always set display type with `.display(Scale, window)`,
never with a raw `.text_size()`. Line height equals the size: the 8×16 cell
already includes its leading.

### 3.2 Voice

- Display strings are **UPPERCASE**. Labels are bracketed: `[ TITLE ]`.
- Numbers are right-aligned and fixed-width (`{:>3}%`).
- Glyph coverage of the display face is CP437 + WGL4: box drawing, shades
  `░▒▓█▀▄▌▐`, arrows. It has **no** eighth blocks `▁▂▃` or quadrants `▖▘`;
  set those (e.g. `ascii::sparkline`) in BODY. Check any symbol with
  `fonts::display_has`.

### 3.3 Icons

Symbols are **pixel icons** (`src/icon.rs`), not font glyphs: 16×16 one-bit
bitmaps drawn with the display face's own stroke (2px stems, 1px margin),
rasterised at the same whole-pixel multiple as display type so they share
its grid, and cached like dither. 21 ship: `plus close check play stop
pause refresh search up down chevron_down chevron_right menu more copy
trash file folder warning dot sliders`. Inside fixed-height controls use
`icon(..).fit(max)`.

---

## 4. Space and geometry

- **Grid:** the display cell, 8×16. Gaps and padding are multiples of 4;
  anything carrying display type is a multiple of 16 tall
  (`tokens::space::{HALF 4, CELL 8, ROW 16, X2 24, X3 32}`).
- **Corners:** 0px. Everywhere. Components, popovers, the window itself
  (`chrome::square_corners` opts out of Windows 11's DWM rounding). There is
  no "slightly rounded" variant.
- **Depth:** no shadows (`shadow: false` in the theme). Depth is expressed by
  stepping `sunken → bg → surface → raised` and by 1px lines.
- **Borders:** 1px `line`; 1px `line_strong` for inputs and emphasis; 1px
  `accent` for focus and the active row.

---

## 5. Texture: dither and ASCII

Ferrite has no gradients, glows, blurs or translucency. Where another system
would reach for those, Ferrite uses **4×4 ordered (Bayer) dither** or a
**text-mode glyph**.

### 5.1 Dither (`src/dither.rs`)

- Fields: `flat`, `horizontal`, `vertical`, `radial` (plain data, so each
  dither is rasterised once and cached; repaints are one textured quad).
- Levels: use `dither::level::{LIGHT 0.25, MEDIUM 0.5, DARK 0.75}` (they map
  to `░ ▒ ▓`) so textures match across apps.
- Cells are sized in **device pixels** and snapped to the device grid, so a
  1-px dither stays 1-px at any scale.
- Default ink is `line_strong`; amber dither is reserved for accent uses
  (progress edges, the title-bar mark).

### 5.2 Where texture goes — structural accents, not noise

| ✅ Use | ❌ Never |
|---|---|
| panel-header fill after the title | behind body text |
| title-bar drag strip | inside inputs or list rows |
| progress leading edge (`▓▒░`) | animated continuously |
| empty states, loading placeholders | as a full-window background behind content |
| large content-free regions | more than one dither density in one small area |

### 5.3 ASCII (`src/ascii.rs`)

`bar`, `spinner`, `sparkline`, `shade`, `bracket`, `rule` and the box-drawing
set. They return strings, so they compose with any element; mind which face
renders them (§3.2).

---

## 6. Motion

Nexis glides; Ferrite **steps**. Machines don't ease.

| Token | Value | Use |
|---|---|---|
| — | instant | default for hover, select, toggle |
| `motion::FRAME` | 40ms | one stepped frame (~25fps, deliberately chunky) |
| `motion::FAST` | 120ms | 3-frame reveals |
| `motion::BASE` | 200ms | default stepped transition |
| `motion::SLOW` | 320ms | large reveals |
| `motion::BLINK` | 1060ms | caret / live-marker cycle, square wave |

**Periodic motion runs on timers, not animations.** Blinks, spinners and
tickers flip state with `cx.notify()` at the rate their content changes; a
repeating `with_animation` redraws the entire window every display frame
(PITFALLS §17).

```rust
el.with_animation("reveal", Animation::new(motion::BASE).with_easing(motion::steps(5)), |el, t| el.opacity(t))
```

No springs, no long ease-out curves, no bounces. **Reduced motion:** check
`motion::reduced(cx)` and render the end state — blinks go solid, stepped
reveals go instant, tickers stop.

---

## 7. Window chrome

The same platform split as Nexis.

- **macOS:** native traffic lights over a transparent titlebar; the title bar
  pads 80px on the left for them.
- **Windows:** self-drawn square title bar. Controls use
  `WindowControlArea` so the OS hit-tests them (that is what makes Snap
  Layouts appear on hovering maximize).
- **Linux:** client-side decorations requested; if the compositor gives
  server-side decorations anyway, Ferrite draws **no** controls (no doubles).
- 32px tall (two display rows). `▓▒░ TITLE` on the left, a light dither grip
  filling the drag area, `_ □ x` on the right; close hovers `danger`.

Use `chrome::window_options(..)` for every window and call
`chrome::square_corners(window)` in the `open_window` closure.

---

## 8. Components

Ferrite owns its everyday controls — `Button`, `checkbox`, `radio`,
`switch`, `tag`, `meter`, `tabs`, `list_item`, `tooltip`, `kbd`, `spinner` —
and its framing (`TitleBar`, `Panel`, `StatusBar`, `rule`, `cursor`,
`progress_bar`, `empty_state`, `Dither`). The heavy machinery (inputs,
virtual lists, tables, docking, menus) still comes from **gpui-component**
wearing the Ferrite theme. Status and plan: [COMPONENTS.md](COMPONENTS.md);
see them all with `cargo run --example components`.

The control grammar, in one place:

| Idea | Ferrite expression |
|---|---|
| primary action | amber fill, dark label |
| pressed / toggled on | **inverse video** (fill and ink swap) |
| disabled | dashed 1px frame, faint ink |
| busy | ASCII spinner replaces the glyph; size never changes |
| checked | `[x]` / `(•)` in amber; `[-]` mixed |
| selected row / tab | 2px amber edge + `accent_dim` wash / open bottom |
| icons | 16×16 pixel `Icon`s on the type grid; text glyphs only if `display_has` |
| keyboard focus | 1px amber frame, keyboard-only (`focus_visible`) |
| floating surface | `raised` fill, 1px `line_strong` frame, hard 4px dithered drop shadow |
| modal | the app behind is **screen-doored** — a 50% dither in the page color — never blurred or tinted |
| match highlight | matched characters in `accent_text`, nothing else |

Conventions for any component, ours or wrapped:

- Builder API (`panel("Logs").meta("12 lines").child(..)`), `Styled` +
  `ParentElement` where it makes sense, `RenderOnce` for stateless pieces.
- Stable element ids for anything interactive (tests and focus need them).
- Colors from `palette(cx)` only. No hex literals in components.
- Focus is visible: 1px accent, never removed.

---

## 9. The checklist

An app belongs to the Ferrite family when it has:

1. The Iron/Paper palettes via `ferrite_design::init` — one amber accent,
   no other brand hue, no blue.
2. PxPlus display type at snapped sizes; JetBrains Mono for everything read.
3. 0px corners everywhere, including the window; no shadows.
4. Square self-drawn chrome on Win/Linux, native on macOS, `▓▒░` mark.
5. Dither and ASCII as structural accents only; no gradients or glows.
6. Stepped motion from `motion::*`, honouring reduced motion.
7. Dark by default, Paper fully supported.
8. The theme installed before the first window opens.
9. Uppercase bracketed display labels; dim, AA-legible secondary text.
