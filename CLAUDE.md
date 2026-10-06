# CLAUDE.md

Read [AGENTS.md](AGENTS.md) — it is the single source of instructions for
this repo, for building apps with Ferrite and for working on Ferrite itself.

To scaffold a new app, use the `ferrite-scaffold` skill
(`.claude/skills/ferrite-scaffold/SKILL.md`), which drives
`scripts/new-app.sh`.

Before finishing any change here: `cargo test`, `cargo clippy --all-targets`,
and look at the result running (AGENTS.md §5 has the headless recipe).
