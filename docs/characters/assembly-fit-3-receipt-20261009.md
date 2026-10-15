# ASSEMBLY-FIT-3 neckline receipt

**Subsequent user rejection:** Matt reported that Ornith's head reads outside
and behind the hoodie. The visual reading claimed below is withdrawn. The
recorded overlap tests remain valid only for their declared geometric regions;
they did not establish a believable garment opening. Retain this proposal as
rejected evidence. See `character-maker-rpdo-20261009.md` and
`character-topology-1-receipt-20261009.md` for the structural audit and editor.

2026-10-09. Owner Codex, Luxel character authoring. Matt's 17:29/17:30 screenshots
showed that a zero-distance center anchor still left Ornith perched on its hood.
The Cyan collar also exposed a rectangular skin patch. Contract:
`assembly-fit-3-contract.md`. Retain the repair; visual acceptance remains open.

## Fitting changes

Ornith's complete head now seats twenty canvas pixels deeper than FIT-2, a total
of thirty-one pixels below its source placement. The hood remains in front of
the lower head and the head pivot moves with its concealed seat. No exposed
bird neck is added. Paws retain their repaired order behind the laptop.

Cyan keeps the accepted eye, mouth and fringe proposals. The torso/collar now
paints in front of the skin, with the collar another six pixels higher. The
existing rectangular neck underpaint is uniformly scaled to 80 percent and
lowered seven pixels, keeping more of its edges behind the head and collar.
The PNGs and built-in native projects are unchanged; these changes are sidecar
recipes applied by the existing solver.

The new human contact marker is six pixels above the head pivot. Existing head
motion includes three pixels of translation plus roughly 0.42 pixels of rotation
at that marker. Its declared contact envelope is therefore four pixels rather
than three. The three-pixel candidate failed the posed check; that finding is
retained in `collar-motion-envelope-finding/`. Independent rendered neckline
continuity is checked as well, rather than using the wider point tolerance as
evidence of an attractive or connected neck.

## Actual evidence

Paths below are under `artifacts/assembly-fit-3-20261009/`.

- Four assembly tests, including actual CLI export, passed. Five existing runtime
  tests passed. `assembly-tests.log` and `runtime-tests.log`.
- Fourteen grouped assembly browser checks passed. Existing edit/export/import,
  mask isolation and laptop occlusion checks remain active.
- In neutral and both opposed poses, all 161 central columns of the authored
  Ornith contact region have at least eight pixels of face/hood overlap.
  The region is x=220..380 and y=395..470; alpha >=128 counts as painted.
  The acceptance floor is 150 columns. A rendered twenty-pixel raised-head
  control has only 133, 106 and 114 qualifying columns respectively.
  `browser/ornith-neck-*.json`.
- Restoring the actual preceding FIT-2 bird guide makes the rendered width gate
  fail with a nonzero browser exit. The final guide is restored byte-for-byte
  afterward and the bundles rebuilt. `perched-head-gate.json` and `perched-head/`.
- Cyan's isolated face/neck/collar composition has no internal transparent gap
  greater than one pixel on three authored rays, x=294/300/306, y=330..390,
  in neutral and opposed poses. Removing skin and underpaint from the rendered
  control fails those same ray checks. `browser/cyan-neck-*.json`.
- The first missing-skin control reused an image element and retained the prior
  raster despite another decode. It failed the expected-gap assertion and is
  preserved in `missing-skin-control-finding/`. The final check decodes a new
  image for the control; neutral missing-skin gaps are 14/15/15 pixels, versus
  zero for the actual proposal. No readiness delay or retry wrapper is used.
- Neutral and opposed actual images were inspected, including the full comparison
  page. Original/initial proposals and the intermediate neckline studies remain
  available; prior FIT-1/FIT-2 receipts are preserved.

The existing workbench source and renderer were unchanged in this slice; the
assembly browser still imports the fitted native output into the actual
workbench. No GPU, inference, Gesso, schema changes or professional avatar claim.
Selected guide snapshots are refreshed in the existing exposed Palette helper;
the CLI and operator surface are unchanged. `source-manifest.json` records source
preservation and current hashes.

## Remaining limits

Ornith now reads as tucked into the hood in the inspected poses, but still lacks
the supplied mascot's arm gesture and connected keyboard-hand art. Cyan's skin
shading is not consistently authored across separate pieces; the neckline can
still look pasted together. Edge contamination and blended eye/mouth ghosting
remain. Overlap/alpha tests certify only their stated geometric regions.

Next: author a connected neck/collar transition and clean concealed surfaces,
then judge the same pose set against the supplied design references. Additional
point fitting alone cannot create missing shading, anatomy or deformation.
