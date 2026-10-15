# Materials and lighting campaign

Status: active, October 12, 2026. User selected this as the sole current
development track. Motion/runtime and broader authoring/application work are
parked except where needed to inspect material/light behavior. The rejected
Heartwood content is not being dressed up as a finished environment.

## Quality target

Luxel must give authors controllable materials, lighting and atmosphere that
can carry the mood, depth, focal hierarchy and environmental storytelling of
a scene. Witcher 3 / Elden Ring remain visual references, not capability names
or numerical quality scores. Geometry and composition still contribute to
storytelling; this campaign does not claim lighting can substitute for them.

Acceptance requires convincing stone, bark/wood, foliage, metal and wet ground
under multiple lighting conditions; useful variation within each family;
coherent key/fill/environment relationships; purposeful local lighting and
atmosphere; and an integrated scene whose images pass visual review. Technical
execution, appearance and cost have separate evidence. No broad parity or
all-style support is inferred from passing the calibration scene.

## Work sequence

1. Reproduce and repair material response defects using analytical controls
   and the existing native calibration scene. Preserve source, packet, receipt
   and raw pixels for fixed-exposure comparisons.
2. Establish material-family and authoring controls where the diagnostics
   demonstrate a gap: physical texture scale, atlas/UV coverage, normal and
   roughness response, two-sided foliage and wet/layered natural surfaces.
   Diagnose content defects independently from shader defects.
3. Develop controllable lighting: environment balance and reflections, bounded
   local lights, shadow/contact behavior and atmospheric depth. Start with the
   exactly-one-directional-light limit in the current native path. Do not
   introduce a speculative universal renderer framework.
4. Prove reusable range in two deliberately different authored lighting moods
   with shared material facilities. Review focal hierarchy, silhouettes,
   value/color separation and depth in the actual native output.
5. Return to the forest settlement as integration evidence once its required
   material/light capabilities execute and look convincing. Close this
   campaign only against the quality target, not a successful render command.

## Existing foundation, retained

`calibration.rs`, `render-calibration`, `tools/build_calibration_glb.py` and
`tools/calibration_sheet.py` already provide a real GLB-to-native-authority
path. Calibration2 includes metal, rough metal, stone, wet stone, bark, wood,
paint, forest floor, emission, grey cards and foliage. Fixed rigs cover sun,
overcast and grazing light, with optional IBL and foliage coverage. Reuse this
work rather than inventing another disconnected material test renderer.

The October 4 reviews accepted basic material readability while leaving bark
quality and variety open. Those historical reviews do not certify today's
renderer, the forest experiment or the requested final visual quality.

## First repair: direct GGX normalization

The native direct-light shader added `0.0001` to the GGX distribution
denominator, both for the base specular lobe and clearcoat. At perceptual
roughness 0.1 this reduces projected NDF area to about 0.0274 instead of 1;
at 0.045 it is about 0.00114. The resulting polished peak can be weaker than
the rougher peak. This is a shader defect, independent of imported assets,
exposure or a scene's artistic direction.

The correction uses the algebraically equivalent stable denominator
`(1 - nh^2) + nh^2 * alpha^2`, with the existing roughness floor of 0.045.
That positive floor makes the additive NDF epsilon unnecessary. Base and
clearcoat share `_ggx_distribution`. The separate BRDF grazing guard,
Fresnel/geometry approximations, IBL, tone mapping, exposure and rigs remain
unchanged. This is not a complete energy-conservation or anti-aliasing repair.
Very narrow specular lobes still need sampling/aliasing review.

The reference is the isotropic specialization of
[PBRT's Trowbridge-Reitz distribution](https://www.pbr-book.org/4ed/Reflection_Models/Roughness_Using_Microfacet_Theory),
with Luxel's existing perceptual-roughness-to-alpha mapping.

`graphics_lab/test/material_response.jl` tests the actual CPU-callable shader
response at an analytically known configuration, for both base and clearcoat,
and independently integrates projected NDF area using Float64 quadrature.
Before correction: 8 passed, 9 failed, exit nonzero. After: 29/29 passed,
including the added normalization checks. The tests are included in the
ordinary Lava adapter suite. Logs are in ignored
`artifacts/material-lighting-20261012/`.

## Native comparison completed

The old renderer was frozen into `before-project` before editing its shader;
`before-source-manifest.json` records its source hashes and the copied
calibration GLB digest. The vendor directory is a shared read-only input.
`capture-pair.py` runs before then after, each under `/tmp/gesso-gpu.lock`,
with the same executable, GLB, rigs, cameras, exposure and material inputs.
The only renderer-source change is the GGX correction. Shader correctness
fixes intentionally change affected default pixels; the original frozen
baseline remains evidence of the earlier renderer, not the expected image
after this correction.

Each arm requests 32 captures: sun/overcast with and without IBL, each with
row, calibration cards, metal, rough metal, stone, wet stone, bark and 5m
foliage views. The calibration CLI now persists its submitted scene packet
and runtime receipt alongside raw RGBA/PPM, certification and attestation.
Grey-card checks remain fixed at their existing tolerance. No exposure retune
is included. The first queued attempt against live sources was canceled
before acquiring the lock so it could be replaced with the frozen-source arm.

Both arms completed successfully: 64 native captures. All 32 submitted packet
pairs are exactly equal. All 16 overcast pairs (with and without IBL) are
bit-identical, isolating the change to direct light. Grey-card measurements
are unchanged and all existing exposure gates pass without retuning. Sunlit
changes are concentrated in highlights, notably wet stone and chrome. The
close calibration view adds 18 clipped RGB channels at the chrome glint;
this reinforces the remaining narrow-highlight sampling concern. Visual
review confirms a localized response correction, not broad scene-quality
parity. Raw evidence and per-frame differences are in `comparison.json`.

## Local lighting: first bounded slice

Implement one directional key plus at most four point lights evaluated at
each fragment's world position. Existing point-light packet fields carry
position, range, linear color and intensity. Use inverse-square attenuation
with a smooth finite-range cutoff and an explicit near-source clamp. Direct
material response is shared; environment and emission must be added once.
Absent points retain the existing shaders, bindings and pixels. Exercise
mesh, alpha-mask/coverage, terrain layers, wet surfaces and reflected views.
Local lights are initially unshadowed: they must not be represented as
occlusion-aware lamps until point-light shadows execute. No volumetric
scattering or new authoring dependency is implied.

Acceptance: analytical falloff/color/additivity controls; malformed and
over-budget light rejection; GPU zero-intensity and out-of-range negative
controls; localized, position-dependent illumination; directional-only
regression; and native warm/cool mood captures with identical materials.


### Implementation and boundary

Point data occupies an owned 9x1 RGBA32F texture at descriptor slot 18: a
count, four position/range records and four color/intensity records. The host
checks the device for 19 sampled-image/sampler slots only on this path.
Twelve typed fragment specializations cover terrain, layers, wet ground,
opaque/cutout/coverage meshes, each with and without cascades. No new varying
components are required. Directional-only packets retain their existing
pipelines and descriptor layouts. Descriptor-cache entries retain the point
texture and sampler they reference.

`intensity` is linear direct-light strength before inverse-square and range
attenuation, using metres. Range fade is `(1 - (distance/range)^4)^2` inside
range and exactly zero outside. The inverse-square denominator is bounded
below by 0.01 square metres to keep the source finite; this is a numerical
near-source convention, not a finite-area lamp model. Point direction and
falloff are evaluated at each shaded fragment, including normal-mapped and
metallic/clearcoat surfaces. Shared PBR evaluation uses zero environment and
emission for each point, so these contributions are added once per surface.
The wet path computes point direct light before its water branch and reuses
it for ground and standing-water composition.

The Rust authority and Julia parser reject more than four points or anything
other than exactly one directional light. Existing numeric/ID bounds still
apply; directly constructed Julia packets are also checked at the adapter.
`local-off`, `local-warm` and `local-cool` are opt-in calibration rigs outside
`CalibrationRig::ALL`. They derive their locations from the bound calibration
ground transform. Warm/cool differ only in point RGB and rig identity;
materials, cameras, light positions/ranges/intensities and exposure are fixed.

### Verification ledger

Rust library tests: 42 passed. Focused lighting/calibration tests: 5 passed,
1 fetched-asset test left explicitly ignored. Julia protocol suite: 65 passed,
including 11 light-admission controls. Strict all-target Clippy and crate fmt
checks pass. A separate saved-contact-artifact test reverified all three
previous CLI views and their corrupted-depth negative controls (1 passed,
837.51 seconds in a debug build); this is audit cost, not renderer frame time.

Development encountered a constructor-field-order failure (narrative retained;
its original smoke log was overwritten), a missing-wrap fixture failure, and
a real SPIR-V dominance failure in the first wet point shader. The source fix
computes point illumination once outside the wet-water branch. A later IBL
fixture incorrectly used the data texture role; correcting it to linear
required no renderer change. Raw variant/IBL failure logs are retained.

The final full adapter suite exited zero: 538 assertions across 22 testsets,
including 14 point-contract checks,
84 analytical/shared-BRDF checks and 45 GPU checks. Those GPU checks cover all
twelve point variants, active-versus-zero controls and IBL composition,
including bit-exact zero-point versus absent-point IBL output. Existing moving
camera (51), cascades (49 GPU) and GGX normalization/response (29) checks pass
in the same run. See `artifacts/local-lighting-20261012/lava-adapter-suite.log`.

All twelve native off/warm/cool mood captures completed successfully. Their
raw RGBA digests match the receipts, and warm/cool submitted state differs
only in packet identity and point RGB. The lossless gallery is at
`D:\Code Projects\Luxel Previews\2026-10-12\materials-lighting\local-lights\index.html`.
Root visually inspected warm row/bark/wet stone and cool bark/wet stone/metal.
Localized bark relief and wet highlights respond coherently, but dark fill
and missing local shadows make objects appear poorly grounded. This is useful
lighting control, not visual parity or a finished scene. The next lighting
milestone is occlusion-aware local lights and useful key/fill balance.

All four native daylight comparisons completed successfully: sun and sun-IBL,
each on wet stone and bark, have exactly equal submitted packets and raw RGBA
pixels against the retained post-GGX/pre-local-light captures. Verification:
`artifacts/local-lighting-20261012/capture-verification.json`. Both native arms
exited zero. The 567.993/527.543-second arm durations include waiting for the
shared GPU lock, initialization and multiple captures; they are not frame-time
or interactive-performance measurements.

Limitations remain explicit: points are unshadowed, have no volumetric
scattering, and do not replace emissive-source geometry. Synthetic wet tests
exercise the reflection/main passes but do not establish the visual quality
of reflected lit meshes in an authored scene. Older wet binding caches retain
packet entries; bounded resource lifetime in long changing-packet sessions
still needs measurement. No interactive frame budget or full materials and
lighting campaign acceptance follows from these checks.
