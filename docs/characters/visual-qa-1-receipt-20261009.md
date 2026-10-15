# VISUAL-QA-1 receipt

2026-10-09. Owner Codex, Luxel native character authoring. Contract:
`visual-qa-1-contract.md`. Bounded assembly repairs BUILD complete; overall
visual quality remains rejected. Matt's reported detached parts, divergent
bird eyes and missing accessories were valid findings. Earlier functional
checks did not establish acceptable facial assembly or natural deformation.

## Repairs and measured evidence

Evidence lives in `artifacts/visual-qa-20261009/`. Both review sets use the real
native evaluator and SVG renderer in isolated Firefox 157.0.1 with software
rendering, WebGL disabled and hardware acceleration disabled. Existing atlas
PNG bytes were retained. No GPU workload, inference or game-engine render ran.

- `baseline/review.html` and its fifteen PNGs retain the original assembly.
  Cyan's three declared neck rays had 9–10 pixel transparent gaps across six
  fixed poses. This baseline receipt records successful evidence capture with
  the join gate false, not visual acceptance.
- `repaired/review.html` retains fifteen current captures and the original
  female reference. An explicit body-attached painted neck underlay closes
  all eighteen measured Cyan rays to zero transparent pixels. A narrower torso
  source crop removes the neighboring atlas component strip. Closed/open mouth
  placement was raised and reduced to leave space above the chin.
- Ornith now defaults to glasses. Setting the accessory control to zero still
  exposes the eyes for review. Both pupils use the same highlight orientation,
  are aligned to their individual eye windows and are clipped during gaze.
  Their measured neutral horizontal offsets are equal, about -0.81 pixels;
  left and right gaze produce the same movement in both eyes.
- The lower beak stays visible at rest, compresses into the closed silhouette
  and expands during speech. Explicit dark interior underpaint prevents the
  pale head showing between the opened beak parts. Paws now follow the laptop's
  prop node, preserving that bounded contact under body lean.
- `repaired/report.json` and `browser-receipt.json` record passing assembly
  probes. Deliberately detached head, reversed right-eye gaze and hidden lower
  beak/glasses are correctly detected. The latter controls are actual rendered
  mutations, not assertions about a mocked renderer.
- `functional/browser-receipt.json`: all seventeen real browser checks pass,
  including editing, actual pointer dragging, undo, downloaded source and
  import, malformed import preservation, textured playback and narrow layout.
  `native-runtime.log`: five numerical/schema tests pass.
- `attempts/first-repair-*` preserves a failed check: changing the accessory
  parameter default alone did not show glasses because native channels measure
  displacement from the default. The base accessory opacity was corrected;
  the existing runtime semantics were preserved.

The neck probe isolates face, torso and underlay before measuring alpha. It
does not let hair conceal an absent join. Its scope is three explicit authored
rays in six poses, not all silhouette contacts or continuous motion. Missing
endpoint coverage uses the documented sentinel 100. Ornith's neutral default
shows glasses; all other review poses uncover the eyes so glasses cannot conceal
the blink/gaze defects. Source fingerprints are in `source-manifest.json`.

## Visual inspection and remaining rejection findings

Neutral, fully closed eyes, full speech, midpoint expression and opposing
head/body poses were inspected against the original female design. The current
PNG set is a review fixture, not a professional approval artifact.

1. Cyan's neck underlay removes transparency but leaves a visible rectangular
   tonal seam. It needs connected underpaint and a deformation that preserves
   the neck/collar silhouette rather than merely covering a gap.
2. Both characters still ghost open/closed features during partial blinks;
   Cyan's mouth can also show mixed contours. Opacity crossfades are insufficient
   for natural closure. A coherent aperture/mesh deformation is the next
   capability gate, with explicit midpoint as well as endpoint review.
3. Cyan's hair and clothing edges contain colored and pale atlas contamination,
   and separate hair patches remain perceptible. Source art needs clean alpha,
   consistent skin/lighting and better concealed surfaces. Source rectangles
   cannot establish correct painted silhouette mattes.
4. Ornith's lower beak is now present and connected through dark underpaint,
   but expression semantics remain primitive. Its head/hoodie silhouette and
   paws/wing connection still need continuous authored coverage under motion.
   Keeping paws attached to the prop does not create an arm or solve anatomy.
5. The head still tilts as planar layers. There is no perspective turn, textured
   mesh skinning, facial volume preservation, live speech or full character
   creator. Baked crest, painted laptop perspective and affine body movement
   remain separate limitations.

Disposition: retain these repairs and the repeatable rejection evidence.
Next closed implementation question: can a small actual textured deformer
produce coherent eye closure and neck/collar continuity through midpoint and
opposing poses without doubled contours or exposed background? Author clean
aligned components and concealed surfaces as an independent art gate. Do not
expand creator breadth or claim game/cinematic parity from these tests.
