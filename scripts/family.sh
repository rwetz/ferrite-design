#!/usr/bin/env bash
# ╔══════════════════════════════════════╗
# ║  Ryan Wetzstein                      ║
# ║  Ferrite                             ║
# ║  2026                                ║
# ╚══════════════════════════════════════╝
#
# One change, every Ferrite app, one PR each. Never merges; uses your `gh`
# login. Apps are listed in scripts/family.txt.
#
#   scripts/family.sh list
#       Each app and the ferrite-design rev or tag it depends on.
#
#   scripts/family.sh bump [vX.Y.Z]
#       Point every app at a release tag (default: the newest v* tag):
#       `ferrite-design = { git = ..., tag = "vX.Y.Z" }`, refresh Cargo.lock,
#       open a PR. Apps already on that tag are skipped.
#
#   scripts/family.sh run --branch <name> --title <title> [--body <text>] -- <command...>
#       Run <command> at the root of a fresh clone of every app (FAMILY_APP
#       is set to the app's name, FERRITE_DESIGN to this checkout). Every app
#       the command changed gets a branch and a PR. This replaces hand
#       sweeps: README style, AGENTS.md, CI files, brand marks.
#       e.g.  scripts/family.sh run --branch ci-cache --title "ci: cache cargo" -- \
#               cp "$FERRITE_DESIGN/templates/ci.yml" .github/workflows/ci.yml
#
# Options (before the subcommand's own arguments): --apps a,b  --dry-run
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
owner="rwetz"
apps_filter=""
dry=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --apps) apps_filter=",$2,"; shift 2 ;;
    --dry-run) dry=1; shift ;;
    *) break ;;
  esac
done
cmd="${1:-}"; shift || true

apps=()
while IFS= read -r line; do
  line="${line%%#*}"; line="${line// /}"
  [[ -z "$line" ]] && continue
  [[ -n "$apps_filter" && "$apps_filter" != *",$line,"* && "$apps_filter" != *",${line#ferrite-},"* ]] && continue
  apps+=("$line")
done < "$here/scripts/family.txt"

work="$(mktemp -d "${TMPDIR:-/tmp}/ferrite-family.XXXXXX")"
trap 'rm -rf "$work"' EXIT
failed=0

clone() { gh repo clone "$owner/$1" "$work/$1" -- --depth 1 --quiet; }
pin_of() { grep -E '^ferrite-design *=' "$1/Cargo.toml" | grep -oE '(rev|tag|branch) *= *"[^"]+"' || echo "path or registry"; }
open_pr() { # app branch title body
  local app="$1" branch="$2" title="$3" body="$4"
  if [[ -n "$(gh pr list -R "$owner/$app" --head "$branch" --json url -q '.[0].url')" ]]; then
    printf '%-20s PR already open on %s\n' "$app" "$branch"; return
  fi
  if [[ $dry -eq 1 ]]; then
    printf '%-20s would open: %s\n' "$app" "$title"
    git -C "$work/$app" --no-pager diff --stat | tail -1
    return
  fi
  git -C "$work/$app" switch --quiet -c "$branch"
  git -C "$work/$app" add -A
  git -C "$work/$app" commit --quiet -m "$title"
  git -C "$work/$app" push --quiet -u origin "$branch"
  printf '%-20s %s\n' "$app" "$(cd "$work/$app" && gh pr create -R "$owner/$app" --head "$branch" --title "$title" --body "$body")"
}

case "$cmd" in
  list)
    for app in "${apps[@]}"; do
      pin="$(gh api "repos/$owner/$app/contents/Cargo.toml" -q .content | base64 --decode | grep -E '^ferrite-design *=' | grep -oE '(rev|tag|branch) *= *"[^"]+"' || echo '?')"
      printf '%-20s %s\n' "$app" "$pin"
    done
    ;;

  bump)
    git -C "$here" fetch --quiet --tags origin
    tag="${1:-$(git -C "$here" tag --list 'v*' --sort=-v:refname | head -1)}"
    [[ -z "$tag" ]] && { echo "family: no v* tag" >&2; exit 1; }
    git -C "$here" rev-parse --quiet --verify "refs/tags/$tag" >/dev/null || { echo "family: no tag $tag" >&2; exit 1; }
    for app in "${apps[@]}"; do
      if ! (
        clone "$app"
        dir="$work/$app"
        old="$(pin_of "$dir")"
        if [[ "$old" == "tag = \"$tag\"" ]]; then printf '%-20s already on %s\n' "$app" "$tag"; exit 0; fi
        # Swap whatever rev/tag/branch the ferrite-design line carries for the tag.
        sed -E -i.bak "/^ferrite-design *=/ s/(rev|tag|branch) *= *\"[^\"]+\"/tag = \"$tag\"/" "$dir/Cargo.toml" && rm "$dir/Cargo.toml.bak"
        (cd "$dir" && cargo update --quiet -p ferrite-design)
        old_rev="$(echo "$old" | grep -oE '"[^"]+"' | tr -d '"')"
        compare="https://github.com/$owner/ferrite-design/compare/${old_rev}...${tag}"
        open_pr "$app" "ferrite-design/$tag" "Move ferrite-design to $tag" \
"Moves \`ferrite-design\` from \`${old}\` to the **${tag}** release tag, so this app follows tagged releases instead of a raw commit.

- [What changed in ferrite-design](${compare})
- gpui must still match ferrite-design's exact pin (PITFALLS §1); check \`Cargo.toml\` if CI fails on a gpui mismatch.

Opened by \`scripts/family.sh bump\` in $owner/ferrite-design."
      ); then printf '%-20s FAILED\n' "$app"; failed=$((failed + 1)); fi
    done
    ;;

  run)
    branch="" title="" body=""
    while [[ $# -gt 0 && "$1" != "--" ]]; do
      case "$1" in
        --branch) branch="$2"; shift 2 ;;
        --title) title="$2"; shift 2 ;;
        --body) body="$2"; shift 2 ;;
        *) echo "family: unknown option $1" >&2; exit 1 ;;
      esac
    done
    [[ "${1:-}" == "--" ]] && shift
    [[ -z "$branch" || -z "$title" || $# -eq 0 ]] && { echo "usage: family.sh run --branch <name> --title <title> [--body <text>] -- <command...>" >&2; exit 1; }
    [[ -z "$body" ]] && body="$title

Opened by \`scripts/family.sh run\` in $owner/ferrite-design: \`$*\`"
    for app in "${apps[@]}"; do
      if ! (
        clone "$app"
        (cd "$work/$app" && FAMILY_APP="$app" FERRITE_DESIGN="$here" "$@")
        if [[ -z "$(git -C "$work/$app" status --porcelain)" ]]; then printf '%-20s no change\n' "$app"; exit 0; fi
        open_pr "$app" "$branch" "$title" "$body"
      ); then printf '%-20s FAILED\n' "$app"; failed=$((failed + 1)); fi
    done
    ;;

  *)
    sed -n '8,27p' "$0" | sed 's/^# \{0,1\}//'
    exit 1
    ;;
esac

[[ $failed -gt 0 ]] && { echo "$failed app(s) failed" >&2; exit 1; }
exit 0
