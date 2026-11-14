//! Closed form for a screw-only block, independently constructed from H.

#[path = "a_second_solver_agrees/cases.rs"]
mod cases;
use load_bench::chain::Pose;
use load_bench::rigid_rod::{HeldOptions, solve_held, solve_rigid};

#[test]
fn the_released_block_keeps_its_inner_shape_and_all_outside_joints_rest() {
    let base = cases::holding_case(1.0);
    let case = cases::build(
        base.chain.reference.clone(),
        base.target.clone(),
        base.ground.clone(),
        1.0,
        1..=3,
    );
    let held = solve_held(
        &case.chain,
        &case.construct,
        &HeldOptions::default(),
        Some(&case.guess),
    )
    .expect("held solve");
    assert!(held.settled, "{:?}", held.reason);
    let rigid = solve_rigid(&case.chain, &case.construct, &held, None).expect("released solve");
    assert!(rigid.settled, "{:?}", rigid.reason);
    assert!(
        held.poses
            .iter()
            .zip(&rigid.poses)
            .any(
                |(a, b)| (a.centre_mm - b.centre_mm).norm() + (a.rotation - b.rotation).norm()
                    > 1e-6
            ),
        "removing nonzero holders must exercise a block motion"
    );
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
        assert!(
            state.deformation[i].norm() < 1e-9,
            "outside joint {i}: {}",
            state.deformation[i].norm()
        );
    }
    assert!(state.ground_reaction.norm() < 1e-9);
    assert!(rigid.ground_reaction.norm() < 1e-9);
}
