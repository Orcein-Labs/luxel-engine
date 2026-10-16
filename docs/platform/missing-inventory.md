# Luxel missing inventory

Written 2026-10-16. This file lists what the native engine does **not have
yet**. Two other ledgers cover the rest:

- things that exist but are wrong: [debt-ledger.md](debt-ledger.md);
- things that are built and ranked: [tooling-upgrades.md](tooling-upgrades.md).

The [graphical parity audit](../world/graphical-parity-audit.md) and the
[engine readiness reset](2026-10-12-engine-readiness-reset.md) hold the
graphics detail. This file covers the whole engine at one level of detail
across every system. Each entry says what exists, so every gap is measured
against the code.

The July 31 version of this file was a Codeweald/MOBA gap list written against
the Bevy viewer and a Unity backend. Both have since been superseded by the
native engine. That version is in git history (last at `d002292`). Its still-open
items are carried over below.

## Where the engine stands

The platform underneath is built and verified:

- the Rust authority builds and validates every scene packet;
- a persistent Julia/Lava Vulkan worker renders it, offscreen or into a live
  window;
- receipts and deterministic replay record what was rendered and simulated;
- the "absent axis = same bytes" identity check (`tools/verify_identity.py`)
  guards existing frames.

What remains is breadth. Most of the systems a game or a Witcher-3-class scene
needs are missing or exist only as a first slice. Two items carried over from
July are not re-checked here and stay in git history: terrain metric gaps
(M1–M3) and provenance and reachability coverage (M5–M7).

## Graphics

| Area | Exists | Missing |
|---|---|---|
| Terrain | Gaea landforms, scanned terrain layers, km-scale backdrop | Ground integration: surface scale, transitions, wear/moss/wetness, paths, decals. Caves, overhangs and arches: terrain is a heightfield (July M4; needs a representation decision first). |
| Atmosphere | Analytic sky, aerial perspective, km-scale haze | Volumetric light shafts, height/forest fog |
| Materials | Scanned PBR sets, normal/roughness/occlusion/emissive roles | Foliage, layered natural surfaces, stylized shading. The IBL arm was rejected on review. |
| Lighting | One directional light with two shadow cascades and contact AO; up to 4 point lights (`MAX_POINT_LIGHTS`), optionally shadowed through a cube atlas | More than 4 local lights, useful key/fill/environment balance, stable shadows under a moving camera |
| Transparency | Alpha-mask cutouts with alpha-to-coverage | Alpha blending (`alpha_mode == :blend` throws), so no glass, smoke, real water surface or particles |
| Water | Still water painted by the terrain shader, with a planar reflection | Flowing water, rivers, shorelines, waves |
| Vegetation | Scatter and a one-species forest | Atlases, crown volume, two-sided shading, wind, hero-scale trees, biome variety |
| Architecture | One imported ruin kit | Settlement kit: paths, bridges, interiors |
| Post | Tonemap, ordered dither; opt-in bloom, grade and vignette (off in the frozen baseline) | Temporal AA or upscaling, depth of field, motion blur |
| VFX | — | Particles, trails, decals, screen-space effects |
| Scale | CPU frustum culling; texture mips | Mesh LOD selection (assets carry LOD metadata the renderer never reads), occlusion culling, streaming, GPU-driven rendering. The adapter has no compute pipeline (`dispatch_calls` is hard-coded to 0). |
| Style | `RenderPolicy` carries the policy the renderer can execute | Four style axes are fenced as unsupported: reflection expectation, volumetrics, outline, posterization. One photoreal-leaning look has been proven. |
| Resolution | 960×640 with 2× supersampling plus MSAA | 1080p and above at a playable frame rate |
| Performance | 10–13 fps presented on CONVERGE-4 ([2026-10-16 note](../research/luxel-consolidation/2026-10-16_presented-fps.md)) | 60 fps. The GPU takes 48–70 ms per frame, and about 22 ms of CPU recording runs before the GPU work each frame instead of alongside it. |

## Engine systems

### Animated characters

**Exists:** CPU evaluation (CHAR-CPU-1): GLB rig and clip decoding, STEP and
LINEAR sampling, skinning, sockets, posed bounds.

**Missing:**
- Rendering skinned meshes: `MeshPacket` has no joints, weights or pose
  streams, so the renderer draws static meshes only (parity audit GP-09).
- Animation blending, state machines, root motion, IK.
- Facial animation and morph targets.

TetCage also waits on this path. It can only be used in a shipped scene once
skinned meshes render.

### Physics

**Exists:**
- A deterministic kinematic controller (`reference_runtime/src/physics.rs`): a
  grounded disc that collides with terrain, obstacles, world bounds, blocked
  regions and slope limits, at a fixed tick, with replay.
- Collision descriptors from the pipeline (`collision_plan.py`).

**Missing:** rigid bodies, joints, ragdolls, projectiles, triggers, vehicles,
destruction. The module calls itself "not a force-based physics engine".
Adopting an existing Rust physics library is the likely route rather than
writing one.

### Audio

**Exists:** nothing in code. The gameplay capability model has an "audio intent"
field and nothing consumes it.

**Missing:** everything: playback, mixing, 3D positional audio, streaming music,
ambience, and the intent → sound binding.

### UI and text

**Exists:** debug evidence overlays only.

**Missing:** gameplay UI: HUD, menus, inventory and dialogue screens, text
rendering and fonts in the native window, and UI input focus.

### Input

**Exists:** typed HID sampling (keyboard, mouse, gamepad) in the worker, a
Rust-owned action map and third-person camera, and a deterministic input trace
(C3.2).

**Missing:** rebinding, multiple local players, and text input for UI.

### Gameplay framework

**Exists:**
- The gameplay contract with capability (GCS) resolution.
- The deterministic reference runtime.
- The C3.2 drive loop and playable session contracts.

**Missing:**
- Live gameplay entities: actors, stats, combat, damage. The model is specified
  and resolvable but nothing plays on it.
- Spawns, objectives and gameplay volumes in the native path (July G3).

### AI and navigation

**Exists:** the pipeline emits a walkable mask, a clearance field and connected
components (`navigation_plan.py`). It deliberately bakes no polygon navmesh.

**Missing:** runtime pathfinding, a navmesh or flow field consumed at runtime,
NPC behaviour (perception, decisions, schedules), crowds.

### World streaming

**Exists:** one scene packet, kept resident by the worker for a presenting
session.

**Missing:** world partitioning, streaming cells in and out, memory and residency
budgets, persistent large worlds. Everything has to fit in one packet.

### Save and load

**Missing:** game-state persistence. The deterministic traces and digests are a
good basis, but there is no save format.

### Networking

**Exists:** authority and network fields in the GCS, which are descriptive
metadata only (`docs/gameplay/gcs-foundation-design.md`).

**Missing:**
- The authority-model decision: client-server or deterministic lockstep (July
  G7, still open).
- Replication, prediction, sessions.

This is only needed if multiplayer or MMO add-ons become real.

### Authoring and editor

**Exists:**
- The DSL (frozen spec), the semantic facade and kernel, and construction plans.
- Blender and Gaea act as the external editors.

**Missing:**
- An in-engine editor: scene layout, live tweaking, an asset browser.
- Add-on implementations: Missions, Armory/Grindstone, Studio and Reforge are
  specs only.

### Live2D / Cubism

**Exists:** the avatar quality contract and the Cyan audit.

**Missing:** an editable rig, a visual runtime proof, and Cubism-format
compatibility. A native 2D puppet is not Cubism compatibility.

## Shortest path to a first playable prototype

Ordered by what unblocks the most:

1. **30+ fps presented.** Overlap CPU recording with GPU work across frames,
   then profile and cut the GPU cost.
2. **Skinned characters rendered on the GPU.** Without them nothing in the
   world moves except the camera.
3. **Rigid-body physics**, most likely from an existing library, behind the
   existing kinematic seam.
4. **Basic audio and UI**: positional sound effects, a HUD, a pause menu.
5. **Live gameplay entities** on the GCS model, with runtime navigation for
   NPCs.

None of these depends on finishing graphics parity, so the graphics and game
tracks can alternate.
