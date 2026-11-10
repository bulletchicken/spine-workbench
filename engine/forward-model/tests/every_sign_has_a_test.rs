//! One test per sign rule in `docs/conventions/signs-and-wedges.md` and
//! `docs/conventions/patient-frame.md`. Each expected position is worked
//! out by hand in the comment above it, from the rule alone, and then the
//! rule itself is checked in words a reader can see on the film.
//!
//! Every case uses the same parallel film unless it says otherwise:
//! 0.5 mm square pixels, no quarter turns, a film 2000 by 4000 pixels, and
//! the patient origin landing at upright pixel (1000, 3000), so
//! `u = 1000 + 2 · across` and `v = 3000 − 2 · z`. The vertebra has height
//! 20 mm, half-widths 15 mm and half-depths 10 mm unless it says otherwise.
//! `√3/2 = cos 30° = sin 60°`.

use forward_model::cameras::parallel::ParallelCamera;
use forward_model::endplates::Endplates;
use forward_model::pedicles::pedicle_centres;
use forward_model::project::{front_marks, side_marks};
use forward_model::vertebra_params::VertebraParams;
use nalgebra::{Vector2, Vector3};
use shapes::film_geometry::{FilmGeometry, ImageRight, QuarterTurns};
use shapes::marks::{FrontMarks, Mark, PedicleMark, SideMarks};

const TOLERANCE_PX: f64 = 1e-9;
const THIRTY_DEGREES: f64 = core::f64::consts::FRAC_PI_6;
const SIXTY_DEGREES: f64 = core::f64::consts::FRAC_PI_3;

fn root3() -> f64 {
    3.0_f64.sqrt()
}

fn film(image_right: ImageRight) -> ParallelCamera {
    let geometry = FilmGeometry {
        mm_per_px: Some(0.5),
        image_right: Some(image_right),
        ..FilmGeometry::UNKNOWN
    };
    ParallelCamera::for_film(&geometry, [2000, 4000], Vector2::new(1000.0, 3000.0))
        .expect("the geometry is complete")
}

fn vertebra(centre: [f64; 3]) -> VertebraParams {
    VertebraParams {
        centre: centre.into(),
        rotation: Vector3::zeros(),
        height: 20.0,
        wedge_front: 0.0,
        wedge_side: 0.0,
        top_half_width: 15.0,
        top_half_depth: 10.0,
        bottom_half_width: 15.0,
        bottom_half_depth: 10.0,
    }
}

fn front(v: &VertebraParams, image_right: ImageRight) -> FrontMarks {
    front_marks(v, &film(image_right)).expect("the vertebra has front corners")
}

fn side(v: &VertebraParams, image_right: ImageRight) -> SideMarks {
    side_marks(v, &film(image_right)).expect("the vertebra has side corners")
}

#[track_caller]
fn assert_at(mark: Mark, x: f64, y: f64) {
    let off = libm::hypot(mark.at.x() - x, mark.at.y() - y);
    assert!(
        off <= TOLERANCE_PX,
        "expected ({x}, {y}), got {:?}, {off} px away",
        mark.at
    );
}

fn pedicle(mark: PedicleMark) -> Mark {
    mark.mark().expect("a projected pedicle is visible")
}

fn mid_x(a: Mark, b: Mark) -> f64 {
    0.5 * (a.at.x() + b.at.x())
}

#[test]
fn tilt_tips_the_column_toward_the_patients_right_going_up() {
    // rotX(+30°) as the rotation vector (π/6, 0, 0). The top endplate centre
    // is (0, 0, 10) turned: (0, −10 sin 30°, 10 cos 30°) = (0, −5, 5√3), so
    // y = −5 and u = 1000 − 10 = 990. The bottom centre is the opposite,
    // u = 1010. Each endplate midpoint on a front film is the mean of its
    // two corners.
    let v = VertebraParams {
        rotation: Vector3::new(THIRTY_DEGREES, 0.0, 0.0),
        ..vertebra([0.0, 0.0, 100.0])
    };
    let m = front(&v, ImageRight::PatientLeft);
    let (top, bottom) = (mid_x(m.sup_l, m.sup_r), mid_x(m.inf_l, m.inf_r));
    assert!((top - 990.0).abs() <= TOLERANCE_PX);
    assert!((bottom - 1010.0).abs() <= TOLERANCE_PX);
    // Patient right is image left on this film: the top is further right
    // on the patient than the bottom.
    assert!(top < bottom);
}

#[test]
fn tilt_lowers_the_right_side_of_an_endplate() {
    // rotX(+30°): the width axis ŷ turns to (0, cos 30°, sin 30°) =
    // (0, √3/2, ½), so the left end of the top endplate is 15 · ½ = 7.5 mm
    // above its centre and the right end 7.5 mm below. Top centre
    // (0, −5, 100 + 5√3):
    //   supL = (0, −5 + 7.5√3, 107.5 + 5√3) → (990 + 15√3, 2785 − 10√3)
    //   supR = (0, −5 − 7.5√3,  92.5 + 5√3) → (990 − 15√3, 2815 − 10√3)
    // and from the bottom centre (0, 5, 100 − 5√3):
    //   infL = (0, 5 + 7.5√3, 107.5 − 5√3) → (1010 + 15√3, 2785 + 10√3)
    //   infR = (0, 5 − 7.5√3,  92.5 − 5√3) → (1010 − 15√3, 2815 + 10√3)
    let r = root3();
    let v = VertebraParams {
        rotation: Vector3::new(THIRTY_DEGREES, 0.0, 0.0),
        ..vertebra([0.0, 0.0, 100.0])
    };
    let m = front(&v, ImageRight::PatientLeft);
    assert_at(m.sup_l, 990.0 + 15.0 * r, 2785.0 - 10.0 * r);
    assert_at(m.sup_r, 990.0 - 15.0 * r, 2815.0 - 10.0 * r);
    assert_at(m.inf_l, 1010.0 + 15.0 * r, 2785.0 + 10.0 * r);
    assert_at(m.inf_r, 1010.0 - 15.0 * r, 2815.0 + 10.0 * r);
    // The right corner sits lower on the film (larger v) by 2 · 15 mm.
    assert!((m.sup_r.at.y() - m.sup_l.at.y() - 30.0).abs() <= TOLERANCE_PX);
}

#[test]
fn lean_tips_the_column_forward_going_up() {
    // rotY(+30°) as (0, π/6, 0). The top centre (0, 0, 10) turns to
    // (10 sin 30°, 0, 10 cos 30°) = (5, 0, 5√3): x = 5, so on a side film
    // facing anterior u = 1000 + 10 = 1010. The bottom centre is at
    // u = 990.
    let v = VertebraParams {
        rotation: Vector3::new(0.0, THIRTY_DEGREES, 0.0),
        ..vertebra([0.0, 0.0, 100.0])
    };
    let m = side(&v, ImageRight::Anterior);
    let (top, bottom) = (mid_x(m.sup_a, m.sup_p), mid_x(m.inf_a, m.inf_p));
    assert!((top - 1010.0).abs() <= TOLERANCE_PX);
    assert!((bottom - 990.0).abs() <= TOLERANCE_PX);
    // Anterior is image right on this film: the top is further forward.
    assert!(top > bottom);
}

#[test]
fn lean_lowers_the_front_of_an_endplate() {
    // rotY(+30°): the depth axis −x̂ (front to back) turns to
    // (−cos 30°, 0, sin 30°) = (−√3/2, 0, ½), so the anterior end,
    // centre − 10 · depth axis, is 5 mm below the centre and the posterior
    // end 5 mm above. Top centre (5, 0, 100 + 5√3):
    //   supA = (5 + 5√3, 0,  95 + 5√3) → (1010 + 10√3, 2810 − 10√3)
    //   supP = (5 − 5√3, 0, 105 + 5√3) → (1010 − 10√3, 2790 − 10√3)
    // and from the bottom centre (−5, 0, 100 − 5√3):
    //   infA = (−5 + 5√3, 0,  95 − 5√3) → (990 + 10√3, 2810 + 10√3)
    //   infP = (−5 − 5√3, 0, 105 − 5√3) → (990 − 10√3, 2790 + 10√3)
    let r = root3();
    let v = VertebraParams {
        rotation: Vector3::new(0.0, THIRTY_DEGREES, 0.0),
        ..vertebra([0.0, 0.0, 100.0])
    };
    let m = side(&v, ImageRight::Anterior);
    assert_at(m.sup_a, 1010.0 + 10.0 * r, 2810.0 - 10.0 * r);
    assert_at(m.sup_p, 1010.0 - 10.0 * r, 2790.0 - 10.0 * r);
    assert_at(m.inf_a, 990.0 + 10.0 * r, 2810.0 + 10.0 * r);
    assert_at(m.inf_p, 990.0 - 10.0 * r, 2790.0 + 10.0 * r);
    // The front corner sits lower on the film by 2 · 10 mm.
    assert!((m.sup_a.at.y() - m.sup_p.at.y() - 20.0).abs() <= TOLERANCE_PX);
}

#[test]
fn twist_turns_the_front_toward_the_patients_left() {
    // rotZ(+30°) as (0, 0, π/6). The front, x̂, turns to (cos 30°, sin 30°, 0):
    // toward +y, the patient's left. The pedicles sit behind the centre at
    // (−12.5, ±11.7, 3.6) (see the pedicle test), so they swing the other
    // way: y′ = x sin 30° + y cos 30° = −6.25 ± 5.85√3, and on a front film
    // facing patient left u = 1000 + 2y′ = 987.5 ± 11.7√3, v = 3000 − 2 · 103.6.
    // The body's half-extent across is √((15 cos 30°)² + (10 sin 30°)²) =
    // √193.75 mm, so its corners are at u = 1000 ± 2√193.75 = 1000 ± √775.
    let r = root3();
    let v = VertebraParams {
        rotation: Vector3::new(0.0, 0.0, THIRTY_DEGREES),
        ..vertebra([0.0, 0.0, 100.0])
    };
    let m = front(&v, ImageRight::PatientLeft);
    assert_at(pedicle(m.ped_l), 987.5 + 11.7 * r, 2792.8);
    assert_at(pedicle(m.ped_r), 987.5 - 11.7 * r, 2792.8);
    assert_at(m.sup_l, 1000.0 + 775.0_f64.sqrt(), 2780.0);
    assert_at(m.sup_r, 1000.0 - 775.0_f64.sqrt(), 2780.0);
    assert_at(m.inf_l, 1000.0 + 775.0_f64.sqrt(), 2820.0);
    assert_at(m.inf_r, 1000.0 - 775.0_f64.sqrt(), 2820.0);
    // The front went left, so the pedicles behind went right: their
    // midpoint moved 2 · 12.5 · sin 30° = 12.5 px to image left (patient
    // right) of the body's centre line.
    let between = mid_x(pedicle(m.ped_l), pedicle(m.ped_r));
    assert!((between - 987.5).abs() <= TOLERANCE_PX);
    assert!(between < mid_x(m.sup_l, m.sup_r));
}

#[test]
fn an_endplate_tilts_by_half_the_wedge() {
    // ωf = 60°: the top normal is rotX(+30°)ẑ = (0, −½, √3/2) and the
    // bottom one rotX(−30°)ẑ = (0, ½, √3/2). Both point up.
    let r = root3();
    let v = VertebraParams {
        wedge_front: SIXTY_DEGREES,
        ..vertebra([0.0, 0.0, 100.0])
    };
    let plates = Endplates::of(&v);
    assert!((plates.top.normal - Vector3::new(0.0, -0.5, r / 2.0)).norm() < 1e-15);
    assert!((plates.bottom.normal - Vector3::new(0.0, 0.5, r / 2.0)).norm() < 1e-15);
}

#[test]
fn front_wedge_makes_the_patients_left_taller() {
    // ωf = 60°, so each endplate tilts 30°. The top width axis is
    // normalise(ŷ − (ŷ·n)n) with n = (0, −½, √3/2): (0, ¾, √3/4)/(√3/2) =
    // (0, √3/2, ½), so the top's left end is 7.5 mm higher than its centre
    // at z = 110 and its right end 7.5 mm lower; the bottom (width axis
    // (0, √3/2, −½)) is the other way round about z = 90:
    //   supL = (0,  7.5√3, 117.5) → (1000 + 15√3, 2765)
    //   supR = (0, −7.5√3, 102.5) → (1000 − 15√3, 2795)
    //   infL = (0,  7.5√3,  82.5) → (1000 + 15√3, 2835)
    //   infR = (0, −7.5√3,  97.5) → (1000 − 15√3, 2805)
    let r = root3();
    let v = VertebraParams {
        wedge_front: SIXTY_DEGREES,
        ..vertebra([0.0, 0.0, 100.0])
    };
    let m = front(&v, ImageRight::PatientLeft);
    assert_at(m.sup_l, 1000.0 + 15.0 * r, 2765.0);
    assert_at(m.sup_r, 1000.0 - 15.0 * r, 2795.0);
    assert_at(m.inf_l, 1000.0 + 15.0 * r, 2835.0);
    assert_at(m.inf_r, 1000.0 - 15.0 * r, 2805.0);
    // Left side 35 mm (70 px) tall, right side 5 mm (10 px).
    let left = m.inf_l.at.y() - m.sup_l.at.y();
    let right = m.inf_r.at.y() - m.sup_r.at.y();
    assert!((left - 70.0).abs() <= TOLERANCE_PX && (right - 10.0).abs() <= TOLERANCE_PX);
    assert!(left > right);
}

#[test]
fn side_wedge_makes_the_front_shorter() {
    // ωs = 60°: the top normal is rotY(+30°)ẑ = (½, 0, √3/2), its width axis
    // stays ŷ and its depth axis n × ŷ = (−√3/2, 0, ½), so the anterior end
    // (centre − 10 · depth axis) is 5 mm lower than the centre at z = 110.
    // The bottom normal (−½, 0, √3/2) gives the depth axis (−√3/2, 0, −½),
    // so its anterior end is 5 mm higher than z = 90:
    //   supA = ( 5√3, 0, 105) → (1000 + 10√3, 2790)
    //   supP = (−5√3, 0, 115) → (1000 − 10√3, 2770)
    //   infA = ( 5√3, 0,  95) → (1000 + 10√3, 2810)
    //   infP = (−5√3, 0,  85) → (1000 − 10√3, 2830)
    let r = root3();
    let v = VertebraParams {
        wedge_side: SIXTY_DEGREES,
        ..vertebra([0.0, 0.0, 100.0])
    };
    let m = side(&v, ImageRight::Anterior);
    assert_at(m.sup_a, 1000.0 + 10.0 * r, 2790.0);
    assert_at(m.sup_p, 1000.0 - 10.0 * r, 2770.0);
    assert_at(m.inf_a, 1000.0 + 10.0 * r, 2810.0);
    assert_at(m.inf_p, 1000.0 - 10.0 * r, 2830.0);
    // Front 10 mm (20 px) tall, back 30 mm (60 px).
    let front_height = m.inf_a.at.y() - m.sup_a.at.y();
    let back_height = m.inf_p.at.y() - m.sup_p.at.y();
    assert!((front_height - 20.0).abs() <= TOLERANCE_PX);
    assert!((back_height - 60.0).abs() <= TOLERANCE_PX);
    assert!(front_height < back_height);
}

#[test]
fn facing_patient_left_puts_the_left_corners_on_image_right() {
    // Centre (0, 10, 100): the top endplate's left end is at y = 25 and its
    // right end at y = −5, both at z = 110; across = y, so
    // supL = (1000 + 50, 2780), supR = (1000 − 10, 2780).
    let m = front(&vertebra([0.0, 10.0, 100.0]), ImageRight::PatientLeft);
    assert_at(m.sup_l, 1050.0, 2780.0);
    assert_at(m.sup_r, 990.0, 2780.0);
    assert!(m.sup_l.at.x() > m.sup_r.at.x());
}

#[test]
fn facing_patient_right_puts_the_left_corners_on_image_left() {
    // The same ends with across = −y: supL = (1000 − 50, 2780),
    // supR = (1000 + 10, 2780). The names stay with the patient.
    let m = front(&vertebra([0.0, 10.0, 100.0]), ImageRight::PatientRight);
    assert_at(m.sup_l, 950.0, 2780.0);
    assert_at(m.sup_r, 1010.0, 2780.0);
    assert!(m.sup_l.at.x() < m.sup_r.at.x());
}

#[test]
fn facing_anterior_puts_the_front_corners_on_image_right() {
    // The top endplate's anterior end is at x = 10, its posterior end at
    // x = −10; across = x, so supA = (1020, 2780), supP = (980, 2780).
    let m = side(&vertebra([0.0, 10.0, 100.0]), ImageRight::Anterior);
    assert_at(m.sup_a, 1020.0, 2780.0);
    assert_at(m.sup_p, 980.0, 2780.0);
    assert!(m.sup_a.at.x() > m.sup_p.at.x());
}

#[test]
fn facing_posterior_puts_the_front_corners_on_image_left() {
    // across = −x: supA = (980, 2780), supP = (1020, 2780).
    let m = side(&vertebra([0.0, 10.0, 100.0]), ImageRight::Posterior);
    assert_at(m.sup_a, 980.0, 2780.0);
    assert_at(m.sup_p, 1020.0, 2780.0);
    assert!(m.sup_a.at.x() < m.sup_p.at.x());
}

#[test]
fn up_is_toward_the_top_of_the_film_and_height_spans_the_endplates() {
    // v = 3000 − 2z: the top endplate at z = 100 + 10 lands at v = 2780,
    // the bottom one at z = 100 − 10 at v = 2820, h = 20 mm = 40 px apart.
    let m = front(&vertebra([0.0, 10.0, 100.0]), ImageRight::PatientLeft);
    assert_at(m.inf_l, 1050.0, 2820.0);
    assert_at(m.inf_r, 990.0, 2820.0);
    assert!(m.sup_l.at.y() < m.inf_l.at.y());
    assert!((m.inf_l.at.y() - m.sup_l.at.y() - 40.0).abs() <= TOLERANCE_PX);
}

#[test]
fn pedicles_sit_behind_to_each_side_and_above_the_centre() {
    // d̄ = 10, w̄ = 15, h = 20: the offsets are (−1.25 · 10, ±0.78 · 15,
    // 0.18 · 20) = (−12.5, ±11.7, 3.6), + to the left. From the centre
    // (0, 10, 100) that is (−12.5, 21.7, 103.6) and (−12.5, −1.7, 103.6),
    // which land at (1000 + 43.4, 3000 − 207.2) and (1000 − 3.4, 2792.8)
    // on a front film facing patient left.
    let v = vertebra([0.0, 10.0, 100.0]);
    let [left, right] = pedicle_centres(&v);
    assert!((left - Vector3::new(-12.5, 21.7, 103.6)).norm() < 1e-12);
    assert!((right - Vector3::new(-12.5, -1.7, 103.6)).norm() < 1e-12);
    let m = front(&v, ImageRight::PatientLeft);
    assert_at(pedicle(m.ped_l), 1043.4, 2792.8);
    assert_at(pedicle(m.ped_r), 996.6, 2792.8);
}

#[test]
fn quarter_turns_are_undone_clockwise_onto_the_stored_film() {
    // A film stored 4000 wide and 2000 tall, shown upright after one
    // clockwise quarter turn (so 2000 wide and 4000 tall upright), with
    // stored spacings 0.4 mm along a row and 0.5 mm down a column. An odd
    // turn swaps them: su = 0.5, sv = 0.4. supL of the centre (0, 10, 100)
    // vertebra lands upright at u = 1000 + 25/0.5 = 1050,
    // v = 3000 − 110/0.4 = 2725. Undoing one clockwise turn,
    // x = v = 2725 and y = (2000 − 1) − u = 949. supR (y = −5) lands
    // upright at u = 990, so at (2725, 1009).
    let geometry = FilmGeometry {
        mm_per_px: Some(0.4),
        mm_per_px_vertical: Some(0.5),
        quarter_turns: QuarterTurns::new(1).expect("one turn"),
        image_right: Some(ImageRight::PatientLeft),
        ..FilmGeometry::UNKNOWN
    };
    let camera = ParallelCamera::for_film(&geometry, [4000, 2000], Vector2::new(1000.0, 3000.0))
        .expect("the geometry is complete");
    let m = front_marks(&vertebra([0.0, 10.0, 100.0]), &camera).expect("corners");
    assert_at(m.sup_l, 2725.0, 949.0);
    assert_at(m.sup_r, 2725.0, 1009.0);
    // Upright up (toward the head) is stored left after a clockwise turn:
    // the bottom endplate (upright v = 3000 − 90/0.4 = 2775) lies further
    // right on the stored film.
    assert_at(m.inf_l, 2775.0, 949.0);
}
