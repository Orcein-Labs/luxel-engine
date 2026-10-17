# Presented frame rate on CONVERGE-4

2026-10-16 · luxel-consolidation branch

The live window now runs CONVERGE-4 at 10–13 fps, up from 1. Three changes
did it:

1. Caching mesh bounds and transform keys.
2. Keeping the scene packet resident in the worker.
3. Presenting with a GPU image blit instead of a CPU readback.

Four later fixes, each pixel-neutral, brought the still close view to
21–23 fps (from 13.6) and the moving camera to 10.8–12.7 fps (from 7.6). See
[Fix 1](#fix-1-share-mesh-geometry-across-view-caches) through
[Fix 4](#fix-4-record-the-next-frame-while-the-gpu-runs-the-last-one).

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
that did not run in a frame reports 0. The `present_wait` stage in the tables
below and in the results table above existed until fix 4, which replaced it
with `previous_frame_wait`.

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

## Fix 2: resolve MSAA once per scene pass

CONVERGE-4 renders the scene into an RGBA32F target. The target is 2×
supersampled with 4× MSAA, so the close view is 1536×1024.

Each `Lava.draw!` into an offscreen target opens its own rendering scope. On a
multisampled target, every scope ends with a full-target resolve. The main pass
issues one draw for the sky, one for the terrain and one per mesh batch,
dozens in all. That means dozens of full resolves of about 100 MB read and
25 MB written each. Only the last resolve is ever read.

The fix resolves on the last draw only:

- Lava's offscreen `draw!` takes `resolve=false` to skip the resolve.
- The scene pass resolves only on its final draw: the last mesh batch, or the
  terrain when no batches are visible.
- `gpu_scene_raster` now has three nested intervals: `gpu_scene_sky`,
  `gpu_scene_terrain` and `gpu_scene_meshes`.
- **Pixels:** the 18 parity frames match the fix 1 digests, 18/18.

p50 in ms, fix 1 → fix 2:

| View | Still frame | Still `gpu_scene_raster` | Pan frame |
|---|---|---|---|
| close | 71.5 → 64.1 | 53.8 → 43.7 | 113 → 115 |
| medium | 90.6 → 70.8 | 71.6 → 53.4 | 140 → 122 |
| wide | 93.0 → 74.5 | 74.6 → 57.9 | 144 → 126 |

On the close view, the sky takes 0.3 ms, the terrain 1.8 ms and the meshes
41.6 ms. Mesh draws are now 95% of the scene raster. The close pan frame did
not move because the CPU bounds it: `scene_record` takes 86 ms.

**Lesson: an open timing slot hangs the worker.** A first draft of this fix
began the sky interval and never ended it. The timestamp readback waits for
every query with `QUERY_RESULT_WAIT_BIT`, so the worker spun at 100% CPU for
hours while holding the GPU lock. Every GPU test run is now wrapped in
`timeout`.

The adapter now guards against this. It records every pass interval it begins,
and removes it when the interval ends. If any are still open when the frame's
timestamps are read, the read raises `gpu_timing` and names the open labels.
It no longer waits on the driver forever. `present_blit_gpu.jl` tests this
with a deliberately open interval.

## Fix 3: cut the moving-camera CPU cost

The fix 1 profile pointed at two pan costs. This fix removes both.

- **Instance bounds:** each instance's world bound sphere is now computed once
  per packet and shared by every visibility test.
- **One upload per view:** Lava flushes the batch and waits on it after every
  upload. A view that gained several batches used to upload seven arrays per
  batch, and each upload waited on all GPU work recorded so far. Now all the
  new batches' per-instance arrays go up in one upload, and each batch reads
  its arrays as views into that one buffer.
- **Upload before recording:** the main and reflection view caches are built
  before any pass is recorded. The uploads then no longer wait on the
  reflection pass.
- **Pixels:** the 18 parity frames match the fix 2 digests, 18/18.
- **Tests:** the camera continuity test builds a view with two new batches. It
  checks that every per-instance array shares one buffer, and it downloads the
  first and the last batch to confirm each view reads its own data.

p50 in ms, fix 2 → fix 3:

| View | Still frame | Pan frame | Pan `scene_record` | Pan `gpu_reflection` |
|---|---|---|---|---|
| close | 64.1 → 54.5 | 115 → 81 | 86 → 40 | 45 → 25 |
| medium | 70.8 → 64.8 | 122 → 85 | 84 → 36 | 46 → 25 |
| wide | 74.5 → 67.3 | 126 → 88 | 87 → 38 | 48 → 26 |

On the close pan:

- `main_prepare` went from 29 ms to 0.4 ms.
- `gpu_prepare` went from 46 ms to 7.1 ms, which is now just the shadow pass
  (`gpu_shadow` is also 7.1 ms).

On the close still view, `main_prepare` went from 3.3 ms to 0.4 ms, and
`scene_record` from 27 ms to 15 ms.

## Fix 4: record the next frame while the GPU runs the last one

- **Old behaviour:** each presented frame waited on its own GPU work before
  returning (`present_wait`, 39–50 ms). The CPU then recorded the next frame
  while the GPU sat idle.
- **Fix:** a presented frame now returns as soon as it is submitted. The window
  session keeps its timeline value. The next frame:
  1. records a full memory barrier, so its GPU work starts only after the
     previous frame's;
  2. records its scene;
  3. waits for the previous frame (`previous_frame_wait`), then acquires the
     swapchain image (`acquire`), records the blit (`blit_record`) and presents
     (`present`).
- **Other entry points drain first:** `render_scene`, the three probes,
  `capture_scene_depth` and `close_window_session!` all wait for the frame in
  flight before they reuse its resources.
- **Timing is now every other frame.** All frames share one timestamp pool. A
  frame is timed only when no timed frame is still in flight, and its report
  arrives with the next frame. A 30-frame run therefore has 15 `gpu_*` samples.
- **Pixels:** the 18 parity frames match the fix 3 digests, 18/18.
- **Tests:** `present_blit_gpu.jl` checks the alternate-frame report counts. It
  also checks that `render_scene` and `render_probe` clear the in-flight frame.
- **Not checked:** Vulkan sync validation. The Khronos validation layer is not
  installed on the benchmark machine.

p50 in ms, fix 3 → fix 4. Batch fps is frames over the whole run's wall time,
so it is the honest throughput number. Two fix 4 runs gave these ranges:

| View | Still frame | Still batch fps | Pan frame | Pan batch fps |
|---|---|---|---|---|
| close | 54.5 → 43.6–45.6 | 17.7 → 21.4–23.2 | 80.7 → 65.2–66.4 | 11.0 → 10.8–12.7 |
| medium | 64.8 → 53.5–55.8 | 15.1 → 12.3–16.7 | 85.0 → 79.8–80.7 | 11.2 → 11.7–11.8 |
| wide | 67.3 → 56.0–59.4 | 13.9 → 15.8–16.3 | 87.7 → 83.3–83.7 | 10.7 → 11.4–11.5 |

- **Still:** the frame time now equals `gpu_frame` on every view. CPU recording
  is fully hidden behind the GPU.
- **Pan:** the p50 dropped, but throughput barely moved. `scene_record` grew
  from 36–40 ms to 64–82 ms because it now absorbs the wait. Every upload
  that still happens during recording flushes and waits on all GPU work in
  flight, and in flight now includes the whole previous frame. While
  the CPU waits, the GPU idles. The pan `gpu_scene_raster` p95 is 144 ms
  against a 42 ms p50; those idle gaps sit inside its timed intervals.
- **Spikes, not yet traced:** each fix 4 run had one isolated long frame, in a
  different view each time: 478 ms (pan close) and 560 ms (still medium). The
  single fix 3 run peaked at 170 ms. A frame that long drags down that view's
  batch fps, which explains the low end of the ranges above.

## Where the time goes now

The still frame is bound by the GPU alone. The close view takes 44 ms:

- **Mesh draws:** about 41 ms of the scene raster on the close view. This is
  the largest single cost.
- **Reflection:** 14 ms still, 21–26 ms panning. The reflection renders the
  scene again at full resolution.
- **Shadows:** about 7 ms panning. The shadow cascades are cached when the view
  is still.
- **Pan uploads:** each Lava upload flushes and waits for the previous frame,
  so a moving camera still runs CPU and GPU in series.

Next steps, in order:

1. **Uploads that do not wait.** A freshly allocated buffer has no GPU readers
   or writers, so its upload needs neither a flush nor a wait. Record the
   staging copy into the active batch and free the staging buffer through
   Lava's timeline-gated deferred free. This is Lava sync work, so it needs
   its own hazard review. Pixel-neutral.
2. **Trace the isolated long frames** with per-frame samples.
3. **Reduce reflection cost** behind the graphics quality setting, for example
   a half-resolution reflection. The top preset keeps today's pixels.
4. **Cut mesh raster cost.** The next step is to find out whether the mesh draws
   are bound by vertex work, fragment work or draw count.
