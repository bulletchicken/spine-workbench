//! Truth checks shared by exact-key and ordinary-intake tests.

use shapes::made_up::CurveRegion;
use shapes::naming::Template;
use shapes::spine_readings::{NotMeasured, Reading, SpineReadings};

pub fn value(reading: Reading) -> f64 {
    match reading {
        Reading::Measured { value_deg, .. } => value_deg,
        other => panic!("a measured answer was expected: {other:?}"),
    }
}

pub fn check(answer: &SpineReadings, truth: &SpineReadings, lumbar_only: bool) {
    let main = |readings: &SpineReadings| {
        readings
            .curves
            .iter()
            .filter(|c| c.region == CurveRegion::MainThoracic)
            .max_by(|a, b| value(a.cobb).abs().total_cmp(&value(b.cobb).abs()))
            .unwrap()
            .clone()
    };
    let actual = main(answer);
    let expected = main(truth);
    assert!(
        (value(actual.cobb).abs() - value(expected.cobb).abs()).abs() <= 6.0,
        "main Cobb: actual {actual:?}, truth {expected:?}"
    );
    let kyphosis_error = if lumbar_only {
        None
    } else {
        Some(value(answer.sagittal.kyphosis_t5_t12) - value(truth.sagittal.kyphosis_t5_t12))
    };
    eprintln!(
        "truth errors: Cobb={} kyphosis={kyphosis_error:?} lordosis={} truth_Cobb={} truth_kyphosis={} truth_lordosis={}",
        value(actual.cobb).abs() - value(expected.cobb).abs(),
        value(answer.sagittal.lordosis_l1_l5) - value(truth.sagittal.lordosis_l1_l5),
        value(expected.cobb),
        value(truth.sagittal.kyphosis_t5_t12),
        value(truth.sagittal.lordosis_l1_l5)
    );
    let names = Template::Standard.names();
    let index = |level| names.iter().position(|l| *l == level).unwrap();
    assert!(
        index(actual.upper_end).abs_diff(index(expected.upper_end)) <= 1,
        "upper end {actual:?}, truth {expected:?}"
    );
    assert!(
        index(actual.lower_end).abs_diff(index(expected.lower_end)) <= 1,
        "lower end {actual:?}, truth {expected:?}"
    );
    for curve in &answer.curves {
        assert!(
            index(curve.lower_end).abs_diff(index(curve.upper_end)) > 1,
            "a curve cannot join neighbouring vertebrae: {curve:?}"
        );
    }
    if lumbar_only {
        assert!(matches!(
            answer.sagittal.kyphosis_t5_t12,
            Reading::NotMeasured {
                reason: NotMeasured::NotOnSideFilm,
                ..
            }
        ));
    } else {
        assert!(
            (value(answer.sagittal.kyphosis_t5_t12) - value(truth.sagittal.kyphosis_t5_t12)).abs()
                <= 8.0,
            "kyphosis: {:?}, truth {:?}",
            answer.sagittal.kyphosis_t5_t12,
            truth.sagittal.kyphosis_t5_t12
        );
    }
    assert!(
        (value(answer.sagittal.lordosis_l1_l5) - value(truth.sagittal.lordosis_l1_l5)).abs() <= 8.0,
        "lordosis: {:?}, truth {:?}",
        answer.sagittal.lordosis_l1_l5,
        truth.sagittal.lordosis_l1_l5
    );
}
