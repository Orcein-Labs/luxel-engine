# ASSEMBLY-FIT-2 repair receipt

2026-10-09. Owner Codex, Luxel character authoring/rendering. Matt's 09:25
screenshots exposed two defects that passed the preceding geometric checks.
His subsequent feedback accepts the proposed Cyan proportions as more correct;
they are retained. Overall visual quality is still rejected.

## Causes and corrections

Both rendered panes used character-only SVG definition IDs. Browser fragment
references could resolve fitted iris masks against the original pane, producing
blank or clipped eyes. Each mount now has independent IDs for every texture and
mask, with matching internal references. Earlier standalone SVG captures could
not reveal this shared-document defect. Inspection now includes the full page.

Ornith's fitted paws had order 95 above the laptop at 90. Contact constraints
checked wrist positions but did not enforce visibility behind the opaque lid.
The guide now assigns paws order 85. The lid hides the left paw and overlapping
right-paw pixels; a side portion remains visible beside the sleeve. This is not
a newly authored hand pose or articulated grip. Complete head seating and the
head pivot at the hoodie are retained. Original built-in projects are preserved.

## Verification

Evidence is under `artifacts/assembly-fit-2-20261009/`.

- Four assembly tests and five existing runtime tests passed.
- Eight grouped assembly browser checks passed, including independent document
  IDs for both subjects and an actual raster comparison across 43649 solid
  laptop pixels. No differences exceeded the texture's alpha transmission
  bound when overlapping paws were hidden. Existing real
  edit/download/import and narrow-layout checks remain active.
- Reversing the ID fix separately makes the browser check fail. Reversing paw
  order separately makes the raster occlusion check fail. `mutation-gates.json`
  and the corresponding directories preserve both nonzero runs. Production
  files are restored byte-for-byte afterward and bundles rebuilt.
- The initial single-pixel occlusion probe missed the reversed paw order and
  was replaced. A strict alpha-255 surface probe then found only thirty fully
  opaque pixels: the supplied atlas is mostly alpha 253–254. The final check
  covers alpha 250–255 and allows at most `255-alpha+2` color levels for real
  background transmission and rounding. Both findings and the initial missed
  mutation remain in the evidence directory; no source pixels were edited.
- The main workbench's seventeen browser checks passed with the repaired shared
  renderer. `functional/browser-receipt.json`.
- Full two-pane Cyan screenshot and isolated fitted human/bird captures were
  visually inspected. Iris visibility is restored in the actual comparison.
  Atlas PNGs and built-in sources remain unchanged; current file hashes and
  preservation checks are in `source-manifest.json`.

All browser work used an isolated software Firefox with WebGL disabled. No GPU,
Gesso, model backend, asset-preparation schema or runtime character schema changes.

## Remaining rejection

Cyan still has mismatched illustrated face/hair features, colored cutout edges,
a collar seam and opacity-blended blink/mouth transitions. The source illustration
is more coherent than the recomposed atlas. Ornith still lacks the connected
raised sleeve and keyboard hand needed for the mascot reference. Properly hiding
the hand avoids an impossible overlap but cannot supply the missing artwork.
The next art/deformation slice must preserve source identity and coherent
occlusion across poses; passing landmark contacts cannot establish this quality.
