# Character authoring MVP receipt

2026-10-09. Owner Codex. Repository Luxel, character authoring layer.
Contract: `character-workbench-mvp-contract.md`. Three passes completed in
sequence; functional MVP gate passed. No GPU scene render, Gesso mutation,
Cyan deployment or interactive Blender launch was used.

## Delivered behavior

Open `character_lab/index.html` directly in a browser. It bundles the actual
shared runtime, renderer, interface and original source projects, with no
server or network dependency. Female Cyan has 34 vector layers; hacker bird
has 35, including its laptop and visible typing paws. The art is an original
control study, not extracted from the supplied flattened mascot image.

Both subjects support editable source points, pivots, draw order, visibility,
color, node parenting and separate layer/rig control gains. Preview controls
include independent blinks, gaze, mouth/beak, head/body and secondary motion.
The bird additionally has authored typing. Play/pause uses elapsed animation
frame time; explicit scrubbing evaluates stateless clips. Undo keeps twenty
source edits. Export/import retains the complete source project and clips;
pose-slider preview values are transient. Export work before reloading.

`luxel.layered-character/v1` is a new bounded native vector reference format.
Hierarchical affine transforms, signed control-point keyforms, moving masks
and declared scalar tracks execute in actual code. Project data contains
structured vector commands, never executable SVG markup. Unsupported schemas,
references, cycles, ranges and malformed inputs fail before live replacement.
The runtime math does not require humanoid anatomy names; these two authored
subjects establish bounded examples, not universal anatomical generality.

The Blender provider separately accepts `editable_blend=...`. It saves and
reopens editable source before exporting GLB, isolates its cache, runs with
two CPU threads and disabled previews/audio, and rejects existing destinations.
Generation must finish before exclusive publication. Ordinary publication
errors roll back outputs published by that invocation; this is not a
crash-atomic multi-file transaction. Actual reviewable examples are
`artifacts/character-workbench-mvp-20261009/editable-control.blend` and `.glb`.

## Evidence and observed failures

Evidence root: `artifacts/character-workbench-mvp-20261009/`.

| Gate | Actual result | Receipt |
| --- | --- | --- |
| Existing provider integration suite | 3 passed, no skips; actual Blender, independent saved-source reopen and Rust asset authority | `edit-1-restored.log`, `edit-1-restored.gate.json` |
| Numerical/boundary suite | 4 passed; independent analytical transform/deformation oracle, serialized controls, clip math and rejection cases | `native-runtime-final.log`, `native-runtime-final.gate.json` |
| Real offline browser | 11 checks passed, Firefox 157.0.1; software rendering, WebGL disabled, isolated temporary profile/downloads | `browser-receipt.json`, `browser.log` |
| Fault injection: omit Blender save | One test failed, exit 1; exact source restored and suite passed | `edit-1-save-mutation.log`, `.gate.json` |
| Fault injection: disable pivot rotation | One analytical test failed, exit 1; exact source restored and suite passed | `native-runtime-mutation.log`, `.gate.json` |

Run the provider suite with `python -m unittest discover -s tests -p
test_rigging_provider.py -v`, numerical suite with
`node character_lab/runtime.test.mjs`, and browser suite with
`node character_lab/browser-smoke.mjs`, from the repository root. The provider
requires the existing Blender executable, Rust authority binary and external
negative-control GLB. The browser requires Firefox on PATH. The browser script
uses concrete readiness, DOM and download events, with timeouts bounding failure.

Browser checks exercised visible posing, actual pointer dragging, undo, elapsed
play/pause, a fresh downloaded JSON file, independent layer/rig gain persistence,
invalid import preservation, import roundtrip, bird custom controls, narrow
layout and absence of browser page errors. Screenshots of desktop human, bird
and 360px layout were inspected. Numerical checks alone are not visual sign-off.

Failures remain recorded rather than hidden: process-sandbox Firefox startup
was terminated; successful browser checks ran outside that sandbox with the
same isolated software settings. An early numerical assertion incorrectly
demanded bit equality for floating-point loop times and was changed to the
existing numerical tolerance without changing production math. Browser review
found a blank control label and occluded bird gesture; later checks found
control selection carried across layer changes. Selection now resets to the
new context. A playback timeout exposed `Negative playback time`: the first
frame timestamp could precede the Play click. Playback now anchors to its
first frame. The final browser receipt is green after these repairs; earlier
failed browser receipts are retained as `browser-attempt-*`.

`pose-cost.json` records a shared-machine observation after 100 warmups and
1000 samples per subject: median CPU pose evaluation about 0.230ms human and
0.161ms bird. It excludes DOM updates, painting and GPU work and establishes
neither frame rate nor a performance improvement.

## Dispositions and remaining boundary

EDIT-1 BUILD complete: editable Blender delivery and failure behavior.
ART-RIG-2D-1 BUILD complete: bounded native layered source and pose execution.
PLAY-2D-1 BUILD complete: actual editable offline workbench and persisted source.
This is the stopping gate for this spiral, with source and evidence reviewable.

This gate establishes functional authoring/playback, not a professional avatar,
Cubism/Live2D export, Vulkan game character playback or Witcher 3/Elden Ring
parity. Raster layers/textures, textured mesh deformation, perspective head
turns, cloth, live speech/activity, retargeting and frontend wiring remain open.
The native vector workbench and Blender 3D control are separate supported paths;
no false conversion between them is implied. Existing asset-preparation receipt
schemas and the pending explicit static-extraction import decision were not
reinterpreted or implemented during these passes.

The next bounded question is visual deformation quality: do independent and
combined poses preserve outlines, concealed surfaces, eye/mouth closures and
prop contacts throughout the intended range? Define a reproducible pose/contact
review set before expanding the art pipeline. Then close separate contracts for
textured layers/mesh deformation and frontend transport. Novel anatomy still
requires an explicit authored rig and motion limits; automatic correct-first-try
anatomy inference is PARK until evidence earns it.
