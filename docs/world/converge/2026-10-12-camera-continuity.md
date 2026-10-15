# Camera continuity contract and evidence

2026-10-12. Bounded continuation of the source-checkout October 9 lighting
audit. Scope: Julia/Lava resource selection and its GPU regression tests.
Packet schemas, Rust authorization, shaders, lighting policy and content
defaults are unchanged.

## Failure and correction

The presented renderer accepts a camera override without changing packet
identity. Mesh draw groups were culled on first use and then reused by packet
identity alone. Overlays were also projected only on first use. Later views
could report correct visibility counts while drawing the preceding view's
instances and marker positions.

The correction first reuses unchanged packet/camera frames directly, avoiding
extra grouping work for fixed views. Camera changes check ordered visible
instance IDs within each pass. Unchanged membership reuses the draw resources
with refreshed view metadata; changed membership rebuilds instance
streams, retains unchanged batches and reuses previously resident static
mesh streams where the mesh is still needed. Overlay projection is keyed by
the complete camera frame, including projection and pixel extent. Terrain,
texture and light-shadow resource ownership stays as before.

Caches remain bounded to the existing per-pass resource slots. This is not a
cross-packet geometry cache or a frame-time optimization claim. New packet
content identity prevents all reuse introduced here. Correctness continues
to rely on the existing producer/authority contract for packet integrity.

## Gates

- On one retained packet identity, two disjoint views must render the second
  view byte-identically whether reached after the first or with fresh view
  resources. The original code must fail this control.
- A small camera pan retaining visible membership must keep mesh resources,
  reproject overlays, and match the cold pan capture exactly.
- Empty views must draw no stale meshes or markers. Reflection membership
  must update independently of main-view membership.
- Unchanged views upload zero new resource bytes. Membership changes retain
  existing mesh positions, normals, UV and tangent buffers. Actual upload
  telemetry is reported; GPU-time improvements are not inferred from it.
- New packet content for changed positions, normals, UVs, tangents and indices
  must invalidate old mesh buffers.
- The actual presented `camera_override` entry point must present A then B
  from the original packet; its B resources must produce the cold B capture.
  Window lifetime is closed in `finally`.
- Protocol, render-policy and complete adapter regression suites must pass.
  Compare the original cold fixture against the corrected cold fixture.

Run the complete GPU adapter suite from the repository root:

```sh
flock /tmp/gesso-gpu.lock julia --project=graphics_lab --startup-file=no graphics_lab/test/lava_adapter.jl
```

The shared lock serializes with the concurrently active local Gesso lane;
it adds no engine dependency. `camera_continuity.jl` also runs standalone.

## Execution ledger

Evidence directory: `artifacts/camera-continuity-20261012/` in the isolated
worktree. An initial startup was interrupted before the probe so subsequent
GPU calls could use the existing shared GPU lock. Its log is retained and is
not a validation result.

The unchanged adapter reproduced the bug on the RTX 5060: **0 passed,
1 failed, 0 errored**, with process exit 1. Warm B capture was
`sha256:4dab7d8f56bad1ae7e3cd36c8b51381c1b0db9e6d198d8a3aa9c60ea22a92317`;
cold B was
`sha256:03ad20303d59f1c9ad1e1e35a834050f3f23b4cff1cbee26944d8e2150133435`.
The baseline source and test recipe are retained beside `baseline.log`.

The complete corrected adapter suite passed with process exit 0: **163 passed,
0 failed, 0 errored across 13 testsets**, including **51 camera continuity
assertions**. The actual native window presented A then B using the original
packet and its camera override; the window was closed successfully. The warm
B resources produced the cold B capture exactly. Main-view, reflection,
empty-view, overlay-pan, static-buffer reuse and changed-packet invalidation
checks all passed.

| Capture / transfer check | Result |
| --- | --- |
| Corrected warm B = cold B | `sha256:21ef12b4e3d8ab076402caeb13a4085822ba48bdb77f2b692136363dbcaaaae8` |
| Small fixture B uploads, warm / cold view resources | 2,416 / 2,584 bytes; the 168-byte difference is retained triangle geometry, not a scene-scale speedup |
| Repeated unchanged B | Identical capture, zero uploaded resource bytes |
| Original cold B fixture, unchanged / corrected adapter | Both `sha256:03ad20303d59f1c9ad1e1e35a834050f3f23b4cff1cbee26944d8e2150133435` |
| A after rebuilding view resources | Original A capture reproduced exactly |
| Source checkout preservation | Adapter byte-identical to the baseline snapshot; status still contains 132 entries |

`provenance.json` binds the tested source, tests, manifest and worker to their
SHA-256 digests. `summary.json` records the final totals and cold-capture
comparison. Shader compilation emitted existing upstream Lava/Vulkan warnings;
no compiler, shader or Vulkan dependency changes were made.

These are direct Julia/Lava adapter and presented-entry-point checks. They are
not a new Rust-supervisor/stdio certification or a fresh-worker restart test.
This slice alone cannot certify Witcher 3/Elden Ring parity, camera-relative
shadow-fit behavior, animated characters, or the historical 131-frame campaign
suite. The latter was not run; the fixed synthetic cold fixture establishes a
much narrower output-preservation result.

The render-policy suite initially stopped at 28 passing assertions and one
error: its two `_shadow_uniform` calls omitted the light-frame argument added
by L-1a. The frozen original adapter reproduces that error. The test now
supplies a concrete light frame and also pins the two default shadow slots to
zero; renderer shadow behavior is unchanged. Corrected policy tests: **300
passed, 0 failed, 0 errored**. Protocol/parser tests: **54 passed, 0 failed,
0 errored**. Both suites passed again against the final source revision.
Original failed logs remain in the evidence directory. Documentation links and
diff whitespace passed; hygiene reported zero new violations.

## Resume frontier

Return to L-1 with a sparse caster/receiver scene and measured shadow
displacement against the packet sun direction. Review consistency before
changing sun position or bias. Camera-relative shadow-map fitting is a
separate open behavior from the mesh/overlay repair proved here. Then register
the cascade slice, followed by R-2 crispness and N-8 contact grounding, before
the redwood forest-settlement campaign.
