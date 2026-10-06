# Scaffolding a New Ferrite App

> Stand up a fresh GPUI desktop app on `ferrite-design`.
>
> **Fastest path:** `scripts/new-app.sh ferrite-<thing> <template>` creates a
> ready-to-run crate from one of six app templates (dashboard, workbench,
> settings, explorer, console, wizard) or `minimal` — see
> [AGENTS.md](../AGENTS.md#1-start-an-app). This page is the same thing by
> hand. Keep [PITFALLS.md](PITFALLS.md) open.

Placeholders: **`ferrite-myapp`** (crate / repo name), **`My App`** (display
name). Ferrite apps are named `ferrite-<thing>`, as Nexis apps are
`nexis-<thing>`.

---

## Step 0 — Prerequisites

- Rust stable (edition 2024; rustc ≥ 1.88).
- **Windows:** MSVC build tools.
- **Linux:** `libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev
  libxcb1-dev libfontconfig-dev` and a Vulkan driver (PITFALLS §44).
- **macOS:** Xcode command-line tools (Metal).

## Step 1 — Create the crate

```bash
cargo new ferrite-myapp
cd ferrite-myapp
```

## Step 2 — Dependencies

```toml
[dependencies]
ferrite-design = { git = "https://github.com/rwetz/ferrite-design" }
# Must match ferrite-design exactly — see PITFALLS §1.
gpui = { package = "gpui-pre", version = "=0.3.8" }
gpui_platform = { package = "gpui-pre-platform", version = "=0.3.8" }
```

And — not optional — optimise dependencies in dev builds, or gpui is
sluggish (PITFALLS §17):

```toml
[profile.dev.package."*"]
opt-level = 3
```

There is no publish step: consume it from git, as `@nexis/design` is. Pin a
`rev` once the app ships.

## Step 3 — `main.rs`

This is `examples/minimal.rs` verbatim (`cargo test` compiles it, so it
cannot rot):

```rust
// Release builds are GUI-subsystem on Windows: no console window behind the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ferrite_design::{Appearance, chrome, components::{power_on_in, status_bar}, palette, tokens::hsla};
use gpui::{App, AppContext as _, Context, IntoElement, ParentElement, Render, Styled,
           Subscription, Window, div, px, size};

struct MyApp {
    _appearance: Subscription,
}

impl Render for MyApp {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        // CRT power-on at every launch; switch-off on close (step 4 below).
        power_on_in("power", div()
            .flex().flex_col().size_full()
            .bg(hsla(p.bg)).text_color(hsla(p.fg))
            .child(chrome::title_bar("My App"))
            .child(div().flex_1().min_h_0() /* your content */)
            .child(status_bar().left("READY")))
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        // 1. Theme + fonts BEFORE any window (no first-frame flash).
        ferrite_design::init(Appearance::Dark, cx);

        // 2. Every window starts from chrome::window_options.
        let options = chrome::window_options("My App", size(px(1200.), px(800.)), cx);
        cx.open_window(options, |window, cx| {
            // 3. Square corners on Windows 11.
            chrome::square_corners(window);
            // 4. Closing plays the CRT switch-off (drawn by power_on_in).
            chrome::power_off_on_close(window, cx);
            // 5. Your view is the window's root — no wrapper.
            cx.new(|_| MyApp { _appearance: ferrite_design::theme::follow_system(window) })
        })
        .unwrap();
        cx.activate(true);
    });
}
```

## Step 4 — Structure

Mirror the Nexis module pattern:

```
ferrite-myapp/
├─ Cargo.toml
├─ src/
│  ├─ main.rs            # init → open_window, nothing else
│  ├─ app.rs             # root view: title bar, workspace, status bar
│  └─ modules/<feature>/ # one folder per feature: view(s) + state + mod.rs
└─ assets/               # app icon source, app-specific assets
```

## Step 5 — Build with the language

- `use ferrite_design::prelude::*;` — components, effects, palette, icons
  and the gpui traits builders need, in one line.
- Frame regions with `panel("Name")`, not bare bordered divs (or
  `ascii_box()` for a text-mode screen — one style per screen).
- Pick a scheme with `theme::set_scheme(..)` after `init`, or let users
  pick one (`SCHEMES`); `theme::apply_env(cx)` honours `FERRITE_SCHEME`,
  `FERRITE_APPEARANCE` and `FERRITE_FPS` for trying looks.
- Display labels: `.display(Scale::X1, window)`, UPPERCASE.
- Body: `.body(text::BASE)`; it's already the theme default for widgets.
- Texture: `dither(..)` only in the places DESIGN_LANGUAGE §5.2 allows.
- Motion: `motion::*` only; check `motion::reduced(cx)`.
- Widgets: `components::*` only — every control, overlay, input, list and
  layout piece is native (COMPONENTS.md). Wrap the root in
  `chrome::window_frame()` so Linux gets resize edges.

## Step 5b — A command palette (recommended)

Every Ferrite app should expose its actions in a palette. One per window:

```rust
use ferrite_design::components::{CommandPalette, TogglePalette, command};

// in the root view's constructor
let palette = cx.new(|cx| {
    let mut p = CommandPalette::new(window, cx);
    p.set_commands(vec![
        command("Open folder").group("File").icon(Icon::Folder).shortcut("Ctrl+O")
            .on_run(|window, cx| { /* … */ }),
        // …
    ], cx);
    p
});

// in render: host it and toggle it
div()
    .on_action(cx.listener(|this, _: &TogglePalette, window, cx| {
        this.palette.update(cx, |p, cx| p.toggle(window, cx));
    }))
    .child(self.palette.clone())

// in main, after ferrite_design::init
cx.bind_keys([KeyBinding::new("ctrl-shift-p", TogglePalette, None)]);
```

Also give it a clickable entry point (a title-bar or toolbar button). The
components example has the full version.

Create the palette in the root view's constructor, as above: it parks
keyboard focus inside your view at startup so Ctrl+Shift+P works before
anything has been clicked (PITFALLS §30).

## Step 5c — Toasts

One `Toaster` per window, hosted next to the palette:

```rust
use ferrite_design::components::{Toaster, toast};

let toaster = cx.new(|_| Toaster::new());          // in the constructor
div().child(self.toaster.clone())                   // in render

self.toaster.update(cx, |t, cx| {
    t.push(toast("Deployed").success().message("staging · build 4412")
        .action("Undo", |window, cx| { /* … */ }), cx);
});
```

Use `.sticky()` for things the user must act on; everything else expires
(4s info/success, 6s warning, 8s danger).

## Step 6 — Run

```bash
cargo run
FERRITE_APPEARANCE=light cargo run --example showcase   # compare against the reference
```

---

## Definition of done

- [ ] First frame is Ferrite (no flash of the library's light theme).
- [ ] Square corners on Windows 11; native traffic lights on macOS.
- [ ] Linux: no doubled window controls under server-side decorations.
- [ ] Window controls work; maximize shows Snap Layouts on Windows.
- [ ] Display type is crisp at 100%, 125% and 150% scale.
- [ ] Dither is crisp (no grey mush) at fractional scale.
- [ ] Paper mode is legible everywhere; the amber accent is used only for
      primary action / active / focus / progress.
- [ ] Reduced-motion OS setting stops blinks and tickers.
- [ ] Idle CPU is near zero with nothing animating (PITFALLS §17).
- [ ] `cargo test` passes in `ferrite-design` at the pinned rev.
