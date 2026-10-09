# Ideas

Apps worth building in Ferrite, and the plan for growing Darkroom into a
full dither and ASCII studio. Brainstormed 2026-10-07.

The filter for every idea: **it has to be better as a native desktop app
than as a web page or a CLI.** That means one or more of: it watches local
files, processes or ports; it moves gigabytes (checkpoints, datasets,
video) that a browser tab can't hold; it lives in the tray or on a second
screen; it needs a global hotkey or native notifications; or it renders
something heavy at 240fps. And it has to *look* right in Ferrite: text
mode, dither, stepped motion and one accent are a natural fit for
instruments, logs, timelines and ML plots, which is most of what's below.

Existing family, so nothing here duplicates it: Almanac (clock), Barometer
(weather), Darkroom (dither studio), Lodestone (launcher), Terrarium
(garden), Wireless (radio).


## API apps and shared infrastructure, 2026-10-09

Development scaffolds now live beside this checkout:

| App | Integration | Current slice |
|---|---|---|
| **Gambit** (`ferrite-gambit`) | Lichess TV | Anonymous NDJSON stream, text-mode board, players and clocks |
| **Folio** (`ferrite-folio`) | Gutendex / Project Gutenberg | Search and paged plain-text reading |
| **Apogee** (`ferrite-apogee`) | NASA APOD | Daily metadata and dithered image preview; external video link |

Each scaffold has a public `rwetz/ferrite-*` repository and a v0.1.0 Windows
release discoverable by Lodestone. They pin a published ferrite-design Git
revision. These initial releases cover the slices above; the additional
features below remain queued.

Next shared priorities:

1. **Shared settings store.** Pick a scheme once and all running Ferrite apps
   follow a shared file through a watcher. This should replace Lodestone's
   launch-only environment handoff. Define explicit per-app overrides, atomic
   writes, invalid-file handling, and watcher-loop prevention before migration.
2. **Ferrite Snap.** A usable visual regression app: gallery pages × schemes,
   side-by-side baselines and candidates, with accent-colour differences.
   Stable capture sizes and deterministic motion matter before pixel comparison.
3. **Pinned panels.** Frameless, always-on-top clock, CPU and now-playing windows,
   with a way to move, unpin and close each panel.

Additional integration queue:

| Integration | Ferrite direction |
|---|---|
| Radio Browser | Wireless station search and station validation |
| NOAA space weather | Kp / aurora gauge, or an Almanac line |
| USGS earthquake feeds | Live quake list and seismic activity view |
| Open-Meteo air quality | Air quality and pollen gauges in Barometer |
| Wikipedia On this day | Almanac ticker items with source links |
| NASA APOD | Apogee date browsing and Darkroom handoff |
| Lichess | Gambit channel selection and pinned live boards |
| Gutenberg | Folio cache, bookmarks and reading-position persistence |

## At a glance

| # | App | One line | Template | Size |
|---|---|---|---|---|
| 1 | **Switchboard** | Every coding agent on this machine on one panel: who's working, who's blocked, what it costs | dashboard | M |
| 2 | **Loupe** | Review an agent's diff hunk by hunk before it touches git | workbench | M |
| 3 | **Oscilloscope** | Local training-run viewer: live loss curves, run compare, no TensorBoard | dashboard | M |
| 4 | **Fluoroscope** | Open a `.safetensors` / GGUF and see inside: tensors, quant, histograms, checkpoint diffs | explorer + workbench | M |
| 5 | **Assay** | Prompt and model eval bench: run a suite, grade it, catch regressions | explorer | M |
| 6 | **Patchbay** | MCP and LLM-API wiretap: every JSON-RPC call and request, live, replayable | console | S–M |
| 7 | **Cassette** | Agent session replay: scrub a transcript like tape, see the repo at every step | workbench | M |
| 8 | **Hopper** | Context packer: pick files, watch the token budget, ship one bundle to a model | workbench | S |
| 9 | **Strata** | A repo in geological layers: churn, age, and how much of it an agent wrote | dashboard | M |
| 10 | **Manifold** | Ports, dev servers and orphaned processes, especially the ones agents left running | explorer | S |
| 11 | **Sluice** | Dataset washer for fine-tuning JSONL: browse 1M rows, dedupe, length histograms, PII flags | explorer | M |
| 12 | **Crucible** | Local-model bench: tokens/s per quant, VRAM, and a thermal guard that pauses before the CPU cooks | dashboard | M |
| 13 | **Teletype** | Record a terminal and print it as a Ferrite-styled GIF/SVG for READMEs | minimal | S |
| 14 | **Darkroom 2** | Darkroom grown into a full dither / ASCII art studio (stills, GIFs, video) | workbench | L |

If picking one to start: **Switchboard** (daily use, nothing good exists,
and it's a dashboard template away from a first version), then **Darkroom 2**
(the flagship; see the second half of this file).

---

## 1. Switchboard: the agent control room

**Problem.** Running three or four Claude Code / Codex / Cursor agents at
once is normal now, and the bottleneck is you: which one is waiting on a
permission prompt, which one is looping on a failing test, which one just
burned $4 re-reading the same file. Terminal tabs don't tell you.

**What it does.** Tails the session transcripts agents already write
(`~/.claude/projects/**/*.jsonl` and friends) and shows one row per live
session: project, model, state (working / waiting on you / idle / done /
erroring), context-window fill, tokens and spend so far, files touched,
last tool call. A lamp lights and a native notification fires when a
session needs you. Click a row to jump to its terminal or window.

**Why native.** Filesystem watching, tray icon with a lamp, native
notifications, a global hotkey to summon it, always-on-top compact mode.

**Ferrite.** Old telephone switchboard: each session is a jack with a
lamp. `ascii_gauge` for context fill, `sparkline` for token burn,
`ticker` for the last tool call, `ping` on state change, `heatmap` of
activity by hour. Compact mode is a single row of lamps.

**First version.** Claude Code only, read-only, dashboard template.
Later: Jev (see the workspace CLAUDE.md) to flag "this session looks
stuck" as a bounded yes/no, never to act on it.

## 2. Loupe: review station for agent diffs

**Problem.** Agents produce diffs faster than people read them. IDE diff
views are built for your own edits; `git add -p` is hostile. The review is
where quality is won or lost now.

**What it does.** Watches a working tree. Groups the uncommitted change
into hunks, shows each with syntax spans and the agent's stated reason
(pulled from the transcript when one exists), and lets you accept, reject,
or comment per hunk. Comments are written to a file the agent can read back
("fix these"). Accepted hunks get staged; rejected ones get reverted only
after a confirm.

**Why native.** Watches the tree live while the agent works, keyboard-first
review at speed (`j/k`, `a`, `r`, `c`), lives beside the terminal.

**Ferrite.** Workbench template: file tree, tabbed diff, inspector with
the reason and risk notes. Rejected hunks dissolve; accepted ones stamp.

**Needs.** The Phase 1 code view (multi-line text with syntax spans).

## 3. Oscilloscope: training runs without TensorBoard

**Problem.** Small local experiments (`my-first-model`,
`scratch-transformer`, `stock-ml`) don't need W&B, and TensorBoard is a
Python server plus a browser tab that lags on long runs.

**What it does.** Point it at a run folder. It reads `tfevents`, CSV, or
JSONL metric logs as they're written and draws loss, LR, grad-norm, eval
metrics live. Overlay runs, smooth, zoom with the mouse, pin a step and
see every metric at it. Per-layer grad norms as a heatmap over time. A
"run finished / NaN appeared / loss diverged" notification.

**Why native.** Tail files cheaply, render 1M points at 240fps, notify.

**Ferrite.** Stepped line charts are already the house style; confidence
bands and smoothing residue as dither instead of translucent fills;
phosphor scheme by default, because it's a scope.

**First version.** JSONL `{"step":…, "loss":…}` plus CSV, one chart page.

## 4. Fluoroscope: see inside a model file

**Problem.** "What's actually in this 8 GB checkpoint?" means a Python
REPL. Comparing a fine-tune to its base means writing a script.

**What it does.** Opens `.safetensors`, GGUF, and PyTorch `.pt` (header
only) memory-mapped. A tree of tensors with shape, dtype, quant type,
bytes. Click one: a weight histogram, min/max/mean/std, sparsity, and the
matrix itself as a dithered heat image. Diff two checkpoints and rank
tensors by how much they moved (the fine-tune's fingerprint). For
attention weights, render heads as a grid of small dithered tiles.

**Why native.** mmap multi-gigabyte files; nothing gets uploaded.

**Ferrite.** A 4096×4096 weight matrix as a Bayer/blue-noise picture is
exactly the dither primitive's job, and it looks great.

## 5. Assay: eval bench for prompts and models

**Problem.** Changing a prompt or swapping a model silently breaks cases
that used to pass. Most people "eval" by eyeballing three examples.

**What it does.** A suite is a folder of cases (input, expectations).
Run it against one or more models/prompts, in parallel, with the Claude
API (or any provider). Grade by exact rules in code where possible, by a
bounded judge (Jev, or a model with a rubric) where not. A results table
with pass/fail per case per variant, side-by-side output diffs, cost and
latency per run, and a regression view against the last committed run.

**Why native.** Long runs in the background with progress in the tray;
results stay on disk next to the suite, versioned with git.

**Ferrite.** "Assay" is testing ore for purity. Results as a grid of
lamps; a run develops cell by cell.

## 6. Patchbay: wiretap for MCP and model APIs

**Problem.** MCP servers fail in ways you can't see: a malformed schema,
a slow tool, a tool the model never calls. Same for raw API traffic:
was the prompt cache actually hit?

**What it does.** Sits as a stdio/HTTP proxy between client and MCP
servers (and optionally as a local HTTPS proxy for model APIs). Every
message in a live console: method, tool, latency, size, errors. Click to
pretty-print, edit, and replay. Per-tool latency histograms; for model
calls, tokens in/out and cache read/write.

**Ferrite.** Console template. A patchbay of cables between client and
servers in text mode, each cable blinking on traffic.

## 7. Cassette: replay an agent session like tape

**Problem.** When an agent goes wrong, the transcript is 4,000 lines of
JSON and the repo has moved on. You can't see *when* it went wrong.

**What it does.** Loads a session transcript and the repo. A tape
timeline of every turn and tool call; scrub it and the file tree and diff
show the repo as of that moment (reconstructed from the edits). Mark the
moment it went wrong, export a minimal repro (prompt + repo state) for
Assay.

**Ferrite.** Tape transport controls, a reel that spins in stepped
frames, the timeline as a `timeline` + `sparkline` of tokens per turn.

## 8. Hopper: pack context on purpose

**Problem.** Context windows are big but not free. Pasting "the relevant
files" is guesswork about size and relevance.

**What it does.** A file tree with a token count per file and folder
(real tokenizer), check boxes, and a budget gauge that fills as you
select. Strip comments/tests, include `git diff` or a symbol outline
instead of whole files. Output: clipboard, a file, or a saved "hopper"
recipe to re-run later.

**Ferrite.** One small workbench screen. The gauge is a hopper filling
with dither grain.

## 9. Strata: a repo's geology

**Problem.** Which parts of a codebase are hot, rotting, or now mostly
written by agents? `git log` won't draw it.

**What it does.** Reads git history. A treemap of the repo sized by
lines and coloured (via dither density) by churn or age. A strata view:
each file as a column of horizontal bands, one per era of edits. Commits
with `Co-Authored-By: Claude` (and similar trailers) are counted
separately, so you see the agent-authored share by folder and over time.

**Ferrite.** Strata as stacked dither bands is the language's natural
texture; it would make a fine README screenshot.

## 10. Manifold: ports and processes

**Problem.** "What's on :3000?" Agents start dev servers and watchers
and forget them; you find out when a port is taken or the fan spins up.

**What it does.** Every listening port and the process tree behind it,
with the command line, working directory, start time, CPU and memory.
Flags processes started by an agent session (parent chain) and anything
idle-but-alive for hours. Kill with a confirm. Tray menu of ports.

**Ferrite.** Explorer template, a pipe manifold drawn in box characters
from ports to processes.

## 11. Sluice: wash a dataset

**Problem.** Fine-tuning data is a JSONL you can't open in an editor.
Duplicates, truncated rows, PII, and one 40k-token outlier sink runs.

**What it does.** Opens million-row JSONL/Parquet with a virtual list.
Filter with a small query language, token-length histogram, near-duplicate
clustering (MinHash), PII and secret flags, schema check, then export the
washed set and a report.

**Ferrite.** Data grid (Phase 1.3) plus histograms. Rejected rows wash
out as dither.

## 12. Crucible: local models, measured

**Problem.** Which quant of which model runs well *on this machine*, and
at what thermal cost? (Relevant here: GPUI builds already overheat this
CPU.)

**What it does.** Drives llama.cpp / Ollama. Runs a fixed prompt set
across models and quants and records tokens/s, time-to-first-token,
memory, and package temperature. A thermal guard pauses the bench (or any
watched process) above a threshold.

**Ferrite.** Big block-digit tokens/s, a temperature gauge that turns
danger red, Barometer's chart style.

## 13. Teletype: terminal recordings as art

**Problem.** README demo GIFs (logscope's, for one) are screen captures:
blurry, huge, in whatever font the terminal had.

**What it does.** Records a terminal session (or imports an asciicast)
and renders it in the PxPlus face with any Ferrite scheme, a CRT frame
optional, at crisp integer scale. Trim, speed up idle stretches, add
title cards. Export GIF, WebP, MP4, or animated SVG.

**Ferrite.** It *is* the text-mode layer, pointed outward. Shares the
export pipeline with Darkroom 2.

---

## 14. Darkroom 2: the dither and ASCII studio

Darkroom today: one still image, three patterns (Bayer, blue noise,
Atkinson), thirteen ASCII sets, brightness/contrast/invert, two-tone ink
from the scheme, PNG/TXT export. ~1,100 lines. The goal is a tool for
making art like the reference images: a knight dithered in one ink on
white with the background gone, a hand cut out in black against a lattice
of dots, and Game Boy / C64 / Teletext palette looks from a preset row.

### What the reference images actually need

- **Duotone ink on paper** with any two colours, not just scheme colours
  (blue knight on white).
- **Subject isolation**: background removed so it becomes clean paper.
- **Cell rendering**: each art pixel drawn as a shape (square, round dot,
  plus) with a gutter, so the result reads as a lattice (the hand).
- **Different treatment per region**: subject in error diffusion, the
  background as an ordered dot field.
- **Palette presets**: one click to B&W, RGB, CMYK, 3-bit, Game Boy,
  Teletext, Apple II, C64, ZX Spectrum, 6-bit RGB, 2-bit grey,
  Vaporwave, Hacker.

### Architecture: a short pipeline, not a filter pile

```
Source ─▶ Adjust ─▶ Mask ─▶ Dither / ASCII ─▶ Render ─▶ Composite ─▶ Export
(image,   (levels,  (subject, (algorithm,       (cell shape,  (layers,   (PNG, GIF,
 GIF,      curves,   luma,     palette,          gutter,       blend)     WebP, MP4,
 video,    blur,     painted)  strength)         scale)                   SVG, TXT,
 webcam)   sharpen)                                                       ANSI, HTML)
```

Each stage is a pure function in `studio/` with tests, as `studio.rs` is
today. The view only edits a `Recipe` (a serialisable struct of every
stage's settings). A recipe *is* a preset: save it as TOML, share it,
apply it from the CLI. One or more **layers**, each a recipe with a
mask, composite into the final print; the default is one layer.

### Algorithms (the catalogue)

Every algorithm takes a palette (2..256 colours) and a **strength**.

| Family | Algorithms |
|---|---|
| Threshold | Fixed, adaptive (local mean), random (white noise) |
| Ordered | Bayer 2×2 / 4×4 / 8×8 / 16×16, blue noise (void-and-cluster), interleaved gradient noise, clustered-dot halftone, line screen, crosshatch, custom tile (paint or import a matrix) |
| Halftone | Dot, line, ellipse, square, cross; per-channel screen angles for CMYK |
| Error diffusion | Floyd–Steinberg, False Floyd–Steinberg, Jarvis–Judice–Ninke, Stucki, Burkes, Sierra / Two-row / Lite, Atkinson, Shiau–Fan, Ostromoukhov (variable coefficients), Zhou–Fang; serpentine scan toggle |
| Path / space-filling | Riemersma (Hilbert curve), dot diffusion (Knuth) |
| Multi-colour ordered | Yliluoma 1 / 2, Knoll (Photoshop-style pattern dither) |
| Stippling | Weighted Voronoi stipple (dots that cluster with tone), line hatching / engraving |
| ASCII | Tone ramps, glyph-shape matching (chafa-style, rasterised from PxPlus CP437), braille 2×4, half/quarter blocks, sextants, edge-direction glyphs (`/ \ | _ -`), colour per cell (fg + bg) |

Error-diffusion kernels are tables, so most of the list is data, not code.
Ship them in waves (see milestones), each with a gallery tile.

### Controls ("intensity" and friends)

- **Strength**: error-diffusion coefficient scale (0 = posterise, 1 =
  classic, >1 = crunchy), or ordered-matrix amplitude.
- **Pixel size / width**: the art grid (today's `cols`), with a lock to
  the source aspect.
- **Threshold bias**, **gamma**, **levels**, **curves**, **posterise**,
  **pre-blur**, **sharpen / unsharp**, **edge boost** (adds linework
  before dithering, gives engravings bite).
- **Colour space**: dither in sRGB or linear light; match colours by RGB,
  Oklab, or CIEDE2000. Oklab as the default; linear light is the
  "correct" setting that people will toggle off for a punchier look.
- **Seed** for anything random, so an export is repeatable.

### Palettes

- The 13 presets from the reference row, plus every Ferrite scheme.
- Import Lospec palettes (`.hex`, `.gpl`, `.pal`), paste hex lists.
- Extract a palette from the image (median cut, k-means in Oklab), with
  a count slider.
- A palette editor: reorder, lock, pick ink/paper; "duotone" mode is just
  a 2-colour palette with a one-click ink swap.

### Render (how each art pixel is drawn)

- Cell shape: square, circle, diamond, plus, glyph.
- Gutter between cells, and cell size modulated by tone (halftone without
  a halftone algorithm).
- Transparent paper for PNG/WebP/SVG export, so the art drops onto
  anything.
- Background field: fill the paper with a pattern (dots, Bayer at a fixed
  level, scanlines) behind the subject. This is the hand image.

### Masks and subject isolation

- Luma key and colour key (cheap, always available).
- Paint mask with a brush, feathered by dither rather than blur.
- Automatic subject segmentation via a small local ONNX model (U²-Net /
  BiRefNet class) behind a feature flag. Offline, nothing uploaded.
- A mask chooses which layer applies where: subject → Floyd–Steinberg in
  blue, background → dot field, as in the references.

### Animation: GIFs and video

- **Import** GIF, APNG, animated WebP; video and webcam later via ffmpeg.
- **Timeline** with a frame scrubber, per-frame preview, playback at the
  source rate; trim, loop, ping-pong, fps change.
- **Temporal stability** is the hard part. Error diffusion shimmers from
  frame to frame. Offer: ordered/blue-noise (stable by nature), error
  diffusion seeded from the previous frame's result where the source
  didn't change, and a stability slider. Read the *Obra Dinn* devlog
  first (REFERENCES.md).
- **Animated effects** using Ferrite's motion vocabulary: develop-in,
  scanline reveal, dither crawl, glitch, decrypt for ASCII.
- **Export** GIF (palette-exact, no requantisation since we already
  chose the palette), APNG, WebP, MP4, sprite sheet, and ASCII film
  (`ascii_film` format and an ANSI-escape player script).

### Workflow

- **Before/after** split slider on the print.
- **Contact sheet**: one image through N algorithms or palettes in a grid,
  click one to adopt it. The fastest way to explore 40 algorithms.
- Preset row (like the reference), recipe save/load, recent recipes.
- Undo/redo over the recipe (cheap: it's a small struct).
- Batch: a folder in, the same recipe applied, a folder out.
- CLI: `ferrite-darkroom --recipe gameboy.toml in.gif -o out.gif`,
  headless, for scripts and Teletype.
- Zoom and pan the print at integer zoom, with a pixel grid at high zoom.

### Performance

- Ordered, threshold and halftone are per-pixel independent: run them in
  parallel (rayon) and they're instant at any size.
- Error diffusion is serial by nature; parallelise across rows with the
  standard wavefront (row *n* starts once row *n−1* is two pixels ahead),
  and across frames for GIFs.
- Preview at screen resolution while dragging, full resolution on
  release and on export. A stepped "developing" progress bar, never a
  spinner over a frozen print.
- Keep the GPU path (compute shaders) as a later option; gpui doesn't
  expose compute today.

### Milestones

1. **0.2: algorithms and palettes.** Pipeline refactor into `Recipe`;
   error-diffusion kernel table (FS, JJN, Stucki, Burkes, Sierra ×3,
   Atkinson, Shiau–Fan); Bayer 2–16; strength, gamma, threshold;
   multi-colour palettes with the 13 presets; Oklab matching; preset row;
   recipe TOML.
2. **0.3: render and compare.** Cell shapes, gutter, transparent paper,
   background field; before/after; contact sheet; undo.
3. **0.4: GIF.** Import, timeline, temporal stability, GIF/APNG/WebP
   export, batch, CLI.
4. **0.5: masks and layers.** Luma/colour key, paint mask, layers with
   per-layer recipes; then ONNX subject segmentation.
5. **0.6: ASCII engine.** Glyph-shape matching on PxPlus CP437, braille,
   blocks, colour ASCII, ANSI/HTML/SVG export, ASCII film.
6. **0.7: the long tail.** Halftone screens with CMYK angles, Riemersma,
   dot diffusion, Yliluoma, Knoll, Ostromoukhov, stippling, hatching,
   custom tiles, palette extraction; video and webcam via ffmpeg.
7. **1.0** when the reference images can each be reproduced from a
   bundled recipe in under a minute.

### What moves into ferrite-design

Darkroom proves things the crate should own. When an algorithm is used by
a second app (Fluoroscope's weight images, Teletype's export), lift it
from Darkroom into `ferrite_design::dither`, keeping the 4×4 Bayer as the
UI signature (REFERENCES.md "Ideas parked").
