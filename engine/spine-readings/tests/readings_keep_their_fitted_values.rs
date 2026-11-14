//! Fitted values, uncertainty bands and missing observations have separate answers.

mod reading_spines;

use reading_spines::{hand_spine, input, measured, near, vertebra};
use shapes::film_geometry::{ImageRight, QuarterTurns};
use shapes::landmarks::SacrumTop;
use shapes::levels::Level;
use shapes::spine_readings::{Band, BodySide, FitTier, NotMeasured, Reading, SpineReadings};
use spine_readings::{measure_spine, readings::versions};

#[test]
fn seven_vertebrae_keep_their_hand_worked_angles_and_signs() {
    let input = hand_spine();
    let answer = measure_spine(&input).unwrap();
    assert_eq!(answer.tier, FitTier::TwoFilms);
    assert_eq!(answer.between.front.len(), 21);
    assert_eq!(answer.between.side.len(), 21);
    for (v, (tilt, lean)) in answer.vertebrae.iter().zip([
        (-15.0, 20.0),
        (-8.0, 12.0),
        (0.0, 0.0),
        (8.0, -10.0),
        (15.0, -15.0),
        (8.0, -5.0),
        (0.0, 25.0),
    ]) {
        near(measured(v.coronal_tilt).0, tilt, 1e-12);
        near(measured(v.forward_lean).0, lean, 1e-12);
        near(measured(v.twist).0, 0.0, 1e-12);
    }
    for (reading, expected) in [
        (answer.sagittal.kyphosis_t2_t5, 8.0),
        (answer.sagittal.kyphosis_t5_t12, 22.0),
        (answer.sagittal.kyphosis_t10_l2, 5.0),
        (answer.sagittal.lordosis_l1_l5, 40.0),
        (answer.sagittal.lordosis_l1_s1, 50.0),
    ] {
        near(measured(reading).0, expected, 1e-10);
    }
    let front_pair = answer
        .between
        .front
        .iter()
        .find(|p| p.upper == Level::T2 && p.lower == Level::L1)
        .unwrap();
    let side_pair = answer
        .between
        .side
        .iter()
        .find(|p| p.upper == Level::T2 && p.lower == Level::L1)
        .unwrap();
    near(measured(front_pair.angle).0, 30.0, 1e-12);
    near(measured(side_pair.angle).0, 35.0, 1e-12);
    let curve = &answer.curves[0];
    assert_eq!(
        (curve.upper_end, curve.lower_end, curve.apex),
        (Level::T2, Level::L1, Level::T10)
    );
    assert_eq!(curve.convex, BodySide::Right);
    near(measured(curve.cobb).0, 30.0, 1e-12);
}

#[test]
fn a_sacrum_reading_uses_upright_pixels_facing_and_the_fits_spacing() {
    let input = hand_spine();
    for turns in 0..4 {
        for facing in [ImageRight::Anterior, ImageRight::Posterior] {
            let mut changed = input.clone();
            let film = changed.side.as_mut().unwrap();
            let SacrumTop::Side { sac_a, sac_p } =
                film.marks.landmarks.sacrum_top.as_mut().unwrap()
            else {
                unreachable!()
            };
            for mark in [sac_a, sac_p] {
                let x = if facing == ImageRight::Anterior {
                    mark.at.x()
                } else {
                    600.0 - mark.at.x()
                };
                let y = mark.at.y();
                mark.at = match turns {
                    0 => shapes::pixels::Px::new(x, y),
                    1 => shapes::pixels::Px::new(y, 999.0 - x),
                    2 => shapes::pixels::Px::new(999.0 - x, 999.0 - y),
                    _ => shapes::pixels::Px::new(999.0 - y, x),
                };
            }
            film.geometry.quarter_turns = QuarterTurns::new(turns).unwrap();
            film.geometry.image_right = Some(facing);
            film.marks.named_for = facing;
            let placement = changed.fit.side_film.as_mut().unwrap();
            // Placement spacings are stored-axis spacings and swap on odd turns.
            placement.mm_per_px = if turns % 2 == 0 {
                [2.0, 3.0]
            } else {
                [3.0, 2.0]
            };
            near(
                measured(measure_spine(&changed).unwrap().sagittal.lordosis_l1_s1).0,
                50.0,
                1e-10,
            );
        }
    }
}

fn all_readings(value: &SpineReadings) -> Vec<Reading> {
    let mut out = Vec::new();
    for v in &value.vertebrae {
        out.extend([v.coronal_tilt, v.forward_lean, v.twist]);
    }
    out.extend([
        value.sagittal.kyphosis_t2_t5,
        value.sagittal.kyphosis_t5_t12,
        value.sagittal.kyphosis_t10_l2,
        value.sagittal.lordosis_l1_l5,
        value.sagittal.lordosis_l1_s1,
    ]);
    for c in &value.curves {
        out.extend([c.cobb, c.apex_twist, c.bending_cobb]);
    }
    out.extend(
        value
            .between
            .front
            .iter()
            .chain(&value.between.side)
            .map(|p| p.angle),
    );
    out
}

#[test]
fn no_typical_side_value_can_change_a_measured_reading() {
    let mut input = hand_spine();
    input.fit.side_film = None;
    input.side = None;
    for v in &mut input.fit.vertebrae {
        v.inferred.depth = true;
        v.inferred.twist = true;
        v.rms_mm.side = None;
        v.sd.angles_rad = [0.02, 0.4, 0.6];
    }
    let before = measure_spine(&input).unwrap();
    assert_eq!(before.tier, FitTier::FrontFilmOnly);
    for v in &before.vertebrae {
        assert!(matches!(
            v.forward_lean,
            Reading::NotMeasured {
                reason: NotMeasured::NoSideFilm,
                typical_deg: Some(_)
            }
        ));
        assert!(matches!(
            v.twist,
            Reading::NotMeasured {
                reason: NotMeasured::TwistNotMeasured,
                typical_deg: Some(_)
            }
        ));
    }
    for reading in [
        before.sagittal.kyphosis_t2_t5,
        before.sagittal.kyphosis_t5_t12,
        before.sagittal.kyphosis_t10_l2,
        before.sagittal.lordosis_l1_l5,
    ]
    .into_iter()
    .chain(before.between.side.iter().map(|p| p.angle))
    {
        assert!(
            matches!(
                reading,
                Reading::NotMeasured {
                    reason: NotMeasured::NoSideFilm,
                    typical_deg: Some(_)
                }
            ),
            "{reading:?}"
        );
    }
    assert!(matches!(
        before.sagittal.lordosis_l1_s1,
        Reading::NotMeasured {
            reason: NotMeasured::NoSideFilm,
            ..
        }
    ));
    for v in &mut input.fit.vertebrae {
        v.angles.forward_lean_rad += 0.3;
        v.angles.twist_rad += 0.4;
        v.pose.rotation = [0.1, v.angles.forward_lean_rad, v.angles.twist_rad];
        v.pose.wedge_side += 0.2;
        v.pose.centre[0] += 31.0;
        v.pose.top_half_depth *= 1.2;
        v.pose.bottom_half_depth *= 1.3;
    }
    let after = measure_spine(&input).unwrap();
    let only_measured = |v: &SpineReadings| {
        all_readings(v)
            .into_iter()
            .filter(|r| matches!(r, Reading::Measured { .. }))
            .collect::<Vec<_>>()
    };
    assert_eq!(only_measured(&before), only_measured(&after));
    assert_ne!(
        before.vertebrae[0].forward_lean,
        after.vertebrae[0].forward_lean
    );
    let drawn = versions(
        &input.fit,
        u64::from(input.seed),
        input.versions,
        0..input.versions,
    )
    .unwrap();
    for version in drawn.versions {
        for (a, b) in input.fit.vertebrae.iter().zip(version.fit.vertebrae) {
            assert_eq!(a.angles.forward_lean_rad, b.angles.forward_lean_rad);
            assert_eq!(a.angles.twist_rad, b.angles.twist_rad);
            assert_eq!(a.pose.centre[0], b.pose.centre[0]);
        }
    }
}

fn percentiles(mut values: Vec<f64>) -> Band {
    values.sort_by(f64::total_cmp);
    // Locate each percentile in the ordered observations and interpolate
    // its two neighbours independently of the band implementation.
    let at = |p: f64| {
        let x = p * (values.len() - 1) as f64;
        let i = libm::floor(x) as usize;
        let j = libm::ceil(x) as usize;
        values[i] * (1.0 - (x - i as f64)) + values[j] * (x - i as f64)
    };
    Band {
        p10: at(0.1),
        p50: at(0.5),
        p90: at(0.9),
    }
}

#[test]
fn a_pair_band_combines_the_two_independent_plate_spreads_without_a_seed() {
    let mut input = hand_spine();
    input.fit.vertebrae[0].sd.angles_rad[0] = 2.0_f64.to_radians();
    input.fit.vertebrae[1].sd.angles_rad[0] = 3.0_f64.to_radians();
    let answer = measure_spine(&input).unwrap();
    let (nominal, band) = measured(answer.between.front[0].angle);
    // Each endplate has sqrt(2) times its body's spread. A difference of
    // independent 2*sqrt(2) and 3*sqrt(2) degree tilts has variance 26 deg².
    let half_width = 1.281_551_565_544_600_4 * libm::sqrt(26.0);
    near(nominal, 7.0, 1e-12);
    near(band.p10, 7.0 - half_width, 1e-12);
    near(band.p50, 7.0, 1e-12);
    near(band.p90, 7.0 + half_width, 1e-12);
    let upper = measured(answer.vertebrae[0].coronal_tilt).1;
    let lower = measured(answer.vertebrae[1].coronal_tilt).1;
    assert!((band.p10 - (lower.p10 - upper.p10)).abs() > 0.1);
    assert!((band.p90 - (lower.p90 - upper.p90)).abs() > 0.1);
    for (seed, versions) in [(0, 1), (17, 8), (u32::MAX, 64)] {
        input.seed = seed;
        input.versions = versions;
        let changed = measure_spine(&input).unwrap();
        assert_eq!(answer.between.front, changed.between.front);
        for (before, after) in answer.vertebrae.iter().zip(changed.vertebrae) {
            assert_eq!(before.coronal_tilt, after.coronal_tilt);
        }
        for (before, after) in answer.curves.iter().zip(changed.curves) {
            assert_eq!(before.cobb, after.cobb);
        }
    }
}

fn drawn_plate_bands(seeds: u64, tolerance_deg: f64) {
    use forward_model::endplates::Endplates;
    use forward_model::vertebra_params::VertebraParams;
    use spine_readings::curves::geometry::sagittal_tilt_deg;
    use spine_readings::curves::geometry::{lower_coronal_tilt_deg, upper_coronal_tilt_deg};

    let mut input = input(vec![
        vertebra(Level::T6, -10.0, 12.0, 0.0, 100.0),
        vertebra(Level::T10, 20.0, -8.0, 0.0, 70.0),
    ]);
    input.fit.vertebrae[0].sd.angles_rad = [2.0_f64.to_radians(), 1.5_f64.to_radians(), 0.0];
    input.fit.vertebrae[1].sd.angles_rad = [3.0_f64.to_radians(), 2.5_f64.to_radians(), 0.0];
    let written = measure_spine(&input).unwrap();
    let mut front = Vec::new();
    let mut side = Vec::new();
    for seed in 0..seeds {
        for version in versions(&input.fit, seed, 32, 0..32).unwrap().versions {
            let v = &version.fit.vertebrae;
            front.push(lower_coronal_tilt_deg(&v[1]) - upper_coronal_tilt_deg(&v[0]));
            let upper = Endplates::of(&VertebraParams::from(&v[0].pose));
            let lower = Endplates::of(&VertebraParams::from(&v[1].pose));
            side.push(
                sagittal_tilt_deg(&upper.top.normal) - sagittal_tilt_deg(&lower.bottom.normal),
            );
        }
    }
    for (reading, drawn, sd_deg) in [
        (written.between.front[0].angle, front, libm::sqrt(26.0)),
        (written.between.side[0].angle, side, libm::sqrt(17.0)),
    ] {
        let (value, closed) = measured(reading);
        let expected_half = 1.281_551_565_544_600_4 * sd_deg;
        near(closed.p10, value - expected_half, 1e-10);
        near(closed.p50, value, 1e-12);
        near(closed.p90, value + expected_half, 1e-10);
        let drawn = percentiles(drawn);
        for (got, expected) in [
            (drawn.p10, closed.p10),
            (drawn.p50, closed.p50),
            (drawn.p90, closed.p90),
        ] {
            near(got, expected, tolerance_deg);
        }
    }
}

#[test]
fn a_small_drawn_set_stays_near_the_closed_form_plate_bands() {
    drawn_plate_bands(8, 1.0);
}

#[test]
#[ignore = "16384 drawn geometries; run in release mode"]
fn sixteen_thousand_drawn_versions_agree_with_the_closed_form_front_and_side_bands() {
    drawn_plate_bands(512, 0.25);
}

#[test]
fn choosing_extreme_ends_again_in_each_version_inflates_the_angle() {
    // Two equally tilted vertebrae at each end: fixed ends retain centred
    // noise, whereas taking a min and max of noisy ties shifts them outward.
    let tilts = [-20.0, -20.0, -18.0, -8.0, 0.0, 8.0, 18.0, 20.0, 20.0];
    let levels = [
        Level::T4,
        Level::T5,
        Level::T6,
        Level::T7,
        Level::T8,
        Level::T9,
        Level::T10,
        Level::T11,
        Level::T12,
    ];
    let mut input = input(
        levels
            .into_iter()
            .zip(tilts)
            .enumerate()
            .map(|(i, (l, t))| {
                vertebra(
                    l,
                    t,
                    0.0,
                    if i == 4 { -12.0 } else { 0.0 },
                    250.0 - i as f64 * 25.0,
                )
            })
            .collect(),
    );
    input.versions = 256;
    input.seed = 1;
    for v in &mut input.fit.vertebrae {
        v.sd.angles_rad[0] = 1.6_f64.to_radians();
    }
    // The closed-form band stays centred on the fixed fitted plate pair.
    let answer = measure_spine(&input).unwrap();
    let (nominal, fixed) = measured(answer.curves[0].cobb);
    let drawn = versions(
        &input.fit,
        u64::from(input.seed),
        input.versions,
        0..input.versions,
    )
    .unwrap();
    let fresh = percentiles(
        drawn
            .versions
            .iter()
            .map(|v| {
                use spine_readings::curves::geometry::{
                    lower_coronal_tilt_deg, upper_coronal_tilt_deg,
                };
                let bodies = &v.fit.vertebrae;
                let min = bodies[..3]
                    .iter()
                    .map(upper_coronal_tilt_deg)
                    .fold(f64::INFINITY, f64::min);
                let max = bodies[6..]
                    .iter()
                    .map(lower_coronal_tilt_deg)
                    .fold(f64::NEG_INFINITY, f64::max);
                max - min
            })
            .collect(),
    );
    near(nominal, 40.0, 1e-10);
    near(fixed.p50, nominal, 0.1);
    assert!(
        fresh.p50 - fixed.p50 > 1.0,
        "fixed {}, fresh {}",
        fixed.p50,
        fresh.p50
    );
    eprintln!(
        "fixed-end median {:.6}; fresh-end median {:.6}; inflation {:.6} degrees",
        fixed.p50,
        fresh.p50,
        fresh.p50 - fixed.p50
    );
}

#[test]
fn reflection_changes_front_signs_and_renaming_changes_no_pair_value() {
    let input = hand_spine();
    let before = measure_spine(&input).unwrap();
    let mut mirror = input.clone();
    for v in &mut mirror.fit.vertebrae {
        v.pose.centre[1] = -v.pose.centre[1];
        v.pose.wedge_front = -v.pose.wedge_front;
        v.corner_tilt_rad = v.corner_tilt_rad.map(|x| -x);
    }
    let reflected = measure_spine(&mirror).unwrap();
    for (a, b) in before.vertebrae.iter().zip(&reflected.vertebrae) {
        near(
            measured(a.coronal_tilt).0,
            -measured(b.coronal_tilt).0,
            1e-10,
        );
        assert_eq!(a.forward_lean, b.forward_lean);
        assert_eq!(a.twist, b.twist);
    }
    for (a, b) in before.between.front.iter().zip(&reflected.between.front) {
        near(measured(a.angle).0, -measured(b.angle).0, 1e-10);
    }
    assert_eq!(before.between.side, reflected.between.side);
    assert_eq!(before.sagittal, reflected.sagittal);
    for (a, b) in before.curves.iter().zip(&reflected.curves) {
        assert_ne!(a.convex, b.convex);
        near(measured(a.cobb).0, -measured(b.cobb).0, 1e-10);
    }
    let mut renamed = input.clone();
    for v in &mut renamed.fit.vertebrae {
        let next = Level::ALL[Level::ALL.iter().position(|l| *l == v.level).unwrap() + 1];
        v.level = next;
        v.pose.level = next;
    }
    let shifted = measure_spine(&renamed).unwrap();
    for (a, b) in before
        .between
        .front
        .iter()
        .chain(&before.between.side)
        .zip(shifted.between.front.iter().chain(&shifted.between.side))
    {
        assert_eq!(a.angle, b.angle);
    }
    for (a, b) in before.vertebrae.iter().zip(&shifted.vertebrae) {
        assert_eq!(a.coronal_tilt, b.coronal_tilt);
        assert_eq!(a.forward_lean, b.forward_lean);
        assert_eq!(a.twist, b.twist);
    }
}

#[test]
fn the_same_fit_is_written_as_the_same_bytes() {
    let mut input = hand_spine();
    for v in &mut input.fit.vertebrae {
        v.sd.angles_rad = [0.01, 0.02, 0.03];
    }
    let a = serde_json::to_vec(&measure_spine(&input).unwrap()).unwrap();
    let b = serde_json::to_vec(&measure_spine(&input).unwrap()).unwrap();
    assert_eq!(a, b);
}

#[test]
fn an_unseen_l1_needs_side_coverage_before_a_sacrum_reading() {
    let mut input = hand_spine();
    input
        .fit
        .vertebrae
        .iter_mut()
        .find(|v| v.level == Level::L1)
        .unwrap()
        .inferred
        .depth = true;
    let Reading::NotMeasured {
        reason,
        typical_deg: Some(value),
    } = measure_spine(&input).unwrap().sagittal.lordosis_l1_s1
    else {
        panic!("L1 is typical even with a marked sacrum");
    };
    assert_eq!(reason, NotMeasured::NotOnSideFilm);
    near(value, 50.0, 1e-10);
    input.side.as_mut().unwrap().marks.landmarks.sacrum_top = None;
    assert_eq!(
        measure_spine(&input).unwrap().sagittal.lordosis_l1_s1,
        Reading::NotMeasured {
            reason: NotMeasured::NotOnSideFilm,
            typical_deg: None
        },
    );
}
