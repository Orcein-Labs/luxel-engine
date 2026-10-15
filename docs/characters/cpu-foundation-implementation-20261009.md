# CPU character and movement foundations

2026-10-09. CHAR-CPU-1 and MOVE-CPU-1 implemented and checked on CPU.
Witcher 3 and Elden Ring remain quality references, not achieved parity claims.
No GPU, Blender render, app launch, frontend mutation or receipt-schema change
was part of this work.

## Character evaluation

`decode_character_glb` now independently validates and decodes one selected
embedded GLB skinned mesh and named clip. Its stateless evaluator samples STEP
and LINEAR TRS, including shortest-path quaternion interpolation, computes
hierarchy globals and mesh-local linear-blend skinning, and returns scene-space
sockets and posed bounds. Non-joint ancestors and nonidentity mesh transforms
are retained. Missing inverse-bind data uses glTF's identity default.

Unsupported morphs, cubic interpolation, extra influence sets, external/sparse
data and required extensions fail by name. Corrupt joints, weights, hierarchy,
animation and transforms fail explicitly. This initial profile limits source
nodes to 4096 and hierarchy depth to 128; it bounds recursion and the shared
parent validator's worst case, without claiming linear-time import.

The decoded object is a numerical oracle. It neither issues a preparation
receipt nor overrides the existing game-specific preparation requirements.
It does not supply animated rendering, normals/tangents, blend graphs,
root motion, IK, retargeting, facial morphs or Live2D deformation.

## Movement

`ActorMovementSession` converts one raw input sample into one explicit 30 Hz
tick through the real `PhysicsWorld`. It snapshots the selected instance's
authored transform and emits a versioned sidecar bound to the source packet,
world and instance. Translation follows body displacement; world yaw composes
with authored rotation; authored scale is preserved. Camera state comes from
the same post-step body. The sealed packet remains unchanged.

Trace entries include semantic input, actor/camera state and the physics receipt
digest. Incremental chained hashing avoids rehashing the whole prior trace on
each new movement frame. Replay reproduces frames and the final digest within
the same executable. Errors commit no partial actor, camera or trace state.

The existing input session now passes seconds to gamepad look mapping and
commits yaw/pitch only after physics succeeds. Raw gamepad axes outside [-1,1]
are rejected. Timestamp scheduling policy otherwise remains unchanged. This
is still grounded planar-disc movement; the window renderer does not yet
consume actor sidecars.

## Verification

All runs were serial, offline, one build job and one test thread. Full outputs,
exact commands, exit codes and timings are retained under
[`artifacts/cpu-character-implementation-20261009/`](../../artifacts/cpu-character-implementation-20261009/).

| CPU gate | Result | Receipt/output |
|---|---|---|
| Runtime preparation and real-GLB deformation oracles, including hierarchy limits | 12 passed, 0 failed | `runtime_preparation_bounded.log` and `.gate.json` |
| Rigging authority revalidation | 4 passed, 0 failed | `rigging_authority.log`; final gate also recorded separately |
| Input mapping and failed-physics atomicity | 9 passed, 0 failed | `input_session_restored.log` and `.gate.json` |
| Real-world actor/camera/replay and provenance failures | 2 passed, 0 failed | `input_movement_restored.log` and `.gate.json` |
| Omit inverse-bind contribution temporarily | Expected failure: exit 101, 1 failed | `ibm_mutation.log`; exact source restored and runtime suite passed |
| Omit actor displacement temporarily | Expected failure: exit 101, 1 failed | `actor_displacement_mutation.log` and `actor_displacement_mutation-control.json`; source byte-restored and movement suite passed |

The movement fixture is the exact 254,523-byte artifact from
`artifacts/parity/ab-converge4-lighting-20261009/control/close/world_artifact.json`,
copied into the integration suite. SHA-256:
`b32cd5818b4c456996a6f3221ee7aa375b0e4e45ad55bf2fd2bc153db505f7a2`.
It records Julia 1.12.6 and the normal
`terrain_lab/bin/luxel_reference_world_fields.jl` producer. Tests validate the
full artifact and use real physics; no simulation or terrain result is stubbed.
The first movement output's physics receipt and state are independently checked
against an explicit physics call. Nonidentity authored X rotation checks yaw
composition order. New test-line additions remain below implementation lines.

The hierarchy traversal bug found in review was corrected before the first
runtime suite. No unrecorded failing test is represented as a passing run.
Fault-injection failures are deliberate controls and not product failures.

## Avatar and next boundaries

Two original flattened avatar design studies and prompts are saved under
`assets/avatars/cyan/source/` and `docs/characters/avatar/`. The current study is
female, following the user's demo-render preference. Both were produced by the
built-in image-generation tool, not Luxel. Neither is layered, rigged or
professional-quality accepted; alpha/fringe cleanup remains open.

The user-supplied Cyan Avatar Frontend contains Open-LLM-VTuber v1.2.1 and
existing sample exports. A CPU probe exercised its actual Python model payload
loader for `mao_pro`, verified 20 references, and validated emotion and idle
mapping. The saved preflight pins upstream/frontend commits and file hashes.
It does not prove MOC3 evaluation, visual playback or new avatar quality.
The frontend repository was inspected read-only.

See the [avatar quality contract](avatar/live2d-avatar-quality-contract.md) for
human and nonhuman subjects, layered-art requirements, semantic activity
controls, prop occlusion, authoring/export and actual consumer evidence. A
Cubism authoring setup and original editable rig remain missing. Live2D is a
separate provider/deformer path from the new 3D skeletal oracle.

The [static-extraction decision](static-extraction-import-decision.md) closes
the next import rule: named unsupported deformation by default; explicit
intent before discarding it. Its versioned implementation is separate from
CHAR-CPU-1; existing receipt schemas and conditioning behavior remain frozen.

Next native rendering work must carry pose/actor state through explicit
graphics contracts, update shading streams/bounds/shadows/reflections, and fix
the previously identified moving-camera cache boundary. Visual promotion needs
a later GPU lease. Do not infer playable-character or cinematic parity from
the CPU proofs above.
