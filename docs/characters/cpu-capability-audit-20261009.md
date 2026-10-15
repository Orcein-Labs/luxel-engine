# CPU capability frontier: characters and a moving Heartwood scene

2026-10-09. Lead: Codex. Repo: Luxel. Layers: Rust assets/runtime and graphics
contracts. Three bounded Luna Max read-only audits informed this note. Claude
owns the GPU; this audit makes no GPU calls or visual/performance claims.

This is the pre-implementation audit snapshot. The subsequent character and
movement builds, CPU receipts, and discovery of the local Cyan Avatar Frontend
are recorded in
[`cpu-foundation-implementation-20261009.md`](cpu-foundation-implementation-20261009.md).
The final import-boundary question below is closed as a decision in
[`static-extraction-import-decision.md`](static-extraction-import-decision.md);
its schema/behavior implementation remains separate from CHAR-CPU-1.

## Current boundary

The current scope is the October 6 `docs/roadmap.md` and the user's Heartwood
quality/playable-demo goal. Older archived arena sprint documents are useful
history, not a reason to add PvP, combat, AI, or HUD to this first scene.

| Capability | Actual state | CPU-only opportunity |
|---|---|---|
| Heartwood environment | Terrain, atmosphere, PBR, imported static assets, forest scatter and still water exist. Hero-scale trunks/roots, settlement kit, biome variety, local lights, flowing water and deep forest lighting remain work. | Author/import geometry, placement, scale, collision and route controls; validate assets and composition before a later GPU beauty review. |
| Rig preparation | Real Blender control; Rust validates bounded skeleton, weights, inverse binds, clips, sockets and LOD/collision metadata. | Preserve this working intake and use it as the starting point. |
| Native animated character | Import summaries exist; native render meshes remain static. No skeletal clip/pose evaluator or facial-morph execution path. | Build CHAR-CPU-1, the byte-to-pose oracle in the companion contract. |
| Movement | Fixed-tick grounded planar-disc physics with terrain/obstacle/slope checks. Input mapping and a follow-camera seam exist. | Test/connect input → physics → actor state → camera → replay; a primitive actor can prove the loop before final character art. |
| Gameplay | Deterministic abilities, NPC turns, objectives, terminal states and replay/checkpoint foundations. | Test and reuse existing rules; they are not yet a unified rendered 3D play loop. |
| Stylized avatar rendering | PBR/clearcoat/alpha/emissive, but no native toon/unlit shading-model selector or outline support. Morph facts are counted without a native morph evaluator. | Prepare reference assets and material/face contracts after conventional character evaluation; actual edge quality needs later GPU captures. |

## Evidence pointers

- `pipeline/rigging_provider.py`: generated three-joint toy humanoid, two LODs,
  idle/locomotion/attack clips and a weapon socket. This is a structural control,
  not arbitrary autorigging or professional art generation.
- `world_core/crates/asset_contract/src/runtime.rs`: preparation validation and
  `RigSummary` / `AnimationSummary` / `SocketSummary`. The runtime package is an
  import manifest, not an evaluated rig.
- `world_core/crates/asset_contract/src/render.rs` and
  `world_core/crates/native_graphics_contract/src/lib.rs`: static `RenderMesh`
  and `MeshPacket` representation, without skeletal or morph streams.
- `world_core/crates/native_graphics_contract/src/deformation.rs`: the product
  TetCage family is `Wind`; it is not imported skeletal animation. TetLab's
  separate synthetic LBS comparator is also not a glTF character pipeline.
- `docs/world/style-profile-contract.md`: outline and posterization remain
  unsupported style axes; requesting them is not evidence of an executor.
- `world_core/crates/reference_runtime/src/physics.rs`: grounded planar-disc
  movement; no jump/gravity/capsule-clearance simulation.
- `world_core/crates/native_graphics_contract/src/input_session.rs`: sample
  mapping, physics/camera seam and trace identity. Current unit tests do not
  exercise the full `apply_frame` path or update a rendered player instance.
- `world_core/crates/gameplay_contract/src/live_runtime.rs`: deterministic
  discrete gameplay session; currently separate from 3D movement/rendering.
- `docs/world/converge/lighting-rpdo-audit-20261009.md`: moving-camera mesh-cache
  correctness finding, which must be reproduced/fixed before a moving-preview
  claim; camera-only overrides can reuse first-view visible mesh resources.

## Dispositions and ordering

**BUILD next:** CHAR-CPU-1. One imported rig, one sampled clip, correct skinning,
socket following and animated bounds. The companion closed recipe names the
files, supported subset, independent analytic oracle, and non-goals.

**NEXT bounded movement recipe:** one route, a visible primitive player, fixed
ticks, follow camera, input trace/replay and one interaction. First test the
simulation/actor/camera coupling on CPU. A later GPU smoke must prove the actor
actually moves and entering scene geometry remains visible. Primitive movement
need not wait for full skeletal character rendering.

**PREPARE:** Heartwood asset kit and composition. Start with one giant trunk/root
cluster, a small dwelling/bridge set and a readable route. Validate dimensions,
contact/placement and playable clearance before adding village-scale density.
The existing world renderer makes an authored approximation feasible; the
reference image's full asset/lighting richness is not an already-supported preset.

**PARK until demanded by the first actor:** animation blending/root-motion
policy, IK/retargeting, face morphs and hair/cloth. Preserve humanoid-independent
rigs so creatures do not require another character engine.

**PARK as downstream profiles:** avatar export/control, film sequencing/capture,
and measured-data visualization. Shared assets, transforms, animation time,
cameras and provenance are reusable foundations. Scientific simulation needs a
validated solver/data model; visual plausibility is not that validation.

## Avatar format and high-end rendering boundaries

The upstream [Open-LLM-VTuber project](https://github.com/Open-LLM-VTuber/Open-LLM-VTuber)
currently advertises custom Live2D avatars. A 3D character or VRM export does not
by itself supply that format. [VRM 1.0](https://vrm.dev/en/vrm1/) has separate
humanoid, expression/look-at, MToon, spring-bone and constraint semantics.
Treat Live2D authoring/integration and 3D avatar export as separate adapters over
the capabilities they actually share. Do not make an Etsy avatar product the
critical path to the Sidhe game demo.

[NVIDIA's September 22 developer update](https://developer.nvidia.com/blog/whats-new-for-game-developers-dlss-5-with-3d-guided-neural-rendering-nvidia-ace-updates-and-new-rtx-kit-capabilities/)
and [DLSS 5 research page](https://research.nvidia.com/labs/adlr/DLSS5/) describe
DLSS 5 as a real-time generative appearance stage using rendered color, engine
motion vectors and temporal state, with developer art controls. This is a
plausible future optional presentation path, not a replacement for authored rigs,
geometry or simulation. Neither source establishes a ready-made Luxel plugin or
proves DLSS 5 SDK compatibility with our Linux/Julia/Lava/Vulkan path. That remains
a separate integration spike. No SDK/plugin was installed.

Continue the native Witcher-3-class quality plan. Optional higher-cost lighting
and reconstruction experiments remain in the lighting RPDO ledger, behind their
own quality, memory and timing gates. Scientific measurement channels must stay
available independently of any generative presentation mode.

## This audit's execution receipt

Existing CPU suites ran serially, offline, with one build job. Logs and exact
commands are saved under `artifacts/cpu-character-audit-20261009/`.

| Suite | Result | Wall including cold compilation | Test execution |
|---|---|---|---|
| `luxel-asset-contract --test runtime_preparation` | 10 passed, 0 failed, 0 ignored | 396.332 s | 1.29 s |
| `luxel-certification-authority --test rigging_authority` | 4 passed, 0 failed, 0 ignored | 271.979 s | 0.30 s |
| `luxel-gameplay-contract --test live_runtime` | 6 passed, 0 failed, 0 ignored | 50.438 s | 0.03 s |

All 20 passed. Known-bad GLB and forged/stale receipt controls are included.
These existing suites prove preparation/gameplay behavior, not new character
playback or visual quality. No product code or test files changed in this audit;
the roadmap's character status was corrected and the next recipe registered.
No GPU or Blender process was used. Broad Julia/physics, graphics, lint and
full-workspace gates were not run in this bounded research session.

### Static-conditioning probe

The one throwaway prototype for this spiral generates three embedded GLB controls
and invokes the actual `luxel-asset-contract prepare-render` CLI: static geometry,
the same geometry with skin/weights and a translating joint clip, and geometry
with a POSITION morph target and default morph weight 1. All three returned
`ready`, no findings, and the same undeformed base positions. The render package
has no joint, weight, clip or morph streams. This confirms that render
conditioning `Ready` is not proof of animated-character support; the default
morph displacement is also absent in this probe's result.

The initial static control was rejected because it omitted tangents while
disabling tangent generation. That failed attempt is retained as
`probe-attempt-01-static-rejected.json`. The corrected control supplies UVs and
enables tangent generation for all three cases. This is a probe repair, not a
change to Luxel or an ignored failure. Final sources, receipts, script and
summary live with the CPU logs.

Next contract question: require explicit static-extraction intent or named
unsupported-deformation findings at this import boundary, without treating a
geometry-only preparation receipt as animation readiness. Do not silently
reinterpret current receipt schemas during CHAR-CPU-1.
