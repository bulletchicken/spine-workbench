//! A fitting contour introduces no tissue deformation or locked-in load.

#[path = "a_second_solver_agrees/cases.rs"]
mod cases;
#[path = "realistic_cases/mod.rs"]
mod realistic_cases;
use forward_model::rotation::matrix_from_rotation_vector;
use load_bench::rigid_rod::{
    Anchor, Construct, HeadKind, HeldOptions, Rod, SolveError, StopReason, solve_held, solve_rigid,
};
use nalgebra::Vector3;

#[test]
fn fitting_rods_leave_all_three_correction_states_at_rest() {
    let case = cases::small(1.0, 0.0, 0.0);
    let standing = case.chain.standing();
    let held = solve_held(&case.chain, &case.construct, &HeldOptions::default(), None)
        .expect("held solve");
    assert!(held.settled, "{:?}", held.reason);
    cases::assert_poses_close(&held.poses, &standing, 1e-11);
    assert!(held.head_forces_on_rod.iter().all(|f| f.norm() < 1e-9));
    assert!(held.holding_torques_nmm.iter().all(|m| m.norm() < 1e-9));
    let rigid = solve_rigid(&case.chain, &case.construct, &held, None).expect("released solve");
    assert!(rigid.settled, "{:?}", rigid.reason);
    cases::assert_poses_close(&rigid.poses, &standing, 1e-11);
    assert!(
        rigid
            .head_wrenches_on_rod
            .iter()
            .flatten()
            .all(|w| w.norm() < 1e-9)
    );
    let elastic = load_bench::springback::solve_balanced(
        &case.chain,
        &rigid.poses,
        &rigid.rods,
        &held.poses,
        &rigid.applied_loads,
    )
    .expect("springback");
    assert!(elastic.settled);
    cases::assert_poses_close(&elastic.poses, &standing, 1e-11);
    assert!(
        elastic
            .head_wrenches_on_rod
            .iter()
            .flatten()
            .all(|w| w.norm() < 1e-9)
    );
    assert!(elastic.displacement.norm() < 1e-11);
    assert_eq!(elastic.stiff_rod_share, 0.0);
}

#[test]
fn a_changed_construct_cannot_index_outside_the_held_state() {
    let mut case = cases::small(1.0, 0.0, 0.0);
    let held = solve_held(&case.chain, &case.construct, &HeldOptions::default(), None).unwrap();
    case.construct.anchors[0].body = case.chain.reference.len();
    assert!(matches!(
        solve_rigid(&case.chain, &case.construct, &held, None),
        Err(SolveError::InvalidInput(_))
    ));
    case.construct.anchors[0].body = 0;
    case.construct.anchors[0].rod = case.construct.rods.len();
    assert!(matches!(
        solve_rigid(&case.chain, &case.construct, &held, None),
        Err(SolveError::InvalidInput(_))
    ));
}

fn axial_endpoint_case(gap_mm: f64) -> (load_bench::chain::Chain, Construct) {
    use load_bench::chain::{Chain, Ground};
    use load_bench::joint_springs::Stiffness;
    let base = cases::small(1.0, 0.0, 0.0);
    let reference = &base.chain.reference[..2];
    // Synthetic stiff rotations keep the closed-form axial branch regular.
    // Bottom body is fixed at z=85; the upper rest centre is z=110.
    // The nominal contour starts gap_mm below the upper rest head.
    // Its unconstrained axial minimum is station -gap_mm.
    let stiffness = Stiffness::new([1.0, 1.0, 1.0, 1e9, 1e9, 1e9]).unwrap();
    let chain = Chain::new(
        reference,
        &[stiffness; 2],
        Ground {
            top_mm: base.chain.ground.centre_mm,
            rotation: base.chain.ground.rotation,
        },
    )
    .unwrap();
    let construct = Construct {
        anchors: vec![
            Anchor {
                body: 0,
                point_mm: Vector3::zeros(),
                rod: 0,
                station_mm: 0.0,
                kind: HeadKind::Screw,
            },
            Anchor {
                body: 1,
                point_mm: Vector3::zeros(),
                rod: 0,
                station_mm: 25.0 - gap_mm,
                kind: HeadKind::Screw,
            },
        ],
        rods: vec![Rod {
            points_mm: vec![
                Vector3::new(0.0, 0.0, 110.0 - gap_mm),
                Vector3::new(0.0, 0.0, 85.0),
            ],
            properties: cases::material(),
        }],
    };
    (chain, construct)
}

#[test]
fn a_free_head_is_refused_when_no_continuation_is_requested() {
    let (chain, construct) = axial_endpoint_case(5.0);
    let result = solve_held(
        &chain,
        &construct,
        &HeldOptions {
            fixed_rods: true,
            end_continuation_mm: 0.0,
            ..HeldOptions::default()
        },
        None,
    )
    .unwrap();
    assert!(!result.settled);
    assert_eq!(result.reason, StopReason::HeadAtContourEnd);
    assert_eq!(result.contour_end_anchors, [0]);
    assert!(result.head_forces_on_rod.is_empty());
}

#[test]
fn a_free_head_uses_straight_continuation_without_moving_nominal_pins_or_contour() {
    let (chain, construct) = axial_endpoint_case(5.0);
    let nominal = construct.rods[0].points_mm.clone();
    let standing = chain.standing();
    let held = solve_held(
        &chain,
        &construct,
        &HeldOptions {
            fixed_rods: true,
            ..HeldOptions::default()
        },
        None,
    )
    .unwrap();
    assert!(held.settled, "{:?}", held.reason);
    assert_eq!(held.end_continuation_mm, 20.0);
    assert!((held.stations_mm[0] + 5.0).abs() < 1e-10);
    assert_eq!(held.stations_mm[1], 20.0);
    assert!(held.contour_end_anchors.is_empty());
    cases::assert_poses_close(&held.poses, &standing, 1e-10);
    assert!(held.head_forces_on_rod.iter().all(|f| f.norm() < 1e-9));
    let rigid = solve_rigid(&chain, &construct, &held, None).unwrap();
    assert!(rigid.settled, "{:?}", rigid.reason);
    assert_eq!(construct.rods[0].points_mm, nominal);
    assert_eq!(
        rigid.rod_points_mm[0],
        vec![Vector3::new(0.0, 0.0, 110.0), nominal[0], nominal[1]]
    );
    assert_eq!(rigid.rod_head_stations_mm[0], vec![0.0, 25.0]);
    assert!(
        rigid
            .head_wrenches_on_rod
            .iter()
            .flatten()
            .all(|w| w.norm() < 1e-9)
    );
    let longer = solve_held(
        &chain,
        &construct,
        &HeldOptions {
            fixed_rods: true,
            end_continuation_mm: 40.0,
            ..HeldOptions::default()
        },
        None,
    )
    .unwrap();
    assert!(longer.settled);
    let released = solve_rigid(&chain, &construct, &longer, None).unwrap();
    assert!(released.settled);
    assert_eq!(released.rod_points_mm, rigid.rod_points_mm);
    assert_eq!(released.rod_head_stations_mm, rigid.rod_head_stations_mm);
    assert!(
        (&released.rods[0].stiffness - &rigid.rods[0].stiffness)
            .abs()
            .max()
            < 1e-12
    );
}

#[test]
fn an_otherwise_settled_head_at_the_continuation_end_is_refused() {
    let (chain, construct) = axial_endpoint_case(20.0);
    let initial = load_bench::rigid_rod::HeldGuess {
        poses: chain.standing(),
        rod_poses: vec![load_bench::chain::Pose {
            centre_mm: Vector3::zeros(),
            rotation: nalgebra::Matrix3::identity(),
        }],
        stations_mm: vec![-20.0, 5.0],
    };
    let result = solve_held(
        &chain,
        &construct,
        &HeldOptions {
            fixed_rods: true,
            ..HeldOptions::default()
        },
        Some(&initial),
    )
    .unwrap();
    assert_eq!(result.iterations, 0);
    assert!(result.constraint_max_mm < 1e-12);
    assert!(result.stationarity_max < 1e-12);
    assert!(!result.settled);
    assert_eq!(result.reason, StopReason::HeadAtContourEnd);
    assert_eq!(result.contour_end_anchors, [0]);
    assert!(result.head_forces_on_rod.is_empty());
    assert!(result.holding_torques_nmm.is_empty());
    assert!(result.pin_tangential_force_n.is_empty());
}

#[test]
fn invalid_continuation_extent_is_refused_before_solving() {
    let (chain, construct) = axial_endpoint_case(5.0);
    for extent in [-1.0, f64::INFINITY, f64::NAN] {
        assert!(matches!(
            solve_held(
                &chain,
                &construct,
                &HeldOptions {
                    end_continuation_mm: extent,
                    ..HeldOptions::default()
                },
                None
            ),
            Err(SolveError::InvalidInput(_))
        ));
    }
}

#[test]
fn a_rod_attached_to_only_one_body_returns_a_singular_construct_code() {
    let case = cases::small(1.0, 0.0, 0.0);
    let body = 2;
    let centre = case.chain.standing()[body].centre_mm;
    let points = [
        Vector3::new(-30.0, 0.0, 5.0),
        Vector3::new(-30.0, 0.0, -5.0),
    ];
    let construct = Construct {
        rods: vec![Rod {
            points_mm: points.iter().map(|point| centre + point).collect(),
            properties: cases::material(),
        }],
        anchors: points
            .iter()
            .enumerate()
            .map(|(head, point)| Anchor {
                body,
                point_mm: *point,
                rod: 0,
                station_mm: head as f64 * 10.0,
                kind: HeadKind::Screw,
            })
            .collect(),
    };
    assert!(matches!(
        solve_held(&case.chain, &construct, &HeldOptions::default(), None),
        Err(SolveError::SingularConstruct(_))
    ));
}

#[test]
fn initial_slide_values_cannot_move_a_prescribed_material_station() {
    let case = cases::small(1.0, 0.0, 0.0);
    let baseline = solve_held(&case.chain, &case.construct, &HeldOptions::default(), None).unwrap();
    let mut guess = case.guess.clone();
    for &i in &baseline.pinned_anchors {
        guess.stations_mm[i] -= 0.25;
    }
    let result = solve_held(
        &case.chain,
        &case.construct,
        &HeldOptions::default(),
        Some(&guess),
    )
    .unwrap();
    assert!(result.settled, "{:?}", result.reason);
    cases::assert_poses_close(&result.poses, &baseline.poses, 1e-10);
    for &i in &result.pinned_anchors {
        assert_eq!(result.stations_mm[i], case.construct.anchors[i].station_mm);
    }
    for station in &mut guess.stations_mm {
        *station -= 0.25;
    }
    let fixed = solve_held(
        &case.chain,
        &case.construct,
        &HeldOptions {
            fixed_stations: true,
            fixed_rods: true,
            ..HeldOptions::default()
        },
        Some(&guess),
    )
    .unwrap();
    assert!(fixed.settled, "{:?}", fixed.reason);
    cases::assert_poses_close(&fixed.poses, &case.chain.standing(), 1e-10);
    for (station, anchor) in fixed.stations_mm.iter().zip(&case.construct.anchors) {
        assert_eq!(*station, anchor.station_mm);
    }
}

#[test]
fn incompatible_fixed_head_spans_return_a_reason_and_no_head_loads() {
    let mut case = cases::small(1.0, 0.0, 0.0);
    // Chosen synthetic geometry: fixed rod points are 20 mm apart, while
    // each rigid body's two local heads remain 30 mm apart. No rigid pose
    // can change that distance, so this is a geometric impossibility.
    for rod in &mut case.construct.rods {
        for point in &mut rod.points_mm {
            point.y *= 2.0 / 3.0;
        }
    }
    let state = solve_held(
        &case.chain,
        &case.construct,
        &HeldOptions {
            fixed_rods: true,
            fixed_stations: true,
            ..HeldOptions::default()
        },
        Some(&case.guess),
    )
    .expect("well-formed but incompatible geometry");
    assert!(!state.settled);
    assert!(matches!(
        state.reason,
        StopReason::InconsistentConstraints | StopReason::LineSearch
    ));
    assert!(state.head_forces_on_rod.is_empty());
    assert!(state.holding_torques_nmm.is_empty());
    assert!(state.pin_tangential_force_n.is_empty());
}

#[test]
fn positive_tissue_stiffness_settles_the_free_turn_about_two_heads() {
    let case = cases::small(1.0, 0.0, 0.0);
    let standing = case.chain.standing();
    let mut initial = case.guess.clone();
    for ((pose, reference), angle) in initial
        .poses
        .iter_mut()
        .zip(&standing)
        .zip([-0.03, 0.02, -0.04, 0.01, 0.025])
    {
        let midpoint = reference.centre_mm + Vector3::new(cases::HEAD_BACK_MM, 0.0, 0.0);
        let turn = matrix_from_rotation_vector(&(Vector3::y() * angle));
        pose.centre_mm = midpoint + turn * (reference.centre_mm - midpoint);
        pose.rotation = turn * reference.rotation;
    }
    // These finite turns preserve both actual head points exactly. They
    // are feasible freedoms, not a singular mode of the positive tissue.
    for anchor in &case.construct.anchors {
        let changed = &initial.poses[anchor.body];
        let reference = &standing[anchor.body];
        assert!(
            (changed.centre_mm + changed.rotation * anchor.point_mm
                - reference.centre_mm
                - reference.rotation * anchor.point_mm)
                .norm()
                < 1e-12
        );
    }
    assert!(case.chain.evaluate(&initial.poses).unwrap().energy_nmm > 1.0);
    let state = solve_held(
        &case.chain,
        &case.construct,
        &HeldOptions {
            fixed_rods: true,
            fixed_stations: true,
            ..HeldOptions::default()
        },
        Some(&initial),
    )
    .expect("regular bilateral constraints");
    assert!(state.settled, "{:?}", state.reason);
    assert_eq!(state.reason, StopReason::Converged);
    cases::assert_poses_close(&state.poses, &standing, 1e-9);
    assert!(state.energy_nmm < 1e-10);
}

#[test]
fn rt1_rods_supplied_through_standing_heads_need_no_correction_or_holding_torque() {
    let mut case = realistic_cases::build(shapes::made_up::RecipeName::RightThoracic40, 1).unwrap();
    let standing = case.chain.standing();
    // Keep the actual table-derived head geometry and material; replace only
    // this regression's contour by layout's dense zero-extra standing-head curve.
    for (rod, contour) in case.construct.rods.iter_mut().enumerate() {
        let mut order: Vec<_> = case
            .construct
            .anchors
            .iter()
            .enumerate()
            .filter(|(_, a)| a.rod == rod)
            .map(|(i, _)| i)
            .collect();
        order.sort_by_key(|&i| case.construct.anchors[i].body);
        let heads: Vec<[f64; 3]> = order
            .iter()
            .map(|&i| {
                let a = &case.construct.anchors[i];
                (standing[a.body].centre_mm + standing[a.body].rotation * a.point_mm).into()
            })
            .collect();
        let levels: Vec<_> = order
            .iter()
            .map(|&i| case.chain.reference[case.construct.anchors[i].body].level)
            .collect();
        contour.points_mm = implant_layout::rods::contour(&heads, &levels, 0.0)
            .unwrap()
            .points_mm
            .into_iter()
            .map(Vector3::from)
            .collect();
        let stations = load_bench::rod_beams::polyline_stations(&contour.points_mm).unwrap();
        for (&anchor, head) in order.iter().zip(&heads) {
            let point = Vector3::from(*head);
            let knot = contour
                .points_mm
                .iter()
                .position(|p| (*p - point).norm() < 1e-10)
                .expect("zero-extra contour retains every supplied standing head");
            case.construct.anchors[anchor].station_mm = stations[knot];
        }
    }
    let check_zero_loads = |stage: &str, rods: &[Vec<load_bench::chain::Wrench>]| {
        let force_n = rods
            .iter()
            .flatten()
            .map(|w| w.fixed_rows::<3>(0).norm())
            .fold(0.0_f64, f64::max);
        let moment_nmm = rods
            .iter()
            .flatten()
            .map(|w| w.fixed_rows::<3>(3).norm())
            .fold(0.0_f64, f64::max);
        eprintln!("standing RT1 {stage}: max force {force_n:e} N, max moment {moment_nmm:e} Nmm");
        assert!(force_n < 1e-9, "{stage}: {force_n:e} N");
        assert!(moment_nmm < 1e-7, "{stage}: {moment_nmm:e} Nmm");
    };
    let held = solve_held(&case.chain, &case.construct, &HeldOptions::default(), None).unwrap();
    assert!(held.settled, "{:?}", held.reason);
    assert_eq!(held.iterations, 0);
    assert_eq!(held.energy_nmm, 0.0);
    cases::assert_poses_close(&held.poses, &standing, 1e-11);
    assert!(held.head_forces_on_rod.iter().all(|v| v.norm() < 1e-9));
    assert!(held.holding_torques_nmm.iter().all(|v| v.norm() < 1e-9));
    let rigid = solve_rigid(&case.chain, &case.construct, &held, None).unwrap();
    assert!(rigid.settled, "{:?}", rigid.reason);
    cases::assert_poses_close(&rigid.poses, &standing, 1e-9);
    check_zero_loads("R", &rigid.head_wrenches_on_rod);
    let elastic = load_bench::springback::solve_balanced(
        &case.chain,
        &rigid.poses,
        &rigid.rods,
        &held.poses,
        &rigid.applied_loads,
    )
    .unwrap();
    assert!(elastic.settled);
    cases::assert_poses_close(&elastic.poses, &standing, 1e-9);
    assert!(elastic.displacement.norm() < 1e-9);
    check_zero_loads("order1", &elastic.head_wrenches_on_rod);
}

#[test]
fn an_undefined_holder_half_turn_is_refused_without_choosing_a_plane() {
    let case = cases::small(1.0, -1.0, 0.0);
    let mut initial = case.guess.clone();
    initial.rod_poses[0].rotation =
        matrix_from_rotation_vector(&(Vector3::x() * core::f64::consts::PI));
    assert!(matches!(
        solve_held(
            &case.chain,
            &case.construct,
            &HeldOptions::default(),
            Some(&initial)
        ),
        Err(SolveError::InvalidInput("holder swing-twist ambiguity"))
    ));
}
