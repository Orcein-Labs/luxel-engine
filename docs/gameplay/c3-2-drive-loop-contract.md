# C3.2 drive loop — Campaign B Star 1

2026-10-13. Owner: Luxel Rust native-graphics contract. Status: open for
implementation. Graphics-parity / lighting work is out of scope. Native Lava
Live2D and Cubism authoring stay parked.

## Outcome

Close the parked C3.2 follow-up: a presented native session consumes **worker
HID samples**, not caller-invented repeats of one sample, and turns each sample
into exactly one fixed tick:

`present_frames_with_input → one InputFrame per returned sample → PhysicsWorld::step → follow camera → next batch`

Replay of the recorded semantic frames reproduces player state, camera intent,
and the input-trace digest. The sealed scene packet stays byte-identical.

This slice does **not** draw a moving actor from the movement sidecar, add
camera-mode profiles, rebind UI, an options overlay, a desktop wallpaper host,
or live-evidence sampling on the presented path. Those are later Campaign B
stars.

## Current seam and gap

`PresentedGraphicsSession::present_frames_with_input` already asks the worker
for `report_input:true` and parses `luxel.input-sample/v1` values.
`InputDrivenSession::sample_and_present` still takes a **caller-supplied**
sample, repeats it across wall-clock ticks, and presents through
`present_frames` (no worker samples). The worker path is therefore not the
drive loop. Sample-count correspondence (`N` presented frames → `N` samples)
is not enforced.

`ActorMovementSession` remains the CPU-only actor-transform sidecar
(MOVE-CPU-1). This contract does not consume that sidecar in the renderer.

## Bounded implementation

1. `InputDrivenSession::sample_and_present(supervisor, target_frames)` presents
   through `present_frames_with_input`, applies one `fixed_tick_frame` per
   returned sample, then leaves the follow camera aimed from the post-step
   body. It does not read a wall clock and does not repeat a sample to fill
   ticks. The window must already be open.
2. `apply_samples` is the worker-free oracle of that loop: one sample, one
   tick, same mapping and physics as the presented path. `replay_semantic_frames`
   starts from the same spawn and must reproduce every `TraceStep` and the
   final `trace_digest`.
3. `present_frames_with_input` fails closed unless the worker returns exactly
   `frames_presented` typed samples, each `InputSample::validate`s, with
   strictly increasing `timestamp_ms`.
4. Successful `apply_frame` updates the presented camera from `camera_intent`.
   A physics **call** failure still commits nothing (existing atomicity). A
   collision/slope **rejection receipt** still consumes the tick and look
   input.
5. No packet schema, render-policy, Julia scene, or lighting file changes.
   Worker `report_input` sampling already exists; do not add actor-transform
   overrides to `render_window`.

## Permitted files

- `docs/gameplay/c3-2-drive-loop-contract.md` — this recipe.
- `docs/README.md` — index the contract.
- `world_core/crates/native_graphics_contract/src/input_session.rs`
- `world_core/crates/native_graphics_contract/src/session.rs` — sample-count
  correspondence only.
- `world_core/crates/native_graphics_contract/tests/input_drive_loop.rs` — CPU
  oracle over the checked-in Riverwatch movement fixture.

Do not edit lighting/IBL/converge tests, `graphics_lab` shaders, the Cubism
bundle, or `pipeline/rigging_provider.py` in this slice.

## CPU acceptance

Using the checked-in Julia-produced Riverwatch movement fixture and a sealed
packet (no live window required for the oracle):

- Three distinct raw samples (forward, strafe, look) applied through
  `apply_samples` produce three ticks. The same samples applied again on a
  fresh session match player state, camera position/forward, trace, and digest.
- A single held-key sample applied once advances exactly one tick. The old
  wall-clock path that repeated one sample across `target_frames` is gone.
- `replay_semantic_frames` of the recorded `InputFrame`s reproduces the live
  trace digest. A stale tick fails and mutates nothing.
- `sample_and_present` without an open window fails closed.
- `accept_presented_samples` rejects a count mismatch, a non-monotonic
  timestamp, and an invalid sample without applying ticks.

Worker HID structure (`N` frames → `N` `luxel.input-sample/v1` samples) is
proven on a GPU lease when one is free; headless CI may skip that present
proof. Structure and count are the bar, not a pressed key.

## Honest leftover

The presented window still draws the **static** sealed mesh. Camera motion is
playable; the actor sidecar is not yet a render override. That is Campaign B
Star 2.
