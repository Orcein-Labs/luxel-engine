use luxel_native_graphics_contract::render_policy::{
    ContactAoPolicy, PointShadowsPolicy, RenderPolicy, ShadowCascadesPolicy, ShadowFitPolicy,
    validate_render_policy,
};

fn cascades() -> ShadowCascadesPolicy {
    ShadowCascadesPolicy::CONVERGE4
}

#[test]
fn absent_axes_preserve_the_empty_policy_json() {
    assert_eq!(
        serde_json::to_string(&RenderPolicy::default()).unwrap(),
        "{}"
    );
}

#[test]
fn cascade_preset_round_trips_as_the_named_optional_axis() {
    let policy = RenderPolicy {
        shadow_cascades: Some(cascades()),
        ..RenderPolicy::default()
    };
    validate_render_policy(&policy).expect("preset must be valid");
    let value = serde_json::to_value(policy).unwrap();
    assert_eq!(
        value["shadow_cascades"],
        serde_json::json!({
            "near_distance_m": 30,
            "far_distance_m": 250,
            "blend_width_m": 6,
            "near_map_size_px": 2048,
            "far_map_size_px": 4096
        })
    );
    let decoded: RenderPolicy = serde_json::from_value(value).unwrap();
    assert_eq!(decoded.shadow_cascades, Some(cascades()));
    assert_eq!(decoded.resolved().shadow_cascades, Some(cascades()));
}

#[test]
fn cascade_ranges_splits_and_map_sides_are_validated() {
    let invalid = [
        ShadowCascadesPolicy {
            near_distance_m: 7,
            ..cascades()
        },
        ShadowCascadesPolicy {
            near_distance_m: 2001,
            ..cascades()
        },
        ShadowCascadesPolicy {
            far_distance_m: 2001,
            ..cascades()
        },
        ShadowCascadesPolicy {
            blend_width_m: 0,
            ..cascades()
        },
        ShadowCascadesPolicy {
            blend_width_m: 101,
            ..cascades()
        },
        ShadowCascadesPolicy {
            blend_width_m: 60,
            ..cascades()
        },
        ShadowCascadesPolicy {
            far_distance_m: 33,
            ..cascades()
        },
        ShadowCascadesPolicy {
            near_map_size_px: 1536,
            ..cascades()
        },
        ShadowCascadesPolicy {
            far_map_size_px: 256,
            ..cascades()
        },
    ];
    for axis in invalid {
        assert!(
            validate_render_policy(&RenderPolicy {
                shadow_cascades: Some(axis),
                ..RenderPolicy::default()
            })
            .is_err(),
            "accepted invalid cascade configuration {axis:?}"
        );
    }
}

#[test]
fn cascades_and_single_fit_cannot_be_combined() {
    let policy = RenderPolicy {
        shadow_fit: Some(ShadowFitPolicy {
            view_distance_m: 60,
            map_size_px: None,
        }),
        shadow_cascades: Some(cascades()),
        ..RenderPolicy::default()
    };
    let error = validate_render_policy(&policy).unwrap_err();
    assert!(error.message.contains("cannot both be set"), "{error}");
}

#[test]
fn contact_ao_bounds_are_checked_without_enabling_other_axes() {
    let valid = RenderPolicy {
        contact_ao: Some(ContactAoPolicy {
            radius_cm: 100,
            strength_bp: 0,
            bias_mm: 100,
        }),
        ..RenderPolicy::default()
    };
    validate_render_policy(&valid).expect("boundary values are valid");
    for ao in [
        ContactAoPolicy {
            radius_cm: 0,
            strength_bp: 1000,
            bias_mm: 0,
        },
        ContactAoPolicy {
            radius_cm: 201,
            strength_bp: 1000,
            bias_mm: 0,
        },
        ContactAoPolicy {
            radius_cm: 100,
            strength_bp: 10001,
            bias_mm: 0,
        },
        ContactAoPolicy {
            radius_cm: 100,
            strength_bp: 1000,
            bias_mm: 101,
        },
    ] {
        let policy = RenderPolicy {
            contact_ao: Some(ao),
            ..RenderPolicy::default()
        };
        assert!(validate_render_policy(&policy).is_err(), "accepted {ao:?}");
    }
}

#[test]
fn cascade_and_contact_axes_reject_unknown_fields() {
    for json in [
        r#"{"shadow_cascades":{"near_distance_m":30,"far_distance_m":250,"blend_width_m":6,"near_map_size_px":2048,"far_map_size_px":4096,"foo":1}}"#,
        r#"{"contact_ao":{"radius_cm":100,"strength_bp":5000,"bias_mm":10,"foo":1}}"#,
        r#"{"point_shadows":{"map_size_px":256,"foo":1}}"#,
    ] {
        assert!(
            serde_json::from_str::<RenderPolicy>(json).is_err(),
            "accepted {json}"
        );
    }
}

#[test]
fn cascade_and_contact_axes_require_complete_integer_shapes() {
    for json in [
        r#"{"shadow_cascades":{"far_distance_m":250,"blend_width_m":6,"near_map_size_px":2048,"far_map_size_px":4096}}"#,
        r#"{"shadow_cascades":{"near_distance_m":true,"far_distance_m":250,"blend_width_m":6,"near_map_size_px":2048,"far_map_size_px":4096}}"#,
        r#"{"shadow_cascades":{"near_distance_m":30,"far_distance_m":250,"blend_width_m":6,"near_map_size_px":2048.0,"far_map_size_px":4096}}"#,
        r#"{"contact_ao":{"radius_cm":100,"strength_bp":5000}}"#,
        r#"{"contact_ao":{"radius_cm":100,"strength_bp":true,"bias_mm":10}}"#,
        r#"{"point_shadows":{}}"#,
        r#"{"point_shadows":{"map_size_px":true}}"#,
    ] {
        assert!(
            serde_json::from_str::<RenderPolicy>(json).is_err(),
            "accepted {json}"
        );
    }
}

#[test]
fn point_shadow_map_sizes_are_exactly_256_or_512() {
    for size in [256, 512] {
        let policy = RenderPolicy {
            point_shadows: Some(PointShadowsPolicy { map_size_px: size }),
            ..RenderPolicy::default()
        };
        validate_render_policy(&policy).expect("allowed face size");
    }
    for size in [128, 384, 1024, 0] {
        let policy = RenderPolicy {
            point_shadows: Some(PointShadowsPolicy { map_size_px: size }),
            ..RenderPolicy::default()
        };
        let error = validate_render_policy(&policy).expect_err("refused face size");
        assert!(
            error.message.contains("point_shadows.map_size_px"),
            "{error}"
        );
    }
}
