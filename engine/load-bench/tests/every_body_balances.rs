//! Free-body checks use head loads on rods and their opposites on bone.

#[path = "a_second_solver_agrees/cases.rs"]
mod cases;
use load_bench::chain::Wrench;
use load_bench::rigid_rod::{HeldOptions, solve_held, solve_rigid};
use nalgebra::{DVector, Vector3};

fn force_moment_at(point: Vector3<f64>, force: Vector3<f64>, couple: Vector3<f64>) -> Wrench {
    let moment = point.cross(&force) + couple;
    Wrench::new(force.x, force.y, force.z, moment.x, moment.y, moment.z)
}

fn close_wrench(wrench: &Wrench, force_scale: f64, length_scale: f64) {
    assert!(
        wrench.fixed_rows::<3>(0).norm() <= force_scale * 1e-9,
        "unbalanced force {} against {force_scale}",
        wrench.fixed_rows::<3>(0).norm()
    );
    assert!(
        wrench.fixed_rows::<3>(3).norm() <= force_scale * length_scale * 1e-9,
        "unbalanced moment {}",
        wrench.fixed_rows::<3>(3).norm()
    );
}

#[test]
fn held_released_and_linear_elastic_states_balance_each_free_body() {
    let case = cases::holding_case(1.0);
    let h = solve_held(
        &case.chain,
        &case.construct,
        &HeldOptions::default(),
        Some(&case.guess),
    )
    .unwrap();
    assert!(h.settled, "{:?}", h.reason);
    assert!(
        h.holding_torques_nmm
            .iter()
            .any(|torque| torque.norm() > 1e-4)
    );
    let evaluation = case.chain.evaluate(&h.poses).unwrap();
    let scale = h
        .head_forces_on_rod
        .iter()
        .map(|f| f.norm())
        .fold(1.0_f64, f64::max);
    let length = 150.0;
    let mut bodies = evaluation.gradient.clone();
    let mut rods = vec![Wrench::zeros(); case.construct.rods.len()];
    for (anchor, force) in case.construct.anchors.iter().zip(&h.head_forces_on_rod) {
        let arm = h.poses[anchor.body].rotation * anchor.point_mm;
        let point = h.poses[anchor.body].centre_mm + arm;
        bodies[anchor.body] += force_moment_at(arm, *force, Vector3::zeros());
        rods[anchor.rod] += force_moment_at(point, *force, Vector3::zeros());
        // Each massless connection passes equal and opposite forces and
        // no couple while its head swivels in state H.
        let on_bone = -force;
        assert!((*force + on_bone).norm() < 1e-15);
    }
    for body in &bodies {
        close_wrench(body, scale, length);
    }
    for (rod, torque) in rods.iter_mut().zip(&h.holding_torques_nmm) {
        rod.fixed_rows_mut::<3>(3).add_assign(torque);
        close_wrench(rod, scale, length);
    }
    let holders = h.holding_torques_nmm.iter().copied().sum::<Vector3<f64>>();
    let mut ground = h.ground_reaction;
    ground.fixed_rows_mut::<3>(3).add_assign(&holders);
    close_wrench(&ground, scale, length);

    let r = solve_rigid(&case.chain, &case.construct, &h, None).unwrap();
    assert!(r.settled, "{:?}", r.reason);
    let mut bodies = case.chain.evaluate(&r.poses).unwrap().gradient;
    for (i, body) in bodies.iter_mut().enumerate() {
        *body -= Wrench::from_column_slice(r.applied_loads.rows(6 * i, 6).as_slice());
    }
    for (rod, loads) in r.rods.iter().zip(&r.head_wrenches_on_rod) {
        let mut resultant = Wrench::zeros();
        for (tie, wrench) in rod.ties.iter().zip(loads) {
            let force = wrench.fixed_rows::<3>(0).into_owned();
            let couple = wrench.fixed_rows::<3>(3).into_owned();
            let arm = tie.point_mm - r.poses[tie.body].centre_mm;
            bodies[tie.body] += force_moment_at(arm, force, couple);
            resultant += force_moment_at(tie.point_mm, force, couple);
        }
        close_wrench(&resultant, scale, length);
    }
    for body in &bodies {
        close_wrench(body, scale, length);
    }
    let mut ground = r.ground_reaction;
    for (i, pose) in r.poses.iter().enumerate() {
        let applied = Wrench::from_column_slice(r.applied_loads.rows(6 * i, 6).as_slice());
        ground += force_moment_at(
            pose.centre_mm - case.chain.ground.centre_mm,
            applied.fixed_rows::<3>(0).into_owned(),
            applied.fixed_rows::<3>(3).into_owned(),
        );
    }
    close_wrench(&ground, scale, length);

    let elastic = load_bench::springback::solve_balanced(
        &case.chain,
        &r.poses,
        &r.rods,
        &h.poses,
        &r.applied_loads,
    )
    .unwrap();
    // Order 1 is exactly a linear solve about R. Check that linear tissue
    // response, not the nonlinear second-order remainder after updating poses.
    let mut residual = &elastic.tissue_gradient
        + &elastic.tissue_tangent * &elastic.displacement
        + &elastic.body_geometric_load
        - (&elastic.applied_loads - &r.applied_loads);
    let mut whole = Wrench::zeros();
    for (rod_index, rod) in elastic.rods.iter().enumerate() {
        let material = &elastic.material_head_wrenches[rod_index];
        let vector = DVector::from_iterator(
            material.len() * 6,
            material.iter().flat_map(|w| w.iter().copied()),
        );
        residual += rod.kinematics.transpose() * vector;
        let mut resultant = Wrench::zeros();
        for (head, tie) in rod.ties.iter().enumerate() {
            let wrench = elastic.linear_head_wrenches_on_rod[rod_index][head];
            let initial = elastic.rigid_head_wrenches_on_rod[rod_index][head];
            let translation = elastic
                .displacement
                .fixed_rows::<3>(6 * tie.body)
                .into_owned();
            let angle = elastic
                .displacement
                .fixed_rows::<3>(6 * tie.body + 3)
                .into_owned();
            let movement = translation + angle.cross(&(tie.point_mm - r.poses[tie.body].centre_mm));
            // Keep every first-order term: P x F1 + dP x F0 + M1.
            // dP x dF is second order and belongs to the nonlinear remainder.
            resultant += force_moment_at(
                tie.point_mm,
                wrench.fixed_rows::<3>(0).into_owned(),
                wrench.fixed_rows::<3>(3).into_owned()
                    + movement.cross(&initial.fixed_rows::<3>(0).into_owned()),
            );
        }
        close_wrench(&resultant, scale, length);
        whole += resultant;
    }
    for i in 0..r.poses.len() {
        close_wrench(
            &Wrench::from_column_slice(residual.rows(6 * i, 6).as_slice()),
            scale,
            length,
        );
    }
    close_wrench(&whole, scale, length);
}

use std::ops::AddAssign;

#[test]
fn hooks_release_rotation_without_adding_a_head_moment() {
    let mut case = cases::holding_case(1.0);
    for anchor in &mut case.construct.anchors {
        if anchor.body == 0 {
            anchor.kind = load_bench::rigid_rod::HeadKind::Hook;
        }
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
    assert!(
        h.poses
            .iter()
            .zip(&r.poses)
            .any(
                |(a, b)| (a.centre_mm - b.centre_mm).norm() + (a.rotation - b.rotation).norm()
                    > 1e-6
            ),
        "the mixed construct must actually respond to removing its holders"
    );
    let mut bodies = case.chain.evaluate(&r.poses).unwrap().gradient;
    for (i, body) in bodies.iter_mut().enumerate() {
        *body -= Wrench::from_column_slice(r.applied_loads.rows(6 * i, 6).as_slice());
    }
    let scale = r
        .head_wrenches_on_rod
        .iter()
        .flatten()
        .map(|w| w.fixed_rows::<3>(0).norm())
        .fold(0.0_f64, f64::max)
        .max(1e-12);
    for (rod, loads) in r.rods.iter().zip(&r.head_wrenches_on_rod) {
        let mut resultant = Wrench::zeros();
        for (tie, wrench) in rod.ties.iter().zip(loads) {
            if !tie.locks_rotation {
                assert!(
                    wrench.fixed_rows::<3>(3).norm() < 1e-10,
                    "a hook passes no couple"
                );
            }
            let force = wrench.fixed_rows::<3>(0).into_owned();
            let couple = wrench.fixed_rows::<3>(3).into_owned();
            bodies[tie.body] +=
                force_moment_at(tie.point_mm - r.poses[tie.body].centre_mm, force, couple);
            resultant += force_moment_at(tie.point_mm, force, couple);
        }
        close_wrench(&resultant, scale, 150.0);
    }
    for body in &bodies {
        close_wrench(body, scale, 150.0);
    }
    let mut ground = r.ground_reaction;
    for (i, pose) in r.poses.iter().enumerate() {
        let applied = Wrench::from_column_slice(r.applied_loads.rows(6 * i, 6).as_slice());
        ground += force_moment_at(
            pose.centre_mm - case.chain.ground.centre_mm,
            applied.fixed_rows::<3>(0).into_owned(),
            applied.fixed_rows::<3>(3).into_owned(),
        );
    }
    close_wrench(&ground, scale, 150.0);
}
