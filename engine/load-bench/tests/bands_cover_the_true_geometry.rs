//! Geometry-band coverage on noisy made-up films, not a patient comparison.

#[expect(
    dead_code,
    reason = "the shared made-up film and mechanics fixtures expose other cases"
)]
#[path = "../../joint-fit/tests/answer_keys/mod.rs"]
mod answer_keys;
#[path = "a_second_solver_agrees/cases.rs"]
mod cases;

use joint_fit::fit_from_films;
use load_bench::chain::{Ground, Pose};
use load_bench::rigid_rod::{HeldOptions, solve_held, solve_rigid};
use nalgebra::{Matrix3, Vector3};
use shapes::levels::Level;
use shapes::made_up::RecipeName;
use shapes::spine_fit::{FilmToFit, FitFromFilms, MarkSpread, TypicalSource};

/// Own test choice: middle-eight-in-ten bands, estimated with 32 versions.
const NOMINAL_COVERAGE: f64 = 0.8;

fn experiment(seed: u32, versions: u32) -> bool {
    let (mut films, key) = answer_keys::input(RecipeName::RightThoracic40, seed, 0.5);
    // Three adjacent real made-up bodies keep this numerical check small.
    films.front.vertebrae.retain(|v| v.level >= Level::L3);
    films.side.vertebrae.retain(|v| v.level >= Level::L3);
    let spread = MarkSpread::Millimetres {
        sd_mm: 0.5,
        shared_sd_mm: 0.0,
    };
    let fit = fit_from_films(&FitFromFilms {
        front: FilmToFit {
            marks: films.front,
            geometry: films.front_geometry,
            size: films.front_size,
            mark_spread: spread,
            facing_assumed: false,
        },
        side: Some(FilmToFit {
            marks: films.side,
            geometry: films.side_geometry,
            size: films.side_size,
            mark_spread: spread,
            facing_assumed: false,
        }),
        typical: TypicalSource::None,
        twist_readings: Vec::new(),
    })
    .expect("noisy made-up marks fit");
    assert!(fit.converged);
    let mut target: Vec<_> = fit
        .vertebrae
        .iter()
        .map(|v| Pose::from_vertebra(&v.pose))
        .collect();
    // Own prescribed test contour: a 1 mm posterior bow at the middle head.
    // Hold this one supplied placement fixed for both truth and all versions.
    target[1].centre_mm.x -= 1.0;
    let ground = Ground {
        top_mm: Vector3::zeros(),
        rotation: Matrix3::identity(),
    };
    let solve = |reference| {
        let case = cases::build(reference, target.clone(), ground.clone(), 1.0, 0..=2);
        let held = solve_held(
            &case.chain,
            &case.construct,
            &HeldOptions::default(),
            Some(&case.guess),
        )
        .expect("held solve");
        assert!(held.settled, "held {:?}", held.reason);
        let rigid = solve_rigid(&case.chain, &case.construct, &held, None).expect("released solve");
        assert!(rigid.settled, "released {:?}", rigid.reason);
        // Signed anterior component avoids the positive-selection bias of a
        // maximum magnitude. The left rod's middle head is index one.
        rigid.head_wrenches_on_rod[0][1][0]
    };
    let truth = solve(
        key.poses
            .into_iter()
            .filter(|p| p.level >= Level::L3)
            .collect(),
    );
    let versions =
        spine_readings::readings::versions(&fit, u64::from(seed) + 101, versions, 0..versions)
            .expect("versions");
    let mut loads: Vec<_> = versions
        .versions
        .iter()
        .map(|v| solve(v.fit.vertebrae.iter().map(|v| v.pose).collect()))
        .collect();
    loads.sort_by(f64::total_cmp);
    let percentile = |p: f64| {
        let at = p * (loads.len() - 1) as f64;
        let lower = libm::floor(at) as usize;
        let upper = libm::ceil(at) as usize;
        loads[lower] + (loads[upper] - loads[lower]) * (at - lower as f64)
    };
    let (low, high) = (percentile(0.1), percentile(0.9));
    eprintln!(
        "geometry coverage seed {seed}: true signed apical force {truth:.6} N, p10 {low:.6}, p90 {high:.6}"
    );
    (low..=high).contains(&truth)
}

fn coverage(seeds: u32, versions: u32) {
    let covered = (1..=seeds)
        .filter(|&seed| experiment(seed, versions))
        .count();
    let rate = covered as f64 / f64::from(seeds);
    // A two-sided three-binomial-SE envelope, plus one trial's resolution.
    // The tiny CI check is a smoke test; the ignored run estimates coverage.
    // This tolerance is chosen from n and 0.8 before seeing the results.
    let tolerance = 3.0
        * libm::sqrt(NOMINAL_COVERAGE * (1.0 - NOMINAL_COVERAGE) / f64::from(seeds))
        + 1.0 / f64::from(seeds);
    if seeds == 2 {
        eprintln!("geometry coverage smoke check: {covered}/{seeds}, at least one required");
        // At nominal coverage, two misses have probability 0.2² = 0.04.
        assert!(covered >= 1, "both small-sample geometry bands missed");
    } else {
        eprintln!(
            "geometry coverage: {covered}/{seeds}, target {NOMINAL_COVERAGE}, tolerance {tolerance}"
        );
        assert!(
            (rate - NOMINAL_COVERAGE).abs() <= tolerance,
            "coverage {rate}, tolerance {tolerance}"
        );
    }
}

#[test]
fn noisy_films_cover_the_true_order_zero_force_in_a_small_check() {
    coverage(2, 8);
}

#[test]
#[ignore = "full numerical geometry coverage experiment"]
fn noisy_films_cover_the_true_order_zero_force_over_more_cases() {
    coverage(64, 32);
}
