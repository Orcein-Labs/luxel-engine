use std::fs;
use std::path::PathBuf;
use std::process::Command;

use luxel_asset_contract::{
    AssetPreparationReceipt, AssetPreparationRequest, CharacterEvaluationError, CharacterSelection,
    CpuCharacterPose, PreparationStatus, RuntimeFindingCode, decode_character_glb, inspect_glb,
    prepare_asset,
};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize)]
struct RuntimeFixture {
    schema_version: String,
    request: AssetPreparationRequest,
    source: SourceFixture,
    expected_status: PreparationStatus,
    expected_findings: Vec<RuntimeFindingCode>,
}

#[derive(Debug, Deserialize)]
struct SourceFixture {
    animation_clips: Vec<String>,
    inject_out_of_range_joint: bool,
    #[serde(default)]
    static_motion_clips: Vec<String>,
    #[serde(default)]
    inactive_skinned_mesh: bool,
    #[serde(default)]
    socket_under_mesh: bool,
    #[serde(default)]
    required_extensions: Vec<String>,
}

fn fixture(name: &str) -> RuntimeFixture {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/fixtures/asset_runtime")
        .join(format!("{name}.json"));
    let bytes = fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let mut value: Value = serde_json::from_slice(&bytes).expect("fixture JSON is valid");
    if let Some(base_name) = value
        .as_object_mut()
        .and_then(|object| object.remove("copy_request_from"))
        .and_then(|value| value.as_str().map(str::to_owned))
    {
        let base_path = path.with_file_name(format!("{base_name}.json"));
        let base: Value = serde_json::from_slice(
            &fs::read(&base_path)
                .unwrap_or_else(|error| panic!("{}: {error}", base_path.display())),
        )
        .expect("base fixture JSON is valid");
        value["request"] = base["request"].clone();
    }
    let fixture: RuntimeFixture = serde_json::from_value(value).expect("fixture JSON is typed");
    assert_eq!(
        fixture.schema_version,
        "luxel.asset-runtime-test-fixture/v1"
    );
    fixture
}

fn make_glb(source: &SourceFixture) -> Vec<u8> {
    let mut binary = Vec::new();
    let positions = [
        [0.0f32, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [0.0, 0.0, 1.0],
    ];
    for point in positions {
        for value in point {
            binary.extend_from_slice(&value.to_le_bytes());
        }
    }
    let indices = [0u16, 2, 1, 0, 1, 3, 0, 3, 2, 1, 2, 3];
    for index in indices {
        binary.extend_from_slice(&index.to_le_bytes());
    }
    for vertex in 0..4 {
        for component in 0..4 {
            let joint = if source.inject_out_of_range_joint && vertex == 0 && component == 1 {
                2u16
            } else if component == 1 {
                1u16
            } else {
                0u16
            };
            binary.extend_from_slice(&joint.to_le_bytes());
        }
    }
    for _ in 0..4 {
        for weight in [0.75f32, 0.25, 0.0, 0.0] {
            binary.extend_from_slice(&weight.to_le_bytes());
        }
    }
    let identity = [
        1.0f32, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    for _ in 0..2 {
        for value in identity {
            binary.extend_from_slice(&value.to_le_bytes());
        }
    }
    for time in [0.0f32, 1.0] {
        binary.extend_from_slice(&time.to_le_bytes());
    }
    for _ in 0..2 {
        for component in [0.0f32, 0.0, 0.0, 1.0] {
            binary.extend_from_slice(&component.to_le_bytes());
        }
    }
    let locomotion_delta = if source
        .static_motion_clips
        .iter()
        .any(|name| name == "locomotion")
    {
        [0.0f32; 6]
    } else {
        [0.0f32, 0.0, 0.0, 0.0, 0.0, 0.25]
    };
    for component in locomotion_delta {
        binary.extend_from_slice(&component.to_le_bytes());
    }
    let attack_pose = if source
        .static_motion_clips
        .iter()
        .any(|name| name == "attack")
    {
        [0.0f32, 0.0, 0.0, 1.0]
    } else {
        [0.0f32, 0.0, 0.70710677, 0.70710677]
    };
    for component in [0.0f32, 0.0, 0.0, 1.0].into_iter().chain(attack_pose) {
        binary.extend_from_slice(&component.to_le_bytes());
    }
    assert_eq!(binary.len(), 392);

    let attributes = json!({"POSITION": 0, "JOINTS_0": 2, "WEIGHTS_0": 3});
    let primitive = json!({"attributes": attributes, "indices": 1, "mode": 4});
    let meshes = [
        json!({"name": "body_lod0", "primitives": [primitive]}),
        json!({"name": "body_lod1", "primitives": [primitive]}),
    ];
    let animations = source
        .animation_clips
        .iter()
        .map(|name| {
            let (output, path) = match name.as_str() {
                "locomotion" => (7, "translation"),
                "attack" => (8, "rotation"),
                _ => (6, "rotation"),
            };
            json!({
                "name": name,
                "samplers": [{"input": 5, "output": output, "interpolation": "LINEAR"}],
                "channels": [{"sampler": 0, "target": {"node": 1, "path": path}}]
            })
        })
        .collect::<Vec<_>>();
    let scene_roots = if source.inactive_skinned_mesh {
        json!([0, 4])
    } else {
        json!([0, 3, 4])
    };
    let nodes = if source.socket_under_mesh {
        json!([
            {"name": "root", "children": [1]},
            {"name": "hand_r"},
            {"name": "weapon_mount", "extras": {"luxel_socket": true}},
            {"name": "body_lod0_node", "mesh": 0, "skin": 0, "children": [2]},
            {"name": "body_lod1_node", "mesh": 1, "skin": 0}
        ])
    } else {
        json!([
            {"name": "root", "children": [1]},
            {"name": "hand_r", "children": [2]},
            {"name": "weapon_mount", "extras": {"luxel_socket": true}},
            {"name": "body_lod0_node", "mesh": 0, "skin": 0},
            {"name": "body_lod1_node", "mesh": 1, "skin": 0}
        ])
    };
    let document = json!({
        "asset": {"version": "2.0", "generator": "luxel-runtime-fixture"},
        "scene": 0,
        "scenes": [{"nodes": scene_roots}],
        "nodes": nodes,
        "meshes": meshes,
        "skins": [{"name": "humanoid", "inverseBindMatrices": 4, "skeleton": 0, "joints": [0, 1]}],
        "animations": animations,
        "extensionsRequired": source.required_extensions,
        "extensionsUsed": source.required_extensions,
        "buffers": [{"byteLength": binary.len()}],
        "bufferViews": [
            {"buffer": 0, "byteOffset": 0, "byteLength": 48},
            {"buffer": 0, "byteOffset": 48, "byteLength": 24},
            {"buffer": 0, "byteOffset": 72, "byteLength": 32},
            {"buffer": 0, "byteOffset": 104, "byteLength": 64},
            {"buffer": 0, "byteOffset": 168, "byteLength": 128},
            {"buffer": 0, "byteOffset": 296, "byteLength": 8},
            {"buffer": 0, "byteOffset": 304, "byteLength": 32},
            {"buffer": 0, "byteOffset": 336, "byteLength": 24},
            {"buffer": 0, "byteOffset": 360, "byteLength": 32}
        ],
        "accessors": [
            {"bufferView": 0, "componentType": 5126, "count": 4, "type": "VEC3", "min": [0, 0, 0], "max": [1, 2, 1]},
            {"bufferView": 1, "componentType": 5123, "count": 12, "type": "SCALAR"},
            {"bufferView": 2, "componentType": 5123, "count": 4, "type": "VEC4"},
            {"bufferView": 3, "componentType": 5126, "count": 4, "type": "VEC4"},
            {"bufferView": 4, "componentType": 5126, "count": 2, "type": "MAT4"},
            {"bufferView": 5, "componentType": 5126, "count": 2, "type": "SCALAR", "min": [0], "max": [1]},
            {"bufferView": 6, "componentType": 5126, "count": 2, "type": "VEC4"},
            {"bufferView": 7, "componentType": 5126, "count": 2, "type": "VEC3"},
            {"bufferView": 8, "componentType": 5126, "count": 2, "type": "VEC4"}
        ]
    });
    let mut json_bytes = serde_json::to_vec(&document).expect("fixture glTF JSON serializes");
    json_bytes.resize((json_bytes.len() + 3) & !3, b' ');
    binary.resize((binary.len() + 3) & !3, 0);
    let total = 12 + 8 + json_bytes.len() + 8 + binary.len();
    let mut glb = Vec::with_capacity(total);
    glb.extend_from_slice(b"glTF");
    glb.extend_from_slice(&2u32.to_le_bytes());
    glb.extend_from_slice(&(total as u32).to_le_bytes());
    glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x4e4f_534au32.to_le_bytes());
    glb.extend_from_slice(&json_bytes);
    glb.extend_from_slice(&(binary.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x004e_4942u32.to_le_bytes());
    glb.extend_from_slice(&binary);
    glb
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CpuFault {
    None,
    Joint,
    Weight,
    Buffer,
    NoInverseBind,
    PositiveQuaternion,
    Cubic,
    Morph,
    MorphChannel,
    ExtraInfluence,
    Sparse,
    External,
    RequiredExtension,
    DuplicateNode,
    Hierarchy,
    SingularMesh,
    DeepHierarchy,
    TooManyNodes,
}

fn cpu_character_glb(fault: CpuFault) -> Vec<u8> {
    let mut binary = Vec::new();
    for value in [1.0f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
        binary.extend_from_slice(&value.to_le_bytes());
    }
    for (vertex, joints) in [[0u16, 1, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]
        .into_iter()
        .enumerate()
    {
        for (component, joint) in joints.into_iter().enumerate() {
            let joint = if fault == CpuFault::Joint && vertex == 0 && component == 0 {
                2
            } else {
                joint
            };
            binary.extend_from_slice(&joint.to_le_bytes());
        }
    }
    let vertex_weights = if fault == CpuFault::Weight {
        [
            [0.5f32, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
        ]
    } else {
        [
            [0.5f32, 0.5, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
        ]
    };
    for vertex in vertex_weights {
        for value in vertex {
            binary.extend_from_slice(&value.to_le_bytes());
        }
    }
    for translation in [3.0f32, 2.0] {
        for value in [
            1.0f32,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            translation,
            0.0,
            0.0,
            1.0,
        ] {
            binary.extend_from_slice(&value.to_le_bytes());
        }
    }
    for value in [0.0f32, 1.0] {
        binary.extend_from_slice(&value.to_le_bytes());
    }
    let end_rotation = if fault == CpuFault::PositiveQuaternion {
        [0.0f32, 0.0, 0.70710677, 0.70710677]
    } else {
        [0.0f32, 0.0, -0.70710677, -0.70710677]
    };
    for value in [0.0f32, 0.0, 0.0, 1.0].into_iter().chain(end_rotation) {
        binary.extend_from_slice(&value.to_le_bytes());
    }
    for value in [
        10.0f32, 0.0, 0.0, 12.0, 0.0, 0.0, 10.0, 0.0, 0.0, 12.0, 0.0, 0.0,
    ] {
        binary.extend_from_slice(&value.to_le_bytes());
    }
    assert_eq!(binary.len(), 324);
    let mut primitive = json!({"attributes": {"POSITION": 0, "JOINTS_0": 1, "WEIGHTS_0": 2}});
    if fault == CpuFault::ExtraInfluence {
        primitive["attributes"]["JOINTS_1"] = json!(1);
        primitive["attributes"]["WEIGHTS_1"] = json!(2);
    }
    if fault == CpuFault::Morph {
        primitive["targets"] = json!([{}]);
    }
    let mut nodes = json!([
        {"name":"scene_parent", "translation":[10,0,0], "children":[1,3,5]},
        {"name":"root", "children":[2]},
        {"name":"hand_r", "translation":[1,0,0], "children":[4]},
        {"name":"body_lod0_node", "mesh":0, "skin":0, "translation":[3,0,0]},
        {"name":"weapon_mount", "translation":[0,2,0], "extras":{"luxel_socket":true}},
        {"name":"body_lod1_node", "mesh":1, "skin":0}
    ]);
    if fault == CpuFault::DuplicateNode {
        nodes[5]["name"] = json!("body_lod0_node");
    }
    if fault == CpuFault::Hierarchy {
        nodes[2]["children"] = json!([1, 4]);
    }
    if fault == CpuFault::SingularMesh {
        nodes[3]["scale"] = json!([0, 1, 1]);
    }
    if matches!(fault, CpuFault::DeepHierarchy | CpuFault::TooManyNodes) {
        let count = if fault == CpuFault::DeepHierarchy {
            luxel_asset_contract::MAX_CPU_CHARACTER_HIERARCHY_DEPTH
        } else {
            luxel_asset_contract::MAX_CPU_CHARACTER_NODES
        };
        let first = nodes.as_array().unwrap().len();
        nodes[0]["children"]
            .as_array_mut()
            .unwrap()
            .push(json!(first));
        for index in 0..count {
            let mut node = json!({"name": format!("depth_guard_{index}")});
            if index + 1 < count {
                node["children"] = json!([first + index + 1]);
            }
            nodes.as_array_mut().unwrap().push(node);
        }
    }
    let mut pose = json!({
        "name":"pose",
        "samplers":[
            {"input":4,"output":5,"interpolation":"LINEAR"},
            {"input":4,"output":6,"interpolation":"LINEAR"}
        ],
        "channels":[
            {"sampler":0,"target":{"node":2,"path":"rotation"}},
            {"sampler":1,"target":{"node":0,"path":"translation"}}
        ]
    });
    if fault == CpuFault::Cubic {
        pose["samplers"][0]["interpolation"] = json!("CUBICSPLINE");
    }
    if fault == CpuFault::MorphChannel {
        pose["channels"]
            .as_array_mut()
            .unwrap()
            .push(json!({"sampler":0,"target":{"node":2,"path":"weights"}}));
    }
    let mut document = json!({
        "asset":{"version":"2.0"}, "scene":0, "scenes":[{"nodes":[0]}], "nodes":nodes,
        "meshes":[
            {"name":"body_lod0","primitives":[primitive.clone()]},
            {"name":"body_lod1","primitives":[primitive]}
        ],
        "skins":[{"inverseBindMatrices":3,"skeleton":1,"joints":[1,2]}],
        "animations":[pose,{
            "name":"step","samplers":[{"input":4,"output":7,"interpolation":"STEP"}],
            "channels":[{"sampler":0,"target":{"node":0,"path":"translation"}}]
        }],
        "buffers":[{"byteLength":324}],
        "bufferViews":[
            {"buffer":0,"byteOffset":0,"byteLength":36}, {"buffer":0,"byteOffset":36,"byteLength":24},
            {"buffer":0,"byteOffset":60,"byteLength":48}, {"buffer":0,"byteOffset":108,"byteLength":128},
            {"buffer":0,"byteOffset":236,"byteLength":8}, {"buffer":0,"byteOffset":244,"byteLength":32},
            {"buffer":0,"byteOffset":276,"byteLength":24}, {"buffer":0,"byteOffset":300,"byteLength":24}
        ],
        "accessors":[
            {"bufferView":0,"componentType":5126,"count":3,"type":"VEC3"},
            {"bufferView":1,"componentType":5123,"count":3,"type":"VEC4"},
            {"bufferView":2,"componentType":5126,"count":3,"type":"VEC4"},
            {"bufferView":3,"componentType":5126,"count":2,"type":"MAT4"},
            {"bufferView":4,"componentType":5126,"count":2,"type":"SCALAR"},
            {"bufferView":5,"componentType":5126,"count":2,"type":"VEC4"},
            {"bufferView":6,"componentType":5126,"count":2,"type":"VEC3"},
            {"bufferView":7,"componentType":5126,"count":2,"type":"VEC3"}
        ]
    });
    if fault == CpuFault::Sparse {
        document["accessors"][0]["sparse"] = json!({});
    }
    if fault == CpuFault::External {
        document["buffers"][0]["uri"] = json!("external.bin");
    }
    if fault == CpuFault::Buffer {
        document["bufferViews"][0]["byteLength"] = json!(400);
    }
    if fault == CpuFault::RequiredExtension {
        document["extensionsRequired"] = json!(["EXT_unimplemented_cpu_test"]);
    }
    if fault == CpuFault::NoInverseBind {
        document["skins"][0]
            .as_object_mut()
            .unwrap()
            .remove("inverseBindMatrices");
    }
    let mut json_bytes = serde_json::to_vec(&document).unwrap();
    json_bytes.resize((json_bytes.len() + 3) & !3, b' ');
    let total = 12 + 8 + json_bytes.len() + 8 + binary.len();
    let mut glb = Vec::with_capacity(total);
    glb.extend_from_slice(b"glTF");
    glb.extend_from_slice(&2u32.to_le_bytes());
    glb.extend_from_slice(&(total as u32).to_le_bytes());
    glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x4e4f_534au32.to_le_bytes());
    glb.extend_from_slice(&json_bytes);
    glb.extend_from_slice(&(binary.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x004e_4942u32.to_le_bytes());
    glb.extend_from_slice(&binary);
    glb
}

fn run_fixture(name: &str) -> (RuntimeFixture, AssetPreparationReceipt) {
    run_custom_case(fixture(name))
}

fn run_custom_case(mut fixture: RuntimeFixture) -> (RuntimeFixture, AssetPreparationReceipt) {
    let bytes = make_glb(&fixture.source);
    fixture.request.expected_source_sha256 = Some(
        inspect_glb(&bytes)
            .expect("fixture GLB is inspectable")
            .identity
            .source_sha256,
    );
    let receipt =
        prepare_asset(&bytes, &fixture.request).expect("fixture GLB is structurally valid");
    (fixture, receipt)
}

fn source_pinned_fixture(name: &str) -> (RuntimeFixture, Vec<u8>) {
    let mut fixture = fixture(name);
    let bytes = make_glb(&fixture.source);
    fixture.request.expected_source_sha256 = Some(
        inspect_glb(&bytes)
            .expect("fixture GLB is inspectable")
            .identity
            .source_sha256,
    );
    (fixture, bytes)
}

#[test]
fn known_good_character_builds_a_stable_runtime_package() {
    let (fixture, first) = run_fixture("good_character");
    let bytes = make_glb(&fixture.source);
    let second = prepare_asset(&bytes, &fixture.request).expect("same fixture prepares twice");

    assert_eq!(first.status, PreparationStatus::Ready);
    assert!(first.findings.is_empty());
    assert_eq!(first, second);
    let package = first
        .package
        .expect("ready receipt has an inspectable package");
    assert!(package.package_id.starts_with("runtime_asset_sha256_"));
    assert_eq!(
        package.rig.as_ref().unwrap().joint_names,
        ["root", "hand_r"]
    );
    assert_eq!(package.animations.len(), 3);
    assert_eq!(package.sockets.len(), 1);
    assert_eq!(package.lods.len(), 2);
    assert_eq!(
        package.provenance.source_sha256,
        first.source_identity.source_sha256
    );
    assert_eq!(first.receipt_sha256, second.receipt_sha256);
}

#[test]
fn known_bad_rig_animation_collision_and_extension_cases_are_rejected() {
    for name in [
        "broken_rig",
        "missing_animation",
        "missing_collision",
        "inactive_mesh",
        "required_extension",
    ] {
        let (fixture, receipt) = run_fixture(name);
        assert_eq!(receipt.status, fixture.expected_status, "case {name}");
        assert!(
            receipt.package.is_none(),
            "case {name} must not produce a package"
        );
        for expected in fixture.expected_findings {
            assert!(
                receipt
                    .findings
                    .iter()
                    .any(|finding| finding.code == expected),
                "case {name} is missing {expected:?}; findings: {:?}",
                receipt.findings
            );
        }
    }
}

#[test]
fn inactive_skinned_lod_is_rejected() {
    let mut case = fixture("good_character");
    case.source.inactive_skinned_mesh = true;
    let (_, receipt) = run_custom_case(case);
    assert_eq!(receipt.status, PreparationStatus::Rejected);
    assert!(receipt.package.is_none());
    assert!(
        receipt
            .findings
            .iter()
            .any(|finding| finding.code == RuntimeFindingCode::InactiveSkinnedMesh)
    );
}

#[test]
fn locomotion_and_attack_must_have_sampled_pose_deltas() {
    for role in ["locomotion", "attack"] {
        let mut case = fixture("good_character");
        case.source.static_motion_clips = vec![role.into()];
        let (_, receipt) = run_custom_case(case);
        assert_eq!(receipt.status, PreparationStatus::Rejected, "{role}");
        assert!(receipt.package.is_none(), "{role}");
        assert!(
            receipt.findings.iter().any(|finding| {
                finding.code == RuntimeFindingCode::AnimationNoMotion && finding.subject == role
            }),
            "{role}: {:?}",
            receipt.findings
        );
    }
}

#[test]
fn socket_parent_must_be_a_valid_joint_and_the_declared_direct_parent() {
    let mut non_joint_parent = fixture("good_character");
    non_joint_parent.source.socket_under_mesh = true;
    non_joint_parent.request.required_sockets[0].parent_joint_name = "body_lod0_node".into();
    let (_, receipt) = run_custom_case(non_joint_parent);
    assert_eq!(receipt.status, PreparationStatus::Rejected);
    assert!(
        receipt
            .findings
            .iter()
            .any(|finding| finding.code == RuntimeFindingCode::InvalidSocketHierarchy)
    );

    let mut wrong_direct_parent = fixture("good_character");
    wrong_direct_parent.request.required_sockets[0].parent_joint_name = "root".into();
    let (_, receipt) = run_custom_case(wrong_direct_parent);
    assert_eq!(receipt.status, PreparationStatus::Rejected);
    assert!(
        receipt
            .findings
            .iter()
            .any(|finding| finding.code == RuntimeFindingCode::InvalidSocketHierarchy)
    );
}

#[test]
fn unsupported_required_gltf_extension_is_rejected() {
    let mut case = fixture("good_character");
    case.source.required_extensions = vec!["EXT_unimplemented_luxel_test".into()];
    let (_, receipt) = run_custom_case(case);
    assert_eq!(receipt.status, PreparationStatus::Rejected);
    assert!(
        receipt
            .findings
            .iter()
            .any(|finding| { finding.code == RuntimeFindingCode::UnsupportedRequiredExtension })
    );
}

#[test]
fn source_digest_pin_is_checked_by_rust() {
    let (fixture, bytes) = source_pinned_fixture("good_character");
    let mut request = fixture.request;
    request.expected_source_sha256 = Some("0".repeat(64));
    let receipt = prepare_asset(&bytes, &request).expect("source remains structurally valid");
    assert_eq!(receipt.status, PreparationStatus::Rejected);
    assert!(
        receipt
            .findings
            .iter()
            .any(|finding| { finding.code == RuntimeFindingCode::SourceDigestMismatch })
    );
}

#[test]
fn prepare_request_without_source_digest_pin_is_rejected() {
    let (fixture, bytes) = source_pinned_fixture("good_character");
    let mut request = fixture.request;
    request.expected_source_sha256 = None;
    let receipt = prepare_asset(&bytes, &request).expect("source remains structurally valid");
    assert_eq!(receipt.status, PreparationStatus::Rejected);
    assert!(
        receipt
            .findings
            .iter()
            .any(|finding| { finding.code == RuntimeFindingCode::MissingSourceDigestPin })
    );
}

#[test]
fn native_cli_exits_green_for_ready_and_red_for_rejected_packages() {
    for (name, expected_exit) in [
        ("good_character", 0),
        ("missing_collision", 3),
        ("inactive_mesh", 3),
        ("required_extension", 3),
    ] {
        let (fixture, bytes) = source_pinned_fixture(name);
        let stem = format!("luxel-asset-runtime-{}-{name}", std::process::id());
        let asset_path = std::env::temp_dir().join(format!("{stem}.glb"));
        let request_path = std::env::temp_dir().join(format!("{stem}.json"));
        fs::write(&asset_path, bytes).expect("temporary fixture GLB is writable");
        fs::write(
            &request_path,
            serde_json::to_vec(&fixture.request).expect("request serializes"),
        )
        .expect("temporary request is writable");

        let output = Command::new(env!("CARGO_BIN_EXE_luxel-asset-contract"))
            .arg("prepare")
            .arg(&asset_path)
            .arg(&request_path)
            .output()
            .expect("native asset preparation CLI starts");
        assert_eq!(
            output.status.code(),
            Some(expected_exit),
            "case {name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let receipt: AssetPreparationReceipt =
            serde_json::from_slice(&output.stdout).expect("CLI returns a typed JSON receipt");
        assert_eq!(receipt.status, fixture.expected_status, "case {name}");
        eprintln!(
            "known case {name}: prepare exit {} / {:?}",
            output.status.code().unwrap(),
            receipt.status
        );

        fs::remove_file(asset_path).expect("temporary fixture GLB is removed");
        fs::remove_file(request_path).expect("temporary request is removed");
    }
}

#[test]
fn fixtures_are_not_accidentally_empty() {
    for name in [
        "good_character",
        "broken_rig",
        "missing_animation",
        "missing_collision",
    ] {
        let fixture = fixture(name);
        assert_eq!(
            fixture.schema_version,
            "luxel.asset-runtime-test-fixture/v1"
        );
    }
}

fn cpu_selection(clip: &str) -> CharacterSelection {
    CharacterSelection::new("body_lod0_node", clip, vec!["weapon_mount".into()])
}

fn assert_vector_close(actual: &[f64], expected: &[f64]) {
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert!(
            (*actual - *expected).abs() <= 1e-6,
            "{actual} != {expected}"
        );
    }
}

fn assert_pose_close(left: &CpuCharacterPose, right: &CpuCharacterPose) {
    assert_eq!(left.sample_time_seconds, right.sample_time_seconds);
    assert_eq!(
        left.positions_mesh_local.len(),
        right.positions_mesh_local.len()
    );
    for (left, right) in left
        .positions_mesh_local
        .iter()
        .zip(&right.positions_mesh_local)
    {
        assert_vector_close(left, right);
    }
    assert_eq!(left.sockets.len(), right.sockets.len());
    for (left, right) in left.sockets.iter().zip(&right.sockets) {
        assert_eq!(left.name, right.name);
        assert_vector_close(&left.scene_from_node, &right.scene_from_node);
    }
    assert_vector_close(&left.bounds_mesh_local.min, &right.bounds_mesh_local.min);
    assert_vector_close(&left.bounds_mesh_local.max, &right.bounds_mesh_local.max);
}

#[test]
fn cpu_character_evaluates_real_glb_bytes_without_claiming_production_readiness() {
    let bytes = cpu_character_glb(CpuFault::None);
    let mut request = fixture("good_character").request;
    request.expected_source_sha256 = Some(inspect_glb(&bytes).unwrap().identity.source_sha256);
    let receipt = prepare_asset(&bytes, &request).unwrap();
    assert_eq!(receipt.status, PreparationStatus::Rejected);
    assert!(receipt.package.is_none());
    assert!(
        receipt
            .findings
            .iter()
            .any(|finding| { finding.code == RuntimeFindingCode::MissingRequiredAnimation })
    );

    let character = decode_character_glb(&bytes, &cpu_selection("pose")).unwrap();
    assert_eq!(character.joint_node_indices(), &[1, 2]);
    assert_eq!(
        character.source_sha256(),
        inspect_glb(&bytes).unwrap().identity.source_sha256
    );
    assert_eq!(character.selection(), &cpu_selection("pose"));
    let rest = character.evaluate(0.0).unwrap();
    for (actual, expected) in
        rest.positions_mesh_local
            .iter()
            .zip([[1.0, 0.0, 0.0], [0.0; 3], [0.0, 1.0, 0.0]])
    {
        assert_vector_close(actual, &expected);
    }
    assert_vector_close(&rest.sockets[0].scene_from_node[12..14], &[11.0, 2.0]);
    let before_start = character.evaluate(-2.0).unwrap();
    assert_eq!(before_start.sample_time_seconds, 0.0);
    assert_pose_close(&rest, &before_start);

    let middle = character.evaluate(0.5).unwrap();
    let diagonal = 3.0 / (2.0 * 2.0f64.sqrt());
    assert_vector_close(
        &middle.positions_mesh_local[0],
        &[diagonal - 0.5, diagonal, 0.0],
    );
    assert_vector_close(
        &middle.sockets[0].scene_from_node[12..14],
        &[12.0 - 2.0f64.sqrt(), 2.0f64.sqrt()],
    );
    assert_eq!(middle, character.evaluate(0.5).unwrap());

    let end = character.evaluate(3.0).unwrap();
    assert_eq!(end.sample_time_seconds, 1.0);
    assert_vector_close(&end.positions_mesh_local[0], &[-0.5, 1.5, 0.0]);
    assert_vector_close(&end.bounds_mesh_local.min, &[-0.5, 0.0, 0.0]);
    assert_vector_close(&end.bounds_mesh_local.max, &[0.0, 1.5, 0.0]);
    assert_vector_close(&end.sockets[0].scene_from_node[12..14], &[11.0, 0.0]);
    assert!(matches!(
        character.evaluate(f64::NAN),
        Err(CharacterEvaluationError::NonFiniteTime)
    ));
    assert_pose_close(&middle, &character.evaluate(0.5).unwrap());

    let positive_sign = decode_character_glb(
        &cpu_character_glb(CpuFault::PositiveQuaternion),
        &cpu_selection("pose"),
    )
    .unwrap();
    assert_pose_close(&middle, &positive_sign.evaluate(0.5).unwrap());

    let step = decode_character_glb(&bytes, &cpu_selection("step")).unwrap();
    assert_vector_close(
        &step.evaluate(0.75).unwrap().sockets[0].scene_from_node[12..14],
        &[11.0, 2.0],
    );
    let no_inverse_bind = decode_character_glb(
        &cpu_character_glb(CpuFault::NoInverseBind),
        &cpu_selection("pose"),
    )
    .unwrap();
    assert_vector_close(
        &no_inverse_bind.evaluate(0.0).unwrap().positions_mesh_local[0],
        &[-1.5, 0.0, 0.0],
    );
}

#[test]
fn cpu_character_rejects_unsupported_or_corrupt_glb_profiles() {
    for (fault, expected) in [
        (CpuFault::Joint, "joint"),
        (CpuFault::Weight, "weight"),
        (CpuFault::Buffer, "buffer"),
        (CpuFault::Cubic, "cubic"),
        (CpuFault::Morph, "morph"),
        (CpuFault::MorphChannel, "morph channel"),
        (CpuFault::ExtraInfluence, "extra influence"),
        (CpuFault::Sparse, "sparse"),
        (CpuFault::External, "external buffer"),
        (CpuFault::RequiredExtension, "extension"),
        (CpuFault::DuplicateNode, "ambiguous selection"),
        (CpuFault::Hierarchy, "hierarchy"),
        (CpuFault::SingularMesh, "singular mesh"),
        (CpuFault::DeepHierarchy, "hierarchy depth limit"),
        (CpuFault::TooManyNodes, "node limit"),
    ] {
        let result = decode_character_glb(&cpu_character_glb(fault), &cpu_selection("pose"));
        let rejected = match (expected, &result) {
            ("joint", Err(CharacterEvaluationError::InvalidJointIndex(_))) => true,
            ("weight", Err(CharacterEvaluationError::InvalidWeights(_))) => true,
            ("buffer", Err(CharacterEvaluationError::InvalidGlb(_))) => true,
            ("cubic", Err(CharacterEvaluationError::UnsupportedCubicSpline)) => true,
            ("morph", Err(CharacterEvaluationError::UnsupportedMorphTargets)) => true,
            ("morph channel", Err(CharacterEvaluationError::UnsupportedMorphChannel)) => true,
            ("extra influence", Err(CharacterEvaluationError::UnsupportedInfluenceSet)) => true,
            ("sparse", Err(CharacterEvaluationError::SparseAccessor(_))) => true,
            ("external buffer", Err(CharacterEvaluationError::ExternalBuffer)) => true,
            ("extension", Err(CharacterEvaluationError::UnsupportedRequiredExtension(_))) => true,
            ("ambiguous selection", Err(CharacterEvaluationError::AmbiguousSelection(_))) => true,
            ("hierarchy", Err(CharacterEvaluationError::InvalidHierarchy(_))) => true,
            ("singular mesh", Err(CharacterEvaluationError::InvalidTransform(_))) => true,
            (
                "hierarchy depth limit" | "node limit",
                Err(CharacterEvaluationError::UnsupportedFeature(detail)),
            ) => detail.contains(expected),
            _ => false,
        };
        assert!(rejected, "{expected}: {result:?}");
    }
}
