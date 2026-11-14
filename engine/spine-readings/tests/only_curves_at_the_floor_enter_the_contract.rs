//! Hand-worked Cobb magnitudes distinguish named curves from small wobbles.

#[expect(
    dead_code,
    reason = "other hand-spine fixtures are used by the report suite"
)]
mod reading_spines;

use reading_spines::{input, measured, near, vertebra};
use shapes::levels::Level;
use spine_readings::{curves, measure_spine, readings::versions};

fn single_curve(degrees: f64) -> shapes::spine_readings::ReadingsInput {
    input(vec![
        vertebra(Level::T6, 0.0, 0.0, 0.0, 100.0),
        vertebra(Level::T8, degrees * 0.5, 0.0, -8.0, 75.0),
        vertebra(Level::T10, degrees, 0.0, 0.0, 50.0),
    ])
}

#[test]
fn nine_point_nine_keeps_all_tilts_and_pairs_but_names_no_curve() {
    let input = single_curve(9.9);
    let answer = measure_spine(&input).unwrap();
    assert!(answer.curves.is_empty());
    assert_eq!(answer.vertebrae.len(), 3);
    assert_eq!(answer.between.front.len(), 3);
    assert_eq!(answer.between.side.len(), 3);
    for (v, expected) in answer.vertebrae.iter().zip([0.0, 4.95, 9.9]) {
        near(measured(v.coronal_tilt).0, expected, 1e-12);
    }
    near(measured(answer.between.front[1].angle).0, 9.9, 1e-12);
    // The level rules still receive the uncut curve geometry.
    let rule_geometry = curves::measure(&input, &input.fit).unwrap();
    assert_eq!(rule_geometry.curves.len(), 1);
    near(rule_geometry.curves[0].cobb_deg, 9.9, 1e-12);
}

#[test]
fn ten_degrees_enters_at_the_inclusive_floor_on_either_side() {
    for sign in [-1.0, 1.0] {
        let answer = measure_spine(&single_curve(sign * 10.0)).unwrap();
        assert_eq!(answer.curves.len(), 1);
        near(measured(answer.curves[0].cobb).0, sign * 10.0, 1e-12);
        assert_eq!(answer.curves[0].versions_found, 32);
    }
}

#[test]
fn a_forty_five_degree_curve_and_a_six_degree_wobble_name_one_curve() {
    // The alternating extremes are -22.5, +22.5 and +16.5 degrees:
    // 45 degrees to the right, then 6 degrees back to the left.
    let input = input(vec![
        vertebra(Level::T6, -22.5, 0.0, 0.0, 125.0),
        vertebra(Level::T8, 0.0, 0.0, -10.0, 100.0),
        vertebra(Level::T10, 22.5, 0.0, 0.0, 75.0),
        vertebra(Level::T12, 16.5, 0.0, 1.0, 50.0),
    ]);
    let answer = measure_spine(&input).unwrap();
    assert_eq!(answer.curves.len(), 1);
    near(measured(answer.curves[0].cobb).0, 45.0, 1e-12);
    assert_eq!(answer.vertebrae.len(), 4);
    assert_eq!(answer.between.front.len(), 6);
    assert_eq!(curves::measure(&input, &input.fit).unwrap().curves.len(), 2);
}

#[test]
fn versions_found_counts_the_fixed_pair_reaching_the_floor() {
    let mut input = single_curve(10.5);
    input.versions = 64;
    // Only the nominal lower end moves. Its plate SD is sqrt(2)*2 degrees.
    // It reaches 10 degrees iff its standard-normal draw is >= -0.177,
    // whose probability is about 0.570. Stratification therefore puts
    // some versions on each side, without depending on a lucky seed.
    input.fit.vertebrae[2].sd.angles_rad[0] = 2.0_f64.to_radians();
    let answer = measure_spine(&input).unwrap();
    let curve = &answer.curves[0];
    assert!(curve.versions_found > 0 && curve.versions_found < input.versions);
    let sampled = versions(
        &input.fit,
        u64::from(input.seed),
        input.versions,
        0..input.versions,
    )
    .unwrap();
    let expected = sampled
        .versions
        .iter()
        .filter(|v| {
            let upper =
                spine_readings::curves::geometry::upper_coronal_tilt_deg(&v.fit.vertebrae[0]);
            let lower =
                spine_readings::curves::geometry::lower_coronal_tilt_deg(&v.fit.vertebrae[2]);
            (lower - upper).abs() >= 10.0
        })
        .count() as u32;
    assert_eq!(curve.versions_found, expected);
    assert!(
        (36..=37).contains(&expected),
        "{expected} fixed pairs reach the floor"
    );
}
