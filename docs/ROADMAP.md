# Roadmap

Where Ferrite is, and what "an end-to-end design framework" still needs.
The bar: a person (or an agent) can go from `new-app.sh` to a shippable,
signed desktop app without leaving the framework or hand-rolling the
common parts — and the result looks and behaves like a Ferrite app on all
three platforms.

## Where it stands (October 2026)

| Layer | Status |
|---|---|
| **Language** — tokens, schemes, type, dither, ASCII, motion, chrome | ✅ Done. 10 schemes (Ferrite + 4 neutral + 5 wild), all contrast- and hue-tested. |
| **Components** — controls, forms, overlays, navigation, data, charts, layout | ✅ ~55 components, all native, including a text-mode (ASCII) layer (COMPONENTS.md). |
| **Motion** — engine + 16 drop-in effects + motion inside components | ✅ Every effect in DESIGN_LANGUAGE §6.1 is demoed on the gallery's Motion page. |
| **Templates** — dashboard, workbench, settings, explorer, console, wizard | ✅ Compiled with the crate, so they can't rot. |
| **Agent path** — AGENTS.md, scaffold script, Claude skill | ✅ |
| **Platforms** | macOS and Windows: developed on. **Linux: runs as of this release** (it opened no window before — PITFALLS §44); resize edges still unverified on a real compositor. |
| **App framework** — persistence, routing, keymaps, async data, packaging | ❌ Every app hand-rolls these today. This is the main gap. |
| **Automated visual testing** | ❌ Pure logic is unit-tested (~95 tests); pixels are checked by hand. |

## Phase 1 — close the component gaps (next)

Ordered by how many app types are blocked without them.

1. **Multi-line text editor** (`TextArea`, then a code view with syntax
   spans). Blocks notes, chat, editors, any "description" field. Build on
   `TextInput`'s `EntityInputHandler` plumbing; line layout via
   `ShapedLine` per line inside a `virtual_list`. *Done when:* IME,
   selection across lines, undo, 100k lines scroll at 60fps.
2. **Combobox** (typeahead select with fuzzy filtering) — reuse `fuzzy` and
   the palette's list; `select` stays for short fixed lists.
3. **Data grid** — `table` on `virtual_list`: frozen header, column resize
   and reorder, cell selection, 100k rows. Explorer and dashboard templates
   move to it.
4. **Menu bar** (File / Edit / View with hover-to-switch between open
   menus) — needs one shared overlay state across the bar's dropdowns.
5. **Tag input**, **time picker / date range**, **file drop zone**,
   **hover card**, **notification centre** (toast history).
6. **Docking / tabbed panes** — split + tabs with drag-to-rearrange; only
   if a real app (workbench) needs it.

Out of scope on purpose: round progress rings, colour pickers (one accent),
carousels — they fight the language or desktop conventions.

## Phase 2 — the app framework (what makes it "end to end")

1. **Settings persistence**: `ferrite_design::store` — a typed, versioned
   settings file in the platform config dir (serde), with the scheme,
   appearance and refresh rate wired in by default. The settings template
   becomes ~50 lines shorter.
2. **One keymap registry**: declare an action once (name, default keys,
   group, icon) and get the key binding, the palette command, the menu
   item's shortcut text and the tooltip `kbd` from it. Today all four are
   typed separately and drift.
3. **`Loadable<T>`**: a state type for async data (`Loading | Ready(T) |
   Empty | Failed(err)`) with a render helper that picks skeleton / content
   / `empty_state` / `alert` + retry. Removes the most common boilerplate in
   every template.
4. **Navigation state**: a tiny router (page enum + history + back/forward
   keys) so sidebar/breadcrumb/palette all drive one source of truth.
5. **Undo stack helper** for editors and forms.
6. **Packaging**: app icon generation from a pixel-art source (it's on
   brand), `cargo-packager` configs for `.app`/`.dmg`, `.msi`, AppImage,
   and code-signing notes. `new-app.sh` emits them.

## Phase 3 — quality and trust

1. **Visual regression in CI.** Proven possible in this release: Xvfb +
   Mesa lavapipe renders Ferrite on Linux, `xdotool` drives it,
   `import` captures it (PITFALLS §44). Next: a `ferrite-snap` harness that
   renders each gallery page in each scheme and diffs against committed
   PNGs. *Done when:* a palette or layout regression fails CI.
2. **CI on macOS, Windows and Linux**: build, test, clippy, the snapshot
   job on Linux.
3. **Idle-CPU budget test** (PITFALLS §17/§40): launch the gallery, sample
   CPU for 5s with nothing animating, fail above a threshold.
4. **Accessibility pass**: screen-reader walk of every component; adopt
   gpui's disabled state and focus scopes when they land (COMPONENTS
   "Known gaps").
5. **Release discipline**: semver tags, a CHANGELOG, a pinned `rev` in the
   templates' Cargo.toml; publish to crates.io (gpui-pre is published, so
   nothing blocks it but deciding the API is stable enough).

## Phase 4 — tooling

1. `cargo ferrite new <name> --template <kind>` replacing `new-app.sh`.
2. **Scheme designer**, built in Ferrite: edit a scheme with live preview of
   the gallery, contrast and hue-gap tests running as you drag, export to
   `schemes.rs`. (The generator used for this release's schemes is the
   seed.)
3. **Token export** for docs and the Nexis web side: JSON/CSS from
   `tokens.rs` + `schemes.rs` (`tokens::css` already exists).
4. **mdBook site** with the screenshots generated by the snapshot harness.

## Risks worth naming

- **gpui is pre-1.0 and pinned exactly.** Every bump can break element
  APIs; the ~50 components are now the largest surface exposed to that.
  Mitigation: bump deliberately, one commit, gallery walk-through.
- **No pixel tests yet.** Phase 3.1 is the single highest-leverage item:
  most bugs fixed in this release (headless Linux, unclickable rows, an
  overflowing row, a stretched tag) were only visible by *looking*.
- **Scope creep.** Each component is maintenance. Add one only when a
  template or a real app needs it, and ship it in the gallery the same day.
