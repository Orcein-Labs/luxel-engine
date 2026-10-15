# Cyan: art repair and Live2D delivery audit

2026-10-10. Owner: Luxel character authoring. Ornith is parked at its
[accepted rest appearance](ornith-parked-checkpoint-20261010.md). This slice
audits Cyan and closes the next source-preparation contract. It does not change
the character, renderer, frontend deployment or preparation schemas.

## Decision

Preserve the existing female Cyan design direction, with the original
illustration as the appearance reference. Keep the fitted pilot as a diagnostic
comparison because Matt preferred its proposed proportions. Repeated fitting
of independently generated atlas pieces has reached its useful limit: it cannot
author a connected shaded neck or make two faded eye drawings become one eyelid.

Use Luxel for source ownership, separation, coverage planning, pose review and
evidence. For the first actual Live2D delivery, use a Cubism authoring/export
path. A native textured-mesh implementation remains valuable Luxel work, but
is a separate capability; building a MOC3 compiler is not a prerequisite for
delivering this avatar through the existing frontend.

The broader Witcher 3/Elden Ring and cinematic quality targets remain in place.
An attractive bust avatar will establish a narrow character workflow, not game
or film graphics parity.

## What is still wrong with Cyan

| Finding | Evidence and implication | Required repair |
| --- | --- | --- |
| Neck/collar reads assembled | The neck bridge stretches a 72×22 source strip into a 58×45 rectangle. Fitting conceals edges and closes measured gaps, but the inspected result still changes shading abruptly at the chin/collar. | One consistently painted neck/chin/collar transition, concealed overlap coverage and a bounded head-to-body attachment. |
| Cutout edges contain colored debris | Neutral and posed captures show blue/red/pale fragments around hair and clothing. The atlas is a generated image, not clean authored layers. | Inspect and clean each silhouette on dark, light, cyan and checkerboard backgrounds; retain intentional hair detail. |
| Partial blinks become ghosted eyes | Open eye, white and iris fade out while a separate closed-eye drawing fades in. The inspected midpoint has competing contours. | Separate eyelids, lashes, sclera and irises; deform/clip one coherent eye aperture through closure. |
| Speech produces competing mouth shapes | The closed and open mouth drawings blend by opacity, with different rectangles and contours. | Authored lips, skin and mouth interior; reviewed open/form keyforms and their combinations. |
| Motion is a limited puppet | Current native source has 17 parts, 15 raster layers, a 12-second demo, hierarchical affine motion and no textured mesh keyforms. The runtime explicitly rejects raster mesh deformation. | Cubism ArtMeshes/deformers for this provider; a distinct Luxel mesh capability if native playback is developed. |
| Source is still a design study | The female reference is flattened, and none of its pixels are fully opaque. Hair crosses the face; lower clothing is cropped. | Preserve the reference, establish intended material opacity and framing, and build editable layered production art. Do not globally force alpha to 255. |

The reference's alpha histogram is 481,710 transparent pixels, 1,091,154
partially transparent pixels and zero opaque pixels, range 0–254. The atlas has
591,514 transparent, 980,491 partial and only 511 opaque pixels. These are
measurements, not evidence that every partially transparent pixel is wrong.
The visible glow/matte contamination and uncertain material opacity require
review; alpha-range statistics alone do not define a valid cleanup operation.

## Evidence refreshed in this audit

Evidence root: [`artifacts/cyan-live2d-audit-20261010/`](../../artifacts/cyan-live2d-audit-20261010/).

- `source-audit.json` records current source hashes, alpha measurements, native
  capabilities, historical capture hashes and the eighteen preserved Ornith
  files. Cyan's native source and guide still match the FIT-3 manifest.
- The actual `assembly-cli.mjs fit` entry point produced
  `cyan-current-fitted.json` and `fit-report.json`. The exported character is
  semantically identical to the FIT-3 fixture behind the inspected captures.
  Declared geometry passes; visual acceptance remains false.
- Inspected the original female reference and the fitted neutral and midpoint
  captures. Capture hashes match the historical manifest. These are retained
  renders, not new rendering or newly passed visual tests.
- The actual local `Live2dModel('mao_pro')` payload entry point loaded again.
  All twenty referenced sample files resolve and match the prior preflight's
  hashes; emotion indexes and the idle group are valid. Details:
  `consumer-preflight.json`.

No new renderer/browser suite, MOC3 evaluation, TTS call or GPU workload ran.
The fresh fit and consumer probes are entry-point checks, not art approval.
No editable PSD/CMO3 or Cyan runtime bundle was found under Cyan's Luxel asset
root; the vendored frontend has runtime samples but no PSD/CMO3 sources in its
tree. Editor availability remains unconfirmed. This is a bounded search, not a
claim about every file on the machine.

## Remaining route to a Live2D avatar

| Stage | Deliverable | Proof required before advancing |
| --- | --- | --- |
| 1. Faithful layered rest art | Editable source and import PSD, explicit layers/ownership, clean silhouette, concealed face/neck/eye/mouth/hair material | Reassemble beside the reference; inspect at normal and enlarged size on four backgrounds; approved rest appearance and a coverage ledger. |
| 2. Editable Cubism rig | Saved `.cmo3`, ArtMeshes/deformers and documented parameter defaults/ranges | Open, close and reopen the actual project; inspect endpoints, intermediate poses and combinations, including blink with gaze, mouth with expression, and head with neck/collar. |
| 3. Runtime package | `.moc3`, `.model3.json`, textures and selected expression, motion and physics files | Export through a confirmed compatible Editor; resolve and hash every reference; load and evaluate the actual MOC3 with the pinned runtime. |
| 4. Cyan frontend delivery | New model registration and character selection with preserved existing configuration | Actual frontend display, speech-driven mouth, bilateral and independent blink, expressions, idle transitions, interruption/reset and reload/persistence. |
| 5. Luxel-native support | Explicit provider/runtime implementation and its own scene integration | Same asset evaluated and rendered through Luxel; frontend success earns no native-render readiness. |

The real export step produces compiled model data, settings and textures; native
Luxel JSON cannot simply be renamed to `.moc3` or `.model3.json`.
[Cubism embedded export](https://docs.live2d.com/en/cubism-editor-manual/export-moc3-motion3-files/)
documents those outputs. Source separation must include newly visible material,
with additions kept editable; see
[Live2D material separation](https://docs.live2d.com/en/cubism-editor-manual/divide-the-material/).

Proposed semantic mapping, to verify in the actual exported model:

| Luxel intent | Cubism target |
| --- | --- |
| Left/right eye openness | `ParamEyeLOpen`, `ParamEyeROpen`; Luxel's current blink amount has the opposite direction, so map open = 1 − blink. |
| Gaze | `ParamEyeBallX`, `ParamEyeBallY`, with reviewed art-specific bounds. |
| Mouth opening/form | `ParamMouthOpenY`, `ParamMouthForm`; declare the model's `LipSync` group and test that actual group. |
| Head orientation | `ParamAngleX/Y/Z` after authoring each supported axis; current `head` tilt does not prove yaw/pitch. |
| Idle/body/secondary motion | Authored body, breath and hair parameters plus reviewed motion/physics data. |

The pinned frontend includes Cubism runtime code and a fallback for
`ParamMouthOpenY`. Its existing sample declares `ParamA` as the lip-sync group.
This makes group binding an explicit integration check; the standard name alone
is insufficient proof. The sample payload class does not render the model or
drive audio. Registration uses the model dictionary and character selection
described in the [Open-LLM-VTuber guide](https://docs.llmvtuber.com/docs/user-guide/live2d/).

The documented [Cubism external API](https://docs.live2d.com/en/cubism-editor-manual/external-application-integration-api-list/)
provides parameter inspection/control and export notifications. Those functions
do not establish automated mesh authoring or an export command. Confirm the
Editor/platform and demonstrate the authoring/export route before making an
automation commitment. Blender remains useful for the separate 3D authoring
path; its existing GLB output does not satisfy the Live2D export gate.

## Next closed slice: CYAN-SOURCE-1

**Objective:** produce a faithful, editable Cyan rest assembly and determine
exactly which newly visible surfaces must be authored before blink/speech.
This is source preparation only. Existing geometry-preparation receipts and
native raster schema meanings remain unchanged.

**Inputs:** immutable female-v2 reference, SHA-256
`82da3196532fc000fdc6ce4569a8fe031c1fb260f5d5dacb3e3f1a9abd53b3b8`;
the current atlas pilot and fitted guide as comparisons; this audit's evidence.
The female design direction is retained. Its final appearance has not yet been
approved as production art.

**Permitted work:** new Cyan source recipes, source-bound masks, editable art
layers and explicitly scoped concealed-material edits; minimal generalization
of the existing reference workbench if required by a closed implementation
contract. Read any module in full before editing it. Preserve the previous
pilot and original reference. Write new evidence under a dedicated slice root.

**Sequence:**

1. Freeze the reference and define the intended framing, visible ownership and
   necessary part stack. Keep original geometry during first separation.
2. Reassemble source-owned pixels without a flattened duplicate underneath.
   Establish a fixed boundary policy; report edge differences and matte
   contamination separately rather than expanding exclusions to get a pass.
3. Resolve the visible neck/collar transition and silhouette contamination with
   localized, separately saved art edits. Save before/after references and
   inspect unchanged face/proportion landmarks. Do not replace individual
   features with unrelated generated designs.
4. Build a coverage ledger for lids, sclera, pupils, lips, mouth interior, face
   behind bangs, neck behind jaw/collar and moving hair overlaps. Name missing
   materials instead of marking them ready.
5. Produce an editable layer source and a PSD import candidate. File creation
   alone is not Cubism-import proof; retain that gate as pending until opened
   through the actual authoring route.

**Evidence gate:** same-canvas source/reassembly comparison, fixed interior and
boundary error counts, layer isolation and no duplicate source ownership,
four-background neutral review, actual export/reopen through the workbench,
and source hashes. For edited regions, compare against the explicitly revised
reference and preserve the original comparison. Human appearance review remains
separate from numerical acceptance.

**Negative controls:** displaced eye/face fragment, omitted collar material,
duplicate flattened underlay and leaked background patch must fail their
respective fidelity/ownership/coverage checks. Reuse existing tests where the
behavior is shared; any new production behavior needs its own failing control.

**Exit:** a reviewable rest assembly plus an honest coverage ledger, or a named
source/authoring limitation with retained evidence. No animation-readiness
claim, no auto-inferred anatomy, no broad character-creator redesign and no
GPU/inference/frontend deployment changes. Source preparation and fixed-pose
QA can proceed on CPU; actual Cubism and consumer visual evidence remain their
own later gates.

The full quality obligations remain in
[`avatar/live2d-avatar-quality-contract.md`](avatar/live2d-avatar-quality-contract.md).
