//! Closed form without horizontal lean, and the constrained released block.

#[path = "a_second_solver_agrees/cases.rs"]
mod cases;
use forward_model::rotation::matrix_from_rotation_vector;
use load_bench::chain::Pose;
use load_bench::rigid_rod::{HeldOptions, solve_held, solve_rigid};
use nalgebra::Vector3;
use shapes::levels::Level;

#[test]
fn without_horizontal_lean_the_old_released_closed_form_has_zero_condition_force() {
    let base = cases::small(1.0, 0.0, 0.0);
    let mut target = base.chain.standing();
    // A nonzero axial correction with equal endpoint shifts leaves the
    // block's length unchanged. Releasing it translates the block down by
    // 3 mm; the middle body retains its extra 2 mm internal correction.
    for (body, shift) in [(1, 3.0), (2, 5.0), (3, 3.0)] {
        target[body].centre_mm.z += shift;
    }
    let mut case = cases::build(base.chain.reference, target, base.ground, 1.0, 1..=3);
    // Centre-line bilateral heads keep the axial example free of a
    // posterior arm's coupling between compression and forward lean.
    for rod in &mut case.construct.rods {
        for point in &mut rod.points_mm {
            point.x -= cases::HEAD_BACK_MM;
        }
    }
    for anchor in &mut case.construct.anchors {
        anchor.point_mm.x = 0.0;
    }
    let held = solve_held(
        &case.chain,
        &case.construct,
        &HeldOptions {
            fixed_rods: true,
            fixed_stations: true,
            ..HeldOptions::default()
        },
        Some(&case.guess),
    )
    .expect("held solve");
    assert!(held.settled, "{:?}", held.reason);
    let rigid = solve_rigid(&case.chain, &case.construct, &held, None).expect("released solve");
    assert!(rigid.settled, "{:?}", rigid.reason);
    assert!((held.poses[3].centre_mm.z - rigid.poses[3].centre_mm.z - 3.0).abs() < 1e-9);
    let standing = case.chain.standing();
    let q = standing[3].rotation * held.poses[3].rotation.transpose();
    let t = standing[3].centre_mm - q * held.poses[3].centre_mm;
    let mut expected = standing.clone();
    for (expected, held) in expected[1..=3].iter_mut().zip(&held.poses[1..=3]) {
        *expected = Pose {
            centre_mm: q * held.centre_mm + t,
            rotation: q * held.rotation,
        };
    }
    let upper_q = expected[1].rotation * standing[1].rotation.transpose();
    let upper_t = expected[1].centre_mm - upper_q * standing[1].centre_mm;
    expected[0] = Pose {
        centre_mm: upper_q * standing[0].centre_mm + upper_t,
        rotation: upper_q * standing[0].rotation,
    };
    cases::assert_poses_close(&rigid.poses, &expected, 1e-9);
    let state = case.chain.evaluate(&rigid.poses).expect("joint state");
    for i in [0, 3, 4] {
        assert!(state.deformation[i].norm() < 1e-9);
    }
    assert!(state.ground_reaction.norm() < 1e-9);
    assert!(rigid.applied_loads.norm() < 1e-9);
}

fn leaning_case(bodies: std::ops::RangeInclusive<usize>) -> cases::Case {
    let base = cases::small(1.0, 0.0, 0.0);
    let mut reference = base.chain.reference;
    for (pose, level) in
        reference
            .iter_mut()
            .zip([Level::T1, Level::T2, Level::T3, Level::L4, Level::L5])
    {
        pose.level = level;
    }
    // Chosen synthetic frontal curve and a correction resting on a tilted
    // lower endpoint. Both horizontal coordinates of the top centre differ
    // after the unconstrained closed-form block release.
    for (pose, angle) in reference.iter_mut().zip([0.0, -0.1, 0.1, 0.2, 0.0]) {
        pose.rotation = [angle, 0.0, 0.0];
    }
    let mut target: Vec<_> = reference.iter().map(Pose::from_vertebra).collect();
    let turn = matrix_from_rotation_vector(&Vector3::new(0.2, 0.02, 0.0));
    for i in 1..=3 {
        target[i] = Pose {
            centre_mm: target[3].centre_mm + turn * Vector3::new(0.0, 0.0, (3 - i) as f64 * 25.0),
            rotation: turn,
        };
    }
    cases::build(reference, target, base.ground, 1.0, bodies)
}

fn held_at_target(case: &cases::Case) -> load_bench::rigid_rod::HeldState {
    let held = solve_held(
        &case.chain,
        &case.construct,
        &HeldOptions {
            fixed_rods: true,
            fixed_stations: true,
            ..HeldOptions::default()
        },
        Some(&case.guess),
    )
    .unwrap();
    assert!(held.settled, "{:?}", held.reason);
    held
}

#[test]
fn a_leaning_correction_keeps_t1_at_its_standing_horizontal_position() {
    let case = leaning_case(1..=3);
    let held = held_at_target(&case);
    let rigid = solve_rigid(&case.chain, &case.construct, &held, None).unwrap();
    assert!(rigid.settled, "{:?}", rigid.reason);
    let standing = case.chain.standing();
    assert!((rigid.poses[0].centre_mm.x - standing[0].centre_mm.x).abs() < 1e-9);
    assert!((rigid.poses[0].centre_mm.y - standing[0].centre_mm.y).abs() < 1e-9);
    assert!(rigid.applied_loads.rows(0, 2).norm() > 1e-3);
    assert_eq!(
        rigid
            .applied_loads
            .rows(2, rigid.applied_loads.len() - 2)
            .norm(),
        0.0
    );
    let tilt = |pose: &Pose| libm::atan2(-pose.rotation[(1, 2)], pose.rotation[(2, 2)]);
    assert!(tilt(&rigid.poses[3]).abs() < tilt(&held.poses[3]).abs());
    // Locking preserves every relative pose inside the screw-connected block.
    let q = rigid.poses[3].rotation * held.poses[3].rotation.transpose();
    let t = rigid.poses[3].centre_mm - q * held.poses[3].centre_mm;
    for i in 1..=3 {
        assert!((rigid.poses[i].centre_mm - q * held.poses[i].centre_mm - t).norm() < 1e-10);
        assert!((rigid.poses[i].rotation - q * held.poses[i].rotation).norm() < 1e-10);
    }
    let state = case.chain.evaluate(&rigid.poses).unwrap();
    assert!(state.deformation[4].fixed_rows::<3>(3).norm() > 1e-3);
    let elastic = load_bench::springback::solve_balanced(
        &case.chain,
        &rigid.poses,
        &rigid.rods,
        &held.poses,
        &rigid.applied_loads,
    )
    .unwrap();
    assert!(elastic.settled);
    assert!((elastic.poses[0].centre_mm.x - standing[0].centre_mm.x).abs() < 1e-9);
    assert!((elastic.poses[0].centre_mm.y - standing[0].centre_mm.y).abs() < 1e-9);
}

#[test]
fn constructs_including_t1_or_reaching_l5_settle_with_horizontal_balance() {
    for span in [0..=3, 1..=4] {
        let case = leaning_case(span.clone());
        let held = held_at_target(&case);
        let rigid = solve_rigid(&case.chain, &case.construct, &held, None).unwrap();
        assert!(rigid.settled, "{span:?}: {:?}", rigid.reason);
        let standing = case.chain.standing();
        assert!((rigid.poses[0].centre_mm.x - standing[0].centre_mm.x).abs() < 1e-9);
        assert!((rigid.poses[0].centre_mm.y - standing[0].centre_mm.y).abs() < 1e-9);
        let elastic = load_bench::springback::solve_balanced(
            &case.chain,
            &rigid.poses,
            &rigid.rods,
            &held.poses,
            &rigid.applied_loads,
        )
        .unwrap();
        assert!(elastic.settled);
        assert!(
            (elastic.poses[0].centre_mm - standing[0].centre_mm)
                .fixed_rows::<2>(0)
                .norm()
                < 1e-9
        );
    }
}

#[test]
fn an_already_balanced_spine_needs_no_condition_force_or_shape_change() {
    let case = cases::small(1.0, 0.0, 0.0);
    let held = solve_held(&case.chain, &case.construct, &HeldOptions::default(), None).unwrap();
    assert!(held.settled);
    let rigid = solve_rigid(&case.chain, &case.construct, &held, None).unwrap();
    assert!(rigid.settled);
    assert!(rigid.applied_loads.norm() < 1e-10);
    cases::assert_poses_close(&rigid.poses, &case.chain.standing(), 1e-10);
    assert!(
        rigid
            .head_wrenches_on_rod
            .iter()
            .flatten()
            .all(|w| w.norm() < 1e-9)
    );
}
