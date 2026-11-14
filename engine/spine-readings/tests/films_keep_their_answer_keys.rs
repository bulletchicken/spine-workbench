//! Saved made-up angles are independent of the fit and of the reader.

use joint_fit::fit_from_films;
use made_up_spines::answer_key::{answer_key, spine_for};
use made_up_spines::film_layout::FilmLayout;
use made_up_spines::patient::make_up_patient;
use made_up_spines::recipes::film_recipes;
use shapes::confidence::{Confidence, ConfidenceBand};
use shapes::landmarks::Landmarks;
use shapes::levels::Level;
use shapes::made_up::{FilmAnswer, MadeUpRecipe, RecipeName, Tier, TrueMark, TrueMarks};
use shapes::marks::{FrontMarks, Mark, MarkSource, PedicleMark, SideMarks};
use shapes::naming::{ChosenBy, LevelNaming, Template};
use shapes::spine_fit::{FilmToFit, FitFromFilms, MarkSpread, TypicalSource};
use shapes::spine_readings::{Reading, ReadingsInput};
use shapes::vertebra::{FilmMarks, VertebraMarks, VertebraOnFilm};
use spine_readings::measure_spine;

fn mark(value: TrueMark) -> Mark {
    Mark {
        at: value.at,
        source: MarkSource::Hand,
        pinned: false,
        inferred: !value.visible,
    }
}

fn pedicle(value: TrueMark) -> PedicleMark {
    if value.visible {
        PedicleMark::Visible {
            at: value.at,
            source: MarkSource::Hand,
            pinned: false,
            inferred: false,
            ring_score: None,
        }
    } else {
        PedicleMark::NotVisible {
            source: MarkSource::Hand,
        }
    }
}

fn film(answer: &FilmAnswer, template: Template, size: [u32; 2]) -> FilmToFit {
    let vertebrae: Vec<_> = answer
        .vertebrae
        .iter()
        .filter(|v| v.level >= Level::T1)
        .enumerate()
        .map(|(i, v)| VertebraOnFilm {
            id: i as u32 + 1,
            level: v.level,
            marks: match v.marks {
                TrueMarks::Front {
                    sup_l,
                    sup_r,
                    inf_l,
                    inf_r,
                    ped_l,
                    ped_r,
                } => VertebraMarks::Front {
                    points: FrontMarks {
                        sup_l: mark(sup_l),
                        sup_r: mark(sup_r),
                        inf_l: mark(inf_l),
                        inf_r: mark(inf_r),
                        ped_l: pedicle(ped_l),
                        ped_r: pedicle(ped_r),
                    },
                },
                TrueMarks::Side {
                    sup_a,
                    sup_p,
                    inf_a,
                    inf_p,
                } => VertebraMarks::Side {
                    points: SideMarks {
                        sup_a: mark(sup_a),
                        sup_p: mark(sup_p),
                        inf_a: mark(inf_a),
                        inf_p: mark(inf_p),
                    },
                },
                TrueMarks::Bending { .. } => unreachable!("standing views"),
            },
            confidence: Confidence {
                score: 1.0,
                band: ConfidenceBand::High,
                reasons: Vec::new(),
                calibrated: false,
                endplate_spread_px: None,
                margin: None,
            },
            outline: None,
        })
        .collect();
    let levels = vertebrae.iter().map(|v| v.level).collect::<Vec<_>>();
    FilmToFit {
        marks: FilmMarks {
            film: answer.kind,
            vertebrae,
            landmarks: Landmarks::NONE,
            naming: LevelNaming {
                template,
                chosen_by: ChosenBy::User,
                alternatives: Vec::new(),
                from_top: Some(levels.clone()),
                from_bottom: Some(levels),
                flags: Vec::new(),
                l5_confirmed_on: None,
            },
            named_for: answer.geometry.image_right.unwrap(),
        },
        geometry: answer.geometry.clone(),
        size,
        mark_spread: MarkSpread::Millimetres {
            sd_mm: 1.0,
            shared_sd_mm: 0.0,
        },
        facing_assumed: false,
    }
}

fn value(reading: Reading) -> f64 {
    match reading {
        Reading::Measured { value_deg, .. } => value_deg,
        v => panic!("measurement expected: {v:?}"),
    }
}

fn check(name: RecipeName) {
    let recipe = MadeUpRecipe {
        name,
        seed: 3,
        tier: Tier::T0,
    };
    let patient = make_up_patient(name, recipe.seed).unwrap();
    let layouts: Vec<_> = film_recipes(&recipe, u64::from(recipe.seed))
        .iter()
        .map(|r| FilmLayout::new(r, spine_for(&patient, r.kind)))
        .collect();
    let key = answer_key(&recipe, &patient, &layouts);
    let front = film(&key.films[0], key.template, layouts[0].size);
    let side = film(&key.films[1], key.template, layouts[1].size);
    let fit_input = FitFromFilms {
        front: front.clone(),
        side: Some(side.clone()),
        typical: TypicalSource::BuiltIn,
        twist_readings: Vec::new(),
    };
    let fit = fit_from_films(&fit_input).unwrap();
    let mut front_only = fit_input;
    front_only.side = None;
    let one_fit = fit_from_films(&front_only).unwrap();
    let readings = |fit, side| {
        measure_spine(&ReadingsInput {
            fit,
            front: Some(front.clone()),
            side,
            bend_left: None,
            bend_right: None,
            versions: 4,
            seed: 3,
        })
        .unwrap()
    };
    let two = readings(fit, Some(side));
    let one = readings(one_fit, None);
    // The fit's independent answer-key suite allows 2° at each endplate;
    // a difference of two such measurements allows 4°.
    for truth in &key.angles.frontal_cobb {
        let find = |r: &shapes::spine_readings::SpineReadings| {
            r.curves
                .iter()
                .filter(|c| c.region == truth.region)
                .max_by(|a, b| value(a.cobb).abs().total_cmp(&value(b.cobb).abs()))
                .unwrap()
                .clone()
        };
        let a = find(&two);
        let b = find(&one);
        for curve in [&a, &b] {
            assert!(
                (value(curve.cobb).abs() - truth.degrees).abs() <= 4.0,
                "{name:?}: {curve:?}, truth {truth:?}"
            );
        }
        assert!(
            (value(a.cobb) - value(b.cobb)).abs() <= 4.0,
            "{name:?}: two {} one {}",
            value(a.cobb),
            value(b.cobb)
        );
        // Locate the extrema independently on the saved front film's
        // endplate lines, within one level of the recipe endpoints. The
        // contract's coronal angles are film readings; a 3D normal can
        // project differently when a body leans and twists.
        let source = patient
            .recipe
            .curves
            .iter()
            .find(|c| c.region == truth.region)
            .unwrap();
        let t1 = patient
            .standing
            .levels
            .iter()
            .position(|l| *l == Level::T1)
            .unwrap() as i32;
        let start = (t1 + libm::floor(source.upper_end) as i32 - 1).max(t1) as usize;
        let end = (t1 + libm::ceil(source.lower_end) as i32 + 1)
            .min(patient.standing.levels.len() as i32 - 1) as usize;
        let seen_tilt = |index: usize, top: bool| {
            let level = patient.standing.levels[index];
            let v = key.films[0]
                .vertebrae
                .iter()
                .find(|v| v.level == level)
                .unwrap();
            let TrueMarks::Front {
                sup_l,
                sup_r,
                inf_l,
                inf_r,
                ..
            } = v.marks
            else {
                unreachable!()
            };
            let (left, right) = if top {
                (sup_l.at, sup_r.at)
            } else {
                (inf_l.at, inf_r.at)
            };
            let scale_x = key.films[0].geometry.mm_per_px.unwrap();
            let scale_y = key.films[0].geometry.mm_per_px_vertical.unwrap_or(scale_x);
            assert_eq!(key.films[0].geometry.quarter_turns.get(), 0);
            // Patient-left is across the picture; pixel y increases down.
            libm::atan2(
                (right.y() - left.y()) * scale_y,
                (left.x() - right.x()) * scale_x,
            )
            .to_degrees()
        };
        let mut expected = (start, start, 0.0_f64);
        for u in start..=end {
            for l in u..=end {
                let angle = (seen_tilt(l, false) - seen_tilt(u, true)).abs();
                if angle > expected.2 {
                    expected = (u, l, angle);
                }
            }
        }
        for (view, curve) in [("two-films", &a), ("front-only", &b)] {
            for (chosen, expected_index, top) in [
                (curve.upper_end, expected.0, true),
                (curve.lower_end, expected.1, false),
            ] {
                let chosen_index = patient
                    .standing
                    .levels
                    .iter()
                    .position(|l| *l == chosen)
                    .unwrap();
                let tilt = |index| seen_tilt(index, top);
                // At a shallow extremum adjacent levels can exchange rank
                // under an error much smaller than the fit's 2° allowance.
                // Pin each selected end to that independently known tilt
                // allowance, rather than demanding an unstable level label.
                assert!(
                    chosen_index.abs_diff(expected_index) <= 1,
                    "{name:?}: chosen {chosen:?}, expected {:?}",
                    patient.standing.levels[expected_index]
                );
                assert!(
                    (tilt(chosen_index) - tilt(expected_index)).abs() <= 2.0,
                    "{name:?} {view}: chosen {chosen:?} tilt {}, expected {:?} tilt {}",
                    tilt(chosen_index),
                    patient.standing.levels[expected_index],
                    tilt(expected_index)
                );
            }
        }
    }
    // Corner tilt is a reading of the same front marks in both fits; its
    // agreement is therefore tighter than the fit's 2-degree allowance.
    for (a, b) in two.vertebrae.iter().zip(one.vertebrae) {
        assert_eq!(a.level, b.level);
        assert!((value(a.coronal_tilt) - value(b.coronal_tilt)).abs() < 1e-10);
    }
}

#[test]
fn a_right_thoracic_curve_keeps_its_answer_through_one_film_or_two() {
    check(RecipeName::RightThoracic40);
}

#[test]
fn a_left_lumbar_curve_keeps_its_answer_through_one_film_or_two() {
    check(RecipeName::LumbarLeft30);
}

#[test]
fn both_major_curves_keep_their_answers_through_one_film_or_two() {
    check(RecipeName::DoubleMajor55To45);
}

mod pair_truth;

#[test]
fn honest_seeded_mark_noise_keeps_full_and_lumbar_pairs_and_their_answers() {
    check_noisy_pairs(false, &[11, 29, 73]);
}

#[test]
fn trained_front_noise_keeps_corresponding_curves_and_sane_pairs_or_exact_fallback() {
    check_noisy_pairs(true, &[11, 29, 41, 53, 67, 73, 89, 97, 101, 113]);
}

fn check_noisy_pairs(trained_front: bool, seeds: &[u64]) {
    use made_up_spines::seeded::Stream;
    use shapes::pixels::Px;
    let recipe = MadeUpRecipe {
        name: RecipeName::RightThoracic40,
        seed: 4,
        tier: Tier::T0,
    };
    let patient = make_up_patient(recipe.name, recipe.seed).unwrap();
    let layouts: Vec<_> = film_recipes(&recipe, u64::from(recipe.seed))
        .iter()
        .map(|r| FilmLayout::new(r, spine_for(&patient, r.kind)))
        .collect();
    let key = answer_key(&recipe, &patient, &layouts);
    let mut exact = FitFromFilms {
        front: film(&key.films[0], key.template, layouts[0].size),
        side: Some(film(&key.films[1], key.template, layouts[1].size)),
        typical: TypicalSource::BuiltIn,
        twist_readings: Vec::new(),
    };
    for view in [&mut exact.front, exact.side.as_mut().unwrap()] {
        view.mark_spread = MarkSpread::Millimetres {
            sd_mm: 1.0,
            shared_sd_mm: 0.0,
        };
    }
    let read = |request: &FitFromFilms| {
        let fit = fit_from_films(request).unwrap();
        assert!(
            fit.converged && (trained_front || request.side.is_none() || fit.side_film.is_some()),
            "the same-spine pair must be used"
        );
        let readings = measure_spine(&ReadingsInput {
            fit: fit.clone(),
            front: Some(request.front.clone()),
            side: request.side.clone(),
            bend_left: None,
            bend_right: None,
            versions: if trained_front { 32 } else { 4 },
            seed: if trained_front { 7 } else { recipe.seed },
        })
        .unwrap();
        (fit, readings)
    };
    let (_, truth) = read(&exact);
    let mut used_count = [0; 2];
    let mut inside = [[0; 4]; 2];
    for &seed in seeds {
        for lumbar_only in [false, true] {
            let mut request = exact.clone();
            if lumbar_only {
                request
                    .side
                    .as_mut()
                    .unwrap()
                    .marks
                    .vertebrae
                    .retain(|v| v.level >= Level::L1);
            }
            let mut stream = Stream::new(
                if trained_front {
                    "trained front shared noise"
                } else {
                    "stated mark noise"
                },
                seed,
            );
            for view in [&mut request.front, request.side.as_mut().unwrap()] {
                let spacing = view.geometry.mm_per_px.unwrap();
                let trained_view =
                    trained_front && view.marks.film == shapes::film::FilmKind::Front;
                if trained_view {
                    view.mark_spread = MarkSpread::ShareOfHeight {
                        share: 0.11,
                        shared_share: 0.08,
                    };
                    view.marks.naming.from_top = None;
                    view.marks.naming.from_bottom = None;
                    view.marks.naming.flags = vec![shapes::naming::NamingFlag::Estimated];
                }
                for v in &mut view.marks.vertebrae {
                    let corners = v.marks.corners();
                    let top = (
                        (corners[0].at.x() + corners[1].at.x()) / 2.0,
                        (corners[0].at.y() + corners[1].at.y()) / 2.0,
                    );
                    let bottom = (
                        (corners[2].at.x() + corners[3].at.x()) / 2.0,
                        (corners[2].at.y() + corners[3].at.y()) / 2.0,
                    );
                    let height = libm::hypot(top.0 - bottom.0, top.1 - bottom.1) * spacing;
                    let own_sd = if trained_view { 0.11 * height } else { 1.0 };
                    let shared_sd = if trained_view { 0.08 * height } else { 0.0 };
                    let shared = if shared_sd > 0.0 {
                        [
                            stream.normal(0.0, shared_sd) / spacing,
                            stream.normal(0.0, shared_sd) / spacing,
                        ]
                    } else {
                        [0.0, 0.0]
                    };
                    let mut move_mark = |mark: &mut Mark, multiplier: f64| {
                        let sd =
                            own_sd * multiplier * if mark.inferred { 3.0 } else { 1.0 } / spacing;
                        mark.at = Px::new(
                            mark.at.x() + stream.normal(0.0, sd) + shared[0],
                            mark.at.y() + stream.normal(0.0, sd) + shared[1],
                        );
                    };
                    match &mut v.marks {
                        VertebraMarks::Front { points } => {
                            if trained_view {
                                points.ped_l = PedicleMark::NotVisible {
                                    source: MarkSource::Auto,
                                };
                                points.ped_r = PedicleMark::NotVisible {
                                    source: MarkSource::Auto,
                                };
                            }
                            for mark in [
                                &mut points.sup_l,
                                &mut points.sup_r,
                                &mut points.inf_l,
                                &mut points.inf_r,
                            ] {
                                move_mark(mark, 1.0);
                            }
                            for pedicle in [&mut points.ped_l, &mut points.ped_r] {
                                if let PedicleMark::Visible {
                                    at,
                                    source,
                                    pinned,
                                    inferred,
                                    ..
                                } = pedicle
                                {
                                    let mut mark = Mark {
                                        at: *at,
                                        source: *source,
                                        pinned: *pinned,
                                        inferred: *inferred,
                                    };
                                    move_mark(&mut mark, 2.0);
                                    *at = mark.at;
                                }
                            }
                        }
                        VertebraMarks::Side { points } => {
                            for mark in [
                                &mut points.sup_a,
                                &mut points.sup_p,
                                &mut points.inf_a,
                                &mut points.inf_p,
                            ] {
                                move_mark(mark, 1.0);
                            }
                        }
                        _ => unreachable!("standing films"),
                    }
                }
                view.geometry.mm_per_px = None;
                view.geometry.mm_per_px_vertical = None;
            }
            eprintln!("honest noise trained={trained_front} seed={seed} lumbar_only={lumbar_only}");
            let (fit, answer) = read(&request);
            if trained_front {
                let mut alone = request.clone();
                alone.side = None;
                let (front_fit, front) = read(&alone);
                let used = fit.side_film.is_some();
                if !used {
                    assert_eq!(
                        fit, front_fit,
                        "a refused Side must leave exactly Front alone"
                    );
                }
                let exact = main_cobb(&truth).0;
                let (front_cobb, front_band) = main_cobb(&front);
                let (pair_cobb, pair_band) = main_cobb(&answer);
                let front_error = (front_cobb - exact).abs();
                let pair_error = (pair_cobb - exact).abs();
                eprintln!(
                    "trained band seed={seed} lumbar_only={lumbar_only} used={used} exact={exact} front={front_cobb} front_band={front_band} pair={pair_cobb} pair_band={pair_band} front_error={front_error} pair_error={pair_error} front_inside={} pair_inside={}",
                    front_error <= front_band,
                    pair_error <= pair_band
                );
                // Corner noise alone can move endplate angles by several
                // degrees. The pair must not make that Front reading worse.
                assert!(
                    pair_error <= front_error + 2.0,
                    "pair made Front worse: seed={seed} lumbar_only={lumbar_only} front_error={front_error} pair_error={pair_error}"
                );
                let names = Template::Standard.names();
                let index = |level| names.iter().position(|l| *l == level).unwrap();
                for curve in &answer.curves {
                    if index(curve.lower_end).abs_diff(index(curve.upper_end)) == 1 {
                        // A noisy existing curve can move an end by one
                        // level. Correspondence requires the same region
                        // and bulge, both ends within one level and Cobb
                        // within 5 degrees; an unrelated kink still fails.
                        assert!(
                            front.curves.iter().any(|c| c.region == curve.region
                                && c.convex == curve.convex
                                && index(c.upper_end).abs_diff(index(curve.upper_end)) <= 1
                                && index(c.lower_end).abs_diff(index(curve.lower_end)) <= 1
                                && (pair_truth::value(c.cobb) - pair_truth::value(curve.cobb))
                                    .abs()
                                    <= 5.0),
                            "pair introduced a curve without Front correspondence: seed={seed} lumbar_only={lumbar_only} {curve:?}"
                        );
                    }
                }
                let scores = trained_corner_scores(&alone, &front_fit);
                if used {
                    let pair_scores = trained_corner_scores(&request, &fit);
                    eprintln!(
                        "trained scores seed={seed} lumbar_only={lumbar_only} all={} front={} side={} reference={} rise={}",
                        pair_scores[0],
                        pair_scores[1],
                        pair_scores[2],
                        scores[1],
                        pair_scores[1] - scores[1]
                    );
                    assert!(pair_scores[0] <= 1.75 && pair_scores[2] <= 1.75);
                    assert!(pair_scores[1] <= scores[1] + 0.5);
                    guard_against_a_contorted_pair(&answer, &truth, lumbar_only);
                    let coverage = usize::from(lumbar_only);
                    used_count[coverage] += 1;
                    inside[coverage][0] += usize::from(front_error <= front_band);
                    inside[coverage][1] += usize::from(pair_error <= pair_band);
                    for (slot, actual, exact) in [
                        (
                            2,
                            answer.sagittal.kyphosis_t5_t12,
                            truth.sagittal.kyphosis_t5_t12,
                        ),
                        (
                            3,
                            answer.sagittal.lordosis_l1_l5,
                            truth.sagittal.lordosis_l1_l5,
                        ),
                    ] {
                        if let Reading::Measured { value_deg, .. } = actual {
                            let (v, band) = written_band(actual);
                            let exact = pair_truth::value(exact);
                            inside[coverage][slot] += usize::from((v - exact).abs() <= band);
                            eprintln!(
                                "trained side band seed={seed} lumbar_only={lumbar_only} slot={slot} exact={exact} value={value_deg} band={band} error={} inside={}",
                                value_deg - exact,
                                (v - exact).abs() <= band
                            );
                        }
                    }
                }
            } else {
                assert!(fit.side_film.is_some());
                pair_truth::check(&answer, &truth, lumbar_only);
            }
        }
    }
    if trained_front {
        for coverage in 0..2 {
            eprintln!(
                "trained coverage lumbar_only={} used={} front={} pair={} kyphosis={} lordosis={}",
                coverage == 1,
                used_count[coverage],
                inside[coverage][0],
                inside[coverage][1],
                inside[coverage][2],
                inside[coverage][3]
            );
        }
    }
}

fn guard_against_a_contorted_pair(
    answer: &shapes::spine_readings::SpineReadings,
    truth: &shapes::spine_readings::SpineReadings,
    lumbar_only: bool,
) {
    use shapes::spine_readings::NotMeasured;
    let main = |r: &shapes::spine_readings::SpineReadings| {
        r.curves
            .iter()
            .filter(|c| c.region == shapes::made_up::CurveRegion::MainThoracic)
            .max_by(|a, b| {
                pair_truth::value(a.cobb)
                    .abs()
                    .total_cmp(&pair_truth::value(b.cobb).abs())
            })
            .unwrap()
            .clone()
    };
    let names = Template::Standard.names();
    let index = |level| names.iter().position(|l| *l == level).unwrap();
    for (actual, expected) in [
        (main(answer).upper_end, main(truth).upper_end),
        (main(answer).lower_end, main(truth).lower_end),
    ] {
        assert!(index(actual).abs_diff(index(expected)) <= 1);
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
        guard_side_curve(
            answer.sagittal.kyphosis_t5_t12,
            truth.sagittal.kyphosis_t5_t12,
        );
    }
    guard_side_curve(
        answer.sagittal.lordosis_l1_l5,
        truth.sagittal.lordosis_l1_l5,
    );
}

fn guard_side_curve(actual: Reading, exact: Reading) {
    let actual = pair_truth::value(actual);
    let exact = pair_truth::value(exact);
    assert!(
        actual * exact > 0.0,
        "the Side curve has the wrong sign: {actual} against {exact}"
    );
    assert!(
        (actual - exact).abs() <= 15.0,
        "a contorted Side curve: {actual} against {exact}"
    );
}

fn written_band(reading: Reading) -> (f64, f64) {
    let Reading::Measured { value_deg, band } = reading else {
        panic!("measured reading")
    };
    (
        value_deg,
        (value_deg - band.p10)
            .abs()
            .max((band.p90 - value_deg).abs()),
    )
}

// Reproject the reported poses through their placements. These cases have
// four corners per body and no pedicles, so all scores can be checked from
// public values without adding diagnostics to the fit's result.
fn trained_corner_scores(request: &FitFromFilms, fit: &shapes::spine_fit::SpineFit) -> [f64; 3] {
    use forward_model::cameras::parallel::ParallelCamera;
    use forward_model::project::{front_marks, side_marks};
    use forward_model::vertebra_params::VertebraParams;
    use nalgebra::Vector2;
    let mut squared = [0.0; 3];
    let mut count = [0; 2];
    for (view, film, placement) in [
        (0, Some(&request.front), Some(fit.front_film)),
        (1, request.side.as_ref(), fit.side_film),
    ] {
        let (Some(film), Some(placement)) = (film, placement) else {
            continue;
        };
        let geometry = shapes::film_geometry::FilmGeometry {
            mm_per_px: Some(placement.mm_per_px[0]),
            mm_per_px_vertical: Some(placement.mm_per_px[1]),
            ..film.geometry.clone()
        };
        let camera =
            ParallelCamera::for_film(&geometry, film.size, Vector2::from(placement.origin_px))
                .unwrap();
        for v in &film.marks.vertebrae {
            let fitted = fit.vertebrae.iter().find(|p| p.level == v.level).unwrap();
            let pose = VertebraParams::from(&fitted.pose);
            let predicted = if view == 0 {
                front_marks(&pose, &camera).unwrap().corners()
            } else {
                side_marks(&pose, &camera).unwrap().corners()
            };
            let corners = v.marks.corners();
            let midpoint = |a: Mark, b: Mark| {
                Vector2::new(
                    (a.at.x() + b.at.x()) * 0.5 * placement.mm_per_px[0],
                    (a.at.y() + b.at.y()) * 0.5 * placement.mm_per_px[1],
                )
            };
            let height =
                (midpoint(corners[0], corners[1]) - midpoint(corners[2], corners[3])).norm();
            let (sd, shared) = match film.mark_spread {
                MarkSpread::ShareOfHeight {
                    share,
                    shared_share,
                } => (share * height, shared_share * height),
                MarkSpread::Millimetres {
                    sd_mm,
                    shared_sd_mm,
                } => (sd_mm, shared_sd_mm),
            };
            for (observed, predicted) in corners.into_iter().zip(predicted) {
                let sd = sd * if observed.inferred { 3.0 } else { 1.0 };
                let difference = Vector2::new(
                    (predicted.at.x() - observed.at.x()) * placement.mm_per_px[0],
                    (predicted.at.y() - observed.at.y()) * placement.mm_per_px[1],
                )
                .norm_squared();
                squared[view] += difference / (sd * sd);
                if view == 0 {
                    squared[2] += difference / (sd * sd + shared * shared);
                }
                count[view] += 1;
            }
        }
    }
    [
        libm::sqrt((squared[0] + squared[1]) / (count[0] + count[1]) as f64),
        libm::sqrt(squared[2] / count[0] as f64),
        if count[1] == 0 {
            0.0
        } else {
            libm::sqrt(squared[1] / count[1] as f64)
        },
    ]
}

fn main_cobb(readings: &shapes::spine_readings::SpineReadings) -> (f64, f64) {
    let curve = readings
        .curves
        .iter()
        .filter(|c| c.region == shapes::made_up::CurveRegion::MainThoracic)
        .max_by(|a, b| {
            pair_truth::value(a.cobb)
                .abs()
                .total_cmp(&pair_truth::value(b.cobb).abs())
        })
        .expect("main thoracic curve");
    let Reading::Measured { value_deg, band } = curve.cobb else {
        panic!("main Cobb is measured");
    };
    (
        value_deg.abs(),
        (value_deg - band.p10)
            .abs()
            .max((band.p90 - value_deg).abs()),
    )
}
