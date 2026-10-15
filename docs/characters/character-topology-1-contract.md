# CHARACTER-TOPOLOGY-1

2026-10-09. Owner Codex. Repo Luxel, character authoring layer. BUILD a bounded
garment-opening editor using existing native masks. Matt rejected FIT-3 because
the head reads outside/behind the hoodie. Repeated placement patches are not an
acceptable substitute for a character maker.

## Research disposition

The current assembly tool can move groups, assign whole-part order and measure
points/overlap. It cannot author a garment opening or divide one painted layer
into front/rear regions. The runtime and renderer already execute nested vector
clipping masks and editable native paths. A single hoodie part cannot be both
behind the head at the rear hood and in front at the small front lip. More
head translation does not add this missing distinction.

BUILD an explicit partition authoring operation, not new renderer semantics.
REJECT overlap width as an appearance oracle. Keep FIT-3 and its evidence as a
rejected proposal rather than silently overwrite or promote it. The earlier
coordinated-art/neutral-recomposition plan was not fulfilled by generated atlases
plus numerical fits; record that process failure and the broader maker direction.

## Exact slice

A strict source-digest-bound `luxel.layer-partition/v1` recipe names one existing
raster part, the subject layers it surrounds, front/rear draw orders and one
simple polygon selecting the front region. Compile complementary native masks
and two raster views of the unchanged source. Reject stale source, invalid,
self-intersecting/degenerate polygons, ID collisions and invalid order brackets.
Retain the original part identity for the rear piece. Preserve transforms and
bindings so masks follow the garment through runtime poses. No new character
or preparation receipt schema or implied deformation readiness.

The offline editor shows current assembly, editable partition and supplied
reference; supports point dragging/numeric edits, region inspection, fixed
neutral/opposed poses, undo/reset, recipe import/export and native export usable
in the existing workbench. Import failure preserves accepted state. Use one
Ornith garment example; generic partition execution must also pass an independent
synthetic fixture. This is the first maker building block, not a finished creator.

Permitted: new character_lab topology module/UI/shell, build.py, existing browser
driver and new focused topology tests, documentation and evidence under
artifacts/character-topology-1-20261009. No original PNG or native-source edits,
no GPU/Gesso/Palette core changes, no global reset/cleanup or unrelated edits.

## Gates and limits

Independent geometric membership oracle and malformed-input rejection; actual
render proves two different occlusion relationships at declared painted regions;
deliberately collapsing the partition to one whole foreground layer must fail
that same render check. Verify export/reimport, actual point editing and posed
mask alignment in a software browser. Inspect the complete page and captures
against the supplied reference. Pixel checks certify only stated relationships.
No claim of reference identity, beautiful art, mesh deformation, automatic
garment inference or a correct-first-try universal character generator.

The maker roadmap must distinguish coordinated layered source, region/attachment
authoring, textured mesh/keyforms and recipe compatibility. Missing content,
arm/sleeve continuity, facial ghosting and mismatched shading remain explicit
art/deformation findings. Blender remains an authoring backend for suitable
3D sources, not an automatic repair for a faulty 2D atlas.
