//! Heads take the contour; joint energy chooses their remaining swivel.

#[path = "a_second_solver_agrees/cases.rs"]
mod cases;
#[path = "a_second_solver_agrees/independent.rs"]
mod independent;
use forward_model::rotation::rotation_vector_from_matrix;
use load_bench::rigid_rod::{HeldOptions, solve_held};
use nalgebra::{Matrix3, Vector3};
use shapes::made_up::RecipeName;

fn check_sliding_heads(recipe: RecipeName, seed: u32) {
    let case = cases::made_up(recipe, seed, 1.0);
    let sliding = solve_held(&case.chain, &case.construct, &HeldOptions::default(), None).unwrap();
    assert!(
        sliding.settled,
        "{recipe:?}/{seed}: {:?}, gap {}, stationarity {}",
        sliding.reason, sliding.constraint_max_mm, sliding.stationarity_max
    );
    let force_scale = sliding
        .head_forces_on_rod
        .iter()
        .map(|f| f.norm())
        .fold(0.0_f64, f64::max)
        .max(1e-12);
    for (i, anchor) in case.construct.anchors.iter().enumerate() {
        let head = sliding.poses[anchor.body].centre_mm
            + sliding.poses[anchor.body].rotation * anchor.point_mm;
        let (contour, tangent) = cases::curve_at(
            &case.construct.rods[anchor.rod].points_mm,
            sliding.stations_mm[i],
        );
        let rod = &sliding.rod_poses[anchor.rod];
        assert!(
            (head - rod.centre_mm - rod.rotation * contour).norm() < 1e-9,
            "head {i} gap"
        );
        // Perpendicularity applies to sliding heads. The lowest material
        // station is physically pinned and carries whatever tangential
        // reaction balance requires; its projection is reported separately.
        if !sliding.pinned_anchors.contains(&i) {
            let tangent = (rod.rotation * tangent).normalize();
            assert!(
                sliding.head_forces_on_rod[i].dot(&tangent).abs() < force_scale * 1e-9,
                "head {i} free slide force"
            );
        }
    }
    for (reference, seated) in case.construct.rods.iter().zip(&sliding.rod_poses) {
        let chord = (reference.points_mm.last().unwrap() - reference.points_mm[0]).normalize();
        // Independently use the principal rotation axis: zero swing–twist
        // makes it perpendicular to the original supplied chord. No inferred
        // contour plane or patient axis enters this condition.
        let turn = rotation_vector_from_matrix(&seated.rotation);
        assert!(turn.dot(&chord).abs() < 1e-10, "held chord twist");
    }
}

fn check_fixed_head_turns(recipe: RecipeName, seed: u32) {
    let case = cases::made_up(recipe, seed, 1.0);
    // This diagnostic explicitly fixes contour frames AND material
    // stations. It isolates the free turn about each *patient-frame*
    // target head line. The separate test below keeps rod poses free while
    // fixing stations. The sliding-H checks use neither diagnostic fixture.
    let fixed = solve_held(
        &case.chain,
        &case.construct,
        &HeldOptions {
            fixed_stations: true,
            fixed_rods: true,
            ..HeldOptions::default()
        },
        Some(&case.guess),
    )
    .unwrap();
    assert!(
        fixed.settled,
        "fixed {recipe:?}/{seed}: {:?}, gap {}, stationarity {}",
        fixed.reason, fixed.constraint_max_mm, fixed.stationarity_max
    );
    let target_energy = case.chain.evaluate(&case.target).unwrap().energy_nmm;
    assert!(
        fixed.energy_nmm < target_energy - 1e-6,
        "the target itself is not the constrained minimum"
    );
    let joint = case.chain.evaluate(&fixed.poses).unwrap();
    let local_midpoint = Vector3::new(cases::HEAD_BACK_MM, 0.0, 0.0);
    for i in 0..fixed.poses.len() {
        let a = &fixed.poses[i];
        let target = &case.target[i];
        let midpoint = target.centre_mm + target.rotation * local_midpoint;
        let axis = target.rotation * Vector3::y();
        assert!(
            ((a.rotation * target.rotation.transpose()) * axis - axis).norm() < 1e-9,
            "head-line axis {i}"
        );
        for sign in [-1.0, 1.0] {
            let local = local_midpoint + Vector3::y() * (sign * cases::HEAD_HALF_WIDTH_MM);
            assert!(
                (a.centre_mm + a.rotation * local - target.centre_mm - target.rotation * local)
                    .norm()
                    < 1e-9,
                "fixed head {i}"
            );
        }
        let velocity = axis.cross(&(a.centre_mm - midpoint));
        let derivative = joint.gradient[i].fixed_rows::<3>(0).dot(&velocity)
            + joint.gradient[i].fixed_rows::<3>(3).dot(&axis);
        assert!(
            derivative.abs() < 1e-8 * joint.gradient[i].norm().max(1.0),
            "body {i} head-line moment {derivative}"
        );
    }
    let reference: Vec<_> = case
        .chain
        .standing()
        .iter()
        .map(|p| independent::Body {
            centre: p.centre_mm,
            rotation: p.rotation,
        })
        .collect();
    let targets: Vec<_> = case
        .target
        .iter()
        .map(|p| independent::Body {
            centre: p.centre_mm,
            rotation: p.rotation,
        })
        .collect();
    let heights: Vec<_> = case.chain.reference.iter().map(|p| p.height).collect();
    let separate = independent::IndependentChain::new(
        &reference,
        &heights,
        &case.diagonals,
        independent::Body {
            centre: case.ground.top_mm,
            rotation: case.ground.rotation,
        },
    );
    let mut largest_target_derivative = 0.0_f64;
    for i in 0..targets.len() {
        let midpoint = targets[i].centre + targets[i].rotation * local_midpoint;
        let axis = targets[i].rotation * Vector3::y();
        let mut plus = targets.clone();
        let mut minus = targets.clone();
        plus[i] = independent::about_line(&targets[i], midpoint, axis, 1e-5);
        minus[i] = independent::about_line(&targets[i], midpoint, axis, -1e-5);
        let derivative =
            (separate.adjacent_energy(&plus, i) - separate.adjacent_energy(&minus, i)) / 2e-5;
        largest_target_derivative = largest_target_derivative.max(derivative.abs());
    }
    // Own regression threshold: every one of these six geometries has a
    // clearly nonstationary target, rather than a round-off-sized swivel.
    assert!(largest_target_derivative > 1.0);
    let (minimum, angles) =
        independent::settle_head_line_angles(&separate, &targets, local_midpoint);
    assert!(
        angles.iter().any(|angle| angle.abs() > 1e-5),
        "bent targets themselves are not the minimum"
    );
    for (i, (a, b)) in fixed.poses.iter().zip(&minimum).enumerate() {
        assert!(
            (a.centre_mm - b.centre).norm() < 3e-6,
            "independent centre {i}: {}",
            (a.centre_mm - b.centre).norm()
        );
        assert!(
            (a.rotation - b.rotation).norm() < 2e-7,
            "independent turn {i}: {}",
            (a.rotation - b.rotation).norm()
        );
    }
}

macro_rules! recipe_checks {
    ($sliding:ident, $fixed:ident, $recipe:expr, $seed:expr) => {
        #[test]
        fn $sliding() {
            check_sliding_heads($recipe, $seed);
        }
        #[test]
        fn $fixed() {
            check_fixed_head_turns($recipe, $seed);
        }
    };
}
recipe_checks!(
    right_thoracic_seed_one_heads_slide_on_the_rods,
    right_thoracic_seed_one_turns_minimise_joint_energy,
    RecipeName::RightThoracic40,
    1
);
recipe_checks!(
    right_thoracic_seed_two_heads_slide_on_the_rods,
    right_thoracic_seed_two_turns_minimise_joint_energy,
    RecipeName::RightThoracic40,
    2
);
recipe_checks!(
    double_major_seed_one_heads_slide_on_the_rods,
    double_major_seed_one_turns_minimise_joint_energy,
    RecipeName::DoubleMajor55To45,
    1
);
recipe_checks!(
    double_major_seed_two_heads_slide_on_the_rods,
    double_major_seed_two_turns_minimise_joint_energy,
    RecipeName::DoubleMajor55To45,
    2
);
recipe_checks!(
    lumbar_left_seed_one_heads_slide_on_the_rods,
    lumbar_left_seed_one_turns_minimise_joint_energy,
    RecipeName::LumbarLeft30,
    1
);
recipe_checks!(
    lumbar_left_seed_two_heads_slide_on_the_rods,
    lumbar_left_seed_two_turns_minimise_joint_energy,
    RecipeName::LumbarLeft30,
    2
);

#[test]
fn fixed_material_stations_leave_the_rod_pose_free() {
    let base = cases::small(1.0, -1.0, 0.0);
    let reference = base
        .target
        .iter()
        .zip(&base.chain.reference)
        .map(|(pose, reference)| pose.to_vertebra(reference))
        .collect();
    let mut case = cases::build(reference, base.target.clone(), base.ground, 1.0, 0..=4);
    // The standing curved spine is an exact zero-energy feasible state.
    // Translate and turn the initial guesses so a fixed-contour implementation
    // cannot return that answer without actually moving its rod variables.
    for rod in &mut case.guess.rod_poses {
        rod.centre_mm.x += 1.0;
        rod.rotation = forward_model::rotation::rot_y(0.02);
    }
    let fixed_stations = solve_held(
        &case.chain,
        &case.construct,
        &HeldOptions {
            fixed_stations: true,
            fixed_rods: false,
            ..HeldOptions::default()
        },
        Some(&case.guess),
    )
    .unwrap();
    assert!(fixed_stations.settled, "{:?}", fixed_stations.reason);
    assert_eq!(fixed_stations.stations_mm, case.guess.stations_mm);
    cases::assert_poses_close(&fixed_stations.poses, &case.chain.standing(), 1e-9);
    for rod in &fixed_stations.rod_poses {
        assert!(rod.centre_mm.norm() < 1e-9);
        assert!((rod.rotation - Matrix3::identity()).norm() < 1e-9);
    }
}

#[test]
#[ignore = "longer made-up geometry sweep"]
fn more_seeds_cover_the_same_head_and_swivel_properties() {
    for recipe in [
        RecipeName::RightThoracic40,
        RecipeName::DoubleMajor55To45,
        RecipeName::LumbarLeft30,
        RecipeName::Severe90,
    ] {
        for seed in 3..=12 {
            check_sliding_heads(recipe, seed);
            check_fixed_head_turns(recipe, seed);
        }
    }
}
