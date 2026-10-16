# Presented frame rate on CONVERGE-4

2026-10-16 · luxel-consolidation branch

The live window now runs CONVERGE-4 at 10–13 fps, up from 1. Three changes
did it:

1. Caching mesh bounds and transform keys.
2. Keeping the scene packet resident in the worker.
3. Presenting with a GPU image blit instead of a CPU readback.

The scenes still look the same. Offscreen, the 18 parity frames render
byte-identical with and without these changes. The frozen references
themselves have drifted from current code for an unrelated reason; see
[Identity](#identity).

## How it was measured

- **Scene:** CONVERGE-4 (`converge4` policy) through `tools/parity_ab_policy.sh`.
- **Batches:** each view (close, medium, wide) presents one batch of 30 frames
  (120 for the baseline) in a real swapchain window, with vsync off.
- **Hardware:** one RTX 5060.
- **Build:** release build of `luxel-native-graphics-contract`.
- **Timings:**
  - The frame percentiles come from the worker's per-frame wall clock.
  - "Batch fps" is frames divided by the whole `render_window` request,
    including the packet transfer.
  - Per-stage times come from the `stage_times_us` that the worker now
    reports with each `render_window` response.

Raw logs are kept locally in `research/2026-10-16_presented-fps/` and are not
committed.

## Results

Batch fps:

| View | Baseline | + bounds/transform caches | + packet residency and GPU present |
|---|---|---|---|
| close 768×512 | 1.05 | 2.18 | **12.97** |
| medium 960×640 | 1.02 | 1.94 | **10.31** |
| wide 960×640 | 1.01 | 2.13 | **10.93** |

p50 stage times on the close view, in ms:

| Stage | Baseline | + caches | + GPU present |
|---|---|---|---|
| scene_record | 529 | 23.5 | 22.1 |
| main_prepare (inside scene_record) | 256 | 3.5 | 3.4 |
| main_raster_record (inside scene_record) | 4.6 | 8.3 | 8.2 |
| gpu_wait_readback / present_wait | 63 | 53 | 48 |
| convert (float to 8-bit, CPU) | 7.0 | 9.9 | — |
| frame p50 | 607 | 101 | 71 |

## What changed

### Bounds and transform caches (`LavaAdapter.jl`)

- **Cause:** `main_prepare` recomputed the bounds of every mesh on every frame,
  and rebuilt the per-instance transform keys every frame too.
- **Fix:** both are now cached per mesh and per instance. That cut
  `scene_record` from 529 ms to 23 ms.

### Packet residency (`luxel_graphics_worker.jl`, `session.rs`)

- **Cause:** after the caches, frames took about 100 ms but a 30-frame batch
  still took about 14 s. The difference was the session re-sending and the
  worker re-validating the whole scene packet on every `render_window`
  request.
- **Fix:**
  - The worker keeps the last validated packet.
  - A presenting session sends its packet once. Later requests name it only by
    `expected_packet_sha256`.
- **Recovery:** if the worker has lost the packet (for example after a restart),
  it answers `packet_not_resident`. The session then retries once with the full
  packet.
- **Tests:** the worker protocol tests cover the `packet_not_resident` reply.

### GPU present (`LavaAdapter.jl`)

**Old path:** the RGBA32F capture was read back to the CPU, clamped and
converted, uploaded again, and drawn into the swapchain with a fullscreen
blit.

**New path:**

1. When the device supports blitting from the capture format to the swapchain
   format, and the extents match, the capture is copied straight into the
   swapchain image with `vkCmdBlitImage` (nearest filtering, same extent).
2. The scene is submitted on its own. Only the blit batch waits on image
   acquire.
3. The frame is presented, and the worker waits on the blit's timeline value.

Anything else falls back to the CPU path. Passing `present=:cpu` forces it.

**Window resize:** the window is resizable, and KDE/Wayland resized it on its
own during a test run (640×480 became 640×489). The old code checked the camera
against the window size *before* acquiring the swapchain image, so a resize in
between crashed the CPU blit with a size mismatch. Now the swapchain extent is
read after acquire, and both paths scale the camera-extent frame into it. The
GPU blit is unfiltered at equal size and linear when scaling. The CPU path uses
a nearest resample. A resized window stretches the image; the camera does not
yet follow the window's aspect ratio.

`graphics_lab/test/present_blit_gpu.jl` compares the two paths on a 257×67 ramp
that runs past both ends of the unorm range. Every byte matches: 0 differing
channels. The same file also tests:

- a scaled blit against the CPU resample: 0 differing pixels, excluding the
  exact texel-boundary ties where the GPU's nearest rounding may go either
  way;
- a live window session resized mid-session (64×64 to 101×73) that keeps
  presenting on both paths.

**Not verified:** Vulkan synchronization validation did not run. The
validation layers are not installed on this machine.

## Identity

`tools/verify_identity.py` fails on both this change and the commit before it
(`69c2ebc`): 0/18 parity frames match their frozen references either way.

- **This change is output-neutral.** The 18 parity frames rendered with and
  without the new `LavaAdapter.jl` have identical capture digests, 18/18. The
  113 calibration frames were not cross-checked this way.
- **The references drifted for two other reasons:**
  - Arms such as `ab-null-tfix` now build packets that differ from the
    reference only in names, from the `wge` → `luxel` rename (texture IDs,
    schema version). Their pixels differ in small patches, up to 178 levels,
    which suggests some rendering step depends on those names. Not yet traced.
  - `ab-converge3-final` builds exactly the reference packet, yet 160–277
    bytes per frame differ by exactly 1. That pattern points at the NVIDIA
    driver upgrade on 2026-10-12 (615.71 → 615.78), but it is not proven. The
    October 12 shadow work could also cause it.

Re-freezing the references, or tracing the name dependence, is an open
decision and is not part of this change.

## Per-pass GPU timings

Presented frames now report GPU timestamp intervals per pass next to the CPU
stage times (`gpu_frame`, `gpu_shadow`, `gpu_reflection`, `gpu_prepare`,
`gpu_scene_raster`, `gpu_resolve`, `gpu_overlay`, `gpu_present_blit`). A pass
that did not run in a frame reports 0.

The benchmark also has a moving-camera mode. `LUXEL_PRESENT_MOTION=pan`
yaws the camera up to 15° and slides it up to 4 m. It sends one
`render_window` request per frame, the way a game loop drives the worker. A
still camera hides costs: the cascaded shadow maps are cached while the view
does not move.

p50 on the close view (768×512), in ms:

| Stage | Still | Pan |
|---|---|---|
| frame | 74.6 | 120 |
| batch fps | 13.6 | 7.6 |
| gpu_frame | 53 | 94 |
| gpu_scene_raster | 53 | 50 |
| gpu_reflection | 14 | 43 |
| gpu_prepare | 0 | 42 |
| gpu_shadow | 0 (cached) | 7.2 |
| present_wait | 45 | 36 |
| scene_record (CPU) | 28 | 81 |
| main_prepare (CPU, inside scene_record) | 3.3 | 28 |

The medium and wide views (960×640) follow the same pattern:

- `gpu_scene_raster` is 72–75 ms.
- `gpu_reflection` is 17 ms still and 43–48 ms panning.

**Reading the GPU intervals:** each interval runs from the pass's first
timestamp to its last, so it includes any time the GPU sits idle in between.
In pan, the CPU uploads new buffers in the middle of recording. That makes
`gpu_prepare` and `gpu_reflection` partly measure the CPU. The clean GPU number
is the still `gpu_scene_raster`, and that one is the main GPU cost.

## Fix 1: share mesh geometry across view caches

- **Old behaviour:** each of the four view caches (main, reflection, near
  shadow, far shadow) expanded and uploaded a mesh's vertex streams again
  whenever the mesh entered that view.
- **Fix:** the streams now come from one cache scoped to the packet digest,
  shared by every view.
- **Pixels:** the 18 parity frames have the same capture digests before and
  after, 18/18.
- **Tests:**
  - The camera continuity test asserts that the main and reflection batches
    share the same position buffer.
  - It also asserts that a warm view uploads only the per-instance arrays.

**The result was smaller than predicted.** The prediction was that this would
remove most of the 25 ms `main_prepare` gap between still and pan. It did not:

- Pan `main_prepare` stayed at 27 ms.
- The pan frame p50 went from 120 to 113 ms on close.
- Medium and wide did not improve beyond noise.

Expanding geometry was not the main CPU cost. A CPU profile of 30 pan frames
found the real costs:

- **Per-instance buffers (about half of all samples, in `_mesh_resources!`):**
  when the visible set changes, every changed batch allocates seven new
  per-instance GPU arrays and uploads them. That means a new `LavaArray`
  allocation, then a staging copy, then a flush.
- **Visibility tests (about a sixth of all samples):** `_instance_visible`
  and `_instance_world_bound` recompute every instance's world bound sphere on
  every visibility test. The tests run once per view cache.
- **Both happen twice:** the reflection pass and the main pass each do this
  work.

The still close run of fix 1 also had one spike: p95 268 ms in
`main_raster_record`. The earlier still run did not have it (p95 76 ms), and it
has not been traced.

## MSAA resolves once per draw

CONVERGE-4 renders the scene into an RGBA32F target. The target is 2×
supersampled with 4× MSAA, so the close view is 1536×1024.

Each `Lava.draw!` into an offscreen target opens its own rendering scope. On a
multisampled target, every scope ends with a full-target resolve. The main pass
issues one draw for the sky, one for the terrain and one per mesh batch,
dozens in all. That means dozens of full resolves of about 100 MB read and
25 MB written each. Only the last resolve is ever read.

Resolving once, on the last draw, should keep pixels identical and is the
next fix.

## Where the time goes now

Close view:

| Cost | Still | Pan |
|---|---|---|
| GPU, mostly the scene raster | about 53 ms | about 50 ms |
| CPU recording, `scene_record` (runs before the GPU work, not alongside it) | about 22 ms | about 77 ms |

Next steps, in order:

1. **Resolve MSAA once** per scene pass, and split `gpu_scene_raster` into sky,
   terrain and meshes. Pixel-neutral.
2. **Cut pan CPU cost:**
   - keep per-instance GPU buffers and update them in place instead of
     allocating new ones;
   - cache each instance's world bound per packet. Pixel-neutral.
3. **Overlap recording with GPU work** across frames. This needs safe reuse of
   per-frame resources.
4. **Reduce reflection cost.** The reflection renders at full resolution. Any
   reduction changes pixels and needs a decision first.
