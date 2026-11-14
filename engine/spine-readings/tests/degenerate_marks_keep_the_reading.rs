//! A missing line direction keeps the fit spread rather than refusing the spine.

#[expect(dead_code, reason = "only exact films and reading inputs are needed")]
mod noisy_films;

use joint_fit::fit_from_films;
use noisy_films::{exact, input, values};
use shapes::landmarks::SacrumTop;
use shapes::levels::Level;
use shapes::made_up::RecipeName;
use shapes::spine_fit::MarkSpread;
use shapes::spine_readings::{NotMeasured, Reading};
use shapes::vertebra::VertebraMarks;
use spine_readings::{ReadingsError, measure_spine};

#[test]
fn coincident_corners_still_return_readings_from_one_or_two_films() {
    for side in [false, true] {
        let mut request = exact(RecipeName::RightThoracic40, side);
        let body = request
            .front
            .marks
            .vertebrae
            .iter_mut()
            .find(|v| v.level == Level::T3)
            .unwrap();
        let VertebraMarks::Front { points } = &mut body.marks else {
            unreachable!()
        };
        points.sup_r.at = points.sup_l.at;
        let fit = fit_from_films(&request).unwrap();
        let mut request = input(&request, fit);
        request.versions = 1;
        let reading = measure_spine(&request).unwrap();
        // The fitted spine has three curves even with this collapsed line.
        assert_eq!(reading.curves.len(), 3);

        let fitted = request
            .fit
            .vertebrae
            .iter()
            .find(|v| v.level == Level::T3)
            .unwrap();
        let body = request
            .front
            .as_ref()
            .unwrap()
            .marks
            .vertebrae
            .iter()
            .find(|v| v.level == Level::T3)
            .unwrap();
        let [_, _, c, d] = body.marks.corners();
        let spacing = request.front.as_ref().unwrap().geometry.mm_per_px.unwrap();
        let lower_width = libm::hypot(c.at.x() - d.at.x(), c.at.y() - d.at.y()) * spacing;
        // Only the top plate falls back. The mean front tilt has variance
        // (2*fit_sd² + 2*(1 mm/lower_width)²)/4.
        let sd = 0.5
            * libm::hypot(
                core::f64::consts::SQRT_2 * fitted.sd.angles_rad[0],
                core::f64::consts::SQRT_2 / lower_width,
            );
        let tilt = reading
            .vertebrae
            .iter()
            .find(|v| v.level == Level::T3)
            .unwrap()
            .coronal_tilt;
        let (_, half) = values(tilt).unwrap();
        assert!((half - 1.281_551_565_544_600_4 * sd.to_degrees()).abs() < 1e-10);
    }
}

#[test]
fn zero_height_bodies_keep_both_fit_spreads_and_a_collapsed_sacrum_is_unreadable() {
    let films = exact(RecipeName::RightThoracic40, true);
    let mut request = input(&films, fit_from_films(&films).unwrap());
    request.versions = 1;
    for film in [&mut request.front, &mut request.side]
        .into_iter()
        .flatten()
    {
        let body = film
            .marks
            .vertebrae
            .iter_mut()
            .find(|v| v.level == Level::T3)
            .unwrap();
        match &mut body.marks {
            VertebraMarks::Front { points } => {
                points.inf_l = points.sup_l;
                points.inf_r = points.sup_r;
            }
            VertebraMarks::Side { points } => {
                points.inf_a = points.sup_a;
                points.inf_p = points.sup_p;
            }
            _ => unreachable!(),
        }
    }
    let SacrumTop::Side { sac_a, sac_p } = request
        .side
        .as_mut()
        .unwrap()
        .marks
        .landmarks
        .sacrum_top
        .as_mut()
        .unwrap()
    else {
        unreachable!()
    };
    sac_p.at = sac_a.at;
    let readings = measure_spine(&request).unwrap();
    assert_eq!(
        readings.sagittal.lordosis_l1_s1,
        Reading::NotMeasured {
            reason: NotMeasured::NoSacrum,
            typical_deg: None
        }
    );
    let mut fallback = request.clone();
    fallback.front = None;
    fallback.side = None;
    let expected = measure_spine(&fallback).unwrap();
    assert_eq!(
        readings.vertebrae.iter().find(|v| v.level == Level::T3),
        expected.vertebrae.iter().find(|v| v.level == Level::T3)
    );

    // Degenerate geometry must not hide an invalid stated spread, and the
    // field must identify the film whose spread needs correcting.
    request.front.as_mut().unwrap().mark_spread = MarkSpread::ShareOfHeight {
        share: -0.1,
        shared_share: 0.0,
    };
    assert!(
        matches!(measure_spine(&request), Err(ReadingsError::InvalidValue { field, .. }) if field == "front.markSpread")
    );
    request.front.as_mut().unwrap().mark_spread = films.front.mark_spread;
    request.side.as_mut().unwrap().mark_spread = MarkSpread::Millimetres {
        sd_mm: 1.0,
        shared_sd_mm: f64::NAN,
    };
    assert!(
        matches!(measure_spine(&request), Err(ReadingsError::InvalidValue { field, .. }) if field == "side.markSpread")
    );
}
