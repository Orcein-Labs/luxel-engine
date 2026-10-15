# CPU movement foundation — MOVE-CPU-1

2026-10-09. Owner: Luxel Rust native-graphics contract. Status: closed for
MOVE-CPU-1 implementation; CPU evidence required before promotion.

Implementation and CPU evidence are recorded in
[`cpu-foundation-implementation-20261009.md`](cpu-foundation-implementation-20261009.md).

## Outcome

Prove one deterministic, grounded player movement loop on CPU:

`raw input sample → semantic fixed-tick frame → validated world step → actor transform state → follow-camera state → replayable trace`

The first Heartwood movement probe may use a primitive actor. The transform
binding is asset-agnostic so a later avatar driver can consume it. The stated
professional Live2D quality target remains a separate presentation/content
profile; this contract does not claim Live2D import, rig playback, or rendering.

## Current seam and gap

`input_session.rs` already maps keyboard/gamepad samples, advances a 30 Hz
grounded planar disc, computes a follow camera, and records input plus final
position. `PhysicsWorld::new` validates the actual `WorldArtifact`; its step
receipt binds the input and resulting state to that world. The current session
does not emit a transform for a scene instance, does not include camera or
physics-receipt identity in its trace, and has no replay entry point. Its
`sample_and_present` path uses elapsed wall time and repeats one sample for
multiple ticks, so it is not the deterministic CPU oracle specified here.

`PresentedGraphicsSession` keeps the sealed base packet fixed while the camera
travels as an override. `GraphicsScenePacket` has a sealed per-instance
`Transform3d`; changing it would change packet identity. This recipe therefore
keeps the base packet untouched and carries motion as a versioned sidecar bound
to the original packet digest, world digest, and one unique actor instance ID.
The session snapshots packet identity and the selected actor transform; it does
not retain a packet borrow. It retains the validated world reference for real
physics steps.
The sidecar proves what transform belongs to that visible scene instance; it
does not claim that the current window renderer consumes dynamic actor
overrides.

## Bounded implementation

1. Add an `ActorMovementSession` and typed `ActorMovementFrame` in
   `world_core/crates/native_graphics_contract/src/input_session.rs`. Keep the
   existing `InputFrame`, `INPUT_TRACE_SCHEMA`, and `GraphicsScenePacket`
   serialization unchanged. The new frame has its own schema and binds
   `source_packet_sha256`, `world_artifact_id` and digest, `actor_instance_id`,
   tick, the actor transform override, camera position/forward, physics
   disposition, and physics receipt digest.
2. Construct the session from a validated base packet, validated world, and an
   actor instance ID that resolves exactly once in the packet. Initialize its
   physics state at the authored player spawn. Preserve the selected instance's
   authored transform at tick zero. On later ticks, add the grounded body
   displacement to its authored translation, compose the body yaw with its
   authored rotation, and preserve its authored scale. This makes the asset's
   origin/foot offset an authored property rather than guessed from a disc.
3. Accept one `InputSample` per explicit simulation tick. Call the existing
   `frame_from_sample` with `1 / REFERENCE_TICK_RATE_HZ`, then `PhysicsWorld::step`.
   Do not read a clock, present a window, or repeat a sample implicitly. A
   rejected collision/slope step consumes the tick and keeps the body
   translation while still applying valid look intent. A malformed or stale
   frame fails before committing yaw, physics state, actor state, camera state,
   or trace.
4. Derive the actor override and `camera_intent` from the same post-step state.
   Store the semantic frame, physics receipt digest/disposition, actor override,
   and camera state in the movement trace. Replay begins from the same packet,
   world, actor ID and spawn, reapplies the recorded semantic frames, and must
   reproduce every movement frame and final trace digest. Use an incremental,
   domain-separated trace digest so appending a frame is O(1) in prior trace
   length.
5. The base packet stays byte-for-byte unchanged when movement is absent and
   throughout this CPU session. No packet schema, render request, or worker
   protocol fields are added. A later renderer integration must explicitly
   consume the sidecar and prove shadow/reflection/culling behavior; that work
   needs its own contract and GPU lease.
6. Correct the existing timestamp-driven session's milliseconds-to-seconds
   conversion for gamepad look and make `apply_frame` commit yaw and pitch only
   after a successful physics call. Preserve the existing timestamp scheduling
   policy; the new explicit-tick API is the deterministic oracle. Cover the
   unit conversion and atomic failure in focused CPU tests.

## Permitted files

- `docs/characters/cpu-movement-foundation-contract.md` — this recipe and a
  later evidence receipt.
- `world_core/crates/native_graphics_contract/src/input_session.rs` — movement
  session, sidecar, and CPU-only unit logic. This module is already public; no
  `lib.rs` export is needed.
- `world_core/crates/native_graphics_contract/tests/input_movement.rs` — one
  integration test for the actual input-to-world-to-actor-to-camera path and
  replay.
- `world_core/crates/native_graphics_contract/tests/fixtures/riverwatch-movement-world.json`
  — a checked-in artifact produced by the normal Julia-backed Riverwatch build,
  not a hand-constructed `WorldArtifact`. Keep it under 512 KiB and call
  `validate_world_artifact` in the test before use.

Do not edit `world_core/crates/native_graphics_contract/src/lib.rs`,
`session.rs`, asset-contract code, `GraphicsScenePacket` schema, Julia/Lava
adapter, or existing lighting/TetCage files in this slice. If the sidecar
cannot remain separate from those contracts, stop and return for a new scope
decision.

## CPU acceptance

The test uses the checked-in, Julia-produced artifact and a valid sealed scene
packet with one designated primitive actor instance. It calls the real
`frame_from_sample`, real `PhysicsWorld`, and the movement session; it does not
stub the terrain or movement result.

- A raw keyboard sample for the authored known-good route advances exactly one
  fixed tick. The receipt names the validated world, and the resulting grounded
  position matches the receipt's next state. For the first S-key tick, an
  independent `PhysicsWorld::step` with velocity `[0, -4]` produces the same
  state and receipt digest as the emitted movement frame.
- The output sidecar names the original packet digest and unique actor instance.
  Its transform equals the authored transform plus the measured body
  displacement; a non-identity authored X rotation verifies analytical
  yaw-Y-times-source-X composition, and scale remains unchanged. A separately
  sealed packet with a different world binding and a valid packet missing the
  actor are rejected. The base packet digest and canonical bytes remain
  unchanged.
- Camera position and forward are the `camera_intent` for that same actor state
  and tick. A look-only frame changes yaw/camera consistently without moving
  the actor.
- A known blocked move consumes one tick, records `rejected`, leaves actor
  translation unchanged, and returns a camera derived from the retained body
  state. A wrong tick and non-finite input fail atomically and append no trace.
- Replaying the semantic frames from a fresh session reproduces every actor
  transform, camera state, physics receipt digest, and final trace digest in the
  same executable. No cross-platform floating-point bit-identity claim is
  made.
- A temporary fault that omits actor displacement must fail the actor-transform
  assertion; restore the exact source and rerun the focused suite.

Run after root grants the serial CPU lease:

```bash
cargo test --manifest-path world_core/Cargo.toml --offline -j 1 \
  -p luxel-native-graphics-contract --lib input_session -- --test-threads=1
cargo test --manifest-path world_core/Cargo.toml --offline -j 1 \
  -p luxel-native-graphics-contract --test input_movement -- --test-threads=1
```

This suite must not open a window or call the graphics worker. The checked-in
world artifact removes Julia startup from the test; its provenance and full
validation still exercise the real world contract. If a real source artifact
cannot be kept below 512 KiB, stop and choose explicitly between the existing
Julia-backed builder and a smaller Julia-produced authored layout.

## Non-goals and promotion gate

No jump, gravity, capsule clearance, sliding, nav, AI, combat, objectives,
character skinning, gait blending, Live2D/VRM adapter, avatar art, network
replication, input-device polling loop, window presentation, GPU work, or
renderer-side dynamic instance update. The current grounded planar-disc model
stays authoritative; this slice must not describe it as full character-body
physics.

Promotion to a playable Heartwood scene requires a separate renderer contract
for applying the actor sidecar to the selected instance, plus a later serial
GPU smoke that visibly moves the primitive actor through a clear route with the
camera following it. Character quality is judged on the professional Live2D
target in its own presentation profile; a CPU transform proof is not visual
avatar evidence.
