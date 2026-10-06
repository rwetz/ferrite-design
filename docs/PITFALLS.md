# Pitfalls — field notes from the first scaffold

> Each entry bit for real while standing up `ferrite-design` and its showcase
> (2026-10-05, Windows 11, 150% scale, gpui-pre 0.3.8, gpui-component 0.7.1).
> Read before SCAFFOLDING.md. Where the crate already handles it, the entry
> says so — keep it here so nobody "simplifies" the fix away.

---

## 1. gpui must be the exact build gpui-component was compiled against

gpui-component 0.7.1 depends on `gpui = { package = "gpui-pre", version =
"=0.3.8" }`. If an app depends on any other gpui (the `gpui` crate on
crates.io, a Zed git rev, a newer `gpui-pre`), it gets **two copies of gpui**
and every API boundary fails with baffling "expected `App`, found `App`"
errors.

**Fix:** name the same package and version, and bump them together:

```toml
gpui = { package = "gpui-pre", version = "=0.3.8" }
gpui_platform = { package = "gpui-pre-platform", version = "=0.3.8" }
gpui-component = "=0.7.1"
```

## 2. There is no `Application::new()` — the platform is a separate crate

This gpui snapshot moved platform backends out of `gpui`. Apps start with
`gpui_platform::application().run(..)`, from `gpui-pre-platform`. Older
examples and blog posts showing `Application::new()` / `App::new()` will not
compile.

## 3. Theme colors written to the live theme get wiped on mode change — ✅ handled

gpui-component reloads the active mode's `ThemeConfig` on every light/dark
switch, overwriting anything set directly on `Theme::global_mut(cx)`.

**Fix (in `theme::install`):** build a full `ThemeConfig` per palette and
install both as `light_theme` / `dark_theme`; then mode changes keep Ferrite.

## 4. Renamed theme keys fail silently — ✅ tripwire

gpui-component's theme schema is deserialised with serde, which ignores
unknown keys. A key renamed in a library bump makes that widget quietly fall
back to the library default (shadcn neutral/blue), with no error.

**Tripwire:** `theme::tests::every_color_key_survives_the_round_trip`
serialises the parsed config back and fails if any key we sent was dropped.
Run `cargo test` after every gpui-component bump.

## 5. The theme `json!` literal needs a raised recursion limit — ✅ handled

The full config is one `serde_json::json!` literal and exceeds the default
macro recursion limit. `lib.rs` sets `#![recursion_limit = "512"]`.

## 6. Pixel fonts blur at fractional scale — ✅ handled by `display_size`

At 125%/150% a 16px pixel font lands on 20/24 device px and smears. Always set
display type through `.display(Scale, window)`; never `.text_size()` on the
display face. Consequence to accept: at 125% display type renders *smaller*
than requested, at 150% *larger*. That is the price of crisp.

## 7. Dither drawn in logical pixels turns to grey mush — ✅ handled

A 1-logical-px checkerboard at 150% covers 1.5 device px per cell and
averages to flat grey. `Dither` sizes cells in **device** pixels and snaps the
grid origin to the device. Don't hand-roll dither with logical sizes.

## 8. Windows 11 rounds every window's corners — ✅ handled

DWM rounds top-level windows regardless of what the app paints, which breaks
the 0px rule. `chrome::square_corners(window)` sets
`DWMWA_WINDOW_CORNER_PREFERENCE = DWMWCP_DONOTROUND`. It must be called per
window, from the `open_window` closure. Harmless on Windows 10.

## 9. `window.window_handle()` is not the raw OS handle

gpui's `Window` has an *inherent* `window_handle()` returning gpui's own
`AnyWindowHandle`, which shadows the `raw_window_handle::HasWindowHandle`
trait method. To get the HWND, call the trait explicitly:
`HasWindowHandle::window_handle(window)`.

## 10. Windows title-bar controls must not have click handlers

On Windows the min/max/close buttons work by declaring
`.window_control_area(WindowControlArea::Min | Max | Close)`; the OS hit-tests
them. Adding `on_click` handlers instead *works* but loses Snap Layouts on
the maximize button. Linux is the opposite: there are no OS control areas, so
the buttons need click handlers. `chrome::TitleBar` does both, per platform.

## 11. Linux may refuse client decorations → doubled controls — ✅ handled

`WindowDecorations::Client` is a request. X11 without a compositor, and some
Wayland compositors, grant server-side decorations anyway, and drawing our own
controls on top gives two close buttons. `TitleBar` checks
`window.window_decorations()` and draws no controls unless client-decorated,
and honours `window.window_controls()` (tiling WMs may offer neither minimize
nor maximize).

## 12. macOS: let the app own title-bar dragging — ✅ handled

Without `app_owns_titlebar_drag: true`, AppKit treats the transparent title
bar as a system drag region, handles double-clicks itself *and* delays every
title-bar click while it waits to see if it's a double-click.
`chrome::window_options` sets it; `TitleBar` starts the move on the first
mouse-move after a press (not on press, so children stay clickable).

## 13. Install the theme before opening the window — ✅ in `init`

gpui-component's `init` installs its own default theme (light). A window
opened before `ferrite_design::init` paints at least one frame of it — the
GPUI equivalent of the Nexis theme flash. Order is always:
`ferrite_design::init(..)` → `cx.open_window(..)`.

## 14. Linux resize edges and `window_border` (open)

With client decorations on Linux the app must provide resize edges.
gpui-component's `window_border()` does, but rounds some corners with its own
radius, which conflicts with 0px. Not yet replaced — see COMPONENTS.md. Until
then, Linux apps either accept that or skip `window_border` and lose edge
resizing.

## 15. Windows: top 1–2px may show what's behind the window (unverified)

Screenshots of the showcase on Windows 11 at 150% showed the desktop through
the top 1–2 device pixels of the window, likely the resize border gpui keeps
on a transparent-titlebar window. Not yet confirmed by eye or fixed. Check
before shipping an app that paints content right up to the top edge.

## 16. Testing note: drive the window, not the desktop

When scripting a Ferrite app for screenshots or interaction tests on Windows:

- **Don't move the real cursor or send global input** (`SetCursorPos`,
  `mouse_event`, `SendInput`). If anything else is in front — and Windows
  often refuses `SetForegroundWindow` — the clicks and scrolls land in
  *that* window. This happened while building the components showcase.
- Instead **post messages to the app's own HWND** (`WM_MOUSEMOVE`, then
  `WM_LBUTTONDOWN`/`UP` with client coordinates; `WM_MOUSEWHEEL` with screen
  coordinates) and **capture with `PrintWindow(hwnd, hdc,
  PW_RENDERFULLCONTENT)`**, which works even when the window is covered.
  gpui handles posted input normally.
- Make the script DPI-aware (`SetProcessDPIAware`) or every coordinate is
  scaled at 125%/150%.
- **Wheel:** `PostMessage(WM_MOUSEWHEEL)` is unreliable; `SendMessage` it
  (lParam = *screen* coordinates). Clicks and keys post fine.
- **PowerShell:** don't name a helper `Move` — the built-in `move` alias
  (`Move-Item`) wins over your function and tries to move files.
- Launch debug builds from a shell, not `Start-Process`: debug binaries are
  console-subsystem (§18) and the console window lands on top.

## 17. Per-frame work pegs a core — ✅ handled

The first showcase idled at **~99% of a CPU core** (debug build). Three causes,
each now fixed; measured after the fix: **3.8%**, with the showcase's own
12.5 Hz progress ticker still running.

1. **`with_animation(..).repeat()` redraws the whole window every display
   frame**, forever, for as long as the element is on screen. A blinking
   cursor built that way made every other element re-render at 60+ fps.
   For anything periodic and discrete (blinks, spinners, tickers) use a
   timer that flips state and calls `cx.notify()` at the rate the *content*
   changes — `components::Cursor` redraws 2×/s.
2. **Dither drawn as per-cell quads** is tens of thousands of quads per
   element per frame. `Dither` now rasterises once into an image at device
   resolution, caches it by (field, size, scale, colors), and paints one
   textured quad. The cache is bounded and frees evicted textures from the
   GPU atlas (`window.drop_image`), so resizing can't leak. This is why
   `dither::Field` is plain data, not a closure: it has to be a cache key.
3. **Unoptimised dependencies.** gpui and gpui-component are dramatically
   slower at `opt-level = 0`. Every Ferrite app's `Cargo.toml` needs:

   ```toml
   [profile.dev.package."*"]
   opt-level = 3
   ```

   Your own crate stays at 0, so debugging is unaffected.

General rule: nothing in a Ferrite view should cost O(pixels) per frame.
Measure idle CPU (`Get-Process`/`top`) whenever adding anything animated.

## 18. Windows: a console window opens behind the app

A Rust binary is console-subsystem by default, so launching it from Explorer
or a shortcut opens a terminal window too. Every Ferrite `main.rs` starts
with:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
```

Release builds are GUI-only; debug builds keep the console for logs.

## 19. gpui has no per-side border colors

`border_t_color` and friends don't exist — one `border_color` covers every
side. For a colored edge (the active tab's amber top), draw a separate
absolutely positioned 2px element. To "open" one side (the active tab onto
its content), give the neighbours that side's border and leave it off this
element, rather than recoloring it.

## 20. A loading button must not change width

The first components showcase relabelled a button "Deploy" → "Deploying"
and added a spinner on click; the button grew, its neighbour shifted under
the pointer, and the next click hit the wrong control. Give loading-capable
buttons a glyph (the spinner replaces it) and keep the label constant.

## 21. Interactive components need a focus handle in keyed state

A focus handle created in `render` is a new handle every frame, so focus is
lost on every redraw. Store it with
`window.use_keyed_state(id, cx, |_, cx| cx.focus_handle())`, as every Ferrite
component does; it lives exactly as long as the element keeps rendering.

## 22. Symbols outside the display face silently fall back — ✅ tripwire

PxPlus IBM VGA covers CP437 + WGL4 only. Anything else — `↻ ▶ ▸ ⚙ ✓` — is
drawn by gpui in a fallback system font with the wrong weight, size and
baseline, and nothing warns you. It looks "a lil wonky" (the first small
refresh button).

- Use a pixel [`Icon`](../src/icon.rs) for symbols. Icons share the type's
  pixel grid and can't fall back.
- `fonts::display_has(c)` answers whether a character is covered (it reads
  the font's own `cmap`).
- `fonts::tests::every_glyph_ferrite_sets_in_the_display_face_exists` lists
  every character Ferrite sets in the display face and fails if any is
  missing. Add to it when you add a glyph.

## 23. Icons inside fixed-height controls must fit, not scale

Display-scale icons snap up at fractional scale factors (X1 is 32 device px
at 150%), which overflows a 24px button. Inside a control use
`icon(..).fit(max)`: the largest whole multiple of 16 device px that fits.
Icon bitmaps keep a 1px margin, so `fit` can use nearly the full inner
height — Button uses 22px (small) / 30px (medium).

## 24. Overlays: decide open/close on mouse *down*, and guard the trigger

A popover's click-outside handler and its trigger both see a click on the
trigger while open. Toggle on `on_mouse_down` and only act if the state
still equals what this render saw (`if s.open == open`); otherwise the
outside handler closes it and the trigger immediately reopens it. Render
the surface with `deferred(anchored(..))` so it paints above everything
and snaps inside the window, and move focus into it on open (or Escape and
arrow keys never reach it). Restore focus on close only if it's still
inside the surface.

## 25. Key bindings run before key listeners — ✅ handled in the palette

gpui dispatches a keystroke to **key bindings first** (deepest key context
first, trying the next only if a handler calls `propagate()`), and only then
to `on_key_down` listeners. So a container listening for ↑/↓/Enter never sees
them while a text input inside it is focused — the input binds those keys.

For anything that navigates while an input has focus, register **bindings**
in your own key context on an ancestor (`.key_context("FerritePalette")` +
`cx.bind_keys(..)` + `.on_action(..)`). A single-line input has nowhere to
move ↑/↓ and propagates them, so your binding gets them next. As a backstop,
also handle the input's `InputEvent::PressEnter`, and make "confirm"
idempotent so receiving both is harmless.

## 26. Don't run user callbacks inside your own entity update

A palette command handler may update any entity — including the palette.
Called directly from the palette's `cx.listener`, that is a re-entrant
update and panics. Close first, then run the handler on the next tick with
`window.defer(cx, ..)`.

## 27. Testing note: more PowerShell aliases, and stubborn wheel events

- `Type` is an alias for `Get-Content` (like `Move` → `Move-Item`, §16):
  name helpers `TypeText`, `MouseTo`. Text goes in as posted `WM_CHAR`.
- Modifier shortcuts (Ctrl+Shift+P) can't be faked with posted messages —
  they read real keyboard state. Give every palette-style feature a
  clickable entry point too (that's good UX anyway) and test through it.
- Sent `WM_MOUSEWHEEL` sometimes takes a couple of rounds after launch
  before gpui scrolls. Send in batches and re-check with a capture.

## 28. A child's `.occlude()` hides the pointer from its parent's hover

Occlusion blocks every hitbox *below* it — including the parent's own.
Put `.occlude()` on each toast and the stack's `on_hover` never fires, so
"pause while pointed at" silently does nothing. Occlude the container that
needs the hover, not its children; the children's hitboxes sit above it and
still get clicks.

## 29. Timers for transient UI: run only while there's something to time

The toast stack ticks every `motion::FRAME` *only while a toast is on
screen*, stops itself when the stack empties, and calls `cx.notify()` only
when something visible changed (a countdown segment, an entry frame, a
removal). An always-on timer or a notify on every tick would put idle
redraws back (§17).

