# Cascades, contact, and Heartwood development

Status: implementation in progress after checkpoint `0afcf8e`. These axes remain
opt-in. CPU contracts passing does not establish visual acceptance or Witcher 3 /
Elden Ring parity. GPU and full authored-frame evidence must be reviewed before
promotion.

**Heartwood visual review failed.** The first full authored medium capture
shows mostly bare poles, almost invisible foliage, untextured trunks and a
flat, hard-edged path strip. Successful CPU and CLI checks did not establish
acceptable forest content. Retain these captures as rejected evidence; do not
promote this profile or describe it as a finished forest environment. Review
the reused foliage atlas/UV coverage, crown volume, bark and ground integration
before recapturing. The existing dry ruin test uses a `riverwatch.layout.json`
input filename; that filename does not establish an authored Riverwatch scene.

The checkpoint preserves the active-camera single-map fix and the measured R-2
output-resolution axis. This slice adds three explicit selections to the actual
Campaign 2 CLI:

- `LUXEL_SHADOW_CASCADES=1`: 2048px near map through the 30m split, 4096px far map
  through 250m, with a 27–33m transition. Replaces the single `shadow_fit` policy.
- `LUXEL_CONTACT_AO=1`: deterministic screen-space contact occlusion, 80cm radius,
  strength 0.3, 20mm receiver bias, before exposure and grading.
- `LUXEL_FOREST_PROFILE=heartwood`: opt-in Converge4 forest content. Other arms
  reject this profile. `none` preserves the previous content.

The two switches accept only `0` or `1`; unknown/non-UTF8 values fail before
opening inputs or starting the graphics worker. Absent switches preserve packet
bytes. All selections pass through explicit typed lowering and supervisor
re-derivation, including resolution. A mismatched authorized selection fails
before dispatch. No dependency on Gesso, Cyan, or Palette is introduced.

## Verification underway

New Rust integration tests cover absent identity, scene preservation for policy
axes, typed selection authorization, depth byte/digest/range/identity rejection,
and contact sampling. Existing R-2 frozen digests remain unchanged. Strict
Clippy passes at the current integration checkpoint. Logs live in ignored
`artifacts/converge-next-20261012/`.

Depth is a separate diagnostic accompanying an AO frame, rather than an
extension of the closed RGBA frame payload. Rust verifies its little-endian
Float32 bytes, dimensions, finite positive camera depth, digest and packet
binding. Depth uses minimum positive linear depth per source 2x2 block at
the output resolution. Diagnostic readback occurs after the color frame receipt
telemetry snapshot; its byte count/cost is separate from color-frame counters.
No end-to-end timing acceptance is inferred from those counters. New included renderer modules are separately covered by source
attestation and source-change detection. The original five source keys remain
valid for historical attestations; only the two named optional module keys are
accepted in addition.

The grounding report samples bases of explicitly named rock, tree, fern and
Heartwood understory props. Bases must agree with actual captured view depth
within 15cm in a 3x3 neighborhood. Hidden bases stay unobserved. Observed gaps are
measured against the same triangle-interpolated terrain used for placement.
A base within 15cm counts as contact. Suspended crowns and ruin fragments are
excluded. The report binds both color and depth digests. The Campaign 2 vector
only becomes measured when at least one eligible base is observed, and Rust
re-derives it from depth during validation. This is a geometric contact proxy,
not a judgment of AO appearance or complete interpenetration analysis.

Contact AO darkens the lit HDR image; it is not a physically separated ambient
lighting term. Screen-space occlusion cannot recover hidden off-screen geometry.
Depth/color sample coverage differs at edges and requires explicit diagnostics.
No artistic or timing acceptance is claimed yet.

## Review findings retained

The first forest draft had normalized crown dimensions applied to metre-scale
trunks. Review rejected that draft before visual acceptance; implementation is
being changed to full-size cutout foliage cards with authored kit alpha material
where available. CPU-only species ordering tests did not catch the absolute
scale error, so new tests must cover world-scale crown/trunk overlap.

A Julia malformed-policy fixture using the literal `2048.0` was normalized by
JSON3 to an integer before the field parser. The test was corrected to a truly
non-integral number and the integer helper is tested directly. Authoritative
Rust typed deserialization rejects decimal tokens for these integer fields;
Julia wire-level integral-decimal rejection remains a documented parser limit.

GPU commands must acquire `/tmp/gesso-gpu.lock`. Do not interrupt another
project's owner. Full baseline/individual-axis/combined captures, active-camera
continuity, reflection selection, contact controls, and visual forest review
remain required.

## Focused native verification

The corrected focused cascade test passes 49/49 assertions on the RTX 5060.
It verifies two maps, near/overlap/far shadows, camera refit with retained
framebuffer/texture/sampler identity, zero repeated-view uploads, warm/cold
capture equality, and legacy rendering before/after cascade use. The sparse
functional fixture uses 512/1024 maps; the actual 2048/4096 preset's density
is checked separately by the 107-assertion CPU geometry test. Rotation stability
and an extended moving-water/reflection sequence have not been measured.

Five saved real shadow pairs all pass the strict direction measurement, with
cosine 0.99998 and displacement about 3.14m. Reversed-light measurement exits 2
and rejects all five. Root inspected the near, overlap-center and far PNGs.
The first full GPU test run had 33 passes, one failure and one error because
the fixture named `capture_sha256` instead of the measurement helper's
`render_capture_sha256`. That log is retained; the clean rerun supersedes it.

Contact AO's focused GPU controls pass 17/17; CPU math/depth helpers pass 11/11.
Per-pose AO on/off controls isolate a ground receiver ring outside the box
silhouette: touching reduces summed RGB energy by 156, floating by zero.
Disabled/zero-strength/flat controls match exactly, camera overrides retain
resource identities, and stale depth identity is refused. Root viewed the actual
64px controls; this is a subtle effect, not evidence of artistic parity.

The moving-camera regression also passes 51/51 with the new backend. Initial
AO test runs failed because the standalone fixture lacked a Vec4f import, then
because it assumed an unsupported RenderPolicy keyword constructor. Those
failures are retained in `artifacts/contact-occlusion-20261012/`. The fixture
now shares camera setup through `graphics_lab/test/support/camera_fixture.jl`.

Heartwood's six portable tests plus the asset-backed three-view test pass 7/7.
The final profile uses metre-scale crossed foliage cards and retained authored
alpha materials; replacement matches exact scatter IDs and preserves shared
authored resources. Full authored-scene baseline/individual/combined captures
completed all fifteen views through the real Rust CLI and supervisor. Heartwood
failed visual review and remains rejected; successful execution does not
accept its content quality. Luna workers reached their session usage
limit; root completed the subsequent test fixes and verification directly.

## Authored scene evidence

The real CLI baseline and cascade arms both completed all three views. The
baseline PPM bytes match the frozen October 9 control exactly in close, medium
and wide (`baseline-frozen-comparison.json`). The cascade arm exercises the full
2048/4096 preset, including the authored layered terrain, cutouts and water.
Root inspected all three cascade PNGs: shadows remain coherent and no obvious
overlap seam appears in these stills. This does not replace a rotation or
temporal stability measurement. Mean absolute RGB changes versus baseline are
6.18, 6.57 and 5.02 code values respectively; these quantify a change rather
than artistic improvement.

Cold per-view receipt costs are retained in `cold-frame-costs.json`: baseline
40.39/18.85/20.09 seconds versus cascades 42.46/19.97/19.82 seconds for
close/medium/wide. These paths include cold runtime work and are not warm FPS
or end-to-end timings. Visible-instance counts match exactly; cascade upload
bytes increase by about 47.5 MB. Upload bytes are not peak VRAM residency.

The CLI now persists the runtime receipt referenced by its vector evidence,
alongside the deterministic certification receipt. The baseline and cascade
arms used the executable before this artifact-only addition; subsequent arms
use the rebuilt executable. Both hashes and their applicability are retained
in `issuer-before-runtime-receipt.json` and `issuer-with-runtime-receipt.json`.
Rendering, packet lowering and authority checks are unchanged between them.

`tests/captured_features.rs` provides an ignored, asset-backed revalidation:
set `LUXEL_CAPTURE_ARTIFACTS` to a completed contact/combined output directory
and run the `captured_features` test with `--ignored --nocapture`. It reopens
the saved color, depth, packet and both receipts, re-derives grounding and
validates the vector evidence. A corrupted depth byte must fail verification.
The initial test compilation exposed an Option/String comparison error, now
corrected; final strict all-targets Clippy passes. Logs are retained.


The saved contact-only CLI artifacts were independently reopened on October
12 using `captured_features` (all three views, exit 0). Re-derived grounding
observed 217/704/818 bases from 8,720 candidates and matched the saved evidence;
corrupted-depth controls were rejected. This verifies the saved contact arm's
evidence, not artistic quality or the rejected forest. The combined arm has
not received this separate offline revalidation. Log:
`artifacts/local-lighting-20261012/contact-offline-revalidation.log`.
