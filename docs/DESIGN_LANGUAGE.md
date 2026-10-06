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
| Tokens have one source of truth | `globals.css` + theme engine | `src/tokens.rs` → `palette(cx)` in every component |
| Theme applied before first paint | `index.html` anti-flash script | `theme::install` before `open_window` |
| Shared motion vocabulary | springs (`snappy/smooth/gentle`) | stepped, eased clips (`animate::play`, `FAST/BASE/SLOW`, `BLINK`) |
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
  desaturated syntax/ANSI colors. (Other schemes, §2.3, may tint their
  neutrals blue — Harbor, Slate, Cyanotype — but blue is never "info".)
- **Usable greys.** Iron greys have no hue; the whites carry a hair of warmth
  so amber reads hot against cold metal instead of muddy.
- **Paper is a printout, not a web page.** Warm off-white stock, ink type,
  the same dither. The accent darkens for text (`accent_text`) because pale
  amber on paper is illegible.
- **Contrast is enforced.** `tokens::tests` fails the build if `fg` drops
  below 7:1, `fg_dim` / `accent_text` / status colors below 4.5:1, or a line
  becomes invisible (or loud). Change a value, run `cargo test`.

### 2.3 Schemes

Iron and Paper are the **Ferrite** scheme: the signature and the default.
`src/schemes.rs` ships nine more, each a dark + light pair, switched at
runtime with `theme::set_scheme` (or `FERRITE_SCHEME=<key>` in the
examples):

| Shelf | Schemes |
|---|---|
| signature | **Ferrite** — phosphor amber on iron grey |
| neutral | **Mono** (pure black/white, accent = inverse video) · **Graphite** (soft mid-greys, silver) · **Slate** (cool blue-greys, steel) · **Concrete** (warm greys, bone) |
| wild | **Harbor** (muted orange on blue-slate) · **Cyanotype** (prussian blueprint, chalk yellow) · **Phosphor** (P1 green CRT) · **Verdigris** (copper brown, patina teal) · **Bruise** (aubergine, acid lime) |

![Neutral schemes](img/schemes-neutral.png)
![Wild schemes](img/schemes-wild.png)

A scheme changes colors and nothing else, and keeps the color rules:

- **One accent per scheme.** A scheme that pairs two hues puts the second
  in the *neutral ramp*: Harbor is orange on blue-tinted iron, not orange
  *and* blue accents. "Active" is always exactly one color.
- **Same ladder.** Every scheme's `sunken → bg → surface → raised → line`
  follows Iron's and Paper's lightness (measured in OKLCH), so depth reads
  the same everywhere. Light variants stay paper: near-neutral stock.
- **An accent never looks like a status.** Any accent with real chroma
  keeps ≥15° of OKLCH hue from danger, success and warning (a scheme may
  move a status hue to make room — Phosphor's success is teal, Cyanotype's
  warning is orange). Tested in `tokens::tests`, with the contrast rules.

The palettes were generated from a small OKLCH model (neutral hue + chroma,
one accent) and contrast-corrected; hand-tune freely, then `cargo test`.

### 2.4 Painting colors yourself

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
its grid, and cached like dither. 32 ship: `plus minus close check play
stop pause refresh search up down chevron_up chevron_down chevron_left
chevron_right menu more copy trash file folder warning info dot sliders
user calendar home bell chart terminal lock`. Inside fixed-height controls use
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
- Patterns (`.pattern(..)`), compared side by side in the showcase:

  | Pattern | Kind | Use |
  |---|---|---|
  | `Bayer4` | ordered | **the texture**; default for fields |
  | `BlueNoise` | ordered (64×64 void-and-cluster tile) | an organic, grid-free grain where the crosshatch is too busy; as stable and steppable as Bayer |
  | `Atkinson` | error diffusion | **pictures only** (`dither::Picture`); default for them. High contrast by design: it drops 2/8 of the error, so shadows crush and highlights blow out |

  Error diffusion never goes on structural texture or anything animated:
  each cell depends on the ones before it, so the pattern reshuffles when
  the size changes and worms across flat fields.

### 5.2 Where texture goes — structural accents, not noise

| ✅ Use | ❌ Never |
|---|---|
| panel-header fill after the title | behind body text |
| title-bar drag strip | inside inputs or list rows |
| progress leading edge (`▓▒░`) | animated continuously |
| empty states, loading placeholders | as a full-window background behind content |
| large content-free regions | more than one dither density in one small area |

### 5.3 ASCII and text mode

The other half of the texture: the machine drawing with its own character
set. Two layers:

- **Strings** (`src/ascii.rs`): `bar`, `gauge`, `spinner` frame sets,
  `sparkline`, `shade`, `bracket`, `rule`, `box_edge`, the 5×5
  `block_glyph` font and `art` (any level function → ASCII art). They
  compose with any element; mind which face renders them (§3.2).
- **Elements** (`components::textmode`), drawn on the display face's cell
  grid:

  | Element | What it is | Use it for |
  |---|---|---|
  | `ascii_box()` | a frame of real box glyphs, `┌─[ TITLE ]─┐`, `╔═╗` (`.double()`), `╒═╕`, `╓─╖` or `+-+` (`.style(..)`), any size; `.shadow()`, `.draw_on(..)` | terminal-feel blocks: system status, boot screens, about boxes, dialogs, a retro mode |
  | `ascii_rule(label)` | `── LABEL ─────` across the width (`.double()`) | dividers inside text-mode screens |
  | `banner(id, text)` | big 5×5 block letters, drawn on with a wipe | title screens, splash, empty states |
  | `ascii_art(picture)` | a grayscale `Picture` as characters: a ramp, or a `Charset` (punctuation, slashes, accents, box drawing, best character…) matched by shape against the face's real glyphs | hero art, about boxes, loading art |
  | `ascii_gauge(v)` | `CPU [████▒·····]  42%` | dense live readouts; never animates |
  | `ascii_table()` | a boxed text grid: `┌──┬──┐ │ │ ├──┼──┤` | readouts, reports, about boxes (`table` is the interactive one) |
  | `ascii_tree()` | `├── src/` `│   └── lib.rs` | file listings, outlines, dependency trees |
  | `ascii_plot(values)` | `*` points joined by `:`, a labelled axis | a series in a text screen |
  | `ascii_bars()` | `MON ████████····· 12.0K` | a comparison in a text screen |
  | `ascii_cal(y, m)` | the month as `cal` prints it | dates in a text screen (`calendar` is the interactive one) |
  | `marquee(id, text)` | a looping ticker | an attract screen, a status ticker — one per screen |
  | `ascii_button(id, label)` / `ascii_list(id)` | `< OK >`, `► ITEM` | text-mode dialogs and menus |
  | `spinner(..).frames(ascii::spinners::X)` | `LINE` `SHADE` `DOTS` `PULSE` `BOUNCE` `ARROWS` `BAR` `GROW` `SUITS` | busy states |
  | `mark()` | the `▓▒░` mark, as real dither | the title bar, sidebars, splash |

Rules:

- **One framing style per screen.** `panel()` (hairlines + dither header)
  for everyday UI; `ascii_box` for screens that want to *be* a terminal.
  Mixing them on one screen looks like two apps.
- **Dense block glyphs are painted, not typeset.** `█ ▀ ▄ ▓ ▒ ░` as text get
  fringed and seamed by subpixel anti-aliasing (PITFALLS §47), so the
  banner, the gauge fill and the mark are quads and dither. Thin glyphs —
  box lines, letters, punctuation — are text.
- ASCII art uses printable ASCII only; for shaded pictures use
  `dither(picture)`, which is the same idea done crisply.

![Text-mode elements](img/components-ascii.png)

## 6. Motion

Nexis glides on springs; Ferrite **snaps**. The brief: **jerky yet smooth,
bold yet contained.**

- **Jerky** — the curve is front-loaded and the effects are mechanical
  (scan lines, dither, noise, tables of keyframes). Apps run at 240fps by
  default, as smooth as the display; `motion::set_fps(25)` brings back the
  classic stepped look, where you can count the frames (§6.2).
- **Smooth** — the steps follow one steep ease-out (`animate::snap`, cubic),
  so the first frames take big bites and the last ones settle. A steady
  cadence plus a front-loaded curve reads as fluid, not broken.
- **Bold** — effects you can't miss: a CRT unroll with an amber scan edge, a
  Bayer dissolve, text that decrypts into place, a shake.
- **Contained** — every effect stays inside its element's box and is over in
  120–320ms. Clips run a timer only while playing; an idle window does no
  work.

| Token | Value | Use |
|---|---|---|
| — | instant | hover, select, focus — anything the pointer does continuously |
| `motion::FRAME` | 40ms | the classic 25fps step; the fixed beat for table effects |
| `motion::frame()` | 4.2ms default (240fps) · up to 83ms | the live step for smooth effects; `motion::set_fps` (12/25/30/60/120/240) |
| `motion::FAST` | 120ms · 3 frames | menus, popovers, switch thumbs |
| `motion::BASE` | 200ms · 5 frames | modals, pages, tab bars, decrypts |
| `motion::SLOW` | 320ms · 8 frames | shakes, number rolls, large unrolls |
| `motion::BLINK` | 1060ms | caret / live-marker cycle, square wave |

### 6.1 The vocabulary

| Effect | What it does | Where Ferrite uses it |
|---|---|---|
| **unroll** | clips top-down to `snap(t)` of the height, 2px amber scan line on the edge | every floating surface opening (menus, submenus, popover, palette, dialog), page switches |
| **screen door** | modal backdrop density steps up the Bayer ramp to 69% | palette, dialog |
| **dissolve** | `bg`-ink dither veil thins through the 16 Bayer levels | content swapping in |
| **decrypt** | characters lock in left→right; the rest churn through `#%&*+=<>/\|$@?!` each frame; spaces hold the word shapes | status lines, results arriving |
| **typewriter** | characters appear in order with a `█` cursor | prompts, log lines |
| **shake** | 0, 6, −5, 4, −3, 2, −1, 0px — one step per frame, never beyond 6px | rejected input, failed actions |
| **travel** | a part moves between two rest positions in eased whole-pixel steps | switch thumb |
| **grow** | an indicator widens from its centre | the active tab's amber bar |
| **step-in** | dither + 40→14→4→0px slide | toasts |
| **count** | a number rolls to its value | totals, timings; meters with `.roll(true)` |
| **flash** | a button floods ▓ with its text color, then dissolves back | every button click |
| **stamp** | a mark arrives through two noise glyphs (`#` `*` → `x`) | checkbox, radio |
| **sweep** | the selection wash fills left→right behind the row; the amber bar lands at once | list items, table rows, tree rows |
| **cascade** | items arrive one frame apart (max 8): decrypting rows, or unrolling items with `cascade_in` | tree branches opening, timelines, install logs |
| **boot** | a header's title decrypts while its dither rule draws on | every panel and the title bar, on first appearance |
| **develop** | a blue-noise veil thins speck by speck, 64 levels | pictures and art arriving (`develop`) |
| **afterglow** | the old value lingers under the new one: amber, then dim, then gone | values that change as events (`afterglow`) |
| **interlace** | even rows sweep down, then the odd rows fill in | page and panel switches, heavier than unroll (`interlace_in`) |
| **tear** | three horizontal bands jolt sideways for three frames, 6px max | system failures — shake is for *input* errors (`tear`) |
| **ping** | a square dither ring steps out 8px, ▓ → ▒ → ░ | a new item arriving at a marker (`ping`) |
| **seek** | the active edge steps from the old tab to the new one, resizing | tab strips |
| **power-on / off** | an amber line draws out from the centre, then the picture opens vertically; closing runs it backwards | every window, each launch (`power_on_in`); the switch-off with `chrome::power_off_on_close` |
| **wipe** | unroll on its side: revealed from one edge behind a vertical amber scan line | drawers, sidebars, section switches (`wipe_in`) |
| **scan** | one amber line passes down over content that is already there | "refreshed in place": a reloaded pane, a re-run query (`scan`) |
| **flash** | the button's click flash for any element: floods ▓ in the accent, dissolves back | "look here" — a value that needs attention (`flash`) |
| **glitch** | the new palette paints at once; strips of the old one tear away over it (solid → ▓ → ▒ → ░, jumping rows each beat, old-accent edges) while the window jolts 4px | every scheme or tone switch, automatically (`chrome::window_frame`) |
| **menu cascade** | under the unroll, each row's label and shortcut decrypt a beat after the row above, sliding in 6 → 2 → 0px | every menu and submenu level |
| **landing** | a picked day floods ▓ and dissolves back while a ring steps out around it; the date picker closes once it has landed | calendar, date picker |
| **boot screen** | a text-mode POST after the power-on: lines print three beats apart (dot leader types on, status decrypts, memory counts up), the boot line types, the app dissolves in. ~1.4s; any key or click skips | opt-in, once per window (`boot_screen`) — consoles and retro tools |
| **draw-on** | charts wipe in left→right once; bars grow from the baseline a frame apart | `line_chart`, `bar_chart` — live updates after that are instant |

### 6.2 Refresh rate

The default is 240fps (`motion::DEFAULT_FPS`): smooth on any common
display, 120Hz included. Apps — or their users — lower it with
`motion::set_fps` for the stepped CRT look (25 is the classic). The
examples take `FERRITE_FPS` and have a REFRESH picker. The rate changes
how *finely* an effect steps, never how long it lasts:

- **Smooth effects** (unroll, dissolve, travel, grow, develop, interlace,
  power-on, seek, and anything on `p.t` / `p.eased()`) take more, smaller
  steps.
- **Table effects** (shake, tear, ping, stamp, toast step-in, decrypt
  churn) index a keyframe table by `p.frame`, which counts fixed 40ms
  beats at any rate — so their timing never changes.
- Cost is per redraw: an idle window still does no work, but anything
  playing redraws at the chosen rate, up to the display's refresh (gpui
  draws on vsync, so 240 on a 60Hz panel costs about what 60 does). The
  showcase's always-running progress demo is the worst case: roughly 40%
  of a core at 240 and ~9% at 25 on an M-series Mac.

### 6.3 Rules

- **Entrances animate; exits don't.** Closing is instant — the user has
  already moved on. One exception: a window switching off like a CRT
  (320ms), because that is the machine powering down, not content leaving.
  The date picker also holds its popup for one `BASE` so the picked day
  lands before it goes — the landing is the entrance of the new value.
- **Continuous input never animates** (hover, drag, typing, scrolling),
  and neither does **live data** — a meter fed several times a second
  updates instantly (`.roll(true)` is for values that change as events).
  Motion marks *events*; something always moving is noise, and it keeps
  the window redrawing at the refresh rate.
- **One effect per event.** A dialog unrolls; it doesn't also slide and
  shake.
- **Periodic motion runs on timers**, not repeating animations (PITFALLS
  §17). Clips do too: `animate::play` ticks at the refresh rate only
  while playing.
- **Reduced motion:** every clip returns "done" — the end state renders
  immediately; blinks go solid.

```rust
// Any element: plays on first render and whenever the key changes.
let p = animate::play("status", &message, motion::BASE, window, cx);
div().child(animate::scramble(&message, p))

// Or the drop-in elements:
decrypt("status", message)
shake("name", error_count, field)
unroll_in("page", page_index, content)
```

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

Every component is Ferrite's own — controls, overlays, the text input,
scrolling, lists, layout and framing. Nothing in a Ferrite window comes
from a component library. Catalogue: [COMPONENTS.md](COMPONENTS.md); see
them all with `cargo run --example components`.

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
| modal | the app behind is **screen-doored** — an 11/16 (~69%) dither in the page color — never blurred or tinted |
| dialog | modal: `[ TITLE ]` header with a dither rule, footer buttons with their keys (Esc / Enter); destructive = danger header + warning icon |
| slider | a row of 6px LED cells snapped to the step, the thumb one tall foreground cell; readout in the display face at a fixed width |
| segmented | one framed strip, selected segment inverse video (amber block) |
| tree | 1px connector guides in `line_strong`, centred under the parent's chevron — never box-drawing glyphs, which break between rows |
| table | display-face header on `surface` with a `line_strong` rule; ▲/▼ in amber on the sorted column; numbers right-aligned |
| submenu | opens to the right, instantly on hover, first row level with its parent row; parent row stays highlighted; `>` chevron marks the row |
| text input | `sunken` well, 1px `line_strong` frame → amber when focused; 2px amber caret blinking at `motion::BLINK`; selection is a translucent amber wash |
| scrollbar | 10px track on `surface` with a hairline, square `line_strong` thumb; `fg_faint` on hover, amber while dragged |
| split | 1px divider, amber while hovered or dragged; resize cursor over a 7px grab zone |
| notification | toast: floating surface + 2px tone bar on the left, display-face code (`INFO OK WARN ERR`), countdown drained in 16 steps along the bottom edge |
| match highlight | matched characters in `accent_text`, nothing else |
| form field | display-face label in a fixed column; hint dim under the control; error replaces the hint in danger with a warning icon, and the row shakes once per new error |
| select | a `sunken` well like an input, value + chevron; the menu marks the current option with a check |
| number | `[-] 008 [+]`: zero-padded, fixed-width display-face readout between two square steppers |
| date | month grid of 32px cells; today framed in amber, selection inverse video, out-of-range days in `line_strong` |
| drawer | a dialog pinned to a window edge: full height, wipes in from that edge over the screen door |
| sidebar | `surface` column; section labels in faint display type with a dither rule; the active item gets the selected-row treatment; collapsed = icon rail with tooltips |
| pagination / steps | square cells; current page inverse video; done steps carry an amber check and an amber connector |
| alert | inline: `raised` box with a 2px tone bar on the left, tone icon, `[ TITLE ]` in the tone — a toast that lives in the layout |
| loading | `skeleton`: light dither in the content's shape with a stepped scan band — never a spinner over empty space |
| charts | steps, not curves: values hold for their column; area = ░ dither in the accent; density = dither level (heatmaps); hover inverts or draws a 1px crosshair |
| avatar | a 5×5 mirrored identicon on the pixel grid, or initials in the display face; presence is a square |

Conventions for any component, ours or wrapped:

- Builder API (`panel("Logs").meta("12 lines").child(..)`), `Styled` +
  `ParentElement` where it makes sense, `RenderOnce` for stateless pieces.
- Stable element ids for anything interactive (tests and focus need them).
- Colors from `palette(cx)` only. No hex literals in components.
- Focus is visible: 1px accent, never removed.

---

## 9. The checklist

An app belongs to the Ferrite family when it has:

1. A shipped scheme via `ferrite_design::init` (+ `theme::set_scheme`;
   Ferrite's Iron/Paper by default) — one accent, no second brand hue.
2. PxPlus display type at snapped sizes; JetBrains Mono for everything read.
3. 0px corners everywhere, including the window; no shadows.
4. Square self-drawn chrome on Win/Linux, native on macOS, `▓▒░` mark.
5. Dither and ASCII as structural accents only; no gradients or glows.
6. Stepped motion from `motion::*`, honouring reduced motion.
7. Dark by default, Paper fully supported.
8. The theme installed before the first window opens.
9. Uppercase bracketed display labels; dim, AA-legible secondary text.
