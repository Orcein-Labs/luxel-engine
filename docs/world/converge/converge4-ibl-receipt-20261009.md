# CONVERGE-4 IBL comparison receipt

2026-10-09. Lead: Codex. Luxel native graphics, branch
`codex/converge4-lighting`. Status: comparison complete; quality promotion
rejected. This is an opt-in experiment, not a default
promotion or a Witcher 3/UE5 parity closeout.

## Change and audit

Added `converge4-ibl`: CONVERGE-4 policy/content plus enabled existing IBL,
with deterministic textures baked after the environment is final. The
launcher supplies the same locked terrain layers, kit2, and backdrop1.
Julia/Lava shaders and scene assets are unchanged. The supervisor continues
to re-lower from explicit campaign inputs and exact-compare authorization.

The Rust implementation and independent review used Luna Max workers; the
lead reviewed the diff, execution logs, and numerical assumptions. Ownership
stayed limited to the Rust selector/lowering, its existing test module, the
launcher, and these docs. Untracked TetCage portability work was untouched.

## CPU evidence

Logs: `artifacts/parity/ab-converge4-lighting-20261009/cpu-validation/`.

| Gate | Result |
| --- | --- |
| Modified `converge4`, debug, all fixture tests included | 8 passed, 0 failed; 317.31 s test execution |
| Disable only new IBL bake, exact release fixture test | Expected failure: 0 passed, 1 failed; IBL bake mismatch |
| Restore source, complete release `converge4` | 8 passed, 0 failed; 20.79 s test execution |
| Focused release `ibl` | 5 passed, 0 failed |
| Native crate `cargo check`, offline, one job | Passed |
| Release binary build, offline, one job | Passed |
| All-target Clippy, `-D warnings` | Failed on unchanged asset-import lint at `asset_contract/src/render.rs:1475` |
| Package-only Clippy, `--no-deps`, `-D warnings` | Failed on nine existing warnings; none in the new lines |
| Diff whitespace / launcher syntax | Passed |

The first mutation command used the wrong working directory and ran no test;
its failure is preserved separately in `01-negative-mutation.log`. The actual
negative evidence is `02-negative-mutation.log`. Source restoration is
byte-exact at SHA-256
`c2c02b2020e4004cc98fad5eefc86567e6d8d2d02b71ad64144834da6429daf1`.
One premature lead release build was stopped to restore serial compilation;
it is not counted as a passing build. No lint suppressions or unrelated fixes.

## Capture provenance

Artifact root: `artifacts/parity/ab-converge4-lighting-20261009/`.

- Preserved baseline release binary SHA-256:
  `5d4074896b8d14d4767b1200ad6abf10c3516ed0024ebd4fd74f6c473fa2a94d`.
- Reviewed candidate release binary SHA-256:
  `75e7a7755fac929f8358dfbca557e2f674da48e3845e71d2c21770b2c89bd2db`.
- Source/asset identities: `baseline-source-digests.json` and
  `candidate-source-digests.json`; shader/asset digests match.
- GPU: RTX 5060, 8151 MiB reported total; Gesso heavy GPU jobs paused by Matt.
  Calls are serial through the real `render-campaign2-layout` command.
- Pre-change baseline: `pre-change-control/`, policy `converge4`.
- Same-binary paired runs stage at `/tmp/luxel-converge4-lighting-20261009/`
  and are archived as `control/` and `candidate/` in this artifact root.
  Logs/costs are saved directly in
  the artifact root. Output storage differs from the pre-change run, so their
  wall times are not a fair performance comparison.

The new binary's C4 control passed all three promoted capture gates and
reproduced the old binary's packet identities and capture bytes exactly.
Evidence: `control-identity.json`. Control campaign wall time: 463.30 s;
maximum child RSS reported by `time`: 2,780,128 KiB. These are cold campaign
costs, not gameplay frame times or total system peak memory.

The pretty packet files are about 195.68 MB; compact reserialization is about
101.79 MB. Successful actual worker calls remain within the 128 MiB frame
bound. Close control telemetry reports 291,157,633 upload bytes, 155 draw
calls, 8,770 instances, and 36,196,472 packed material texture bytes.
Packed texture bytes are not actual float-texture transfers or peak VRAM.
Deterministic receipts zero timing telemetry; campaign timing fields include
capture work and do not establish steady-state FPS.

## Candidate disposition and next question

`REJECT` as a drop-in quality change. All three candidate packets/captures
passed native promotion/technical quality gates, and the real candidate bodies
equal C4 after removing only enabled IBL and its two textures
(`candidate-body-comparison.json`). The registered comparator's self-controls
passed, but the candidate failed in all views (`compare.json`):

| View | Changed pixels | Mean luminance change | Highlight fraction ≥200 change |
| --- | --- | --- | --- |
| Close | 87.8535% | +31.7141% | +1.0381 percentage points |
| Medium | 92.9564% | +31.5991% | +1.0318 percentage points |
| Wide | 88.4797% | +27.4001% | +1.0860 percentage points |

Registered bounds remain 12% mean movement and 1 percentage point highlight
movement. The ≥200 highlight proxy does not establish clipping at 255. Sky
banding comparison is unavailable; missing metrics are not treated as zero.
Lead review of close/medium/wide finds lighter surfaces and washed-out terrain
and canopy contrast. The pool does not gain distant-ridge geometry reflection.
No exposure retuning or threshold changes. Default C4 remains unchanged; this
experimental selector is kept only in the uncommitted branch for reproduction.

Candidate campaign wall time: 420.31 s; maximum child RSS: 2,608,848 KiB. The
control was 463.30 s. These single cold calls do not establish a speedup.
Candidate upload bytes are 291,843,009 / 299,729,929 / 299,784,809 for
close/medium/wide: exactly +685,376 bytes in each view (four times the packed
IBL texture payload). No per-resource timing or steady-state FPS claim.

Review sheets and PNG captures:
`artifacts/parity/review-converge4-ibl-20261009/`.
All GPU calls have finished; CPU archiving/review does not reserve the GPU.

Frozen 131-frame identity suite has not been re-run: no shared shader changed
and there is no retained/default quality promotion in this experiment.

The follow-up ledger is `lighting-rpdo-audit-20261009.md`. It distinguishes
actual transfer attribution, optional geometry/texture optimizations, repeated
IBL bake costs, material limitations, and parked high-end quality paths.
