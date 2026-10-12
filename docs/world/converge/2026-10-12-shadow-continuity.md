# Camera-relative shadows and sparse sun consistency

Continuation of L-1 after the [mesh/overlay camera correction](2026-10-12-camera-continuity.md).
Status: camera-relative shadow correction verified. Sparse before/after and full
adapter regressions pass; cascade quality work remains separate.

## Defect and correction

The renderer fitted directional shadows to the packet camera on first use, then
cached the shadow resources by packet content alone. The presented preview
allows a camera override without changing packet identity. After moving beyond
the original fit, visible geometry could update while the shadow map still
covered the original view. This is separate from the mesh/overlay defect fixed
in the preceding slice.

The adapter now fits shadows from the active `CameraFrame`, comparing the
resulting light frame as well as packet content before reuse. A changed fit
redraws the map while retaining its framebuffer, texture, sampler, and reusable
mesh streams. An unchanged fit returns the resident shadow resources directly.
The whole-world shadow path remains independent of the preview camera.
Reflection and main passes share the main camera's shadow fit; the reflected
view must not replace it with a second fit over a mirrored frustum.

This introduces no packet axis, lighting change, cascade, bias change, or
quality-default promotion. Cold fixed-camera frames should remain identical.
It does not stabilize the fitted map to a texel grid or solve shadow shimmering.

## Sparse control

`graphics_lab/test/shadow_consistency.jl` renders a flat receiver with two widely
separated four-metre prism casters. Each view sees one caster. Matching bare
receiver controls isolate cast-shadow pixels from the caster's own projected
footprint. Two opposed oblique sun directions test directionality.
`tools/shadow_consistency_measure.py` reports a darkness-weighted centroid in
world coordinates and tests its alignment with the packet light's horizontal
ray direction. Its thresholds are a direction cosine of at least 0.85, at least
eight shadow pixels, and a centroid displacement of 1–8 m. These bounded tests
check direction and gross displacement; they do not certify exact penumbra,
full shadow silhouette, dense forest coverage, or near-field shadow density.

The same observed pixels are tested against a deliberately reversed sun vector.
The negative-control process must exit 2 with both directions rejected. A blank
capture cannot count as a successful reversed-direction test. Warm view B after
view A must match cold B byte-for-byte, including the actual presented camera
override. Resource-identity assertions verify reuse and unchanged-view uploads.

Run in a fresh process, taking the shared GPU lock:

```sh
flock /tmp/gesso-gpu.lock julia --project=graphics_lab --startup-file=no graphics_lab/test/shadow_consistency.jl
```

The lock coordinates with the concurrent Gesso work; it is not an engine dependency.
Set `LUXEL_SHADOW_OUTPUT` to separate repeated evidence directories. The
`LUXEL_SHADOW_ADAPTER` override exists for the frozen pre-fix negative control.

## Verification ledger

Evidence directory: `artifacts/shadow-consistency-20261012/` in this isolated checkout.

- CPU render-policy suite: 303 assertions passed, including camera-packet/frame
  equivalence and translated fitted-frustum checks.
- Julia protocol/parser suite: 54 assertions passed.
- Python sparse-measurement tests: 4 passed.
- Python resolution-measurement tests: 12 passed.
- Frozen pre-fix GPU failure reproduction: exit 1, 24 passed / 3 failed /
  0 errored. All failures concern warm-camera-B versus cold-B image or
  measurement equality. Direction controls pass (525 shadow pixels each,
  centroid displacement about 3.108 m, direction cosine about 0.99999).
  The reversed-direction process exits 2 with both directions rejected.
  Root inspected the actual warm/cold and sun-control PNGs.
- Corrected sparse control and presented override: exit 0, all 36 assertions
  pass in a fresh process. Root inspected warm/cold B PNGs: the cast shadow
  is present in both, and their capture hashes match exactly. Reuse assertions
  and both measured direction controls pass.
- Complete adapter GPU regression: exit 0, 199/199 assertions across 14
  test sets, including 51 moving-camera and 36 sparse-shadow assertions.
  Full-suite shadow captures and measurement JSON match the standalone control
  byte-for-byte. Raw log SHA-256:
  `11cfaf9975c74a4b5d2b4d30ad555552d0dd81a79fc86b1be71292267ced6f52`.
- Historical cold scene preservation: fresh native-resolution render exactly matches
  all RGB pixels of the retained October 9 `pre-change-control/wide/native_capture.ppm`
  (960×640). Its cold and warm capture hashes match. This includes fixed-camera
  water; moving reflected-view coverage remains outside the sparse fixture.

An agent's first attempted syntax check executed the test instead of parsing it
and terminated with signal 11 before captures. That attempt is not validation.
Subsequent syntax checks use `Meta.parseall`; GPU jobs take the shared lock.

## Next acceptance frontier

After these controls pass, implement opt-in cascades against L-1's existing
near-camera density gate (at least 20 texels/m within 30 m), retaining distant
forest coverage through 250 m. Measure split transitions, moving-camera
stability, GPU cost and memory separately. Then proceed with depth/contact
occlusion and forest settlement content. Higher resolution has its own
[R-2 attribution](2026-10-12-crispness-attribution.md), so it cannot conceal
incorrect shadows or establish commercial-reference parity by itself.

For the wide camera's 50-degree vertical field of view and 3:2 aspect ratio,
the current sphere-fit arithmetic gives about 8.38 texels/m for 4096 over
250 m. A separate 2048 near map fitted through 30 m would give about 34.93
texels/m, above the existing density gate. These are geometry estimates, not
rendered cascade results. Two maps are therefore a reasonable first experiment;
do not add a third until measured split behavior or cost justifies it.
The current RGBA32F shadow color attachment plus D32 depth uses a nominal
320 MiB at 4096 and another 80 MiB at 2048, before driver allocation overhead.
Measure actual residency before choosing the production formats or budget.

The reference remains the original game's rendering completeness rather than
an unspecified later remaster. NVIDIA's [May 2015 graphics guide](https://www.nvidia.com/en-us/geforce/news/the-witcher-3-wild-hunt-graphics-performance-and-tweaking-guide/)
provides feature-isolated examples for shadow quality, occlusion, vegetation,
water, and antialiasing. These are comparison references, not Luxel dependencies.

## Cascade implementation constraints

A read-only feasibility review identifies the next integration seams:
`ShadowResources`, `_shadow_resources!`, `_render_shadow_map!`, `_shadow_frame`,
and common terrain/mesh `_shadow_visibility` in `graphics_lab/src/LavaAdapter.jl`.
The second map must reach both ordinary materials and layered terrain. Current
layered terrain uses bindings 0–15, including shadow at 5 and IBL at 14–15;
this is the current layout, not a verified device limit. Verify a texture-array
path or deliberately extend/redesign the descriptor layout before choosing it.
Existing varyings from location 21 onward also carry terrain/water inputs, so
new light coordinates and main-view linear depth require an explicit shared
layout across every shader variant.

Use a separate optional Rust-authorized cascade policy, omitted when absent,
with matching strict Julia parsing and explicit rejection of contradictory
single-map/cascade selections. First prove absent-axis packet identity. Fit
and receipt the actual camera's near-map extent; nominal distance/map size
alone does not prove density. A blend overlap may require fitting beyond 30 m,
so recompute density using the final overlap/caster margin. Independently snap
each light-space center to its texel grid with stable extents. Test split
continuity and sub-texel camera motion before judging forest stills.

Keep a typed pair of resources alive, preserve image identity on camera-only
rerenders, and update descriptor caches together on policy changes. Reflection
and main passes must share the maps and the main-view cascade selection basis.
Sparse near/far casters, terrain/mesh/cutout paths, split sweeps, water reflection,
GPU cost/memory, and absent-axis capture identity are required before promotion.
This is a design frontier, not a completed cascade implementation.
