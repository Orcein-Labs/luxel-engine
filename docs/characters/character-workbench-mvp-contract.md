# Character workbench MVP: sequential RPDO work items

2026-10-09. Owner Codex, Luxel authoring layer. Authorized by Matt to continue
several passes through a working MVP. Each pass closes its question before the
next begins. No GPU scene renders, Gesso changes or Cyan deployment changes.

## Pass 1 / EDIT-1: retain editable Blender source

Question: can the existing provider deliver an editable project and GLB with
honest failure behavior? BUILD optional `editable_blend` output. Both outputs
must be distinct and in one directory. Existing destinations are rejected;
the provider never silently overwrites source. Generate in a unique staging
directory with isolated cache, two CPU threads, null audio and disabled
previews. Save and reopen source before export. Publish only after exit 0 and
nonempty expected files; exclusive hard links prevent overwriting races.
On publication failure remove only files published by this invocation.
Timeout/export failures publish nothing. GLB time starts explicitly at zero.
No runtime receipt schema or existing character acceptance rule changes.

Permitted files: `pipeline/rigging_provider.py`, existing
`tests/test_rigging_provider.py`, this contract, final receipt document and
`artifacts/character-workbench-mvp-20261009/**`.
Acceptance: run the existing provider integration file through actual Blender
and Rust authority, with new editable output. Independently reopen saved source
and inspect bones, armature modifier, vertex groups, actions and socket. Run an
actual failing Blender job and confirm no destination is published. Temporarily
disable editable save and prove the corresponding test fails; restore exactly.
No test exceeds new source additions; no stubbed Blender/authority proof.

## Pass 2 / ART-RIG-2D-1: bounded native layered source

Question: what minimum editable representation supports both a female human
and bird-with-laptop without universal anatomy inference? BUILD a small native
2D reference module in `character_lab/`. It is Luxel-owned authoring/playback
machinery, separate from the Vulkan renderer and from Cubism formats.

Permit `character_lab/**`, original vector-source projects under
`assets/avatars/native/**`, documentation and artifact evidence. No dependencies,
network calls, external SVG markup or executable content in project data.
Schema `luxel.layered-character/v1` is new and explicitly unrelated to asset
preparation receipts. Projects contain canvas, stable part IDs, structured
vector paths, draw order/masks, parented pivots, named finite parameter ranges,
bounded per-part parameter transforms and control-point deformations. Semantic
bindings are asset-specific. Reject invalid graphs, paths, references and
nonfinite/oversized data before changing live state. Unknown schema/features
fail explicitly. Author complete underlying shapes rather than segmenting the
flattened PNG. This is original vector control art, not a cleanup of that PNG,
PSD/CMO3/MOC3 or professional visual sign-off.

Acceptance: actual serialized human and bird projects decode, reconstruct neutral
state, exercise independent blink, gaze, mouth/beak, head/body and secondary
motion; test mask/parent cycles, unknown controls and malformed paths. Independent
analytical transform and deformation oracles, neutral roundtrip and negative
mutation must fail when real evaluation is broken. Preserve authored bounds and
range constraints; no arbitrary anatomy-specific names in runtime math.

## Pass 3 / PLAY-2D-1: usable workbench

Question: can we edit and replay these same projects through a real user surface?
BUILD a local standalone workbench using the same module, plus a generated
self-contained copy for easy opening. No duplicate animation implementation.
Controls: choose supplied subjects; inspect/hide layers; edit selected pivot,
draw order and control gain; adjust declared parameters; explicit play/pause
and scrub; reset pose; export/import complete project JSON. Evaluate a stateless
clip from explicit seconds; UI playback must use elapsed time, not frame count.
User-requested playback alone may animate continuously. Export preserves edits;
invalid import leaves active project untouched. Imports never execute markup.

Acceptance: actual browser interaction changes rendered source, edit-save-reload
survives, invalid imports fail atomically, narrow layout works and no page errors.
Use software/headless browser rendering with GPU disabled for automation. Do not
launch Cyan, the game renderer or interactive Blender. All edits stay local.
Final MVP gate: provider tests and native module tests pass; playable workbench
opens with editable human/bird parts and saved source. Record remaining art,
deformation and integration limits separately from the passing functional gate.

## Non-goals and next gate

No photorealism, automatic arbitrary anatomy, 3D game playback, normal/tangent
skinning, cloth, retargeting, true head-yaw reconstruction, compiled Live2D,
audio/frontend integration or full production editor. No marketing parity claim.
If browser inspection cannot run, preserve the workbench but leave its gate open.
Next pass is chosen from actual findings, not expanding this MVP indefinitely.
