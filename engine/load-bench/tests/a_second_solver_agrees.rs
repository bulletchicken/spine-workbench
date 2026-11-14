//! A separate matrix-free relaxation checks a five-body held correction.

#[path = "a_second_solver_agrees/cases.rs"]
mod cases;
#[path = "a_second_solver_agrees/independent.rs"]
mod independent;
use load_bench::rigid_rod::{HeldOptions, solve_held};

#[test]
fn five_vertebrae_with_one_rod_agree_with_independent_dynamic_relaxation() {
    let mut case = cases::small(1.0, -1.0, 0.0);
    case.construct.rods.truncate(1);
    case.construct.anchors.retain(|a| a.rod == 0);
    // A centre-plane head gives an exactly sagittal problem. The second
    // solver independently evolves x,z and the sagittal rotation only;
    // all omitted components vanish by reflection symmetry.
    for a in &mut case.construct.anchors {
        a.point_mm.y = 0.0;
    }
    for p in &mut case.construct.rods[0].points_mm {
        p.y = 0.0;
    }
    case.guess.rod_poses.truncate(1);
    case.guess.stations_mm = case
        .construct
        .anchors
        .iter()
        .map(|a| a.station_mm)
        .collect();
    let exact = solve_held(
        &case.chain,
        &case.construct,
        &HeldOptions::default(),
        Some(&case.guess),
    )
    .unwrap();
    assert!(exact.settled, "{:?}", exact.reason);
    let reference_z: Vec<_> = case.chain.reference.iter().map(|p| p.centre[2]).collect();
    let heights: Vec<_> = case.chain.reference.iter().map(|p| p.height).collect();
    let stations: Vec<_> = case
        .construct
        .anchors
        .iter()
        .map(|a| a.station_mm)
        .collect();
    let problem = independent::PlanarRelaxation::new(
        &reference_z,
        &heights,
        &case.diagonals,
        case.ground.top_mm.z,
        &case.construct.rods[0].points_mm,
        &stations,
        cases::HEAD_BACK_MM,
    );
    let targets: Vec<_> = case
        .target
        .iter()
        .map(|p| independent::Body {
            centre: p.centre_mm,
            rotation: p.rotation,
        })
        .collect();
    let other = problem.solve(&targets);
    // The independent method uses a finite head penalty, so its residual
    // gap bounds the comparison instead of pretending it is a hard tie.
    assert!(other.gap_mm < 0.0003, "penalty gap {}", other.gap_mm);
    assert!(other.gradient_equivalent < 1e-7);
    for (i, (a, b)) in exact.poses.iter().zip(&other.poses).enumerate() {
        assert!(
            (a.centre_mm - b.centre).norm() < 0.002,
            "body {i} centre {}",
            (a.centre_mm - b.centre).norm()
        );
        assert!(
            (a.rotation - b.rotation).norm() < 1e-4,
            "body {i} turn {}",
            (a.rotation - b.rotation).norm()
        );
    }
    let force_scale = exact
        .head_forces_on_rod
        .iter()
        .map(|f| f.norm())
        .fold(0.0_f64, f64::max);
    for (a, b) in exact.head_forces_on_rod.iter().zip(&other.forces_on_rod) {
        assert!(
            (a - b).norm() < force_scale * 0.002,
            "independent head force difference {}",
            (a - b).norm()
        );
    }
}
