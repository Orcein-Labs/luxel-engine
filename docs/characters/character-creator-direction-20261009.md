# Character creator: retained direction

2026-10-09. Owner Codex, Luxel authoring layer. PLAN, not an implemented creator
or an implementation contract. Trigger: Matt requested Build-A-Bear-style avatar
assembly and Dark Souls/Code Vein-style character creation, alongside the
current VTuber/chibi quality push.

## Product behavior

Start from a supported family and art direction. Choose compatible face/head,
eyes, hair/crest/ears, outfit, accessories and props. Adjust proportions only
within authored limits. Preview expressions, gaze, bilateral blink, speech
shapes and the relevant body/prop motions as choices change. Save a reproducible
editable recipe with source references, choices, rig bindings and versions.

The same workflow can present a detailed anime woman, chibi bird, feline mascot
or explicitly authored unfamiliar creature without requiring identical eyes,
anatomy or silhouettes. Semantic actions map onto each family's actual controls:
speech might move lips or a beak; secondary motion might move hair or tentacles.
Style is an authored package, not a global filter on one human template.

## What the current pilot earns

Two distinct art directions compose from actual image-tool parts; native
transforms, feature opacity, source placement and saved source work. Ornith has
an independently controlled sunglasses layer. This earns a small layered-art
authoring path. It does not establish interchangeable faces/outfits, automatic
rig transfer, face sculpting or a user-facing character creator. The generated
bird head even includes its crest despite requesting separation: part coverage
must be inspected rather than inferred from prompts.

## First implementation question

Can two actual compatible choices for one meaningful slot survive the same
rig and expression review set without manual repair? Use a single 2D family
first. A closed recipe should name exact source assets, attachment landmarks,
supported shape/parameter ranges, required concealed surfaces, draw/mask rules
and which controls remain valid. A choice with missing coverage must produce a
named compatibility finding or limit, not silently shrink the animation range.
The recipe retains editable layer identity, provenance and source versions;
its saved choices must reconstruct the same result after reopening.

Prove one interchangeable slot, then add face/hair/outfit choices. Do not build
a slot registry, parametric anatomy framework or permanent species-specific
subsystem merely to preserve this idea. The current pilot's next quality gate
is still textured deformation and art cleanup.

## 3D path

The 3D character creator needs authored meshes, material regions, morph bases,
skinning and attachment landmarks. Blender can author those resources; Luxel
owns selection, bounded parameter application, editable recipe identity and
runtime acceptance. A successful 2D portrait assembly cannot create the unseen
back of a head, topology, clothing fit or animation-ready weights.

Evaluate morph combinations with actual skinning and clothing poses, normal/
tangent behavior, bounds, collisions and LOD continuity. Regenerate/fix weights
or restrict a shape using explicit findings when its topology/fit changes.
Share recipe identity and semantic controls with 2D where evidence supports it;
retain separate image/deformer and mesh/skinning execution paths.

## Novel anatomy and quality

Novel bodies require authored anatomy/attachments and a declared movement
envelope. Generation can propose parts and rigs; validation must expose missing
coverage and motion failures. Arbitrary correct-first-try creature generation
remains PARK. Game/cinematic character quality and broad style support remain
the actual goals; an avatar picker alone cannot establish them.
