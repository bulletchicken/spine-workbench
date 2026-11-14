//! Independent geometry answers: analytic endplates, reflected spines and
//! the made-up generator's saved angles. No film in this file is real.

use forward_model::endplates::Endplates;
use forward_model::rotation::matrix_from_rotation_vector;
use implant_layout::curves::measure;
use made_up_spines::answer_key::{answer_key, spine_for};
use made_up_spines::film_layout::FilmLayout;
use made_up_spines::patient::make_up_patient;
use made_up_spines::recipes::film_recipes;
use shapes::confidence::{Confidence, ConfidenceBand};
use shapes::film::FilmKind;
use shapes::film_geometry::{ImageRight, QuarterTurns};
use shapes::implant_layout::{CannotTellCode, ChildFacts, LayoutFilm, LayoutInput, NamingDoubt};
use shapes::landmarks::{Landmarks, SacrumTop};
use shapes::levels::Level;
use shapes::made_up::{
    AnswerKey, FilmAnswer, MadeUpRecipe, RecipeName, Tier, TrueMark, TrueMarks, VertebraPose,
};
use shapes::marks::{EndplateLines, FrontMarks, Mark, MarkSource, PedicleMark, SideMarks};
use shapes::naming::{ChosenBy, LevelNaming, Template};
use shapes::pixels::Px;
use shapes::spine_fit::{
    FitAngles, FitInput, FitRmsMm, FitStandardDeviations, FitStopReason, FittedVertebra,
    InferredFit, SpineFit,
};
use shapes::spine_readings::BodySide;
use shapes::vertebra::{FilmMarks, VertebraMarks, VertebraOnFilm};

fn near(got: f64, expected: f64, tolerance: f64) {
    assert!(
        (got - expected).abs() <= tolerance,
        "{got} against {expected} (tolerance {tolerance})"
    );
}

fn fitted(pose: VertebraPose) -> FittedVertebra {
    let rotation = matrix_from_rotation_vector(&pose.rotation.into());
    let angles = FitAngles {
        side_tilt_rad: libm::atan2(-rotation[(1, 2)], rotation[(2, 2)]),
        forward_lean_rad: libm::asin(rotation[(0, 2)].clamp(-1.0, 1.0)),
        twist_rad: libm::atan2(-rotation[(0, 1)], rotation[(0, 0)]),
    };
    FittedVertebra {
        level: pose.level,
        pose,
        angles,
        sd: FitStandardDeviations {
            centre_mm: [0.0; 3],
            angles_rad: [0.0; 3],
        },
        inferred: InferredFit {
            depth: false,
            width: false,
            twist: false,
        },
        rms_mm: FitRmsMm {
            front: Some(0.0),
            side: Some(0.0),
        },
        corner_tilt_rad: None,
    }
}

fn fit(poses: Vec<VertebraPose>) -> SpineFit {
    let origin_level = poses.last().expect("a spine").level;
    SpineFit {
        vertebrae: poses.into_iter().map(fitted).collect(),
        side_scale: 1.0,
        side_offset_mm: 0.0,
        origin_level,
        iterations: 0,
        converged: true,
        stop_reason: FitStopReason::SmallStep,
        rms_mm: 0.0,
        lengths: shapes::spine_fit::FitLengths::Measured,
        typical_curve_deg: None,
        front_film: shapes::spine_fit::FilmPlacement {
            mm_per_px: [1.0; 2],
            origin_px: [0.0; 2],
            facing_assumed: false,
        },
        side_film: Some(shapes::spine_fit::FilmPlacement {
            mm_per_px: [1.0; 2],
            origin_px: [0.0; 2],
            facing_assumed: false,
        }),
    }
}

fn input(fit: SpineFit) -> LayoutInput {
    LayoutInput {
        fit,
        front: None,
        bend_left: None,
        bend_right: None,
        child: ChildFacts {
            age_years: None,
            weight_kg: None,
            activity: None,
        },
        naming: NamingDoubt::AsRead,
        versions: 32,
        seed: 3,
        pedicle_child_share: None,
    }
}

fn hand_spine() -> LayoutInput {
    // Top-to-bottom tilt: -15,-8,0,+8,+15 degrees. A -4-degree
    // wedge at each end adds 2 degrees at its outside endplate, giving
    // -17 at T4's top and +17 at T8's bottom: Cobb = 34 degrees.
    // End centres have y=0, so the chord has y=0 everywhere, and T6's
    // y=-12 is the unique greatest deviation: apex T6, right-convex.
    let poses = [Level::T4, Level::T5, Level::T6, Level::T7, Level::T8]
        .into_iter()
        .zip([-15.0_f64, -8.0, 0.0, 8.0, 15.0])
        .zip([0.0, -6.0, -12.0, -6.0, 0.0])
        .enumerate()
        .map(|(i, ((level, tilt), y))| VertebraPose {
            level,
            centre: [0.0, y, 120.0 - i as f64 * 25.0],
            rotation: [tilt.to_radians(), 0.0, 0.0],
            height: 20.0,
            wedge_front: if i == 0 || i == 4 {
                -4.0_f64.to_radians()
            } else {
                0.0
            },
            wedge_side: 0.0,
            top_half_width: 15.0,
            top_half_depth: 10.0,
            bottom_half_width: 15.0,
            bottom_half_depth: 10.0,
        })
        .collect();
    input(fit(poses))
}

#[test]
fn hand_worked_tilts_endplates_cobb_and_apex_agree() {
    let input = hand_spine();
    let reading = measure(&input, &input.fit).expect("the hand spine is readable");
    assert_eq!(reading.curves.len(), 1);
    let curve = &reading.curves[0];
    assert_eq!(
        (curve.upper_index, curve.lower_index, curve.apex_index),
        (0, 4, 2)
    );
    assert_eq!(curve.convex, BodySide::Right);
    near(reading.vertebrae[0].upper_tilt_deg, -17.0, 1e-12);
    near(reading.vertebrae[4].lower_tilt_deg, 17.0, 1e-12);
    near(curve.cobb_deg, 34.0, 1e-12);
    near(curve.signed_cobb_deg, 34.0, 1e-12);
    near(curve.apical_deviation_mm, -12.0, 1e-12);
    assert_eq!(reading.minor_curve_minimum_deg, None);
}

#[test]
fn mirroring_the_spine_keeps_curve_size_and_swaps_convexity() {
    let original = hand_spine();
    let before = measure(&original, &original.fit).unwrap();
    let mut mirrored = original.clone();
    for v in &mut mirrored.fit.vertebrae {
        v.pose.centre[1] = -v.pose.centre[1];
        v.pose.rotation[0] = -v.pose.rotation[0];
        v.pose.rotation[2] = -v.pose.rotation[2];
        v.pose.wedge_front = -v.pose.wedge_front;
        v.angles.side_tilt_rad = -v.angles.side_tilt_rad;
        v.angles.twist_rad = -v.angles.twist_rad;
    }
    let after = measure(&mirrored, &mirrored.fit).unwrap();
    assert_eq!(after.curves.len(), before.curves.len());
    for (a, b) in before.curves.iter().zip(&after.curves) {
        assert_eq!(
            (a.upper_index, a.lower_index, a.apex_index),
            (b.upper_index, b.lower_index, b.apex_index)
        );
        near(a.cobb_deg, b.cobb_deg, 1e-12);
        near(a.signed_cobb_deg, -b.signed_cobb_deg, 1e-12);
        near(a.apical_deviation_mm, -b.apical_deviation_mm, 1e-12);
        assert_eq!(a.convex, BodySide::Right);
        assert_eq!(b.convex, BodySide::Left);
    }
}

#[test]
fn renaming_every_level_keeps_the_measured_geometry() {
    let original = hand_spine();
    let before = measure(&original, &original.fit).unwrap();
    let mut renamed = original.clone();
    for v in &mut renamed.fit.vertebrae {
        v.level = v.level.below().unwrap();
        v.pose.level = v.level;
    }
    renamed.fit.origin_level = renamed.fit.origin_level.below().unwrap();
    let after = measure(&renamed, &renamed.fit).unwrap();
    for (a, b) in before.vertebrae.iter().zip(&after.vertebrae) {
        near(a.upper_tilt_deg, b.upper_tilt_deg, 1e-12);
        near(a.lower_tilt_deg, b.lower_tilt_deg, 1e-12);
        assert_eq!(a.centre_mm, b.centre_mm);
        assert_eq!(a.twist_deg, b.twist_deg);
        assert_eq!(a.disc_below_deg, b.disc_below_deg);
    }
    for (a, b) in before.curves.iter().zip(&after.curves) {
        assert_eq!(
            (a.upper_index, a.lower_index, a.apex_index),
            (b.upper_index, b.lower_index, b.apex_index)
        );
        near(a.cobb_deg, b.cobb_deg, 1e-12);
        near(a.apical_deviation_mm, b.apical_deviation_mm, 1e-12);
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

fn marks(answer: &FilmAnswer, template: Template) -> FilmMarks {
    let vertebrae = answer
        .vertebrae
        .iter()
        .filter(|v| v.level >= Level::T1)
        .enumerate()
        .map(|(index, v)| VertebraOnFilm {
            id: index as u32 + 1,
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
    FilmMarks {
        film: answer.kind,
        vertebrae,
        landmarks: if answer.kind == FilmKind::Front {
            Landmarks {
                sacrum_top: Some(SacrumTop::Front {
                    sac_l: mark(answer.landmarks.sacrum_top[0]),
                    sac_r: mark(answer.landmarks.sacrum_top[2]),
                }),
                ..Landmarks::NONE
            }
        } else {
            Landmarks::NONE
        },
        naming: LevelNaming {
            template,
            chosen_by: ChosenBy::User,
            alternatives: Vec::new(),
            from_top: None,
            from_bottom: None,
            flags: Vec::new(),
            l5_confirmed_on: None,
        },
        named_for: answer.geometry.image_right.unwrap(),
    }
}

fn made_up(name: RecipeName) -> (LayoutInput, AnswerKey, Vec<FilmLayout>) {
    let recipe = MadeUpRecipe {
        name,
        seed: 3,
        tier: Tier::T0,
    };
    let patient = make_up_patient(name, recipe.seed).expect("the recipe stacks");
    let layouts: Vec<_> = film_recipes(&recipe, u64::from(recipe.seed))
        .iter()
        .map(|film| FilmLayout::new(film, spine_for(&patient, film.kind)))
        .collect();
    let key = answer_key(&recipe, &patient, &layouts);
    let mut input = input(fit(key
        .poses
        .iter()
        .filter(|p| p.level >= Level::T1)
        .cloned()
        .collect()));
    let film = |i: usize| LayoutFilm {
        marks: marks(&key.films[i], key.template),
        geometry: key.films[i].geometry.clone(),
        size: layouts[i].size,
    };
    input.front = Some(film(0));
    input.bend_left = Some(film(2));
    input.bend_right = Some(film(3));
    (input, key, layouts)
}

fn check_truth(input: &LayoutInput, key: &AnswerKey, tolerance_deg: f64) {
    let reading = measure(input, &input.fit).expect("made-up geometry measures");
    for truth in &key.angles.frontal_cobb {
        let found = reading
            .curves
            .iter()
            .filter(|curve| curve.region == Some(truth.region))
            .max_by(|a, b| a.cobb_deg.total_cmp(&b.cobb_deg))
            .unwrap_or_else(|| panic!("missing {:?}: {:?}", truth.region, reading.curves));
        assert!(
            (found.cobb_deg - truth.degrees).abs() <= tolerance_deg,
            "{:?}: {} against {}; curves: {:?}; tilts: {:?}",
            truth.region,
            found.cobb_deg,
            truth.degrees,
            reading.curves,
            reading
                .vertebrae
                .iter()
                .map(|v| (v.level, v.upper_tilt_deg, v.lower_tilt_deg))
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn thoracic_readings_match_the_independent_truth_angles() {
    let (input, key, _) = made_up(RecipeName::RightThoracic40);
    check_truth(&input, &key, 2.0);
    let reading = measure(&input, &input.fit).unwrap();
    assert_eq!(
        reading.curves[reading.major.unwrap()].convex,
        BodySide::Right
    );
}

#[test]
fn lumbar_readings_match_the_independent_truth_angles() {
    let (input, key, _) = made_up(RecipeName::LumbarLeft30);
    check_truth(&input, &key, 2.0);
    let reading = measure(&input, &input.fit).unwrap();
    assert_eq!(
        reading.curves[reading.major.unwrap()].convex,
        BodySide::Left
    );
}

#[test]
fn both_major_curves_match_the_independent_truth_angles() {
    let (input, key, _) = made_up(RecipeName::DoubleMajor55To45);
    check_truth(&input, &key, 2.0);
}

#[test]
fn a_joint_fit_still_matches_the_saved_curve_angles() {
    let (mut input, key, layouts) = made_up(RecipeName::RightThoracic40);
    let mut fit_input = FitInput {
        front: input.front.as_ref().unwrap().marks.clone(),
        side: marks(&key.films[1], key.template),
        front_geometry: key.films[0].geometry.clone(),
        side_geometry: key.films[1].geometry.clone(),
        front_size: layouts[0].size,
        side_size: layouts[1].size,
    };
    // Keep the whole curve and one neighbouring level while avoiding an
    // unnecessary full-chain covariance in this small CI check.
    for film in [&mut fit_input.front, &mut fit_input.side] {
        film.vertebrae
            .retain(|v| (Level::T4..=Level::L1).contains(&v.level));
    }
    input.fit = joint_fit::fit_spine(&fit_input).expect("perfect generated marks fit");
    // The fit's published test tolerance is 2 degrees per endplate angle;
    // an angle difference allows 4 degrees when both ends move.
    check_truth(&input, &key, 4.0);
}

#[test]
fn absent_films_and_unmeasured_twist_keep_their_codes() {
    let (mut input, _, _) = made_up(RecipeName::RightThoracic40);
    input.front.as_mut().unwrap().marks.landmarks.sacrum_top = None;
    input.bend_left = None;
    for v in &mut input.fit.vertebrae {
        v.inferred.depth = true;
        v.rms_mm.side = None;
        v.inferred.twist = true;
    }
    let reading = measure(&input, &input.fit).unwrap();
    assert_eq!(reading.csvl_y_mm, Err(CannotTellCode::NoSacrum));
    assert_eq!(reading.kyphosis_t2_t5_deg, Err(CannotTellCode::NoSideFilm));
    assert_eq!(reading.kyphosis_t5_t12_deg, Err(CannotTellCode::NoSideFilm));
    assert_eq!(reading.kyphosis_t10_l2_deg, Err(CannotTellCode::NoSideFilm));
    for v in &reading.vertebrae {
        assert_eq!(v.twist_deg, Err(CannotTellCode::TwistNotMeasured));
    }
    for curve in &reading.curves {
        assert_eq!(curve.bending_cobb_deg, Err(CannotTellCode::NoBendingFilms));
    }
    for (index, curve) in reading.curves.iter().enumerate() {
        if Some(index) != reading.major {
            assert!(!matches!(curve.structural, Ok(true)));
        }
    }
}

#[test]
fn generated_curve_ends_are_the_tilt_extremes_near_the_recipe_ends() {
    for name in [
        RecipeName::RightThoracic40,
        RecipeName::LumbarLeft30,
        RecipeName::DoubleMajor55To45,
    ] {
        let (input, _, _) = made_up(name);
        let patient = make_up_patient(name, 3).unwrap();
        let reading = measure(&input, &input.fit).unwrap();
        for recipe in &patient.recipe.curves {
            // This independently enumerates the generator's allowed endpoint
            // window. It does not ask the detector where its own answer lies.
            let t1 = patient
                .standing
                .levels
                .iter()
                .position(|l| *l == Level::T1)
                .unwrap() as i32;
            let start = (t1 + libm::floor(recipe.upper_end) as i32 - 1).max(t1) as usize;
            let end = (t1 + libm::ceil(recipe.lower_end) as i32 + 1)
                .min(patient.standing.levels.len() as i32 - 1) as usize;
            let mut expected = (start, start, 0.0);
            for upper in start..=end {
                let a = Endplates::of(&patient.standing.vertebrae[upper]).top.normal;
                for lower in upper..=end {
                    let b = Endplates::of(&patient.standing.vertebrae[lower])
                        .bottom
                        .normal;
                    let cobb = (libm::atan2(-a.y, a.z) - libm::atan2(-b.y, b.z))
                        .abs()
                        .to_degrees();
                    if cobb > expected.2 {
                        expected = (upper, lower, cobb);
                    }
                }
            }
            let found = reading
                .curves
                .iter()
                .filter(|curve| curve.region == Some(recipe.region))
                .max_by(|a, b| a.cobb_deg.total_cmp(&b.cobb_deg))
                .expect("the recipe's curve");
            assert_eq!(
                input.fit.vertebrae[found.upper_index].level,
                patient.standing.levels[expected.0]
            );
            assert_eq!(
                input.fit.vertebrae[found.lower_index].level,
                patient.standing.levels[expected.1]
            );
        }
    }
}

fn turn_and_stretch(film: &mut LayoutFilm, turns: u8) {
    // Independent physical re-expression of the same front film:
    // horizontal pixels twice as wide, vertical pixels half as high,
    // patient right on screen right, then store with a quarter turn.
    assert_eq!(film.geometry.quarter_turns.get(), 0);
    let mm = film.geometry.mm_per_px.unwrap();
    let upright = [film.size[0].div_ceil(2), film.size[1] * 2];
    film.size = if turns.is_multiple_of(2) {
        upright
    } else {
        [upright[1], upright[0]]
    };
    let place = |mark: &mut Mark| {
        let u = f64::from(upright[0] - 1) - mark.at.x() * 0.5;
        let v = mark.at.y() * 2.0;
        mark.at = match turns {
            0 => Px::new(u, v),
            1 => Px::new(v, f64::from(film.size[1] - 1) - u),
            2 => Px::new(
                f64::from(film.size[0] - 1) - u,
                f64::from(film.size[1] - 1) - v,
            ),
            _ => Px::new(f64::from(film.size[0] - 1) - v, u),
        };
    };
    for v in &mut film.marks.vertebrae {
        match &mut v.marks {
            VertebraMarks::Front { points } => {
                for mark in [
                    &mut points.sup_l,
                    &mut points.sup_r,
                    &mut points.inf_l,
                    &mut points.inf_r,
                ] {
                    place(mark);
                }
                for pedicle in [&mut points.ped_l, &mut points.ped_r] {
                    if let PedicleMark::Visible { at, .. } = pedicle {
                        let mut mark = Mark::auto(*at);
                        place(&mut mark);
                        *at = mark.at;
                    }
                }
            }
            VertebraMarks::Bending { lines } => {
                for mark in lines.sup.iter_mut().chain(lines.inf.iter_mut()) {
                    place(mark);
                }
            }
            VertebraMarks::Side { .. } => panic!("a front view"),
        }
    }
    if let Some(SacrumTop::Front { sac_l, sac_r }) = &mut film.marks.landmarks.sacrum_top {
        place(sac_l);
        place(sac_r);
    }
    let spacing = if turns.is_multiple_of(2) {
        [mm * 2.0, mm * 0.5]
    } else {
        [mm * 0.5, mm * 2.0]
    };
    film.geometry.mm_per_px = Some(spacing[0]);
    film.geometry.mm_per_px_vertical = Some(spacing[1]);
    film.geometry.quarter_turns = QuarterTurns::new(turns).unwrap();
    film.geometry.image_right = Some(ImageRight::PatientRight);
    film.marks.named_for = ImageRight::PatientRight;
}

#[test]
fn csvl_and_bending_angles_survive_turns_facing_and_unequal_pixels() {
    let (input, _, _) = made_up(RecipeName::RightThoracic40);
    let before = measure(&input, &input.fit).unwrap();
    assert!(before.csvl_y_mm.is_ok());
    for turns in 0..4 {
        let mut changed = input.clone();
        for film in [
            &mut changed.front,
            &mut changed.bend_left,
            &mut changed.bend_right,
        ] {
            turn_and_stretch(film.as_mut().unwrap(), turns);
        }
        let after = measure(&changed, &changed.fit).unwrap();
        near(after.csvl_y_mm.unwrap(), before.csvl_y_mm.unwrap(), 1e-9);
        for (a, b) in before.curves.iter().zip(&after.curves) {
            near(
                a.bending_cobb_deg.unwrap(),
                b.bending_cobb_deg.unwrap(),
                1e-9,
            );
            assert_eq!(a.straightening_bend, b.straightening_bend);
            match (a.bending_disc_below_deg, b.bending_disc_below_deg) {
                (Ok(a), Ok(b)) => near(a, b, 1e-9),
                (Err(a), Err(b)) => assert_eq!(a, b),
                values => panic!("disc readability changed under film geometry: {values:?}"),
            }
        }
    }
}

#[test]
fn positive_regional_kyphosis_has_the_stated_sign() {
    let mut input = hand_spine();
    input.fit.vertebrae.truncate(2);
    for (v, (level, lean)) in input
        .fit
        .vertebrae
        .iter_mut()
        .zip([(Level::T5, 20.0_f64), (Level::T12, -10.0)])
    {
        v.level = level;
        v.pose.level = level;
        v.pose.rotation = [0.0, lean.to_radians(), 0.0];
        v.pose.wedge_front = 0.0;
        v.angles.side_tilt_rad = 0.0;
        v.angles.forward_lean_rad = lean.to_radians();
    }
    near(
        implant_layout::curves::geometry::regional_kyphosis(&input.fit, Level::T5, Level::T12)
            .unwrap(),
        30.0,
        1e-12,
    );
}

#[test]
fn marked_sacrum_pixel_offset_is_added_to_the_l5_centre() {
    let (mut input, _, _) = made_up(RecipeName::RightThoracic40);
    let l5 = input
        .fit
        .vertebrae
        .iter_mut()
        .find(|v| v.level == Level::L5)
        .unwrap();
    l5.pose.centre = [0.0, 7.0, 10.0];
    l5.pose.rotation = [0.0; 3];
    l5.pose.wedge_front = 0.0;
    l5.angles = FitAngles {
        side_tilt_rad: 0.0,
        forward_lean_rad: 0.0,
        twist_rad: 0.0,
    };
    let front = input.front.as_mut().unwrap();
    front.geometry.mm_per_px = Some(0.5);
    front.geometry.mm_per_px_vertical = None;
    let marked = front
        .marks
        .vertebrae
        .iter_mut()
        .find(|v| v.level == Level::L5)
        .unwrap();
    let VertebraMarks::Front { points } = &mut marked.marks else {
        unreachable!()
    };
    points.inf_l.at = Px::new(130.0, 100.0);
    points.inf_r.at = Px::new(70.0, 100.0);
    front.marks.landmarks.sacrum_top = Some(SacrumTop::Front {
        sac_l: Mark::auto(Px::new(132.0, 120.0)),
        sac_r: Mark::auto(Px::new(92.0, 120.0)),
    });
    // Lower endplate y=7, marked inferior midpoint u=100, sacrum u=112,
    // spacing0.5 => CSVL =7 +(112-100)*0.5 =13 mm to patient's left.
    near(
        implant_layout::curves::geometry::csvl(&input, &input.fit).unwrap(),
        13.0,
        1e-12,
    );
    let mut version = input.fit.clone();
    let l5 = version
        .vertebrae
        .iter_mut()
        .find(|v| v.level == Level::L5)
        .unwrap();
    l5.pose.centre[1] += 3.0;
    near(
        implant_layout::curves::geometry::csvl(&input, &version).unwrap(),
        13.0,
        1e-12,
    );
}

#[test]
fn bending_cobb_uses_the_same_end_vertebrae_and_the_smaller_residual() {
    let (mut input, _, _) = made_up(RecipeName::RightThoracic40);
    for (film, upper_deg, lower_deg) in [
        (input.bend_left.as_mut().unwrap(), 10.0_f64, -20.0_f64),
        (input.bend_right.as_mut().unwrap(), 5.0, -7.0),
    ] {
        film.geometry.mm_per_px = None;
        film.geometry.mm_per_px_vertical = None;
        let line = |degrees: f64| {
            [
                Mark::auto(Px::new(10.0, 50.0)),
                Mark::auto(Px::new(
                    110.0,
                    50.0 - 100.0 * libm::tan(degrees.to_radians()),
                )),
            ]
        };
        for (level, degrees, upper) in [(Level::T4, upper_deg, true), (Level::T8, lower_deg, false)]
        {
            let v = film
                .marks
                .vertebrae
                .iter_mut()
                .find(|v| v.level == level)
                .unwrap();
            let VertebraMarks::Bending { lines } = &mut v.marks else {
                unreachable!()
            };
            if upper {
                lines.sup = line(degrees);
            } else {
                lines.inf = line(degrees);
            }
        }
    }
    let left = implant_layout::curves::geometry::bending_angle(
        input.bend_left.as_ref().unwrap(),
        Level::T4,
        Level::T8,
    )
    .unwrap();
    near(left, 30.0, 1e-12);
    let (residual, bend) =
        implant_layout::curves::geometry::bending_cobb(&input, Level::T4, Level::T8).unwrap();
    near(residual, 12.0, 1e-12);
    assert_eq!(bend, FilmKind::BendRight);
}

#[test]
fn bending_cobb_keeps_an_obtuse_curve_angle() {
    let (mut input, _, _) = made_up(RecipeName::RightThoracic40);
    let film = input.bend_left.as_mut().unwrap();
    film.geometry.mm_per_px = None;
    film.geometry.mm_per_px_vertical = None;
    for (level, degrees, upper) in [(Level::T4, -60.0_f64, true), (Level::T8, 60.0, false)] {
        let line = [
            Mark::auto(Px::new(10.0, 200.0)),
            Mark::auto(Px::new(
                110.0,
                200.0 - 100.0 * libm::tan(degrees.to_radians()),
            )),
        ];
        let v = film
            .marks
            .vertebrae
            .iter_mut()
            .find(|v| v.level == level)
            .unwrap();
        let VertebraMarks::Bending { lines } = &mut v.marks else {
            unreachable!()
        };
        if upper {
            lines.sup = line;
        } else {
            lines.inf = line;
        }
    }
    // Top -60 and bottom +60 degrees give Cobb120, not the supplementary60.
    near(
        implant_layout::curves::geometry::bending_angle(film, Level::T4, Level::T8).unwrap(),
        120.0,
        1e-12,
    );
}

#[test]
fn a_bending_film_in_the_other_slot_is_not_readable() {
    let (mut input, _, _) = made_up(RecipeName::RightThoracic40);
    input.bend_left.as_mut().unwrap().marks.film = FilmKind::BendRight;
    assert_eq!(
        implant_layout::curves::geometry::bending_cobb(&input, Level::T4, Level::T8),
        Err(CannotTellCode::NoBendingFilms)
    );
    let (mut input, _, _) = made_up(RecipeName::RightThoracic40);
    input.bend_right.as_mut().unwrap().marks.film = FilmKind::BendLeft;
    assert_eq!(
        implant_layout::curves::geometry::bending_cobb(&input, Level::T4, Level::T8),
        Err(CannotTellCode::NoBendingFilms)
    );
}

#[test]
fn a_straight_made_up_spine_has_no_coronal_curve() {
    let (input, _, _) = made_up(RecipeName::Straight);
    let reading = measure(&input, &input.fit).unwrap();
    assert!(reading.curves.is_empty(), "{:?}", reading.curves);
    assert_eq!(reading.major, None);
}
