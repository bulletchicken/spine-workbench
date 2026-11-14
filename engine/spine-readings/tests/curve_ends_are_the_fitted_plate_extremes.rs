//! Turns and end vertebrae follow hand-stated fitted plate extremes.

#[expect(dead_code, reason = "only the small fitted-spine builder is needed")]
mod reading_spines;

use reading_spines::{input, near, vertebra};
use shapes::levels::Level;
use spine_readings::curves;

#[test]
fn a_flat_stretch_keeps_its_first_equal_extreme_and_creates_no_turns() {
    // The raw upper profile is flat through T4, then rises monotonically.
    // There is exactly one curve, starting at the first equal minimum.
    let levels = [
        Level::T1,
        Level::T2,
        Level::T3,
        Level::T4,
        Level::T5,
        Level::T6,
    ];
    let request = input(
        levels
            .into_iter()
            .zip([0.0, 0.0, 0.0, 0.0, 5.0, 15.0])
            .enumerate()
            .map(|(i, (level, tilt))| vertebra(level, tilt, 0.0, 0.0, 200.0 - 25.0 * i as f64))
            .collect(),
    );
    let reading = curves::measure(&request, &request.fit).unwrap();
    assert_eq!(reading.curves.len(), 1);
    let curve = &reading.curves[0];
    assert_eq!((curve.upper_index, curve.lower_index), (0, 5));
    near(curve.signed_cobb_deg, 15.0, 1e-12);
}

#[test]
fn each_end_uses_its_own_plate_extreme_in_the_turning_neighbourhood() {
    // Upper tilts peak at T5 (+15); T6's lower plate peaks at +18.
    // Keeping its upper plate at +10 requires centre tilt +14, wedge -8.
    let rows = [
        (Level::T1, -10.0),
        (Level::T2, -5.0),
        (Level::T3, 0.0),
        (Level::T4, 5.0),
        (Level::T5, 15.0),
        (Level::T6, 10.0),
        (Level::T7, 0.0),
    ];
    let mut request = input(
        rows.into_iter()
            .enumerate()
            .map(|(i, (level, tilt))| vertebra(level, tilt, 0.0, 0.0, 200.0 - 25.0 * i as f64))
            .collect(),
    );
    request.fit.vertebrae[5].corner_tilt_rad = Some(14.0_f64.to_radians());
    request.fit.vertebrae[5].pose.wedge_front = -8.0_f64.to_radians();
    let reading = curves::measure(&request, &request.fit).unwrap();
    assert_eq!(reading.curves.len(), 2);
    let curve = &reading.curves[0];
    assert_eq!((curve.upper_index, curve.lower_index), (0, 5));
    near(curve.signed_cobb_deg, 28.0, 1e-12);
}
