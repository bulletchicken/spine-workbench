//! A loaded rigid block must not acquire a false buckling mode from its chart.
mod realistic_cases;

use load_bench::rigid_rod::{HeldOptions, solve_held, solve_rigid};
use load_bench::springback;
use shapes::made_up::RecipeName;

#[test]
#[ignore = "full sourced construct; small independent geometric-curvature checks run in CI"]
fn the_loaded_double_major_construct_has_positive_objective_stiffness() {
    let case = realistic_cases::build(RecipeName::DoubleMajor55To45, 2).unwrap();
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
    let state = springback::solve_balanced(
        &case.chain,
        &rigid.poses,
        &rigid.rods,
        &held.poses,
        &rigid.applied_loads,
    )
    .unwrap();
    assert!(state.settled && state.linear_relative_residual < 1e-9);
    // The joint-only target and supplied-frame holder remove this case's
    // former accidental negative bare-chart mode. The independent small
    // curvature tests retain that counterexample; this sourced construct
    // must still have a positive complete objective tangent.
    assert!(state.combined_tangent.cholesky().is_some());
}

#[test]
#[ignore = "six current default constructs; run in release beside the report"]
fn each_current_default_construct_settles_with_the_held_continuations() {
    for (recipe, seed) in realistic_cases::CASES {
        let case = realistic_cases::build(recipe, seed).unwrap();
        let held = solve_held(
            &case.chain,
            &case.construct,
            &HeldOptions::default(),
            Some(&case.guess),
        )
        .unwrap();
        assert!(
            held.settled,
            "{recipe:?}/{seed}: {:?}, gap {}, stationarity {}, ends {:?}",
            held.reason, held.constraint_max_mm, held.stationarity_max, held.contour_end_anchors
        );
        let rigid = solve_rigid(&case.chain, &case.construct, &held, None).unwrap();
        assert!(rigid.settled, "{recipe:?}/{seed}: {:?}", rigid.reason);
        let state = springback::solve_balanced(
            &case.chain,
            &rigid.poses,
            &rigid.rods,
            &held.poses,
            &rigid.applied_loads,
        )
        .unwrap();
        assert!(state.settled && state.linear_relative_residual < 1e-9);
    }
}
