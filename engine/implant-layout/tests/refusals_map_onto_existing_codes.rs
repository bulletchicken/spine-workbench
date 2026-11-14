//! Every refusal crosses to the app as an existing error code, with its
//! own name in the detail, as the joint fit's refusals do.

use implant_layout::LayoutError;
use shapes::errors::{EngineError, ErrorCode};

#[test]
fn each_refusal_takes_an_existing_code_and_keeps_its_name() {
    let cases = [
        (
            LayoutError::Refused("no-instrumented-levels".to_string()),
            ErrorCode::ToolRefused,
            "implant-layout-refused: no-instrumented-levels",
        ),
        (
            LayoutError::InvalidValue {
                field: "versions".to_string(),
                reason: "zero".to_string(),
            },
            ErrorCode::CaseInvalidValue,
            "implant-layout-invalid-value: versions: zero",
        ),
        (
            LayoutError::NoSpine,
            ErrorCode::PassNoSpineFound,
            "implant-layout-no-spine",
        ),
    ];
    for (error, code, name) in cases {
        let crossed = EngineError::from(error);
        assert_eq!(crossed.code, code, "{name}");
        assert_eq!(crossed.message, name);
        assert_eq!(crossed.detail.as_deref(), Some(name));
    }
}

#[test]
fn rebuilding_a_layout_refuses_with_its_unbuilt_reason() {
    use implant_layout::rebuild_layout;
    use shapes::implant_layout::{ChildFacts, LayoutInput, NamingDoubt, RebuildInput, RodMaterial};
    use shapes::levels::Level;
    use shapes::spine_fit::{FilmPlacement, FitLengths, FitStopReason, SpineFit};

    let input = RebuildInput {
        layout: LayoutInput {
            fit: SpineFit {
                vertebrae: Vec::new(),
                side_scale: 1.0,
                side_offset_mm: 0.0,
                origin_level: Level::L5,
                iterations: 0,
                converged: true,
                stop_reason: FitStopReason::SmallStep,
                rms_mm: 0.0,
                lengths: FitLengths::Measured,
                typical_curve_deg: None,
                front_film: FilmPlacement {
                    mm_per_px: [1.0; 2],
                    origin_px: [0.0; 2],
                    facing_assumed: false,
                },
                side_film: None,
            },
            front: None,
            bend_left: None,
            bend_right: None,
            child: ChildFacts {
                age_years: None,
                weight_kg: None,
                activity: None,
            },
            naming: NamingDoubt::AsRead,
            versions: 32,
            seed: 1,
            pedicle_child_share: None,
        },
        upper: Level::T4,
        lower: Level::L1,
        anchors: Vec::new(),
        rod_material: RodMaterial::CobaltChrome,
        rod_diameter_mm: 5.5,
    };
    let error = rebuild_layout(&input).unwrap_err();
    assert_eq!(error, LayoutError::RebuildNotBuilt);
    let crossed = EngineError::from(error);
    assert_eq!(crossed.code, ErrorCode::ToolRefused);
    assert_eq!(crossed.detail.as_deref(), Some("rebuild-not-built"));
}
