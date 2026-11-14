//! Millimetre spreads use the side film's weighting scale before magnification.

#[expect(dead_code, reason = "only exact films and reading inputs are needed")]
mod noisy_films;

use joint_fit::fit_from_films;
use noisy_films::{exact, input, values};
use shapes::made_up::RecipeName;
use shapes::spine_fit::MarkSpread;
use spine_readings::measure_spine;

#[test]
fn ten_percent_more_stated_side_spacing_divides_the_band_by_one_point_one() {
    let mut films = exact(RecipeName::RightThoracic40, true);
    let mut before = input(&films, fit_from_films(&films).unwrap());
    assert!(before.fit.side_film.is_some());
    before.versions = 1;
    let baseline = measure_spine(&before).unwrap();
    let geometry = &mut films.side.as_mut().unwrap().geometry;
    geometry.mm_per_px = geometry.mm_per_px.map(|s| s * 1.1);
    geometry.mm_per_px_vertical = geometry.mm_per_px_vertical.map(|s| s * 1.1);
    let mut after = input(&films, fit_from_films(&films).unwrap());
    assert!(after.fit.side_film.is_some());
    after.versions = 1;
    let scaled = measure_spine(&after).unwrap();
    for (a, b) in [
        (
            baseline.sagittal.kyphosis_t5_t12,
            scaled.sagittal.kyphosis_t5_t12,
        ),
        (
            baseline.sagittal.lordosis_l1_s1,
            scaled.sagittal.lordosis_l1_s1,
        ),
    ] {
        let (_, original_half) = values(a).unwrap();
        let (_, scaled_half) = values(b).unwrap();
        assert!((scaled_half - original_half / 1.1).abs() < 1e-10);
    }

    // A share-of-height spread divides height by width in the same film,
    // so an overall spacing change cancels out of that plate angle.
    for request in [&mut before, &mut after] {
        request.side.as_mut().unwrap().mark_spread = MarkSpread::ShareOfHeight {
            share: 0.11,
            shared_share: 0.08,
        };
    }
    let original_half = values(measure_spine(&before).unwrap().sagittal.kyphosis_t5_t12)
        .unwrap()
        .1;
    let scaled_half = values(measure_spine(&after).unwrap().sagittal.kyphosis_t5_t12)
        .unwrap()
        .1;
    assert!((scaled_half - original_half).abs() < 1e-10);
}
