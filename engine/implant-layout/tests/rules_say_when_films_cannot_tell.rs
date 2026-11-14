//! Film removals exercise the public level-rule boundary, using a wholly
//! synthetic spine and flat bending films. Missing inputs are never votes.

use forward_model::rotation::{rot_x, rot_z, rotation_vector_from_matrix};
use implant_layout::curves::{self, SpineReading};
use implant_layout::evidence::{Coordinate, PublishedValue, Table, tables};
use implant_layout::level_rules::{self, FusionRegion, RuleContext};
use shapes::confidence::{Confidence, ConfidenceBand};
use shapes::film::FilmKind;
use shapes::film_geometry::{FilmGeometry, ImageRight};
use shapes::implant_layout::{
    CannotTellCode, ChildFacts, LayoutFilm, LayoutInput, LevelRule, Measurement, NamingDoubt,
    Readable,
};
use shapes::landmarks::{Landmarks, SacrumTop};
use shapes::levels::Level;
use shapes::made_up::VertebraPose;
use shapes::marks::{EndplateLines, FrontMarks, Mark, MarkSource, PedicleMark};
use shapes::naming::{ChosenBy, LevelNaming, Template};
use shapes::pixels::Px;
use shapes::spine_fit::{
    FitAngles, FitRmsMm, FitStandardDeviations, FitStopReason, FittedVertebra, InferredFit,
    SpineFit,
};
use shapes::vertebra::{FilmMarks, VertebraMarks, VertebraOnFilm};

fn mark(x: f64, y: f64) -> Mark {
    Mark::auto(Px::new(x, y))
}

fn film(kind: FilmKind) -> LayoutFilm {
    let vertebrae = Level::MEASURED
        .into_iter()
        .enumerate()
        .map(|(i, level)| {
            let marks = if kind == FilmKind::Front {
                VertebraMarks::Front {
                    points: FrontMarks {
                        sup_l: mark(115.0, 25.0),
                        sup_r: mark(85.0, 25.0),
                        inf_l: mark(115.0, 45.0),
                        inf_r: mark(85.0, 45.0),
                        ped_l: PedicleMark::NotVisible {
                            source: MarkSource::Auto,
                        },
                        ped_r: PedicleMark::NotVisible {
                            source: MarkSource::Auto,
                        },
                    },
                }
            } else {
                VertebraMarks::Bending {
                    lines: EndplateLines {
                        sup: [mark(90.0, 25.0), mark(110.0, 25.0)],
                        inf: [mark(90.0, 45.0), mark(110.0, 45.0)],
                    },
                }
            };
            VertebraOnFilm {
                id: i as u32,
                level,
                marks,
                confidence: Confidence {
                    score: 1.0,
                    band: ConfidenceBand::High,
                    reasons: Vec::new(),
                    calibrated: false,
                    endplate_spread_px: None,
                    margin: None,
                },
                outline: None,
            }
        })
        .collect();
    LayoutFilm {
        marks: FilmMarks {
            film: kind,
            vertebrae,
            landmarks: if kind == FilmKind::Front {
                Landmarks {
                    sacrum_top: Some(SacrumTop::Front {
                        sac_l: mark(110.0, 60.0),
                        sac_r: mark(90.0, 60.0),
                    }),
                    ..Landmarks::NONE
                }
            } else {
                Landmarks::NONE
            },
            naming: LevelNaming {
                template: Template::Standard,
                chosen_by: ChosenBy::User,
                alternatives: Vec::new(),
                from_top: None,
                from_bottom: None,
                flags: Vec::new(),
                l5_confirmed_on: None,
            },
            named_for: ImageRight::PatientLeft,
        },
        geometry: FilmGeometry {
            mm_per_px: Some(1.0),
            image_right: Some(ImageRight::PatientLeft),
            ..FilmGeometry::UNKNOWN
        },
        size: [300, 600],
    }
}

fn deviation_threshold() -> f64 {
    let cell = tables()
        .lookup(
            Table::LevelRuleThresholds,
            &Coordinate {
                key: "first-cephalad-deviation-min".into(),
                group: Some("deviated-vertebra".into()),
                ..Coordinate::default()
            },
        )
        .unwrap();
    let PublishedValue::Scalar { value } = cell.value else {
        panic!("scalar deviation threshold")
    };
    value
}

fn spine() -> LayoutInput {
    let deviation = deviation_threshold();
    let tilts = [
        -15.0_f64, -15.0, -15.0, -15.0, -10.0, -5.0, 0.0, 5.0, 10.0, 15.0, 10.0, 5.0, 0.0, -5.0,
        -2.0, 0.0, 0.0,
    ];
    let offsets = [
        0.0,
        0.0,
        0.0,
        0.0,
        -5.0,
        -10.0,
        -(deviation + 5.0),
        -(deviation - 0.5),
        -5.0,
        0.0,
        2.0,
        4.0,
        8.0,
        4.0,
        2.0,
        0.0,
        0.0,
    ];
    let vertebrae = Level::MEASURED
        .into_iter()
        .enumerate()
        .map(|(i, level)| {
            let tilt = tilts[i].to_radians();
            let twist = if i < 11 {
                3.0_f64.to_radians()
            } else {
                -1.0_f64.to_radians()
            };
            FittedVertebra {
                level,
                pose: VertebraPose {
                    level,
                    centre: [0.0, offsets[i], 500.0 - i as f64 * 25.0],
                    rotation: rotation_vector_from_matrix(&(rot_x(tilt) * rot_z(twist))).into(),
                    height: 20.0,
                    wedge_front: 0.0,
                    wedge_side: 0.0,
                    top_half_width: 15.0,
                    top_half_depth: 10.0,
                    bottom_half_width: 15.0,
                    bottom_half_depth: 10.0,
                },
                angles: FitAngles {
                    side_tilt_rad: tilt,
                    forward_lean_rad: 0.0,
                    twist_rad: twist,
                },
                sd: FitStandardDeviations {
                    centre_mm: [0.0, 1.0, 0.0],
                    angles_rad: [1.0_f64.to_radians(); 3],
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
        })
        .collect();
    LayoutInput {
        fit: SpineFit {
            vertebrae,
            side_scale: 1.0,
            side_offset_mm: 0.0,
            origin_level: Level::L5,
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
        },
        front: Some(film(FilmKind::Front)),
        bend_left: Some(film(FilmKind::BendLeft)),
        bend_right: Some(film(FilmKind::BendRight)),
        child: ChildFacts {
            age_years: Some(14.0),
            weight_kg: None,
            activity: None,
        },
        naming: NamingDoubt::AsRead,
        versions: 2,
        seed: 21,
        pedicle_child_share: None,
    }
}

fn context<'a>(input: &'a LayoutInput, reading: &'a SpineReading) -> RuleContext<'a> {
    RuleContext {
        input,
        fit: &input.fit,
        reading,
        outlines: &[],
        region: FusionRegion::Structural,
    }
}

#[test]
fn removing_the_sacrum_leaves_every_centre_line_rule_unreadable() {
    let mut input = spine();
    input.front.as_mut().unwrap().marks.landmarks.sacrum_top = None;
    let reading = curves::measure(&input, &input.fit).unwrap();
    assert_eq!(reading.csvl_y_mm, Err(CannotTellCode::NoSacrum));
    let votes = level_rules::evaluate(&context(&input, &reading)).unwrap();
    for rule in [
        LevelRule::LastTouched,
        LevelRule::StableMinusOne,
        LevelRule::SubstantiallyTouched,
        LevelRule::DeviatedVertebra,
    ] {
        assert_eq!(
            votes.iter().find(|vote| vote.rule == rule).unwrap().level,
            Err(CannotTellCode::NoSacrum),
            "{rule:?}"
        );
    }
    let check = level_rules::liv_index::check(&context(&input, &reading), Level::L1).unwrap();
    assert_eq!(
        check.flagged,
        Readable::NotReadable {
            reason: CannotTellCode::NoSacrum
        }
    );
}

#[test]
fn removing_the_side_film_keeps_a_negative_bending_test_from_settling_a_minor() {
    let mut input = spine();
    for vertebra in &mut input.fit.vertebrae {
        vertebra.inferred.depth = true;
        vertebra.rms_mm.side = None;
    }
    let reading = curves::measure(&input, &input.fit).unwrap();
    assert_eq!(reading.kyphosis_t10_l2_deg, Err(CannotTellCode::NoSideFilm));
    // Flat bends establish flexible coronal minors; without sagittal
    // geometry the OR in the structural definition remains unresolved.
    let vote = level_rules::upper_end_vertebra::evaluate(&context(&input, &reading)).unwrap();
    assert_eq!(vote.level, Err(CannotTellCode::NoSideFilm));
    let selective = level_rules::selective_thoracic::evaluate(&context(&input, &reading)).unwrap();
    let kyphosis = selective
        .criteria
        .iter()
        .find(|criterion| criterion.measurement == Measurement::KyphosisT10L2)
        .unwrap();
    assert_eq!(kyphosis.value, Err(CannotTellCode::NoSideFilm));
    assert_eq!(kyphosis.holds(), Err(CannotTellCode::NoSideFilm));
}

#[test]
fn removing_either_bend_retains_the_matching_reason_for_both_selective_rules() {
    for left_missing in [true, false] {
        let mut input = spine();
        if left_missing {
            input.bend_left = None;
        } else {
            input.bend_right = None;
        }
        let reading = curves::measure(&input, &input.fit).unwrap();
        let ctx = context(&input, &reading);
        assert_eq!(
            level_rules::upper_end_vertebra::evaluate(&ctx)
                .unwrap()
                .level,
            Err(CannotTellCode::NoBendingFilms)
        );
        for selective in [
            level_rules::selective_thoracic::evaluate(&ctx).unwrap(),
            level_rules::selective_lumbar::evaluate(&ctx).unwrap(),
        ] {
            let bending = selective
                .criteria
                .iter()
                .find(|criterion| criterion.measurement == Measurement::BendingCobb)
                .unwrap();
            assert_eq!(bending.value, Err(CannotTellCode::NoBendingFilms));
            assert_eq!(bending.holds(), Err(CannotTellCode::NoBendingFilms));
        }
    }
}

#[test]
fn removing_measured_twist_never_turns_the_template_into_a_neutral_vote() {
    let mut input = spine();
    for vertebra in &mut input.fit.vertebrae {
        vertebra.inferred.twist = true;
    }
    let reading = curves::measure(&input, &input.fit).unwrap();
    let ctx = context(&input, &reading);
    assert_eq!(
        level_rules::suk_neutral::evaluate(&ctx).unwrap().level,
        Err(CannotTellCode::TwistNotMeasured)
    );
    for selective in [
        level_rules::selective_thoracic::evaluate(&ctx).unwrap(),
        level_rules::selective_lumbar::evaluate(&ctx).unwrap(),
    ] {
        let rotation = selective
            .criteria
            .iter()
            .find(|criterion| criterion.measurement == Measurement::ApicalRotationRatio)
            .unwrap();
        assert_eq!(rotation.value, Err(CannotTellCode::TwistNotMeasured));
        assert_eq!(rotation.holds(), Err(CannotTellCode::TwistNotMeasured));
    }
}

#[test]
fn shoulders_and_rotation_grades_remain_unreadable_even_with_every_film() {
    let input = spine();
    let reading = curves::measure(&input, &input.fit).unwrap();
    assert_eq!(
        level_rules::lenke_shoulder::evaluate(&context(&input, &reading))
            .unwrap()
            .level,
        Err(CannotTellCode::ShouldersOutsideTheModel)
    );
    let ctx = RuleContext {
        region: FusionRegion::Lumbar,
        ..context(&input, &reading)
    };
    assert_eq!(
        level_rules::lenke_l3_l4::evaluate(&ctx).unwrap().level,
        Err(CannotTellCode::NoRotationGrade)
    );
}

#[test]
fn renaming_type_five_c_ends_moves_both_relative_rules_one_anatomical_level() {
    // At the rule boundary this made-up TL/L reading is declared type5C.
    // The words say UEV and LEV (flat bends reach neutral), so relabelling
    // the same bodies and film lines moves both answers by one name.
    let mut input = spine();
    let mut reading = curves::measure(&input, &input.fit).unwrap();
    let mut lumbar = reading
        .curves
        .iter()
        .find(|c| c.region == Some(shapes::made_up::CurveRegion::ThoracolumbarLumbar))
        .unwrap()
        .clone();
    lumbar.structural = Ok(true);
    reading.curves = vec![lumbar];
    reading.major = Some(0);
    reading.classification.curve_type = Ok(shapes::implant_layout::CurveType::ThoracolumbarLumbar);
    reading.classification.lumbar_modifier =
        Ok(shapes::implant_layout::LumbarModifier::ClearOfApex);
    let ctx = RuleContext {
        region: FusionRegion::Lumbar,
        ..context(&input, &reading)
    };
    let before = [
        level_rules::uev_5c::evaluate(&ctx).unwrap(),
        level_rules::lev_plus_one_5c::evaluate(&ctx).unwrap(),
    ];
    let names = Template::Standard.names();
    let next = |level| names[names.iter().position(|&l| l == level).unwrap() + 1];
    for v in &mut input.fit.vertebrae {
        v.level = next(v.level);
        v.pose.level = v.level;
    }
    input.fit.origin_level = next(input.fit.origin_level);
    for film in [
        &mut input.front,
        &mut input.bend_left,
        &mut input.bend_right,
    ]
    .into_iter()
    .flatten()
    {
        for v in &mut film.marks.vertebrae {
            v.level = next(v.level);
        }
    }
    for v in &mut reading.vertebrae {
        v.level = next(v.level);
    }
    let ctx = RuleContext {
        region: FusionRegion::Lumbar,
        ..context(&input, &reading)
    };
    let after = [
        level_rules::uev_5c::evaluate(&ctx).unwrap(),
        level_rules::lev_plus_one_5c::evaluate(&ctx).unwrap(),
    ];
    for (a, b) in before.iter().zip(after) {
        assert!(a.level.is_ok());
        assert_eq!(b.level, a.level.map(next));
        for (x, y) in a.inputs.iter().zip(b.inputs) {
            assert_eq!(x.value, y.value);
            assert_eq!(x.sd, y.sd);
            assert_eq!(x.threshold, y.threshold);
        }
    }
}

#[test]
fn an_under_twelve_lumbar_outline_is_missing_from_the_published_tables() {
    let mut input = spine();
    input.child.age_years = Some(10.0);
    // Missing table coverage and count conservation do not depend on fit
    // noise. Keep this small fixture fixed while exercising both rows.
    for vertebra in &mut input.fit.vertebrae {
        vertebra.sd.centre_mm = [0.0; 3];
        vertebra.sd.angles_rad = [0.0; 3];
    }
    let result = level_rules::measure(&input).unwrap();
    assert_eq!(
        result.nominal.classification.lumbar_modifier,
        Err(CannotTellCode::NoPedicleTable)
    );
    assert!(result.votes.iter().all(|vote| {
        vote.counts
            .levels
            .iter()
            .map(|count| count.versions)
            .sum::<u32>()
            + vote
                .counts
                .unread
                .iter()
                .map(|count| count.versions)
                .sum::<u32>()
            == input.versions
    }));
}

#[test]
fn every_reported_flip_moves_the_level_when_applied_to_the_same_input() {
    let mut input = spine();
    // The hand-worked minimum concerns lateral translation. Keep angular
    // readings fixed so this fixture isolates its stated source of noise.
    for vertebra in &mut input.fit.vertebrae {
        vertebra.sd.angles_rad = [0.0; 3];
    }
    let before = level_rules::measure(&input).unwrap();
    let mut applied = 0;
    for region in &before.regions {
        for vote in &region.votes {
            let Some(flip) = vote.flip else {
                continue;
            };
            let moved = level_rules::apply_flip(&input, &flip).unwrap();
            let after = level_rules::nominal(&moved).unwrap();
            let nominal = region
                .nominal_rules
                .iter()
                .find(|rule| rule.rule == vote.rule)
                .unwrap();
            let ctx = RuleContext {
                input: &moved,
                fit: &moved.fit,
                reading: &after.reading,
                outlines: &after.outlines,
                region: region.region,
            };
            let rules = level_rules::evaluate(&ctx).unwrap();
            let changed = rules.iter().find(|rule| rule.rule == vote.rule).unwrap();
            assert_eq!(
                changed.level,
                Ok(flip.moves_to),
                "{:?} {:?}: {flip:?}",
                region.region,
                vote.rule
            );
            assert_ne!(
                changed.level, nominal.level,
                "{:?} {:?}",
                region.region, vote.rule
            );
            let own_sd = vote
                .inputs
                .iter()
                .find(|value| {
                    value.measurement == flip.measurement
                        && value.level == flip.level
                        && value.region == flip.region
                })
                .unwrap()
                .sd
                .unwrap();
            assert!((flip.change_in_sd - flip.change_value / own_sd).abs() < 1e-8);
            applied += 1;
        }
    }
    assert!(
        applied > 0,
        "the noisy hand spine must expose a movable rule"
    );
    // Walking cranially, T7 is the first nominal deviation above the
    // table limit. T8 lies exactly half a millimetre inside it and its
    // own SD is1mm. Every more caudal centre is at least2mm inside;
    // moving T7 back to the boundary takes5mm. The smallest one-input
    // change is therefore just beyond -0.5 SD at T8, read independently
    // from the fixture and the rule's strict "more than" definition.
    let flip = before
        .votes
        .iter()
        .find(|vote| vote.rule == LevelRule::DeviatedVertebra)
        .unwrap()
        .flip
        .unwrap();
    assert_eq!(flip.measurement, Measurement::CentreLineOffset);
    assert_eq!(flip.level, Some(Level::T8));
    assert_eq!(flip.moves_to, Level::T8);
    assert!(flip.change_value < -0.5 && flip.change_value > -0.500001);
    assert!(flip.change_in_sd < -0.5 && flip.change_in_sd > -0.500001);
}

#[test]
fn a_five_degree_t9_tilt_change_moves_the_end_plus_one_level() {
    let mut input = spine();
    for vertebra in &mut input.fit.vertebrae {
        vertebra.sd.centre_mm = [0.0; 3];
        vertebra.sd.angles_rad = [0.0; 3];
        if vertebra.level == Level::T9 {
            vertebra.sd.angles_rad[0] = 1.0_f64.to_radians();
        }
    }
    let before = level_rules::measure(&input).unwrap();
    // The standing upper tilts are T9=10 and T10=15 degrees. The
    // only uncertain angle is T9 with SD1degree. Five degrees takes
    // that endplate to the maximum, moving LEV from T10 to T9 and
    // therefore EV+1 from T11 to T10. No other one-input change has
    // a nonzero SD in this deliberately isolated mechanical fixture.
    for region in before.regions.iter().filter(|region| {
        matches!(
            region.region,
            FusionRegion::Structural | FusionRegion::Thoracic
        )
    }) {
        let vote = region
            .votes
            .iter()
            .find(|vote| vote.rule == LevelRule::EndPlusOne)
            .unwrap();
        let nominal = region
            .nominal_rules
            .iter()
            .find(|rule| rule.rule == LevelRule::EndPlusOne)
            .unwrap();
        assert_eq!(nominal.level, Ok(Level::T11));
        let flip = vote.flip.unwrap();
        assert_eq!(flip.measurement, Measurement::CoronalTilt);
        assert_eq!(flip.level, Some(Level::T9));
        assert_eq!(flip.moves_to, Level::T10);
        assert!((flip.change_value - 5.0).abs() < 1e-6);
        assert!((flip.change_in_sd - 5.0).abs() < 1e-6);
        let moved = level_rules::apply_flip(&input, &flip).unwrap();
        let after = level_rules::nominal(&moved).unwrap();
        let context = RuleContext {
            input: &moved,
            fit: &moved.fit,
            reading: &after.reading,
            outlines: &after.outlines,
            region: region.region,
        };
        let result = level_rules::end_plus_one::evaluate(&context).unwrap();
        assert_eq!(result.level, Ok(Level::T10));
    }
}
