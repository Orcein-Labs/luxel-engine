//! CPU-only Heartwood forest content tests. The synthetic packet is fully
//! portable and exercises content replacement without kit, backdrop or GPU
//! assets.

use luxel_native_graphics_contract::backdrop::backdrop_set_from_glb;
use luxel_native_graphics_contract::{
    Axis, BackdropSet, BufferReference, CameraProjection, Campaign2FeatureInputs,
    Campaign2Features, Campaign2Inputs, Campaign2LoweringInputs, Campaign2OutputResolution,
    Campaign2View, CaptureFormat, CoordinateSystem, EnvironmentIntent, ForestProfile,
    GraphicsCamera, GraphicsCaptureRequest, GraphicsScenePacket, GraphicsScenePacketBody,
    Handedness, InstanceImportance, InstancePacket, KitSet, LightIntent, LightKind, MaterialIntent,
    MeshPacket, ParityContent, ParityPolicyCandidate, TerrainPacket, TerrainSurface,
    TerrainWetZone, Transform3d, apply_forest_profile, canonical_json, load_kit_set,
    lower_campaign2_packet_with_features, validate_scene_packet,
};

const SOURCE_SHA: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[path = "support/synthetic_backdrop.rs"]
mod synthetic_backdrop;
#[path = "support/synthetic_layers.rs"]
mod synthetic_layers;

fn synthetic_body() -> GraphicsScenePacketBody {
    let resolution = 65;
    let width = 120.0;
    let length = 120.0;
    let heights = (0..resolution)
        .flat_map(|row| {
            (0..resolution).map(move |column| {
                let x = column as f32 / (resolution - 1) as f32 * width - width * 0.5;
                let z = length * 0.5 - row as f32 / (resolution - 1) as f32 * length;
                x * 0.018 + z * 0.009
            })
        })
        .collect::<Vec<_>>();
    let zero_f32 = vec![0.0; resolution * resolution];
    let zero_u8 = vec![0; resolution * resolution];
    let mut body = GraphicsScenePacketBody {
        schema_version: "luxel.graphics-scene-packet/v6".into(),
        packet_id: "synthetic-heartwood".into(),
        scene_artifact_id: None,
        scene_artifact_sha256: None,
        world_artifact_id: "synthetic-world".into(),
        world_artifact_sha256: format!("sha256:{SOURCE_SHA}"),
        spatial_fields_sha256: format!("sha256:{SOURCE_SHA}"),
        frame_seed: 9,
        coordinate_system: CoordinateSystem {
            up_axis: Axis::Y,
            handedness: Handedness::Right,
            units_per_meter: 1.0,
        },
        camera: GraphicsCamera {
            camera_id: "campaign2-wide".into(),
            projection: CameraProjection::Perspective {
                fov_y_degrees: 50.0,
            },
            position_xyz_m: [19.0, 11.0, 24.0],
            forward_xyz: [-0.5, -0.2, -0.8],
            up_xyz: [0.0, 1.0, 0.0],
            near_plane_m: 0.1,
            far_plane_m: 256.0,
            width_px: 960,
            height_px: 640,
        },
        terrain: TerrainPacket {
            terrain_id: "synthetic-terrain".into(),
            width_m: width,
            length_m: length,
            resolution,
            material_id: "terrain".into(),
            heights_m: BufferReference::inline_f32("heights", heights),
            slope_grade: BufferReference::inline_f32("slope", zero_f32),
            region_codes: BufferReference::inline_u8("regions", zero_u8),
            layers: None,
            wet_zone: Some(TerrainWetZone {
                center_xz_m: [-3.8, 2.2],
                radii_xz_m: [3.8, 2.7],
                falloff_m: 0.6,
                albedo_scale: 0.6,
                roughness_scale: 0.25,
                normal_scale: 0.4,
                standing_water: None,
            }),
        },
        materials: vec![MaterialIntent {
            material_id: "terrain".into(),
            base_color_rgba: [0.3, 0.3, 0.3, 1.0],
            metallic: 0.0,
            roughness: 0.9,
            clearcoat: 0.0,
            clearcoat_roughness: 0.045,
            alpha_mode: luxel_native_graphics_contract::AlphaMode::Opaque,
            alpha_cutoff: None,
            double_sided: None,
            metallic_from_texture: None,
            texture_ids: Vec::new(),
            normal_texture_id: None,
            roughness_texture_id: None,
            occlusion_texture_id: None,
            emissive_texture_id: None,
            normal_scale: 1.0,
            occlusion_strength: 1.0,
            emissive_factor_rgb: [0.0; 3],
        }],
        textures: Vec::new(),
        meshes: Vec::new(),
        instances: Vec::new(),
        lights: vec![LightIntent {
            light_id: "sun".into(),
            kind: LightKind::Directional {
                direction_xyz: [0.3, -1.0, -0.2],
            },
            color_rgb: [1.0; 3],
            intensity: 1.0,
        }],
        environment: EnvironmentIntent {
            sky_top_rgb: [0.01; 3],
            sky_horizon_rgb: [0.2; 3],
            ground_rgb: [0.03; 3],
            fog_color_rgb: [0.1; 3],
            fog_density: 0.001,
            exposure: 1.0,
        },
        overlays: Vec::new(),
        capture: GraphicsCaptureRequest {
            capture_id: "synthetic".into(),
            camera_id: "campaign2-wide".into(),
            width_px: 960,
            height_px: 640,
            format: CaptureFormat::Rgba8Srgb,
            include_depth: false,
            deterministic: true,
        },
        deformation: None,
        render_policy: None,
    };
    for id in [
        "kit-tree-s0001-00",
        "kit-fern-s0001-00",
        "kit-rock-s0001-00",
        "kit-ruin-ruin-00",
        "backdrop-tile-0000",
    ] {
        let material_id = format!("{id}-material");
        let mesh_id = format!("{id}-mesh");
        body.materials.push(MaterialIntent {
            material_id: material_id.clone(),
            base_color_rgba: [0.4, 0.4, 0.4, 1.0],
            metallic: 0.0,
            roughness: 0.8,
            clearcoat: 0.0,
            clearcoat_roughness: 0.045,
            alpha_mode: luxel_native_graphics_contract::AlphaMode::Opaque,
            alpha_cutoff: None,
            double_sided: None,
            metallic_from_texture: None,
            texture_ids: Vec::new(),
            normal_texture_id: None,
            roughness_texture_id: None,
            occlusion_texture_id: None,
            emissive_texture_id: None,
            normal_scale: 1.0,
            occlusion_strength: 1.0,
            emissive_factor_rgb: [0.0; 3],
        });
        body.meshes.push(MeshPacket {
            mesh_id: mesh_id.clone(),
            positions_m: vec![[0.0, 0.0, 0.0]; 3],
            normals: vec![[0.0, 1.0, 0.0]; 3],
            uv0: vec![[0.0, 0.0]; 3],
            indices: vec![0, 1, 2],
            material_id: material_id.clone(),
            tangents: Vec::new(),
        });
        body.instances.push(InstancePacket {
            instance_id: id.into(),
            mesh_id,
            material_id,
            importance: InstanceImportance::Background,
            transform: Transform3d {
                translation_xyz_m: [0.0; 3],
                rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
                scale_xyz: [1.0; 3],
            },
            variation: None,
        });
    }
    body
}

fn cameras() -> [[f32; 3]; 3] {
    [[8.0, 5.2, 11.0], [14.0, 8.0, 18.0], [19.0, 11.0, 24.0]]
}

fn apply(
    body: &mut GraphicsScenePacketBody,
) -> luxel_native_graphics_contract::ForestProfileReceipt {
    apply_forest_profile(body, ForestProfile::Heartwood, [0.0, 0.0, 0.0], &cameras())
        .expect("Heartwood applies to the synthetic terrain")
}

fn segment_distance(point: [f32; 2], from: [f32; 2], to: [f32; 2]) -> f32 {
    let d = [to[0] - from[0], to[1] - from[1]];
    let length2 = d[0] * d[0] + d[1] * d[1];
    let t = (((point[0] - from[0]) * d[0] + (point[1] - from[1]) * d[1]) / length2).clamp(0.0, 1.0);
    let closest = [from[0] + d[0] * t, from[1] + d[1] * t];
    ((point[0] - closest[0]).powi(2) + (point[1] - closest[1]).powi(2)).sqrt()
}

fn assert_winding_matches_normals(
    mesh: &luxel_native_graphics_contract::MeshPacket,
    index_limit: usize,
) {
    for triangle in mesh.indices[..index_limit].chunks_exact(3) {
        let a = mesh.positions_m[triangle[0] as usize];
        let b = mesh.positions_m[triangle[1] as usize];
        let c = mesh.positions_m[triangle[2] as usize];
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let cross = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        let normal = (0..3)
            .map(|axis| {
                (mesh.normals[triangle[0] as usize][axis]
                    + mesh.normals[triangle[1] as usize][axis]
                    + mesh.normals[triangle[2] as usize][axis])
                    / 3.0
            })
            .collect::<Vec<_>>();
        let dot = cross[0] * normal[0] + cross[1] * normal[1] + cross[2] * normal[2];
        if cross.iter().map(|v| v * v).sum::<f32>() > 1.0e-8 {
            assert!(
                dot > 0.0,
                "inward or inverted triangle in {}: {triangle:?}",
                mesh.mesh_id
            );
        }
    }
}

#[test]
fn profile_is_deterministic_and_replacement_is_idempotent() {
    let mut a = synthetic_body();
    let mut b = synthetic_body();
    let first = apply(&mut a);
    let second = apply(&mut b);
    assert_eq!(
        a, b,
        "same synthetic input produces the same packet content"
    );
    assert_eq!(first, second);
    let repeated = apply(&mut a);
    assert_eq!(a.meshes, b.meshes, "reapplication meshes deterministic");
    assert_eq!(
        a.materials, b.materials,
        "reapplication materials deterministic"
    );
    assert_eq!(
        a.textures, b.textures,
        "reapplication textures deterministic"
    );
    assert_eq!(
        a.instances
            .iter()
            .map(|i| &i.instance_id)
            .collect::<Vec<_>>(),
        b.instances
            .iter()
            .map(|i| &i.instance_id)
            .collect::<Vec<_>>(),
        "reapplication instance ids deterministic"
    );
    assert_eq!(
        a.instances, b.instances,
        "reapplication transforms deterministic"
    );
    assert_eq!(repeated.removed_legacy_instances, first.added_instances);
}

#[test]
fn replaces_only_tree_and_fern_scatter_and_preserves_landmarks_and_rocks() {
    let mut body = synthetic_body();
    let receipt = apply(&mut body);
    assert_eq!(receipt.removed_legacy_instances, 2);
    let ids = body
        .instances
        .iter()
        .map(|i| i.instance_id.as_str())
        .collect::<Vec<_>>();
    assert!(!ids.iter().any(|id| id.starts_with("kit-tree-s")));
    assert!(!ids.iter().any(|id| id.starts_with("kit-fern-s")));
    for prefix in ["kit-rock-s", "kit-ruin-", "backdrop-"] {
        assert!(
            ids.iter().any(|id| id.starts_with(prefix)),
            "preserved {prefix}"
        );
    }
    for removed in ["kit-tree-s0001-00", "kit-fern-s0001-00"] {
        assert!(
            !body
                .meshes
                .iter()
                .any(|mesh| mesh.mesh_id == format!("{removed}-mesh"))
        );
        assert!(
            !body
                .materials
                .iter()
                .any(|material| material.material_id == format!("{removed}-material"))
        );
    }
    for preserved in [
        "kit-rock-s0001-00",
        "kit-ruin-ruin-00",
        "backdrop-tile-0000",
    ] {
        assert!(
            body.meshes
                .iter()
                .any(|mesh| mesh.mesh_id == format!("{preserved}-mesh"))
        );
        assert!(
            body.materials
                .iter()
                .any(|material| material.material_id == format!("{preserved}-material"))
        );
    }
    assert_eq!(
        receipt.added_instances,
        receipt.tree_instances * 2 + receipt.understory_instances + 2
    );
}

#[test]
fn replacement_uses_exact_scatter_id_shape_and_preserves_shared_resources() {
    let mut body = synthetic_body();
    let tree = body
        .instances
        .iter()
        .find(|instance| instance.instance_id == "kit-tree-s0001-00")
        .unwrap()
        .clone();
    let fern = body
        .instances
        .iter()
        .find(|instance| instance.instance_id == "kit-fern-s0001-00")
        .unwrap()
        .clone();
    let mut special = tree.clone();
    special.instance_id = "kit-tree-special-authored".into();
    body.instances.push(special);
    let mut suffix_lookalike = tree;
    suffix_lookalike.instance_id = "kit-tree-s0001-00-authored".into();
    body.instances.push(suffix_lookalike);
    let mut fern_hero = fern.clone();
    fern_hero.instance_id = "kit-fern-hero".into();
    body.instances.push(fern_hero);
    let mut malformed_index = fern;
    malformed_index.instance_id = "kit-fern-s0001-0x".into();
    body.instances.push(malformed_index);

    let receipt = apply(&mut body);
    assert_eq!(receipt.removed_legacy_instances, 2);
    for retained in [
        "kit-tree-special-authored",
        "kit-tree-s0001-00-authored",
        "kit-fern-hero",
        "kit-fern-s0001-0x",
    ] {
        assert!(
            body.instances
                .iter()
                .any(|instance| instance.instance_id == retained),
            "preserved authored or malformed lookalike {retained}"
        );
    }
    // The surviving authored references share the legacy tree resources, so
    // pruning must retain those resources even though the scatter instance is gone.
    assert!(
        body.meshes
            .iter()
            .any(|mesh| mesh.mesh_id == "kit-tree-s0001-00-mesh")
    );
    assert!(
        body.materials
            .iter()
            .any(|material| material.material_id == "kit-tree-s0001-00-material")
    );
}

#[test]
fn species_ages_roots_understory_litter_and_path_are_real_geometry() {
    let mut body = synthetic_body();
    let receipt = apply(&mut body);
    assert!(
        receipt.species_counts.iter().all(|count| *count > 0),
        "species counts: {:?}",
        receipt.species_counts
    );
    assert!(
        receipt.age_counts.iter().all(|count| *count > 0),
        "age counts: {:?}",
        receipt.age_counts
    );
    assert!(receipt.understory_instances > 0);
    assert!(receipt.floor_patch_count > 50);
    assert!(
        receipt.packet_geometry_bytes < 2_000_000,
        "{} bytes",
        receipt.packet_geometry_bytes
    );
    assert!(
        body.meshes
            .iter()
            .any(|mesh| mesh.mesh_id == "heartwood-mesh-trunk-redwood")
    );
    assert!(
        body.meshes
            .iter()
            .any(|mesh| mesh.mesh_id == "heartwood-mesh-path-ribbon")
    );
    assert!(
        body.meshes
            .iter()
            .any(|mesh| mesh.mesh_id == "heartwood-mesh-litter-patches")
    );
    let foliage = body
        .materials
        .iter()
        .find(|material| material.material_id == "heartwood-material-foliage-cutout")
        .unwrap();
    assert_eq!(
        foliage.alpha_mode,
        luxel_native_graphics_contract::AlphaMode::Mask
    );
    assert!(foliage.double_sided.unwrap_or(false));
    assert!(!foliage.texture_ids.is_empty());
    let redwood = body
        .meshes
        .iter()
        .find(|mesh| mesh.mesh_id == "heartwood-mesh-trunk-redwood")
        .unwrap();
    assert!(
        redwood.positions_m.iter().any(|p| p[0].abs() > 1.0),
        "buttress roots extend beyond trunk radius"
    );
    let path = body
        .meshes
        .iter()
        .find(|mesh| mesh.mesh_id == "heartwood-mesh-path-ribbon")
        .unwrap();
    assert!(
        path.positions_m.len() >= 40,
        "visible ribbon vertices are emitted"
    );
    let trunk = body
        .meshes
        .iter()
        .find(|mesh| mesh.mesh_id == "heartwood-mesh-trunk-redwood")
        .unwrap();
    // The last eight triangles describe the radial buttress fins, whose
    // normals are intentionally two-sided; check the closed trunk sidewalls.
    assert_winding_matches_normals(trunk, trunk.indices.len() - 8 * 3);
    for mesh in [
        path,
        body.meshes
            .iter()
            .find(|m| m.mesh_id == "heartwood-mesh-litter-patches")
            .unwrap(),
    ] {
        assert_winding_matches_normals(mesh, mesh.indices.len());
    }
    for mesh in body
        .meshes
        .iter()
        .filter(|mesh| mesh.mesh_id.starts_with("heartwood-mesh-crown-"))
    {
        assert_winding_matches_normals(mesh, mesh.indices.len());
    }
    let mesh = |id: &str| body.meshes.iter().find(|m| m.mesh_id == id).unwrap();
    let radius = |mesh: &luxel_native_graphics_contract::MeshPacket| {
        mesh.positions_m
            .iter()
            .map(|p| p[0].hypot(p[2]))
            .fold(0.0f32, f32::max)
    };
    let redwood_radius = radius(mesh("heartwood-mesh-crown-redwood"));
    let hemlock_radius = radius(mesh("heartwood-mesh-crown-hemlock"));
    let broadleaf_radius = radius(mesh("heartwood-mesh-crown-broadleaf"));
    assert!(
        broadleaf_radius > redwood_radius * 1.3,
        "broadleaf silhouette is wider"
    );
    assert!(
        hemlock_radius < broadleaf_radius,
        "hemlock keeps a narrow conifer silhouette"
    );
    let height = |mesh: &luxel_native_graphics_contract::MeshPacket| {
        mesh.positions_m.iter().map(|p| p[1]).fold(0.0f32, f32::max)
    };
    assert!(
        height(mesh("heartwood-mesh-trunk-redwood")) > height(mesh("heartwood-mesh-trunk-hemlock")),
        "redwood has a hero-scale trunk profile"
    );
    let tree_instance = |id: &str| {
        body.instances
            .iter()
            .find(|instance| instance.instance_id == id)
            .unwrap()
    };
    let redwood_trunk = body
        .instances
        .iter()
        .find(|instance| {
            instance.mesh_id == "heartwood-mesh-trunk-redwood"
                && instance.transform.scale_xyz[1] >= 0.9
        })
        .expect("at least one mature redwood is present");
    let redwood_crown = tree_instance(&redwood_trunk.instance_id.replace("-trunk", "-crown"));
    let trunk_top = redwood_trunk.transform.translation_xyz_m[1]
        + height(mesh("heartwood-mesh-trunk-redwood")) * redwood_trunk.transform.scale_xyz[1];
    let crown_top = redwood_crown.transform.translation_xyz_m[1]
        + height(mesh("heartwood-mesh-crown-redwood")) * redwood_crown.transform.scale_xyz[1];
    assert!(
        (trunk_top - crown_top).abs() < 0.4 * redwood_trunk.transform.scale_xyz[1],
        "redwood foliage crown meets trunk top in world metres ({trunk_top} vs {crown_top})"
    );
    assert!(
        redwood_radius * redwood_crown.transform.scale_xyz[0] >= 1.0,
        "mature redwood foliage extends at least one metre from its trunk"
    );
    for trunk_instance in body.instances.iter().filter(|instance| {
        instance.instance_id.starts_with("heartwood-tree-")
            && instance.instance_id.ends_with("-trunk")
    }) {
        let crown_instance_id = trunk_instance.instance_id.replace("-trunk", "-crown");
        let crown_instance = tree_instance(&crown_instance_id);
        let trunk_mesh = body
            .meshes
            .iter()
            .find(|mesh| mesh.mesh_id == trunk_instance.mesh_id)
            .unwrap();
        let crown_mesh = body
            .meshes
            .iter()
            .find(|mesh| mesh.mesh_id == crown_instance.mesh_id)
            .unwrap();
        let trunk_top = trunk_instance.transform.translation_xyz_m[1]
            + height(trunk_mesh) * trunk_instance.transform.scale_xyz[1];
        let crown_top = crown_instance.transform.translation_xyz_m[1]
            + height(crown_mesh) * crown_instance.transform.scale_xyz[1];
        let crown_bottom = crown_instance.transform.translation_xyz_m[1]
            + crown_mesh
                .positions_m
                .iter()
                .map(|p| p[1])
                .fold(f32::INFINITY, f32::min)
                * crown_instance.transform.scale_xyz[1];
        let scale = trunk_instance.transform.scale_xyz[1];
        assert!(
            (trunk_top - crown_top).abs() < 0.4 * scale,
            "crown meets trunk: {trunk_top} vs {crown_top}"
        );
        let trunk_height = height(trunk_mesh) * scale;
        assert!(
            crown_bottom < trunk_instance.transform.translation_xyz_m[1] + trunk_height * 0.8,
            "crown spans a substantial part of the tree"
        );
    }
    assert!(
        body.meshes
            .iter()
            .filter(|mesh| mesh.mesh_id.starts_with("heartwood-mesh-"))
            .flat_map(|mesh| &mesh.normals)
            .all(|normal| (normal.iter().map(|v| v * v).sum::<f32>() - 1.0).abs() < 1.0e-4)
    );
}

#[test]
fn trees_follow_terrain_and_respect_water_ruin_path_and_sightlines() {
    let mut body = synthetic_body();
    apply(&mut body);
    let surface = TerrainSurface::of(&body.terrain).unwrap();
    let wet = body.terrain.wet_zone.unwrap();
    let cameras = cameras();
    let path = body
        .meshes
        .iter()
        .find(|mesh| mesh.mesh_id == "heartwood-mesh-path-ribbon")
        .unwrap();
    let path_centers = path
        .positions_m
        .chunks_exact(2)
        .map(|pair| {
            [
                (pair[0][0] + pair[1][0]) * 0.5,
                (pair[0][2] + pair[1][2]) * 0.5,
            ]
        })
        .collect::<Vec<_>>();
    let ruin_footprints = luxel_native_graphics_contract::kit::ruin_footprints([0.0; 3]);
    let trees = body
        .instances
        .iter()
        .filter(|i| {
            i.instance_id.starts_with("heartwood-tree-") && i.instance_id.ends_with("-trunk")
        })
        .collect::<Vec<_>>();
    assert!(!trees.is_empty());
    for tree in trees {
        let [x, y, z] = tree.transform.translation_xyz_m;
        assert!(
            (y - surface.height(x, z)).abs() < 1e-4,
            "tree base ({x}, {y}, {z})"
        );
        let p = [x, z];
        let water = ((p[0] - wet.center_xz_m[0]) / (wet.radii_xz_m[0] * 1.55)).powi(2)
            + ((p[1] - wet.center_xz_m[1]) / (wet.radii_xz_m[1] * 1.55)).powi(2);
        assert!(water >= 1.0, "tree inside expanded water exclusion: {p:?}");
        for (center, radius) in &ruin_footprints {
            assert!(
                ((p[0] - center[0]).powi(2) + (p[1] - center[1]).powi(2)).sqrt() >= radius + 1.8,
                "tree inside ruin exclusion: {p:?}"
            );
        }
        for camera in cameras {
            assert!(
                segment_distance(p, [camera[0], camera[2]], [0.0, 0.0]) >= 3.0,
                "tree interrupts camera sightline: {p:?}"
            );
        }
        let path_gap = path_centers
            .windows(2)
            .map(|line| segment_distance(p, line[0], line[1]))
            .fold(f32::INFINITY, f32::min);
        assert!(path_gap >= 4.0, "tree inside visible path corridor: {p:?}");
    }
}

#[test]
fn unsupported_terrain_and_nonfinite_camera_inputs_fail_closed() {
    let mut body = synthetic_body();
    body.terrain.width_m = 0.0;
    assert!(
        apply_forest_profile(&mut body, ForestProfile::Heartwood, [0.0; 3], &cameras()).is_err()
    );
    let mut body = synthetic_body();
    assert!(
        apply_forest_profile(
            &mut body,
            ForestProfile::Heartwood,
            [0.0; 3],
            &[[f32::NAN, 0.0, 0.0]]
        )
        .is_err()
    );
}

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("crate is inside the workspace")
        .to_path_buf()
}

fn reference_packet() -> GraphicsScenePacket {
    let path = repo_root().join(
        "artifacts/campaign2/adapter-v6-baseline-replay-2026-09-29/run-a/graphics_scene_packet.json",
    );
    serde_json::from_slice(&std::fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "portable Campaign 2 replay fixture is unavailable at {}: {error}",
            path.display()
        )
    }))
    .expect("Campaign 2 replay fixture parses")
}

fn actual_kit2() -> KitSet {
    let root = repo_root();
    load_kit_set(&root.join("tools/kit/kit2.lock.json"), &root).unwrap_or_else(|error| {
        panic!("actual kit2 loads (build tools/build_kit.py --set kit2): {error}")
    })
}

fn synthetic_backdrop() -> BackdropSet {
    let glb = synthetic_backdrop::backdrop_glb(2, 5, 4000.0, 900.0);
    backdrop_set_from_glb("synthetic-heartwood", "sha256:synthetic", &glb)
        .expect("synthetic backdrop loads")
}

/// Uses the actual pinned kit2 assets and the same portable terrain/backdrop
/// fixtures as the convergence tests. Run with `--include-ignored` after
/// building or linking the ignored local kit2 GLBs. No GPU is used.
#[test]
#[ignore = "requires the local kit2 GLB assets and Campaign 2 replay packet"]
fn actual_kit2_heartwood_profile_is_identical_and_valid_in_all_campaign_views() {
    let reference = reference_packet();
    let kit = actual_kit2();
    println!(
        "kit2 tree materials: {:?}",
        kit.asset("tree")
            .unwrap()
            .projection
            .materials
            .iter()
            .map(|material| (
                material.material_id.as_str(),
                material.alpha_mode,
                material.alpha_cutoff,
                material.texture_ids.len(),
            ))
            .collect::<Vec<_>>()
    );
    let layers = synthetic_layers::synthetic_set();
    let backdrop = synthetic_backdrop();
    let campaign = Campaign2Inputs {
        candidate: ParityPolicyCandidate::Converge4,
        content: ParityContent {
            hero_materials: false,
            converge0: true,
        },
        terrain_layers: Some(&layers),
        kit: Some(&kit),
        backdrop: Some(&backdrop),
    };
    let inputs = Campaign2FeatureInputs {
        resolution: Campaign2LoweringInputs {
            campaign: &campaign,
            output_resolution: Campaign2OutputResolution::Native,
        },
        features: Campaign2Features {
            forest_profile: Some(ForestProfile::Heartwood),
            ..Campaign2Features::default()
        },
    };
    let packets = [
        Campaign2View::Close,
        Campaign2View::Medium,
        Campaign2View::Wide,
    ]
    .into_iter()
    .map(|view| {
        let packet = lower_campaign2_packet_with_features(&reference, view, &inputs)
            .unwrap_or_else(|error| panic!("Heartwood {view:?} lowering: {error}"));
        validate_scene_packet(&packet)
            .unwrap_or_else(|error| panic!("Heartwood {view:?} packet validation: {error}"));
        let foliage = packet
            .body
            .materials
            .iter()
            .find(|material| material.material_id == "heartwood-material-foliage-cutout")
            .expect("Heartwood foliage material is present");
        assert_eq!(
            foliage.alpha_mode,
            luxel_native_graphics_contract::AlphaMode::Mask
        );
        assert!(
            !foliage.texture_ids.is_empty(),
            "foliage uses a cutout texture"
        );
        assert!(foliage.texture_ids.iter().all(|id| {
            packet
                .body
                .textures
                .iter()
                .any(|texture| &texture.texture_id == id)
        }));
        packet
    })
    .collect::<Vec<_>>();
    for packet in &packets[1..] {
        assert_eq!(packet.body.instances, packets[0].body.instances);
        assert_eq!(packet.body.meshes, packets[0].body.meshes);
        assert_eq!(packet.body.materials, packets[0].body.materials);
        assert_eq!(packet.body.textures, packets[0].body.textures);
        assert_eq!(packet.body.terrain, packets[0].body.terrain);
    }
    let content = serde_json::json!({
        "instances": packets[0].body.instances,
        "meshes": packets[0].body.meshes,
        "materials": packets[0].body.materials,
        "textures": packets[0].body.textures,
        "terrain": packets[0].body.terrain,
    });
    let bytes = canonical_json(&content).expect("canonical content JSON");
    assert!(bytes.len() < 128 * 1024 * 1024, "{} bytes", bytes.len());
}
