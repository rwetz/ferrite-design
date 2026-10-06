#!/usr/bin/env bash
# Scaffold a new Ferrite app from one of the templates.
#
#   scripts/new-app.sh <name> [template] [dir]
#
#   name      crate name, e.g. ferrite-pulse (Ferrite apps are ferrite-<thing>)
#   template  dashboard | workbench | settings | explorer | console | wizard | minimal
#             (default: minimal)
#   dir       where to create it (default: ../<name>, next to this checkout)
#
# The app depends on ferrite-design from git, pinned to the commit this
# checkout is on (override with FERRITE_REV=<rev>, or FERRITE_PATH=<path>
# to depend on a local checkout while developing both).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
name="${1:-}"
template="${2:-minimal}"
if [[ -z "$name" ]]; then
  sed -n '2,15p' "$0" | sed 's/^# \{0,1\}//'
  exit 1
fi
if [[ ! "$name" =~ ^[a-z][a-z0-9_-]*$ ]]; then
  echo "error: '$name' is not a valid crate name (lowercase, digits, - and _)" >&2
  exit 1
fi
case "$template" in
  minimal) src="$here/examples/minimal.rs" ;;
  dashboard|workbench|settings|explorer|console|wizard) src="$here/examples/app_$template.rs" ;;
  *) echo "error: unknown template '$template'" >&2; exit 1 ;;
esac
dir="${3:-$here/../$name}"
if [[ -e "$dir" ]]; then
  echo "error: $dir already exists" >&2
  exit 1
fi

if [[ -n "${FERRITE_PATH:-}" ]]; then
  dep="ferrite-design = { path = \"$FERRITE_PATH\" }"
else
  rev="${FERRITE_REV:-$(git -C "$here" rev-parse HEAD 2>/dev/null || echo main)}"
  dep="ferrite-design = { git = \"https://github.com/rwetz/ferrite-design\", rev = \"$rev\" }"
fi
# The display name: ferrite-pulse -> Pulse.
display="$(echo "${name#ferrite-}" | sed -E 's/[-_]+/ /g; s/(^| )([a-z])/\1\u\2/g')"

mkdir -p "$dir/src"
cat > "$dir/Cargo.toml" <<TOML
[package]
name = "$name"
version = "0.1.0"
edition = "2024"
rust-version = "1.88"

[dependencies]
$dep
# Must match ferrite-design exactly (PITFALLS §1).
gpui = { package = "gpui-pre", version = "=0.3.8" }
gpui_platform = { package = "gpui-pre-platform", version = "=0.3.8" }

# gpui is sluggish unoptimised; optimise dependencies even in dev builds.
[profile.dev.package."*"]
opt-level = 3
TOML

# The template, with its window title and app name swapped for this app's.
title="$(grep -o 'window_options("[^"]*"' "$src" | head -1 | sed 's/window_options("//; s/"$//')"
if [[ -n "$title" ]]; then
  sed "s/\"$title\"/\"$display\"/g" "$src" > "$dir/src/main.rs"
else
  cp "$src" "$dir/src/main.rs"
fi

cat > "$dir/.gitignore" <<'IGN'
/target
IGN

cat > "$dir/AGENTS.md" <<MD
# $display

A native desktop app on [ferrite-design](https://github.com/rwetz/ferrite-design)
(GPUI, Rust), started from the \`$template\` template.

Before changing UI code, read ferrite-design's AGENTS.md — it is the
contract for how Ferrite apps are built (components, colors, type, motion,
and the mistakes that cost time). Short version:

- \`use ferrite_design::prelude::*;\` — every component comes from there.
- Colors only through \`palette(cx)\`; no hex values in views.
- Display type via \`.display(Scale::X1, window)\`, UPPERCASE; body via \`.body(text::BASE)\`.
- Controlled components: you own the value, the handler gets the new one.
- Check: \`cargo build\`, \`cargo clippy\`, and run it.
MD
cp "$dir/AGENTS.md" "$dir/CLAUDE.md"

echo "created $dir from the $template template"
echo "  cd $dir && cargo run"
