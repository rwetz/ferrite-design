# AGENTS.md — building with Ferrite

Instructions for coding agents (and people) using **ferrite-design** to
build native desktop apps, and for working on ferrite-design itself.
Read the section you need; the API cheat sheet is exact, so copy from it
rather than guessing builder names.

- **Ferrite** = a design language + component library for **GPUI** (Rust,
  GPU-rendered, no webview). Amber on iron grey, 0px corners, pixel display
  type, dither instead of gradients, stepped motion. Ten color schemes.
- **Not for** Tauri/React/web apps (that's `nexis-design`).

---

## 1. Start an app

```bash
# from a checkout of ferrite-design
scripts/new-app.sh ferrite-<thing> <template> [dir]
cd ../ferrite-<thing> && cargo run
```

`new-app.sh` writes `Cargo.toml` (ferrite-design pinned by git rev, gpui
pinned exactly, dev deps optimised), copies the template to `src/main.rs`
with the window title renamed, and adds an `AGENTS.md` for the new app.
The git pin only resolves once the rev is pushed; to work against a local
checkout use `FERRITE_PATH=/path/to/ferrite-design scripts/new-app.sh …`.

Without the script: copy `examples/app_<template>.rs` to `src/main.rs` and
the dependency block from `docs/SCAFFOLDING.md`.

### Pick the template by the shape of the app

| Template | Shape | Start here for |
|---|---|---|
| `dashboard` | sidebar · toolbar · KPI tiles · live chart · sortable table · drawer | monitoring, analytics, system/fleet status, finance, home-lab |
| `workbench` | split: tree · tabs · virtualised document · inspector | editors, notes, IDE-likes, asset/file browsers, API clients, git clients |
| `settings` | sidebar sections · labelled fields · save/discard draft · confirm dialog | preferences windows, config tools, admin consoles |
| `explorer` | search + filter toolbar · paginated sortable table · details drawer | CRUD/admin panels, issue trackers, CRMs, database browsers, inventories |
| `console` | filtered virtual log · follow-tail · command prompt · spinner | log viewers, REPLs, job runners, serial monitors, chat transcripts |
| `wizard` | fixed window · steps · validated pages · progress · done | installers, onboarding, setup and export flows |
| `minimal` | title bar · empty body · status bar | anything else; utilities with one screen |

Run any of them first: `cargo run --example app_<template>`
(`FERRITE_SCHEME=harbor` to see another scheme).

### Recipes for app types without a template

Combine templates; every piece below is a component in the prelude.

- **Chat / messaging** — `workbench` shell: `sidebar` of channels (counts as
  meta) → a `virtual_list` of messages (`avatar` + name + body rows) → a
  `TextInput` composer pinned at the bottom (Enter = `InputEvent::Submit`).
  New messages: `ping` the channel's count; no animation per message.
- **File manager** — `explorer`'s toolbar + `breadcrumb` path + `split`
  with a `tree` of folders and a `table` of entries; `context_menu` on rows;
  `dialog` for delete; `toast` with an Undo action.
- **Media / gallery** — a wrapping grid of `dither(Picture)` tiles
  (`dither::Picture::new(w, h, levels)`), `develop` as each arrives,
  `drawer` for metadata (`property_list`), `segmented` for grid density.
- **Music / media player** — `sidebar` library, `table` tracklist,
  a bottom bar with `Button` icons (`Icon::Play/Pause/Stop`), a `slider`
  for position and volume, `meter` as a VU meter (live: no `.roll`).
- **API client (Postman-like)** — `workbench`: `tree` of requests, `tabs`
  of open requests, `select` for the method + `TextInput` for the URL in a
  `toolbar`, `accordion` for headers/params, `property_list` for the
  response headers, a read-only `virtual_list` body, `stat` for timing.
- **System monitor** — `dashboard` with `meter`s per core, `line_chart`
  per resource, a process `table`, `heatmap` of load by hour.
- **Retro / terminal-styled tool** (a BBS client, a hardware monitor, a
  game launcher) — `minimal` or `console`, framed with `ascii_box` instead
  of `panel`, a `banner` title, `ascii_gauge`s for readouts, `ascii_art`
  for a splash, `boot_screen` inside `power_on_in` for a POST at launch,
  and `motion::set_fps(25)` for the stepped look.
- **Kanban / tracker** — `explorer` data + columns of `panel`s with
  `list_item` cards; `steps` or `tag`s for status; `drawer` for details.

---

## 2. The rules (non-negotiable)

1. `ferrite_design::init(Appearance::Dark, cx)` **before** opening any
   window; every window from `chrome::window_options(..)`, then
   `chrome::square_corners(window)`; root wrapped in
   `window_frame().child(power_on_in("power", …))`.
2. **Colors only from `palette(cx)`** (`hsla(p.bg)`, `hsla(p.fg_dim)`, …).
   Never a hex literal in a view — it breaks every other scheme.
3. **One accent.** `p.accent` marks *the* primary action, the active/selected
   item, focus and live progress — never decoration.
4. **Type:** display face via `.display(Scale::X1, window)` for short
   UPPERCASE labels and numbers; body via `.body(text::BASE)` for anything
   read or typed. Never `.text_size()` on the display face.
5. **Symbols are `icon(Icon::X)`**, not Unicode glyphs (the pixel font
   lacks most of them and falls back badly).
6. **0px corners, no shadows, no gradients.** Depth = `sunken → bg →
   surface → raised` + 1px `line`. Texture = `dither(..)` in the places
   DESIGN_LANGUAGE §5.2 allows.
7. **Motion marks events.** Entrances animate, exits don't; live data never
   animates (use `StatusBar::left_live`, plain divs, `meter` without
   `.roll`). Use the drop-in effects; don't hand-roll timers.
8. **Frame regions with `panel("Title")`**, not bordered divs.
9. Every interactive element gets a **stable, unique `ElementId`**.
10. Every app exposes its actions in a **`CommandPalette`** (Ctrl+Shift+P).

---

## 3. API cheat sheet

```rust
use ferrite_design::prelude::*;                       // components, effects, palette, icons, traits
use gpui::{App, AppContext as _, Context, Entity, IntoElement, Render, SharedString,
           Subscription, Window, div, px, size};
```

### App skeleton

(Compiled as a doctest in `src/prelude.rs`, so it can't drift.)

```rust
struct MyApp { palette: Entity<CommandPalette>, toaster: Entity<Toaster>, _appearance: Subscription }

impl MyApp {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let palette = cx.new(|cx| {                  // in the constructor: it parks focus so shortcuts work
            let mut p = CommandPalette::new(window, cx);
            p.set_commands(vec![command("New file").group("File").icon(Icon::Plus).shortcut("Ctrl+N")
                .on_run(|window, cx| { /* … */ })], cx);
            p
        });
        Self { palette, toaster: cx.new(|_| Toaster::new()), _appearance: theme::follow_system(window) }
    }
}

impl Render for MyApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        window_frame().child(power_on_in("power", div()
            .flex().flex_col().size_full().bg(hsla(p.bg)).text_color(hsla(p.fg)).body(text::BASE)
            .on_action(cx.listener(|this, _: &TogglePalette, window, cx| this.palette.update(cx, |p, cx| p.toggle(window, cx))))
            .child(self.palette.clone()).child(self.toaster.clone())
            .child(title_bar("My App"))
            .child(div().flex().flex_row().flex_1().min_h_0() /* sidebar + content */)
            .child(status_bar().left("READY").right_live(format!("{}FPS", motion::fps())))))
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        ferrite_design::init(Appearance::Dark, cx);
        theme::apply_env(cx);                 // FERRITE_SCHEME / _APPEARANCE / _FPS dev overrides
        // theme::set_scheme(schemes::by_key("harbor").unwrap(), cx);   // or pick a scheme in code
        cx.bind_keys([gpui::KeyBinding::new("ctrl-shift-p", TogglePalette, None)]);
        let options = chrome::window_options("My App", size(px(1200.), px(800.)), cx);
        cx.open_window(options, |window, cx| {
            chrome::square_corners(window);
            chrome::power_off_on_close(window, cx);
            cx.new(|cx| MyApp::new(window, cx))
        }).unwrap();
        cx.activate(true);
    });
}
```

### Wiring state (the part agents get wrong)

| Handler signature | Components | Wire with |
|---|---|---|
| `Fn(&T, &mut Window, &mut App)` | `Button::on_click` (`&ClickEvent`), `on_select`/`on_change` of tabs, segmented, select, slider, number_input, switch, checkbox, sidebar, table, tree, pagination, steps, calendar, date_picker | `cx.listener(\|this, v: &T, window, cx\| { this.x = *v; cx.notify(); })` |
| `Fn(&mut Window, &mut App)` | `menu_item(..).on_select`, `dialog.confirm/on_close`, `drawer.on_close`, `alert.on_close`, `command(..).on_run`, `toast(..).action` | a weak handle: `let w = cx.weak_entity(); move \|_, cx\| { let _ = w.update(cx, \|this, cx\| { …; cx.notify(); }); }` |
| events from an entity | `TextInput` → `InputEvent::{Change, Submit}` | `cx.subscribe(&input, \|this, input, ev: &InputEvent, cx\| …)`; store the `Subscription` |

Entities (`TextInput`, `CommandPalette`, `Toaster`) are created once in the
constructor with `cx.new(..)` and rendered with `.child(entity.clone())`.
Everything else is a builder you re-create every render.

### Components (all builders; `id` = anything `Into<ElementId>`)

Every chain below is compiled in `examples/cheatsheet.rs`.

```rust
// Controls
Button::new(id).label("Run").icon(Icon::Play).primary()|.secondary()|.ghost()|.danger().small()
    .shortcut("Ctrl+R").tooltip("…").loading(b).selected(b).disabled(b).full_width().on_click(..)
checkbox(id).label("…").checked(b).indeterminate(b).on_change(..)      // radio(id) same; switch(id) same
segmented(id).option("A").option_with_icon("B", Icon::Menu).selected(i).on_select(..)
slider(id).range(0., 100.).step(5.).value(v).format(|v| format!("{v:.0}%").into()).on_change(..)
tag("live").accent()|.danger()|.success()|.warning().outline()       meter(0.6).label("cpu").id("cpu")
kbd("Ctrl+Shift+P")   spinner(id)   cursor(id)   progress_bar(0.4, px(16.), cx)

// Forms
cx.new(|cx| TextInput::new(window, cx).placeholder("…").prompt(">").masked(true))   // .value(), .set_value(v, cx)
field(id, "Name").required().hint("…").error(Some("…")).stacked().child(control)
select(id).options(["a", "b"]).selected(Some(i)).placeholder("Choose…").width(px(220.)).on_change(..)
number_input(id).value(8.).range(1., 64.).step(1.).digits(3).decimals(0).suffix("ms").on_change(..)
date_picker(id).selected(Some(Date::new(2026, 10, 6))).range(Some(Date::today()), None).on_select(..)
calendar(id).selected(..).range(..).on_select(..)      // Date: .add_days(n) .add_months(n) .iso() .weekday()

// Overlays
dropdown_menu(id).trigger(Button::new(..)).item(menu_item("Open").icon(Icon::Folder).shortcut("Ctrl+O").on_select(..))
    .submenu(submenu("Recent").item(..)).separator().label("View").item(menu_item("Wrap").checked(b))
context_menu(id).item(..).child(area)           popover(id).title("Filters").trigger(..).content(|window, cx| …)
dialog(id).open(b).title("Delete?").description("…").danger().confirm("Delete", h).cancel("Keep").on_close(h)
drawer(id).open(b).title("Details").left().width(px(380.)).child(..).footer(Button::new(..)).on_close(h)
toaster.update(cx, |t, cx| { t.push(toast("Saved").success().message("…").action("Undo", h), cx); });

// Navigation
sidebar(id).brand("App").section("Workspace").item("key", "Label", Icon::Home)
    .item_with_meta("logs", "Logs", Icon::Terminal, "12").selected(key).collapsed(b).footer(el).on_select(..)  // &SharedString
toolbar().child(..).separator().spacer().child(..)
tabs(id).tab("Logs").tab_with_meta("Jobs", "3").selected(i).on_select(..)
breadcrumb(id).crumbs(["src", "app.rs"]).on_select(..)
pagination(id).total(20).current(i).on_change(..)          steps(id).step("Account").step("Done").current(i).on_select(..)
accordion(id).open(["General"]).single().section(accordion_section("General").meta("3").child(..))

// Data
list_item(id, "name.rs").icon(Icon::File).meta("2 KB").selected(b).on_click(..)
tree(id).nodes([tree_node("src", "src/").icon(Icon::Folder).children([tree_node("src/a.rs", "a.rs")])])
    .expanded(["src"]).selected(Some(id)).on_select(..)       // &SharedString
table(id).column(column("Name").sortable()).column(column("CPU").width(px(80.)).align_right().sortable())
    .rows(rows_of_strings).sort(Some((col, SortDir::Asc))).selected(Some(i)).on_sort(..).on_select(..)
virtual_list(id, count, move |range, window, cx| range.map(|i| row(i).into_any_element()).collect()).size_full()
scroll_area(id).flex_1().min_h_0().child(..)          split(id).initial(0.25).min(px(180.)).first(a).second(b)
property_list().row("pid", "4412").row_with("state", tag("run").accent())
stat(id, "Requests", "18.2K").delta(4.2).lower_is_better().trend(values).caption("vs last week")
avatar("Ada Lovelace").initials().size(px(32.)).presence(Presence::Online)
timeline(id).time_width(px(80.)).event(event("12:04", "Deployed").detail("…").tone(Tone::Success))
alert(id, "Disk almost full").warning().message("…").action(Button::new(..)).on_close(h)
skeleton(id).h(px(64.)).w_full()    skeleton_text(id, 3, px(10.))    empty_state("No matches", "hint", window, cx)

// Charts
line_chart(id, values).title("Latency").compare(baseline).labels(labels).format(|v| format!("{v:.0}MS")).height(px(180.))
bar_chart(id).bars([("Mon", 12.), ("Tue", 19.)]).highlight(Some(1))
sparkline(values).size(px(120.), px(20.))      heatmap(rows_0_to_1).row_labels(["Mon", "Tue"])

// Text mode (ASCII) — one framing style per screen: these, or panel()
ascii_box().title("System").double().ink(hsla(p.accent)).child(..)    ascii_rule(Some("logs")).double()
banner(id, "FERRITE").shadow()    ascii_art(picture).cols(48).ramp(&ascii::BUBBLES)    ascii_gauge(0.42).label("cpu").cells(12)
ascii_art(picture).cols(120).charset(ascii::Charset::Full).fit(ascii::Fit::Shape).contrast(1.5).invert(false).diffuse(false)
// Charset: Classic Punctuation Slashes Lines Accents Letters Digits Binary Greek Box Blocks Symbols Full (best character)
ascii_film(id, frames /* Rc<[Picture]> or Vec */).cols(64).charset(ascii::Charset::Full).fps(12)   // looping ASCII animation, splash/idle only
ascii_box().style(ascii::DOUBLE_H).shadow().draw_on(id, key)      // styles: SINGLE DOUBLE DOUBLE_H DOUBLE_V PLAIN
ascii_table().header(["name", "pid"]).row(["cargo", "9021"]).selected(Some(0))    ascii_tree().item(0, "src/").item(1, "lib.rs")
ascii_plot(values).size(48, 8).format(|v| format!("{v:.0}MS"))    ascii_bars().bar("mon", 12.).cells(24)    ascii_cal(2026, 10).today(Some(6))
marquee(id, "NOW PLAYING …").cells(32)    ascii_button(id, "Ok").primary().on_click(..)    ascii_list(id).item("Quick").selected(Some(0)).on_select(..)
ascii::table(..) ascii::tree(..) ascii::plot(..) ascii::cal(..) ascii::frame(lines, ascii::PLAIN, Some("t"))   // the same, as plain strings
spinner(id).frames(ascii::spinners::DOTS)    mark()    ascii::gauge(0.4, 10)  ascii::box_edge(ascii::SINGLE, 20, true, Some("t"))

// Framing, type, color
panel("Title").meta("12 items").flex_1().child(..)    rule(Some("section"), window, cx)
div().display(Scale::X1, window).text_color(hsla(p.fg_dim)).child("LABEL")
div().body(text::SM).text_color(hsla(p.fg)).child("body text")
icon(Icon::Search).fit(px(16.)).color(hsla(p.accent))
dither(dither::flat(dither::level::LIGHT)).ink(hsla(p.line)).size_full()

// Motion (plays on first render and whenever `key` changes)
decrypt(id, text)  typewriter(id, text)  count_up(id, value, |v| format!("{v:.0}"))  afterglow(id, text)
shake(id, key, el)  tear(id, key, || el)  flash(id, key, el)  ping(id, key, marker)  scan(id, key, el)
unroll_in(id, key, el)  wipe_in(id, key, el).from_right()  interlace_in(id, key, el)  dissolve(id, key, el)
develop(id, key, dither(picture))  cascade_in(id, key).flex().flex_col().children(items)  power_on_in(id, root)
boot_screen("boot", root).title("Console")   // inside power_on_in: a POST at launch, any key skips (retro apps)

// Schemes and appearance
theme::set_scheme(&SCHEMES[i], cx)   schemes::by_key("mono")   theme::set_appearance(Appearance::Light, window, cx)
motion::set_fps(25)                  // the classic stepped look; default 240
theme::set_density(Density::Compact, cx)   // row heights: Compact 24 · Cozy 28 (default) · Roomy 36; theme::row_height(cx) for your own rows
```

Icons: `Plus Minus Close Check Play Stop Pause Refresh Search Up Down ChevronUp
ChevronDown ChevronLeft ChevronRight Menu More Copy Trash File Folder Warning
Info Dot Sliders User Calendar Home Bell Chart Terminal Lock`.
Schemes: `ferrite mono graphite slate concrete harbor cyanotype phosphor verdigris bruise`.

---

## 4. Gotchas that cost time

- **Text in a flex row overflows instead of wrapping/ellipsizing** → the
  flexible child needs `.flex_1().min_w_0()` (and `.min_h_0()` vertically
  for scroll areas).
- **Two components with the same id in one view share state** (focus,
  animation clips, menus). Ids are unique per view; in loops use tuples
  `("row", i)`.
- **A replayed animation needs a new key**, not just a re-render; to replay
  with the same key, change the id: `decrypt(("status", n), text)`.
- **Never call back into your own entity from inside its own update**; menu
  and palette handlers get `(&mut Window, &mut App)` — go through a weak
  handle (table above).
- **Shortcuts do nothing at launch** unless something in your view has
  focus — create the `CommandPalette` in the constructor (PITFALLS §30).
- **`uniform_list` directly?** Give rows `.w_full()` or clicks past the text
  miss. `virtual_list` does it for you.
- **Live counters in `status_bar().left(..)` animate forever** → `left_live`.
- **Linux builds need** `libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev
  libxcb1-dev libfontconfig-dev`; macOS needs the Metal toolchain
  (PITFALLS §42).
- **gpui versions must match ferrite-design exactly** (`=0.3.8`), and dev
  builds need `opt-level = 3` for dependencies.
- Pick a template that's close and delete; don't build a shell from scratch.

---

## 5. Verify before you call it done

```bash
cargo build && cargo clippy --all-targets && cargo test
cargo run      # look at it: every page, dark and light, one wild scheme
```

**Headless (CI, a cloud container):** Ferrite renders under Xvfb with
Mesa's software Vulkan.

```bash
apt-get install -y xvfb mesa-vulkan-drivers xdotool imagemagick \
  libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libxcb1-dev libfontconfig-dev
Xvfb :99 -screen 0 1280x960x24 &
export DISPLAY=:99 VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json
cargo run & sleep 10                         # the power-on runs first
xdotool mousemove 400 300 click 1            # drive it
import -window root shot.png                 # look at it
```

Look at the screenshot. Most real bugs in Ferrite apps (overflow, a
stretched tag, an unclickable row, a window that never opens) are only
visible that way.

---

## 6. Working on ferrite-design itself

- `cargo test` — contrast/hue tripwires for every scheme, glyph coverage,
  icon rules, and the pure logic of every component.
- `cargo run --example components` — the gallery; a new component ships
  here in the same commit (one page per family; scheme picker at the top).
- Component rules: [docs/COMPONENTS.md](docs/COMPONENTS.md#rules-for-writing-a-native-component).
- Field notes: [docs/PITFALLS.md](docs/PITFALLS.md) — read before touching
  chrome, overlays, input or animation.
- A new display-face glyph → add it to `fonts::tests`. A new icon → follow
  the art rules in `icon.rs` (2px stems, 1px margin) and add it to `ALL`.
- New scheme → `src/schemes.rs`; it must pass `tokens::tests`.
- What's next: [docs/ROADMAP.md](docs/ROADMAP.md).
