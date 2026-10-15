# ASSEMBLY-FIT-1 receipt

2026-10-09. Owner Codex, Luxel character authoring. Contract:
`assembly-fit-1-contract.md`. Disposition: retain the bounded fitting tool;
professional visual acceptance remains rejected.

## Delivered

The offline `character_lab/assembly.html` desk presents the supplied design
references beside original and fitted native renders. Editable semantic groups
move complete assemblies with their landmarks and covered rig nodes. Explicit
joint seats and occlusion orders support head/hood contact. Contact distances
and authored proportion ratios are evaluated through the actual runtime.
Guide and fitted-project exports are independent; invalid imports preserve the
accepted composition. Rapid successive edits survive asynchronous refresh.

Ornith's whole head is lowered into its hoodie, with the head pivot seated at
the hood. Matt's correction is binding: this design needs no exposed neck.
Paws meet sleeve-adjacent laptop edge grips. Cyan's eyes are raised relative to
the nose, front fringe shortened and collar raised over the existing skin seam.
These are explicit proposals, not automatic anatomical inference. The guides
are manually authored, source-digest-bound `luxel.assembly-guide/v1` sidecars.
Native character and asset-preparation receipt schemas retain their semantics.

The same solver is exposed by `assembly-cli.mjs`. A selective runnable snapshot
under Palette's exposed stack workspace `tools/luxel-assembly/` records its
Luxel origin and file hashes. It uses the existing Palette operator surface,
without new server registration, backend changes or service restarts.

## Evidence

All paths below are relative to `artifacts/assembly-fit-20261009/`.

- Four assembly tests passed: independent triangle similarity oracle,
  real-source posed contacts, malformed/stale/incomplete rig rejection and
  actual CLI export with overwrite refusal. `assembly-tests.log`.
- Five existing native runtime tests passed. `native-runtime.log`.
- Temporarily forcing contact acceptance produced exit 1 with two failed tests.
  The module was restored byte-for-byte and the suite passed afterward.
  `contact-mutation.log` and `contact-mutation.gate.json`.
- Five grouped software Firefox assembly checks passed, including both subjects,
  edited guide download/reimport, invalid edit/import preservation, rapid edits,
  narrow layout and fitted-source import into the actual workbench.
  `browser/browser-receipt.json`.
- All seventeen existing workbench browser checks passed separately.
  `functional/browser-receipt.json`.
- Live Palette call 699 inspected both source/guide pairs: baseline constraints
  failed, fitted constraints passed, visual acceptance stayed false. Call 700
  exported an actual 808173-byte Ornith native project through the same CLI.
  `palette-inspect-receipt.json` and `palette-fit-receipt.json` retain workspace
  identity and epoch. Output is in the tool snapshot's `results/` directory.
- Original native projects, PNG atlases, runtime and renderer hashes were
  compared with the preceding visual-QA manifest. `source-manifest.json`
  records current hashes and preservation results.

The initial sandboxed numerical run passed three tests and failed the real CLI
subprocess with `spawnSync /usr/bin/node EPERM`. The successful four-test run
used the same code outside that process sandbox. Browser checks likewise used
software rendering with WebGL and hardware video decoding disabled. No GPU or
inference work was run, and Gesso was not changed.

## Visual rejection and next gate

Actual captures were inspected for neutral, blink, partial blink and opposed
poses. Ornith's head seating and sleeve/paw contact improve. Its edge grips
still differ from the supplied mascot's glasses-touch and keyboard gesture:
the necessary raised arm, sleeve and keyboard-contact art is absent. Do not
describe this fit as articulated typing or reference-pose parity.

Partial blinks still dissolve between images, exposing ghosted eye contours.
Cyan retains colored atlas contamination around hair and clothing and lacks
connected neck deformation. Uniform part transforms cannot create perspective
head turns, concealed surfaces or natural textured deformation. Passing guide
constraints leaves `visual_acceptance:false`.

The next closed work item should establish clean, aligned underpaint and real
eye/mouth aperture deformation, plus authored connected arm/sleeve art for one
declared Ornith gesture. Preserve the independent vector controls and require
posed visual evidence. The current desk makes assembly defects inspectable;
it does not solve missing art or establish professional avatar quality.
