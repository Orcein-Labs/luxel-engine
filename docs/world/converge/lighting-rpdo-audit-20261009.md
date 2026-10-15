# Lighting audit and next RPDO questions

2026-10-09. Lead: Codex. Repo/layer: Luxel native graphics. Workers:
Luna Max implementation, independent validation, and quality-frontier research.
This is a research ledger beside the closed C4 IBL comparison recipe, not a
replacement for CONVERGE-4 or a new implementation contract.

## Current disposition

`BUILD` completed for one opt-in `converge4-ibl` comparison using existing IBL.
`REJECT` quality promotion: fixed-exposure mean brightness rose 27–32%, failing
the registered comparator in every view and washing out scene contrast in the
lead's visual review. The reproduction arm remains experimental and uncommitted.
No default change, new shader, exposure retuning, or new scene content.
Implementation review found no blocking authorization defect; execution and
visual disposition are recorded in the comparison receipt.
Witcher 3 is the completeness target. A higher-end option remains research;
none of the work here establishes Witcher 3 or UE5 parity.

## What the workers left to examine

- `SOURCE FINDING; NEXT SLICE`: preview camera updates retain the same packet
  identity (`session.rs:252-277, 361-380`), while main/reflection mesh batches
  are built from camera visibility and cached only by that packet identity
  (`LavaAdapter.jl:5122-5136, 5723-5731, 6357-6420`). A later camera request can
  therefore omit newly visible meshes while reporting freshly computed
  visibility. Overlay resources have a similar camera-dependent cache. This
  is source-audited, not live-reproduced or fixed here. The shadow frame is
  light-derived and does not need invalidation for this specific camera change.
  New primary evidence puts moving-camera correctness ahead of further beauty
  work. Current fixed-camera comparisons use distinct packets and are unaffected.
- `MEASURE FIRST`: attribute transfers before ranking optimizations. Material
  texture residency reports 36,196,472 bytes of packed RGBA8 data, not GPU
  transfers. The adapter expands uploaded texels to RGBA32F (16 bytes each).
  If every referenced texel uploads, that is 144,785,888 bytes; actual transfer
  is lower or equal because material textures load lazily for visible draws.
  An earlier worker assessment calling textures secondary was withdrawn.
- `MEASURE`: the adapter already instances repeated objects, but keeps
  separate flattened geometry resources for shadow, reflection, and main
  passes (`LavaAdapter.jl:5131-5233`). The close control's 1,633,068 main-stream
  vertices imply about 91.45 MB of static streams for one copy. Its total
  reported uploads are 291.16 MB. This supports examining cross-pass static
  geometry reuse; it does not prove the exact redundant-byte count yet.
  Per-pass visibility and instance transforms must remain separate. The full
  packet hash also includes camera/capture fields, invalidating resource
  caches between views; camera-independent immutable resource keys are a
  separate opportunity. Future wind/skinning needs explicit invalidation.
- `PARK UNTIL ATTRIBUTED`: native RGBA8 UNORM is present in vendored Lava.
  The manual image path could use Vulkan sRGB for color textures while keeping
  UNORM for data textures and alpha. Preserve mip chains, format capability
  checks, and correct linear filtering. Exact sampling/capture equality is
  unproven; a fourfold reduction in texture transfer is a candidate, not a
  measured engine speedup. IBL's 171,344 raw texture bytes currently become
  685,376 upload bytes. The paired captures measure an exact 685,376-byte
  increase in every view; the earlier 16-byte placeholder subtraction was
  not reflected in the actual counter and is withdrawn.
- `MEASURE`: `ibl::apply_ibl` bakes, sealing validates by baking again, and the
  supervisor validates and re-lowers from explicit campaign inputs. There are
  roughly five bakes before rendering each candidate view, with further
  validation possible during receipt/evidence production. One bake performs
  about 1.8 million sampling iterations. This is a structural count, not a
  timing attribution. The BRDF LUT is environment-independent and is a useful
  separate cache candidate. Any cache must key every effective bake input and
  algorithm revision and preserve exact output and rejection behavior.
- `MEASURE`: packets are large and several paths canonicalize/hash them.
  Pretty-printed file size is not worker-frame size. The worker's 128 MiB bound
  applies to compact framed payloads. Upload telemetry measures submitted
  bytes, not peak VRAM. Cold campaign wall time and capture-labelled timings
  do not establish steady-state gameplay performance.
- `PARK`: the sky/ground environment has no actual terrain, canopy, buildings,
  or local-light occlusion. It cannot supply scene-dependent diffuse bounce
  or distant ridge reflections. Adding spatial probes changes the renderer
  contract and requires its own experiment.
- `PARK`: the current foliage material uses the existing opaque GGX lighting;
  IBL does not add transmission through thin leaves. Inspect leaf highlights,
  pool response, and material separation before accepting the lighting arm.
- `PARK`: the RGBA8 atlas has six roughness levels (128² down to 4²), a 32²
  irradiance tile, and a 64² LUT. Higher precision is unearned until a moving
  highlight test or offline oracle exposes material error.
- `PARK`: missing-input messages in `main.rs` omit the new selector, although
  shared predicates and the campaign launcher supply its required assets.
  The older terrain-only helper lacks kit/backdrop inputs and fails closed;
  the comparison uses `render-campaign2-layout`.

## Bounded next experiments

Before optimization, reproduce the preview issue through two camera-update
requests on one packet, comparing camera B after A against B in a fresh worker.
Use deliberately disjoint visible mesh sets and real renderer output. Then
write a separate fix recipe: invalidate camera-dependent instance/overlay
state while retaining immutable geometry and light-dependent shadows. Check
both visibility correctness and transfer cost; a blanket full-resource flush
on every camera update is not an earned efficiency design.

0. **Attribute uploads.** Break out actual texture transfers, static mesh
   streams, and instance streams across all passes in one close C4 render.
   If identical repeated static uploads exceed 20% of total uploads, write a
   separate recipe for a bounded mesh-content cache. Keep it only if packet
   and capture hashes remain identical and upload bytes fall at least 20%.
   Reject stale reuse after positions, indices, normals, UVs, or tangents
   change. Upload reduction alone cannot establish a frame-time improvement.
   If textures exceed 20% instead, test an RGBA8/sRGB upload path on the
   smallest real fixture with an explicit sampling oracle, alpha/mip checks,
   and fixed-exposure capture parity. Reject it if those gates fail; do not
   invent or tune an error allowance after viewing its output.
1. **Attribute candidate CPU cost.** Profile cold and warm lower → seal →
   supervisor re-derivation separately, including bake count/time, hash/JSON
   time, allocation/RSS, and compact bytes. Reject optimization work if IBL
   bookkeeping is below 5% of campaign CPU. Keep a cache only if it removes
   at least 50% of candidate-only CPU/allocation cost while preserving exact
   bytes, input invalidation, tamper rejection, and bounded storage.
2. **Review material regions at fixed exposure.** Compare pool, landmark, and
   foliage crops in all three views. Measure output clipping, local luminance
   range and highlight coverage. HDR values require a new honest measurement
   path; existing RGBA8 captures cannot provide them. Reject retention if
   readability worsens, clipping appears, or pool appearance regresses.
3. **Precision oracle only if needed.** Compare the existing bake to a higher
   precision offline reference over direction and roughness, then a short pan.
   Record saturation and radiance error alongside visible discontinuities.
   Reject higher precision if it has no visible moving-scene benefit.

The rejected drop-in lighting probe leaves a calibration question, not an
algorithm diagnosis. `_material_response` replaces the legacy 0.52 diffuse
ambient coefficient and heuristic specular constants with normalized IBL; it
does not add both branches. The bake also includes the drawn sky's glow/scatter.
Distinguish scene radiance calibration from missing spatial sky visibility in
a separately registered neutral-material experiment. Do not compensate with
exposure inside this completed probe or treat a valid packet as aesthetic proof.

## Optional high-end tier: parked research

Keep L-1 shadows → R-2 crispness → N-8 contact AO → Sidhe as the active
sequence. Heartwood needs real hero trunks/root geometry, canopy, settlement,
flowing water, warm local lights, and eventually people/Cervideth animation.
These content and motion requirements survive any renderer upgrade.

| Candidate | First useful test | Dependency and kill gate |
| --- | --- | --- |
| Probe diffuse GI | Fixed Heartwood canopy/interior regions against IBL during a camera pan | Real canopy/walls and local-light behavior first. Reject leaks, pumping, weak bounce benefit, or a predeclared time/memory budget failure. |
| Shadowed volumetric fog | Sun-only forest corridor and short pan; cheap L-9 path first | L-1 shadows and typed density policy first. Reject unshadowed shafts, banding, washout, or weak depth benefit. |
| Temporal reconstruction | Fine foliage/trunks at native resolution versus a fixed lower internal resolution during a 10-second pan | Correct motion vectors, native TAA/specular AA, wind and skinning first. Reject trails, disocclusion ghosts, excess blur, or budget failure. |

Every arm needs a declared GPU-time and peak-memory budget on the RTX 5060
8 GB target before implementation. Probe volumes, acceleration structures,
fog grids, scene assets, and history buffers compete for that budget. No
performance figures are inferred from other engines.

Primary research references: [NVIDIA DDGI shader responsibilities](https://github.com/NVIDIAGameWorks/RTXGI-DDGI/blob/main/docs/ShaderAPI.md),
[probe GI paper](https://arxiv.org/abs/2009.10796),
[Epic volumetric fog](https://dev.epicgames.com/documentation/en-us/unreal-engine/volumetric-fog-in-unreal-engine),
[Epic temporal reconstruction](https://dev.epicgames.com/documentation/en-us/unreal-engine/temporal-super-resolution-in-unreal-engine),
[Filament material antialiasing](https://google.github.io/filament/Materials.md.html).

Next spiral: disposition the actual IBL captures, then reproduce and repair
the moving-camera cache issue before resuming L-1's consistency/cascade work.
Profiling is a separate slice; it must not weaken authorization checks.
