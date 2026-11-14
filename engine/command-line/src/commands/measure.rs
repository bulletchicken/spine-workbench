//! Find and fit front/side films, or fit a made-up case's exact marks,
//! then print the same readings contract.

use crate::{FilmPath, MeasureArgs, Outcome, Recipe};
use made_up_spines::answer_key::{answer_key, spine_for};
use made_up_spines::film_layout::FilmLayout;
use made_up_spines::patient::make_up_patient;
use made_up_spines::recipes::film_recipes;
use shapes::auto_pass::{AutoPassInput, ChainOptions, FrontFinder};
use shapes::confidence::{Confidence, ConfidenceBand};
use shapes::film::FilmKind;
use shapes::film_geometry::{FilmGeometry, GeometryGuess, PolarityGuess};
use shapes::landmarks::{Landmarks, SacrumTop};
use shapes::levels::Level;
use shapes::made_up::{FilmAnswer, MadeUpRecipe, RecipeName, Tier, TrueMark, TrueMarks};
use shapes::marks::{EndplateLines, FrontMarks, Mark, MarkSource, PedicleMark, SideMarks};
use shapes::naming::{ChosenBy, LevelNaming};
use shapes::spine_fit::{FilmToFit, FitFromFilms, FitLengths, MarkSpread, TypicalSource};
use shapes::spine_readings::{Reading, ReadingsInput, SpineReadings};
use shapes::vertebra::{FilmMarks, VertebraMarks, VertebraOnFilm};

pub fn run(args: &MeasureArgs) -> Outcome {
    let input = if args.recipe.is_some() {
        made_up_input(args)?
    } else {
        films_input(&args.films)?
    };
    let readings = spine_readings::measure_spine(&input)?;
    if args.json {
        println!("{}", serde_json::to_string(&readings)?);
    } else {
        print_table(&readings, input.fit.lengths);
    }
    Ok(())
}

pub(crate) fn made_up_input(
    args: &MeasureArgs,
) -> Result<ReadingsInput, Box<dyn std::error::Error>> {
    let recipe = MadeUpRecipe {
        name: name(args.recipe.ok_or("a made-up measurement needs --recipe")?),
        seed: args.seed.ok_or("a made-up measurement needs --seed")?,
        tier: Tier::T0,
    };
    let patient = make_up_patient(recipe.name, recipe.seed)
        .map_err(|error| format!("made-up-unsatisfiable: {error:?}"))?;
    let layouts: Vec<_> = film_recipes(&recipe, u64::from(recipe.seed))
        .iter()
        .map(|film| FilmLayout::new(film, spine_for(&patient, film.kind)))
        .collect();
    let key = answer_key(&recipe, &patient, &layouts);
    let films: Vec<_> = key
        .films
        .iter()
        .zip(&layouts)
        .map(|(answer, layout)| film(answer, key.template, layout.size))
        .collect();
    let front = films[0].clone();
    let side = (!args.front_only).then(|| films[1].clone());
    let fit = joint_fit::fit_from_films(&FitFromFilms {
        front: front.clone(),
        side: side.clone(),
        typical: TypicalSource::BuiltIn,
        twist_readings: Vec::new(),
    })?;
    Ok(ReadingsInput {
        fit,
        front: Some(front),
        side,
        bend_left: Some(films[2].clone()),
        bend_right: Some(films[3].clone()),
        versions: 32,
        seed: recipe.seed,
    })
}

/// Trained front-corner allowance chosen for this command: the per-axis
/// share and the shared box displacement requested for the finder output.
const TRAINED_FRONT_SPREAD: MarkSpread = MarkSpread::ShareOfHeight {
    share: 0.11,
    shared_share: 0.08,
};

/// The joint fit's existing one-millimetre allowance for placed marks,
/// with no shared displacement. The side pass reports no calibrated spread.
const SIDE_SPREAD: MarkSpread = MarkSpread::Millimetres {
    sd_mm: 1.0,
    shared_sd_mm: 0.0,
};

pub(crate) fn films_input(
    inputs: &[FilmPath],
) -> Result<ReadingsInput, Box<dyn std::error::Error>> {
    validate_film_kinds(inputs)?;
    let mut decoded = Vec::with_capacity(inputs.len());
    let mut films = Vec::with_capacity(inputs.len());
    for input in inputs {
        let bytes = std::fs::read(&input.path).map_err(|_| "cannot read an input film")?;
        let pixels = film_reader::read_film(&bytes, input.kind)?;
        let guess = film_geometry::sense_geometry(&pixels)?;
        let geometry = geometry(&guess);
        let mut film = super::pass::record(&pixels, geometry);
        if guess.polarity == PolarityGuess::LooksInverted {
            film.values.inverted = !film.values.inverted;
        }
        films.push(film);
        decoded.push(pixels);
    }
    let input = AutoPassInput {
        case_version: 0,
        films,
        options: ChainOptions {
            acquisition: shapes::auto_pass::Acquisition::Unknown,
            population: shapes::auto_pass::Population::Adolescent,
            template: None,
        },
        first_vertebra_id: 1,
        front_finder: FrontFinder::Trained,
        side_finder: shapes::SideFinder::HandBuilt,
    };
    let found = super::pass::conduct_auto_pass(&input, &decoded)?;
    let mut front = None;
    let mut side = None;
    for marks in found.marks {
        let record = input
            .films
            .iter()
            .find(|film| film.kind == marks.film)
            .ok_or("the pass returned marks for an unknown film")?;
        if record.geometry.image_right.is_none() {
            eprintln!(
                "{} facing has no cue; using the pass's unconfirmed {:?} convention",
                record.kind.name(),
                marks.named_for
            );
        }
        let spread = if marks.film == FilmKind::Front {
            TRAINED_FRONT_SPREAD
        } else {
            SIDE_SPREAD
        };
        let to_fit = marked_film(
            marks,
            record.geometry.clone(),
            [record.image.width, record.image.height],
            spread,
        );
        if record.kind == FilmKind::Front {
            front = Some(to_fit);
        } else {
            side = Some(to_fit);
        }
    }
    let front = front.ok_or("the pass returned no front film")?;
    let fit = joint_fit::fit_from_films(&FitFromFilms {
        front: front.clone(),
        side: side.clone(),
        typical: TypicalSource::BuiltIn,
        twist_readings: Vec::new(),
    })?;
    Ok(ReadingsInput {
        fit,
        front: Some(front),
        side,
        bend_left: None,
        bend_right: None,
        versions: 32,
        seed: 1,
    })
}

fn validate_film_kinds(inputs: &[FilmPath]) -> Result<(), Box<dyn std::error::Error>> {
    let front = inputs
        .iter()
        .filter(|film| film.kind == FilmKind::Front)
        .count();
    let side = inputs
        .iter()
        .filter(|film| film.kind == FilmKind::Side)
        .count();
    if front != 1 || side > 1 || inputs.len() != front + side {
        return Err("measure needs one --film front=PATH and at most one --film side=PATH".into());
    }
    Ok(())
}

/// Use the proposed orientation, facing and preferred file scale without
/// marking a proposal confirmed. No scale remains absent for the typical fit.
fn geometry(guess: &GeometryGuess) -> FilmGeometry {
    let scale = guess.scales.first();
    FilmGeometry {
        mm_per_px: scale.map(|scale| scale.mm_per_px),
        mm_per_px_vertical: scale.and_then(|scale| scale.mm_per_px_vertical),
        scale: scale.map(|scale| scale.source.clone()),
        quarter_turns: guess.upright.quarter_turns,
        image_right: guess
            .image_right
            .image_right
            .or(guess.image_right.file_says),
        ..FilmGeometry::UNKNOWN
    }
}

/// Match the camera to the facing under which the pass named its corners.
/// With no facing cue this keeps its stated convention explicit, unconfirmed.
fn marked_film(
    marks: FilmMarks,
    mut geometry: FilmGeometry,
    size: [u32; 2],
    mark_spread: MarkSpread,
) -> FilmToFit {
    geometry.image_right = Some(marks.named_for);
    FilmToFit {
        marks,
        geometry,
        size,
        mark_spread,
        // The facing is the finder's own convention, not a confirmed one.
        facing_assumed: true,
    }
}

fn film(answer: &FilmAnswer, template: shapes::naming::Template, size: [u32; 2]) -> FilmToFit {
    let vertebrae: Vec<_> = answer
        .vertebrae
        .iter()
        .filter(|v| v.level >= Level::T1)
        .enumerate()
        .map(|(index, truth)| VertebraOnFilm {
            id: index as u32 + 1,
            level: truth.level,
            marks: match truth.marks {
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
                TrueMarks::Bending { sup, inf } => VertebraMarks::Bending {
                    lines: EndplateLines {
                        sup: sup.map(mark),
                        inf: inf.map(mark),
                    },
                },
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
    let levels: Vec<_> = vertebrae.iter().map(|v| v.level).collect();
    let [first, _, second] = answer.landmarks.sacrum_top;
    let sacrum_top = Some(if answer.kind == shapes::film::FilmKind::Side {
        SacrumTop::Side {
            sac_a: mark(first),
            sac_p: mark(second),
        }
    } else {
        SacrumTop::Front {
            sac_l: mark(first),
            sac_r: mark(second),
        }
    });
    FilmToFit {
        marks: FilmMarks {
            film: answer.kind,
            vertebrae,
            landmarks: Landmarks {
                c7: None,
                sacrum_top,
                hips: Vec::new(),
            },
            naming: LevelNaming {
                template,
                chosen_by: ChosenBy::User,
                alternatives: Vec::new(),
                from_top: Some(levels.clone()),
                from_bottom: Some(levels),
                flags: Vec::new(),
                l5_confirmed_on: None,
            },
            named_for: answer
                .geometry
                .image_right
                .expect("a made-up film has a facing"),
        },
        geometry: answer.geometry.clone(),
        size,
        // Exact marks, with a stated allowance for reading their position.
        mark_spread: MarkSpread::Millimetres {
            sd_mm: 0.5,
            shared_sd_mm: 0.0,
        },
        // A made-up film states its facing.
        facing_assumed: false,
    }
}

fn mark(truth: TrueMark) -> Mark {
    Mark {
        at: truth.at,
        source: MarkSource::Hand,
        pinned: false,
        inferred: !truth.visible,
    }
}

fn pedicle(truth: TrueMark) -> PedicleMark {
    if truth.visible {
        PedicleMark::Visible {
            at: truth.at,
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

fn name(recipe: Recipe) -> RecipeName {
    match recipe {
        Recipe::Random => RecipeName::Random,
        Recipe::Straight => RecipeName::Straight,
        Recipe::RightThoracic40 => RecipeName::RightThoracic40,
        Recipe::DoubleMajor55To45 => RecipeName::DoubleMajor55To45,
        Recipe::LumbarLeft30 => RecipeName::LumbarLeft30,
        Recipe::Severe90 => RecipeName::Severe90,
        Recipe::SixLumbar => RecipeName::SixLumbar,
        Recipe::ElevenRibs => RecipeName::ElevenRibs,
        Recipe::UpsideDown => RecipeName::UpsideDown,
        Recipe::MirroredFront => RecipeName::MirroredFront,
        Recipe::SideFacingLeft => RecipeName::SideFacingLeft,
        Recipe::HeavyShoulders => RecipeName::HeavyShoulders,
    }
}

fn print_table(readings: &SpineReadings, lengths: FitLengths) {
    println!(
        "films: {}; lengths: {}; {}; {} versions; degrees",
        if readings.tier == shapes::spine_readings::FitTier::TwoFilms {
            "front + side"
        } else {
            "front"
        },
        match lengths {
            FitLengths::Measured => "measured",
            FitLengths::TypicalSize => "typical-size",
        },
        match readings.tier {
            shapes::spine_readings::FitTier::TwoFilms => "two-films",
            shapes::spine_readings::FitTier::FrontFilmOnly => "front-film-only",
        },
        readings.versions
    );
    println!("reading                 value [p10, p90]");
    for curve in &readings.curves {
        println!(
            "Cobb {:?}–{:?} ({:?})  {}",
            curve.upper_end,
            curve.lower_end,
            curve.convex,
            reading(&curve.cobb)
        );
    }
    let sagittal = &readings.sagittal;
    for (name, value) in [
        ("kyphosis T2–T5", &sagittal.kyphosis_t2_t5),
        ("kyphosis T5–T12", &sagittal.kyphosis_t5_t12),
        ("kyphosis T10–L2", &sagittal.kyphosis_t10_l2),
        ("lordosis L1–L5", &sagittal.lordosis_l1_l5),
        ("lordosis L1–S1", &sagittal.lordosis_l1_s1),
    ] {
        println!("{name:23} {}", reading(value));
    }
}

fn reading(value: &Reading) -> String {
    match value {
        Reading::Measured { value_deg, band } => {
            format!("{value_deg:.1} [{:.1}, {:.1}]", band.p10, band.p90)
        }
        Reading::NotMeasured {
            reason,
            typical_deg,
        } => {
            let reason = serde_json::to_value(reason).expect("a reading reason serialises");
            let reason = reason.as_str().expect("a reason is a string");
            typical_deg.map_or_else(
                || format!("not-measured: {reason}"),
                |value| format!("not-measured: {reason}; typical {value:.1}"),
            )
        }
    }
}

#[cfg(test)]
#[path = "../../../spine-readings/tests/pair_truth/mod.rs"]
mod pair_truth;

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn ordinary_four_film_intake_keeps_truth_or_retries_front_exactly() {
        let recipe = MadeUpRecipe {
            name: RecipeName::RightThoracic40,
            seed: 4,
            tier: Tier::T1,
        };
        let made = made_up_spines::make_up_spine(&recipe).unwrap();
        let decoded: Vec<_> = made
            .films
            .iter()
            .map(|f| film_reader::read_film(&f.png, f.kind).unwrap())
            .collect();
        let films: Vec<_> = decoded
            .iter()
            .map(|d| super::super::pass::record(d, FilmGeometry::UNKNOWN))
            .collect();
        let pass = AutoPassInput {
            case_version: 0,
            films: films.clone(),
            options: ChainOptions {
                acquisition: shapes::auto_pass::Acquisition::Unknown,
                population: shapes::auto_pass::Population::Adolescent,
                template: None,
            },
            first_vertebra_id: 1,
            front_finder: FrontFinder::HandBuilt,
            side_finder: shapes::SideFinder::HandBuilt,
        };
        let found = auto_pass::run::auto_pass(&pass, &decoded).unwrap();
        let to_fit = |kind| {
            let record = films.iter().find(|f| f.kind == kind).unwrap();
            let marks = found.marks.iter().find(|m| m.film == kind).unwrap().clone();
            FilmToFit {
                geometry: FilmGeometry {
                    image_right: Some(marks.named_for),
                    ..FilmGeometry::UNKNOWN
                },
                marks,
                size: [record.image.width, record.image.height],
                mark_spread: SIDE_SPREAD,
                facing_assumed: true,
            }
        };
        let request = FitFromFilms {
            front: to_fit(FilmKind::Front),
            side: Some(to_fit(FilmKind::Side)),
            typical: TypicalSource::BuiltIn,
            twist_readings: Vec::new(),
        };
        let fit = joint_fit::fit_from_films(&request).unwrap();
        assert!(fit.converged);
        let read = |request: &FitFromFilms, fit| {
            spine_readings::measure_spine(&ReadingsInput {
                fit,
                front: Some(request.front.clone()),
                side: request.side.clone(),
                bend_left: None,
                bend_right: None,
                versions: 4,
                seed: recipe.seed,
            })
            .unwrap()
        };
        let observed = read(&request, fit.clone());
        eprintln!(
            "ordinary intake: side_used={} curves={:?} sagittal={:?}",
            fit.side_film.is_some(),
            observed.curves,
            observed.sagittal
        );
        if fit.side_film.is_none() {
            let mut alone = request.clone();
            alone.side = None;
            assert_eq!(fit, joint_fit::fit_from_films(&alone).unwrap());
        } else {
            let exact = FitFromFilms {
                front: film(
                    &made.answer_key.films[0],
                    made.answer_key.template,
                    request.front.size,
                ),
                side: Some(film(
                    &made.answer_key.films[1],
                    made.answer_key.template,
                    request.side.as_ref().unwrap().size,
                )),
                typical: TypicalSource::BuiltIn,
                twist_readings: Vec::new(),
            };
            let truth = read(&exact, joint_fit::fit_from_films(&exact).unwrap());
            pair_truth::check(&observed, &truth, false);
        }
    }

    #[test]
    fn measure_accepts_the_recipe_seed_and_view_switches() {
        let parsed = crate::Cli::try_parse_from([
            "spine",
            "measure",
            "--recipe",
            "right-thoracic-40",
            "--seed",
            "1",
            "--front-only",
            "--json",
        ])
        .expect("measure arguments");
        let crate::Command::Measure(args) = parsed.command else {
            panic!("the measure command")
        };
        assert!(matches!(args.recipe, Some(Recipe::RightThoracic40)));
        assert_eq!(args.seed, Some(1));
        assert!(args.front_only && args.json);
    }

    #[test]
    fn film_arguments_cannot_mix_with_recipe_arguments_or_repeat_views() {
        for args in [
            vec!["spine", "measure"],
            vec!["spine", "measure", "--recipe", "straight"],
            vec!["spine", "measure", "--seed", "1"],
            vec![
                "spine", "measure", "--film", "front=x", "--recipe", "straight", "--seed", "1",
            ],
            vec!["spine", "measure", "--film", "front=x", "--front-only"],
            vec!["spine", "measure", "--film", "front=x", "--seed", "1"],
        ] {
            assert!(crate::Cli::try_parse_from(args).is_err());
        }
        for views in [
            vec!["side=x"],
            vec!["front=x", "front=y"],
            vec!["front=x", "side=y", "side=z"],
            vec!["front=x", "bend-left=y"],
        ] {
            let inputs: Vec<_> = views
                .iter()
                .map(|view| crate::film_path(view).expect("a named film"))
                .collect();
            assert!(validate_film_kinds(&inputs).is_err());
        }
        let parsed = crate::Cli::try_parse_from([
            "spine", "measure", "--film", "side=y", "--film", "front=x", "--json",
        ])
        .expect("film arguments");
        let crate::Command::Measure(args) = parsed.command else {
            panic!("measure")
        };
        assert!(validate_film_kinds(&args.films).is_ok());
        assert!(args.recipe.is_none() && args.seed.is_none() && args.json);
    }

    fn no_marks() -> FilmMarks {
        FilmMarks {
            film: FilmKind::Front,
            vertebrae: Vec::new(),
            landmarks: Landmarks::NONE,
            naming: LevelNaming {
                template: shapes::naming::Template::Standard,
                chosen_by: ChosenBy::Engine,
                alternatives: Vec::new(),
                from_top: None,
                from_bottom: None,
                flags: Vec::new(),
                l5_confirmed_on: None,
            },
            named_for: shapes::film_geometry::ImageRight::PatientRight,
        }
    }

    #[test]
    fn found_marks_keep_their_facing_size_and_spread_without_claiming_confirmation() {
        let geometry = FilmGeometry {
            mm_per_px: Some(0.4),
            mm_per_px_vertical: Some(0.6),
            quarter_turns: shapes::film_geometry::QuarterTurns::new(1).unwrap(),
            ..FilmGeometry::UNKNOWN
        };
        let marks = no_marks();
        let converted = marked_film(
            marks.clone(),
            geometry.clone(),
            [800, 1200],
            TRAINED_FRONT_SPREAD,
        );
        assert_eq!(converted.marks, marks);
        assert_eq!(converted.geometry.image_right, Some(marks.named_for));
        assert_eq!(converted.geometry.mm_per_px, Some(0.4));
        assert_eq!(converted.geometry.mm_per_px_vertical, Some(0.6));
        assert_eq!(converted.geometry.quarter_turns.get(), 1);
        assert_eq!(
            converted.geometry.confirmed,
            FilmGeometry::UNKNOWN.confirmed
        );
        assert_eq!(converted.size, [800, 1200]);
        assert_eq!(
            converted.mark_spread,
            MarkSpread::ShareOfHeight {
                share: 0.11,
                shared_share: 0.08
            }
        );
        let typical = marked_film(marks, FilmGeometry::UNKNOWN, [800, 1200], SIDE_SPREAD);
        assert_eq!(typical.geometry.mm_per_px, None);
        assert_eq!(
            typical.mark_spread,
            MarkSpread::Millimetres {
                sd_mm: 1.0,
                shared_sd_mm: 0.0
            }
        );
    }

    #[test]
    fn geometry_uses_the_preferred_file_scale_and_keeps_missing_scale_absent() {
        use shapes::film_geometry::{
            Certainty, FileScaleRule, ImageRight, ImageRightGuess, QuarterTurns, ScalePlane,
            ScaleResult, ScaleSource, UprightGuess,
        };
        let mut guess = GeometryGuess {
            upright: UprightGuess {
                quarter_turns: QuarterTurns::new(2).unwrap(),
                certainty: Certainty::Low,
                evidence: Vec::new(),
            },
            image_right: ImageRightGuess {
                image_right: None,
                file_says: Some(ImageRight::PatientRight),
                certainty: Certainty::Low,
                evidence: Vec::new(),
                looks_mirrored: false,
            },
            polarity: PolarityGuess::AsStored,
            scales: Vec::new(),
            lead_markers: Vec::new(),
            text_hints: Vec::new(),
            collimation: Vec::new(),
        };
        let unknown = geometry(&guess);
        assert_eq!(unknown.mm_per_px, None);
        assert_eq!(unknown.image_right, Some(ImageRight::PatientRight));
        assert_eq!(unknown.quarter_turns.get(), 2);
        let source = ScaleSource::File {
            rule: FileScaleRule::Fiducial,
        };
        guess.scales.push(ScaleResult {
            mm_per_px: 0.3,
            mm_per_px_vertical: Some(0.4),
            source: source.clone(),
            plane: ScalePlane::FiducialDepth,
            relative_uncertainty: Some(0.01),
            weak: false,
        });
        let measured = geometry(&guess);
        assert_eq!(measured.mm_per_px, Some(0.3));
        assert_eq!(measured.mm_per_px_vertical, Some(0.4));
        assert_eq!(measured.scale, Some(source));
        assert!(
            !measured.confirmed.scale
                && !measured.confirmed.left_right
                && !measured.confirmed.upright
        );
    }
}
