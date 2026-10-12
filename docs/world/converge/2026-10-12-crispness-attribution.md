# R-2 crispness attribution measurement

Status: typed output-resolution control and bounded still-image analysis implemented.
CPU compatibility and authorization checks pass. Direct adapter captures and
timings are recorded below; real Rust-entry-point acceptance remains open.
No quality default is promoted.

`tools/crispness_measure.py BASELINE CANDIDATE` compares two still captures over
the same full-frame normalized image domain. The captures must have the same
aspect ratio. It downsamples the larger raster with Lanczos to the smaller
raster and reports linear-luminance MAE/RMSE/max difference, edge magnitude,
edge fraction above a fixed luminance-gradient threshold, and Laplacian RMS.
It also reports the same descriptive edge/detail metrics on each native raster
alongside its dimensions. Those native metrics are scale-dependent diagnostics;
greater edge or Laplacian energy can come from resolved detail, sharpening,
noise, or aliasing and is not a quality result. A still comparison cannot
establish temporal stability.

For scene-backed attribution, pass both `--baseline-packet` and
`--candidate-packet`. The tool performs minimal wrapper/schema checks and checks the packet's camera and capture
dimensions against the image files, confirms capture/camera identity, requires
a deterministic capture, and requires the same fixed `environment.exposure`.
It computes a scene-identity digest after removing only packet/capture
identifiers and output dimensions; all other packet content must match. This
means the supplied scene fields match under that rule. It does not validate the
Rust packet seal or prove that either image was produced from its supplied
packet; the report marks both bindings `not_verified` and hashes the packet
files as inputs. Without packet files the report marks scene and exposure
provenance unverified. The tool does not register images, correct crop/framing
differences, or adjust exposure.

## Current capture seam

The native graphics contract already carries output size as typed data:
`GraphicsCamera.width_px/height_px` and `GraphicsCaptureRequest.width_px/height_px`
in `world_core/crates/native_graphics_contract/src/lib.rs`. Rust
`validate_capture` requires the capture dimensions to equal the camera and
checks the frame byte bound. The Julia receiver mirrors this contract in
`graphics_lab/src/LuxelGraphics.jl`: `_parse_camera` and `_validate_capture`
parse and cross-check both pairs. The rendered camera aspect and attachments
then flow through `graphics_lab/src/LavaAdapter.jl` from the packet camera and
capture dimensions.

Rust owns campaign lowering. `Campaign2LoweringInputs` carries the typed R-2
resolution selection into `lower_campaign2_packet_with_resolution_inputs`,
which applies it to one fixed view before packet validation/sealing. The
supervisor re-derives the packet with the same selection before rendering.
Do not compare different campaign views as a resolution experiment: their
camera poses and fields of view differ. Preserve the aspect ratio so the view
framing stays fixed.

The Julia target is the matching `LuxelGraphics` packet boundary and the
`LavaAdapter` render path that allocates/sets the output raster. If Rust lowers
the selected size into the existing camera/capture fields, there is no need for
a Julia command or a new serialized policy field; Julia still validates and
executes the exact dimensions Rust authorized. Confirm the output image size
and the frame receipt against those packet dimensions on the real entry point.
Report actual GPU frame cost, memory, and packet-size impact separately from
these image metrics, as the R-2 contract requires.

The Rust `render-campaign2-layout` entry point accepts `LUXEL_CAPTURE_SCALE=2`
to double both dimensions of each fixed view while preserving aspect ratio.
Unset or `LUXEL_CAPTURE_SCALE=1` keeps the current dimensions; every other
value is rejected before layout loading or worker startup. The selection is
lowered into the existing camera and capture dimensions and is omitted from
the scene packet schema.

## Identity and compatibility constraints for that axis

- Keep the axis typed and Rust-authoritative. A resolution choice must produce
  matching camera and capture dimensions and retain the existing capture
  format, deterministic flag, fixed exposure, camera pose, world, and scene.
- The baseline/default axis must keep today's per-view dimensions and produce
  byte-identical baseline packet/capture evidence when absent. Prefer lowering
  the typed selection to the already-required width/height fields, leaving no
  extra serialized key when the axis is absent.
- If a serialized field is later required, Rust's strict packet schema and
  Julia's `_exact_keys` allowlist must change together. Omit the field when
  absent so canonical packet bytes and the packet digest remain unchanged;
  version the schema only if the accepted payload contract itself changes.
- A selected non-default resolution legitimately changes packet and capture
  digests. For comparison, retain those original digests as provenance and
  derive a separate scene-identity digest that excludes output dimensions and
  per-capture IDs while still binding every other scene and camera field.
- Reject mismatched capture-to-camera/image dimensions and mismatched scene,
  camera framing, or exposure before attributing any image metric to
  resolution. Keep absent-axis known-good captures as the control.

## Verification ledger

The new `resolution_axis` integration target passes four CPU tests. An independently
built pre-change source revision (`f8d10bf`) supplies the expected packet digests
for close, medium, and wide views. Both the legacy lowering call and explicit
`Native` selection match those digests. `Scale2` changes only output dimensions
and consequent packet identity. A worker-protocol fixture proves that authorization
rejects a scale mismatch before dispatch; the matching selection reaches the
worker capability gate. These tests do not render a GPU frame.

The measurement helper has twelve focused tests, including changed camera pose,
exposure, render policy, unknown scene fields, dimensions, and schema rejection.
Its packet digest is a recorded claim, not a validated seal; image-to-packet
binding remains explicitly unverified.

Evidence is retained under `artifacts/crispness-20261012/`. The queued direct
Julia probe compares cold and warm captures at native and doubled dimensions,
recording frame timings and uploads. Its derived high-resolution packet bypasses
Rust authorization and cannot establish acceptance of the Rust entry point.
The first attempted source was an older `wge.graphics-scene-packet/v6` artifact;
the current receiver rejected that schema before rendering. That failed attempt
is retained separately and the new probe uses a current Luxel packet.

Full package formatting has a pre-existing backlog. Scoped formatting and strict
Clippy results belong to the separate native-graphics maintenance receipt.

The first current-schema probe rendered the native frame with matching cold/warm
hashes and all RGB pixels identical to the retained October 9 control. It then
stalled while hashing the approximately 196 MB source JSON string for its report.
After about eleven minutes the process was interrupted to release the shared
GPU slot; no scale-2 capture or complete timing summary exists from that run.
The interrupt stack points to `SHA.update!` through Julia array alias checks and
`objectid`, before JSON formatting. A bounded retry hashes copied byte buffers
instead of the large string and has its own evidence directory (`retry/`).
Neither the interruption nor a queued retry counts as a completed comparison.

## Direct adapter observations

The byte-buffer retry saved both complete capture pairs and its summary. Each
scale's cold/warm hashes match, with zero warm uploads, 113 draw calls, and the
same 6,164 visible instances. The native RGB pixels again match the historical
control. Root inspected the actual 1920×1280 image: foliage/ruin details resolve
more clearly, while sparse floor cover, repeated trees and weak grounding remain.
No quality-default promotion follows from this still-image experiment.

| Recorded warm sample | Native | Scale2 |
| --- | ---: | ---: |
| Raster | 960×640 | 1920×1280 |
| GPU frame time | 74.810 ms | 153.343 ms |
| End-to-end adapter frame time | 771.639 ms | 1498.362 ms |
| Readback bytes | 2,457,600 | 9,830,400 |
| Source texture payload bytes | 36,196,472 | 36,196,472 |

These are one warm sample per setting, not a distribution or interactive FPS.
Peak device memory and total attachment residency were not measured. Texture
payload bytes are source data telemetry, not VRAM residency. The helper reports
normalized linear-luminance MAE 0.006849 and RMSE 0.012055; those describe pixel
differences and do not score quality. Packet inputs were not supplied to the
helper, so its provenance is explicitly unverified; the scratch summary separately
binds the source file and adapter digests. The Scale2 packet is still derived
directly in Julia and not certified through Rust's promotion entry point.

The retry command returned status 143 after writing both captures and the
complete summary. Its log has no final Julia exception or explicit completion
marker. The cause of that process termination is unresolved; do not call this
a clean process pass. Complete capture-level observations above survive, but
R-2 acceptance remains open, including clean shutdown, the real Rust entry
point, repeated timing samples, memory, and moving-camera aliasing.

A subsequent fresh process (`clean-exit-control/`) completed with an explicit
`CAPTURES_AND_REPORT_COMPLETE` marker and exit 0. Both PPMs and capture digests
are byte-identical to the prior retry. Its warm GPU samples were 77.534 ms
(native) and 152.744 ms (Scale2); end-to-end times were 829.156 and 1392.702 ms.
Thus there are two recorded warm observations per setting, still insufficient
for an interactive-performance claim. The earlier termination remains unexplained
and retained, but did not reproduce. Clean direct-adapter completion is now
proved; Rust-entry-point certification, peak memory and temporal acceptance
remain open. Production/test source hashes are unchanged since the reviewed
and tested revision.

Before committing this checkpoint, Rust package formatting was applied and
strict Clippy plus the focused suites were rerun successfully. The independent
pre-change packet digests still match. This supersedes the earlier statement
that package formatting remained blocked; the original failure log is retained.
