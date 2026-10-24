# Luxel character workbench

Offline authoring MVP for native vector and textured layered characters.
Run `python3 character_lab/build.py`, then open `index.html` in a browser; it
contains the actual modules and source projects, with no server or network
dependency. The built pages are gitignored.
Node 26.11.1 and Firefox 157.0.1 were used for the recorded checks.

For the fresh original-art Ornith experiment, open `reference.html`.
It preserves the supplied frontal illustration, separates ten visible regions
with editable native masks, and provides isolation, overlay/flicker, difference
inspection, contrasting backgrounds, boundary editing, undo and recipe save/reopen.
Its native export imports into this workbench with playback disabled: concealed
surfaces, independent lower-jaw artwork and animation have not been authored.
Earlier generated pilots below remain diagnostic controls, not the new source.

`reference-author.py` converts the original WebP to pixel-identical PNG and
proposes vector boundaries from source analysis and authored polygons. Running
it replaces the bundled proposal/assets, not exported user recipes. Rebuild
with `python3 character_lab/build.py`; check with
`node character_lab/reference-separation.test.mjs` and
`node character_lab/browser-smoke.mjs --reference`. Evidence lives in
`artifacts/original-ornith-proof-20261010/`. The v2 native part field
`maskSampling` explicitly chooses `pixel` (crisp mask coverage) or `coverage`;
absence retains coverage sampling. This does not change texture filtering or
any preparation receipt schema.

The default choices are Cyan's VTuber art pilot and Ornith's chibi art pilot.
The original vector Cyan and hacker bird remain diagnostic controls.
Pose sliders control independent
blink, gaze, mouth/beak, head tilt, body lean and secondary sway. The bird adds
prop-relative paw motion with foreground laptop occlusion. Connected sleeve and
keyboard articulation are not yet supported by the illustrated art. This is not live
speech/activity tracking. Play or scrub an explicit time. The illustrated
subjects have a twelve-second demo with bilateral blinks, held gaze targets
and smooth head/body transitions; independent wink controls remain available.

Select a layer in the list or preview. For vector art, enable rig/points and
drag control points to edit source geometry. Textured layers expose rectangular
placement/size, pivots and rig controls, not pixel or mesh editing.
Edit vector colors, visibility, draw order and pivots; edit
node parents/pivots and layer/rig control strength. Invalid edits are rejected.
Undo retains the last twenty source edits on the active character; switching
characters clears undo history but retains each character's edits in memory.

Export downloads the complete editable project. Import reads that format and
rejects malformed projects without replacing the active character. Pose sliders
are transient preview state; export retains source and authored clips. Reloading
the page resets its in-memory edits, so export work you want to retain.

## Sources and rebuild

- `runtime.mjs`: bounded decoding, hierarchical affine transforms, signed
  control-point keyforms, scalar clip evaluation, conservative control-hull bounds.
- `renderer.mjs`: SVG composition and moving clipping masks; existing art DOM
  nodes are reused during playback, with inspection overlays separate from art.
- `workbench.mjs`, `shell.html`, `style.css`: actual authoring interface.
- `../assets/avatars/native/*.json`: editable original control projects.
- `author_controls.py`: reconstructs the original sources; running it overwrites
  those two built-in JSON sources. Keep exported user edits separately.
- `author_textured.py`: reconstructs the illustrated source recipes from
  unchanged, generated PNG atlases. Source rectangles were inspected and bound
  explicitly; a generated grid is not assumed to be exact or fully separated.
- `build.py`: bundles modules and current JSON sources into `index.html`,
  `workbench-fragment.html`, `fixtures.mjs` and `review.html`. Run after source changes.
- `visual-review.mjs`: actual software composition captures, bounded rendered
  neck/gaze/accessory probes and deliberately broken controls. No beauty score.

Run `node character_lab/runtime.test.mjs` from the repository root for numerical
and boundary checks. `node character_lab/browser-smoke.mjs` exercises the real
offline page using an isolated headless Firefox, software rendering, WebGL off,
unique temporary profile/downloads and the standard WebDriver BiDi protocol.
It needs Firefox on PATH and creates its evidence directory at
`artifacts/visual-qa-20261009/functional`.
Run `node character_lab/browser-smoke.mjs --visual-review` to capture fifteen
fixed review poses and enforce the bounded assembly probes, or open `review.html`
to inspect the actual current composition beside the female reference. Review
evidence goes to `artifacts/visual-qa-20261009/repaired`. `--baseline` captures
findings without requiring assembly to pass; it does not restore historical art.
Functional and assembly checks never promote overall art acceptance.
In the desktop
process sandbox Firefox was terminated during startup; the recorded successful
check ran outside that sandbox with the same software-rendering restrictions.
It never attaches to an existing user browser/profile.

## Boundaries

This is Luxel-owned reference authoring/playback code, **not** the Vulkan game
renderer, a Cubism reader/compiler, a professional avatar or 3D character rig.
Blender editable 3D output is separately supported by
`pipeline/rigging_provider.py`; its real provider integration suite is
`tests/test_rigging_provider.py`.

Mesh skinning, perspective head turns, automatic
anatomy inference, cloth/physics, audio-driven lip sync and frontend integration
are not implemented here. Data contains structured paths, never executable SVG
markup. The texture-backed `luxel.layered-character/v2` profile embeds bounded
PNG data and rectangular source/destination regions; it rejects external URLs,
invalid dimensions, unsupported mesh deformations and unknown fields. Import
decodes actual image data before replacing active source. Vector v1 remains a
separate diagnostic profile. No asset-preparation receipt schema changes.

The current raster faces use opacity blends between authored open/closed
features and affine layer motion. They do not reconstruct perspective or
deform textured meshes. Semi-transparent transitions may expose doubled
contours; the human neck and atlas edges still need art cleanup. Ornith's crest
is baked into its generated head, so no independent crest sway is claimed.
Numerical validity does not establish attractive deformation, line
quality, concealed-surface sufficiency or professional game/cinematic parity.

Ornith now defaults to sunglasses, with an explicit toggle for uncovered-eye
review. Its pupils share an aligned source and move inside authored eye windows;
the lower beak remains visible at rest. Paws follow the laptop's prop node, but
arm/wing-to-paw continuity still needs authored geometry. Cyan has an explicit
painted neck bridge; it closes the measured gap but leaves a visible skin seam.
## Assembly desk

Open `assembly.html` for the reference, original and fitted compositions side by
side. Choose a group to edit its bounded offset and uniform scale, inspect
landmarks and contact distances, and review neutral, blink, speaking and opposed
poses. Export a fitted native project to import into `index.html`; built-in
sources are preserved. Export a guide separately to retain the fitting recipe.
Guide import is atomic and rejects stale source digests or invalid controls.

The human guide raises the eyes, shortens the front fringe and seats the collar.
Ornith's complete head moves into the hoodie with its pivot at the hood contact;
no exposed neck is required. Paws are fitted beside the sleeves and painted
behind the opaque laptop lid. Visible side portions are a placement study;
the current artwork does not establish an articulated grip.
The reference's glasses-touch gesture and keyboard hand require new arm/sleeve
art. Passing these manually authored constraints does not establish natural
anatomy or visual quality. Both illustrated subjects remain visually rejected,
especially for partial-blink ghosting and contaminated cutout edges.

`assembly.mjs` is the shared CPU solver. Its strict `luxel.assembly-guide/v1`
sidecar is separate from native projects and asset-preparation receipts. Groups
cover complete moved rig subtrees; declared landmarks, joints and draw orders
keep assembly intent explicit. Regenerate the reference guides with
`node character_lab/author_assembly.mjs`, then rebuild with
`python3 character_lab/build.py`. Guide regeneration replaces the two bundled
guides; retain exported edits separately and reassess landmarks after art changes.

From the repository root:

```sh
node character_lab/assembly-cli.mjs inspect assets/avatars/native/ornith_chibi.json character_lab/guides/ornith_chibi.json
node character_lab/assembly-cli.mjs fit assets/avatars/native/ornith_chibi.json character_lab/guides/ornith_chibi.json /tmp/ornith-new-fit.json
node --test character_lab/assembly.test.mjs
node character_lab/browser-smoke.mjs --assembly
node character_lab/browser-smoke.mjs --assembly-regression
```

Fit refuses to overwrite an existing output. Browser checks use actual native
rendering, downloads and imports; evidence is retained under
`artifacts/assembly-fit-20261009/`. The real Palette connector can invoke a
selective snapshot in its exposed stack workspace at
`tools/luxel-assembly/`. That snapshot records file hashes and must be explicitly
refreshed after Luxel changes; no Palette core or model backend changes are needed.

The subsequent FIT-2 repair gives every mounted renderer independent SVG
definition IDs. This prevents one comparison pane's iris masks from clipping
the other pane. It also corrects fitted Ornith's paw/laptop occlusion. The
proposed Cyan proportions are retained following Matt's feedback. Run
`node character_lab/browser-smoke.mjs --assembly --fit-repair` for the repaired
assembly checks, or substitute `--assembly-regression` for the main workbench.
Evidence and deliberate bug reversals are retained separately under
`artifacts/assembly-fit-2-20261009/`.

FIT-3 seats Ornith's full head another twenty canvas pixels into the hood,
retaining the foreground hood and concealed pivot. Cyan keeps the proposed
facial proportions; its collar is foreground, six pixels higher, and its
rectangular neck underpaint is narrowed and concealed. These are fitting
proposals, not connected neck deformation or matched skin shading.
Run `node character_lab/browser-smoke.mjs --assembly --fit-neck` for actual
neutral/opposed-pose neckline checks and evidence under
`artifacts/assembly-fit-3-20261009/`. Bird overlap covers a declared central
region, while Cyan continuity covers three authored rays; neither grades beauty.

Matt subsequently rejected FIT-3's visual result: the head reads outside/behind
the hoodie. Its numerical pass is not visual acceptance. The proposal remains
available as a diagnostic control; its head placement is not promoted further.

## Garment opening editor

Open `topology.html` for the first character-maker building block. It compares
the rejected whole-hood composition with an editable rear hood/front lip and
the supplied mascot reference. Drag polygon points or enter coordinates, inspect
each garment region, and review neutral/opposed, blink and speaking poses.
Undo/reset and source-bound recipe import/export retain authoring intent.
Export a native character to import into the existing playback workbench.

`topology.mjs` partitions a raster layer with complementary native vector masks;
both views reuse the same unchanged texture and garment controls. The strict
`luxel.layer-partition/v1` recipe is separate from character and preparation
receipt schemas. Invalid or stale imports preserve the accepted editor state.
The module supports arbitrary named raster layers, while this initial UI is
an Ornith garment study. It is not a general character picker or anatomy tool.

Run `python3 character_lab/build.py`, `node character_lab/topology.test.mjs` and
`node character_lab/browser-smoke.mjs --topology` from the repository root.
Evidence is under `artifacts/character-topology-1-20261009/`. The browser check
tests actual point dragging, undo, rejection, download/reimport and two painted
occlusion relationships; a production mutation to a whole foreground hood fails
the same render gate. No GPU is used.

The current painted source still differs from the reference. Partitioning cannot
create connected sleeve/hand artwork, concealed material or textured deformation.
