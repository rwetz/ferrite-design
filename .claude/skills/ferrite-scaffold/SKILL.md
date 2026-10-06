---
name: ferrite-scaffold
description: Scaffold and build a native desktop app with ferrite-design (GPUI, Rust). Use when asked to create, start or prototype a desktop app, tool, dashboard, editor, settings window, admin panel, log viewer, installer or wizard with Ferrite, or to add Ferrite UI to a GPUI app.
---

# Scaffold a Ferrite app

Ferrite is a design language + component library for GPUI desktop apps.
AGENTS.md (repo root) is the contract; this skill is the procedure.

## 1. Choose a template from the app's shape

| The app is mostly… | Template |
|---|---|
| live numbers, charts, a status table | `dashboard` |
| a tree/list of things + an open document + an inspector | `workbench` |
| labelled options grouped in sections | `settings` |
| records to search, filter, sort, page and inspect | `explorer` |
| a stream of lines and a command prompt | `console` |
| a linear sequence of steps | `wizard` |
| one small screen | `minimal` |

If none fits, pick the closest and use AGENTS.md §1 "Recipes" to adapt it.
Ask the user only if two templates fit equally and the choice changes the
layout substantially.

## 2. Create it

```bash
scripts/new-app.sh ferrite-<thing> <template> [dir]
```

- Name apps `ferrite-<thing>`, lowercase.
- Use `FERRITE_PATH=$(pwd)` when the ferrite-design rev isn't pushed yet
  (the git pin only resolves for pushed commits).
- Run `cargo run --example app_<template>` in ferrite-design first if you
  want to see what you're starting from.

## 3. Make it the user's app

1. Replace the template's stand-in data (`Telemetry`, `records()`,
   `document()`, the timer in `Console::new`, `TASKS`) with the real source.
   Keep the view a function of state; keep filtering/sorting in one method.
2. Rename views, sidebar items, palette commands and titles.
3. Delete what the app doesn't need — templates are generous on purpose.
4. Every action goes in the `CommandPalette` too.
5. Copy builder chains from AGENTS.md §3 (they're compile-checked); wire
   state with the handler table there (`cx.listener` vs a weak handle).

## 4. Hold the line on the rules

Colors only via `palette(cx)`; display type via `.display(Scale, window)`,
UPPERCASE; symbols via `icon(Icon::…)`; 0px corners; motion only for
events (live values use `*_live` / plain text); `panel()` for regions;
unique ids. Full list: AGENTS.md §2.

## 5. Verify, then report

```bash
cargo build && cargo clippy --all-targets
```

Then **run it and look**: headless boxes can use the Xvfb + lavapipe recipe
in AGENTS.md §5 to screenshot the window. Check each screen in dark and
light (`FERRITE_APPEARANCE=light`) and one other scheme
(`FERRITE_SCHEME=harbor`). Report what you built, what you verified by
looking, and anything you couldn't check (e.g. macOS/Windows chrome).
