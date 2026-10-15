# Original Ornith: faithful rest assembly

2026-10-10. The first source-separation slice is implemented. Technical rest,
persistence and negative-control gates pass. This is an appearance-review
candidate, not an animated mascot or general character-maker promotion.
Gate B is unstarted. No GPU was used.

**Later user decision, 2026-10-10:** Matt accepted the shown rest appearance and
asked to park Ornith while work returns to Cyan. See
[`ornith-parked-checkpoint-20261010.md`](ornith-parked-checkpoint-20261010.md).
Historical test reports retain their original acceptance fields; the decision
does not promote animation readiness or remove the source limits below.

## Result

Open `character_lab/reference.html`. The immutable supplied frontal WebP is
preserved byte-for-byte. Its decoded RGB is encoded once as lossless PNG, with
zero conversion difference against Firefox's independent original WebP decode.
Source pixels remain at their original coordinates. Ten visible raster regions
share that resource and use 31 invisible native mask definitions (41 parts total,
maximum mask depth six). Overlapping higher regions are excluded from lower
regions; disjoint exclusions share a compound mask to bound nesting. Invalid
edits requiring more than six exclusion levels are rejected atomically.

The offline editor supports actual layer isolation, boundary point dragging and
numeric editing, undo, overlay/flicker, difference map, zoom, four backdrops,
source-bound recipe export/reopen, and complete native export. Its source and
native fixture are new files; earlier rejected illustrations remain available.
No Palette, Gesso or Cyan infrastructure was changed.

## Evidence

Evidence root: `artifacts/original-ornith-proof-20261010/`.

- Four focused separation tests and five existing runtime tests pass. The
  ownership oracle independently evaluates native mask chains against the
  recipe's topmost region, with no double ownership at sampled source points.
- Eleven actual Firefox software-browser checks pass, including ten isolated
  renders, real pointer editing/undo, invalid-import preservation, exported
  recipe reopening, and the real native workbench import.
- At 1024x1024, all 554,078 declared interior pixels match the original exactly:
  maximum RGB difference zero, changed interior count zero, outside leakage
  zero. Internal boundary changes are also zero.
- The fixed two-source-pixel boundary band contains 61,615 pixels. There are
  6,903 changed/partially covered boundary pixels, at the outer cutout. They
  remain reported and excluded from the interior claim. This is not universal
  pixel-perfect transparent extraction from an opaque image.
- Native export imported through the actual existing workbench produces the
  identical complete RGBA render, including the boundary behavior. Its playback
  control is disabled because the source has no authored clip or parameters.
- Production mutations prove the tests detect missing region exclusions and
  missing mask-sampling validation: each yields one failed test and exit 1.
  Restored production runs pass. Logs retain explicit failed-test counts.

The same browser comparison rejects these deliberate controls:

| Control | Observed rejection |
| --- | --- |
| Glasses image translated three source pixels | 3,602 changed interior pixels |
| Visible beak hidden | 14,554 changed interior pixels |
| 30x30 gray backdrop patch | 900 leaked pixels and invalid artwork structure |
| Full flattened source concealed underneath | Invalid artwork structure despite matching pixels |
| Coverage sampling restored on partition masks | 7,168 internal boundary changes |

Detailed per-region results, difference map, native export, edited recipe,
isolated parts and dark/light/cyan/checkerboard captures are in `browser/`.
`failed-mask-chain/` retains the first browser error about excessive SVG mask
nesting. `failed-coverage-seams/` retains the subsequent visually rejected
antialiasing seams. Neither is promoted as an appearance success.
The later comparison-control check also exposed a test-driver variable-scope
error and a preview-width change when zoom scrollbars appeared. Their failed
receipts are retained as `failed-browser-harness-scope.json` and
`failed-zoom-gutter.json`. Scoped driver expressions and a stable scrollbar
gutter correct those findings; the final eleven-check browser run passes.

## Visual review and remaining source limits

Inspected the side-by-side light and cyan workspaces, enlarged contours, full-resolution neutral
and imported renders, and isolated head, glasses and beak regions. The fresh
assembly retains the actual source proportions, pupils, glasses and illustrated
head/hood contact. Thin internal seams seen in the coverage-sampled candidate
are absent with explicit pixel mask sampling. Numerical checks do not record
Matt's visual acceptance; it remains false in the report.

The outer contour is an authored polygon approximation of source ink. Hard mask
sampling can alias at some sizes; the original opaque gray edge contamination
cannot be uniquely unmixed. Inspect enlarged boundaries before production use.
Current masks divide visible source ownership, not complete independently
drawable anatomy. The head/hood contact includes painted edges and shadows that
will need separation/coverage refinement before moving. The beak shadow region
contains cast shadow and nearby face: it is explicitly not a lower jaw. No
distinct movable lower-jaw art is present in the reference.

| Region relation | Rest representation | Requirement before independent movement |
| --- | --- | --- |
| Pupils and glasses follow head | Fixed original coordinates; separate visible masks | Head group, face underpaint, eye-window/eyelid coverage |
| Glasses cover face | Frame excluded from underlying source regions; no inferred lens art | Boundary cleanup and concealed face if frame moves |
| Beak sits on face | Painted beak plus named cast-shadow region | Actual lower jaw, mouth interior, concealed face; shadow response |
| Head sits inside hoodie | Original illustrated contact and separate collar/body regions | Concealed head/hood material and authored attachment deformation |
| Drawstrings attach to garment | Separate visible strings at source positions | Covered attachment surfaces before sway |
| Bust ends at canvas boundary | Original crop retained | New lower-body/off-canvas art for wider framing |

All four proposed movements in the coverage ledger remain unsupported pending
authored coverage. The next closed slice is real lower-beak articulation with
separate concealed artwork and a predeclared motion range, followed by head/hood
contact and blink. It must retain this zero-pose evidence and pass intermediate
poses. Static source fidelity earns none of those capabilities by itself.
