# Luxel development recovery

Recovered 2026-10-12 from repository documents, implementation, and Claude's
closing 2026-10-06 session. This records the continuation order, not a claim
that commercial graphics parity has been reached.

## Product direction

Luxel is a creative engine for worlds, characters, avatars, cinematography,
interactive backgrounds and other applications. Games are one consumer.
Blender and Gaea are the intended external authoring dependencies. Unreal,
Unity, Godot and ComfyUI integrations in the tree do not become required
product dependencies by existing. A UE5-class cinematic mode means a Luxel
quality target; it does not imply an Unreal runtime requirement.

Witcher 3 is the current visual-completeness reference. Elden Ring also sets
an art-direction and composition reference. Photorealistic cinema is a later
quality tier. Multiple art styles remain a product requirement; the existing
[style profile contract](../style-profile-contract.md) expresses style across
geometry, surfaces, lighting, camera and composition. Many of its axes do not
yet execute in the renderer.

An actual Cubism-format Live2D provider has additional format/runtime needs.
The later avatar work explicitly distinguishes a native 2D puppet from a
Live2D export; support for one does not prove the other. The two-authoring-tool
direction does not establish that proprietary Live2D authoring/runtime support
already exists.

## Recovered sources and precedence

| Source | What it establishes |
| --- | --- |
| [Graphics convergence audit](../graphics-convergence-audit.md), especially E and F | Witcher-class completeness requires credible surfaces, atmosphere, foliage, architecture, grounding and motion; Elden Ring also needs authored landmarks, sightlines and value structure. |
| [Landscape parity plan](../landscape-parity-plan.md), October 5 | Gaea owns landform; built erosion outputs are pinned source artifacts. Its opening "nothing built" and proposed ordering are historical, superseded by the implementation notes in CONVERGE-4. |
| [CONVERGE-4 contracts](converge4-contracts.md), October 6 | Accepted backdrop; implemented single-map shadow expansion and metric bias; shadow consistency still under review, cascades pending; crispness then contact AO/depth, then forest scene. |
| [Roadmap](../../roadmap.md) | Layered engine maturation; world remains the active graphics campaign. The source checkout has later uncommitted character updates. |
| Claude session `adfcaab9-8112-4c21-a8f4-c5f2073eb93e`, October 6 closing messages | Shadow review, crispness profiling/resolution/AA, then redwood forest village. Repository cleanup was also listed; it is not a prerequisite for this bounded renderer slice. |
| Source-checkout October 9 IBL comparison receipt and lighting RPDO audit | Later Codex continuation: IBL comparison rejected; moving-camera cache correctness moves ahead of beauty work. These notes are uncommitted in the source checkout and are not Claude's October 6 checkpoint. |
| Source-checkout October 9–10 character and avatar receipts | CPU rig/movement and separate avatar authoring experiments; they do not establish animated native 3D playback or production Live2D support. Ornith rest appearance was accepted and explicitly parked. |

The authoritative source checkout is `/mnt/d/Code Projects/luxel-engine`,
branch `codex/converge4-lighting`, HEAD `8ea9cf1`. Its triage reported 132 dirty
entries before this work. This continuation uses an isolated worktree at
`/home/mattc/rnd/luxel-camera-continuity-20261012`, based on that HEAD; none of
those dirty entries were reset, staged or silently incorporated.

## Where graphics actually stands

- Scanned terrain layers, imported ruin content, alpha-masked foliage,
  deterministic scatter, analytic sky/aerial perspective, still water and a
  Gaea-built mountain backdrop exist.
- The backdrop was accepted for atmosphere and scale, not Alpine landform
  fidelity. Its reported compact packet was 101.8 MB: under the worker bound,
  above the contract's 100 MB goal. The foreground/background contrast ratio
  was not measurable with the then-current masking; no pass can be inferred.
- L-1a expands the single shadow map to 4096 pixels over 250 m and uses a
  world-space bias. It does not satisfy the near-camera 20 texels/m gate.
  The user's shadow-direction concern remains a measurement/review task.
- The later opt-in IBL comparison raised fixed-exposure mean luminance by
  27–32% and was rejected as a quality promotion. Its technical gate passes
  do not overturn that disposition. It must not become the default by accident.
- Current captures are small (960×640, close 768×512). Contact AO/depth,
  cascades, temporal stability, wind, richer water and local lighting remain
  distinct gaps. Cold campaign timings do not establish interactive FPS.
- Visual inspection of the saved October 9 wide comparison confirms repetitive
  vegetation, sparse near-ground detail and weak surface separation. The IBL
  candidate is visibly lighter and flatter. These are observations of the
  saved frames, not a new commercial-reference comparison; resolution alone
  cannot establish the requested quality.

## Continuation order

1. **Camera correctness.** Reproduce stale visibility/overlay resources with
   two separated camera views on one retained packet identity. Compare the
   second view after the first against a cold second view on the real GPU.
   Fix view-dependent reuse while preserving terrain, textures, shadow state
   and reusable mesh geometry. Keep packet authorization unchanged.
2. **Shadow consistency, then cascades.** Use a sparse caster/receiver control
   to measure sun direction and shadow displacement before changing sun or
   bias. Register the cascade slice against the existing near-field density
   gate; review distant forest and camera movement as well as stills.
3. **Crispness.** Attribute resolution, resolve filtering, texture density
   and aliasing first. Add a typed output-resolution axis with fixed-exposure
   comparisons and actual GPU-time/memory costs. More pixels alone do not
   close temporal stability.
4. **Contact grounding.** Depth capture and contact AO, measured after scatter.
5. **Sidhe/Heartwood forest settlement.** Hero redwood trunks and roots,
   appropriate canopy/species and floor cover, settlement paths/bridges,
   moss/wet surfaces, forest atmosphere, flowing water and warm local lights.
   Register these as content and renderer slices rather than a single giant
   engine rewrite.
6. **Moving-world completeness and art direction.** Wind, stable antialiasing,
   animated native characters, time/weather and authored reveals. Compare
   actual moving scenes against the selected quality references before any
   parity claim. Cinematic quality and lightweight wallpaper budgets can then
   use explicit profiles over the same engine capabilities.

Each slice retains absent-axis identity, known-bad controls and provenance.
Render correctness, visual quality and performance have separate evidence;
none substitutes for the other. No Gesso, Cyan or Palette edits are included.
