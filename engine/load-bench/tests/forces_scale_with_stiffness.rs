//! Uniform tissue scale leaves constrained geometry unchanged exactly.

#[path = "a_second_solver_agrees/cases.rs"]
mod cases;
use load_bench::rigid_rod::{HeldOptions, solve_held, solve_rigid};

#[test]
fn tissue_scale_changes_loads_and_holding_torques_in_the_same_ratio() {
    let base = cases::holding_case(1.0);
    let h0 = solve_held(
        &base.chain,
        &base.construct,
        &HeldOptions::default(),
        Some(&base.guess),
    )
    .unwrap();
    assert!(h0.settled, "{:?}", h0.reason);
    assert!(
        h0.holding_torques_nmm
            .iter()
            .any(|torque| torque.norm() > 1e-4),
        "this case must exercise the external holders"
    );
    let r0 = solve_rigid(&base.chain, &base.construct, &h0, None).unwrap();
    assert!(r0.settled, "{:?}", r0.reason);
    for scale in [0.25, 4.0] {
        let case = cases::holding_case(scale);
        let h = solve_held(
            &case.chain,
            &case.construct,
            &HeldOptions::default(),
            Some(&case.guess),
        )
        .unwrap();
        let r = solve_rigid(&case.chain, &case.construct, &h, None).unwrap();
        assert!(h.settled, "{:?}", h.reason);
        assert!(r.settled, "{:?}", r.reason);
        cases::assert_poses_close(&h.poses, &h0.poses, 1e-10);
        cases::assert_poses_close(&r.poses, &r0.poses, 1e-10);
        for (a, b) in h.head_forces_on_rod.iter().zip(&h0.head_forces_on_rod) {
            assert!((a - b * scale).norm() < 1e-9 * b.norm().max(1.0));
        }
        for (a, b) in h.holding_torques_nmm.iter().zip(&h0.holding_torques_nmm) {
            assert!((a - b * scale).norm() < 1e-9 * b.norm().max(1.0));
        }
        for (a, b) in r
            .head_wrenches_on_rod
            .iter()
            .flatten()
            .zip(r0.head_wrenches_on_rod.iter().flatten())
        {
            assert!((a - b * scale).norm() < 1e-9 * b.norm().max(1.0));
        }
    }
}

#[test]
fn elastic_shape_depends_only_on_the_tissue_to_rod_stiffness_ratio() {
    let run = |joint_scale: f64, rod_scale: f64| {
        let mut case = cases::small(joint_scale, -0.5, 0.0);
        for rod in &mut case.construct.rods {
            rod.properties.young_modulus_mpa *= rod_scale;
            rod.properties.shear_modulus_mpa *= rod_scale;
        }
        let h = solve_held(
            &case.chain,
            &case.construct,
            &HeldOptions::default(),
            Some(&case.guess),
        )
        .unwrap();
        assert!(h.settled, "{:?}", h.reason);
        let r = solve_rigid(&case.chain, &case.construct, &h, None).unwrap();
        assert!(r.settled, "{:?}", r.reason);
        load_bench::springback::solve(&case.chain, &r.poses, &r.rods, &h.poses).unwrap()
    };
    let base = run(1.0, 1.0);
    let together = run(4.0, 4.0);
    cases::assert_poses_close(&base.poses, &together.poses, 1e-10);
    for (a, b) in together
        .head_wrenches_on_rod
        .iter()
        .flatten()
        .zip(base.head_wrenches_on_rod.iter().flatten())
    {
        assert!((a - b * 4.0).norm() < 1e-9 * b.norm().max(1.0));
    }
    let first = run(4.0, 2.0);
    let ratio = run(2.0, 1.0);
    cases::assert_poses_close(&first.poses, &ratio.poses, 1e-10);
}
