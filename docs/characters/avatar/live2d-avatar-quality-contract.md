# Live2D avatar quality contract

2026-10-09. Status: quality target defined; authoring/export/runtime evidence pending.

## Purpose and boundary

Produce one original, professionally authored Live2D avatar as a 2D art,
deformer, and interaction stress test before Heartwood. The intended functional
consumer is the actual Open-LLM-VTuber Web/Electron frontend. Passing there proves
that provider path only. It does not prove Luxel-native rendering, 3D skeletal
skinning, morph evaluation, or Heartwood character quality.

This explores another product surface while Luxel gains animation and rigging
for games, cinematography and scenes. Etsy, sales and distribution are not
acceptance requirements. Witcher 3 and Elden Ring remain distinct broader
quality references; this avatar does not establish parity with either game.
Human demo characters default to women at the user's request. Mascots and
nonhuman characters remain supported subjects across different art styles.

Live2D is a distinct format/provider. Do not substitute a flattened PNG, SVG,
procedural puppet, unlicensed sample model, or a VRM/GLB and call it a Live2D
avatar. The final identity and visual direction remain for the user to choose.

## Current source and blocked state

The current female design study is
[`cyan-source-art-female-v2.png`](../../../assets/avatars/cyan/source/cyan-source-art-female-v2.png),
edited from v1 with the built-in image-generation tool using
[`cyan-source-art-female-v2.prompt.txt`](cyan-source-art-female-v2.prompt.txt).
Image SHA-256: `82da3196532fc000fdc6ce4569a8fe031c1fb260f5d5dacb3e3f1a9abd53b3b8`.
It remains a flattened 1024 × 1536 reference, not a Luxel render or Live2D model.
Alpha still ranges from 0 to 254 with no opaque pixels, and the requested clean
silhouette/clear central fringe has not been achieved. Keep its cleanup and
layer separation open rather than promoting a generated image to rig-ready art.

The provisional Cyan reference is
[`cyan-source-art-v1.png`](../../../assets/avatars/cyan/source/cyan-source-art-v1.png),
a 1024 × 1536 flattened RGBA image generated with the built-in image-generation
tool from [`cyan-source-art-v1.prompt.txt`](cyan-source-art-v1.prompt.txt).
Image SHA-256: `f5b8d3924f5e8f069881abe5032d7a16a5ab22014dec3df33e0a8af50824882c`.
Prompt SHA-256: `70133bc8eb3d6654d6759c1c776d6b3f9066b4222816333943776919d807a118`.
The prompt and image are provenance/reference, not an approved final identity,
layered source, rig, or professional-quality sign-off.

Source-art QA is currently failed: the generated bangs cross the eye/face and
the lower clothing is cropped, contrary to the prompt's clear-eye and head-to-
waist framing requirements. The PNG alpha ranges from 0 to 254 with no fully
opaque pixels; approximately 457,000 pixels are fully transparent and 1.116
million are partially transparent. Treat it as a visual reference requiring
user-directed cleanup or replacement, not as a clean silhouette or layer-ready
portrait.

The initial bounded audit found no Editor or model assets in Luxel, Cyan
Harness, or the user's local pictures folder. The user then supplied a separate
frontend root, the Cyan avatar frontend checkout (tests read it from
`CYAN_FRONTEND_ROOT`). It contains the pinned
Open-LLM-VTuber v1.2.1 checkout, frontend commit
`06a659b114fff788cf0daaa86e484576db4975bf`, Cubism runtime libraries, and existing
`mao_pro` and `shizuku` sample exports. Its current model dictionary registers
`mao_pro`; the local Astra/Bea profile selects it. This discovery supersedes
the earlier absence of a known local frontend, not the absence of an original
editable rig. No frontend files, profiles or deployment settings were changed.

The actual local `Live2dModel` Python entry point loaded `mao_pro` on CPU. All
20 settings-file dependencies resolved and were hashed; emotion indexes and the
idle group were valid. See
[`local-live2d-consumer-preflight.json`](../../../artifacts/cpu-character-implementation-20261009/local-live2d-consumer-preflight.json).
This is payload/reference evidence only: no MOC3 evaluation, frontend display,
lip-sync, animation quality or GPU run occurred. The upstream samples are
controls, not newly created avatars. No confirmed Editor or original editable
PSD/CMO3 source has been found; no installation or license acceptance occurred.

## Production inputs and layer preparation

Before rigging, retain an editable layered PSD (or another source format proven
to import correctly) with documented author and usage rights. The flattened
reference may guide that work, but cannot serve as the rig source. Preserve
layer/group names and draw order so every moving part remains traceable into the
Cubism project.

Prepare separate, complete regions for the intended motion: face/outline and
ears; eye whites, irises, pupils, lashes and brows; upper/lower lips and mouth
interior; front, side and back hair masses and individual moving locks; neck,
torso, shoulders, and any garment panels or accessories expected to sway. Add
paint beneath overlaps so head turns, gaze, blinking, and mouth motion do not
reveal transparent holes or cut edges. In particular, extend the face behind
hair, eye whites behind moving eyeballs, the neck below the moving face, and
mouth/skin fills behind the closed-mouth contour. Confirm the layer plan against
the chosen art rather than assuming the current reference has these hidden
surfaces. Cubism imports PSD layers as ArtMeshes; it does not invent the missing
illustration content. [Live2D's PSD preparation tutorial](https://docs.live2d.com/en/cubism-editor-tutorials/psd/)
and [PSD import manual](https://docs.live2d.com/en/cubism-editor-manual/psd-import/)
describe this source-material workflow.

## Rig and visual acceptance

### Character-agnostic control surface

The avatar profile must not require a human face or skeleton. An Ornith-style
hacker bird can use a beak, crest, wings and a laptop; a scientist cat can use
ears, paws, a lab coat and a computer. Keep foreground props and independently
moving body parts in separate layers with deliberate draw order and masks.
The supplied Ornith mascot is a reference for that direction, not a rig or
permission to treat the text/marketing panel as avatar artwork.

Use named semantic controls for idle, listening, speaking, thinking and typing,
mapped by each avatar to its actual parameter IDs, expressions and motion
groups. Speech may drive a beak instead of lips; secondary motion may affect
a crest, ears or coat instead of hair. Application activity controls must come
from real supplied activity state, never invented background work. Test prop
occlusion, transitions, neutral reset and simultaneous speech/gesture. A static
laptop image and a looping hand animation do not prove interactive typing.

### Deformation and presentation

The Cubism project must remain editable as `.cmo3`. Record each parameter's
stable ID, default, minimum, maximum, and intended control. At minimum, the test
matrix must exercise head X/Y/Z, body movement supported by the art, eye gaze,
left/right blinking, mouth open/form, and brows. Test each defined endpoint and
the combinations that expose overlap problems: yaw with pitch, head turn with
gaze and blink, and head/body motion with expressions and mouth movement.
Ranges are art-specific and must be agreed during authoring; do not invent
degrees or claim coverage from testing only neutral poses. Cubism stores
deformation in mesh/deformer parameter keyforms, so endpoint and combination
review must use the actual model. [Cubism parameter keys](https://docs.live2d.com/en/cubism-editor-manual/edit-parameters/)
and [deformers](https://docs.live2d.com/en/cubism-editor-manual/deformer/)
describe these controls.

The exported avatar must have a working neutral/default state and at least the
frontend emotion set selected for the character, with every emotion key mapped
to an existing expression. Verify transitions and reset-to-neutral. Verify
blinking on both eyes and each eye independently. Speech must exercise the
mouth-open/form controls using the actual frontend audio path; if the selected
frontend build does not drive lip sync, record that as a provider gap instead
of marking lip sync passed. Define hit areas and at least one corresponding
tap-motion group if click interaction is part of the target setup.

If the art includes secondary-motion parts, configure hair and any selected
clothing/accessory physics at the intended runtime rate. Test motion from rest,
after rapid reversals, and at head/body range extremes; reject visible runaway,
permanent drift, popping, self-intersection, or clipping. Record the physics
frame-rate setting and the exact viewer/frontend used. [Cubism physics](https://docs.live2d.com/en/cubism-editor-manual/physics-operation/)
is a separate exported runtime setting.

At every tested endpoint and combination, review at the intended frontend size.
Reject unintended holes, broken outlines, mirrored or missing parts, collapsed
meshes, exposed fill, or silhouette changes that do not match the art direction.
Passing requires human review of the actual moving model; file validity or a
static portrait cannot certify visual quality.

## Export, package, and provenance

Export for Cubism 5 as a self-contained runtime bundle containing `.moc3`,
`.model3.json`, every referenced texture, and all selected physics, motion, and
expression files. Resolve every path in the model settings file against the
bundle; reject missing, external, or mismatched files. Keep the `.cmo3` project
and layered art source with the editable production record. Record SHA-256 for
each delivered file, Cubism Editor edition/version, export target, model/SDK
compatibility, art author, rigging author, source-art provenance, and applicable
model/SDK usage rights. Cubism's embedded export produces `.moc3`,
`.model3.json`, and textures, with optional physics/motion data; these are runtime
files, distinct from the editable `.cmo3` project. [File types](https://docs.live2d.com/en/cubism-editor-manual/file-type-and-extension/)
and [embedded export](https://docs.live2d.com/en/cubism-editor-manual/export-moc3-motion3-files/)
are the format references.

The current Open-LLM-VTuber release notes say its frontend uses the official
Cubism Web SDK, supports Cubism 5, and drops Cubism 2. Use and record an exact
compatible frontend release/commit; the live user guide's older renderer note
is stale. Place the bundle in that frontend's `live2d-models/<id>/` location,
register the `.model3.json` path and motion/expression/hit-area mappings in its
model dictionary, and select the model in character configuration. Acceptance
requires the actual frontend to load the bundle, display it, exercise every
required expression and hit area, and drive the spoken-audio lip-sync check.
Capture the frontend version/configuration and a review recording or equivalent
runtime evidence. A Cubism Viewer open is a useful asset smoke check, but does
not replace the Open-LLM-VTuber consumer test. [Open-LLM-VTuber v1.2.0 release](https://github.com/Open-LLM-VTuber/Open-LLM-VTuber/releases/tag/v1.2.0)
and its [Live2D setup guide](https://docs.llmvtuber.com/docs/user-guide/live2d/)
define the current consumer path.

Public distribution is outside this internal demo's acceptance scope. Retain
source provenance. Record the Editor edition used; FREE has texture, ArtMesh
and parameter limits described in the [official comparison](https://www.live2d.com/en/cubism/comparison/).

## Evidence gates

| Gate | Acceptance evidence | Current state |
|---|---|---|
| Source art | Chosen direction; editable layered source; provenance recorded | Pending: only flattened design studies and prompts are present |
| Cubism authoring | Editable `.cmo3`; documented parameter ranges; reviewed deformations, expressions, blink, lip sync, and physics | Blocked: no confirmed existing Editor location or rigged model |
| Export integrity | Versioned `.moc3`/`.model3.json` bundle; all references resolve; hashes recorded | Blocked: no runtime export exists |
| Provider function | Compatible Open-LLM-VTuber build loads and exercises the actual model/config; evidence captured | Local v1.2.1 frontend and sample payload/file preflight confirmed; new avatar and visual runtime pending |
| Luxel-native render | A separate Luxel provider loads the same export and proves native composition, interaction, and motion | Unsupported today; upstream frontend success does not satisfy it |

CPU-only checks can later verify provenance hashes, package contents, JSON paths,
version declarations, and frontend configuration without Cubism rendering. They
cannot prove art quality, MOC3 deformation, expressions, physics, or runtime
lip-sync. The Cubism Editor external API can read/set parameters and report
export events, but requires user approval and is not a batch model authoring or
export command; it has not been connected in this CPU slice. [External API](https://docs.live2d.com/en/cubism-editor-manual/external-application-integration-api/)
and [API function list](https://docs.live2d.com/en/cubism-editor-manual/external-application-integration-api-list/).

Luxel-native proof is a later, independent gate requiring an explicit Live2D
provider/runtime path and render evidence. Do not convert MOC3 into a static
GLB mesh or infer Luxel support from the 3D CPU character foundation or from
Open-LLM-VTuber display.
