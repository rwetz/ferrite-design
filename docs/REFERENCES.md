# References

Outside projects worth revisiting for Ferrite's dither and ASCII work.
Study, don't vendor: check each license before copying code into this
Apache-2.0 crate.

## Dithering

| Project | What to take from it |
|---|---|
| [libdither](https://github.com/ImageProcessing-ElectronicPublications/libdither) (C) | Compact reference implementations: blue noise, Atkinson, Stucki and other error-diffusion kernels. Check ports against it. |
| [hitherdither](https://github.com/missionfloyd/hitherdither) (Python) | Yliluoma's ordered dithering for *multi-colour palettes*, plus cluster-dot matrices. Read if dither ever mixes amber, iron and danger instead of two tones, or for a halftone look on Paper. |
| [tyrfig/dither](https://pkg.go.dev/github.com/tyrfig/dither) (Go) | Clean kernel tables for every error-diffusion variant, next to Bayer matrices. |
| [eink_dither](https://pub.dev/packages/eink_dither) (Dart) | 13 kernels in one place: Floyd–Steinberg, JJN, Stucki, Burkes, Sierra family, Atkinson, Bayer 2/4/8, blue noise. Handy for side-by-side comparisons. |
| Christoph Peters, free blue-noise textures (momentsingraphics.de) | CC0 precomputed tiles, an alternative to our own void-and-cluster tile. Not checked against the source yet. |
| Lucas Pope, *Return of the Obra Dinn* devlog | Keeping dither stable while things move. Read before animating any non-Bayer pattern. Not checked against the source yet. |

## ASCII and Unicode art

| Project | What to take from it |
|---|---|
| [chafa](https://github.com/strk/chafa) (C, LGPL-3) | The standard for image → Unicode art. It picks the glyph whose *shape* best matches each cell instead of mapping brightness to a character ramp. The idea to borrow: rasterise CP437 from the embedded PxPlus 8×16 face and shape-match picture blocks, so pictures render as real DOS art in the display face. LGPL, so reimplement rather than copy. |
| play.core (ertdfgcvb) | A live ASCII-field playground; the aesthetic reference for animated ASCII. Not checked against the source yet. |

## Ideas parked from the 2026-10-06 review

- **Done:** blue-noise pattern, Atkinson for pictures, the showcase comparison.
- **8×8 Bayer:** 64 levels instead of 16, for big ramps (empty states, hero
  areas) where 16 levels band. Opt-in only: the 4×4 crosshatch is the
  signature.
- **Clustered-dot halftone:** ordered, dots that grow; a print feel for Paper.
- **Floyd–Steinberg:** a neutral, more accurate picture mode next to Atkinson.
- **Skipped:** JJN, Stucki, Burkes, Sierra. Indistinguishable from each other
  at UI scale; if a smooth photo mode is ever wanted, take Sierra Lite only.
- **Glyph-matched ASCII engine** (after chafa): the most distinctive next step.
