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

## Where the time goes now

- The GPU takes 48–70 ms per frame (`present_wait`). This is now the largest
  cost. Next step: profile the GPU passes. The suspected costs are:
  - the reflection pass;
  - MSAA on top of 2× supersampling;
  - shadow map rendering.
- About 22 ms of CPU recording (`scene_record`) runs before the GPU work on
  each frame, not alongside it. Overlapping frame N+1's recording with frame N's
  GPU work should bring the frame time down to the GPU cost, about 15–20 fps.
  This first needs safe reuse of per-frame resources.
- The p95/p99 spikes are 2–3× p50, mostly in `scene_record`. They have not been
  investigated yet.
