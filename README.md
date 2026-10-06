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
| `theme` | Which palette is on screen: appearance preference, switching, follow-system. |
| `fonts` | Embedded PxPlus IBM VGA 8×16 (display) + JetBrains Mono (body), and device-pixel snapping for the pixel face. |
| `dither` | Dither as an element over fields (flat, ramp, radial) or grayscale pictures. Patterns: 4×4 Bayer (the texture), blue noise, Atkinson (pictures). Device-pixel cells, rasterised once and cached. |
| `ascii` | Shade ramps, bars, spinners, sparklines, brackets, rules, box drawing. |
| `motion` | Stepped motion vocabulary, blink, reduced-motion check, and the live refresh rate (`set_fps`, 12–240fps). |
| `chrome` | Window options, square corners on Windows 11, Ferrite's own `TitleBar`, and `window_frame` (Linux resize edges). |
| `components` | Native controls — `Button`, `checkbox`, `radio`, `switch`, `tag`, `meter`, `tabs`, `list_item`, `tooltip`, `kbd`, `spinner`, `popover`, `dropdown_menu`, `context_menu`, `submenu`, `CommandPalette`, `Toaster`, `dialog`, `segmented`, `slider`, `tree`, `table`, `TextInput`, `scroll_area`, `scrollbar`, `virtual_list`, `split` — and framing: `Panel`, `StatusBar`, `rule`, `cursor`, `progress_bar`, `empty_state`. |
| `icon` | 21 pixel icons drawn on the display font's grid; they never fall back to a system font. |
| `animate` | The motion engine: stepped, eased clips (`play`) and effects — unroll, dissolve, decrypt, typewriter, shake, count, develop, afterglow, interlace, tear, ping, seek, power-on. |
| `fuzzy` | The palette's matcher: exact best-alignment scoring (word starts, runs, gaps) with match positions. |

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
cargo run --example showcase                       # every primitive, Iron
FERRITE_APPEARANCE=light cargo run --example showcase   # Paper
cargo run --example components                     # every native component, live
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
- [COMPONENTS.md](docs/COMPONENTS.md) — every native component, and what each one replaced; was: the plan to
  replace it with native Ferrite components.

## Components: all native

Every component is Ferrite's own — controls, overlays, the command palette,
toasts, dialogs, the text input, scrollbars, virtual lists, split panes,
trees and tables. The crate depends on gpui and nothing built on it. Status
and the replacement log: [COMPONENTS.md](docs/COMPONENTS.md).

## License

Code: [Apache-2.0](LICENSE), matching the rest of the family.

Fonts (embedded, unmodified) keep their own licenses — see [NOTICE](NOTICE):
PxPlus IBM VGA 8×16 by VileR (CC BY-SA 4.0, int10h.org) and JetBrains Mono
(OFL-1.1).
