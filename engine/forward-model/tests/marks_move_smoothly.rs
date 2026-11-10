//! Small changes in a vertebra's thirteen numbers make small, smooth
//! changes in its marks: nothing in the model jumps, so a fit's gradient
//! can be trusted everywhere a real spine can be.
//!
//! # The bounds
//!
//! Vertebrae here are poses a spine can take: each rotation component
//! within ±0.6 rad, each wedge within ±0.4 rad, half-sizes 3 to 40 mm,
//! heights 5 to 50 mm, seen on films with pixels of 0.05 to 1 mm. Every
//! mark lies within 100 mm of the vertebra's centre, which is 2·10³ px on
//! the finest pixel. A unit change in a rotation-vector component turns
//! the vertebra by at most a radian, and its rate of turning changes by
//! at most about one radian per unit as well; a wedge tilts a rim of at
//! most 40 mm by half as much; a centre, height or size change moves a
//! mark at most 1 mm per mm, 20 px. So each mark's first and second
//! derivatives with respect to any one number stay under `K = 10⁴` px per
//! unit (and per unit squared), a margin of about five over the worst
//! case.
//!
//! Two tests use them. One nudges each of the thirteen numbers by `δ` =
//! 1e-6 at random poses: no mark may move more than `K δ` = 0.01 px. The
//! other sweeps each rotation component and each wedge across its whole
//! range in 5000 equal steps `δ`, passing exactly through zero rotation,
//! where the rotation formula changes form. Between neighbours no mark
//! may move more than `K δ`, and its second difference
//! `p(k+1) − 2 p(k) + p(k−1)` may not exceed `K δ²` (under 6e-4 px for the
//! rotation sweeps). A jump of size `J` anywhere along the sweep shows up
//! as a second difference of about `J`, so the sweep catches any jump
//! larger than about a thousandth of a pixel; corners swapping names
//! would be tens of pixels.

use forward_model::cameras::parallel::ParallelCamera;
use forward_model::project::{front_marks, side_marks};
use forward_model::vertebra_params::VertebraParams;
use nalgebra::Vector2;
use proptest::prelude::*;
use shapes::film_geometry::{FilmGeometry, ImageRight};
use shapes::marks::PedicleMark;

const K: f64 = 1e4;
const NUDGE: f64 = 1e-6;
const SWEEP_STEPS: i32 = 5_000;

/// The positions of the rotation components and wedges among the thirteen
/// numbers, with the half-range each sweeps.
const ANGLES: [(usize, f64); 5] = [(3, 0.6), (4, 0.6), (5, 0.6), (7, 0.4), (8, 0.4)];

type Points = [Vector2<f64>; 10];

fn numbers() -> impl Strategy<Value = [f64; VertebraParams::COUNT]> {
    (
        (-300.0..300.0, -300.0..300.0, -100.0..600.0),
        (-0.6..0.6, -0.6..0.6, -0.6..0.6),
        5.0..50.0,
        (-0.4..0.4, -0.4..0.4),
        (3.0..40.0, 3.0..40.0, 3.0..40.0, 3.0..40.0),
    )
        .prop_map(|(c, r, h, w, s)| {
            [
                c.0, c.1, c.2, r.0, r.1, r.2, h, w.0, w.1, s.0, s.1, s.2, s.3,
            ]
        })
}

fn film(image_right: ImageRight, mm_per_px: f64) -> ParallelCamera {
    let geometry = FilmGeometry {
        mm_per_px: Some(mm_per_px),
        image_right: Some(image_right),
        ..FilmGeometry::UNKNOWN
    };
    ParallelCamera::for_film(&geometry, [3000, 6000], Vector2::new(1500.0, 5000.0))
        .expect("the geometry is complete")
}

/// Every point a vertebra puts on a front film and a side film.
fn points(
    n: [f64; VertebraParams::COUNT],
    front: &ParallelCamera,
    side: &ParallelCamera,
) -> Points {
    let v = VertebraParams::from_numbers(n);
    let f = front_marks(&v, front).expect("a real pose has front corners");
    let s = side_marks(&v, side).expect("a real pose has side corners");
    let pedicle = |p: PedicleMark| p.mark().expect("a projected pedicle is visible").at;
    [
        f.sup_l.at,
        f.sup_r.at,
        f.inf_l.at,
        f.inf_r.at,
        pedicle(f.ped_l),
        pedicle(f.ped_r),
        s.sup_a.at,
        s.sup_p.at,
        s.inf_a.at,
        s.inf_p.at,
    ]
    .map(|p| Vector2::new(p.x(), p.y()))
}

/// The largest distance any point moved between two sets.
fn largest_move(before: &Points, after: &Points) -> f64 {
    before
        .iter()
        .zip(after)
        .map(|(a, b)| (a - b).norm())
        .fold(0.0, f64::max)
}

/// The largest step and the largest second difference of any point while
/// number `i` sweeps from `−half` to `+half` in steps that land exactly on
/// zero.
fn sweep(
    mut n: [f64; VertebraParams::COUNT],
    i: usize,
    half: f64,
    front: &ParallelCamera,
    side: &ParallelCamera,
) -> (f64, f64) {
    let step = 2.0 * half / f64::from(SWEEP_STEPS);
    let mut at = |k: i32| {
        n[i] = f64::from(k - SWEEP_STEPS / 2) * step;
        points(n, front, side)
    };
    let (mut before, mut now) = (at(0), at(1));
    let (mut largest_step, mut largest_bend) = (largest_move(&before, &now), 0.0_f64);
    for k in 2..=SWEEP_STEPS {
        let next = at(k);
        largest_step = largest_step.max(largest_move(&now, &next));
        for p in 0..before.len() {
            largest_bend = largest_bend.max((next[p] - now[p] * 2.0 + before[p]).norm());
        }
        (before, now) = (now, next);
    }
    (largest_step, largest_bend)
}

fn check_sweep(
    n: [f64; VertebraParams::COUNT],
    i: usize,
    half: f64,
    front: &ParallelCamera,
    side: &ParallelCamera,
) -> Result<(), TestCaseError> {
    let step = 2.0 * half / f64::from(SWEEP_STEPS);
    let (largest_step, largest_bend) = sweep(n, i, half, front, side);
    prop_assert!(
        largest_step <= K * step,
        "number {i}: a step moved a mark {largest_step} px"
    );
    prop_assert!(
        largest_bend <= K * step * step,
        "number {i}: a mark bent {largest_bend} px in one step"
    );
    Ok(())
}

fn front_facing() -> impl Strategy<Value = ImageRight> {
    prop_oneof![
        Just(ImageRight::PatientLeft),
        Just(ImageRight::PatientRight)
    ]
}

fn side_facing() -> impl Strategy<Value = ImageRight> {
    prop_oneof![Just(ImageRight::Anterior), Just(ImageRight::Posterior)]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn a_nudge_to_any_number_moves_every_mark_a_little(
        n in numbers(),
        mm_per_px in 0.05..1.0,
        front in front_facing(),
        side in side_facing(),
    ) {
        let (front, side) = (film(front, mm_per_px), film(side, mm_per_px));
        let before = points(n, &front, &side);
        for i in 0..VertebraParams::COUNT {
            let mut nudged = n;
            nudged[i] += NUDGE;
            let moved = largest_move(&before, &points(nudged, &front, &side));
            prop_assert!(moved <= K * NUDGE, "number {i} moved a mark {moved} px");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(4))]

    #[test]
    fn sweeping_an_angle_never_jumps(
        n in numbers(),
        mm_per_px in 0.05..1.0,
        front in front_facing(),
        side in side_facing(),
    ) {
        let (front, side) = (film(front, mm_per_px), film(side, mm_per_px));
        for (i, half) in ANGLES {
            check_sweep(n, i, half, &front, &side)?;
        }
    }
}

#[test]
fn sweeping_through_no_rotation_never_jumps() {
    // Each rotation component swept through zero with the other two at
    // zero, so the rotation vector itself passes through (0, 0, 0).
    let straight = [
        0.0, 10.0, 100.0, 0.0, 0.0, 0.0, 20.0, 0.1, -0.1, 15.0, 10.0, 16.0, 11.0,
    ];
    let (front, side) = (
        film(ImageRight::PatientLeft, 0.05),
        film(ImageRight::Anterior, 0.05),
    );
    for i in 3..6 {
        if let Err(failure) = check_sweep(straight, i, 0.6, &front, &side) {
            panic!("{failure}");
        }
    }
}
