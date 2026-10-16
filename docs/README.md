# Luxel Engine docs

Start with [roadmap.md](roadmap.md). For the October 12 continuation, read the
[active materials and lighting campaign](world/converge/2026-10-12-materials-lighting-campaign.md), then the
[engine readiness reset](platform/2026-10-12-engine-readiness-reset.md), then the
[development recovery](world/converge/2026-10-12-development-recovery.md) and
[camera continuity contract](world/converge/2026-10-12-camera-continuity.md), followed by
[camera-relative shadows](world/converge/2026-10-12-shadow-continuity.md) and
[resolution attribution](world/converge/2026-10-12-crispness-attribution.md).
The engine's name and the internal `luxel` prefix: [adr/0001-luxel-engine-name.md](adr/0001-luxel-engine-name.md). Docs are filed by roadmap layer; closed
handoffs and reports live in `archive/`, bundled by month and topic, each with
a `SUMMARY.md`.

| Folder | What it holds | Code it describes |
|---|---|---|
| [platform/](platform/) | architecture, contracts every layer uses, ledgers, open decisions | `world_core/crates/{native_graphics_contract, asset_contract, certification_authority, project_ledger, worldspec, live_evidence_contract, intake_repair_contract}`, `graphics_lab/` (Julia/Lava renderer), `world_core/apps/world_viewer` |
| [world/](world/) | terrain, atmosphere, materials, lighting, vegetation, architecture; the CONVERGE contracts | `native_graphics_contract` (scene lowering, `backdrop.rs`, `kit.rs`, `scatter.rs`, `terrain_layers.rs`, `still_water.rs`), `terrain_lab/`, `pipeline/` terrain, zone and forestry modules, `tools/` kit, backdrop and measurement scripts |
| [characters/](characters/) | authoring workbench, textured atlas pilots, Ornith parked checkpoint | `character_lab/`, `pipeline/rigging_provider.py`, `assets/avatars/` |
| [deformation/](deformation/) | TetCage seam | `graphics_lab/tet-lab/`, `native_graphics_contract/src/deformation.rs` |
| [gameplay/](gameplay/) | gameplay kit, capability resolution, live loop and runtime | `world_core/crates/{gameplay_contract, reference_runtime}`, `pipeline/` navigation, collision and acceptance modules |
| [content-sdk/](content-sdk/) | semantic facade, construction plans, semantic kernel | `world_core/crates/{semantic_kernel, luxel_control_plane}`, `pipeline/luxel_*.py`, `pipeline/worldbuilder_dsl.py`; language spec in `DSL docs/` (hash-frozen, stays put) |
| [add-ons/](add-ons/) | Grindstone, Reforge | (none yet) |
| [integrations/](integrations/) | tools Luxel drives from outside | Gaea: `pipeline/gaea_*.py`, `Code Projects/Gaea/`; Blender: `pipeline/blender_*.py`, `tools/blender_*.py`; ComfyUI: `pipeline/comfy_generate_texture.py`; engines: `engine_adapters/`, `pipeline/zone_to_{unity,unreal}.py`, `pipeline/*.gd` |
| [adr/](adr/) | decision records | |
| [archive/](archive/) | superseded and closed docs | |
| [media/](media/) | screenshots used by the top-level README | |
| [research/](research/) | isolated research campaigns (not production) | [PJ-0 Performant Julia](research/pj-0/REPORT.md) |

Code still lives where it was. It moves into per-layer homes when that layer
becomes the active campaign, so a move never lands in the middle of
someone else's sprint; this table is the map until then.

## World: materials and lighting

- [Native graphics strict lint cleanup](platform/2026-10-12-native-graphics-lints.md) — bounded Clippy repair and retained formatting backlog.
- [Cascades, contact, and Heartwood development](world/converge/2026-10-12-cascades-contact-heartwood.md) — opt-in implementation and verification frontier.
- [Local point-light shadows](world/converge/2026-10-12-local-shadows.md) — optional cube-atlas occlusion on the four local lights.
- [CONVERGE-4 IBL comparison](world/converge/converge4-ibl-comparison-contract.md) · [receipt](world/converge/converge4-ibl-receipt-20261009.md) — rejected; the `converge4-ibl` arm stays for re-measurement.
- [Lighting structural audit](world/converge/lighting-rpdo-audit-20261009.md) — October 9 RPDO pass that put moving-camera correctness ahead of beauty work.

## Characters

Parked October 10 after the accepted Ornith rest appearance. Materials and
lighting is the active campaign; these receipts stay indexed so the files are
findable.

- [Character authoring MVP receipt](characters/character-workbench-mvp-receipt-20261009.md)
- [Character creator direction](characters/character-creator-direction-20261009.md)
- [Character maker structural audit](characters/character-maker-rpdo-20261009.md)
- [Textured authoring pilot](characters/textured-authoring-pilot-contract.md) · [receipt](characters/textured-authoring-pilot-receipt-20261009.md)
- [VISUAL-QA-1](characters/visual-qa-1-contract.md) · [receipt](characters/visual-qa-1-receipt-20261009.md)
- [ASSEMBLY-FIT-1](characters/assembly-fit-1-contract.md) · [receipt](characters/assembly-fit-1-receipt-20261009.md)
- [ASSEMBLY-FIT-2](characters/assembly-fit-2-contract.md) · [receipt](characters/assembly-fit-2-receipt-20261009.md)
- [ASSEMBLY-FIT-3](characters/assembly-fit-3-contract.md) · [receipt](characters/assembly-fit-3-receipt-20261009.md)
- [CHARACTER-TOPOLOGY-1](characters/character-topology-1-contract.md) · [receipt](characters/character-topology-1-receipt-20261009.md)
- [Original Ornith proof](characters/original-ornith-proof-contract-20261009.md) · [receipt](characters/original-ornith-proof-receipt-20261010.md)
- [Ornith parked checkpoint](characters/ornith-parked-checkpoint-20261010.md)
- [Cyan art and Live2D audit](characters/cyan-live2d-audit-20261010.md)

## Gameplay

- [C3.2 drive loop](gameplay/c3-2-drive-loop-contract.md) — Campaign B Star 1: worker samples to fixed ticks with replay.
- [C3.2 playable session](gameplay/c3-2-playable-session-contract.md) — Campaign B Stars 2–5: actor-transform override, camera profiles, action map, presented live-evidence. Wallpaper host stays later.
