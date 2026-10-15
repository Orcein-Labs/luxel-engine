# C3.2 playable session — Campaign B Stars 2–5

2026-10-13. Owner: Luxel Rust native-graphics contract. Status: open for
implementation. Graphics-parity / lighting work is out of scope. Native Lava
Live2D and Cubism authoring stay parked. Wallpaper host (Campaign C) and GCS
unification (Campaign D) stay later.

Star 1 (worker-sample drive loop) is closed in
[`c3-2-drive-loop-contract.md`](c3-2-drive-loop-contract.md). This recipe
consumes that loop.

## Outcome

A presented native session is playable: the movement sidecar's actor transform
is a request-side override on presented frames, the follow camera has typed
profiles, raw HID maps through a rebindable action map, and the presented path
records live-evidence Tier B with periodic Tier A promotion.

The sealed scene packet stays byte-identical. Overrides travel beside it, the
same way `camera_override` already does.

## Stars

### Star 2 — draw the moving actor

`ActorMovementSession` already emits a packet-bound actor transform
(MOVE-CPU-1). The presented window still draws the authored mesh. Consume that
sidecar as a **request-side override**:

1. `PresentedGraphicsSession` stores `instance_overrides` of
   `{instance_id, Transform3d}`. `set_instance_override` fails closed unless
   the id resolves exactly once in the base packet and the transform
   validates. The base packet is never mutated.
2. `present_frames_internal` sends optional `instance_overrides` on
   `render_window` when the map is nonempty. Worker allowed keys gain that
   field. `ADAPTER_REVISION` stays `luxel.lava-adapter/v7`.
3. The worker applies overrides to a **copied** instance list and keeps the
   original `packet_sha256` / `content_sha256`. Mesh and shadow GPU caches
   that bake instance transforms salt their key with an instance-transform
   fingerprint so a moving actor cannot reuse a static upload.
4. `PresentedPlaySession` binds one actor onto `InputDrivenSession`. Each
   applied tick writes the sidecar transform through `set_instance_override`
   and the follow camera through `set_camera`, using the same
   `moved_actor_transform` composition as MOVE-CPU-1 (authored translation
   plus body displacement, yaw composed with authored rotation, scale
   preserved).
5. `capture_and_promote` reseals the current camera **and** current instance
   overrides into the offscreen snapshot packet. Presented frames still send
   the sealed base packet plus overrides.

### Star 3 — typed camera profiles

Keep the default third-person boom (`camera_intent`) byte-comparable for
Star 1 tests. Add `CameraProfile`:

- `third_person` — existing boom, shoulder, pitch clamp, min height.
- `third_person_collision` — the same boom, then a march from the desired eye
  toward the look target against the packet heightfield. The eye stays at
  least `CAMERA_MIN_HEIGHT_M` above sampled terrain. Off-map samples pull in.
- `first_person` — eye at body origin plus 1.6 m, look along yaw/pitch.
  Pitch stays clamped so forward is never collinear with world up.
- `top_down` — orthographic, looking down, up is world north (`+Z`). Span
  and height are fixed play constants.

Projection and up travel on `camera_override` (already a full
`GraphicsCamera`). They do not reseal the presented packet.

### Star 4 — action map, rebind, persist

The DS3 keyboard/gamepad map becomes a typed `luxel.action-map/v1` value:
bindings per action, look invert X/Y, gamepad deadzone. `frame_from_sample`
keeps today's DS3 default. The play session can `set_action_map`. Rebind is
data: replace a binding list, validate, round-trip JSON. There is no HUD
renderer in this slice. Options invert/deadzone live on the same map.

### Star 5 — live-evidence on the presented path

`luxel-live-evidence-contract` stays the state machine; this slice is the
caller. `PresentedEvidenceBinder` records one Tier B attestation per present
batch from `WindowFrameReport` (frame time from the batch, GPU time
unreported as 0, draw calls = frames presented, instance counts from the
base packet). It never reads a wall clock: the caller supplies monotonic ms,
or the binder advances by summed batch microseconds.

When `tick` returns a snapshot request, `capture_and_promote` runs the
existing offscreen authority path. The binder maps a promoted receipt onto
`TierAResult::Certified` bound to the **live** (base) packet identity.
`LiveGraphicsSession` remains the offscreen adapter; this binder is the
presented-path caller.

## Permitted files

- `docs/gameplay/c3-2-playable-session-contract.md` — this recipe.
- `docs/README.md` — index.
- `world_core/crates/native_graphics_contract/src/session.rs`
- `world_core/crates/native_graphics_contract/src/input_session.rs`
- `world_core/crates/native_graphics_contract/src/action_map.rs`
- `world_core/crates/native_graphics_contract/src/camera_profile.rs`
- `world_core/crates/native_graphics_contract/src/lib.rs` — re-exports and
  `pub(crate)` validation reuse only.
- `world_core/crates/native_graphics_contract/tests/input_play_session.rs`
- `graphics_lab/bin/luxel_graphics_worker.jl` — allow `instance_overrides`.
- `graphics_lab/src/LuxelGraphics.jl` — parse helper.
- `graphics_lab/src/LavaAdapter.jl` — apply overrides; salt mesh/shadow cache.
- `graphics_lab/test/runtests.jl` — parse/fingerprint tests that need no GPU.

Do not edit lighting/IBL/converge tests, Cubism, `pipeline/rigging_provider.py`,
or `GraphicsScenePacket` schema.

## CPU acceptance

Using the checked-in Riverwatch movement fixture (no live window required):

- Binding a unique actor and applying a forward sample writes an instance
  override whose transform matches `ActorMovementSession` on the same sample.
  Replay reproduces it. An unknown instance id fails closed and mutates
  nothing. The base packet digest is unchanged.
- `capture_packet` with an override reseals a packet whose named instance
  carries the override transform; the presented base packet is untouched.
- Camera profiles: first-person eye is at the body, not the 3P boom;
  top-down projection is orthographic with downward forward; collision 3P
  eye height is at least min-height above the heightfield at the eye xz.
  Default 3P still matches `camera_intent`.
- Action map: DS3 default matches today's `frame_from_sample`. Rebinding
  forward to another key, inverting look X, and changing deadzone are
  visible in the mapped frame. JSON round-trip preserves the map. An
  invalid deadzone or empty schema fails closed.
- Evidence binder: a present report with typed times records a live frame;
  a zero/empty time fails closed; a due tick schedules a snapshot; a
  matching certified result is accepted against the live packet identity.

Worker HID present and actor-override draw are proven on a GPU lease when
one is free; headless CI may skip that present proof.

## Honest leftover

No options HUD is drawn. No wallpaper host. Native Live2D stays parked.
Lighting/graphics-parity stays out of scope.
