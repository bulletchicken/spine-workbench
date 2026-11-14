//! Independent boundaries, look triggers and end-to-end synthetic checks.

use super::*;
use crate::patterns::{Pattern, apply};
use crate::readings::readings;
use shapes::implant_layout::{ChildFacts, PedicleEstimate};
use shapes::made_up::VertebraPose;
use shapes::spine_fit::{
    FitAngles, FitRmsMm, FitStandardDeviations, FitStopReason, FittedVertebra, InferredFit,
    SpineFit,
};

fn zero_band() -> Band {
    Band {
        p10: 0.0,
        p50: 0.0,
        p90: 0.0,
    }
}
fn look_input() -> LookInputs {
    LookInputs {
        level: Level::T6,
        side: BodySide::Right,
        position: CurvePosition::OUTSIDE,
        right_thoracic_curve: false,
        missing_table: false,
        no_diameter: false,
        outer_narrow_share: 0.0,
        inner_narrow_share: 0.0,
        convergence: zero_band(),
        max_abs_twist_deg: 0.0,
        shorter_than_grid: false,
        nearest_chord_age: false,
    }
}
fn has(input: LookInputs, code: LookCode) -> bool {
    look_codes(input).unwrap().0.contains(&code)
}

#[test]
fn each_anchor_look_has_a_trigger_and_an_absent_case() {
    let base = look_input();
    for code in [
        LookCode::ConcaveApexNeedsCt,
        LookCode::MayBeTooNarrow,
        LookCode::NearTheAorta,
        LookCode::DirectionUnreadable,
        LookCode::ShorterThanTables,
        LookCode::NoTableForThisAge,
        LookCode::ChordFromNearestAge,
    ] {
        assert!(!has(base, code), "{code:?}");
        let mut trigger = base;
        match code {
            LookCode::ConcaveApexNeedsCt => {
                trigger.position = CurvePosition {
                    curve_side: CurveSide::Concave,
                    apex_distance: 1.0,
                    neutral_distance: None,
                }
            }
            LookCode::MayBeTooNarrow => trigger.no_diameter = true,
            LookCode::NearTheAorta => {
                trigger.level = Level::T4;
                trigger.side = BodySide::Left;
                trigger.right_thoracic_curve = true;
            }
            LookCode::DirectionUnreadable => {
                trigger.max_abs_twist_deg = required_scalar(
                    Table::PlacementChecks,
                    "pedicle-method-visibility-limit",
                    "cerny-pedicle-shadow",
                )
                .unwrap()
                .0 + 1.0
            }
            LookCode::ShorterThanTables => trigger.shorter_than_grid = true,
            LookCode::NoTableForThisAge => trigger.missing_table = true,
            LookCode::ChordFromNearestAge => trigger.nearest_chord_age = true,
        }
        assert!(has(trigger, code), "{code:?}");
    }
}

#[test]
fn apical_aorta_and_direction_boundaries_do_not_grow_beyond_their_rules() {
    let mut input = look_input();
    input.position = CurvePosition {
        curve_side: CurveSide::Concave,
        apex_distance: 1.0,
        neutral_distance: Some(4.0),
    };
    for level in [Level::T3, Level::T9] {
        input.level = level;
        assert!(has(input, LookCode::ConcaveApexNeedsCt));
    }
    for level in [Level::T2, Level::T10] {
        input.level = level;
        assert!(!has(input, LookCode::ConcaveApexNeedsCt));
    }
    input.level = Level::T6;
    input.position.apex_distance = 1.01;
    assert!(!has(input, LookCode::ConcaveApexNeedsCt));
    input = look_input();
    input.side = BodySide::Left;
    input.level = Level::T4;
    assert!(!has(input, LookCode::NearTheAorta));
    input.right_thoracic_curve = true;
    assert!(has(input, LookCode::NearTheAorta));
    input.side = BodySide::Right;
    assert!(!has(input, LookCode::NearTheAorta));
    input = look_input();
    let limit = required_scalar(
        Table::PlacementChecks,
        "pedicle-method-visibility-limit",
        "cerny-pedicle-shadow",
    )
    .unwrap()
    .0;
    input.max_abs_twist_deg = limit;
    input.convergence.p90 = limit;
    assert!(!has(input, LookCode::DirectionUnreadable));
    input.convergence.p90 = limit + 1e-6;
    assert!(has(input, LookCode::DirectionUnreadable));
    input = look_input();
    input.outer_narrow_share = sizing::DEFAULT_ALPHA;
    input.inner_narrow_share = sizing::DEFAULT_ALPHA;
    assert!(!has(input, LookCode::MayBeTooNarrow));
    input.inner_narrow_share += 1e-6;
    assert!(has(input, LookCode::MayBeTooNarrow));
}

fn fixture_input() -> LayoutInput {
    let vertebrae = Level::MEASURED
        .iter()
        .enumerate()
        .map(|(i, &level)| {
            let u = i as f64 / 16.0;
            let tilt = (u - 0.5) * 0.5;
            let lean = 0.3 * (1.0 - u);
            let rotation = forward_model::rotation::rotation_vector_from_matrix(
                &(forward_model::rotation::rot_x(tilt) * forward_model::rotation::rot_y(lean)),
            );
            let pose = VertebraPose {
                level,
                centre: [0.0, -20.0 * 4.0 * u * (1.0 - u), 500.0 - 25.0 * i as f64],
                rotation: rotation.into(),
                height: 20.0,
                wedge_front: 0.0,
                wedge_side: 0.0,
                top_half_width: 15.0,
                top_half_depth: 12.0,
                bottom_half_width: 15.0,
                bottom_half_depth: 12.0,
            };
            FittedVertebra {
                level,
                pose,
                angles: FitAngles {
                    side_tilt_rad: tilt,
                    forward_lean_rad: lean,
                    twist_rad: 0.0,
                },
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
        front: None,
        bend_left: None,
        bend_right: None,
        child: ChildFacts {
            age_years: Some(14.0),
            weight_kg: None,
            activity: None,
        },
        naming: NamingDoubt::AsRead,
        versions: 8,
        seed: 11,
        pedicle_child_share: None,
    }
}

#[test]
fn typical_scale_changes_position_status_but_never_table_sizes() {
    let measured = fixture_input();
    let mut typical = measured.clone();
    typical.fit.lengths = shapes::spine_fit::FitLengths::TypicalSize;
    let measured_readings = readings(&measured, 0..measured.versions).unwrap();
    let typical_readings = readings(&typical, 0..typical.versions).unwrap();
    for side in [BodySide::Left, BodySide::Right] {
        let measured_anchor = anchor(
            &measured,
            &measured_readings.versions.versions,
            &measured_readings.measured,
            Level::T9,
            side,
        )
        .unwrap();
        let typical_anchor = anchor(
            &typical,
            &typical_readings.versions.versions,
            &typical_readings.measured,
            Level::T9,
            side,
        )
        .unwrap();
        assert_eq!(measured_anchor.entry_status, ValueStatus::FromTemplate);
        assert_eq!(typical_anchor.entry_status, ValueStatus::Typical);
        assert_eq!(measured_anchor.entry_mm, typical_anchor.entry_mm);
        assert_eq!(measured_anchor.diameter_mm, typical_anchor.diameter_mm);
        assert_eq!(measured_anchor.length_mm, typical_anchor.length_mm);
        assert_eq!(measured_anchor.diameter_status, ValueStatus::FromTables);
        assert_eq!(typical_anchor.diameter_status, ValueStatus::FromTables);
        assert_eq!(measured_anchor.length_status, ValueStatus::FromTables);
        assert_eq!(typical_anchor.length_status, ValueStatus::FromTables);
    }
    assert_eq!(
        crate::rods::geometry_status(&typical.fit),
        ValueStatus::Typical
    );
    let levels: Vec<_> = typical.fit.vertebrae.iter().map(|v| v.level).collect();
    let target =
        crate::rods::target_spine(&typical.fit, &levels, &typical_readings.nominal).unwrap();
    assert_eq!(target.geometry_status, ValueStatus::Typical);
}

#[test]
fn a_complete_synthetic_proposal_keeps_bands_sources_and_apical_flags() {
    let input = fixture_input();
    let all = readings(&input, 0..input.versions).unwrap();
    let curve = all.nominal.major.map(|i| &all.nominal.curves[i]).unwrap();
    for index in curve.apex_index - 1..=curve.apex_index + 1 {
        let level = input.fit.vertebrae[index].level;
        let a = anchor(
            &input,
            &all.versions.versions,
            &all.measured,
            level,
            BodySide::Left,
        )
        .unwrap();
        if (Level::T3..=Level::T9).contains(&level) {
            assert!(a.looks.contains(&LookCode::ConcaveApexNeedsCt));
        }
        assert_eq!(a.entry_status, ValueStatus::FromTemplate);
        assert_eq!(a.pedicle.status, ValueStatus::FromTables);
        assert!(a.pedicle.outer_width_mm.is_some());
        assert!(
            a.sources
                .iter()
                .all(|key| crate::evidence::tables().source(key).is_some())
        );
        let (k, _) = sizing::default_width_ratio().unwrap();
        let widths: Vec<_> = all
            .versions
            .versions
            .iter()
            .zip(&all.measured)
            .map(|(version, reading)| {
                pedicles::drawn_outer_width(
                    level,
                    BodySide::Left,
                    input.child.age_years,
                    pedicles::position_in_curve(reading, level, BodySide::Left),
                    version,
                    &NamedSampler::new(u64::from(input.seed), input.versions).unwrap(),
                    input.pedicle_child_share,
                )
                .unwrap()
                .unwrap()
            })
            .collect();
        if let Some(d) = a.diameter_mm {
            assert!(
                widths.iter().filter(|&&w| w < d / k).count() as f64 / widths.len() as f64
                    <= sizing::DEFAULT_ALPHA
            );
        }
        assert_eq!(
            a,
            anchor(
                &input,
                &all.versions.versions,
                &all.measured,
                level,
                BodySide::Left
            )
            .unwrap()
        );
    }
}

#[test]
fn a_childs_lumbar_table_is_absent_and_cannot_make_a_screw() {
    let mut input = fixture_input();
    input.child.age_years = Some(10.0);
    let all = readings(&input, 0..input.versions).unwrap();
    let a = anchor(
        &input,
        &all.versions.versions,
        &all.measured,
        Level::L1,
        BodySide::Left,
    )
    .unwrap();
    assert_eq!(a.pedicle.outer_width_mm, None);
    assert_eq!(a.diameter_mm, None);
    assert_eq!(a.length_mm, None);
    assert!(a.looks.contains(&LookCode::NoTableForThisAge));
    assert_ne!(a.kind, AnchorKind::Screw);
    let older = fixture_input();
    let all = readings(&older, 0..older.versions).unwrap();
    let a = anchor(
        &older,
        &all.versions.versions,
        &all.measured,
        Level::L1,
        BodySide::Left,
    )
    .unwrap();
    assert!(a.pedicle.outer_width_mm.is_some());
    assert!(!a.looks.contains(&LookCode::NoTableForThisAge));
}

fn candidate(level: Level, side: BodySide) -> AnchorProposal {
    AnchorProposal {
        level,
        side,
        kind: AnchorKind::Screw,
        entry_mm: [0.0; 3],
        entry_status: ValueStatus::FromTemplate,
        direction_unit: [1.0, 0.0, 0.0],
        direction_status: ValueStatus::FromTables,
        medial_angle: None,
        medial_angles_beside: Vec::new(),
        convergence_deg: zero_band(),
        diameter_mm: Some(1.0),
        diameter_also_seen: Vec::new(),
        diameter_status: ValueStatus::FromTables,
        width_ratio: None,
        diameters_beside: Vec::new(),
        screw_to_pedicle: None,
        chord_mm: None,
        chord_share: None,
        length_mm: Some(1.0),
        length_also_seen: Vec::new(),
        length_status: ValueStatus::FromTables,
        usual_length_mm: None,
        pedicle: PedicleEstimate {
            outer_width_mm: None,
            inner_width_mm: None,
            versions_too_narrow: 0,
            versions_inner_under_1_mm: 0,
            curve_side: CurveSide::Outside,
            status: ValueStatus::FromTables,
        },
        looks: Vec::new(),
        sources: Vec::new(),
    }
}
fn bilateral(levels: &[Level]) -> Vec<AnchorProposal> {
    levels
        .iter()
        .flat_map(|&l| [candidate(l, BodySide::Left), candidate(l, BodySide::Right)])
        .collect()
}

#[test]
fn patterns_preserve_their_order_substitute_missing_sizes_and_obey_the_density_cap() {
    let levels = &Level::MEASURED[2..14];
    let original = bilateral(levels);
    let mut all = original.clone();
    all[0].diameter_mm = None;
    all[3].diameter_mm = None;
    apply(&mut all, levels, Some(Level::T8), Pattern::AllPedicles).unwrap();
    assert_eq!(all[0].kind, AnchorKind::Hook);
    assert_eq!(all[3].kind, AnchorKind::None);
    assert!(
        all.iter()
            .enumerate()
            .all(|(i, a)| a.level == original[i].level && a.side == original[i].side)
    );
    let mut hooks = original.clone();
    apply(&mut hooks, levels, None, Pattern::HooksAtTop).unwrap();
    assert!(
        hooks[..2].iter().all(|a| a.kind == AnchorKind::Hook
            && a.diameter_mm.is_none()
            && a.length_mm.is_none())
    );
    assert!(hooks[2..].iter().all(|a| a.kind == AnchorKind::Screw));
    let mut key = original.clone();
    apply(&mut key, levels, Some(Level::T8), Pattern::KeyLevels).unwrap();
    let density = central(cell(Table::AnchorPatterns, "density", "mimo-low-max").unwrap()).unwrap();
    let count = key.iter().filter(|a| a.kind != AnchorKind::None).count();
    assert!(count as f64 <= density * levels.len() as f64);
    assert!(key[..2].iter().all(|a| a.kind == AnchorKind::Screw));
    assert!(
        key[key.len() - 2..]
            .iter()
            .all(|a| a.kind == AnchorKind::Screw)
    );
    for a in &key {
        assert!(
            a.sources
                .iter()
                .all(|key| crate::evidence::tables().source(key).is_some())
        );
    }
    let short = &levels[..2];
    let mut key = bilateral(short);
    apply(&mut key, short, None, Pattern::KeyLevels).unwrap();
    assert!(
        key.iter().filter(|a| a.kind != AnchorKind::None).count() as f64
            <= density * short.len() as f64
    );
}

#[test]
fn context_codes_change_with_the_inputs_that_supply_them() {
    let mut input = fixture_input();
    let codes = context_looks(&input, Some(0));
    for code in [
        CannotTellCode::EndsUnsettled,
        CannotTellCode::NoSacrum,
        CannotTellCode::NoBendingFilms,
    ] {
        assert!(codes.iter().any(|c| c.code == code));
    }
    for code in [
        CannotTellCode::NamesMayBeOneOff,
        CannotTellCode::StandingBends,
        CannotTellCode::TwistNotMeasured,
    ] {
        assert!(!codes.iter().any(|c| c.code == code));
    }
    input.naming = NamingDoubt::MayBeOneOff;
    input.fit.vertebrae[0].inferred.twist = true;
    let codes = context_looks(&input, Some(input.versions));
    assert!(
        !codes
            .iter()
            .any(|c| c.code == CannotTellCode::EndsUnsettled)
    );
    assert!(
        codes
            .iter()
            .any(|c| c.code == CannotTellCode::NamesMayBeOneOff)
    );
    assert!(
        codes
            .iter()
            .any(|c| c.code == CannotTellCode::TwistNotMeasured)
    );
}

#[test]
fn supplied_sacrum_and_bends_remove_the_absence_codes_and_add_standing_bends() {
    use shapes::confidence::{Confidence, ConfidenceBand};
    use shapes::film::FilmKind;
    use shapes::film_geometry::{FilmGeometry, ImageRight};
    use shapes::implant_layout::LayoutFilm;
    use shapes::landmarks::{Landmarks, SacrumTop};
    use shapes::marks::{FrontMarks, Mark, MarkSource, PedicleMark};
    use shapes::naming::{ChosenBy, LevelNaming, Template};
    use shapes::pixels::Px;
    use shapes::vertebra::{FilmMarks, VertebraMarks, VertebraOnFilm};
    let mark = |x, y| Mark::auto(Px::new(x, y));
    let mut input = fixture_input();
    let front = LayoutFilm {
        size: [100, 200],
        geometry: FilmGeometry {
            mm_per_px: Some(1.0),
            image_right: Some(ImageRight::PatientLeft),
            ..FilmGeometry::UNKNOWN
        },
        marks: FilmMarks {
            film: FilmKind::Front,
            vertebrae: vec![VertebraOnFilm {
                id: 0,
                level: Level::L5,
                marks: VertebraMarks::Front {
                    points: FrontMarks {
                        sup_l: mark(60.0, 140.0),
                        sup_r: mark(40.0, 140.0),
                        inf_l: mark(60.0, 160.0),
                        inf_r: mark(40.0, 160.0),
                        ped_l: PedicleMark::NotVisible {
                            source: MarkSource::Auto,
                        },
                        ped_r: PedicleMark::NotVisible {
                            source: MarkSource::Auto,
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
            }],
            landmarks: Landmarks {
                sacrum_top: Some(SacrumTop::Front {
                    sac_l: mark(60.0, 180.0),
                    sac_r: mark(40.0, 180.0),
                }),
                ..Landmarks::NONE
            },
            naming: LevelNaming {
                template: Template::Standard,
                chosen_by: ChosenBy::Engine,
                alternatives: Vec::new(),
                from_top: None,
                from_bottom: None,
                flags: Vec::new(),
                l5_confirmed_on: None,
            },
            named_for: ImageRight::PatientLeft,
        },
    };
    let mut left = front.clone();
    left.marks.film = FilmKind::BendLeft;
    left.marks.vertebrae.clear();
    let mut right = left.clone();
    right.marks.film = FilmKind::BendRight;
    input.front = Some(front);
    input.bend_left = Some(left);
    input.bend_right = Some(right);
    let codes = context_looks(&input, Some(input.versions));
    assert!(!codes.iter().any(|c| c.code == CannotTellCode::NoSacrum));
    assert!(
        !codes
            .iter()
            .any(|c| c.code == CannotTellCode::NoBendingFilms)
    );
    assert!(
        codes
            .iter()
            .any(|c| c.code == CannotTellCode::StandingBends)
    );
}

#[test]
fn widths_use_regional_walls_and_truncate_at_the_project_floor() {
    let input = fixture_input();
    let mut version = crate::readings::version(&input.fit, 1, 8, 0).unwrap();
    version.z_child = -1e6;
    let sampler = NamedSampler::new(1, 8).unwrap();
    for (level, side, group) in [
        (Level::T3, BodySide::Left, "chiu-left-t1-t3"),
        (Level::T4, BodySide::Left, "chiu-left-t4-t7"),
        (Level::T8, BodySide::Left, "chiu-left-t8-l5"),
        (Level::T3, BodySide::Right, "chiu-right-t1-t3"),
        (Level::T4, BodySide::Right, "chiu-right-t4-t10"),
        (Level::T11, BodySide::Right, "chiu-right-t11-l5"),
    ] {
        let medial =
            central(cell(Table::InnerWalls, "medial-wall-thickness", group).unwrap()).unwrap();
        let lateral = central(
            cell(
                Table::InnerWalls,
                "lateral-wall-thickness",
                if side == BodySide::Left {
                    "chiu-left"
                } else {
                    "chiu-right"
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(pedicles::wall_sum(level, side).unwrap().0, medial + lateral);
        let w = pedicles::draw_width(
            level,
            side,
            None,
            CurvePosition::OUTSIDE,
            &version,
            &sampler,
            Some(1.0),
        )
        .unwrap()
        .unwrap();
        assert_eq!(w.outer_mm, pedicles::MIN_OUTER_MM);
        assert_eq!(w.inner_mm, 0.0);
        let estimate = pedicles::estimate(&[Some(w)], CurveSide::Outside);
        assert_eq!(estimate.versions_too_narrow, 1);
        assert_eq!(estimate.versions_inner_under_1_mm, 1);
    }
    assert!(
        pedicles::width_prior(Level::T10, Some(6.0), CurveSide::Outside).is_none(),
        "the malformed female width does not borrow a mate or level"
    );
    assert!(pedicles::width_prior(Level::T10, Some(10.0), CurveSide::Outside).is_some());
}

#[test]
fn side_factor_interpolation_respects_apical_and_neutral_zones() {
    // Arbitrary synthetic ratios: plateau1 through apex+/-1, then a
    // straight line to ratio3 at neutral+/-1 (neutral distance5).
    assert_eq!(pedicles::side_factor(1.0, 3.0, 0.0, 5.0), 1.0);
    assert_eq!(pedicles::side_factor(1.0, 3.0, 1.0, 5.0), 1.0);
    assert_eq!(pedicles::side_factor(1.0, 3.0, 2.5, 5.0), 2.0);
    assert_eq!(pedicles::side_factor(1.0, 3.0, 4.0, 5.0), 3.0);
    assert_eq!(pedicles::side_factor(1.0, 3.0, 7.0, 5.0), 3.0);
    assert_eq!(pedicles::side_factor(1.0, 3.0, 2.0, 1.0), 3.0);
}

#[test]
fn supplied_levels_build_two_contoured_rods_with_sourced_geometry() {
    let input = fixture_input();
    let all = readings(&input, 0..input.versions).unwrap();
    let levels = &Level::MEASURED[..];
    let mut candidates = anchors(&input, &all.versions.versions, &all.measured, levels).unwrap();
    apply(
        &mut candidates,
        levels,
        Some(Level::T9),
        Pattern::AllPedicles,
    )
    .unwrap();
    let rods = crate::rods::propose(
        &input.fit,
        levels,
        &all.nominal,
        &candidates,
        &crate::rods::default_choice().unwrap(),
    )
    .unwrap();
    assert_eq!(rods.len(), 2);
    assert_eq!(rods[0].side, BodySide::Left);
    assert_eq!(rods[1].side, BodySide::Right);
    for rod in rods {
        assert!(rod.length_mm > 0.0);
        assert!(rod.points_mm.len() > 2);
        assert!(
            rod.sources
                .iter()
                .all(|key| crate::evidence::tables().source(key).is_some())
        );
    }
}

#[test]
fn an_unfitted_intervening_level_does_not_shorten_the_distance_from_the_apex() {
    let input = fixture_input();
    let mut reading = crate::curves::measure(&input, &input.fit).unwrap();
    reading
        .vertebrae
        .retain(|v| [Level::T5, Level::T7, Level::T9].contains(&v.level));
    let mut curve = reading.curves[0].clone();
    curve.upper_index = 0;
    curve.lower_index = 2;
    curve.apex_index = 1;
    curve.convex = BodySide::Right;
    reading.curves = vec![curve];
    reading.major = Some(0);
    let position = pedicles::position_in_curve(&reading, Level::T9, BodySide::Left);
    assert_eq!(position.apex_distance, 2.0);
    let mut look = look_input();
    look.level = Level::T9;
    look.position = position;
    assert!(!has(look, LookCode::ConcaveApexNeedsCt));
}
