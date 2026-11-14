//! Independent plate draws preserve their projected angles and missing side geometry.

use forward_model::endplates::Endplates;
use forward_model::rotation::{rot_x, rot_y, rot_z, rotation_vector_from_matrix};
use forward_model::vertebra_params::VertebraParams;
use shapes::levels::Level;
use shapes::made_up::VertebraPose;
use shapes::spine_fit::{
    FitAngles, FitRmsMm, FitStandardDeviations, FitStopReason, FittedVertebra, InferredFit,
    SpineFit,
};
use spine_readings::curves::geometry::{
    lower_coronal_tilt_deg, sagittal_tilt_deg, upper_coronal_tilt_deg,
};
use spine_readings::readings::{NamedSampler, versions};

fn nominal() -> SpineFit {
    SpineFit {
        vertebrae: [Level::T8]
            .into_iter()
            .enumerate()
            .map(|(i, level)| FittedVertebra {
                level,
                pose: VertebraPose {
                    level,
                    centre: [10.0, -20.0, 240.0 - 30.0 * i as f64],
                    rotation: [0.0; 3],
                    height: 20.0,
                    wedge_front: 0.0,
                    wedge_side: 0.0,
                    top_half_width: 15.0,
                    bottom_half_width: 15.0,
                    top_half_depth: 10.0,
                    bottom_half_depth: 10.0,
                },
                angles: FitAngles {
                    side_tilt_rad: 0.0,
                    forward_lean_rad: 0.0,
                    twist_rad: 0.0,
                },
                sd: FitStandardDeviations {
                    centre_mm: [0.5 + i as f64, 1.5 + i as f64, 2.5 + i as f64],
                    angles_rad: [0.005, 0.015, 0.025],
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
            })
            .collect(),
        side_scale: 1.0,
        side_offset_mm: 0.0,
        origin_level: Level::T9,
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

fn tilted() -> SpineFit {
    let mut fit = nominal();
    let v = &mut fit.vertebrae[0];
    v.angles = FitAngles {
        side_tilt_rad: 0.65,
        forward_lean_rad: 0.55,
        twist_rad: 0.75,
    };
    v.pose.wedge_front = 0.3;
    v.pose.wedge_side = -0.2;
    v.pose.rotation = rotation_vector_from_matrix(
        &(rot_x(v.angles.side_tilt_rad)
            * rot_y(v.angles.forward_lean_rad)
            * rot_z(v.angles.twist_rad)),
    )
    .into();
    v.sd.centre_mm = [0.0; 3];
    fit
}

fn plate_angles(v: &FittedVertebra) -> [[f64; 2]; 2] {
    let plates = Endplates::of(&VertebraParams::from(&v.pose));
    [
        [upper_coronal_tilt_deg(v), lower_coronal_tilt_deg(v)],
        [
            sagittal_tilt_deg(&plates.top.normal),
            sagittal_tilt_deg(&plates.bottom.normal),
        ],
    ]
}

fn assert_front_draws(fit: &SpineFit, sampled: &SpineFit, k: u32) {
    let sampler = NamedSampler::new(39, 32).unwrap();
    let original = &fit.vertebrae[0];
    let before = plate_angles(original);
    let after = plate_angles(&sampled.vertebrae[0]);
    for (plate, label) in ["upper", "lower"].into_iter().enumerate() {
        let z = sampler
            .normal(&format!("versions-geometry/0/plate/front/{label}"), k)
            .unwrap();
        let expected = before[0][plate]
            + (core::f64::consts::SQRT_2 * original.sd.angles_rad[0] * z).to_degrees();
        assert!((after[0][plate] - expected).abs() < 1e-8);
    }
}

#[test]
fn angled_plates_keep_all_four_independent_projected_draws() {
    assert_plate_draws(&tilted());
}

fn assert_plate_draws(fit: &SpineFit) {
    let before = plate_angles(&fit.vertebrae[0]);
    let sampler = NamedSampler::new(39, 32).unwrap();
    let ensemble = versions(fit, 39, 32, 0..32).unwrap();
    for sample in ensemble.versions {
        assert_front_draws(fit, &sample.fit, sample.index);
        let after = plate_angles(&sample.fit.vertebrae[0]);
        for (plate, label) in ["upper", "lower"].into_iter().enumerate() {
            let z = sampler
                .normal(
                    &format!("versions-geometry/0/plate/side/{label}"),
                    sample.index,
                )
                .unwrap();
            let expected = before[1][plate]
                + (core::f64::consts::SQRT_2 * fit.vertebrae[0].sd.angles_rad[1] * z).to_degrees();
            assert!((after[1][plate] - expected).abs() < 1e-8);
        }
    }
}

#[test]
fn a_side_wedge_seen_nearly_end_on_keeps_its_draws() {
    let mut fit = tilted();
    let v = &mut fit.vertebrae[0];
    v.corner_tilt_rad = Some(0.03);
    v.angles.side_tilt_rad = -0.08;
    v.angles.forward_lean_rad = 0.25;
    v.angles.twist_rad = 1.51;
    v.pose.wedge_front = -0.045;
    v.pose.wedge_side = 0.04;
    v.pose.rotation = rotation_vector_from_matrix(
        &(rot_x(v.angles.side_tilt_rad)
            * rot_y(v.angles.forward_lean_rad)
            * rot_z(v.angles.twist_rad)),
    )
    .into();
    v.sd.angles_rad = [0.05, 0.04, 0.0];
    v.inferred.twist = true;
    assert_plate_draws(&fit);
}

#[test]
fn front_draws_keep_unseen_lean_and_side_wedge_fixed() {
    for has_partial_side in [false, true] {
        let mut fit = tilted();
        if !has_partial_side {
            fit.side_film = None;
        }
        let original = &mut fit.vertebrae[0];
        original.inferred.depth = true;
        original.inferred.twist = true;
        let lean = original.angles.forward_lean_rad;
        let wedge = original.pose.wedge_side;
        let twist = original.angles.twist_rad;
        let ensemble = versions(&fit, 39, 32, 0..32).unwrap();
        for sample in ensemble.versions {
            assert_front_draws(&fit, &sample.fit, sample.index);
            let v = &sample.fit.vertebrae[0];
            assert_eq!(v.angles.forward_lean_rad.to_bits(), lean.to_bits());
            assert_eq!(v.pose.wedge_side.to_bits(), wedge.to_bits());
            assert_eq!(v.angles.twist_rad.to_bits(), twist.to_bits());
            assert_eq!(v.corner_tilt_rad, None);
        }
    }
}

#[test]
fn zero_spreads_keep_the_entire_angled_fit_unchanged() {
    let mut fit = tilted();
    fit.vertebrae[0].sd.angles_rad = [0.0; 3];
    let ensemble = versions(&fit, 39, 32, 0..32).unwrap();
    for sample in ensemble.versions {
        assert_eq!(sample.fit, fit);
    }
}

#[test]
fn a_pure_twist_draw_keeps_a_flat_plate_at_ninety_degrees_of_lean() {
    let mut fit = nominal();
    let v = &mut fit.vertebrae[0];
    v.sd.centre_mm = [0.0; 3];
    v.sd.angles_rad = [0.0, 0.0, 0.02];
    v.angles.forward_lean_rad = core::f64::consts::FRAC_PI_2;
    v.pose.rotation = rotation_vector_from_matrix(&rot_y(v.angles.forward_lean_rad)).into();
    for sample in versions(&fit, 39, 32, 0..32).unwrap().versions {
        let v = &sample.fit.vertebrae[0];
        let plates = Endplates::of(&VertebraParams::from(&v.pose));
        assert!((plates.top.normal - nalgebra::Vector3::x()).norm() < 1e-14);
        assert!((plates.bottom.normal - nalgebra::Vector3::x()).norm() < 1e-14);
        assert_ne!(v.angles.twist_rad, 0.0);
        assert_eq!(v.pose.wedge_front, 0.0);
        assert_eq!(v.pose.wedge_side, 0.0);
    }
}

#[test]
fn incompatible_projected_angles_are_refused() {
    let mut fit = nominal();
    let v = &mut fit.vertebrae[0];
    v.angles.side_tilt_rad = 1.56;
    v.angles.forward_lean_rad = 0.2;
    v.pose.rotation = rotation_vector_from_matrix(&(rot_x(1.56) * rot_y(0.2))).into();
    v.sd.angles_rad = [0.02, 0.0, 0.0];
    // A side angle below 90 degrees requires an upward normal. Some
    // independent front draws cross 90, so no common normal can fit both.
    assert!(versions(&fit, 39, 32, 0..32).is_err());
}
