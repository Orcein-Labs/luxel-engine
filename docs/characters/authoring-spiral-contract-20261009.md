# Character authoring spiral: layered art, editable rig, playback

2026-10-09. Owner: Codex. Repository: Luxel. Layer: authoring providers and
runtime boundary. This is a bounded R&D work item, not a parity certification.

## Load-bearing questions and dispositions to close

1. Layered art: can a flattened reference supply hidden surfaces and independent
   moving parts? Separate source construction from segmentation and packaging.
2. Editable rig: can one explicit joint/weight construction mechanism survive
   Blender save/reopen/export for humanoid, bird and asymmetric branching forms?
3. Playback: does the exported deformation agree with Blender's actual CPU
   dependency-graph evaluation through Luxel's existing character evaluator?

## One permitted prototype

CPU-EDIT-ROUNDTRIP-1: one headless Blender script constructs three small numerical
controls from explicit bone graphs, saves editable `.blend` files, reopens each,
exports GLB, and samples neutral, intermediate and extreme animation times.
A small standalone Rust executable calls the real `decode_character_glb` and
`evaluate` entry points. An independent comparison checks every posed vertex
with one-to-one correspondence allowing export reordering and records maximum
position error, vertex counts, movement and editable-state evidence.

The controls intentionally have explicit hand-assigned weights. They test
transport, editability, branching and CPU deformation, not automatic anatomy
inference, acceptable joint topology, realistic skin or attractive art.
Use Blender's ordinary linear blend modifier; preserve-volume/dual quaternion,
constraints, morphs and procedural deformation remain outside this profile.

Permitted files: this contract; `docs/characters/authoring-spiral-20261009.md`;
`artifacts/character-authoring-spiral-20261009/**` only. No product source,
existing tests, receipt schemas, frontend profiles or deployment changes.
Artifacts are experiments, never preparation or animation-readiness receipts.

## Execution and acceptance

Use isolated headless Blender, factory startup, at most two CPU threads; never
render or launch a frontend. Claude retains the GPU. Rust builds are serial,
offline and one job. Retain commands, output, versions, hashes and failures.

Pass requires all three editable files reopen with expected bones, weights,
armature modifier and named action, all three GLBs decode without changing
Luxel, matching vertex counts and maximum component error at most 1e-5 Blender
units at five sampled times over one second, plus nonzero motion. An additional
out-of-grid fractional sample checks interpolation without assuming Blender
constraint evaluation survives export. Omit animation in a negative comparison
and require the numerical gate to fail. Any unsupported export feature or
error is a finding; preserve it rather than silently dropping deformation.

The written disposition must separately address all three directions and list
the next bounded product contracts. Layered source creation, original CMO3/MOC3,
visual playback, native shading and professional quality remain unpassed.

## Architecture candidates

Evaluate a provider-backed design: Luxel records part identity, intended motion,
anatomy-specific mappings and evidence; Blender authors editable 3D and exports
supported runtime data; Cubism authors genuine Live2D separately. Do not build
a universal anatomy DSL or universal auto-rigger before these cases justify it.
Agents/MCP operate authoring tools; deterministic engine code executes playback.
