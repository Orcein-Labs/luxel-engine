use luxel_native_graphics_contract::{
    CalibrationRig, GraphicsScenePacket, LightIntent, LightKind, MAX_POINT_LIGHTS,
    PointShadowsPolicy, seal_scene_packet,
};
use std::path::PathBuf;

fn body() -> luxel_native_graphics_contract::GraphicsScenePacketBody {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let path = root.join("artifacts/campaign2/adapter-v6-baseline-replay-2026-09-29/run-a/graphics_scene_packet.json");
    serde_json::from_slice::<GraphicsScenePacket>(&std::fs::read(path).unwrap())
        .unwrap()
        .body
}

fn point(index: usize) -> LightIntent {
    LightIntent {
        light_id: format!("local-{index}"),
        kind: LightKind::Point {
            position_xyz_m: [1.0, 2.0, 3.0],
            range_m: 5.0,
        },
        color_rgb: [1.0, 0.4, 0.1],
        intensity: 18.0,
    }
}

#[test]
fn native_lighting_budget_and_invalid_inputs_are_admitted_before_execution() {
    let mut base = body();
    base.lights.truncate(1);
    assert_eq!(MAX_POINT_LIGHTS, 4);
    for count in 0..=MAX_POINT_LIGHTS {
        let mut b = base.clone();
        b.lights.extend((0..count).map(point));
        assert!(seal_scene_packet(b).is_ok(), "{count} local lights");
    }
    for mutation in 0..8 {
        let mut b = base.clone();
        b.lights.push(point(0));
        match mutation {
            0 => b.lights.extend((1..5).map(point)),
            1 => {
                b.lights.remove(0);
            }
            2 => {
                let mut second = b.lights[0].clone();
                second.light_id = "second-sun".into();
                b.lights.push(second);
            }
            3 => b.lights[1].intensity = f32::NAN,
            4 => b.lights[1].intensity = -1.0,
            5 => {
                b.lights[1].kind = LightKind::Point {
                    position_xyz_m: [0.0; 3],
                    range_m: 0.0,
                }
            }
            6 => {
                b.lights[1].kind = LightKind::Point {
                    position_xyz_m: [f32::INFINITY, 0.0, 0.0],
                    range_m: 5.0,
                }
            }
            _ => b.lights[1].light_id = b.lights[0].light_id.clone(),
        }
        assert!(seal_scene_packet(b).is_err(), "invalid mutation {mutation}");
    }
}

#[test]
fn local_calibration_rigs_change_only_light_color_between_moods() {
    let mut base = body();
    let mut staging = base.instances[0].clone();
    staging.instance_id = "scene::calibration-ground_plane".into();
    staging.transform.translation_xyz_m = [-16.0, 12.0, -26.5];
    staging.transform.rotation_xyzw = [0.0, 1.0, 0.0, 0.0];
    staging.transform.scale_xyz = [1.0; 3];
    base.instances.push(staging);
    let mut warm = base.clone();
    let mut cool = base.clone();
    let mut off = base.clone();
    CalibrationRig::LocalWarm.apply(&mut warm).unwrap();
    CalibrationRig::LocalCool.apply(&mut cool).unwrap();
    CalibrationRig::LocalOff.apply(&mut off).unwrap();
    assert_eq!(warm.lights.len(), 5);
    assert_eq!(off.lights.len(), 1);
    assert_eq!(warm.environment, cool.environment);
    assert_eq!(warm.environment, off.environment);
    assert_eq!(warm.materials, base.materials);
    assert_eq!(warm.instances, base.instances);
    assert_eq!(warm.render_policy, cool.render_policy);
    assert_eq!(
        warm.render_policy.as_ref().unwrap().point_shadows,
        Some(PointShadowsPolicy { map_size_px: 512 })
    );
    assert!(
        off.render_policy
            .as_ref()
            .and_then(|policy| policy.point_shadows)
            .is_none()
    );
    assert_eq!(warm.lights[0], cool.lights[0]);
    assert_eq!(warm.lights[0], off.lights[0]);
    for (a, b) in warm.lights.iter().skip(1).zip(cool.lights.iter().skip(1)) {
        assert_eq!(a.kind, b.kind);
        assert_eq!(a.light_id, b.light_id);
        assert_eq!(a.intensity, b.intensity);
        assert_ne!(a.color_rgb, b.color_rgb);
    }
    for rig in CalibrationRig::LOCAL {
        assert_eq!(CalibrationRig::parse(rig.name()), Some(rig));
        assert!(!CalibrationRig::ALL.contains(&rig));
        assert!(!rig.exposure_calibrated());
    }
    assert!(CalibrationRig::LocalWarm.apply(&mut body()).is_err());
}
