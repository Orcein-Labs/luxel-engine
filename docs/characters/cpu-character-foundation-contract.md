# CPU character foundation — CHAR-CPU-1

2026-10-09. Owner: Codex, Luxel Rust asset-contract layer.
Disposition: BUILD. Status: implemented; CPU evidence recorded in the companion receipt.

## Problem and outcome

Luxel can validate a rigged GLB and issue a content-addressed preparation
manifest, but cannot evaluate that asset into an animated mesh. The first
character slice must prove source bytes → decoded rig → sampled pose → deformed
geometry without requiring a GPU, Blender, or another runtime engine.

This is useful to villagers, creatures such as Cervideth, film characters, and
future avatar tooling. It must not assume a humanoid bone layout or require an
attack clip merely to sample an asset's named animation.

## Permitted files

- `world_core/crates/asset_contract/src/runtime/character.rs` — new typed decoded
  character data and deterministic CPU evaluation.
- `world_core/crates/asset_contract/src/runtime.rs` — child-module declaration
  and public re-exports; reuse existing GLB/accessor helpers and validation.
- `world_core/crates/asset_contract/tests/runtime_preparation.rs` — extend the
  existing real GLB-byte controls and process-boundary tests.
- This contract and a companion implementation receipt.

No dependencies or existing preparation/receipt schemas change. No changes to
graphics packets, shaders, Julia/Lava, TetCage, gameplay, or provider machinery.
If those become necessary, stop this slice and record the new question.

## Bounded profile

1. Decode a selected mesh instance, its one skin, and one explicitly named clip
   from embedded glTF 2.0 GLB data. Preserve non-joint ancestor nodes needed to
   compute transforms. The source and selection identify the decoded result.
2. Preserve positions, joint indices, weights, inverse bind matrices, rest node
   transforms, animation samplers/channels, and selected socket nodes. Decode
   through the existing checked accessor machinery rather than a second parser.
3. Support up to four influences (`JOINTS_0` / `WEIGHTS_0`). Validate the selected
   profile independently; an existing preparation `Ready` summary is not proof
   that this execution profile supports the asset. Expose an independently
   validated `decode_character_glb(bytes, selection)` numerical import API. Its
   decoded object is not a preparation receipt, package, or production-readiness
   verdict. The existing application-specific Idle/Locomotion/Attack preparation
   profile remains unchanged; CPU decoding need not require those roles.
4. Sample STEP and LINEAR TRS channels. Use shortest-path normalized quaternion
   interpolation for LINEAR rotations. Evaluate explicit time in seconds,
   clamped to the clip endpoints. Seeking must not depend on prior evaluations.
5. Compute hierarchy globals, then linear-blend skinning with the glTF
   mesh-node/inverse-bind relationship. Return posed mesh-local positions,
   scene-space socket transforms, and bounds derived from posed positions.
   State spaces and column-major matrix convention in the types/API documentation.
6. Keep calculations in the source glTF coordinate space. The CPU result is an
   execution oracle, not a render-conditioned asset; applying a runtime axis/unit
   conversion twice is forbidden. Test a nonidentity mesh-node transform.
7. Reject unsupported features with named failures: morph targets/channels,
   CUBICSPLINE, sparse or external buffers, additional influence sets, ambiguous
   selections, unsupported required extensions, malformed hierarchies/accessors,
   invalid weights, and nonfinite or singular required transforms. Do not silently
   substitute a bind pose. The evaluator must not bypass an existing preparation
   rejection to claim an asset is ready for production use.
8. Bound this initial execution profile to 4096 source nodes and hierarchy depth
   128 (including the root). Reject excess with a named unsupported-feature
   finding before recursive transform evaluation. These limits bound the shared
   parent validation's worst case; they do not claim a linear-time import path.
   Missing inverse-bind matrices use glTF's identity default.

## Acceptance and oracle

Use self-contained real GLB bytes in the existing integration suite. Expected
coordinates must come from simple independently derived transforms, not a second
invocation of the production evaluator.

- Rest pose with nonidentity inverse binds recovers the expected geometry.
- A two-joint weighted vertex has the analytically expected intermediate pose;
  changing hierarchy order or inverse-bind composition must fail the test.
- Include a rotating joint, a non-joint ancestor, and a nonidentity mesh node to
  expose matrix-order/space mistakes that translation-only controls miss.
- A child socket follows its animated joint. Its scene-space transform and the
  posed geometry's mesh-local AABB agree with independent expected values.
- STEP, LINEAR translation, quaternion sign equivalence, endpoints, repeated
  seeking, and nonfinite time have explicit controls.
- Corrupt joint/weight/buffer controls fail; unsupported interpolation and morph
  data fail rather than producing partial output.
- Existing preparation receipts and CLI good/bad outcomes remain unchanged.

Set numeric tolerances before implementation: analytic position/socket/bounds
checks use absolute error ≤ 1e-6 in fixture units; quaternion comparisons allow
equivalent signs. Repeated evaluation in the same executable must be identical.
No cross-platform bit-identity claim is made for floating-point interpolation.

Run serially, offline, with one build job:

```bash
cargo test --manifest-path world_core/Cargo.toml --offline -j 1 \
  -p luxel-asset-contract --test runtime_preparation -- --test-threads=1
cargo test --manifest-path world_core/Cargo.toml --offline -j 1 \
  -p luxel-certification-authority --test rigging_authority -- --test-threads=1
```

Prove the new oracle fails with a temporary production mutation that omits the
inverse-bind contribution or parent transform; restore the exact source, then
rerun the suite. Record the failing assertion and failed-test count. Keep added
test lines no greater than added implementation lines.

## Non-goals and promotion boundary

This slice does not render an animated character or certify visual quality. It
does not provide normals/tangents for rendering, a general animation graph,
loop/blend policies, root motion, IK, retargeting, autorigging, facial morphs,
cloth/hair physics, VRM/Live2D export, or professional character art.

The next question is how to carry the CPU-proven rig/pose through a versioned
native graphics contract with GPU skinning, correct shading streams, moving
bounds, shadow/reflection parity, and measured upload cost. CPU correctness
must precede that GPU comparison. TetCage must earn improvements against this
conventional baseline rather than replace the missing baseline.
