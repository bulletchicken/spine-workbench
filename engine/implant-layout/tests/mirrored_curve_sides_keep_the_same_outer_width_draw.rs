//! A reflection relabels a pedicle; it does not change its latent child.
//! Widths are compared before the explicitly physical-side Chiu walls.

use implant_layout::pedicles::{CurvePosition, drawn_outer_width};
use implant_layout::readings::{NamedSampler, PedicleDraw, Version};
use shapes::implant_layout::CurveSide;
use shapes::levels::Level;
use shapes::made_up::VertebraPose;
use shapes::spine_fit::{
    FitAngles, FitRmsMm, FitStandardDeviations, FitStopReason, FittedVertebra, InferredFit,
    SpineFit,
};
use shapes::spine_readings::BodySide;

fn fit() -> SpineFit {
    SpineFit {
        vertebrae: vec![FittedVertebra {
            level: Level::T7,
            pose: VertebraPose {
                level: Level::T7,
                centre: [0.0, 0.0, 100.0],
                rotation: [0.0; 3],
                height: 20.0,
                wedge_front: 0.0,
                wedge_side: 0.0,
                top_half_width: 15.0,
                top_half_depth: 10.0,
                bottom_half_width: 15.0,
                bottom_half_depth: 10.0,
            },
            angles: FitAngles {
                side_tilt_rad: 0.0,
                forward_lean_rad: 0.0,
                twist_rad: 0.0,
            },
            sd: FitStandardDeviations {
                centre_mm: [0.0; 3],
                angles_rad: [0.0; 3],
            },
            inferred: InferredFit {
                depth: false,
                width: false,
                twist: false,
            },
            rms_mm: FitRmsMm {
                front: Some(0.0),
                side: Some(0.0),
            },
            corner_tilt_rad: None,
        }],
        side_scale: 1.0,
        side_offset_mm: 0.0,
        origin_level: Level::T7,
        iterations: 0,
        converged: true,
        stop_reason: FitStopReason::SmallStep,
        rms_mm: 0.0,
        lengths: shapes::spine_fit::FitLengths::Measured,
        typical_curve_deg: None,
        front_film: shapes::spine_fit::FilmPlacement {
            mm_per_px: [1.0; 2],
            origin_px: [0.0; 2],
            facing_assumed: false,
        },
        side_film: Some(shapes::spine_fit::FilmPlacement {
            mm_per_px: [1.0; 2],
            origin_px: [0.0; 2],
            facing_assumed: false,
        }),
    }
}

#[test]
fn a_concave_pedicle_keeps_its_width_when_reflected_to_the_other_side() {
    // Every size swaps sides and nothing else changes. The width equation has
    // no physical-side term. Chiu walls are deliberately excluded here.
    // Set all supplied latent draws to zero to isolate the side-factor
    // dimension in pedicles::draw_width.
    let sampler = NamedSampler::new(7, 32).unwrap();
    let version = Version {
        index: 2,
        fit: fit(),
        z_child: 0.0,
        pedicles: [BodySide::Left, BodySide::Right]
            .into_iter()
            .map(|side| PedicleDraw {
                level: Level::T7,
                side,
                z_pedicle: 0.0,
            })
            .collect(),
    };
    let position = CurvePosition {
        curve_side: CurveSide::Concave,
        apex_distance: 0.0,
        neutral_distance: None,
    };
    let width = |side| {
        drawn_outer_width(
            Level::T7,
            side,
            Some(14.0),
            position,
            &version,
            &sampler,
            None,
        )
        .unwrap()
        .unwrap()
    };
    assert_eq!(width(BodySide::Left), width(BodySide::Right));
}

#[test]
fn reflection_keeps_nonzero_independent_pedicle_normals_with_their_curve_role() {
    let sampler = NamedSampler::new(7, 32).unwrap();
    let version = Version {
        index: 2,
        fit: fit(),
        z_child: 0.3,
        pedicles: vec![
            PedicleDraw {
                level: Level::T7,
                side: BodySide::Left,
                z_pedicle: 0.4,
            },
            PedicleDraw {
                level: Level::T7,
                side: BodySide::Right,
                z_pedicle: -0.7,
            },
        ],
    };
    for curve_side in [CurveSide::Concave, CurveSide::Convex] {
        let position = CurvePosition {
            curve_side,
            apex_distance: 0.0,
            neutral_distance: None,
        };
        let draw = |version: &Version, side| {
            drawn_outer_width(
                Level::T7,
                side,
                Some(14.0),
                position,
                version,
                &sampler,
                None,
            )
            .unwrap()
            .unwrap()
        };
        assert_eq!(
            draw(&version, BodySide::Left),
            draw(&version, BodySide::Right)
        );
        let mut changed = version.clone();
        for pedicle in &mut changed.pedicles {
            pedicle.z_pedicle += 0.5;
        }
        assert_ne!(
            draw(&version, BodySide::Left),
            draw(&changed, BodySide::Left),
            "the independent normal is retained rather than discarded"
        );
    }
}
