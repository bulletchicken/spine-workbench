//! A lumbar side film cannot turn typical thoracic angles into measurements.

use forward_model::project::{front_marks, side_marks};
use joint_fit::fit_from_films;
use made_up_spines::film_layout::FilmLayout;
use made_up_spines::patient::{Patient, make_up_patient};
use made_up_spines::recipes::film_recipes;
use shapes::confidence::{Confidence, ConfidenceBand};
use shapes::film::FilmKind;
use shapes::film_geometry::ImageRight;
use shapes::landmarks::Landmarks;
use shapes::levels::Level;
use shapes::made_up::{MadeUpRecipe, RecipeName, Tier};
use shapes::naming::{ChosenBy, LevelNaming};
use shapes::spine_fit::{FilmToFit, FitFromFilms, FitLengths, MarkSpread, TypicalSource};
use shapes::spine_readings::{FitTier, NotMeasured, Reading, ReadingsInput};
use shapes::vertebra::{FilmMarks, VertebraMarks, VertebraOnFilm};
use spine_readings::measure_spine;

fn film(patient: &Patient, layout: &FilmLayout) -> FilmToFit {
    let vertebrae: Vec<_> = patient
        .standing
        .levels
        .iter()
        .zip(&patient.standing.vertebrae)
        .filter(|(level, _)| **level >= Level::T1)
        .enumerate()
        .map(|(index, (&level, body))| VertebraOnFilm {
            id: index as u32 + 1,
            level,
            marks: match layout.kind {
                FilmKind::Front => VertebraMarks::Front {
                    points: front_marks(body, &layout.camera).unwrap(),
                },
                FilmKind::Side => VertebraMarks::Side {
                    points: side_marks(body, &layout.camera).unwrap(),
                },
                _ => unreachable!("standing films only"),
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
            film: layout.kind,
            vertebrae,
            landmarks: Landmarks::NONE,
            naming: LevelNaming {
                template: patient.recipe.template,
                chosen_by: ChosenBy::User,
                alternatives: Vec::new(),
                from_top: Some(levels.clone()),
                from_bottom: Some(levels),
                flags: Vec::new(),
                l5_confirmed_on: None,
            },
            named_for: layout.geometry.image_right.unwrap(),
        },
        geometry: layout.geometry.clone(),
        size: layout.size,
        mark_spread: MarkSpread::Millimetres {
            sd_mm: 1.0,
            shared_sd_mm: 0.0,
        },
        facing_assumed: false,
    }
}

fn not_on_side_film(reading: Reading) -> f64 {
    match reading {
        Reading::NotMeasured {
            reason: NotMeasured::NotOnSideFilm,
            typical_deg: Some(value),
        } => {
            assert!(value.is_finite());
            value
        }
        value => panic!("a typical value for an unseen level was expected: {value:?}"),
    }
}

#[test]
fn a_lumbar_side_film_leaves_thoracic_readings_typical_and_front_readings_unchanged() {
    lumbar_side_readings(true);
}

#[test]
fn a_lumbar_side_film_without_spacing_keeps_the_same_reading_coverage() {
    lumbar_side_readings(false);
}

fn lumbar_side_readings(with_spacing: bool) {
    let recipe = MadeUpRecipe {
        name: RecipeName::RightThoracic40,
        seed: 3,
        tier: Tier::T0,
    };
    let patient = make_up_patient(recipe.name, recipe.seed).unwrap();
    let layouts = film_recipes(&recipe, u64::from(recipe.seed));
    let mut front = film(&patient, &FilmLayout::new(&layouts[0], &patient.standing));
    let mut side = film(&patient, &FilmLayout::new(&layouts[1], &patient.standing));
    if !with_spacing {
        for film in [&mut front, &mut side] {
            film.geometry.mm_per_px = None;
            film.geometry.mm_per_px_vertical = None;
        }
    }
    assert!(!side.facing_assumed);
    // Remove the thoracic marks before fitting: only L1 to L5 are on this film.
    side.marks.vertebrae.retain(|v| v.level >= Level::L1);
    let side_levels = side
        .marks
        .vertebrae
        .iter()
        .map(|v| v.level)
        .collect::<Vec<_>>();
    side.marks.naming.from_top = Some(side_levels.clone());
    side.marks.naming.from_bottom = Some(side_levels);
    assert_eq!(side.marks.vertebrae.len(), 5);
    let fit = fit_from_films(&FitFromFilms {
        front: front.clone(),
        side: Some(side.clone()),
        typical: TypicalSource::BuiltIn,
        twist_readings: Vec::new(),
    })
    .unwrap();
    assert!(fit.converged && fit.side_film.is_some());
    assert_eq!(
        fit.lengths,
        if with_spacing {
            FitLengths::Measured
        } else {
            FitLengths::TypicalSize
        }
    );
    assert!(!fit.side_film.unwrap().facing_assumed);
    assert_eq!(fit.vertebrae.len(), 17);
    for v in &fit.vertebrae {
        assert_eq!(v.inferred.depth, v.level < Level::L1, "{:?}", v.level);
    }
    let mut input = ReadingsInput {
        fit,
        front: Some(front),
        side: Some(side),
        bend_left: None,
        bend_right: None,
        versions: 32,
        seed: recipe.seed,
    };
    let answer = measure_spine(&input).unwrap();
    assert_eq!(answer.tier, FitTier::TwoFilms);
    assert!(matches!(
        answer.sagittal.lordosis_l1_l5,
        Reading::Measured { .. }
    ));
    for reading in [
        answer.sagittal.kyphosis_t2_t5,
        answer.sagittal.kyphosis_t5_t12,
        answer.sagittal.kyphosis_t10_l2,
    ] {
        not_on_side_film(reading);
    }
    for (reading, fitted) in answer.vertebrae.iter().zip(&input.fit.vertebrae) {
        if fitted.level < Level::L1 {
            assert_eq!(
                not_on_side_film(reading.forward_lean),
                fitted.angles.forward_lean_rad.to_degrees()
            );
        } else {
            assert!(matches!(reading.forward_lean, Reading::Measured { .. }));
        }
    }
    assert_eq!(answer.between.side.len(), 136);
    for pair in &answer.between.side {
        if pair.upper < Level::L1 || pair.lower < Level::L1 {
            not_on_side_film(pair.angle);
        } else {
            assert!(matches!(pair.angle, Reading::Measured { .. }));
        }
    }
    // Independently read the midline between the two front endplate lines.
    let front = input.front.as_ref().unwrap();
    assert_eq!(front.geometry.quarter_turns.get(), 0);
    assert_eq!(front.geometry.image_right, Some(ImageRight::PatientLeft));
    for (reading, marked) in answer.vertebrae.iter().zip(&front.marks.vertebrae) {
        let VertebraMarks::Front { points } = &marked.marks else {
            unreachable!();
        };
        let across =
            points.sup_l.at.x() + points.inf_l.at.x() - points.sup_r.at.x() - points.inf_r.at.x();
        let rise =
            points.sup_r.at.y() + points.inf_r.at.y() - points.sup_l.at.y() - points.inf_l.at.y();
        let Reading::Measured { value_deg, .. } = reading.coronal_tilt else {
            panic!("front marks remain measured");
        };
        assert!((value_deg - libm::atan2(rise, across).to_degrees()).abs() < 1e-10);
    }
    // Changing only the side-observation flag cannot change anything read
    // from the same front geometry, including its repeatable bands and curves.
    for v in &mut input.fit.vertebrae {
        v.inferred.depth = false;
    }
    let flagged_as_seen = measure_spine(&input).unwrap();
    for (a, b) in answer.vertebrae.iter().zip(&flagged_as_seen.vertebrae) {
        assert_eq!(a.coronal_tilt, b.coronal_tilt);
    }
    assert_eq!(answer.between.front, flagged_as_seen.between.front);
    assert_eq!(answer.curves, flagged_as_seen.curves);
}
