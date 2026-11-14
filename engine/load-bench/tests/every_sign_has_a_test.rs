//! Right-hand moments, rod/bone action-reaction and signed pull-out.

#[path = "a_second_solver_agrees/cases.rs"]
mod cases;
use forward_model::rotation::matrix_from_rotation_vector;
use load_bench::chain::{Chain, Ground, Pose};
use load_bench::evidence::Evidence;
use load_bench::joint_springs::Stiffness;
use load_bench::rigid_rod::{HeldOptions, RigidState, solve_held, solve_rigid};
use load_bench::rod_statics::pull_out_component;
use nalgebra::{Matrix3, Vector3};
use shapes::{levels::Level, made_up::VertebraPose};

#[test]
fn each_force_and_moment_resists_its_positive_deformation() {
    let reference = VertebraPose {
        level: Level::L5,
        centre: [0.0, 0.0, 10.0],
        rotation: [0.0; 3],
        height: 20.0,
        wedge_front: 0.0,
        wedge_side: 0.0,
        top_half_width: 15.0,
        top_half_depth: 10.0,
        bottom_half_width: 15.0,
        bottom_half_depth: 10.0,
    };
    let evidence = Evidence::read().unwrap();
    let k = Stiffness::from_evidence(&evidence, true).unwrap();
    let chain = Chain::new(
        &[reference],
        &[k],
        Ground {
            top_mm: Vector3::zeros(),
            rotation: Matrix3::identity(),
        },
    )
    .unwrap();
    for axis in 0..3 {
        let mut pose = chain.standing()[0].clone();
        pose.centre_mm[axis] += 0.5;
        let g = chain.evaluate(&[pose]).unwrap().gradient[0];
        assert!((g[axis] - k.0[axis] * 0.5).abs() < 1e-10);
        assert!(
            -g[axis] < 0.0,
            "joint force on bone resists positive translation"
        );
        let mut vector = Vector3::zeros();
        vector[axis] = 0.4;
        let q = matrix_from_rotation_vector(&vector);
        let pose = Pose {
            centre_mm: q * Vector3::from(reference.centre),
            rotation: q,
        };
        let g = chain.evaluate(&[pose]).unwrap().gradient[0];
        assert!((g[axis + 3] - k.0[axis + 3] * 0.4).abs() < 1e-8);
        assert!(
            -g[axis + 3] < 0.0,
            "joint moment resists the right-hand turn"
        );
    }
    let rx = matrix_from_rotation_vector(&(Vector3::x() * 0.4));
    let ry = matrix_from_rotation_vector(&(Vector3::y() * 0.4));
    let rz = matrix_from_rotation_vector(&(Vector3::z() * 0.4));
    assert!((rx * Vector3::z()).y < 0.0);
    assert!((ry * Vector3::z()).x > 0.0);
    assert!((rz * Vector3::x()).y > 0.0);
    assert_eq!(pull_out_component(-Vector3::x() * 7.0, Vector3::x()), 7.0);
    assert_eq!(pull_out_component(Vector3::x() * 7.0, Vector3::x()), -7.0);
}

#[test]
fn a_more_kyphotic_contour_pulls_its_apex_backward() {
    let case = cases::peaked(1.0, -2.0, 0.0);
    let h = solve_held(
        &case.chain,
        &case.construct,
        &HeldOptions::default(),
        Some(&case.guess),
    )
    .unwrap();
    assert!(h.settled, "{:?}", h.reason);
    let released = solve_rigid(&case.chain, &case.construct, &h, None).unwrap();
    assert!(released.settled, "{:?}", released.reason);
    let apex_on_bone = case
        .construct
        .anchors
        .iter()
        .enumerate()
        .filter(|(_, a)| a.body == 2)
        .map(|(i, _)| force_on_bone(&released, i))
        .sum::<Vector3<f64>>();
    assert!(apex_on_bone.x < 0.0, "posterior pull is -x: {apex_on_bone}");
}

#[test]
fn concave_apical_screws_see_pull_out_on_a_convex_right_curve() {
    let bent = cases::peaked(1.0, 0.0, -4.0);
    let reference: Vec<_> = bent
        .target
        .iter()
        .zip(&bent.chain.reference)
        .map(|(p, r)| p.to_vertebra(r))
        .collect();
    let target = bent.chain.standing();
    let case = cases::build(reference, target, bent.ground, 1.0, 0..=4);
    let h = solve_held(
        &case.chain,
        &case.construct,
        &HeldOptions::default(),
        Some(&case.guess),
    )
    .unwrap();
    assert!(h.settled, "{:?}", h.reason);
    let index = case
        .construct
        .anchors
        .iter()
        .position(|a| a.body == 2 && a.point_mm.y > 0.0)
        .unwrap();
    // Order 0 is R. Locked heads share force and moment by rod strain
    // energy; the swivel-force multipliers in H can contain a different
    // self-balanced allocation between the two rods.
    let released = solve_rigid(&case.chain, &case.construct, &h, None).unwrap();
    assert!(released.settled, "{:?}", released.reason);
    let on_bone = force_on_bone(&released, index);
    let inward = Vector3::new(1.0, -0.4, 0.0).normalize();
    assert!(
        on_bone.y > 0.0,
        "straightening a right apex pulls it patient-left"
    );
    assert!(
        pull_out_component(on_bone, inward) > 0.0,
        "concave apical pull-out: {on_bone}"
    );
}

fn force_on_bone(state: &RigidState, anchor: usize) -> Vector3<f64> {
    for (rod, order) in state.anchor_order.iter().enumerate() {
        if let Some(head) = order.iter().position(|&index| index == anchor) {
            return -state.head_wrenches_on_rod[rod][head]
                .fixed_rows::<3>(0)
                .into_owned();
        }
    }
    panic!("anchor has no released head load");
}

#[test]
fn mirroring_positions_mirrors_forces_and_reverses_axial_vector_moments() {
    let case = cases::holding_case(1.0);
    let mirror = cases::mirrored(&case);
    let h = solve_held(
        &case.chain,
        &case.construct,
        &HeldOptions::default(),
        Some(&case.guess),
    )
    .unwrap();
    let m = solve_held(
        &mirror.chain,
        &mirror.construct,
        &HeldOptions::default(),
        Some(&mirror.guess),
    )
    .unwrap();
    assert!(h.settled, "{:?}", h.reason);
    assert!(m.settled, "{:?}", m.reason);
    let s = Matrix3::from_diagonal(&Vector3::new(1.0, -1.0, 1.0));
    for (a, b) in h.poses.iter().zip(&m.poses) {
        assert!((s * a.centre_mm - b.centre_mm).norm() < 1e-9);
        assert!((s * a.rotation * s - b.rotation).norm() < 1e-9);
    }
    for (a, b) in h.head_forces_on_rod.iter().zip(&m.head_forces_on_rod) {
        assert!((s * a - b).norm() < 1e-8 * a.norm().max(1.0));
    }
    for (a, b) in h.holding_torques_nmm.iter().zip(&m.holding_torques_nmm) {
        assert!((-s * a - b).norm() < 1e-8 * a.norm().max(1.0));
    }
}
