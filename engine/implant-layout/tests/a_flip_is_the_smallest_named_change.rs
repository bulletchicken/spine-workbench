//! Independent angular contact and unread-count cases from the landmark words.

use forward_model::rotation::{rot_x, rotation_vector_from_matrix};
use implant_layout::level_rules;
use shapes::confidence::{Confidence, ConfidenceBand};
use shapes::film::FilmKind;
use shapes::film_geometry::{FilmGeometry, ImageRight};
use shapes::implant_layout::{
    CannotTellCode, ChildFacts, LayoutFilm, LayoutInput, LevelRule, Measurement, NamingDoubt,
    RuleFlip,
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

fn point(x: f64, y: f64) -> Mark {
    Mark::auto(Px::new(x, y))
}

fn film(kind: FilmKind) -> LayoutFilm {
    LayoutFilm {
        geometry: FilmGeometry {
            mm_per_px: Some(1.0),
            image_right: Some(ImageRight::PatientLeft),
            ..FilmGeometry::UNKNOWN
        },
        size: [400, 600],
        marks: FilmMarks {
            film: kind,
            named_for: ImageRight::PatientLeft,
            naming: LevelNaming {
                template: Template::Standard,
                chosen_by: ChosenBy::User,
                alternatives: Vec::new(),
                from_top: None,
                from_bottom: None,
                flags: Vec::new(),
                l5_confirmed_on: None,
            },
            landmarks: if kind == FilmKind::Front {
                Landmarks {
                    sacrum_top: Some(SacrumTop::Front {
                        sac_l: point(110.0, 140.0),
                        sac_r: point(90.0, 140.0),
                    }),
                    ..Landmarks::NONE
                }
            } else {
                Landmarks::NONE
            },
            vertebrae: Level::MEASURED
                .into_iter()
                .enumerate()
                .map(|(index, level)| VertebraOnFilm {
                    id: index as u32,
                    level,
                    marks: if kind == FilmKind::Front {
                        VertebraMarks::Front {
                            points: FrontMarks {
                                sup_l: point(115.0, 100.0),
                                sup_r: point(85.0, 100.0),
                                inf_l: point(115.0, 120.0),
                                inf_r: point(85.0, 120.0),
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
                                sup: [point(90.0, 100.0), point(110.0, 100.0)],
                                inf: [point(90.0, 120.0), point(110.0, 120.0)],
                            },
                        }
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
                .collect(),
        },
    }
}

fn input() -> LayoutInput {
    let tilts: [f64; 17] = [
        -10.0, -8.0, -6.0, -4.0, -2.0, 0.0, 2.0, 4.0, 6.0, 8.0, 10.0, 0.0, -5.0, -3.0, -2.0, -1.0,
        0.0,
    ];
    LayoutInput {
        fit: SpineFit {
            vertebrae: Level::MEASURED
                .into_iter()
                .enumerate()
                .map(|(index, level)| FittedVertebra {
                    level,
                    pose: VertebraPose {
                        level,
                        centre: [
                            0.0,
                            if level == Level::T12 {
                                15.001
                            } else if level == Level::T7 {
                                -30.0
                            } else {
                                0.0
                            },
                            500.0 - index as f64 * 25.0,
                        ],
                        rotation: rotation_vector_from_matrix(&rot_x(tilts[index].to_radians()))
                            .into(),
                        height: 20.0,
                        wedge_front: 0.0,
                        wedge_side: 0.0,
                        top_half_width: 15.0,
                        bottom_half_width: 15.0,
                        top_half_depth: 10.0,
                        bottom_half_depth: 10.0,
                    },
                    angles: FitAngles {
                        side_tilt_rad: tilts[index].to_radians(),
                        forward_lean_rad: 0.0,
                        twist_rad: 0.0,
                    },
                    sd: FitStandardDeviations {
                        centre_mm: [0.0; 3],
                        angles_rad: [
                            if level == Level::T12 {
                                1.0_f64.to_radians()
                            } else {
                                0.0
                            },
                            0.0,
                            0.0,
                        ],
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
                })
                .collect(),
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
        seed: 71,
        pedicle_child_share: None,
    }
}

#[test]
fn the_first_body_contact_can_flip_by_tilt_without_any_centre_uncertainty() {
    // LTV is the most cephalad TL/L body touched.
    // T12 initially spans y=[0.001,30.001], so L1 is the first touch.
    // With a positive 0.001-radian tilt, its left edge is
    // 15.001 - 15 cos(a) - 10 sin(a) < 0, so T12 becomes the LTV.
    // Only this tilt has nonzero SD; a finite angular flip must exist.
    let input = input();
    let before = level_rules::measure(&input).unwrap();
    let nominal = before
        .nominal_rules
        .iter()
        .find(|rule| rule.rule == LevelRule::LastTouched)
        .unwrap();
    assert_eq!(nominal.level, Ok(Level::L1));
    let known_change = RuleFlip {
        measurement: Measurement::CoronalTilt,
        level: Some(Level::T12),
        region: None,
        change_value: 0.001_f64.to_degrees(),
        change_in_sd: 0.001_f64.to_degrees(),
        moves_to: Level::T12,
    };
    let moved = level_rules::apply_flip(&input, &known_change).unwrap();
    let after = level_rules::nominal(&moved).unwrap();
    assert_eq!(
        after
            .rules
            .iter()
            .find(|rule| rule.rule == LevelRule::LastTouched)
            .unwrap()
            .level,
        Ok(Level::T12)
    );
    let returned = before
        .votes
        .iter()
        .find(|vote| vote.rule == LevelRule::LastTouched)
        .unwrap()
        .flip
        .expect("a known one-input angular change reaches T12");
    assert!(returned.change_in_sd.abs() <= known_change.change_in_sd);
    // Solve 15 cos(a)+10 sin(a)=15.001 independently as
    // sqrt(325) cos(a-atan2(10,15))=15.001. This is the first
    // contact, and either tilt direction has the same support.
    let first_contact =
        (libm::atan2(10.0, 15.0) - libm::acos(15.001 / libm::sqrt(325.0))).to_degrees();
    assert_eq!(returned.measurement, Measurement::CoronalTilt);
    assert_eq!(returned.level, Some(Level::T12));
    assert!((returned.change_value.abs() - first_contact).abs() < 1e-7);
    let replay = level_rules::apply_flip(&input, &returned).unwrap();
    let replay = level_rules::nominal(&replay).unwrap();
    assert_eq!(
        replay
            .rules
            .iter()
            .find(|rule| rule.rule == LevelRule::LastTouched)
            .unwrap()
            .level,
        Ok(returned.moves_to)
    );
    let smaller = RuleFlip {
        change_value: returned.change_value * 0.999,
        change_in_sd: returned.change_in_sd * 0.999,
        ..returned
    };
    let smaller = level_rules::apply_flip(&input, &smaller).unwrap();
    let smaller = level_rules::nominal(&smaller).unwrap();
    assert_eq!(
        smaller
            .rules
            .iter()
            .find(|rule| rule.rule == LevelRule::LastTouched)
            .unwrap()
            .level,
        Ok(Level::L1)
    );
}

#[test]
fn a_reported_centre_flip_must_not_hide_a_smaller_angular_change() {
    // The same exact contact calculation above gives an angular change
    // of about 0.00573 SD. Give the centre a 0.001 mm SD as well: its
    // 0.001 mm move is about 1 SD, hence cannot be the minimum.
    let mut input = input();
    input
        .fit
        .vertebrae
        .iter_mut()
        .find(|v| v.level == Level::T12)
        .unwrap()
        .sd
        .centre_mm[1] = 0.001;
    let measured = level_rules::measure(&input).unwrap();
    let returned = measured
        .votes
        .iter()
        .find(|v| v.rule == LevelRule::LastTouched)
        .unwrap()
        .flip
        .expect("the centre change reaches contact");
    let moved = level_rules::apply_flip(&input, &returned).unwrap();
    assert_eq!(
        level_rules::nominal(&moved)
            .unwrap()
            .rules
            .iter()
            .find(|r| r.rule == LevelRule::LastTouched)
            .unwrap()
            .level,
        Ok(returned.moves_to)
    );
    let known_smaller = RuleFlip {
        measurement: Measurement::CoronalTilt,
        level: Some(Level::T12),
        region: None,
        change_value: 0.001_f64.to_degrees(),
        change_in_sd: 0.001_f64.to_degrees(),
        moves_to: Level::T12,
    };
    let smaller = level_rules::apply_flip(&input, &known_smaller).unwrap();
    assert_eq!(
        level_rules::nominal(&smaller)
            .unwrap()
            .rules
            .iter()
            .find(|r| r.rule == LevelRule::LastTouched)
            .unwrap()
            .level,
        Ok(Level::T12)
    );
    assert!(
        returned.change_in_sd.abs() <= known_smaller.change_in_sd,
        "reported {returned:?} despite a known smaller angular flip {known_smaller:?}"
    );
    let first_contact =
        (libm::atan2(10.0, 15.0) - libm::acos(15.001 / libm::sqrt(325.0))).to_degrees();
    assert_eq!(returned.measurement, Measurement::CoronalTilt);
    assert_eq!(returned.level, Some(Level::T12));
    assert!((returned.change_in_sd.abs() - first_contact).abs() < 1e-7);
}

#[test]
fn missing_inputs_conserve_every_versions_level_or_refusal() {
    for missing in ["sacrum", "side", "bend-left", "bend-right", "twist"] {
        let mut input = input();
        input.versions = 32;
        for vertebra in &mut input.fit.vertebrae {
            vertebra.sd.angles_rad = [0.0; 3];
        }
        let (dependent, reason) = match missing {
            "sacrum" => {
                input.front.as_mut().unwrap().marks.landmarks.sacrum_top = None;
                (
                    vec![
                        LevelRule::LastTouched,
                        LevelRule::StableMinusOne,
                        LevelRule::SubstantiallyTouched,
                        LevelRule::DeviatedVertebra,
                    ],
                    CannotTellCode::NoSacrum,
                )
            }
            "side" => {
                for vertebra in &mut input.fit.vertebrae {
                    vertebra.inferred.depth = true;
                    vertebra.rms_mm.side = None;
                }
                (
                    vec![LevelRule::UpperEndVertebra, LevelRule::EndPlusOne],
                    CannotTellCode::NoSideFilm,
                )
            }
            "twist" => {
                for vertebra in &mut input.fit.vertebrae {
                    vertebra.inferred.twist = true;
                }
                (
                    vec![LevelRule::SukNeutral],
                    CannotTellCode::TwistNotMeasured,
                )
            }
            _ => {
                if missing == "bend-left" {
                    input.bend_left = None;
                } else {
                    input.bend_right = None;
                }
                (
                    vec![LevelRule::UpperEndVertebra, LevelRule::EndPlusOne],
                    CannotTellCode::NoBendingFilms,
                )
            }
        };
        let measured = level_rules::measure(&input).unwrap();
        for region in &measured.regions {
            for vote in &region.votes {
                assert_eq!(
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
                            .sum::<u32>(),
                    input.versions,
                    "{missing} {:?} {:?}",
                    region.region,
                    vote.rule
                );
            }
        }
        for version in &measured.version_rules {
            for rule in &dependent {
                assert_eq!(
                    version
                        .iter()
                        .find(|reading| reading.rule == *rule)
                        .unwrap()
                        .level,
                    Err(reason),
                    "{missing} {rule:?}"
                );
            }
            assert_eq!(
                version
                    .iter()
                    .find(|rule| rule.rule == LevelRule::LenkeShoulder)
                    .unwrap()
                    .level,
                Err(CannotTellCode::ShouldersOutsideTheModel)
            );
        }
        for rule in dependent {
            let vote = measured
                .votes
                .iter()
                .find(|vote| vote.rule == rule)
                .unwrap();
            assert!(vote.counts.levels.is_empty(), "{missing} {rule:?}");
            assert_eq!(
                vote.counts
                    .unread
                    .iter()
                    .find(|count| count.reason == reason)
                    .unwrap()
                    .versions,
                input.versions
            );
        }
    }
}

#[test]
fn reflecting_the_geometric_spine_preserves_each_nominal_end() {
    // Reflection reverses signed lateral geometry, while the contact
    // magnitudes, zero twist and anatomical order are unchanged. Thus
    // every readable end and every absence reason must be identical.
    let input = input();
    let before = level_rules::nominal(&input).unwrap();
    let mut reflected = input.clone();
    for vertebra in &mut reflected.fit.vertebrae {
        vertebra.pose.centre[1] = -vertebra.pose.centre[1];
        vertebra.angles.side_tilt_rad = -vertebra.angles.side_tilt_rad;
        vertebra.pose.rotation =
            rotation_vector_from_matrix(&rot_x(vertebra.angles.side_tilt_rad)).into();
    }
    // All synthetic film lines are horizontal, centred at x=100. Their
    // reflection, with left/right names exchanged, has these same marks.
    let after = level_rules::nominal(&reflected).unwrap();
    for rule in &before.rules {
        let reflected_rule = after
            .rules
            .iter()
            .find(|other| other.rule == rule.rule)
            .unwrap();
        assert_eq!(reflected_rule.level, rule.level, "{:?}", rule.rule);
    }
}

#[test]
fn an_uncertain_measured_twist_can_be_the_first_body_contact() {
    use forward_model::rotation::{rot_y, rot_z};
    for forward_lean in [0.0, core::f64::consts::FRAC_PI_2] {
        let mut input = input();
        for vertebra in &mut input.fit.vertebrae {
            vertebra.sd.centre_mm = [0.0; 3];
            vertebra.sd.angles_rad = [0.0; 3];
            if vertebra.level == Level::T12 {
                vertebra.pose.centre[1] = 10.001;
                vertebra.pose.top_half_width = 10.0;
                vertebra.pose.bottom_half_width = 10.0;
                vertebra.pose.top_half_depth = 15.0;
                vertebra.pose.bottom_half_depth = 15.0;
                vertebra.angles.side_tilt_rad = 0.0;
                vertebra.angles.twist_rad = 0.0;
                vertebra.angles.forward_lean_rad = forward_lean;
                vertebra.pose.rotation =
                    rotation_vector_from_matrix(&(rot_y(forward_lean) * rot_z(0.0))).into();
                vertebra.sd.angles_rad[2] = 1.0_f64.to_radians();
            }
        }
        // This upright synthetic body's transverse ellipse has half-width10
        // and half-depth15. A measured twist t gives y support
        // sqrt(100cos²t+225sin²t). Equating that to centre10.001 yields
        // sin²t=(10.001²-100)/(225-100), independently of the flip search.
        // Ry leaves patient y unchanged, so the same answer holds at90degree
        // forward lean: the twist axis is then patient x, rather than z.
        let first_contact =
            libm::asin(libm::sqrt((10.001 * 10.001 - 100.0) / (225.0 - 100.0))).to_degrees();
        let before = level_rules::measure(&input).unwrap();
        assert_eq!(
            before
                .nominal_rules
                .iter()
                .find(|rule| rule.rule == LevelRule::LastTouched)
                .unwrap()
                .level,
            Ok(Level::L1)
        );
        let known = RuleFlip {
            measurement: Measurement::Twist,
            level: Some(Level::T12),
            region: None,
            change_value: first_contact + 0.001_f64.to_degrees(),
            change_in_sd: first_contact + 0.001_f64.to_degrees(),
            moves_to: Level::T12,
        };
        let known_moved = level_rules::apply_flip(&input, &known).unwrap();
        assert_eq!(
            level_rules::nominal(&known_moved)
                .unwrap()
                .rules
                .iter()
                .find(|rule| rule.rule == LevelRule::LastTouched)
                .unwrap()
                .level,
            Ok(Level::T12)
        );
        let returned = before
            .votes
            .iter()
            .find(|vote| vote.rule == LevelRule::LastTouched)
            .unwrap()
            .flip
            .expect("a measured twist reaches contact");
        assert_eq!(returned.measurement, Measurement::Twist);
        assert_eq!(returned.level, Some(Level::T12));
        assert_eq!(returned.moves_to, Level::T12);
        assert!((returned.change_value.abs() - first_contact).abs() < 1e-7);
        assert!((returned.change_in_sd.abs() - first_contact).abs() < 1e-7);
        let replay = level_rules::apply_flip(&input, &returned).unwrap();
        assert_eq!(
            level_rules::nominal(&replay)
                .unwrap()
                .rules
                .iter()
                .find(|rule| rule.rule == LevelRule::LastTouched)
                .unwrap()
                .level,
            Ok(Level::T12)
        );
        let smaller = RuleFlip {
            change_value: returned.change_value * 0.999,
            change_in_sd: returned.change_in_sd * 0.999,
            ..returned
        };
        let smaller = level_rules::apply_flip(&input, &smaller).unwrap();
        assert_eq!(
            level_rules::nominal(&smaller)
                .unwrap()
                .rules
                .iter()
                .find(|rule| rule.rule == LevelRule::LastTouched)
                .unwrap()
                .level,
            Ok(Level::L1)
        );
    }
}

#[test]
fn a_narrow_body_contact_interval_keeps_its_analytic_first_contact() {
    let mut input = input();
    let radius = libm::sqrt(15.0 * 15.0 + 10.0 * 10.0);
    let centre = radius - 1e-9;
    for vertebra in &mut input.fit.vertebrae {
        vertebra.sd.centre_mm = [0.0; 3];
        vertebra.sd.angles_rad = [0.0; 3];
        if vertebra.level == Level::T12 {
            vertebra.pose.centre[1] = centre;
            vertebra.sd.angles_rad[0] = 1.0_f64.to_radians();
        }
    }
    // With half-width15 and half-height10, the positive-quadrant lateral
    // support is R*cos(a-phi), R=sqrt325, phi=atan2(10,15). The first
    // contact is phi-acos(centre/R); contact ends at phi+acos(centre/R).
    // Setting centre only1e-9mm inside R makes that interval about
    // 0.0012degree wide, away from the nominal angle. These are explicit
    // synthetic geometry margins, not a clinical angular threshold.
    let peak = libm::atan2(10.0, 15.0);
    let half_interval = libm::acos(centre / radius);
    let first_contact = (peak - half_interval).to_degrees();
    assert!((2.0 * half_interval).to_degrees() < 0.002);
    let before = level_rules::nominal(&input).unwrap();
    assert_eq!(
        before
            .rules
            .iter()
            .find(|rule| rule.rule == LevelRule::LastTouched)
            .unwrap()
            .level,
        Ok(Level::L1)
    );
    let known = RuleFlip {
        measurement: Measurement::CoronalTilt,
        level: Some(Level::T12),
        region: None,
        change_value: peak.to_degrees(),
        change_in_sd: peak.to_degrees(),
        moves_to: Level::T12,
    };
    let known_moved = level_rules::apply_flip(&input, &known).unwrap();
    assert_eq!(
        level_rules::nominal(&known_moved)
            .unwrap()
            .rules
            .iter()
            .find(|rule| rule.rule == LevelRule::LastTouched)
            .unwrap()
            .level,
        Ok(Level::T12)
    );
    let returned = level_rules::flips::nominal_flip(
        &input,
        LevelRule::LastTouched,
        level_rules::FusionRegion::Structural,
    )
    .unwrap()
    .expect("a finite narrow interval contains body contact");
    assert_eq!(returned.measurement, Measurement::CoronalTilt);
    assert_eq!(returned.level, Some(Level::T12));
    assert!((returned.change_value.abs() - first_contact).abs() < 1e-7);
    assert!((returned.change_in_sd.abs() - first_contact).abs() < 1e-7);
    let replay = level_rules::apply_flip(&input, &returned).unwrap();
    assert_eq!(
        level_rules::nominal(&replay)
            .unwrap()
            .rules
            .iter()
            .find(|rule| rule.rule == LevelRule::LastTouched)
            .unwrap()
            .level,
        Ok(Level::T12)
    );
    let smaller = RuleFlip {
        change_value: returned.change_value * 0.999,
        change_in_sd: returned.change_in_sd * 0.999,
        ..returned
    };
    let smaller = level_rules::apply_flip(&input, &smaller).unwrap();
    assert_eq!(
        level_rules::nominal(&smaller)
            .unwrap()
            .rules
            .iter()
            .find(|rule| rule.rule == LevelRule::LastTouched)
            .unwrap()
            .level,
        Ok(Level::L1)
    );
}

#[test]
fn an_exact_body_tangency_keeps_the_inclusive_contact() {
    let mut input = input();
    let radius = libm::sqrt(325.0);
    let contact_angle = libm::atan2(10.0, 15.0).to_degrees();
    input
        .fit
        .vertebrae
        .iter_mut()
        .find(|vertebra| vertebra.level == Level::T12)
        .unwrap()
        .pose
        .centre[1] = radius;
    // The source includes touching. This made-up offset equals the
    // greatest support15cos(a)+10sin(a), attained only at atan2(10,15)
    // in this quadrant. First establish that the concrete physical pose
    // represents the inclusive equality before asking the flip search.
    let nominal = level_rules::nominal(&input).unwrap();
    assert_eq!(
        nominal
            .rules
            .iter()
            .find(|rule| rule.rule == LevelRule::LastTouched)
            .unwrap()
            .level,
        Ok(Level::L1)
    );
    let known = RuleFlip {
        measurement: Measurement::CoronalTilt,
        level: Some(Level::T12),
        region: None,
        change_value: contact_angle,
        change_in_sd: contact_angle,
        moves_to: Level::T12,
    };
    let witness = level_rules::apply_flip(&input, &known).unwrap();
    assert_eq!(
        level_rules::nominal(&witness)
            .unwrap()
            .rules
            .iter()
            .find(|rule| rule.rule == LevelRule::LastTouched)
            .unwrap()
            .level,
        Ok(Level::T12)
    );
    let returned = level_rules::flips::nominal_flip(
        &input,
        LevelRule::LastTouched,
        level_rules::FusionRegion::Structural,
    )
    .unwrap()
    .expect("the physical equality witnesses contact");
    assert_eq!(returned.measurement, Measurement::CoronalTilt);
    assert_eq!(returned.level, Some(Level::T12));
    // At the support maximum the angular slope is zero and support
    // loss is R*delta²/2. One ulp of support therefore gives angular
    // uncertainty of order sqrt(EPSILON), even though the pose witnesses
    // exact contact. This is a numerical oracle bound only; the body's
    // inclusive contact comparison receives no added tolerance.
    let stationary_angle_round_off = (4.0 * libm::sqrt(f64::EPSILON)).to_degrees();
    assert!(returned.change_value.abs() <= contact_angle);
    assert!(
        (returned.change_value.abs() - contact_angle).abs() < stationary_angle_round_off,
        "returned {returned:?}; isolated contact at {contact_angle} degrees"
    );
    let replay = level_rules::apply_flip(&input, &returned).unwrap();
    assert_eq!(
        level_rules::nominal(&replay)
            .unwrap()
            .rules
            .iter()
            .find(|rule| rule.rule == LevelRule::LastTouched)
            .unwrap()
            .level,
        Ok(Level::T12)
    );
    let smaller = RuleFlip {
        change_value: returned.change_value * 0.999,
        change_in_sd: returned.change_in_sd * 0.999,
        ..returned
    };
    let smaller = level_rules::apply_flip(&input, &smaller).unwrap();
    assert_eq!(
        level_rules::nominal(&smaller)
            .unwrap()
            .rules
            .iter()
            .find(|rule| rule.rule == LevelRule::LastTouched)
            .unwrap()
            .level,
        Ok(Level::L1)
    );
}
