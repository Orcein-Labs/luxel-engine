# Textured character authoring pilot receipt

2026-10-09. Owner Codex, Luxel authoring layer. Contract:
`textured-authoring-pilot-contract.md`. Dispositions: texture-backed native
authoring BUILD complete; coordinated demo MOTION-2 BUILD complete; professional
art/deformation quality remains partial; character creator PLAN retained.

## Delivered

The offline `character_lab/index.html` now defaults to Cyan's VTuber art pilot
(16 layers) and offers Ornith's chibi art pilot (14 layers). Original vector
controls remain available as diagnostics. The generated original female design
was the reference for the new human parts; Ornith's supplied mascot was the
reference for the bird. These are newly generated separated art sheets and
authored bindings, not a direct rigging of the old flattened image.

Generated assets, made with the built-in image tool and retained byte-for-byte:

- `assets/avatars/cyan/source/cyan-parts-atlas.png`
- `assets/avatars/ornith/source/ornith-parts-atlas.png`

Exact final prompt set:

- `docs/characters/avatar/cyan-parts-atlas.prompt.txt`
- `docs/characters/avatar/ornith-parts-atlas.prompt.txt`

The prompts request separated art components and transparent backgrounds.
Explicit source regions were bound after inspecting the actual sheets; no
offline pixel editing was used. Source data and provenance are embedded in
`assets/avatars/native/cyan_vtuber.json` and `ornith_chibi.json`.
`character_lab/author_textured.py` reconstructs these source recipes; rebuilding
with `build.py` produces the self-contained page from the same actual modules.

The new `luxel.layered-character/v2` profile carries embedded PNG atlases,
declared dimensions and explicit source/destination rectangles. It validates
bounded data, real PNG dimensions, graph/reference/range correctness and
rejects external URLs and unsupported textured mesh deformation. The browser
decodes actual image data before replacing active source. Raster layers support
placement, size, pivots, parenting, visibility, order and gains. The inspector
does not offer vector control-point edits for pixels. Existing asset-preparation
receipt schemas and the static-extraction import decision remain unchanged.

The twelve-second authored demo has paired blinks, short close/longer reopening,
held gaze targets and smoothstep head/body/sway tracks. Independent wink sliders
remain available. Mouth/beak movement is authored, not speech recognition or
audio-driven lip sync. Raster open/closed features use opacity blends, not
textured mesh deformation. Ornith's sunglasses are a pose-controlled accessory,
not a complete saved-choice character creator.

## Validation

Evidence: `artifacts/textured-authoring-20261009/`.

- `native-runtime.log` / `.gate.json`: 5 tests passed, including actual serialized
  textured subjects, independent eyes, bilateral demo timing, curve oracle,
  invalid PNG metadata/URLs/regions and unsupported deformation.
- `png-dimension-mutation.log` / `.gate.json`: disabling dimension agreement
  caused one failed test and exit 1; source restored byte-for-byte and five
  tests then passed.
- `browser-receipt.json` / `browser.log`: 17 checks passed in actual isolated
  Firefox 157.0.1 with software rendering and WebGL disabled. Includes previous
  vector drag/undo/persistence gates, real textured playback, textured blink,
  a fresh downloaded source and import roundtrip, invalid PNG decode leaving
  active source intact, chibi accessory/beak controls and 360px layout.
- Actual neutral, blink and combined head/gaze/mouth screenshots were inspected.
  `human-textured*.png`, `bird-textured*.png` and `narrow-textured.png` retain
  the review evidence. A neighboring face sliver in the back-hair source crop
  was removed from its runtime region after visual inspection.

A focused inspector check also failed because grid styling overrode the
placement section's hidden state on vector subjects. Its failed browser receipt
is retained as `browser-hidden-fields-failure.json`; the scoped hidden selector
was corrected before the final run.

No GPU scene render, Gesso change, frontend deployment, interactive Blender or
Cubism authoring/export occurred. The earlier Blender provider suite was not
rerun because its source was unchanged in this pilot; its existing receipt is
separate from these five native tests and seventeen browser checks.

## Quality findings and next spiral

The illustrated direction is now represented in the real workbench. It is not
professional visual sign-off. Generated transparent edges and atlas gutters
need cleanup; human hair masses can still show crop boundaries and the neck
join is rough. Feature blends can produce doubled contours in intermediate
mouth/eyelid states. Independent vector iris masks bound human gaze, but this
does not establish realistic eyes or perspective volume.

Ornith's generated head includes its crest, contrary to the requested separated
crest. The extra atlas crest is unused: no independent crest sway is claimed.
The laptop uses a painted perspective while its character layers use planar
transforms; hand/keyboard contact and depth under motion remain limited.
Individual expression frames being attractive does not prove natural motion.

Next quality question: can a textured mesh/deformer and cleaner aligned feature
source preserve volume, line continuity and eye/mouth closure through the actual
combined-pose range? Define a small actual mesh and an independent deformation
oracle, then review midpoint and endpoint frames. Keep complete underpaint,
source alignment and art cleanup as separate acceptance gates. Follow the
creator direction in `character-creator-direction-20261009.md` only after real
part interchangeability earns a closed implementation contract.
