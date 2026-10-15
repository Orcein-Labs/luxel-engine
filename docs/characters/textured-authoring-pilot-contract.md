# Textured authoring pilot / ART-2 and MOTION-2

2026-10-09. Owner Codex, Luxel character authoring layer. User feedback:
primitive controls work, shared human/bird eye styling does not; restore the
earlier female VTuber direction and cute chibi bird identity. Do not promote a
functional rig to professional visual quality.

Question: can generated, explicitly separated raster pieces replace the vector
control art while retaining native editing, real playback and saved source?
BUILD a bounded texture-backed profile, `luxel.layered-character/v2`, with
embedded PNG atlases and explicit source/destination regions. It remains a
native reference renderer, not Cubism. Keep the existing vector controls as
diagnostics. Atlas crops are runtime views of unchanged generated images, not
image edits or claimed PSD sources. Complete concealed surfaces are an art gate.

Permitted: `character_lab/**`, new sources under `assets/avatars/native/**`,
new generated art/provenance under `assets/avatars/*/source/**` and
`docs/characters/**`, evidence under `artifacts/textured-authoring-20261009/**`.
No GPU scene render, frontend deployment, receipt reinterpretation or Blender
infrastructure changes. Use the built-in image tool for artwork; retain images
and prompts. Inspect output before binding regions. Reject an unusable atlas
rather than treating generated parts as correct by declaration.

Decode bounded embedded PNG data only, reject external URLs and invalid sizes,
regions, references and unknown fields before replacement. Render actual image
pieces under the same affine rig. Source geometry inspection remains vector
only; textured layers expose placement/pivot/rig controls. Do not pretend they
are mesh-deformed or that pixels are editable control points.

MOTION-2: authored demo blinks should be bilateral by default, with short close
and slower reopen, held gaze targets and smooth head/body transitions. Retain
independent wink controls for deliberate expressions. Demo speech is authored,
not audio tracking. No arbitrary simulation or procedural anatomy inference.

Acceptance: numerical/boundary tests for the actual texture-backed project,
real browser load/pose/edit/save/import, successful image decoding, software
screenshots at normal and narrow widths, preserved opacity/occlusion and honest
visual findings. At least one negative mutation must fail a relevant test.
Generated artwork can be rejected while the texture import capability passes;
record these dispositions independently. No professional-quality claim.

Character creator disposition: PLAN a family-scoped recipe retaining stable
part/rig identities, authored choices and bounded proportions. Do not implement
an unrestricted body editor or universal creature generator in this pilot.
The first creator contract follows evidence about interchangeable actual art.
