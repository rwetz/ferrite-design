# Ferrite app scaffold comparison

Audited October 11, 2026. This compares the current local checkouts, rather than
claiming that every implementation detail still comes from the original author.
Git scaffold commits and Claude coauthor trailers establish the two groups.

| Group | Apps |
| --- | --- |
| Claude scaffolds | Almanac, Barometer, Darkroom, Lodestone, Terrarium, Wireless |
| Codex scaffolds | Apogee, Aurora, Folio, Gambit, Prism, Relay, Resonance, Snap |

## Findings and changes

| Area | Comparison | Change to Codex apps |
| --- | --- | --- |
| Finding settings | Claude provides a sliders button, a drawer, Ctrl+, and a palette command. Codex mostly placed theme choices only in the command palette. | All eight apps now expose the same visible settings entry points, including `--settings`. |
| Appearance | Claude offers explicit Dark, Light, and System choices. Codex offered a binary toggle. | All eight drawers expose all three modes and show the current selection. Existing palette toggles remain available. |
| Persistence | Claude saves appearance and motion settings. Apogee, Folio, and Gambit did not save theme changes; the other five saved only scheme and appearance. | All eight persist scheme, appearance, refresh rate, and row density. Existing JSON theme files remain compatible. |
| Launcher inheritance | Lodestone supplies Ferrite environment overrides. | Saved preferences load first, then the existing environment overrides apply. |
| Saving failures | Claude's settings handlers display failures. Codex's preference helper discarded write and rename errors. | Errors appear in the settings drawer and stderr. Writes use a temporary file and replace the destination only after writing succeeds. |
| Listening controls | Wireless persists its volume. Resonance reset volume and visualization mode. | Resonance now remembers both controls, without reopening or autoplaying a song. |
| Windows application icons | Both groups have custom pixel SVGs and ICOs, embedded as resource 1 with product and process names. | Preserve the existing embedding. Add the family charcoal frame to Prism and Resonance and regenerate their ICOs. |
| Control icons | Both groups use Ferrite's icon set; Relay had a text multiplication sign for closing widgets. | Relay uses `Icon::Close` with a tooltip. |
| Window chrome | Codex apps already remember window placement and use the Ferrite title bar and close animation. | Preserve these implementations. Settings buttons remain outside the caption drag area. |
| Pull request checks | Claude apps have CI on pull requests; Codex apps only checked code in main/tag release builds. | Add Windows pull request CI for formatting, Clippy, Rust tests, and release-version script tests to all eight apps. |
| Platform packaging | Claude release workflows build Windows, macOS, and Linux. Codex release workflows explicitly package Windows. | Keep the current Windows release scope. Adding other operating systems requires platform build and runtime validation beyond this local Windows audit. |
| App structure | Both groups use native GPUI, Ferrite tokens, command palettes, background service work, and app-specific content. | Keep each app's existing content and layout; do not homogenize their product screens. |

The settings UI uses two columns for scheme buttons and a numeric FPS control.
The latter correctly displays custom launcher refresh rates between 12 and 240,
rather than highlighting an unrelated preset.

## Verification boundaries

All eight apps are checked with locked dependency versions, four Cargo jobs, and
a reused build cache. Persistence tests cover replacement of an existing file
on Windows and a failed replacement that leaves the destination intact and
cleans up the temporary file.

Local Windows results: all eight build and Clippy runs pass with locked dependencies; 41 Rust
tests and 40 Python release-version tests pass; formatting and diff checks
pass. The eight new CI workflows parse successfully. CI has not run remotely.

Native inspection uses Aurora as the representative instance of the settings
implementation copied into all eight repositories. Its button and Ctrl+,
shortcut, scheme selection, light appearance, saved appearance after restart,
Escape dismissal, and two-column drawer layout were inspected live. This does not establish
native runtime coverage for every screen of all eight apps, or for macOS/Linux.

These are local source changes. Published releases have not been updated.
