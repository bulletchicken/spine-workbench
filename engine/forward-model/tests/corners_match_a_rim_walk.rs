//! The closed-form corner matches a brute-force walk around the endplate
//! rim, for random vertebrae on random parallel films.
//!
//! # The walk
//!
//! The walk never touches the closed form or the camera's jacobian. It
//! steps round the rim `c + w cos t ê_w + d sin t ê_d` in 360 equal steps,
//! sends each 3D rim point through the camera's `project`, and scores it by
//! how far it lands along the long direction in the view (itself found
//! with `project` alone: where `c` lands, and where `c` plus the naming
//! axis lands). The best step brackets the true end to within one step
//! either side, and bisection then closes in on it, keeping the half where
//! the score still rises: `s(t + h) − s(t − h) > 0`, with `h = 0.25` rad.
//! The worst step gives the other corner the same way.
//!
//! # The tolerance: 1e-6 px
//!
//! Under a parallel camera the score is exactly `R cos(t − t*)`, so the
//! symmetric difference `−2R sin(t − t*) sin h` is zero only at the true
//! end, whatever `h`; `h` only sets how steep the test is. What stops the
//! bisection is rounding. Film coordinates here stay under 2·10⁴ px, so
//! each projected coordinate is good to about 1.5·10⁻¹¹ px and a score to
//! `e ≈ 3·10⁻¹¹` px. The bisection can then misjudge `t` only by
//! `|t − t*| ≲ e / (R sin h)`, which moves the point along the outline by
//! at most `(|A| + |B|) · e / (R sin h)`. The long direction is the
//! projected naming axis, so `R ≥ |A|` across an endplate (`R ≥ |B|`
//! front to back), and cases where that axis projects to less than 2% of
//! the camera's scale are left out (they are the end-on cases the model
//! refuses, and their neighbours). With half-sizes from 3 to 40 mm that
//! keeps `|B| / |A|` (or `|A| / |B|`) under 40/3 · 50 ≈ 670, so the walk's
//! own error is under 670 · 3·10⁻¹¹ / sin 0.25 ≈ 8·10⁻⁸ px. 1e-6 px leaves
//! a margin of more than ten on top of that, and is still a million times
//! finer than any pixel.
//!
//! # How many cases
//!
//! Each property runs 1024 random vertebrae on random films, and each case
//! checks four endplate views (top and bottom, across and front to back),
//! so about 4,000 views a property. The known-answers spec's K2 asks for
//! 1e5. The tests run unoptimised on every push, where 1024 cases take
//! about 4 s and 1e5 views (25,000 cases) would take about 90 s; built
//! with `--release`, 25,000 cases pass in about 4 s. To run K2 at full
//! size, raise `CASES` and run with `--release`. K2 also walks 1e5 rim
//! points per endplate; the walk here takes 360 steps and then bisects
//! to rounding, which places each corner more finely than an even 1e5
//! steps would.
//!
//! # Refusals
//!
//! The model may refuse an endplate only when it is seen end-on. Every
//! endplate whose naming axis projects to 2% of the camera's scale or more
//! must be answered, so a refusal threshold raised into the range a real
//! spine reaches fails here rather than going unnoticed.

use forward_model::cameras::Camera;
use forward_model::cameras::parallel::ParallelCamera;
use forward_model::corners::CornerAxis;
use forward_model::endplates::{Endplate, Endplates};
use forward_model::project::{endplate_lines, front_marks, side_marks};
use forward_model::vertebra_params::VertebraParams;
use nalgebra::{Vector2, Vector3};
use proptest::prelude::*;
use shapes::film_geometry::{FilmGeometry, ImageRight, QuarterTurns};
use shapes::marks::Mark;

const TOLERANCE_PX: f64 = 1e-6;
const STEPS: usize = 360;
const SLOPE_STEP: f64 = 0.25;
/// Random vertebra-and-film cases per property; see "How many cases".
const CASES: u32 = 1024;

/// A walked corner: where it lands, and the 3D rim point it came from.
struct Walked {
    at: Vector2<f64>,
    rim: Vector3<f64>,
}

/// The two ends of `plate`'s outline on the film along the projection of
/// `toward_first`: first the one furthest along it, then the one furthest
/// against it.
fn walk<C: Camera>(plate: &Endplate, camera: &C, toward_first: Vector3<f64>) -> [Walked; 2] {
    let rim = |t: f64| {
        plate.centre
            + plate.width_axis * (plate.half_width * libm::cos(t))
            + plate.depth_axis * (plate.half_depth * libm::sin(t))
    };
    let centre = camera.project(&plate.centre);
    let long = camera.project(&(plate.centre + toward_first)) - centre;
    let score = |t: f64| (camera.project(&rim(t)) - centre).dot(&long);
    let step = core::f64::consts::TAU / STEPS as f64;
    let end = |sign: f64| {
        let best = (0..STEPS)
            .map(|k| k as f64 * step)
            .max_by(|&a, &b| (sign * score(a)).total_cmp(&(sign * score(b))))
            .expect("the walk has steps");
        let (mut low, mut high) = (best - step, best + step);
        for _ in 0..80 {
            let mid = 0.5 * (low + high);
            let rising = sign * (score(mid + SLOPE_STEP) - score(mid - SLOPE_STEP)) > 0.0;
            if rising {
                low = mid;
            } else {
                high = mid;
            }
        }
        let t = 0.5 * (low + high);
        Walked {
            at: camera.project(&rim(t)),
            rim: rim(t),
        }
    };
    [end(1.0), end(-1.0)]
}

/// Whether the naming axis is seen well enough for the tolerance above.
fn well_seen<C: Camera>(plate: &Endplate, camera: &C, toward_first: Vector3<f64>) -> bool {
    let jacobian = camera.jacobian(&plate.centre);
    (jacobian * toward_first).norm() >= 0.02 * jacobian.norm()
}

fn at(mark: &Mark) -> Vector2<f64> {
    Vector2::new(mark.at.x(), mark.at.y())
}

/// Checks one endplate's two closed-form corners against the walk, and
/// that the first corner's rim point lies on the side its name says.
fn check<C: Camera>(
    plate: &Endplate,
    camera: &C,
    axis: CornerAxis,
    closed: [Vector2<f64>; 2],
) -> Result<(), TestCaseError> {
    let toward_first = axis.toward_first(plate);
    if !well_seen(plate, camera, toward_first) {
        return Ok(());
    }
    let [first, second] = walk(plate, camera, toward_first);
    let off_first = (closed[0] - first.at).norm();
    let off_second = (closed[1] - second.at).norm();
    prop_assert!(
        off_first <= TOLERANCE_PX,
        "{axis:?} first corner {off_first} px from the walk"
    );
    prop_assert!(
        off_second <= TOLERANCE_PX,
        "{axis:?} second corner {off_second} px from the walk"
    );
    prop_assert!((first.rim - plate.centre).dot(&toward_first) > 0.0);
    prop_assert!((second.rim - plate.centre).dot(&toward_first) < 0.0);
    Ok(())
}

fn vertebra() -> impl Strategy<Value = VertebraParams> {
    (
        (-300.0..300.0, -300.0..300.0, -100.0..600.0),
        (-1.5..1.5, -1.5..1.5, -1.5..1.5),
        5.0..50.0,
        (-0.8..0.8, -0.8..0.8),
        (3.0..40.0, 3.0..40.0, 3.0..40.0, 3.0..40.0),
    )
        .prop_map(|(centre, rotation, height, wedges, sizes)| VertebraParams {
            centre: Vector3::new(centre.0, centre.1, centre.2),
            rotation: Vector3::new(rotation.0, rotation.1, rotation.2),
            height,
            wedge_front: wedges.0,
            wedge_side: wedges.1,
            top_half_width: sizes.0,
            top_half_depth: sizes.1,
            bottom_half_width: sizes.2,
            bottom_half_depth: sizes.3,
        })
}

fn camera() -> impl Strategy<Value = ParallelCamera> {
    (
        prop_oneof![
            Just(ImageRight::PatientLeft),
            Just(ImageRight::PatientRight),
            Just(ImageRight::Anterior),
            Just(ImageRight::Posterior),
        ],
        0_u8..4,
        (500_u32..6000, 500_u32..6000),
        (0.05..1.0, 0.05..1.0),
        (-1000.0..6000.0, -1000.0..6000.0),
    )
        .prop_map(|(image_right, turns, size, spacing, origin)| {
            let geometry = FilmGeometry {
                mm_per_px: Some(spacing.0),
                mm_per_px_vertical: Some(spacing.1),
                quarter_turns: QuarterTurns::new(turns).expect("0 to 3 turns"),
                image_right: Some(image_right),
                ..FilmGeometry::UNKNOWN
            };
            ParallelCamera::for_film(
                &geometry,
                [size.0, size.1],
                Vector2::new(origin.0, origin.1),
            )
            .expect("the geometry is complete")
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(CASES))]

    #[test]
    fn every_endplate_corner_matches_the_walk(v in vertebra(), cam in camera()) {
        let plates = Endplates::of(&v);
        for plate in [plates.top, plates.bottom] {
            for axis in [CornerAxis::LeftRight, CornerAxis::FrontBack] {
                let answer = cam.corners(&plate, axis);
                if well_seen(&plate, &cam, axis.toward_first(&plate)) {
                    prop_assert!(answer.is_ok(), "{axis:?} corners refused on a well-seen endplate");
                }
                if let Ok(closed) = answer {
                    check(&plate, &cam, axis, closed)?;
                }
            }
        }
    }

    #[test]
    fn projected_marks_are_the_walked_corners(v in vertebra(), cam in camera()) {
        let plates = Endplates::of(&v);
        let (top, bottom) = (&plates.top, &plates.bottom);
        let both_seen = |axis: CornerAxis| {
            [top, bottom].iter().all(|plate| well_seen(plate, &cam, axis.toward_first(plate)))
        };
        match front_marks(&v, &cam) {
            Ok(front) => {
                check(top, &cam, CornerAxis::LeftRight, [at(&front.sup_l), at(&front.sup_r)])?;
                check(bottom, &cam, CornerAxis::LeftRight, [at(&front.inf_l), at(&front.inf_r)])?;
            }
            Err(_) => prop_assert!(!both_seen(CornerAxis::LeftRight), "front marks refused"),
        }
        match side_marks(&v, &cam) {
            Ok(side) => {
                check(top, &cam, CornerAxis::FrontBack, [at(&side.sup_a), at(&side.sup_p)])?;
                check(bottom, &cam, CornerAxis::FrontBack, [at(&side.inf_a), at(&side.inf_p)])?;
            }
            Err(_) => prop_assert!(!both_seen(CornerAxis::FrontBack), "side marks refused"),
        }
        match endplate_lines(&v, &cam) {
            Ok(lines) => {
                check(top, &cam, CornerAxis::LeftRight, lines.sup.map(|m| at(&m)))?;
                check(bottom, &cam, CornerAxis::LeftRight, lines.inf.map(|m| at(&m)))?;
            }
            Err(_) => prop_assert!(!both_seen(CornerAxis::LeftRight), "endplate lines refused"),
        }
    }
}
