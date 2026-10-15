# Character authoring: three RPDO dispositions

2026-10-09. Lead: Codex. Repository: Luxel. Layer: authoring providers and
native runtime. Scope follows [the experiment contract](authoring-spiral-contract-20261009.md).

## Decision

BUILD provider-backed authoring and native deterministic playback. Blender MCP
is useful authoring control; Blender's Python API and headless jobs provide a
reproducible execution path. Luxel owns identities, intended motion, supported
deformation profiles, validation and runtime behavior. Genuine Live2D remains
a separate Cubism authoring/export/runtime provider. No universal automatic
character generator or arbitrary-anatomy quality guarantee is earned here.

The existing Blender provider constructs a three-bone numerical humanoid and
exports GLB; it does not save an editable project or infer anatomy. The current
female avatar images are flattened references with unresolved alpha/fringe
issues. The local Cyan frontend consumes existing Cubism samples; its Python
preflight is not visual playback. CHAR-CPU-1 evaluates supported skeletal GLBs
numerically; it does not animate the native renderer.

## Spiral A: layered art

Question: is this principally image extraction, or construction of parts that
remain complete under the intended motion?

Candidates: segment the flattened reference; generate each part independently;
author a coordinated layered source; render layers from an editable 3D source.

REJECT segmentation alone as a complete source-authoring method. It cannot
recover unseen cheek, eye, mouth, feathers or garment content. Reject independent
part generation without alignment/composite review: identity, light, outline
and perspective can drift between requests. PARK 3D-to-2D layer rendering until
there is an appropriate 3D source and a GPU lease; it is an alternative art
workflow, not an automatic Cubism rig or guaranteed hand-drawn aesthetic.

BUILD a coordinated part workflow, initially executed with existing art tools.
Use a small project-specific part plan before generalizing a schema. Record
stable part names, source/canvas coordinates, pivot, draw order, mask relations,
intended movement, full hidden regions and provenance. Each part must be
editable and complete beneath overlaps; a crop of the visible surface fails.
Neutral recomposition and extreme-pose review must retain the chosen identity.

Cubism converts prepared PSD layers into ArtMeshes; it does not construct the
missing painting. See [PSD import](https://docs.live2d.com/en/cubism-editor-manual/psd-import/).
Blender can author and animate image planes, but that is a different runtime
path from editable CMO3 and compiled MOC3.

First slice ART-1: use one chosen original design, build a deliberate layer
inventory, then author and review the actual separated source. CPU checks can
enforce path containment, unique IDs, canvas alignment, finite pivots, alpha
integrity and acyclic draw/mask dependencies. Blank, missing or cropped parts
are failures. Paint quality, concealed-content sufficiency and line consistency
remain visual gates, never flags inferred from file presence. PSD import into
the real Editor is its own acceptance step.

Quality/cost frontier: define the desired head/body turn range before painting.
Wider turns demand more hidden art and often alternate views; reduce a declared
range or author the required views, rather than stretching a frontal portrait
until it tears. Adaptive mesh density and atlas layout are later measurable
optimizations after source and deformations work. No throughput gain measured.

Disposition: BUILD coordinated layered source and its checks; PARK native
automated hidden-surface reconstruction. Current artwork remains unaccepted.

## Spiral B: editable rig

Question: can the same deformation machinery support ordinary and unfamiliar
anatomies without baking humanoid assumptions into Luxel?

Candidates: humanoid-only templates; composable chains/branches with explicit
weights; universal inferred anatomy. BUILD explicit authoring with reusable
chain/branch mechanisms and anatomy-specific recipes. REJECT forcing every
subject into human limb names. PARK universal inference pending useful failure
and correction data. Rigify already offers component-based rig generation and
tentacle controls; reuse that ecosystem when suitable rather than duplicating
its authoring system. [Rigify manual](https://docs.blender.org/manual/en/latest/addons/rigify/index.html).

One CPU experiment constructed humanoid, bird and asymmetric creature bone
graphs, with 6, 7 and 8 bones respectively. Each control has genuine two-joint
blend weights where a parent exists, a linear armature modifier and a quaternion
action. Each `.blend` was saved and reopened; parents, vertex groups, individual
weights, modifier target and action/slot were compared exactly. GLBs exported
through Blender's actual exporter and decoded through Luxel's actual CPU API.
No geometry-only preparation receipt participated.

These are small open tube segments with explicit weights, not production
anatomy, connected skin, automatic weighting, IK controls or an attractive
character. They demonstrate that the evaluator needs no human-specific names
for these three cases. They do not prove all branched or nonmanifold anatomies.

For a Lovecraftian subject, first specify visible parts, joint chains, branches,
attachments, limits, contact expectations and topology-changing behavior.
An asymmetric tentacle chain fits the tested skeletal mechanism. A creature
that splits, grows new limbs, flows as liquid or changes topology needs another
declared deformation technique; it must fail by name until that exists. Never
silently replace it with a static mesh or stretch a human rig around it.

First product slice EDIT-1: extend `pipeline/rigging_provider.py` to optionally
retain an editable `.blend` alongside its existing control export. Extend the
existing `tests/test_rigging_provider.py` with a real save/reopen/export check
and pin explicit export time origin. Before implementation, close destination,
overwrite, failure-cleanup and job-isolation behavior in its work-item contract.
Do not promote this experiment script into a universal rig DSL or copy its toy
mesh generator into production character art. Preserve authoring controls and
source project separately from the runtime deform skeleton and baked clips.

Disposition: BUILD editable provider outputs and explicit anatomy recipes;
PARK broad automatic rig inference. Genuine Live2D still requires editable CMO3.

## Spiral C: playback

Question: can correct exported motion be proved without confusing a runtime
file or preparation receipt with animation execution?

The experiment compared Blender dependency-graph evaluated vertices against
Luxel's `decode_character_glb` / `evaluate`, after coordinate conversion. A
one-to-one point matching gate allows export reordering while retaining vertex
multiplicity. It covers 0, .25, .5, .75, 1 and .371 seconds. Frozen-pose controls
must fail the same numerical gate. This validates positions for this profile;
it does not validate normals, tangents, faces, shadows, screen composition or
appearance. The fixture uses sampled quaternion animation and ordinary linear
blend skinning, not preserve-volume skinning or exported live constraints.

BUILD native execution as machinery. Next native product slice PLAY-1 must
explicitly carry pose identity/time and actor state through graphics contracts,
then update positions, normals/tangents, bounds and the passes using them.
Moving-camera resource caching requires an independent fix and same-packet
camera A-to-B comparison against a fresh B render. Start with CPU contracts and
validation; visual acceptance waits for a GPU lease. An agent issuing frame
commands is authoring automation, not an animation runtime.

For the 2D branch, keep two clearly named routes: native Luxel layered-mesh
playback, if implemented, and Cubism SDK playback of genuine MOC3 exports.
The former must not claim to compile/export Live2D. The latter can use the
existing Open-LLM-VTuber consumer first. Speech/blink/gaze/activity map to each
asset's declared controls, allowing beaks, crests, paws and props. The observed
sample's lip-sync parameter is `ParamA`, which already defeats hardcoded human
parameter assumptions. Frontend package checks alone do not evaluate MOC3.

The documented [Cubism external API](https://docs.live2d.com/en/cubism-editor-manual/external-application-integration-api-list/)
can inspect/set parameters and receive export notifications. Its documented
function list does not provide a general model-authoring or batch-export
command. Blender MCP does not fill that compiler/Editor gap.

Disposition: BUILD native skeletal playback and declared 2D provider routes;
PARK visual promotion while Claude has the GPU. No professional-avatar or
Witcher 3/Elden Ring/UE5 parity gate passed in this experiment.

## Experiment results and failures

Evidence is under `artifacts/character-authoring-spiral-20261009/`:
`blender-receipt.json` records editable state and file hashes;
`comparison-receipt.json` records samples, errors and negative controls;
`blender-cli.gate.json` and `rust-build.gate.json` record actual process results.
The standalone Rust probe has an offline-resolved Cargo lock; it did not change
the product workspace's manifest, lockfile or code.

Final Blender 5.2.2 LTS process and Rust build both exited 0. All three
save/reopen comparisons, eighteen pose comparisons and three deliberately
frozen-pose negative controls satisfied their experiment gates.

| Numerical control | Bones | Vertices | Largest component error, Blender units |
|---|---:|---:|---:|
| Humanoid | 6 | 144 | 5.96e-7 |
| Bird | 7 | 168 | 5.26e-7 |
| Asymmetric branching creature | 8 | 192 | 8.32e-7 |

The gate was 1e-5. This is agreement for positions in these small controls,
not a speed benchmark, mesh-quality score or general animation certification.

Initial failures were retained, not counted as passes:

- MCP executes the script without `__file__`; use its documented `args` input.
- MCP headless inherits its server virtual environment, lacking `numpy` for
  glTF export and `cattrs` for an extension subsystem. Read-only environment
  evidence is saved; no plugin installation/configuration was modified.
- Blender defaults exported frame 1 to 1/24 seconds. The initial zero-based
  comparison failed. Explicit `export_anim_slide_to_zero=True` fixes the
  experiment convention; no evaluator or tolerance was changed to hide it.
- CLI exports completed but audio shutdown exceeded 90 seconds, then 30
  seconds. The final job uses a process-local null OpenAL backend, bounded
  numeric-library threads and an artifact-local cache; it exits normally.
- An attempted preview setting used a nonexistent property and failed with
  exit 1. Actual RNA introspection supplied `file_preview_type='NONE'`; final
  jobs disable previews explicitly. The failed script/log are retained.

No scene render or frontend launch was requested. Earlier attempts emitted
thumbnail-write warnings; final jobs disable preview creation. Do not treat
any of these experiments as visual-quality evidence. Provider/MCP repair is a
separate work item from character runtime development.

## Next spiral and order

Proceed EDIT-1 and ART-1 with explicit ownership, then PLAY-1 consuming their
outputs. Use one original female human avatar to expose face/line quality,
then a bird with a foreground laptop to expose nonhuman controls and occlusion;
the branching creature is the stress case. None substitutes for a professionally
authored 3D game character. Do not create one permanent subsystem per creature.

Next load-bearing question: which actual production rig/source construction
survives combined motion without collapsed volume, holes, line breakup or prop
intersection? Numerical agreement establishes a trustworthy comparison path;
it cannot answer whether both Blender and Luxel deform bad source art badly.
