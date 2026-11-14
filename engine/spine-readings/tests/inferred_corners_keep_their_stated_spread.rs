//! An inferred line endpoint carries three times the ordinary corner spread.

#[expect(
    dead_code,
    reason = "only exact films and the reading input are needed"
)]
mod noisy_films;

use joint_fit::fit_from_films;
use noisy_films::{exact, input, values};
use shapes::landmarks::SacrumTop;
use shapes::levels::Level;
use shapes::made_up::RecipeName;
use shapes::marks::Mark;
use shapes::spine_fit::FilmToFit;
use shapes::spine_readings::Reading;
use shapes::vertebra::VertebraMarks;
use spine_readings::measure_spine;

fn width(film: &FilmToFit, a: Mark, b: Mark) -> f64 {
    assert_eq!(film.geometry.quarter_turns.get(), 0);
    let row = film.geometry.mm_per_px.unwrap();
    let column = film.geometry.mm_per_px_vertical.unwrap_or(row);
    libm::hypot((a.at.x() - b.at.x()) * row, (a.at.y() - b.at.y()) * column)
}

fn check(before: Reading, after: Reading, width_mm: f64) {
    let (value, half_width) = values(before).unwrap();
    let (changed_value, changed_half_width) = values(after).unwrap();
    assert_eq!(changed_value, value);
    // Replacing one 1 mm corner SD with 3 mm adds (3² - 1²)/width²
    // radians² to the line variance and hence to the angle variance.
    let added_sd_deg = (libm::sqrt(8.0) / width_mm).to_degrees();
    let expected_half_width = libm::hypot(half_width, 1.281_551_565_544_600_4 * added_sd_deg);
    assert!((changed_half_width - expected_half_width).abs() < 1e-10);
}

#[test]
fn one_inferred_corner_widens_front_side_and_sacral_line_bands() {
    let request = exact(RecipeName::RightThoracic40, true);
    let mut request = input(&request, fit_from_films(&request).unwrap());
    request.versions = 1;
    let before = measure_spine(&request).unwrap();
    let front = request.front.as_mut().unwrap();
    let body = front
        .marks
        .vertebrae
        .iter()
        .find(|v| v.level == Level::T2)
        .unwrap();
    let [a, b, _, _] = body.marks.corners();
    let front_width = width(front, a, b);
    let body = front
        .marks
        .vertebrae
        .iter_mut()
        .find(|v| v.level == Level::T2)
        .unwrap();
    let VertebraMarks::Front { points } = &mut body.marks else {
        unreachable!()
    };
    points.sup_l.inferred = true;

    let side = request.side.as_mut().unwrap();
    let body = side
        .marks
        .vertebrae
        .iter()
        .find(|v| v.level == Level::T2)
        .unwrap();
    let [a, b, _, _] = body.marks.corners();
    let side_width = width(side, a, b);
    let body = side
        .marks
        .vertebrae
        .iter_mut()
        .find(|v| v.level == Level::T2)
        .unwrap();
    let VertebraMarks::Side { points } = &mut body.marks else {
        unreachable!()
    };
    points.sup_a.inferred = true;
    let SacrumTop::Side { sac_a, sac_p } = side.marks.landmarks.sacrum_top.unwrap() else {
        unreachable!()
    };
    let sacrum_width = width(side, sac_a, sac_p);
    let SacrumTop::Side { sac_a, .. } = side.marks.landmarks.sacrum_top.as_mut().unwrap() else {
        unreachable!()
    };
    sac_a.inferred = true;
    let after = measure_spine(&request).unwrap();
    let front_pair = |reading: &shapes::spine_readings::SpineReadings| {
        reading
            .between
            .front
            .iter()
            .find(|p| p.upper == Level::T2 && p.lower == Level::T5)
            .unwrap()
            .angle
    };
    check(front_pair(&before), front_pair(&after), front_width);
    check(
        before.sagittal.kyphosis_t2_t5,
        after.sagittal.kyphosis_t2_t5,
        side_width,
    );
    check(
        before.sagittal.lordosis_l1_s1,
        after.sagittal.lordosis_l1_s1,
        sacrum_width,
    );
}
