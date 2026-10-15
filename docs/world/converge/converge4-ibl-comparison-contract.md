# CONVERGE-4 IBL comparison

2026-10-09. Owner: Codex. Repo and layer: Luxel native graphics.
Status: implementation and comparison complete; quality promotion rejected.
See `converge4-ibl-receipt-20261009.md`. The experimental arm remains on the
uncommitted feature branch for reproduction; no default or quality promotion.

## Question and disposition

Does the already-implemented image-based lighting improve the CONVERGE-4 vista?
BUILD one explicit experimental comparison arm. Retention is a later decision
from actual renders, not implied by wiring the arm or passing packet tests.

Evidence: CONVERGE-1 accepted IBL in calibration, but CONVERGE-4 inherits a
policy with `ibl` absent. The saved `ab-converge4-n3c/medium` packet also omits
it. CONVERGE-3's closeout explicitly records that its campaign surfaces still
use the constant-ambient branch. No new lighting algorithm is required here.

This is a bounded lighting probe alongside the open L-1 work, not a replacement
for the recorded L-1 -> R-2 -> N-8 -> Sidhe sequence.

## Behavior

- Add an explicit `converge4-ibl` experimental policy candidate.
- It uses exactly CONVERGE-4's terrain, kit2, backdrop1, camera, lights,
  atmosphere, shadow fit, foliage coverage, and exposure.
- Its only authored policy difference is `ibl: { enabled: true }`.
- Use the existing deterministic Rust IBL bake after all content, environment,
  and policy changes and before sealing. Carry its existing digest-bound
  textures and run existing validation; never bypass promotion authorization.
- The supervisor must reconstruct this same candidate from explicit campaign
  inputs. Changing packet bytes after authorization is forbidden.
- `converge4` and every earlier candidate retain their previous packet bytes.
- The candidate is experimental and does not become the default.

## Permitted files and ownership

- Rust worker: `world_core/crates/native_graphics_contract/src/lib.rs` and
  `world_core/crates/native_graphics_contract/tests/converge4.rs`.
- Lead: this recipe, `tools/parity_ab_policy.sh`, and a small run receipt under
  `docs/world/converge/` if useful.
- Read-only supporting seams: `src/ibl.rs`, `src/supervisor.rs`,
  `src/calibration.rs`, existing IBL tests, and Julia/Lava implementation.
- If a required change falls outside this set, report the concrete dependency
  to the lead before expanding ownership. No simultaneous edits to one file.

## CPU gates

1. New policy differs from CONVERGE-4 only in enabled IBL; prior candidates
   still omit it. Asset requirements and arm parsing remain explicit.
2. Lower a complete candidate through the real campaign producer with built
   kit2 and a synthetic backdrop. The packet passes validation and carries the
   existing IBL resources. After removing IBL policy/resources and normalizing
   view identity, its content equals the baseline packet.
3. A corrupted or missing IBL texture is rejected by packet validation.
4. Run the modified Rust test file directly, including asset-dependent tests
   once input files have been checked. Run focused IBL tests and native crate
   check/clippy. Any wider contract change owes the affected workspace gate.
5. Prove a new behavioral test fails with the new IBL behavior temporarily
   disabled, then restore the implementation. Preserve the failed result.

Rust builds and tests are serial, offline, with one build job. Do not start
Julia or Vulkan initialization in this CPU phase. Keep RAM use bounded while
Gesso runs. Existing untracked TetCage portability files are outside ownership.

## GPU and visual gates

Only after Matt confirms Gesso's heavy GPU jobs are paused:

1. Render the current CONVERGE-4 control and the experimental arm serially,
   using the same locked assets and binary, into fresh output directories.
2. Save close/medium/wide images and complete packets, receipts, telemetry,
   command lines, source identity, and asset identities. Display review sheets.
3. Compare material response, shadow readability, water, and depth. Fixed
   exposure is deliberate: do not compensate away a regression in this probe.
4. Record packet size, upload bytes, and observed capture cost separately from
   steady-state runtime performance. Historical GPU-labelled capture timings
   do not establish gameplay frame time.
5. Re-render frozen references with the existing identity verifier if the
   candidate is retained or any shared shader execution changes.

## Stop and kill conditions

- Stop rendering if Gesso still owns the GPU, a build/input digest is missing,
  packet validation fails, or the worker-frame bound is exceeded.
- Reject retention if material readability worsens, water loses its accepted
  appearance, highlights clip conspicuously, or earlier arms change bytes.
- An inconclusive result is recorded as such. No threshold relaxation and no
  quality claim from the generic `good` capture status.

## Non-goals and next question

No cascades, contact AO, exposure retuning, new IBL mathematics, GI, probes,
new assets, characters, rigging, TetCage integration, infrastructure changes,
Gesso scheduling changes, or default-policy promotion in this slice.

Next question: after the IBL comparison has a disposition, close L-1's shadow
consistency evidence and write the exact cascade implementation recipe.
