# Luxel production engineering audit

2026-10-16 · PEA Mode 1, full audit · branch `luxel-consolidation`, audited at
a9e4dd4 · fix pass 2026-10-17, through 705f059 · validation fixes (L-15,
L-16, L-17) and the GPU gate 2026-10-17

## 1. Executive result

**Release decision: Pass with accepted risks**, for continued development on
`luxel-consolidation` and a fast-forward merge to `main`. Luxel is **not**
production-ready as a real-time engine. That is a known roadmap gap, not a
defect in this branch.

- Correctness is well guarded. Rendering is deterministic and gated by capture
  digests (18/18 parity frames). The GPU suites pass: lava_adapter 26/26,
  present_blit 61/61, camera_continuity 88/88, plus the full lighting, shadow
  and material suites.
- The fix pass closed 10 of 17 findings: L-01, L-03, L-04, L-06, L-12, L-13,
  L-14, L-15, L-16 and L-17. The Rust workspace is green again (the CPU gate passes locally and
  `live.rs` passes 5/5), and fmt and clippy are clean.
- The first Khronos validation-layer run found no sync hazards in the
  frame-overlap paths it reached. It found two spec violations in generated
  shaders and one image-layout error after a window resize (L-15, L-16,
  L-17). All three are fixed without changing a pixel, and the six GPU suites
  now run under the layer, sync validation on, with an empty log. That run is
  the local GPU gate, `tools/gpu-gate.sh`.
- The main production risks still open are:
  - frame rate (15–23 fps against a 60 fps target; L-10). The half-second
    GC hitches (L-05) are fixed.
- No Blockers.

## 2. Ranked findings

| ID | Campaign | Severity | Summary | Status |
|---|---|---|---|---|
| L-01 | E / A | Important | Vendored Lava edit in 985600e has no patch file | Fixed, d8e5c2c |
| L-12 | D | Important | Three fixture digests stale since the rename; `cargo test --workspace` had 10 failures | Fixed, de63c14 |
| L-13 | D | Important | Reachability test read ~10 GB of scratch and ran past an hour | Fixed, 444241c |
| L-02 | C | Important | In-recording uploads wait on the whole previous frame; pan throughput is flat | Fixed, Lava patch 0005 (presented-fps fix 5) |
| L-03 | D | Important | No CI, and no Vulkan validation-layer run, for the new frame-overlap sync | Fixed, 705f059 and the validation run; findings in L-15, L-16, L-17 |
| L-15 | D | Important | Terrain shaders emit invalid SPIR-V (`OpConstantNull` of a Function-storage pointer) | Fixed, f3b7681 (Lava patch 0004) |
| L-16 | D | Important | Wet-terrain point-light pipeline exceeds the 128-component vertex output limit | Fixed, b7324b2 |
| L-17 | D | Important | After a window resize, a render pass begins on a swapchain image still in `PRESENT_SRC_KHR` | Fixed, b382fb7 |
| L-04 | D | Important | Bare `catch` in GPU timing permanently disables timestamps and misreports the capability | Fixed, 4099abb |
| L-05 | D / C | Important | Isolated 478 ms and 560 ms frames, not traced | Fixed: traced to GC; GC threads plus allocation cuts (Lava patch 0006; presented-fps fix 6) |
| L-06 | E | Important | Docs index and debt ledger out of date | Fixed, 842a808 |
| L-14 | D | Advisory | `test_zone_compiler.py` errors 52 times without reference content | Fixed with the CPU gate |
| L-07 | D | Advisory | `verify_identity.py` frozen references fail 0/18 by known drift | Open |
| L-08 | B | Advisory | `LavaAdapter.jl` is a 7966-line single module | Open, deferred |
| L-09 | C | Advisory | Textures arrive base64-encoded inside the JSON packet; cold-load cost not measured | Open, measure first |
| L-10 | C | Advisory | 60 fps target exists, but there is no per-pass budget or quality-tier budget | Open |
| L-11 | E | Advisory | Empty untracked root directories; ignored artifacts unreviewed | Plan below, not executed |

### L-01 — Vendored Lava edit is not recorded as a patch

- **Campaign / severity:** E (hygiene), A (dependency boundary). Important.
- **Location:**
  - `graphics_lab/vendor/Lava/src/graphics/api.jl` (`draw!`, new `resolve::Bool=true`);
  - `graphics_lab/vendor/README.md`;
  - the patch series `graphics_lab/vendor/lava-000N-*.patch`.
- **Observed:** commit 985600e added a `resolve` keyword to Lava's offscreen
  `draw!`. The vendor README says the tree is upstream 11c7e31 plus patches
  0001 (fragment discard) and 0002 (multisample), and that `diff -r` against
  that recipe must be empty. No patch covered the change.
- **Consequence:** rebuilding the vendored tree from the recipe silently drops
  the change. The renderer would then resolve on every draw again (a
  performance regression with identical pixels, so the digests would not catch
  it), or fail on the unknown keyword.
- **Fix (d8e5c2c):** the hunk is now `lava-0003-draw-resolve.patch`, with a
  README row. Upstream 11c7e31 plus patches 0001–0003 matches
  `vendor/Lava/src` under `diff -r`.
- **Status:** fixed. It had been introduced by this branch.

### L-12 — Three content-hashed fixtures were not re-derived after the rename

- **Campaign / severity:** D. Important. `cargo test --workspace` was red on
  `main`: 10 tests failed in 4 targets. A red suite hides any new failure in
  those binaries.
- **Location:**
  - `tests/fixtures/gameplay_contract/vertical_slice_v1.expected.json`,
    field `receipt_sha256`;
  - `source_bundle_id` in 6 files under
    `world_core/crates/intake_repair_contract/fixtures/intake/`;
  - `response_sha256` in `intake-draft.json` (found once the bundle id was
    fixed, as the audit predicted might happen).
- **Cause:** 05ab2c8 (2026-10-06, the Luxel rename) renamed schema ids inside
  these fixtures but did not re-derive the digests that hash them. Its gate run
  skipped `luxel-gameplay-contract`, `luxel-intake-repair-contract` and
  `luxel-project-ledger`.
- **Proof that the rename is the only cause:** for each digest, mapping
  `luxel.` back to `wge.` reproduces the old fixture value exactly, and the
  unmapped input gives the value the code computes now.
  - Receipt: old `018af969…`, now `b699c36c…`.
  - Bundle id: old `46a2eef8…`, now `49a3f7cd…`.
  - Provider response: old `b430275c…`, now `af40f709…`.
- **Fix (de63c14):** the three digests were re-derived, with the proof in the
  commit message. Evidence only; no behaviour change.
- **Status:** fixed. The three crates pass.

### L-13 — The reachability test read the research scratch tree

- **Campaign / severity:** D. Important. Found during the fix pass.
- **Location:** `pipeline/reachability.py`, `_source_files`.
- **Observed:** the real-pipeline reachability test walks the repo root and
  read every source-suffixed file under the git-ignored `research/` tree,
  about 10 GB of raw capture JSON. The Python suite ran past an hour and looked
  hung.
- **Fix (444241c):** `research/` and `.local/` (the hygiene standard's scratch
  homes) are skipped at the walk root only, so the committed `docs/research/`
  is still scanned.
- **Verification:** `test_reachability` 18 OK in 76 s.
- **Status:** fixed.

### L-02 — Uploads during recording serialize CPU and GPU on a moving camera

- **Campaign / severity:** C1 (dataflow). Important.
- **Location:**
  - `LavaAdapter.jl` mesh/instance upload path;
  - vendored Lava `copy_buffer!` → `flush!`.
- **Observed:** every upload issued while recording flushes and waits on all
  in-flight batches. Since a9e4dd4, in flight includes the whole previous
  frame.
- **Evidence:** `docs/research/luxel-consolidation/2026-10-16_presented-fps.md`
  (fix 4).

  | View | Before (fix 3) | After (fix 4), two runs |
  |---|---|---|
  | Still close | 17.7 batch fps | 21.4–23.2 batch fps |
  | Pan close | 11.0 batch fps | 10.8–12.7 batch fps |

  Pan `scene_record` went from 36–40 ms to 64–82 ms. Pan
  `gpu_scene_raster` is 144 ms at p95 against 42 ms at p50.
- **Consequence:** a moving camera, the normal case in play, gets no benefit
  from frame overlap.
- **Remedy:** non-waiting uploads into freshly allocated buffers. Lava patch
  0005 adds `upload_recorded!`: it records the staging copy into the open
  batch, then a transfer-to-all-commands barrier. The staging buffer is
  pinned to that batch and freed when the batch retires. `_mesh_resources!`
  uses it. Pixel-neutral. This is roadmap step 4, Part 2.
- **Verification** (`2026-10-16_presented-fps.md`, fix 5):
  - the 18 digests are unchanged, 18/18;
  - the GPU gate passes: 6 suites, sync validation on, empty validation log;
  - pan p50 frame fell 16–17 ms on every view;
  - pan `scene_record` fell from 64–82 ms to 23–41 ms;
  - pan batch fps rose on close (12.9 → 16.3) and medium (11.8 → 15.3).
    Wide stayed at 10.8 because of one 558 ms frame; that is L-05.
- **Change risk:** medium. This is Lava sync work. Hazard review: the
  destination is freshly allocated, so no GPU work reads or writes it; draws
  pin their arrays, so an in-flight array is not freed and reused under the
  copy; and the copy is recorded outside rendering scopes. Sync validation does
  not track buffer-device-address loads, so the graphics-stage barrier is
  reasoned, not tested.
- **Status:** fixed.

### L-03 — No CI and no validation-layer evidence

- **Campaign / severity:** D (reliability). Important.
- **Observed at audit:**
  - No workflow ran `cargo test`, the Python suite or the Julia suites.
  - The GPU suites have never run with `VK_LAYER_KHRONOS_validation`.
  - a9e4dd4 introduced CPU/GPU frame overlap. Its correctness rests on digest
    equality and the absence of hangs.
- **Consequence:**
  - A sync hazard (a write-after-read on a reused buffer) can produce correct
    digests on this driver and corrupt frames on another.
  - Regressions land unless someone runs the suites by hand.
- **Fix, CI part:** `tools/cpu-gate.sh` and `.github/workflows/cpu-gate.yml`.
  The gate runs fmt, clippy with `-D warnings`, every Rust test target that
  needs no Julia worker, a bin build, and the Python suite minus the
  Julia-backed modules.
  - The exclusions were measured, not guessed: the full suites ran with
    `LUXEL_JULIA=/bin/false` and no reference content, and every excluded
    target failed only with `julia_worker_failure` (or the GPU test lock that
    one such failure poisons).
  - The excluded targets are listed in the script. They stay in the local GPU
    gate.
  - The CI job has not run on GitHub yet: nothing is pushed. A local run with
    Julia off `PATH` and no reference content is recorded in section 4.
- **Validation layer, run 2026-10-17:**
  - Arch `vulkan-validation-layers` 1.4.363.0, signature checked, unpacked
    without installing (no sudo), and loaded through `VK_ADD_LAYER_PATH`.
  - Settings: `VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation`,
    `VK_KHRONOS_VALIDATION_VALIDATE_SYNC=true`, errors and warnings reported,
    under the GPU lock.
  - Lava's `check_validation_errors!` turns any validation message into a
    `LavaError`, so the first message ends the test set that raised it.
  - Run 1: every suite stopped at the first terrain pipeline on
    `VUID-VkShaderModuleCreateInfo-pCode-08737` (L-15).
  - Run 2 filtered that one message ID:
    - lava_adapter ran every set through "local point attenuation", including
      the persistent adapter (26/26) and moving-camera continuity (88/88).
      These sets cover the frame-overlap path. **No `SYNC-HAZARD` message.**
      It then stopped at the wet-terrain point-light pipeline on
      `VUID-RuntimeSpirv-Location-06272` (L-16). The sets after that one did
      not run under the layer.
    - camera_continuity: 88/88 with no validation message at all.
    - present_blit: the GPU-vs-CPU present match (4/4) and resized-window
      present (6/6) were clean. "presented session survives a window resize"
      stopped on `VUID-vkCmdBeginRendering-pRenderingInfo-09592` (L-17), and
      no later set ran.
- **Status:** fixed. CI exists, and the validation run is done. Its three
  findings are L-15, L-16 and L-17. A clean full run waits on them.

### L-04 — GPU timing turned any error into "timestamps unsupported"

- **Campaign / severity:** D. Important.
- **Location:** `graphics_lab/src/LavaAdapter.jl`, five bare `catch` blocks in
  the begin/end frame and pass timing functions and in
  `_read_gpu_timing_reports`.
- **Observed:**
  - Each block set `state.gpu_timestamp_supported = false`. A bare `catch`
    also catches `InterruptException` and real device or batch errors.
  - The flag was never reset, and the capabilities report publishes it as
    `gpu_timestamps`.
- **Consequence:** one transient failure silently ended GPU timing for the
  session, and the capability report then said the device lacked timestamps.
- **Fix (4099abb):**
  - `gpu_timestamp_supported` is now a device fact, set once from the limits
    and never downgraded.
  - Every timing failure goes through `_gpu_timing_failed!`. It rethrows
    `InterruptException`, counts the failure, keeps the last message, warns
    once, and drops only that frame's readback (a start slot without an end
    would hang the query). The next frame times afresh.
  - The two `_end_*_timing!` bodies are now one `_end_gpu_timing!`.
- **Verification:** a new present_blit test injects a timing failure. It
  asserts that the capability stays true, that the failure is counted with its
  message, that the failed frame reports no times, and that the next frame
  reports them again. present_blit 61/61, lava_adapter all sets,
  camera_continuity 88/88.
- **Limit:** the failure count and last message live on the backend state and
  in a one-time warning. They are not yet in frame telemetry, because that
  would change the packet schema. Publish them when the telemetry schema is
  next revised.
- **Status:** fixed.

### L-05 — Isolated long frames are not traced

- **Campaign / severity:** D / C. Important.
- **Evidence:** in the presented-fps note (fix 4), each run had one isolated
  frame of 478 ms (pan close) or 560 ms (still medium). The single fix 3 run
  peaked at 170 ms.
- **Consequence:** a half-second hitch is a visible stall. In the runs above it
  also sets the low end of batch fps.
- **Trace:** `LUXEL_FRAME_TRACE=<path>` now writes one JSON line per frame:
  GC pause, mark and sweep time, allocation, compile time, pipeline creations,
  upload bytes, CPU stage timings and GPU pass timings. The spikes were Julia
  GC: one 491 ms (pan) or 541 ms (still) collection, on one GC thread. The
  worker allocated 7–17 MiB a frame.
- **Remedy** (presented-fps fix 6):
  - the supervisor starts the worker with `--gcthreads=4,1` unless
    `JULIA_NUM_GC_THREADS` is set;
  - Lava patch 0006, `map_args`: Base `map` over a tuple of more than 31
    arguments boxes every element, and the 47-argument mesh draw allocated
    about 7 MiB a frame;
  - `_mesh_resources!` packs the seven per-instance streams into one buffer
    and groups instances by index, without copying them.
- **Verification:**

  | Trace | Worst GC (ms) | GC total (ms) | Worst frame (ms) | Alloc/frame (MiB) |
  |---|---|---|---|---|
  | still, 1 GC thread | 541 | 894 | 564 | 7.4 |
  | still, after | 0 | 0 | 61 | 1.1 |
  | pan, 1 GC thread | 491 | 742 | 521 | 17.4 |
  | pan, after | 35 | 89 | 99 | 4.9 |

  Steady-state frames, 84 per trace. The 18 digests are unchanged, 18/18, and
  the GPU gate passes. Pan wide batch fps rose 10.8 → 14.8.
- **Left:** the first frame of each process (about 200 ms of compile) and the
  first frame of each new view (GPU time 200–235 ms, against 45–65 ms steady)
  are still long. That is first-use warmup, not GC. It is a separate follow-up
  (section 7).
- **Status:** fixed.

### L-06 — Documentation drift

- **Campaign / severity:** E. Important.
- **Observed at audit:**
  - 58 tracked docs were not listed in `docs/README.md`.
  - `docs/platform/debt-ledger.md` still listed D4 (a doc that stayed with
    Codeweald) and D8 (`godot_renderer` as a Luxel directory) as open.
  - The world viewer README pointed at a `godot_renderer/` tree that is not in
    this repo.
- **Fix (842a808):** the 58 docs are indexed; D4 and D8 are recorded as
  resolved; the world viewer README now uses `pipeline/` and
  `LUXEL_REFERENCE_CONTENT`, as the tests do.
- **Remaining references:** `godot_renderer` still appears in 13 tracked
  files. They are archived docs, the topology doc that records it as historical,
  repository fences, and the default `LUXEL_REFERENCE_CONTENT` path in the
  tests and the world viewer. All are deliberate.
- **Not done:** re-baselining the 1495-line ledger and archiving its
  Godot/Bevy-era history. That moves files, so it needs the plan shown first.
- **Status:** fixed (index and stale entries); ledger re-baseline deferred.

### L-14 — `test_zone_compiler.py` failed without reference content

- **Campaign / severity:** D. Advisory. Found while measuring the CPU gate.
- **Observed:** with no reference content present, the module raised 52 errors
  where the other content-backed modules skip.
- **Fix:** the class now has
  `@unittest.skipUnless(content_available(), …)`, as the other modules do.
  52 skipped, OK. Committed with the CPU gate.
- **Status:** fixed.

### L-15 — Terrain shaders emit invalid SPIR-V

- **Campaign / severity:** D. Important. Found by the L-03 validation run.
- **Location:** the terrain vertex and fragment shaders (`_terrain_vertex`,
  `_terrain_fragment`), as lowered by vendored Lava's Julia → SPIR-V path.
- **Observed:** `VUID-VkShaderModuleCreateInfo-pCode-08737`, spirv-val:
  "Instruction may only return a logical pointer in the StorageBuffer or
  Workgroup storage classes with appropriate variable pointers capability:
  `%2473 = OpConstantNull %_ptr_Function_uint`".
- **Consequence:**
  - The module breaks the SPIR-V environment rules. This driver accepts it,
    and the digests prove its output is right here. A stricter driver may
    reject the module or miscompile it.
  - Every validation run stops at the first terrain pipeline unless this
    message ID is filtered, so the validation layer cannot yet be a gate.
- **Remedy:** find the Julia construct that produces a null Function-storage
  pointer (likely a `Ref` or an uninitialized local that survives to codegen).
  Then either rewrite it in the shader or fix the lowering in Lava, as a
  `lava-000N` patch. Check with `spirv-val` on the dumped module, then
  confirm the 18 digests are unchanged.
- **Cause:** StructurizeCFG leaves `phi ptr` chains over an SROA-split `sret`
  box that only one branch writes. They lower to an `OpPhi` that yields a
  Function-storage pointer, with an `undef` edge that becomes
  `OpConstantNull`, and in three modules an illegal pointer `OpBitcast`.
  14 terrain and point-light fragment shaders were affected.
- **Fix:** Lava patch 0004 adds `fold_uniform_function_ptr_phis!` to
  `run_llvm_passes!`. A pointer phi whose incoming values all name one
  Function-storage address (or are `undef`) becomes that address. Modules
  without such a phi are untouched. The patch is recorded in
  `graphics_lab/vendor/README.md`, and the rebuild recipe diffs clean.
- **Verified:**
  - all 18 parity frames are byte-identical to a render of the commit before
    the fix;
  - no Function-pointer phi remains in the dumped modules, and
    `spirv-val --target-env vulkan1.3 --scalar-block-layout` passes them;
  - the GPU suites pass with an empty validation log.
- **Tooling gap:** the validator in `SPIRV_Tools_jll` (v2025.4) accepts these
  shaders. A current `spirv-val` (v2026.4) rejects them. Do not treat the
  bundled validator as proof of validity.
- **Status:** fixed.

### L-16 — Wet-terrain point-light pipeline exceeds the vertex output limit

- **Campaign / severity:** D. Important. Found by the L-03 validation run.
- **Location:** `state.point_pipelines[5]` and `[6]`, the wet terrain with
  point lights, recorded in `_record_scene_passes!` (`LavaAdapter.jl`).
- **Observed:** `VUID-RuntimeSpirv-Location-06272`. The vertex outputs and the
  fragment inputs use Location 31, component 3. That is 32 locations (128
  components) plus the 4 built-in `Position` components, so 132 against
  `maxVertexOutputComponents` and `maxFragmentInputComponents` of 128.
- **Consequence:** the pipeline breaks a device limit. It works on this
  driver, but a driver that enforces the limit fails to create the pipeline,
  and the wet-terrain point-light path then has no fallback.
- **Remedy:** pack the varyings. Several are scalars or `Vec3`s that each take a
  whole location. Pack them, or derive values in the fragment shader from data
  it already has. The wet terrain and point light varyings are the
  candidates. Pixel-neutral if done exactly. Verify with the 18 digests and a
  rerun under the layer.
- **Fix:** `_terrain_layered_wet_vertex` sends `wet_d` in location 14, the
  emissive slot, which no layered fragment reads, and no longer writes
  location 31. The outputs are now 31 locations plus `Position`, exactly 128
  components. `_terrain_layered_shade` reads `wet_d` from location 14.
- **Verified:** all 18 parity frames byte-identical to the pre-fix render;
  lava_adapter, camera_continuity 88/88, local_lighting_gpu 45/45,
  local_shadows_gpu 11/11 and shadow_cascades_gpu 49/49 pass with an empty
  validation log.
- **Status:** fixed.

### L-17 — Image layout error after a window resize

- **Campaign / severity:** D. Important. Found by the L-03 validation run.
- **Location:** the window present path in `render_window_frames!`
  (`LavaAdapter.jl`) and vendored Lava's swapchain layout tracking
  (`graphics/framebuffer.jl`, `graphics/pipeline.jl`, `graphics/api.jl`).
- **Observed:** in `present_blit_gpu.jl`, "presented session survives a window
  resize", the window is resized and two frames are presented on `:auto`, then
  two on `:cpu`. On the `:cpu` frames, the submit inside
  `readback_framebuffer` failed validation with
  `VUID-vkCmdBeginRendering-pRenderingInfo-09592`: the command buffer begins
  rendering on a swapchain color image and expects
  `COLOR_ATTACHMENT_OPTIMAL`, but the image's current layout is
  `PRESENT_SRC_KHR`.
- **Consequence:** a render pass reads or writes a presentable image in a
  layout it does not hold. This driver tolerates it, and the test passes
  without the layer. On another driver the result is undefined: corrupt or
  stale pixels in the first frames after a resize, or a device loss.
- **Likely cause, not yet confirmed:** a draw recorded in load mode (which
  assumes `COLOR_ATTACHMENT_OPTIMAL`) onto a swapchain image that was
  acquired fresh after the resize or the path switch. A freshly acquired
  image is in `PRESENT_SRC_KHR`, or `UNDEFINED` after the swapchain is
  recreated.
- **Remedy:** trace which recorded draw targets the swapchain image in that
  batch. Then make Lava's window draw transition from the layout the image
  really holds, tracked per image and reset when the swapchain is recreated.
  Do this as a `lava-000N` patch. Verify the resize set under the layer, then
  run present_blit 61/61 and the 18 digests.
- **Cause, confirmed:** the `:cpu` present fallback uploaded the frame and
  blitted it with `clear=false`. A load-mode draw records its transition from
  `COLOR_ATTACHMENT_OPTIMAL`, but the image was just acquired, so it was in
  `PRESENT_SRC_KHR` (or `UNDEFINED` after the swapchain was recreated).
- **Fix:** the fallback blits with `clear=true`, which transitions from
  `UNDEFINED`. The opaque fullscreen blit covers every pixel, so the clear
  never shows. The fix is one adapter call; no Lava layout-tracking patch was
  needed.
- **Verified:** present_blit_gpu passes every set (GPU blit matches CPU,
  resize, session survives resize 36/36, timing slots) with an empty
  validation log; 18 parity frames byte-identical.
- **Status:** fixed.

### L-07 — `verify_identity.py` cannot pass

- **Campaign / severity:** D. Advisory.
- **Observed:** the frozen references fail 0/18 because of known, accepted
  drift. The real oracle is the digest chain in the Julia parity suites. The
  roadmap still names `tools/verify_identity.py` as the identity rule.
- **Consequence:** a gate that always fails trains people to ignore it.
- **Remedy:** re-freeze the references from the current digest chain, or retire
  the tool and point the roadmap at the parity suite. That is a choice for the
  owner.
- **Status:** open.

### L-08 — `LavaAdapter.jl` monolith

- **Campaign / severity:** B. Advisory.
- **Observed:** 7966 lines in one module. It covers window session and present,
  capability and limit probes, mesh and terrain resources, shadow cascades and
  point shadows, IBL, sky, overlay and GPU timing. `LuxelGraphics.jl` is 2285
  lines. `native_graphics_contract/src/lib.rs` is 7643 lines, although it
  already has 21 submodules.
- **Remedy:** extract one subsystem per `include`d file along the existing seams
  (timing, shadows, IBL/sky, window session) with no behavior change. Do it when
  each area is next touched, not as a standalone campaign.
- **Status:** open, deferred.

### L-09 — Base64 textures inside the JSON packet

- **Campaign / severity:** C2. Advisory, unmeasured.
- **Observed:** texture payloads travel as base64 strings in the scene packet.
  That costs about 33% in size, plus a decode and copy on load. Packet
  residency already removed per-frame resends, so this costs only on cold load.
- **Remedy:** first measure cold-load time and packet size on CONVERGE-4. Only
  if that is material, move textures to a side-channel blob referenced by
  digest.
- **Status:** open, measure first.

### L-10 — No frame budget below the 60 fps target

- **Campaign / severity:** C. Advisory.
- **Observed:** `docs/platform/missing-inventory.md` sets 60 fps as the target,
  but no document splits the 16.7 ms frame across passes or quality presets.
- **Remedy:** write a one-page budget for the top preset and one lower preset
  (mesh, reflection, shadow, post, CPU record), and track it in the
  presented-fps benchmark.
- **Status:** open. It belongs with the quality-preset work.

### L-11 — Root and naming hygiene

- **Campaign / severity:** E. Advisory.
- **Inventory (2026-10-17):**
  - Empty untracked root directories: `Playable Demos`, `.aws`, `.codex`,
    `.agents`. Each holds nothing, and none is tracked.
  - Ignored items include `artifacts/`, `.freebuff/`, `.local/`,
    `graphics_lab/TetLab.zip`, ten `graphics_lab/tet-lab/logs/*.log` files,
    `character_lab/*.html` generated pages, the Unity harness `bin/` and
    `obj/`, a Cubism `.psd` import, and `docs/research/pj-0/`.
  - The tet-lab reference paper and its text extract are ignored on purpose:
    the paper must never be committed.
- **Plan (not executed; deletes need the owner's approval):**
  1. Remove the four empty root directories, unless a tool recreates them (the
     `.codex` and `.agents` names suggest agent tooling does).
  2. Decide on `docs/research/pj-0/`. Curated research belongs committed under
     `docs/research/<branch>/`; if this is raw evidence, it belongs under
     `research/`.
  3. Size the ignored artifacts and register any long-lived scratch with
     `hygiene gc register`.
- **Status:** plan only.

## 3. Remediation record

The audit itself was report-only. The fix pass on 2026-10-17 made these
commits on `luxel-consolidation`. None is pushed.

| Commit | Finding | Change |
|---|---|---|
| de63c14 | L-12 | Re-derived three fixture digests the rename left stale |
| d8e5c2c | L-01 | Recorded the vendored Lava `draw!` resolve change as patch 0003 |
| b4137d0 | — | Cleared the two `luxel-control-plane` clippy warnings |
| 73f89a8 | — | Applied rustfmt to the files the rename left unformatted |
| 842a808 | L-06 | Indexed every current doc and retired two stale ledger entries |
| 444241c | L-13 | Stopped the reachability walker reading the research scratch tree |
| 4099abb | L-04 | Kept the GPU timestamp capability when one timing write fails |
| 705f059 | L-03, L-14 | CPU-only gate script and CI workflow; zone compiler skip guard |
| f3b7681 | L-15 | Lava patch 0004 folds uniform Function-pointer phis |
| b7324b2 | L-16 | Wet-terrain `wet_d` varying moved to location 14 |
| b382fb7 | L-17 | CPU present fallback blits with `clear=true` |
| 75bd226 | L-03 | `tools/gpu-gate.sh`: the GPU suites under the validation layer |
| bc576f0 | L-02 | Lava patch 0005 records per-instance uploads into the open batch |
| (this commit) | L-05 | Frame trace; GC threads; Lava patch 0006 and allocation cuts |

Not attempted, and why:
- L-02 and L-05 are feature and measurement work on the roadmap. Both were
  done after the pass (Lava patches 0005 and 0006).
- L-07 needs an owner decision (re-freeze or retire).
- L-08 and L-09 are deferred by their own remedies.
- L-10 belongs with the quality presets.
- L-11 deletes files, so it needs approval.
- L-15, L-16 and L-17 were found at the end of the pass and fixed in a
  second pass the same day (see their sections), with `tools/gpu-gate.sh`.

## 4. Evidence

| Check | Result |
|---|---|
| Julia GPU: persistent Lava adapter | 26/26 at audit; all sets pass after L-04 |
| Julia GPU: present blit, resize, open-slot guard | 52/52 at audit; 61/61 after L-04; GPU vs CPU present bytes differing = 0 |
| Julia GPU: moving camera continuity | 88/88 |
| Julia GPU: cascades, point lights, point shadows, GGX, contact AO, materials | all pass at audit (107, 49, 84, 45, 48, 36, 17, 12, 11, …) |
| Identity digests (parity frames) | 18/18 byte-identical, A/B against a render of the pre-change commit (the frozen references fail by L-07) |
| `cargo test --workspace --no-fail-fast` at audit | 451 passed, **10 failed** (all L-12), 15 ignored |
| `cargo test --workspace --no-fail-fast` after L-12 | 460 passed, 1 failed: the source-identity guard in `native_graphics_contract/tests/live.rs`, tripped because L-04 edited `graphics_lab` during the run. Rerun after: `live` 5/5 passed |
| `cargo fmt --all -- --check` | clean after 73f89a8 |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean after b4137d0 |
| Python unittest | at audit, the suite did not finish (L-13). After the fixes: modules 0–547 OK, the remaining modules 194 OK, `test_reachability` 18 OK in 76 s |
| CPU gate, local, Julia off `PATH`, no reference content | exit 0; Rust 379 passed, 0 failed, 15 ignored; Python 690 OK, 99 skipped |
| Python error handling | 0 bare `except:`; the `except …: pass` sites in `pipeline/erosion.py` are subprocess-shutdown cleanup (fine) |
| Rust non-test `unwrap`/`expect` | `native_graphics_contract`: 6, all invariant `expect`s with messages; `project_ledger`: 1; `luxel_control_plane` and `world_viewer`: 0 |
| Vulkan validation layer, sync validation on | run 1: stopped at L-15 in every suite. Run 2 (L-15 filtered): no sync hazard in the sets reached; lava_adapter stopped at L-16; camera_continuity 88/88 clean; present_blit stopped at L-17 in the resize set. After the L-15–L-17 fixes, no filter: all six GPU suites exit 0 with an empty log, confirmed by a full `tools/gpu-gate.sh` run at b382fb7 |
| Presented fps (CONVERGE-4, fix 4) | still 12.3–23.2, pan 10.8–12.7 batch fps (presented-fps note) |
| Presented fps (CONVERGE-4, fix 6, after L-02 and L-05) | still 17.1–23.2, pan 14.8–17.2 batch fps; worst steady frame 61 ms still, 99 ms pan |

Warnings in the GPU logs that are known and not findings:
- Lava `replace_unreachable!` on `gpu_gc_pool_alloc`.
- `Skipping non-aggregate constant global: jl_small_typeof`.

These are compiler-pass diagnostics, and the parity suites cover their output.

**Checked and not findings:**
- The v6/v7 packet schema pair is deliberate, as documented in `deformation.rs`.
- The limit-probe catches in `LavaAdapter.jl` rethrow `AdapterError`
  correctly.
- The 101 grandfathered hygiene items are schema version names or engine
  plugin conventions.
- `docs/DSL docs/` breaks the naming rule on purpose: the language spec is
  hash-frozen, and `docs/README.md` records that it stays put.

## 5. Inheritance assessment

- The Rust authority → packet → Julia/Lava worker split is sound. Authority owns
  truth, and the renderer is a replaceable consumer. Inherit it.
- The digest-gated identity discipline is the project's strongest asset:
  - every pixel-changing change is visible;
  - the "top preset equals today's pixels" rule for quality presets builds on
    it directly.
- Vendored Lava is the main supply risk. The repo now carries six local
  changes, all recorded as patches (L-01 fixed). Patch 0005, the L-02 sync
  work, deepens the fork. Keep the patch series exact, and consider offering the
  changes upstream.
- The Bevy `world_viewer` is an inspection instrument, not the renderer. Do not
  build production features on it.

## 6. Release decision

**Pass with accepted risks**, for the fast-forward of `luxel-consolidation` to
`main`. The merge and push wait for the owner.

The fixes the audit recommended before merge are in: L-01 (patch file) and L-12
(fixture digests). `main` would get a green Rust suite and a CPU gate.

Accepted risks, each with an owner (user) and a next step:

1. L-02 pan throughput: fixed by non-waiting uploads (Lava patch 0005).
2. L-15, L-16 and L-17 validation errors: fixed, and a clean validation run
   is now the local GPU gate (`tools/gpu-gate.sh`). Run it on every sync
   change in Part 2.
3. L-05 spikes: traced to GC and fixed (GC threads, Lava patch 0006).
   First-use warmup frames remain (section 7).

This is not a release of a production engine. The 60 fps target, 1080p, LOD,
streaming and animation are roadmap work (`missing-inventory.md`).

## 7. Deferred work

In order:

1. ~~L-15, L-16 and L-17, then a clean full validation run.~~ Done.
2. ~~L-02 non-waiting uploads, validated under the layer.~~ Done.
3. ~~L-05 spike trace.~~ Done.
4. L-10 frame budget, with the quality presets and half-resolution reflection.
5. Mesh raster cost attribution: is it bound by vertex work, fragment work or
   draw count?
6. Step 6, the camera follows the window aspect.
7. Skinned characters.
8. L-04 telemetry fields, at the next telemetry schema revision.
9. L-07 decision; L-11 plan approval; L-06 ledger re-baseline.
10. L-08 and L-09 as their areas are touched.
11. First-use warmup: the first frame of a process (about 200 ms of compile)
    and of each new view (200–235 ms of GPU time). Prewarm pipelines and
    compile at startup, or record a precompile workload.
