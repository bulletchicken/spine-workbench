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
