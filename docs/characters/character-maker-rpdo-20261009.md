# Character maker: structural audit and next spiral

2026-10-09. Owner Codex, Luxel authoring. Matt rejected the repeated Ornith
fitting attempts. The missing capability is coordinated character construction:
placement controls and overlap measurements alone cannot establish connected
anatomy, a garment opening, compatible artwork or attractive deformation.

## What went wrong

The earlier `authoring-spiral-20261009.md` already required coordinated source,
hidden surfaces and neutral recomposition. We did not earn those gates before
advancing the generated atlases into animation. The generated bird head includes
its crest, the garment and hands do not form a connected authored pose, and
several features differ from the supplied design. More uniform translations
cannot supply that missing source content.

The assembly desk supports group placement, scale, draw order and landmarks.
It could put the whole hoodie in front of the face or behind it, but could not
author which region should be in front. FIT-3's overlap gate passed while the
user rejected the visual result. That is a limitation of the oracle and a
process failure, not evidence that the user needs to accept the result.

The runtime already executes native clipping masks. CHARACTER-TOPOLOGY-1 exposes
one useful authoring operation: split one painted layer into complementary rear
and front regions with an editable boundary. This improves representation and
inspectability without new renderer or receipt semantics. It does not approve
the source art or complete a character maker.

## Dispositions

| Candidate | Decision | Evidence or next gate |
| --- | --- | --- |
| More head translation / whole-part order | REJECT as the solution | Prior fits failed user review despite geometric passes. |
| Editable garment front/rear partition | BUILD, bounded implementation complete | Native mask execution, actual browser edits and production negative control. |
| Registered layered art in one coordinate frame | BUILD next | Neutral reconstruction must be visually accepted before rig polish. |
| Connected attachments and concealed surfaces | BUILD after source gate | Preserve continuity through the declared movement envelope. |
| Textured mesh/keyform editing | PLAN | Replace feature crossfades with real authored intermediate geometry; test masks and joins. |
| Compatible character choices | PLAN | Two meaningful slot choices must survive identical poses without manual repair. |
| Arbitrary correct-first-try anatomy inference | PARK | No demonstrated source coverage, attachment or rig transfer guarantee. |
| Blender as universal 2D atlas repair | REJECT | Backend authoring does not recover missing painted anatomy automatically. |

## Next bounded question: registered source and neutral reconstruction

Can one Ornith source set reconstruct the intended mascot pose coherently before
animation? Start with one approved character-only reference; separate its text
and branding from the character artwork. Use layers registered on the same
canvas, with explicit rear hood, front lip, head, crest, glasses, upper/lower
beak, connected sleeves/hands and laptop. Record which concealed surfaces exist
and which movement directions lack coverage. A grid of independent generated
parts is only a proposal, not registered production source.

The authoring surface needs a reference overlay, per-layer visibility, attachment
and occlusion roles, and a saved source recipe. The immediate acceptance result
is the assembled neutral pose and named findings against the reference. Do not
grade it with a single distance, overlap width or automatic beauty score. Do not
introduce expressive motion to disguise rejected neutral art. After neutral
acceptance, review opposed head/body motion, blink, speech and prop contacts,
including intermediate poses where missing surfaces and doubled lines emerge.

This is a research direction, not a closed implementation contract. Close exact
source ownership, permitted art edits, review poses and reconstruction tolerance
before the next source-authoring experiment. Preserve Cyan's previously preferred
facial proportions; her neck shading remains a separate authored-source finding.

## Product structure and efficiency

Luxel owns source recipes, part roles, garment/attachment editing, deformer
controls and native playback acceptance. Palette invokes these tools. Blender
authors suitable meshes, weights and morphs for the separate 3D path. Neither
requires species-specific renderer branches: humanoid, chibi and unfamiliar
anatomy use explicit authored roles and supported motion ranges.

Keep reusable artwork in shared texture resources. The new partition adds three
native parts and two views of the existing texture, without duplicating PNG data
or adding a render loop. Its editor redraws on edits and fixed-pose changes;
there is no per-frame generation, inference or GPU requirement. No performance
benchmark or large-character scaling result is claimed.

Build-A-Bear-style selection becomes useful once parts are actually compatible.
The Dark Souls/Code Vein-style 3D creator additionally needs morph bases, skinning,
material regions and clothing-fit tests. Shared recipe identity and semantic
actions can connect these surfaces, but 2D masked cutouts do not earn those 3D
capabilities. Witcher 3/Elden Ring and cinematic quality remain visual production
goals, not capability labels implied by this prototype.
