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

![Menus, popover and pixel icons](docs/img/components-overlays.png)

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
| `theme` | Projects the palettes onto gpui-component's theme; appearance switching and follow-system. |
| `fonts` | Embedded PxPlus IBM VGA 8×16 (display) + JetBrains Mono (body), and device-pixel snapping for the pixel face. |
| `dither` | 4×4 Bayer ordered dither as an element: flat, ramp, radial. Device-pixel cells, rasterised once and cached. |
| `ascii` | Shade ramps, bars, spinners, sparklines, brackets, rules, box drawing. |
| `motion` | Stepped motion vocabulary, blink, reduced-motion check. |
| `chrome` | Window options, square corners on Windows 11, and Ferrite's own `TitleBar`. |
| `components` | Native controls — `Button`, `checkbox`, `radio`, `switch`, `tag`, `meter`, `tabs`, `list_item`, `tooltip`, `kbd`, `spinner`, `popover`, `dropdown_menu`, `context_menu` — and framing: `Panel`, `StatusBar`, `rule`, `cursor`, `progress_bar`, `empty_state`. |
| `icon` | 21 pixel icons drawn on the display font's grid; they never fall back to a system font. |

## Use

```toml
[dependencies]
ferrite-design = { git = "https://github.com/rwetz/ferrite-design" }
gpui = { package = "gpui-pre", version = "=0.3.8" }            # must match exactly
gpui_platform = { package = "gpui-pre-platform", version = "=0.3.8" }
gpui-component = "=0.7.1"

[profile.dev.package."*"]
opt-level = 3      # gpui is sluggish unoptimised — see PITFALLS §17
```

```rust
gpui_platform::application().run(|cx: &mut App| {
    ferrite_design::init(Appearance::Dark, cx);          // before any window
    let options = chrome::window_options("My App", size(px(1200.), px(800.)), cx);
    cx.open_window(options, |window, cx| {
        chrome::square_corners(window);
        let view = cx.new(|_| MyApp::new());
        cx.new(|cx| Root::new(view, window, cx))
    }).unwrap();
});
```

Full walkthrough: [docs/SCAFFOLDING.md](docs/SCAFFOLDING.md).

```bash
cargo run --example showcase                       # every primitive, Iron
FERRITE_APPEARANCE=light cargo run --example showcase   # Paper
cargo run --example components                     # every native component, live
cargo run --example minimal                        # the scaffolding guide's app
cargo test                                         # palette/schema/dither tripwires
```

## Docs

- [DESIGN_LANGUAGE.md](docs/DESIGN_LANGUAGE.md) — the language: color, type,
  space, texture, motion, chrome, and the family checklist.
- [SCAFFOLDING.md](docs/SCAFFOLDING.md) — new app, step by step.
- [PITFALLS.md](docs/PITFALLS.md) — field notes; read before scaffolding.
- [COMPONENTS.md](docs/COMPONENTS.md) — gpui-component today, the plan to
  replace it with native Ferrite components.

## Components: native first, gpui-component for the heavy parts

Everyday controls, popovers and menus are native Ferrite components.
Inputs, virtual lists, tables and docking still come from
[gpui-component](https://github.com/longbridge/gpui-kit), re-skinned by
Ferrite's theme, and are being replaced one at a time. Status, order and
rules: [COMPONENTS.md](docs/COMPONENTS.md).

## License

Code: [Apache-2.0](LICENSE), matching the rest of the family.

Fonts (embedded, unmodified) keep their own licenses — see [NOTICE](NOTICE):
PxPlus IBM VGA 8×16 by VileR (CC BY-SA 4.0, int10h.org) and JetBrains Mono
(OFL-1.1).
