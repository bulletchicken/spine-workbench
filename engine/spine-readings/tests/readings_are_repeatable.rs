//! Independent quantiles, strata, named-angle composition and batch identity.

use forward_model::rotation::matrix_from_rotation_vector;
use nalgebra::Vector3;
use shapes::levels::Level;
use shapes::made_up::VertebraPose;
use shapes::spine_fit::{
    FitAngles, FitRmsMm, FitStandardDeviations, FitStopReason, FittedVertebra, InferredFit,
    SpineFit,
};
use shapes::spine_readings::BodySide;
use spine_readings::ReadingsError;
use spine_readings::readings::{NamedSampler, inverse_normal, version, versions};

fn fit() -> SpineFit {
    SpineFit {
        vertebrae: [Level::T7, Level::T8]
            .into_iter()
            .enumerate()
            .map(|(i, level)| FittedVertebra {
                level,
                pose: VertebraPose {
                    level,
                    centre: [0.0, -12.0, 200.0 - 30.0 * i as f64],
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
                    centre_mm: [1.0, 2.0, 3.0],
                    angles_rad: [0.01, 0.02, 0.03],
                },
                inferred: InferredFit {
                    depth: false,
                    width: false,
                    twist: false,
                },
                rms_mm: FitRmsMm {
                    front: Some(0.25),
                    side: Some(0.25),
                },
                corner_tilt_rad: None,
            })
            .collect(),
        side_scale: 1.0,
        side_offset_mm: 0.0,
        origin_level: Level::T8,
        iterations: 0,
        converged: true,
        stop_reason: FitStopReason::SmallStep,
        rms_mm: 0.25,
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
fn every_dimension_places_one_draw_in_each_stratum() {
    for n in [1, 2, 3, 7, 32, 65, 127] {
        for seed in [0, 1, 35, u64::MAX] {
            let sampler = NamedSampler::new(seed, n).unwrap();
            for name in ["geometry", "a/b:3", "", "a-dimension-added-later"] {
                let mut strata = vec![0; n as usize];
                for k in 0..n {
                    let u = sampler.uniform(name, k).unwrap();
                    assert!(u > 0.0 && u < 1.0);
                    strata[libm::floor(u * f64::from(n)) as usize] += 1;
                }
                assert!(strata.iter().all(|&count| count == 1), "{name}, {n}");
            }
        }
    }
}

#[test]
fn adding_or_removing_other_dimensions_never_moves_a_draw() {
    let sampler = NamedSampler::new(53, 32).unwrap();
    let alone = sampler.uniform("readings-example/weight", 17).unwrap();
    let normal = sampler.normal("readings-example/weight", 17).unwrap();
    for name in [
        "readings-example/ribs",
        "new/dimension",
        "versions-geometry/0",
    ] {
        for k in (0..32).rev() {
            let _ = sampler.uniform(name, k).unwrap();
        }
    }
    assert_eq!(
        alone.to_bits(),
        sampler
            .uniform("readings-example/weight", 17)
            .unwrap()
            .to_bits()
    );
    assert_eq!(
        normal.to_bits(),
        sampler
            .normal("readings-example/weight", 17)
            .unwrap()
            .to_bits()
    );
    assert_eq!(normal.to_bits(), inverse_normal(alone).unwrap().to_bits());
    assert_ne!(
        alone,
        NamedSampler::new(54, 32)
            .unwrap()
            .uniform("readings-example/weight", 17)
            .unwrap()
    );
}

#[test]
fn a_version_is_bit_identical_alone_and_in_any_batch() {
    let fit = fit();
    let whole = versions(&fit, 6, 32, 0..32).unwrap();
    let mut split = versions(&fit, 6, 32, 0..5).unwrap().versions;
    split.extend(versions(&fit, 6, 32, 5..29).unwrap().versions);
    split.extend(versions(&fit, 6, 32, 29..32).unwrap().versions);
    assert_eq!(whole.versions, split);
    assert_eq!(whole.nominal, fit);
    assert_eq!(whole.total, 32);
    for k in 0..32 {
        // Bit checks distinguish signed zero as well as numerical equality.
        let alone = version(&fit, 6, 32, k).unwrap();
        assert_eq!(alone, whole.versions[k as usize]);
        for (a, b) in alone
            .fit
            .vertebrae
            .iter()
            .zip(&whole.versions[k as usize].fit.vertebrae)
        {
            for (a, b) in a
                .pose
                .centre
                .iter()
                .chain(&a.pose.rotation)
                .zip(b.pose.centre.iter().chain(&b.pose.rotation))
            {
                assert_eq!(a.to_bits(), b.to_bits());
            }
        }
    }
    assert!(versions(&fit, 6, 32, 7..7).unwrap().versions.is_empty());
}

#[test]
fn known_normal_quantiles_and_symmetric_tails_are_resolved() {
    // Standard normal reference quantiles, independently tabulated.
    let cases = [
        (0.5, 0.0),
        (0.975, 1.959_963_984_540_054),
        (0.025, -1.959_963_984_540_054),
        (0.001, -3.090_232_306_167_813),
        (1e-12, -7.034_483_825_301_132),
        (1e-300, -37.047_096_299_361_2),
        (1e-310, -37.663_060_331_949_52),
        (f64::from_bits(1), -38.467_405_617_144_344),
    ];
    for (probability, expected) in cases {
        let got = inverse_normal(probability).unwrap();
        assert!((got - expected).abs() < 1e-9, "{probability}: {got}");
    }
    assert_eq!(inverse_normal(0.5).unwrap().to_bits(), 0.0_f64.to_bits());
    for p in [0.125, 0.25, 0.375] {
        assert_eq!(
            inverse_normal(p).unwrap().to_bits(),
            (-inverse_normal(1.0 - p).unwrap()).to_bits()
        );
    }
    for p in [0.0, 1.0, -0.2, f64::INFINITY, f64::NAN] {
        assert!(inverse_normal(p).is_err());
    }
}

#[test]
fn centres_and_independent_plates_keep_their_named_streams() {
    let nominal = fit();
    let sampler = NamedSampler::new(5, 32).unwrap();
    let version = version(&nominal, 5, 32, 9).unwrap();
    assert_eq!(
        version.z_child,
        sampler.normal("versions-pedicles/child", 9).unwrap()
    );
    for (i, (got, original)) in version
        .fit
        .vertebrae
        .iter()
        .zip(&nominal.vertebrae)
        .enumerate()
    {
        assert_eq!(got.inferred, original.inferred);
        assert_eq!(got.sd, original.sd);
        for axis in 0..3 {
            let centre_z = sampler
                .normal(&format!("versions-geometry/{i}/centre/{axis}"), 9)
                .unwrap();
            assert_eq!(
                got.pose.centre[axis],
                original.pose.centre[axis] + original.sd.centre_mm[axis] * centre_z
            );
        }
        let plates = forward_model::endplates::Endplates::of(
            &forward_model::vertebra_params::VertebraParams::from(&got.pose),
        );
        for (plane, name, sd, actual) in [
            (
                0,
                "front",
                original.sd.angles_rad[0],
                [
                    spine_readings::curves::geometry::upper_coronal_tilt_deg(got),
                    spine_readings::curves::geometry::lower_coronal_tilt_deg(got),
                ],
            ),
            (
                1,
                "side",
                original.sd.angles_rad[1],
                [
                    spine_readings::curves::geometry::sagittal_tilt_deg(&plates.top.normal),
                    spine_readings::curves::geometry::sagittal_tilt_deg(&plates.bottom.normal),
                ],
            ),
        ] {
            for (j, label) in ["upper", "lower"].into_iter().enumerate() {
                let expected = (core::f64::consts::SQRT_2
                    * sd
                    * sampler
                        .normal(&format!("versions-geometry/{i}/plate/{name}/{label}"), 9)
                        .unwrap())
                .to_degrees();
                assert!(
                    (actual[j] - expected).abs() < 1e-10,
                    "plane={plane} plate={j}"
                );
            }
        }
        assert_eq!(
            got.angles.twist_rad,
            original.sd.angles_rad[2]
                * sampler
                    .normal(&format!("versions-geometry/{i}/angle/2"), 9)
                    .unwrap()
        );
        let angles = [
            got.angles.side_tilt_rad,
            got.angles.forward_lean_rad,
            got.angles.twist_rad,
        ];
        // Written out by hand: the third column of Rx(a) Ry(b) Rz(c)
        // is (sin b, -sin a cos b, cos a cos b), independent of c.
        let rotation = matrix_from_rotation_vector(&Vector3::from(got.pose.rotation));
        let expected_up = Vector3::new(
            libm::sin(angles[1]),
            -libm::sin(angles[0]) * libm::cos(angles[1]),
            libm::cos(angles[0]) * libm::cos(angles[1]),
        );
        assert!((rotation.column(2) - expected_up).norm() < 1e-14);
        for (side, name) in [(BodySide::Left, "left"), (BodySide::Right, "right")] {
            assert_eq!(
                version.pedicle(got.level, side).unwrap().z_pedicle,
                sampler
                    .normal(&format!("versions-pedicles/{i}/{name}"), 9)
                    .unwrap()
            );
        }
    }
    assert_ne!(version.pedicles[0].z_pedicle, version.pedicles[1].z_pedicle);
    assert_ne!(version.pedicles[0].z_pedicle, version.pedicles[2].z_pedicle);
}

#[test]
fn zero_uncertainty_and_renaming_do_not_change_numerical_geometry() {
    let original = fit();
    let mut zero = original.clone();
    for vertebra in &mut zero.vertebrae {
        vertebra.sd.centre_mm = [0.0; 3];
        vertebra.sd.angles_rad = [0.0; 3];
    }
    assert_eq!(version(&zero, 9, 7, 3).unwrap().fit, zero);
    let mut renamed = original.clone();
    for vertebra in &mut renamed.vertebrae {
        vertebra.level = vertebra.level.below().unwrap();
        vertebra.pose.level = vertebra.level;
    }
    let a = version(&original, 9, 7, 3).unwrap();
    let b = version(&renamed, 9, 7, 3).unwrap();
    assert_eq!(a.z_child, b.z_child);
    for (a, b) in a.fit.vertebrae.iter().zip(&b.fit.vertebrae) {
        assert_eq!(a.pose.centre, b.pose.centre);
        assert_eq!(a.pose.rotation, b.pose.rotation);
        assert_eq!(a.angles, b.angles);
    }
    assert_eq!(
        a.pedicles.iter().map(|p| p.z_pedicle).collect::<Vec<_>>(),
        b.pedicles.iter().map(|p| p.z_pedicle).collect::<Vec<_>>()
    );
}

#[test]
fn invalid_ensembles_and_spreads_are_refused() {
    assert!(NamedSampler::new(0, 0).is_err());
    assert!(NamedSampler::new(0, 3).unwrap().uniform("x", 3).is_err());
    let mut fit = fit();
    assert!(versions(&fit, 0, 3, 0..4).is_err());
    let reversed_start = 2;
    assert!(versions(&fit, 0, 3, reversed_start..1).is_err());
    fit.vertebrae[0].sd.angles_rad[0] = -1.0;
    assert!(version(&fit, 0, 3, 0).is_err());
    fit.vertebrae.clear();
    assert_eq!(version(&fit, 0, 3, 0).unwrap_err(), ReadingsError::NoSpine);
}

#[test]
fn typical_side_values_and_inferred_twist_are_not_drawn() {
    let mut nominal = fit();
    nominal.side_film = None;
    for v in &mut nominal.vertebrae {
        v.inferred.twist = true;
        v.inferred.depth = true;
    }
    for sampled in versions(&nominal, 7, 32, 0..32).unwrap().versions {
        for (a, b) in nominal.vertebrae.iter().zip(&sampled.fit.vertebrae) {
            assert_eq!(a.pose.centre[0].to_bits(), b.pose.centre[0].to_bits());
            assert_eq!(
                a.angles.forward_lean_rad.to_bits(),
                b.angles.forward_lean_rad.to_bits()
            );
            assert_eq!(a.angles.twist_rad.to_bits(), b.angles.twist_rad.to_bits());
        }
    }
}
