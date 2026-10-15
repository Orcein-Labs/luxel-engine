# Original Ornith: source fidelity and articulation proof

2026-10-09. Owner Codex. Repo Luxel, character-authoring layer. Disposition:
BUILD a bounded reference-separation experiment, then evaluate articulation.
This document closes the first implementation slice; no implementation or
passing result is claimed here. Trigger: Matt requested using the actual mascot,
following the missing steps, and proving the approach before expanding tooling.

Amended 2026-10-10: Matt supplied a cleaner frontal original. It is now the
first proof source. The earlier sunglasses/hand/laptop image remains a later
contact/prop case. This source change does not constitute a passing proof.

## Primary evidence and hypothesis

Current source: `character_lab/references/ornith-front-original.webp`, copied
byte-for-byte from the user's local `header-logo-Bjk6zmTF.webp`, SHA-256
`2c9cd4d85ee65c2f7de8e78b2bf3d02c9628018df1556b09669c4065a47c4f90`.
It is a 1024x1024 RGB WebP, 48,340 bytes, fully opaque after decode. It depicts
a frontal bust with visible eyes, clear glasses, beak and hoodie; no text,
laptop or contacting hand obscures the first face/hood experiment. The gray
background is baked in. This is still flattened artwork, not layered source.

Earlier source: `character_lab/references/ornith-mascot.png` is byte-identical to
the user's local `ornith-mascot.png`, SHA-256
`27262699b4d6f97e74324a2df9258ce5d8d774091362a7a206f2c2176b71c40d`.
It is a flattened 1200x675 paletted PNG containing branding, backdrop and mascot.
A targeted search of Pictures and the Luxel avatar/reference locations found
this image and the generated pilot atlas, but no layered original. This is not
a claim that no layered original exists anywhere on the machine.

Hypothesis: preserving visible source pixels and their canvas coordinates,
authoring explicit part boundaries/overlaps, and adding concealed material on
separate layers will retain the original identity through bounded motion more
reliably than independently generated parts plus placement fitting. Static
reconstruction can prove source preservation, but cannot prove this entire
hypothesis or animation readiness.

The new reference's frontal pose is the rest pose. Preserve its actual asymmetry,
crest silhouette, eyes, glasses, beak and neckline. The original image is immutable. Keep the rejected
pilot and FIT evidence as controls; do not overwrite or relabel them as success.

## First slice: faithful source separation

Permitted implementation files: new `character_lab/reference-*` modules, shell,
styles, focused tests and reference recipe; `character_lab/build.py` for offline
bundling; a bounded branch in `character_lab/browser-smoke.mjs`; new original-
Ornith native fixture under `assets/avatars/native/`; this contract/receipt and
reference image under `character_lab/references/`; evidence under
`artifacts/original-ornith-proof-20261010/`.

Use the existing native renderer, masks and image resource. Any discovered
runtime change needs its own explicit contract amendment before editing. No
Palette/Gesso changes, existing character/source replacement, preparation schema
changes, GPU/inference job, wholesale cleanup or commit is part of this slice.

The current native texture profile accepts PNG. If required for this WebP source,
permit a derived lossless PNG encoding of its decoded pixels, with pixel equality
checked against an independent original decode. Keep the original WebP and both
file hashes; do not resize, sharpen, recolor or regenerate the visible artwork.

Build an offline workspace that loads the immutable reference and authors named
vector masks in the original coordinate frame. It needs reference/assembly
overlay, reference/assembly flicker, difference inspection, zoom, layer isolation,
boundary editing, undo and source-bound recipe save/reopen. Preserve source
positions at rest rather than guessing independent crop placement. Reuse one
embedded image resource; do not duplicate its encoded PNG per layer.

Separate the mascot silhouette from its gray background, including interior
holes. Outline semantic regions before fine separation: head/crest, left/right
eyes, glasses frame, upper/lower beak, rear hood/front lip, torso and drawstrings.
Regions may remain connected when no independent movement is earned. Maintain
an explicit occlusion/attachment list: eyes and glasses follow the head; the
frame covers the face and eye regions where appropriate; the head sits inside
the hood; drawstrings attach to the garment. Do not retain baked-in facial
features underneath independently moving copies. The illustration does not
show an authored lens treatment distinct from its underlying face; do not infer
one. A bounding-box crop retaining the gray backdrop is not a cutout. The bust
ends at the image boundary; lower body and off-canvas sleeves are unsupported.

Keep a coverage ledger per proposed movement: existing visible material,
concealed material required, whether authored, and unsupported exposure. New
concealed artwork belongs on separate editable resources. It must not silently
repaint visible reference features. If drawing assistance is used later, retain
its provenance and review the proposed additions; a generation request does not
establish matching art or complete layers.

## Gate A: appearance and persistence

Render actual native layers at the source pixel scale and compare against an
independent source decode over the declared character region. Visible interiors
must preserve RGB to within one 8-bit level per channel. Report spatial difference
maps and all differing regions, not only a whole-image average. Register any
boundary/matte exception region before evaluating; do not enlarge exclusions
after failure. Review original-sized and enlarged contours on light, dark and
saturated backgrounds for gray fringe, missing black outlines, holes and seams.

Pre-evaluation boundary policy (2026-10-10): reserve a fixed band of two source
pixels around authored silhouette and region boundaries, including partially
covered pixels, for visual matte/seam review. Report its size and differences;
do not include it in the interior fidelity claim. Native output uses one image
resource with nested, disjoint visible-region masks; invisible mask definitions
are not additional artwork or concealed coverage.

Bounded renderer amendment (2026-10-10, after first visual inspection): permit
`runtime.mjs`, `renderer.mjs` and the existing runtime tests to add explicit
`maskSampling: "pixel" | "coverage"` on v2 native parts. Absence retains coverage
sampling. Pixel masks request SVG crisp-edge clipping for source-pixel partitions;
this controls mask coverage only, not texture filtering, concealed artwork or
animation readiness. Save/import must preserve the choice. The first coverage-
sampled candidate had visible internal seams despite exact interiors and is
retained as a failed appearance candidate. This amendment does not expand the
fixed boundary exclusion or alter preparation receipt schemas.

Flattened edge pixels already include backdrop color; a mask cannot uniquely
recover their original foreground alpha/color. Record this as a matte limitation.
If boundary reconstruction is necessary, keep it as separate authored work and
review its contour against the reference. Do not promise universal pixel-perfect
transparent extraction. Higher-resolution repainting is outside this first proof.

Prove actual layer identity by isolated renders and visibility changes. A single
intact image under cosmetic layer labels cannot pass. The source layer must not
remain beneath the assembled character to hide omissions. Save, close/reopen,
export/import through the actual workbench and repeat comparison. Stale reference
digests, malformed masks and failed imports must preserve accepted source state.

Deliberate controls must fail the same relevant gates: translate glasses by three
source pixels; hide a visible beak piece; include a patch of gray backdrop; and
reintroduce the flattened reference as a concealed reconstruction crutch. Retain
failed-test counts/nonzero exits for machine checks and named visual findings for
subjective contour defects. Human visual review remains separate from numerical
source-fidelity results.

## Gate B: first articulation, separately earned

Only after Gate A's faithful rest assembly, close the movement implementation
contract using the coverage ledger. The first articulation is lower-beak motion
over a predeclared small range, with its interior/underpaint authored separately.
Do not treat a source with a fixed illustrated mouth as a rigged talking beak.
Review zero, both bounds, intermediate samples and a slow forward/reverse sweep
on the same contrasting backgrounds. Return to zero must reproduce Gate A.
Reject holes, double outlines, exposed backdrop, detached surfaces and an
incorrect upper/lower-beak relationship. Missing material is a named source
finding; do not silently reduce the range to obtain a pass.

This proves only that articulation. The next experiment must address the actual
failure region: head within hood, with small declared tilts. Determine whether
authored mesh/keyform deformation is required rather than assuming affine cutouts
suffice. Then test bilateral blink/eyes behind glasses and combined/intermediate
motions. The earlier mascot becomes a separate later contact/prop experiment:
the glasses-touching hand and connected sleeve must follow their actual contact,
and the keyboard hand must remain attached with correct laptop occlusion.
No current runtime support or passing result is implied for these later gates.

## Promotion and stop conditions

Promote the source-separation workflow when faithful rest rendering and one
honest articulation survive real save/reopen/export/playback. This earns an
Ornith proof, not general character creation, professional quality or 3D readiness.
Repeat with Cyan or a second distinctly shaped source before generalizing anatomy
or compatibility abstractions. Extend existing tools only where a step repeatedly
needs assistance; do not build a full paint application to preserve this process.

If rest reconstruction fails, fix masks/mattes/registration before rig polish.
If rest passes but articulation fails, classify missing art, attachment rules,
deformation capability or rendering as the cause. Use that finding to select
the next slice rather than regenerating the whole character.

## Basis

Current Luxel findings: `character-maker-rpdo-20261009.md`,
`character-topology-1-receipt-20261009.md`, actual original image and source recipes.
Live2D's material-separation procedure likewise uses source copies, reversible
layer masks and separate drawing additions for regions exposed by movement:
https://docs.live2d.com/en/cubism-editor-manual/divide-the-material/
This supports the workflow choice; it does not validate Luxel's implementation.
