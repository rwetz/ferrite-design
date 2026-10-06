# ferrite-design

```
▓▒░ FERRITE ░▒▓   phosphor amber on iron grey
```

The design language for the **Ferrite** family of native GPUI desktop apps:
tokens, theme, pixel display type, ordered-dither and ASCII primitives, square
window chrome, and the rules that hold them together.

![Ferrite showcase — Iron](docs/img/showcase-iron.png)

| Paper | Dither & empty state |
|---|---|
| ![Paper](docs/img/showcase-paper.png) | ![Dither](docs/img/showcase-dither.png) |

**Native components** (`cargo run --example components`):

| Iron | Paper |
|---|---|
| ![Components — Iron](docs/img/components-iron.png) | ![Components — Paper](docs/img/components-paper.png) |

| ![Menus, popover and pixel icons](docs/img/components-overlays.png) | ![Cascading submenus](docs/img/components-submenus.png) |
| --- | --- |

| ![Command palette — fuzzy filter over a screen-doored app](docs/img/components-palette.png) | ![Toasts — stepped countdown, one per kind](docs/img/components-toasts.png) |
| --- | --- |

![Data & input — segmented control, sliders, tree, sortable table](docs/img/components-data.png)

| ![A destructive confirm dialog](docs/img/components-dialog.png) |
| --- |

![Text input — native IME, undo, word motion, password mask](docs/img/components-input.png)

![Split pane, scroll area and a 10,000-row virtual list with Ferrite scrollbars](docs/img/components-layout.png)

![Motion, caught mid-frame: the page unrolling behind an amber scan line, a dissolve, a decrypt](docs/img/motion-midframe.png)

**Forms, navigation, charts and text mode:**

| ![Forms — field, select, number, date picker, calendar, accordion](docs/img/components-forms.png) | ![Navigation — sidebar, toolbar, breadcrumb, pagination, steps, alerts, avatars, timeline](docs/img/components-navigation.png) |
| --- | --- |
| ![Charts — stat tiles, stepped line chart, bars, heatmap, sparklines, skeletons](docs/img/components-charts.png) | ![Text mode — ascii_box, banner, gauges, spinners, ASCII art](docs/img/components-ascii.png) |

**Ten color schemes** — Ferrite (amber on iron) plus four neutral and five
wild, each a dark + light pair, all contrast- and hue-tested
([DESIGN_LANGUAGE §2.3](docs/DESIGN_LANGUAGE.md#23-schemes)):

![Neutral schemes: Ferrite, Mono, Graphite, Slate, Concrete](docs/img/schemes-neutral.png)
![Wild schemes: Harbor, Cyanotype, Phosphor, Verdigris, Bruise](docs/img/schemes-wild.png)

**App templates** — whole apps to start from (`scripts/new-app.sh`):

| ![Dashboard](docs/img/app-dashboard.png) | ![Workbench](docs/img/app-workbench.png) |
| --- | --- |
| ![Settings](docs/img/app-settings.png) | ![Explorer](docs/img/app-explorer.png) |
| ![Console](docs/img/app-console.png) | ![Wizard](docs/img/app-wizard.png) |

## Scope

**For:** native desktop apps built on [GPUI](https://gpui.rs) (Rust,
GPU-rendered, no webview).

**Not for:** Tauri / React / Tailwind webview apps. Those are the Nexis
family and use [`nexis-design`](https://github.com/rwetz/nexis-design).

Ferrite keeps Nexis's *rules* — one accent, tokens with a single source of
truth, a shared motion vocabulary, self-drawn chrome with native macOS
controls, no flash on startup, tripwire tests — and replaces its *look*
entirely. Where Nexis is coral glass with rounded corners and springs,
Ferrite is amber on iron: 0px corners, pixel type, dither instead of
gradients, stepped motion. The side-by-side is in
[DESIGN_LANGUAGE.md §1](docs/DESIGN_LANGUAGE.md#1-inherited-from-nexis-vs-new-to-ferrite).

## What's in it

| Module | |
|---|---|
| `tokens` | The Iron (dark) and Paper (light) palettes, type scale, spacing grid. Contrast is enforced by tests. |
| `schemes` | Ten named schemes (Ferrite + Mono, Graphite, Slate, Concrete + Harbor, Cyanotype, Phosphor, Verdigris, Bruise), each dark + light. |
| `theme` | What's on screen: scheme, appearance (dark/light/system), switching, follow-system, `apply_env` dev overrides. |
| `fonts` | Embedded PxPlus IBM VGA 8×16 (display) + JetBrains Mono (body), and device-pixel snapping for the pixel face. |
| `dither` | Dither as an element over fields or grayscale pictures: Bayer, blue noise, Atkinson. Rasterised once and cached. |
| `ascii` | Text-mode strings: bars, gauges, spinner sets, sparklines, box edges, a 5×5 block font, ASCII art. |
| `motion` / `animate` | The stepped motion vocabulary and the engine: keyed clips at the live refresh rate (default 240fps). |
| `chrome` | Window options, square corners on Windows 11, `TitleBar`, `window_frame` (Linux resize edges). |
| `components` | ~55 native components: controls, forms, overlays, navigation, data, charts, text mode, layout, 16 drop-in motion effects ([COMPONENTS.md](docs/COMPONENTS.md)). |
| `icon` | 32 pixel icons drawn on the display font's grid. |
| `fuzzy` | The palette's matcher. |
| `prelude` | `use ferrite_design::prelude::*;` — everything a view needs. |

## Use

```toml
[dependencies]
ferrite-design = { git = "https://github.com/rwetz/ferrite-design" }
gpui = { package = "gpui-pre", version = "=0.3.8" }            # must match exactly
gpui_platform = { package = "gpui-pre-platform", version = "=0.3.8" }

[profile.dev.package."*"]
opt-level = 3      # gpui is sluggish unoptimised — see PITFALLS §17
```

```rust
gpui_platform::application().run(|cx: &mut App| {
    ferrite_design::init(Appearance::Dark, cx);          // before any window
    let options = chrome::window_options("My App", size(px(1200.), px(800.)), cx);
    cx.open_window(options, |window, cx| {
        chrome::square_corners(window);
        cx.new(|_| MyApp::new())                         // your view is the root
    }).unwrap();
});
```

Full walkthrough: [docs/SCAFFOLDING.md](docs/SCAFFOLDING.md).

```bash
scripts/new-app.sh ferrite-pulse dashboard         # a new app from a template
cargo run --example components                     # every component; scheme picker at the top
cargo run --example app_dashboard                  # …app_workbench, app_settings, app_explorer, app_console, app_wizard
FERRITE_SCHEME=harbor cargo run --example app_console   # any example in another scheme
cargo run --example showcase                       # every primitive, Iron
FERRITE_APPEARANCE=light cargo run --example showcase   # Paper
FERRITE_FPS=60 cargo run --example components      # any example at another refresh rate
cargo run --example minimal                        # the scaffolding guide's app
cargo test                                         # palette/dither/input/layout tripwires
```

## Docs

- [DESIGN_LANGUAGE.md](docs/DESIGN_LANGUAGE.md) — the language: color, type,
  space, texture, motion, chrome, and the family checklist.
- [SCAFFOLDING.md](docs/SCAFFOLDING.md) — new app, step by step.
- [PITFALLS.md](docs/PITFALLS.md) — field notes; read before scaffolding.
- [REFERENCES.md](docs/REFERENCES.md) — outside dithering and ASCII-art projects worth
  revisiting.
- [COMPONENTS.md](docs/COMPONENTS.md) — every component, by family, and the rules for writing one.
- [ROADMAP.md](docs/ROADMAP.md) — what "end to end" still needs, in order.
- [AGENTS.md](AGENTS.md) — for coding agents (and people): templates by app type, the rules, an
  exact API cheat sheet, verification. Claude Code gets a `ferrite-scaffold` skill.

## Platforms

Developed on macOS and Windows. Linux works as of this release — before
it, gpui's Linux backend was compiled without a display server and every
window silently opened headless (PITFALLS §44). Linux builds need
`libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libxcb1-dev
libfontconfig-dev`; headless boxes can render and screenshot Ferrite with
Xvfb + Mesa lavapipe ([AGENTS.md §5](AGENTS.md#5-verify-before-you-call-it-done)).

## License

Code: [Apache-2.0](LICENSE), matching the rest of the family.

Fonts (embedded, unmodified) keep their own licenses — see [NOTICE](NOTICE):
PxPlus IBM VGA 8×16 by VileR (CC BY-SA 4.0, int10h.org) and JetBrains Mono
(OFL-1.1).
