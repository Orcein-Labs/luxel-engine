# CHARACTER-TOPOLOGY-1 receipt

2026-10-09. Owner Codex, Luxel character authoring. Bounded contract:
`character-topology-1-contract.md`. The editor implementation and functional
gates are complete. Character art remains unapproved; this is one maker
building block, not a professional avatar or a complete character creator.

## Result

`character_lab/topology.html` exposes an editable garment opening. It compares
the rejected whole-hood assembly with a native partition and the supplied
reference. Point dragging/numeric edits, region inspection, five fixed review
poses, undo/reset and recipe/native downloads operate on the actual runtime.
The exported character imports into the existing workbench for playback.

`topology.mjs` accepts a strict source-digest-bound recipe and emits complementary
vector clipping masks plus a second raster view of unchanged artwork. The rear
retains the original part identity; both views/masks retain garment transforms
and bindings. Native character and asset-preparation schemas are unchanged.
The generic operation is tested separately from the Ornith-specific UI preset.

## Evidence

All paths below are under `artifacts/character-topology-1-20261009/`.

- Four topology tests passed: independent winding evaluation of actual compiled
  masks against a rectangle oracle in both orientations; source immutability and
  shared art/bindings; generic named-layer fixture; malformed/stale/order/collision
  rejection. `topology-tests.log`.
- Five existing runtime tests and four existing assembly tests passed.
  `runtime-tests.log`, `assembly-tests.log`. The first sandboxed assembly run
  failed at real subprocess launch with EPERM; the complete suite passed outside
  that process sandbox, without modifying tests.
- Eight grouped real Firefox checks passed. They cover two painted occlusion
  relationships, posed mask motion/captures, actual pointer editing and undo,
  invalid edits preserving accepted geometry, real recipe/native downloads and
  reimports, narrow layout, actual workbench import and no browser errors.
  `browser/browser-receipt.json` and captures.
- At canvas (300,430), the rejected whole foreground hood paints RGB (45,52,67).
  The partition preserves face RGB (253,248,240). At (300,444), the partition
  paints front-lip RGB (19,23,34) over the face. These two samples establish only
  those declared relationships. `browser/occlusion-relations.json`.
- Temporarily changing the production front mask to the full canvas produces a
  nonzero browser exit at the same upper-chin check. The source is restored
  byte-for-byte in a finally block and rebuilt. `negative-control.json`,
  `whole-foreground-control/`, `whole-foreground-control.log`.
- The first new-page run failed with a script syntax error: the offline bundle
  used top-level await inside a non-async wrapper. The topology wrapper is now
  async. This failure was not an art or readiness success.
- Actual complete-page, neutral and opposed-pose captures were inspected against
  the displayed reference. `browser/opening-editor.png`, `neutral.png`,
  `left.png`, `right.png`. The head/beak shape, garment proportions and connected
  arm gesture still do not match the reference; these remain source findings.

Software Firefox uses WebGL off, software rendering and its own temporary
profile/downloads. No existing user tab, GPU or Gesso process is used.
`source-manifest.json` checks original PNG/native sources, guides and the existing
assembly bundle against FIT-3. No Palette core or deployed helper changes.

## Limits and next gate

The hood opening is an editable region boundary, not new painted material,
sewn geometry or a volumetric garment. Polygon antialiasing and source matte
quality can expose seams; no general seamless-boundary guarantee is claimed.
Masks follow affine garment motion; textured mesh deformation, perspective turns
and correct intermediate eyelid/mouth shapes remain absent. The UI currently
authors one Ornith partition; arbitrary imported characters, multiple partitions
and point insertion/deletion are not exposed there.

Do not promote FIT-3 or this proposal as visually accepted. Follow the source
reconstruction gate in `character-maker-rpdo-20261009.md` before rigging polish
or interchangeable character choices. Missing surfaces and connected anatomy
require coordinated authored art; repeated placement changes cannot create them.
