//! Free elastic rods and the least-strain-energy rigid force limit.

use load_bench::chain::{Pose, Wrench};
use load_bench::rod_beams::{BeamProperties, CondensedRod, polyline_stations};
use load_bench::springback::{HeadTie, LockedRod, least_energy_loads, prepare_rods, solve_linear};
use nalgebra::{DMatrix, DVector, Matrix3, Vector3};

fn rods(scale: f64) -> (Vec<Pose>, Vec<LockedRod>) {
    let poses: Vec<_> = (0..3)
        .map(|i| Pose {
            centre_mm: Vector3::new(0.0, 0.0, 100.0 - 50.0 * i as f64),
            rotation: Matrix3::identity(),
        })
        .collect();
    let points: Vec<_> = poses
        .iter()
        .map(|p| p.centre_mm + Vector3::new(-12.0, 0.0, 0.0))
        .collect();
    let stations = polyline_stations(&points).unwrap();
    // Synthetic section, intentionally without a material name or table value.
    let properties = BeamProperties {
        young_modulus_mpa: 80_000.0 * scale,
        shear_modulus_mpa: 30_000.0 * scale,
        diameter_mm: 4.0,
        shear_coefficient: 0.8,
    };
    let beam = CondensedRod::new(&points, &stations, properties).unwrap();
    let rod = LockedRod {
        ties: points
            .iter()
            .enumerate()
            .map(|(body, &point_mm)| HeadTie {
                body,
                point_mm,
                locks_rotation: true,
            })
            .collect(),
        stiffness: beam.stiffness,
    };
    (poses, vec![rod.clone(), rod])
}

fn gradient() -> (DVector<f64>, Vec<Wrench>) {
    // Explicit balanced loads: transverse 1,-2,1; axial -1,0,1;
    // couples 3,-6,3. Both force and moment sums are zero. The two
    // coincident identical rods must share each component equally.
    let forces = [
        Vector3::new(4.0, 0.0, -2.0),
        Vector3::new(-8.0, 0.0, 0.0),
        Vector3::new(4.0, 0.0, 2.0),
    ];
    let couples = [
        Vector3::new(3.0, 0.0, 0.0),
        Vector3::new(-6.0, 0.0, 0.0),
        Vector3::new(3.0, 0.0, 0.0),
    ];
    let arm = Vector3::new(-12.0, 0.0, 0.0);
    let mut g = DVector::zeros(18);
    let mut loads = Vec::new();
    for (i, (force, couple)) in forces.iter().zip(&couples).enumerate() {
        let moment = arm.cross(force) + couple;
        g.rows_mut(6 * i, 3).copy_from(&(-force));
        g.rows_mut(6 * i + 3, 3).copy_from(&(-moment));
        let mut load = Wrench::zeros();
        load.fixed_rows_mut::<3>(0).copy_from(&(force * 0.5));
        load.fixed_rows_mut::<3>(3).copy_from(&(couple * 0.5));
        loads.push(load);
    }
    (g, loads)
}

#[test]
fn identical_free_rods_share_the_least_energy_loads_equally() {
    let (poses, input) = rods(1.0);
    let prepared = prepare_rods(&poses, &input).unwrap();
    let (g, expected) = gradient();
    let loads = least_energy_loads(&g, &prepared).unwrap();
    for rod in &loads {
        for (actual, expected) in rod.iter().zip(&expected) {
            assert!((actual - expected).norm() < 1e-8, "{actual} {expected}");
        }
    }
    // The selector has no iterate or starting-point argument: its solution
    // is fixed by these equations and the rod strain metric alone.
    let repeated = least_energy_loads(&g, &prepared).unwrap();
    assert_eq!(loads, repeated);
}

#[test]
fn two_elastic_stiffnesses_extrapolate_to_the_rigid_limit() {
    let (g, _) = gradient();
    let (poses, input) = rods(1.0);
    let prepared = prepare_rods(&poses, &input).unwrap();
    let limit = least_energy_loads(&g, &prepared).unwrap();
    let tangent = DMatrix::from_diagonal(&DVector::from_fn(
        18,
        |i, _| if i % 6 < 3 { 2.0 } else { 40.0 },
    ));
    let scales = [10_000.0, 20_000.0];
    let elastic: Vec<_> = scales
        .iter()
        .map(|&scale| {
            let (poses, input) = rods(scale);
            let prepared = prepare_rods(&poses, &input).unwrap();
            solve_linear(&tangent, &g, &prepared)
                .unwrap()
                .head_wrenches_on_rod
        })
        .collect();
    for (rod, rod_limit) in limit.iter().enumerate() {
        for (head, head_limit) in rod_limit.iter().enumerate() {
            let extrapolated = (elastic[1][rod][head] * scales[1]
                - elastic[0][rod][head] * scales[0])
                / (scales[1] - scales[0]);
            assert!(
                (extrapolated - head_limit).norm() < 2e-7,
                "{extrapolated} versus {}",
                head_limit
            );
        }
    }
}

#[test]
fn a_load_on_a_free_rigid_motion_has_no_rigid_force_solution() {
    let (poses, input) = rods(1.0);
    let prepared = prepare_rods(&poses, &input).unwrap();
    let mut g = DVector::zeros(18);
    g[0] = 1.0;
    assert!(least_energy_loads(&g, &prepared).is_err());
}

#[test]
fn coordinate_conditioning_does_not_amplify_resting_body_round_off() {
    let (mut poses, input) = rods(1e12);
    // A fourth, uninstrumented body carries only floating-point rest noise.
    // Its zero material row retains scale one, while rod rows are strongly
    // equilibrated. Expected physical loads remain the hand-balanced split.
    poses.push(Pose {
        centre_mm: Vector3::new(0.0, 0.0, -50.0),
        rotation: Matrix3::identity(),
    });
    let prepared = prepare_rods(&poses, &input).unwrap();
    let (balanced, expected) = gradient();
    let mut g = DVector::zeros(24);
    g.rows_mut(0, 18).copy_from(&balanced);
    g[18] = 32.0 * f64::EPSILON * balanced.norm();
    let loads = least_energy_loads(&g, &prepared).unwrap();
    for rod in &loads {
        for (actual, answer) in rod.iter().zip(&expected) {
            assert!((actual - answer).norm() < 1e-8);
        }
    }
    // A real unsupported force is not a round-off residual and must fail.
    g[18] = 1.0;
    assert!(least_energy_loads(&g, &prepared).is_err());
}

#[test]
fn unequal_rods_share_load_in_proportion_to_their_stiffnesses() {
    let (poses, mut input) = rods(1.0);
    input[1].stiffness *= 3.0;
    let prepared = prepare_rods(&poses, &input).unwrap();
    let (g, half) = gradient();
    let loads = least_energy_loads(&g, &prepared).unwrap();
    for (head, half_load) in half.iter().enumerate() {
        assert!((loads[0][head] - half_load * 0.5).norm() < 1e-8);
        assert!((loads[1][head] - half_load * 1.5).norm() < 1e-8);
    }
}

#[path = "a_second_solver_agrees/cases.rs"]
mod cases;

#[test]
fn a_released_chain_has_the_same_force_limit_from_another_start() {
    use forward_model::rotation::rot_y;
    use load_bench::rigid_rod::{HeldOptions, solve_held, solve_rigid};
    let case = cases::small(1.0, -1.0, 0.0);
    let held = solve_held(
        &case.chain,
        &case.construct,
        &HeldOptions::default(),
        Some(&case.guess),
    )
    .unwrap();
    assert!(held.settled, "{:?}", held.reason);
    let rigid = solve_rigid(&case.chain, &case.construct, &held, None).unwrap();
    assert!(rigid.settled, "{:?}", rigid.reason);
    let turn = rot_y(0.08);
    let shift = Vector3::new(1.0, -2.0, 0.5);
    let other: Vec<_> = rigid
        .poses
        .iter()
        .map(|p| Pose {
            centre_mm: turn * p.centre_mm + shift,
            rotation: turn * p.rotation,
        })
        .collect();
    let repeated = solve_rigid(&case.chain, &case.construct, &held, Some(&other)).unwrap();
    assert!(repeated.settled, "{:?}", repeated.reason);
    cases::assert_poses_close(&rigid.poses, &repeated.poses, 1e-9);
    for (a, b) in rigid
        .head_wrenches_on_rod
        .iter()
        .flatten()
        .zip(repeated.head_wrenches_on_rod.iter().flatten())
    {
        assert!((a - b).norm() < 1e-8 * a.norm().max(1.0));
    }
    let scales = [5000.0, 10000.0];
    let elastic: Vec<_> = scales
        .iter()
        .map(|scale| {
            let mut rods = rigid.rods.clone();
            for rod in &mut rods {
                rod.stiffness *= *scale;
            }
            load_bench::springback::solve_balanced(
                &case.chain,
                &rigid.poses,
                &rods,
                &held.poses,
                &rigid.applied_loads,
            )
            .unwrap()
            .head_wrenches_on_rod
        })
        .collect();
    for (rod, loads) in rigid.head_wrenches_on_rod.iter().enumerate() {
        for (head, limit) in loads.iter().enumerate() {
            let extrapolated = (elastic[1][rod][head] * scales[1]
                - elastic[0][rod][head] * scales[0])
                / (scales[1] - scales[0]);
            assert!(
                (extrapolated - limit).norm() < 2e-6 * limit.norm().max(1.0),
                "{}",
                (extrapolated - limit).norm()
            );
        }
    }
}
