//! Exact independent projections test intake scales without painting films.

#[expect(dead_code, reason = "the other fit tests use the remaining oracles")]
mod answer_keys;

use answer_keys::{angles, distance, input, sizes, upright_input};
use forward_model::cameras::parallel::ParallelCamera;
use forward_model::project::{front_marks, side_marks};
use forward_model::vertebra_params::VertebraParams;
use joint_fit::{FitError, fit_from_films, fit_spine};
use nalgebra::Vector2;
use shapes::film_geometry::{ImageRight, QuarterTurns};
use shapes::levels::Level;
use shapes::made_up::{AnswerKey, RecipeName};
use shapes::pixels::Px;
use shapes::spine_fit::{
    FilmToFit, FitFromFilms, FitInput, FitLengths, MarkSpread, SpineFit, TypicalSource,
};
use shapes::vertebra::VertebraMarks;

fn request(input: FitInput, typical: TypicalSource) -> FitFromFilms {
    let spread = MarkSpread::ShareOfHeight {
        share: 0.01,
        shared_share: 0.0,
    };
    FitFromFilms {
        front: FilmToFit {
            marks: input.front,
            geometry: input.front_geometry,
            size: input.front_size,
            mark_spread: spread,
            facing_assumed: true,
        },
        side: Some(FilmToFit {
            marks: input.side,
            geometry: input.side_geometry,
            size: input.side_size,
            mark_spread: spread,
            facing_assumed: false,
        }),
        typical,
        twist_readings: Vec::new(),
    }
}

/// The key supplies independent truth only here. Forward-model cameras
/// project the same poses through either facing and stored pixel layout.
fn projected(
    name: RecipeName,
    aspect: bool,
    turn: bool,
    reversed: bool,
) -> (FitFromFilms, AnswerKey) {
    let (mut pair, key) = input(name, 3, 0.0);
    for side in [false, true] {
        let (marks, geometry, size) = if side {
            (&mut pair.side, &mut pair.side_geometry, pair.side_size)
        } else {
            (&mut pair.front, &mut pair.front_geometry, pair.front_size)
        };
        let facing = match (side, reversed) {
            (false, false) => ImageRight::PatientLeft,
            (false, true) => ImageRight::PatientRight,
            (true, false) => ImageRight::Anterior,
            (true, true) => ImageRight::Posterior,
        };
        geometry.image_right = Some(facing);
        geometry.mm_per_px = Some(0.3);
        geometry.mm_per_px_vertical = aspect.then_some(0.6);
        geometry.quarter_turns = QuarterTurns::new(u8::from(turn && !side)).expect("zero or one");
        marks.named_for = facing;
        let camera =
            ParallelCamera::for_film(geometry, size, Vector2::new(600.0, 2200.0)).expect("camera");
        for (v, truth) in marks.vertebrae.iter_mut().zip(&key.poses) {
            let body = VertebraParams::from(truth);
            v.marks = if side {
                VertebraMarks::Side {
                    points: side_marks(&body, &camera).expect("Side"),
                }
            } else {
                VertebraMarks::Front {
                    points: front_marks(&body, &camera).expect("Front"),
                }
            };
        }
    }
    (request(pair, TypicalSource::BuiltIn), key)
}

#[test]
fn exact_pairs_use_side_with_all_four_spacing_combinations() {
    let mut failures = Vec::new();
    let mut attempted = 0;
    for name in [
        RecipeName::Straight,
        RecipeName::RightThoracic40,
        RecipeName::LumbarLeft30,
    ] {
        for aspect in [false, true] {
            for turn in [false, true] {
                for reversed in [false, true] {
                    let (scaled, key) = projected(name, aspect, turn, reversed);
                    let reference = fit_from_films(&scaled).expect("scaled pair");
                    assert!(reference.converged, "{name:?} {aspect} {turn} {reversed}");
                    // Existing exact-pair limits: 0.05 mm centres, 2 degrees and
                    // 2.5% sizes against independently generated true poses.
                    for (v, truth) in reference.vertebrae.iter().zip(&key.poses) {
                        assert!(
                            distance(v.pose.centre, truth.centre) < 0.05,
                            "centre {:?}",
                            v.level
                        );
                        for (a, b) in angles(&v.pose).into_iter().zip(angles(truth)) {
                            assert!((a - b).abs().to_degrees() < 2.0, "angle {:?}", v.level);
                        }
                        for (a, b) in sizes(&v.pose).into_iter().zip(sizes(truth)) {
                            assert!((a / b - 1.0).abs() < 0.025, "size {:?}", v.level);
                        }
                    }
                    for front_scale in [true, false] {
                        for side_scale in [true, false] {
                            // Removing non-square spacing changes the pixel
                            // convention. Its separate test below checks that
                            // square convention rather than scale invariance.
                            if aspect && (!front_scale || !side_scale) {
                                continue;
                            }
                            let mut ordinary = scaled.clone();
                            if !front_scale {
                                ordinary.front.geometry.mm_per_px = None;
                                ordinary.front.geometry.mm_per_px_vertical = None;
                            }
                            let side = ordinary.side.as_mut().expect("Side");
                            if !side_scale {
                                side.geometry.mm_per_px = None;
                                side.geometry.mm_per_px_vertical = None;
                            }
                            attempted += 1;
                            let fit = match fit_from_films(&ordinary) {
                                Ok(fit) => fit,
                                Err(error) => {
                                    failures.push(format!("{name:?} aspect={aspect} turn={turn} reversed={reversed} FrontScale={front_scale} SideScale={side_scale}: {error}"));
                                    continue;
                                }
                            };
                            let lengths = if front_scale || side_scale {
                                FitLengths::Measured
                            } else {
                                FitLengths::TypicalSize
                            };
                            if !fit.converged || fit.side_film.is_none() || fit.lengths != lengths {
                                failures.push(format!("{name:?} aspect={aspect} turn={turn} reversed={reversed} FrontScale={front_scale} SideScale={side_scale}: converged={} SideUsed={} lengths={:?}", fit.converged, fit.side_film.is_some(), fit.lengths));
                                continue;
                            }
                            assert!(fit.front_film.facing_assumed);
                            assert!(!fit.side_film.expect("used Side").facing_assumed);
                            // Only one global length gauge is free. Match it using
                            // Front spacing, rather than assuming a typical size is truth.
                            let gauge =
                                fit.front_film.mm_per_px[0] / reference.front_film.mm_per_px[0];
                            let mut largest = [0.0_f64; 3];
                            for (a, b) in fit.vertebrae.iter().zip(&reference.vertebrae) {
                                for (x, y) in angles(&a.pose).into_iter().zip(angles(&b.pose)) {
                                    largest[0] = largest[0].max((x - y).abs().to_degrees());
                                }
                                for (x, y) in a.pose.centre.into_iter().zip(b.pose.centre) {
                                    largest[1] = largest[1].max((x / gauge - y).abs());
                                }
                                for (x, y) in sizes(&a.pose).into_iter().zip(sizes(&b.pose)) {
                                    largest[2] = largest[2].max((x / gauge / y - 1.0).abs());
                                }
                            }
                            // Keep the specified limits for every attempted
                            // combination; collect failures so all are exercised.
                            if largest[0] >= 0.05 || largest[1] >= 0.05 || largest[2] >= 0.025 {
                                failures.push(format!("{name:?} aspect={aspect} turn={turn} reversed={reversed} FrontScale={front_scale} SideScale={side_scale}: angle deg, centre mm, size share {largest:?}"));
                            }
                        }
                    }
                }
            }
        }
    }
    eprintln!(
        "ordinary calibration matrix: {attempted} combinations; {} failed",
        failures.len()
    );
    assert_eq!(attempted, 60);
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn films_without_spacing_are_square_and_non_square_pixels_need_spacing() {
    let (calibrated, key) = projected(RecipeName::Straight, true, false, false);
    let known = fit_from_films(&calibrated).expect("non-square films with spacing");
    assert!(known.converged && known.side_film.is_some());
    // The independent forward projections have 0.3 by 0.6 mm pixels.
    // Keeping both spacings recovers the existing exact-pair limits.
    for (fitted, truth) in known.vertebrae.iter().zip(&key.poses) {
        assert!(distance(fitted.pose.centre, truth.centre) < 0.05);
        for (actual, expected) in sizes(&fitted.pose).into_iter().zip(sizes(truth)) {
            assert!((actual / expected - 1.0).abs() < 0.025);
        }
    }
    let mut without = calibrated.clone();
    without.front.geometry.mm_per_px = None;
    without.front.geometry.mm_per_px_vertical = None;
    let side = without.side.as_mut().expect("Side");
    side.geometry.mm_per_px = None;
    side.geometry.mm_per_px_vertical = None;
    let square = fit_from_films(&without).expect("films interpreted as square");
    assert!(square.converged);
    assert_eq!(square.lengths, FitLengths::TypicalSize);
    assert_eq!(
        square.front_film.mm_per_px[0],
        square.front_film.mm_per_px[1]
    );
    let side = square
        .side_film
        .expect("the square-convention pair is used");
    assert_eq!(side.mm_per_px[0], side.mm_per_px[1]);
    // Erasing aspect is not a change of one global length gauge. The
    // drawn height differs after matching row spacing, so non-square
    // films need their spacing to recover the original bodies.
    let gauge = square.front_film.mm_per_px[0] / known.front_film.mm_per_px[0];
    assert!(
        square
            .vertebrae
            .iter()
            .zip(&known.vertebrae)
            .any(|(a, b)| (a.pose.height / gauge / b.pose.height - 1.0).abs() > 0.025)
    );
    assert!(without.front.geometry.mm_per_px.is_none());
    assert!(without.side.expect("Side").geometry.mm_per_px.is_none());
}

fn twice_side_density(pair: &mut FitInput) {
    for v in &mut pair.side.vertebrae {
        let VertebraMarks::Side { points } = &mut v.marks else {
            unreachable!("Side")
        };
        for m in [
            &mut points.sup_a,
            &mut points.sup_p,
            &mut points.inf_a,
            &mut points.inf_p,
        ] {
            m.at = Px::new(2.0 * m.at.x(), 2.0 * m.at.y());
        }
    }
    pair.side_size = pair.side_size.map(|s| 2 * s);
    pair.side_geometry.mm_per_px = Some(0.25);
}

#[test]
fn shared_vertical_marks_determine_an_unscaled_sides_density() {
    let mut pair = upright_input();
    twice_side_density(&mut pair);
    let truth = fit_spine(&pair).expect("analytic pair");
    let mut ordinary = request(pair, TypicalSource::None);
    ordinary.side.as_mut().expect("Side").geometry.mm_per_px = None;
    let fit = fit_from_films(&ordinary).expect("unscaled Side");
    assert!(fit.converged && fit.side_film.is_some());
    // Analytic 0.5 mm Front pixels and twice-density Side imply exactly
    // 0.25 mm Side pixels and ratio 0.5. 1e-6 is a numerical-solver check.
    assert!((fit.side_scale - 0.5).abs() < 1e-6, "{}", fit.side_scale);
    for spacing in fit.side_film.expect("used Side").mm_per_px {
        assert!((spacing - 0.25).abs() < 1e-6);
    }
    assert_same_bodies(&fit, &truth, 0.05);
    let mut reverse = ordinary.clone();
    reverse.typical = TypicalSource::BuiltIn;
    reverse.front.geometry.mm_per_px = None;
    reverse.side.as_mut().expect("Side").geometry.mm_per_px = Some(0.25);
    let fit = fit_from_films(&reverse).expect("Side length anchor");
    assert!(fit.converged);
    assert_eq!(fit.lengths, FitLengths::Measured);
    assert!((fit.side_scale - 1.0).abs() < 1e-12);
    for spacing in fit.side_film.expect("Side").mm_per_px {
        assert!((spacing - 0.25).abs() < 1e-6);
    }
    // Same analytic dimensions in the Side's measured millimetres, not
    // Front's provisional typical gauge. Existing 0.05 mm exact limit.
    assert_same_bodies(&fit, &truth, 0.05);
}

fn assert_same_bodies(a: &SpineFit, b: &SpineFit, tolerance: f64) {
    for (a, b) in a.vertebrae.iter().zip(&b.vertebrae) {
        assert_eq!(a.level, b.level);
        for (x, y) in a
            .pose
            .centre
            .into_iter()
            .zip(b.pose.centre)
            .chain(sizes(&a.pose).into_iter().zip(sizes(&b.pose)))
        {
            assert!((x - y).abs() < tolerance, "{:?}: {x} versus {y}", a.level);
        }
    }
}

#[test]
fn unused_side_is_exactly_front_only_and_cannot_supply_measured_lengths() {
    let (mut pair, _) = projected(RecipeName::Straight, false, false, false);
    pair.front.geometry.mm_per_px = None;
    let mut alone = pair.clone();
    alone.side = None;
    let front = fit_from_films(&alone).expect("Front only");
    assert_eq!(front.lengths, FitLengths::TypicalSize);
    for unusable in [0, 1, 2, 3, 4] {
        let mut candidate = pair.clone();
        let side = candidate.side.as_mut().expect("Side");
        // A level absent from Front must never leak out of the retry.
        side.marks.vertebrae[0].level = Level::C7;
        match unusable {
            0 => side.marks.vertebrae.clear(),
            1 => side.marks.vertebrae.truncate(1),
            2 => side.geometry.image_right = None,
            3 => side.size = [0, 0],
            _ => side.geometry.mm_per_px = Some(-1.0),
        }
        let fallback = fit_from_films(&candidate).expect("Front retry");
        // Deterministic reconstruction of the same Front problem is
        // bit-identical, stronger than the required 1e-9 numerical limit.
        assert_eq!(fallback, front, "unusable Side {unusable}");
        assert!(fallback.side_film.is_none());
        assert_eq!((fallback.side_scale, fallback.side_offset_mm), (1.0, 0.0));
        assert!(
            fallback
                .vertebrae
                .iter()
                .all(|v| v.rms_mm.side.is_none() && v.level != Level::C7)
        );
    }
    alone.front.marks.vertebrae.truncate(1);
    assert_eq!(fit_from_films(&alone), Err(FitError::TooFewVertebrae));
    pair.typical = TypicalSource::None;
    pair.side.as_mut().expect("Side").marks.vertebrae.clear();
    assert_eq!(
        fit_from_films(&pair),
        Err(FitError::MissingScale(shapes::film::FilmKind::Front))
    );
}

#[test]
fn facing_echoes_are_independent_and_legacy_placements_are_false() {
    let input = upright_input();
    let legacy = fit_spine(&input).expect("legacy pair");
    assert!(!legacy.front_film.facing_assumed && !legacy.side_film.expect("Side").facing_assumed);
    for front in [false, true] {
        for side in [false, true] {
            let mut pair = request(input.clone(), TypicalSource::None);
            pair.front.facing_assumed = front;
            pair.side.as_mut().expect("Side").facing_assumed = side;
            let fit = fit_from_films(&pair).expect("pair");
            assert_eq!(fit.front_film.facing_assumed, front);
            assert_eq!(fit.side_film.expect("used Side").facing_assumed, side);
        }
    }
}
