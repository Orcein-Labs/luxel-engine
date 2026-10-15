# Ornith: accepted rest appearance, parked

2026-10-10. Matt accepted the fresh original-reference assembly with “That's
perfect” and asked to shelve Ornith while work returns to Cyan. This records
acceptance of the rest appearance shown in `character_lab/reference.html`.
It does not certify blinking, speech, deformation, transparency at all scales,
or professional avatar readiness. No assets were removed and no Git stash was
used. Earlier rejected generated pilots remain diagnostic evidence.

## Resume point

- Workbench: [`reference.html`](../../character_lab/reference.html).
- Immutable reference: `character_lab/references/ornith-front-original.webp`,
  SHA-256 `2c9cd4d85ee65c2f7de8e78b2bf3d02c9628018df1556b09669c4065a47c4f90`.
- Editable separation: `character_lab/reference-ornith-recipe.json`.
- Native source: `assets/avatars/native/ornith_original_front.json`.
- Technical proof: [`original-ornith-proof-receipt-20261010.md`](original-ornith-proof-receipt-20261010.md).
- Evidence: `artifacts/original-ornith-proof-20261010/`, including the source
  manifest, rest comparison, native import and negative controls.

The new Cyan audit rechecked all eighteen files in that source manifest; all
hashes still match. See
[`source-audit.json`](../../artifacts/cyan-live2d-audit-20261010/source-audit.json).
Historical machine reports retain their original `visual_acceptance: false`;
this dated user decision supplements them rather than rewriting recorded runs.

## Deferred work

1. Author actual lower-beak/mouth-interior and concealed-face material, then
   define and test a bounded opening. The existing beak-shadow region is not a
   lower jaw.
2. Author eyelid and underlying face coverage; prove both eyes close cleanly at
   intermediate poses. The rest separation alone cannot do this.
3. Introduce the laptop and connected sleeve/wing/hand artwork as a separately
   reviewed composition. This frontal reference contains no laptop; the older
   laptop pilot is not a drop-in approved continuation.
4. Select a voice and test actual audio-driven playback. Voice suitability was
   not evaluated in this checkpoint. Fixed parameter sweeps or recorded audio
   can test articulation before a final voice is selected.

Head/hood movement also requires concealed material and an authored attachment;
keep it static until that gate passes. Preserve the accepted zero pose in every
later comparison. Continue with the Cyan audit below; Ornith animation is not
an active implementation task.

Next work: [`cyan-live2d-audit-20261010.md`](cyan-live2d-audit-20261010.md).
