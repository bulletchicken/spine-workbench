//! Published-direction plausibility probes, not a comparison with patients.
//!
//! Abe et al. (2015), PMID 25810754, reports greater concave apical
//! pull-out and a rise with lower density during simultaneous rod rotation.
//! These static prescribed-shape probes record agreement or disagreement
//! with those directions, and check equilibrium independently. Their
//! boundary does not reproduce the surgical manoeuvre or its force values.

#[path = "a_second_solver_agrees/cases.rs"]
mod cases;

use load_bench::evidence::{Evidence, TableKind};
use load_bench::rigid_rod::{HeldOptions, solve_held};
use load_bench::rod_beams::BeamProperties;
use load_bench::rod_statics::pull_out_component;
use nalgebra::Vector3;

fn correction() -> cases::Case {
    let standing = cases::peaked(1.0, 0.0, -4.0);
    let reference = standing
        .target
        .iter()
        .zip(&standing.chain.reference)
        .map(|(pose, reference)| pose.to_vertebra(reference))
        .collect();
    cases::build(
        reference,
        standing.chain.standing(),
        standing.ground,
        1.0,
        0..=4,
    )
}

/// Independently sum each body's forces and moments, and each rod's
/// resultant plus its external holding torque. No published force enters
/// these equilibrium answers: every residual must be zero.
fn seated(case: &cases::Case) -> load_bench::rigid_rod::HeldState {
    let held = solve_held(
        &case.chain,
        &case.construct,
        &HeldOptions::default(),
        Some(&case.guess),
    )
    .expect("seated correction");
    assert!(held.settled, "{:?}", held.reason);
    assert!(held.energy_nmm.is_finite() && held.energy_nmm >= 0.0);
    assert_eq!(held.head_forces_on_rod.len(), case.construct.anchors.len());
    let evaluation = case.chain.evaluate(&held.poses).expect("joint reactions");
    let mut bodies = evaluation.gradient;
    let mut rods = vec![load_bench::chain::Wrench::zeros(); case.construct.rods.len()];
    let force_scale = held
        .head_forces_on_rod
        .iter()
        .map(Vector3::norm)
        .fold(1.0_f64, f64::max);
    let length_scale = held
        .poses
        .iter()
        .map(|p| p.centre_mm.norm())
        .fold(1.0_f64, f64::max);
    for (anchor, force) in case.construct.anchors.iter().zip(&held.head_forces_on_rod) {
        assert!(force.iter().all(|v| v.is_finite()));
        let pose = &held.poses[anchor.body];
        assert!(
            pose.centre_mm
                .iter()
                .chain(pose.rotation.iter())
                .all(|v| v.is_finite())
        );
        assert!(
            (pose.rotation.transpose() * pose.rotation - nalgebra::Matrix3::identity()).norm()
                < 1e-9
        );
        let arm = pose.rotation * anchor.point_mm;
        let point = pose.centre_mm + arm;
        let add = |wrench: &mut load_bench::chain::Wrench, at: Vector3<f64>| {
            for axis in 0..3 {
                wrench[axis] += force[axis];
                wrench[axis + 3] += at.cross(force)[axis];
            }
        };
        add(&mut bodies[anchor.body], arm);
        add(&mut rods[anchor.rod], point);
    }
    for (rod, torque) in rods.iter_mut().zip(&held.holding_torques_nmm) {
        assert!(torque.iter().all(|v| v.is_finite()));
        for axis in 0..3 {
            rod[axis + 3] += torque[axis];
        }
    }
    let mut ground = held.ground_reaction;
    for torque in &held.holding_torques_nmm {
        for axis in 0..3 {
            ground[axis + 3] += torque[axis];
        }
    }
    for wrench in bodies.iter().chain(&rods).chain([&ground]) {
        // Own numerical tolerance: the same 1e-9 relative physical
        // equilibrium tolerance used by the independent free-body checks.
        assert!(
            wrench.fixed_rows::<3>(0).norm() <= force_scale * 1e-9,
            "force residual {wrench:?}"
        );
        assert!(
            wrench.fixed_rows::<3>(3).norm() <= force_scale * length_scale * 1e-9,
            "moment residual {wrench:?}"
        );
    }
    held
}

fn apical_pull_out(
    case: &cases::Case,
    held: &load_bench::rigid_rod::HeldState,
    side: usize,
) -> f64 {
    let index = case
        .construct
        .anchors
        .iter()
        .position(|a| a.body == 2 && a.rod == side)
        .expect("apical screw");
    // The screw's inward trajectory is local to its vertebra and must
    // turn with that vertebra before projecting a patient-frame force.
    let local = Vector3::new(1.0, if side == 0 { -0.4 } else { 0.4 }, 0.0).normalize();
    let inward = held.poses[case.construct.anchors[index].body].rotation * local;
    assert!((inward.norm() - 1.0).abs() < 1e-12);
    let pull_out = pull_out_component(-held.head_forces_on_rod[index], inward);
    assert!(pull_out.is_finite());
    assert!(pull_out.abs() <= held.head_forces_on_rod[index].norm() * (1.0 + 1e-12));
    pull_out
}

#[test]
fn records_concave_and_alternate_level_published_direction_plausibility() {
    let dense = correction();
    let dense_held = seated(&dense);
    let concave = apical_pull_out(&dense, &dense_held, 0);
    let convex = apical_pull_out(&dense, &dense_held, 1);
    let mut alternate = correction();
    // Bilateral alternate levels 0,2,4. Retaining both ends makes this
    // six of ten heads; it is an alternate-level pattern, not exactly
    // half the screw count and not unilateral removal of the convex apex.
    let keep: Vec<_> = alternate
        .construct
        .anchors
        .iter()
        .enumerate()
        .filter_map(|(i, a)| a.body.is_multiple_of(2).then_some(i))
        .collect();
    alternate.construct.anchors = keep
        .iter()
        .map(|&i| alternate.construct.anchors[i].clone())
        .collect();
    alternate.guess.stations_mm = keep
        .iter()
        .map(|&i| alternate.guess.stations_mm[i])
        .collect();
    assert_eq!(alternate.construct.anchors.len(), 6);
    let alternate_held = seated(&alternate);
    let alternate_concave = apical_pull_out(&alternate, &alternate_held, 0);
    let concave_direction_agrees = concave > convex;
    let density_direction_agrees = alternate_concave > concave;
    eprintln!(
        "plausibility, static H signed pull-out N: dense concave {concave:.6}, convex {convex:.6}; alternate-level concave {alternate_concave:.6}; concave direction agrees={concave_direction_agrees}, density direction agrees={density_direction_agrees}"
    );
    // Abe's simultaneous rotation is not this static straight-rod toy.
    // Rotation of a straight rod about itself is an invisible freedom.
    // Record agreement/disagreement; neither published patient comparison
    // establishes a universal density inequality for this boundary value.
}

#[test]
fn records_unequal_contour_published_direction_plausibility() {
    let force = |bow: f64| {
        let mut case = correction();
        // Own synthetic contour difference, posterior at the concave apex.
        // Both comparisons use the same target-pose initialisation policy.
        for (i, point) in case.construct.rods[0].points_mm.iter_mut().enumerate() {
            point.x -= bow * [0.0, 0.0, 0.2, 1.0, 0.2, 0.0, 0.0][i];
        }
        let held = seated(&case);
        held.head_forces_on_rod
            .iter()
            .map(Vector3::norm)
            .fold(0.0_f64, f64::max)
    };
    let (small, large) = (force(0.5), force(1.0));
    let contour_direction_agrees = large > small;
    eprintln!(
        "plausibility, static H contour bow 0.5/1.0 mm: largest force {small:.6}/{large:.6} N; contour direction agrees={contour_direction_agrees}"
    );
    // A source-specific trend is checked and reported as plausibility.
    // Force redistribution and changes of nonlinear solution branch do
    // not supply a universal inequality for arbitrary contour pairs.
}

#[test]
fn a_larger_diameter_raises_bending_stiffness_by_its_fourth_power() {
    let evidence = Evidence::read().unwrap();
    let e = evidence
        .value(TableKind::RodMaterials, "cobalt-chrome", "young-modulus")
        .unwrap()
        .value
        * 1000.0;
    let ei = |diameter| {
        let beam = BeamProperties::from_young_modulus(e, diameter);
        beam.young_modulus_mpa * beam.second_moment_mm4()
    };
    let ratio = ei(6.0) / ei(5.5);
    // Independent circular-section law EI ∝ d⁴, not a finite-element fit.
    let expected = (12.0_f64 / 11.0) * (12.0 / 11.0) * (12.0 / 11.0) * (12.0 / 11.0);
    assert!((ratio - expected).abs() < 1e-14);
    assert!((ratio - 1.42).abs() < 0.005, "5.5 to 6 mm EI ratio {ratio}");
}
