//! Primitive known answers read from independently worked fixtures.

use forward_model::rotation::matrix_from_rotation_vector;
use load_bench::chain_solver::BorderedSystem;
use load_bench::joint_springs::{Stiffness, rotation_deformation};
use load_bench::rod_beams::{BeamProperties, CondensedRod, polyline_at};
use load_bench::rod_statics::{HeadLoad, goodman_equivalent, section_at, surface_stress};
use nalgebra::{DMatrix, DVector, Matrix3, Matrix6, SVector, Vector3};
use serde_json::Value;

fn fixture(text: &str, count: usize) -> Value {
    let value: Value = serde_json::from_str(text).expect("fixture JSON");
    assert_eq!(value["cases"].as_array().expect("cases").len(), count);
    value
}

fn number(value: &Value) -> f64 {
    let result = value.as_f64().expect("fixture number");
    assert!(result.is_finite());
    result
}

fn vector(value: &Value) -> Vector3<f64> {
    let values = value.as_array().expect("fixture vector");
    assert_eq!(values.len(), 3);
    Vector3::new(number(&values[0]), number(&values[1]), number(&values[2]))
}

fn close(actual: f64, expected: f64, tolerance: &Value) {
    assert!(actual.is_finite(), "non-finite result against {expected}");
    let bound = number(&tolerance["absolute"]) + number(&tolerance["relative"]) * expected.abs();
    assert!(
        (actual - expected).abs() <= bound,
        "{actual:.17e} against {expected:.17e}, bound {bound:.3e}"
    );
}

fn close_vector(actual: &Vector3<f64>, expected: &Value, tolerance: &Value) {
    for i in 0..3 {
        close(actual[i], number(&expected[i]), tolerance);
    }
}

#[test]
fn one_joint_uses_the_exact_log_at_large_angles() {
    let fixture = fixture(
        include_str!(
            "../../../proving-ground/known-answers/load-bench/one-joint-at-large-angles.json"
        ),
        9,
    );
    for case in fixture["cases"].as_array().expect("cases") {
        let input = &case["input"];
        let expect = &case["expect"];
        let tolerance = &fixture["tolerance"];
        let axis = vector(&input["axisUnit"]);
        let moment = vector(&input["momentNmm"]);
        let stiffness = number(&input["stiffnessNmmPerRad"]);
        let angle = axis.dot(&moment) / stiffness;
        close(angle, number(&expect["angleRad"]), tolerance);
        let rotation = matrix_from_rotation_vector(&(axis * angle));
        let deformation = rotation_deformation(
            &Matrix3::identity(),
            &Matrix3::identity(),
            &rotation,
            &Matrix3::identity(),
        );
        close_vector(&deformation, &expect["rotationVectorRad"], tolerance);
        let spring = Stiffness::new([stiffness; 6]).expect("positive fixture stiffness");
        let mut six = SVector::<f64, 6>::zeros();
        six.fixed_rows_mut::<3>(3).copy_from(&deformation);
        let response = spring.response(&six);
        close_vector(
            &response.fixed_rows::<3>(3).into_owned(),
            &input["momentNmm"],
            tolerance,
        );
        for row in 0..3 {
            for column in 0..3 {
                close(
                    rotation[(row, column)],
                    number(&expect["rotationMatrix"][row][column]),
                    tolerance,
                );
            }
        }
    }
}

#[test]
fn three_head_rod_moments_match_both_free_body_cuts() {
    let fixture = fixture(
        include_str!("../../../proving-ground/known-answers/load-bench/three-head-rod.json"),
        2,
    );
    for case in fixture["cases"].as_array().expect("cases") {
        let input = &case["input"];
        let expect = &case["expect"];
        let tolerance = &fixture["tolerance"];
        let polyline: Vec<_> = input["centreLineMm"]
            .as_array()
            .expect("centre line")
            .iter()
            .map(vector)
            .collect();
        let heads: Vec<_> = input["heads"]
            .as_array()
            .expect("heads")
            .iter()
            .enumerate()
            .map(|(i, head)| HeadLoad {
                station_mm: number(&head["sMm"]),
                position_mm: vector(&head["positionMm"]),
                force_n: vector(&head["forceN"]),
                couple_nmm: vector(&input["headCouplesNmm"][i]),
            })
            .collect();
        let force = heads
            .iter()
            .fold(Vector3::zeros(), |sum, h| sum + h.force_n);
        let moment = heads.iter().fold(Vector3::zeros(), |sum, h| {
            sum + h.position_mm.cross(&h.force_n) + h.couple_nmm
        });
        close_vector(&force, &expect["netForceN"], tolerance);
        close_vector(&moment, &expect["netMomentAboutOriginNmm"], tolerance);
        let stations = input["stationsMm"].as_array().expect("stations");
        let answers = expect["stations"].as_array().expect("answers");
        assert_eq!(stations.len(), answers.len());
        for (station, answer) in stations.iter().zip(answers) {
            let s = number(station);
            let section = section_at(&polyline, &heads, s).expect("section");
            let (position, _) = polyline_at(&polyline, s).expect("point");
            close(s, number(&answer["sMm"]), tolerance);
            close_vector(&position, &answer["positionMm"], tolerance);
            close_vector(&section.position_mm, &answer["positionMm"], tolerance);
            close_vector(&section.moment_nmm, &answer["momentNmm"], tolerance);
            close_vector(&section.moment_nmm, &answer["prefixMomentNmm"], tolerance);
            // Independent opposite free-body cut, from remaining head loads.
            let suffix = heads
                .iter()
                .filter(|h| h.station_mm > s)
                .fold(Vector3::zeros(), |sum, h| {
                    sum + (h.position_mm - position).cross(&h.force_n) + h.couple_nmm
                });
            close_vector(&suffix, &answer["suffixMomentNmm"], tolerance);
            for i in 0..3 {
                close(section.moment_nmm[i] + suffix[i], 0.0, tolerance);
            }
        }
    }
}

#[test]
fn a_timoshenko_cantilever_has_the_closed_form_tip_and_stress() {
    let fixture = fixture(
        include_str!("../../../proving-ground/known-answers/load-bench/cantilever.json"),
        2,
    );
    for case in fixture["cases"].as_array().expect("cases") {
        let input = &case["input"];
        let expect = &case["expect"];
        let tolerance = &fixture["tolerance"];
        let length = number(&input["lengthMm"]);
        let diameter = number(&input["diameterMm"]);
        let properties = BeamProperties {
            young_modulus_mpa: number(&input["youngModulusNPerMm2"]),
            shear_modulus_mpa: number(&input["shearModulusNPerMm2"]),
            diameter_mm: diameter,
            shear_coefficient: number(&input["shearCoefficient"]),
        };
        let clamp = vector(&input["clampMm"]);
        let end = clamp + vector(&input["axisUnit"]) * length;
        let polyline = [clamp, end];
        let rod = CondensedRod::new(&polyline, &[0.0, length], properties).expect("beam");
        let tip = rod.stiffness.view((6, 6), (6, 6)).into_owned();
        let mut load = DVector::zeros(6);
        let force = vector(&input["endForceN"]);
        let moment = vector(&input["endMomentNmm"]);
        load.rows_mut(0, 3).copy_from(&force);
        load.rows_mut(3, 3).copy_from(&moment);
        let movement = tip.cholesky().expect("clamped tip stiffness").solve(&load);
        close(movement[1], number(&expect["tipDeflectionMm"]), tolerance);
        close(
            movement[5],
            number(&expect["tipSectionRotationRad"]),
            tolerance,
        );
        for axis in [0, 2, 3, 4] {
            close(movement[axis], 0.0, tolerance);
        }
        // Section properties and the split compliance check the fixture's other outputs.
        let area = properties.area_mm2();
        let second = properties.second_moment_mm4();
        close(area, number(&expect["areaMm2"]), tolerance);
        close(second, number(&expect["secondMomentMm4"]), tolerance);
        let shear =
            force.y * length / (properties.shear_coefficient * properties.shear_modulus_mpa * area);
        close(shear, number(&expect["shearTipDeflectionMm"]), tolerance);
        close(
            movement[1] - shear,
            number(&expect["bendingTipDeflectionMm"]),
            tolerance,
        );
        // Statics at the clamp from its reaction, opposite the imposed tip load.
        let reactions = [HeadLoad {
            station_mm: 0.0,
            position_mm: clamp,
            force_n: -force,
            couple_nmm: -(end - clamp).cross(&force) - moment,
        }];
        let clamp_cut = section_at(&polyline, &reactions, 0.0).expect("clamp section");
        let stress = surface_stress(&clamp_cut, diameter).expect("surface stress");
        close(
            stress.bending_mpa,
            number(&expect["maxBendingStressMpa"]),
            tolerance,
        );
    }
}

#[test]
fn two_series_springs_transmit_the_same_signed_force() {
    let fixture = fixture(
        include_str!("../../../proving-ground/known-answers/load-bench/two-springs-in-series.json"),
        4,
    );
    for case in fixture["cases"].as_array().expect("cases") {
        let rotational = case["kind"] == "rotational";
        let input = &case["input"];
        let expect = &case["expect"];
        let tolerance = &fixture["tolerance"];
        let (k1_name, k2_name, load_name, combined, stretch1, stretch2, total) = if rotational {
            (
                "stiffness1NmmPerRad",
                "stiffness2NmmPerRad",
                "momentNmm",
                "combinedStiffnessNmmPerRad",
                "rotation1Rad",
                "rotation2Rad",
                "totalRotationRad",
            )
        } else {
            (
                "stiffness1NPerMm",
                "stiffness2NPerMm",
                "forceN",
                "combinedStiffnessNPerMm",
                "stretch1Mm",
                "stretch2Mm",
                "totalStretchMm",
            )
        };
        let k1 = number(&input[k1_name]);
        let k2 = number(&input[k2_name]);
        let applied = number(&input[load_name]);
        let axis = vector(&input["axisUnit"]);
        let index = (0..3).find(|&i| axis[i] == 1.0).expect("fixture axis")
            + if rotational { 3 } else { 0 };
        let mut first = Matrix6::identity();
        let mut second = Matrix6::identity();
        let mut lower = Matrix6::zeros();
        first[(index, index)] = k1 + k2;
        second[(index, index)] = k2;
        lower[(index, index)] = -k2;
        let system = BorderedSystem {
            diagonal: vec![first, second],
            lower: vec![lower],
            coupling: DMatrix::zeros(12, 0),
            border: DMatrix::zeros(0, 0),
        };
        let mut load = DVector::zeros(12);
        load[6 + index] = applied;
        let displacement = system
            .factor()
            .expect("grounded springs")
            .solve(&load)
            .expect("spring movement");
        let first_change = displacement[index];
        let second_change = displacement[6 + index] - first_change;
        close(first_change, number(&expect[stretch1]), tolerance);
        close(second_change, number(&expect[stretch2]), tolerance);
        close(displacement[6 + index], number(&expect[total]), tolerance);
        close(
            applied / displacement[6 + index],
            number(&expect[combined]),
            tolerance,
        );
        close(k1 * first_change, applied, tolerance);
        close(k2 * second_change, applied, tolerance);
    }
}

#[test]
fn goodman_rejects_the_non_finite_domain() {
    let fixture = fixture(
        include_str!("../../../proving-ground/known-answers/load-bench/goodman.json"),
        7,
    );
    for case in fixture["cases"].as_array().expect("cases") {
        let input = &case["input"];
        let expect = &case["expect"];
        let tolerance = &fixture["tolerance"];
        let amplitude = number(&input["amplitudeMpa"]);
        let mean = number(&input["meanMpa"]);
        let ultimate = number(&input["ultimateMpa"]);
        close(
            (ultimate - mean) / ultimate,
            number(&expect["denominator"]),
            tolerance,
        );
        let result = goodman_equivalent(amplitude, mean, ultimate);
        match expect["status"].as_str().expect("status") {
            "finite" => close(
                result.expect("finite amplitude"),
                number(&expect["equivalentAmplitudeMpa"]),
                tolerance,
            ),
            "no-finite-answer" => {
                assert_eq!(result, None);
                assert!(expect["equivalentAmplitudeMpa"].is_null());
            }
            _ => panic!("unknown fixture status"),
        }
    }
}
