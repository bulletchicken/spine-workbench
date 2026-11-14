//! Independent hand geometry for the landmark definitions in fusion-levels
//! sections 2a–2b. All vertebrae and outlines in this file are made up.

use forward_model::pedicles::pedicle_centres;
use forward_model::rotation::{rot_x, rot_z, rotation_vector_from_matrix};
use forward_model::vertebra_params::VertebraParams;
use implant_layout::curves::lumbar::PedicleOutlines;
use implant_layout::curves::{self, SpineReading};
use implant_layout::evidence::{Coordinate, PublishedValue, Table, tables};
use implant_layout::level_rules::{self, FusionRegion, RuleContext};
use shapes::implant_layout::{
    ChildFacts, CurveType, LayoutInput, LevelRule, LumbarModifier, NamingDoubt, Readable,
    SagittalModifier,
};
use shapes::levels::Level;
use shapes::made_up::VertebraPose;
use shapes::naming::Template;
use shapes::spine_fit::{
    FitAngles, FitRmsMm, FitStandardDeviations, FitStopReason, FittedVertebra, InferredFit,
    SpineFit,
};

fn next_standard_name(level: Level) -> Level {
    let names = Template::Standard.names();
    names[names.iter().position(|name| *name == level).unwrap() + 1]
}

fn scalar(group: &str, key: &str) -> f64 {
    let cell = tables()
        .lookup(
            Table::LevelRuleThresholds,
            &Coordinate {
                group: Some(group.into()),
                key: key.into(),
                ..Coordinate::default()
            },
        )
        .unwrap();
    let PublishedValue::Scalar { value } = cell.value else {
        panic!("expected a scalar threshold")
    };
    value
}

fn hand_spine() -> (LayoutInput, SpineReading, Vec<PedicleOutlines>) {
    let deviation = scalar("deviated-vertebra", "first-cephalad-deviation-min");
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
        Level::L1,
        Level::L2,
        Level::L3,
        Level::L4,
        Level::L5,
    ];
    let y = [
        25.0,
        15.0,
        0.0,
        -15.0,
        0.0,
        10.0,
        deviation + 4.0,
        deviation + 2.0,
        9.0,
        2.0,
        4.0,
        3.0,
        2.0,
        0.0,
    ];
    let tilts = [
        -15.0_f64, -12.0, -8.0, 0.0, 8.0, 12.0, 15.0, 10.0, 5.0, -2.0, -5.0, 0.0, 4.0, 5.0,
    ];
    let twists = [
        4.0_f64, 4.0, 4.0, 4.0, 4.0, 4.0, 4.0, 4.0, 1.0, -0.5, -4.0, -4.0, -4.0, -4.0,
    ];
    let vertebrae: Vec<_> = levels
        .iter()
        .enumerate()
        .map(|(i, &level)| FittedVertebra {
            level,
            pose: VertebraPose {
                level,
                centre: [0.0, y[i], 400.0 - i as f64 * 25.0],
                rotation: rotation_vector_from_matrix(
                    &(rot_x(tilts[i].to_radians()) * rot_z(twists[i].to_radians())),
                )
                .into(),
                height: 20.0,
                wedge_front: 0.0,
                wedge_side: 0.0,
                top_half_width: 10.0,
                top_half_depth: 8.0,
                bottom_half_width: 10.0,
                bottom_half_depth: 8.0,
            },
            angles: FitAngles {
                side_tilt_rad: tilts[i].to_radians(),
                forward_lean_rad: 0.0,
                twist_rad: twists[i].to_radians(),
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
        })
        .collect();
    let input = LayoutInput {
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
        seed: 13,
        pedicle_child_share: None,
    };
    // Geometry is measured from the composed mechanical poses. The
    // independent synthetic CSVL y=0 and flat bends are supplied directly
    // at this rule boundary, which accepts premeasured film readings.
    let mut reading = curves::measure(&input, &input.fit).unwrap();
    reading.csvl_y_mm = Ok(0.0);
    for (i, vertebra) in reading.vertebrae.iter_mut().enumerate() {
        vertebra.centre_line_offset_mm = Ok(y[i]);
    }
    for (i, curve) in reading.curves.iter_mut().enumerate() {
        curve.bending_cobb_deg = Ok(0.0);
        curve.bending_disc_below_deg = Ok(0.0);
        curve.structural = Ok(Some(i) == reading.major);
    }
    reading.classification.curve_type = Ok(CurveType::MainThoracic);
    reading.classification.lumbar_modifier = Ok(LumbarModifier::BetweenPedicles);
    reading.classification.sagittal_modifier = Ok(SagittalModifier::Normal);
    let outlines = input
        .fit
        .vertebrae
        .iter()
        .map(|vertebra| {
            let centres = pedicle_centres(&VertebraParams::from(&vertebra.pose));
            PedicleOutlines {
                level: vertebra.level,
                left_y_mm: [centres[0].y - 0.25, centres[0].y + 0.25],
                right_y_mm: [centres[1].y - 0.25, centres[1].y + 0.25],
            }
        })
        .collect();
    (input, reading, outlines)
}

fn context<'a>(
    input: &'a LayoutInput,
    reading: &'a SpineReading,
    outlines: &'a [PedicleOutlines],
) -> RuleContext<'a> {
    RuleContext {
        input,
        fit: &input.fit,
        reading,
        outlines,
        region: FusionRegion::Thoracic,
    }
}

#[test]
fn each_relative_landmark_follows_its_hand_worked_definition() {
    // LEV T10 -> EV+1 T11. The last twist crossing is T12(+1) to
    // L1(-0.5), whose nearer-zero member is L1: three below LEV, so
    // Suk gives NV-1=T12. CSVL y=0 first touches a TL/L body at T12
    // (centre9, half-width10); SV is L1 (centre2, local pedicles +/-7.8).
    // A gives SV-1=T12. At T12 the right outline is wholly
    // lateral to zero, making nSTV; at L1 its right pedicle is crossed
    // medially, so STV and nSTV+1 both give L1. Going up from the
    // sacrum, T11 is the first centre above the deviation threshold.
    let (input, reading, outlines) = hand_spine();
    let ctx = context(&input, &reading, &outlines);
    let cases = [
        (
            level_rules::end_plus_one::evaluate(&ctx).unwrap(),
            Level::T11,
        ),
        (
            level_rules::suk_neutral::evaluate(&ctx).unwrap(),
            Level::T12,
        ),
        (
            level_rules::last_touched::evaluate(&ctx).unwrap(),
            Level::T12,
        ),
        (
            level_rules::stable_minus_one::evaluate(&ctx).unwrap(),
            Level::T12,
        ),
        (
            level_rules::substantially_touched::evaluate(&ctx).unwrap(),
            Level::L1,
        ),
        (
            level_rules::deviated_vertebra::evaluate(&ctx).unwrap(),
            Level::T11,
        ),
        (
            level_rules::upper_end_vertebra::evaluate(&ctx).unwrap(),
            Level::T4,
        ),
    ];
    for (case, expected) in cases {
        assert_eq!(case.level, Ok(expected), "{:?}", case.rule);
        assert!(!case.inputs.is_empty(), "{:?}", case.rule);
        assert!(!case.sources.is_empty(), "{:?}", case.rule);
        assert!(
            case.sources
                .iter()
                .all(|key| tables().source(key).is_some())
        );
    }
}

#[test]
fn a_deviation_equal_to_the_threshold_is_skipped_on_the_cranial_walk() {
    let threshold = scalar("deviated-vertebra", "first-cephalad-deviation-min");
    for (offset, expected) in [
        (threshold - 0.25, Level::T11),
        (threshold, Level::T11),
        (threshold + 0.25, Level::L4),
    ] {
        let (mut input, mut reading, outlines) = hand_spine();
        let i = input
            .fit
            .vertebrae
            .iter()
            .position(|v| v.level == Level::L4)
            .unwrap();
        input.fit.vertebrae[i].pose.centre[1] = offset;
        reading.vertebrae[i].centre_mm[1] = offset;
        reading.vertebrae[i].centre_line_offset_mm = Ok(offset);
        let result =
            level_rules::deviated_vertebra::evaluate(&context(&input, &reading, &outlines))
                .unwrap();
        assert_eq!(result.level, Ok(expected));
    }
}

#[test]
fn the_stable_minus_one_rule_distinguishes_a_and_c() {
    let (input, mut reading, outlines) = hand_spine();
    // The same SV L1 is used; C stops there, A/B stop one above it.
    for (modifier, expected) in [
        (LumbarModifier::BetweenPedicles, Level::T12),
        (LumbarModifier::TouchesApex, Level::T12),
        (LumbarModifier::ClearOfApex, Level::L1),
    ] {
        reading.classification.lumbar_modifier = Ok(modifier);
        assert_eq!(
            level_rules::stable_minus_one::evaluate(&context(&input, &reading, &outlines))
                .unwrap()
                .level,
            Ok(expected)
        );
    }
}

#[test]
fn relabelling_relative_rules_moves_the_answer_and_preserves_their_values() {
    let (input, reading, outlines) = hand_spine();
    let before = level_rules::evaluate(&context(&input, &reading, &outlines)).unwrap();
    let mut renamed = input.clone();
    for v in &mut renamed.fit.vertebrae {
        v.level = next_standard_name(v.level);
        v.pose.level = v.level;
    }
    renamed.fit.origin_level = next_standard_name(renamed.fit.origin_level);
    let original_geometry = curves::measure(&input, &input.fit).unwrap();
    let renamed_geometry = curves::measure(&renamed, &renamed.fit).unwrap();
    for (a, b) in original_geometry
        .vertebrae
        .iter()
        .zip(&renamed_geometry.vertebrae)
    {
        assert_eq!(a.centre_mm, b.centre_mm);
        assert_eq!(a.upper_tilt_deg, b.upper_tilt_deg);
        assert_eq!(a.lower_tilt_deg, b.lower_tilt_deg);
        assert_eq!(a.twist_deg, b.twist_deg);
        assert_eq!(a.disc_below_deg, b.disc_below_deg);
    }
    for (a, b) in original_geometry
        .curves
        .iter()
        .zip(&renamed_geometry.curves)
    {
        assert_eq!(a.cobb_deg, b.cobb_deg);
        assert_eq!(a.signed_cobb_deg, b.signed_cobb_deg);
        assert_eq!(a.apical_deviation_mm, b.apical_deviation_mm);
        assert_eq!(
            (a.upper_index, a.lower_index, a.apex_index),
            (b.upper_index, b.lower_index, b.apex_index)
        );
    }
    let mut shifted = reading.clone();
    for v in &mut shifted.vertebrae {
        v.level = next_standard_name(v.level);
    }
    let changed_outlines: Vec<_> = outlines
        .iter()
        .map(|outline| PedicleOutlines {
            level: next_standard_name(outline.level),
            ..*outline
        })
        .collect();
    let after = level_rules::evaluate(&context(&renamed, &shifted, &changed_outlines)).unwrap();
    for a in before.iter().filter(|vote| {
        matches!(
            vote.rule,
            LevelRule::EndPlusOne
                | LevelRule::SukNeutral
                | LevelRule::LastTouched
                | LevelRule::StableMinusOne
                | LevelRule::SubstantiallyTouched
                | LevelRule::DeviatedVertebra
                | LevelRule::UpperEndVertebra
        )
    }) {
        let b = after.iter().find(|vote| vote.rule == a.rule).unwrap();
        assert_eq!(b.level, a.level.map(next_standard_name), "{:?}", a.rule);
        // A newly named T12 may add an earlier LTV-domain probe. Compare
        // the corresponding physical input rather than an audit-row count.
        for x in &a.inputs {
            let moved_level = x.level.map(next_standard_name);
            let y = b
                .inputs
                .iter()
                .find(|y| {
                    y.measurement == x.measurement
                        && y.level == moved_level
                        && y.region == x.region
                        && y.threshold == x.threshold
                })
                .unwrap();
            assert_eq!(x.value, y.value, "{:?} {:?}", a.rule, x.measurement);
            assert_eq!(x.sd, y.sd);
            assert_eq!(x.status, y.status);
        }
    }
}

#[test]
fn the_liv_index_is_a_pair_check_with_a_strict_zero_boundary() {
    let (input, reading, outlines) = hand_spine();
    // Both NV and SV are L1. LIV=T12 gives -1+-1=-2; L1 gives0;
    // L2 gives1+1=2. Only the negative index meets Chen's flag.
    let ctx = context(&input, &reading, &outlines);
    for (lower, flagged) in [(Level::T12, true), (Level::L1, false), (Level::L2, false)] {
        let result = level_rules::liv_index::check(&ctx, lower).unwrap();
        assert_eq!(result.rule, LevelRule::LivIndex);
        assert_eq!(
            result.flagged,
            Readable::Share {
                versions: u32::from(flagged)
            }
        );
        assert!(tables().source(&result.source).is_some());
    }
}

fn vote(
    rule: LevelRule,
    end: shapes::implant_layout::RuleEnd,
    level: Level,
    versions: u32,
) -> shapes::implant_layout::RuleVote {
    shapes::implant_layout::RuleVote {
        rule,
        end,
        counts: shapes::implant_layout::RuleCounts {
            levels: vec![shapes::spine_readings::LevelCount { level, versions }],
            unread: Vec::new(),
        },
        inputs: Vec::new(),
        flip: None,
        sources: Vec::new(),
    }
}

fn rule_reading(
    rule: LevelRule,
    end: shapes::implant_layout::RuleEnd,
    level: Result<Level, shapes::implant_layout::CannotTellCode>,
) -> level_rules::RuleReading {
    level_rules::RuleReading {
        rule,
        end,
        level,
        inputs: Vec::new(),
        sources: Vec::new(),
    }
}

#[test]
fn pair_candidates_are_the_union_of_modes_and_their_fitted_neighbours() {
    use shapes::implant_layout::{CannotTellCode, RuleEnd, UnreadCount};
    let (input, _, _) = hand_spine();
    let mut votes = vec![
        vote(LevelRule::UpperEndVertebra, RuleEnd::Upper, Level::T4, 5),
        vote(LevelRule::EndPlusOne, RuleEnd::Lower, Level::L1, 5),
        vote(LevelRule::LastTouched, RuleEnd::Lower, Level::L1, 5),
        vote(
            LevelRule::SubstantiallyTouched,
            RuleEnd::Lower,
            Level::T12,
            5,
        ),
    ];
    let mut unread = vote(LevelRule::SukNeutral, RuleEnd::Lower, Level::L5, 5);
    unread.counts.levels.clear();
    unread.counts.unread = vec![UnreadCount {
        reason: CannotTellCode::TwistNotMeasured,
        versions: 5,
    }];
    votes.push(unread);
    // On this fitted chain T12's successor is L1. T13 is absent and
    // cannot create another candidate or increase a level distance.
    assert_eq!(
        implant_layout::pairs::candidates(&input.fit, &votes, RuleEnd::Upper),
        vec![Level::T4, Level::T5]
    );
    assert_eq!(
        implant_layout::pairs::candidates(&input.fit, &votes, RuleEnd::Lower),
        vec![Level::T11, Level::T12, Level::L1, Level::L2]
    );
    let version = vec![
        rule_reading(LevelRule::UpperEndVertebra, RuleEnd::Upper, Ok(Level::T4)),
        rule_reading(LevelRule::EndPlusOne, RuleEnd::Lower, Ok(Level::L1)),
        rule_reading(LevelRule::LastTouched, RuleEnd::Lower, Ok(Level::L1)),
        rule_reading(
            LevelRule::SubstantiallyTouched,
            RuleEnd::Lower,
            Ok(Level::T12),
        ),
        rule_reading(
            LevelRule::SukNeutral,
            RuleEnd::Lower,
            Err(CannotTellCode::TwistNotMeasured),
        ),
    ];
    let pairs = implant_layout::pairs::from_rules(&input.fit, &votes, &vec![version; 5]);
    assert_eq!(pairs.len(), 8);
    let pair = pairs
        .iter()
        .find(|pair| pair.upper == Level::T4 && pair.lower == Level::L1)
        .unwrap();
    assert_eq!(pair.versions_holding, 5);
    assert_eq!(pair.rules_at.len(), 3);
    assert!(pair.rules_at.contains(&LevelRule::UpperEndVertebra));
    assert!(pair.rules_at.contains(&LevelRule::EndPlusOne));
    assert!(pair.rules_at.contains(&LevelRule::LastTouched));
    assert_eq!(pair.rules_near, vec![LevelRule::SubstantiallyTouched]);
    assert!(pair.rules_against.is_empty());
    assert_eq!(pair.free_segments_below, 5);
    assert_eq!(
        implant_layout::pairs::first_pair(&pairs).map(|p| (p.upper, p.lower)),
        Some((Level::T4, Level::L1))
    );
}

#[test]
fn a_version_without_a_readable_end_has_no_modal_pair() {
    use shapes::implant_layout::{CannotTellCode, RuleEnd};
    let mut rules = vec![
        rule_reading(LevelRule::UpperEndVertebra, RuleEnd::Upper, Ok(Level::T5)),
        rule_reading(LevelRule::Uev5C, RuleEnd::Upper, Ok(Level::T4)),
        rule_reading(LevelRule::EndPlusOne, RuleEnd::Lower, Ok(Level::T12)),
        rule_reading(LevelRule::LastTouched, RuleEnd::Lower, Ok(Level::L1)),
    ];
    // Equal upper counts take the cranial end; equal lower counts take
    // the caudal end. An absent rule contributes no hidden zero vote.
    assert_eq!(
        implant_layout::pairs::modal_pair(&rules),
        Some((Level::T4, Level::L1))
    );
    for rule in &mut rules {
        if rule.end == RuleEnd::Lower {
            rule.level = Err(CannotTellCode::NoSacrum);
        }
    }
    assert_eq!(implant_layout::pairs::modal_pair(&rules), None);
}

fn pair(
    upper: Level,
    lower: Level,
    rules_at: Vec<LevelRule>,
    versions_holding: u32,
) -> shapes::implant_layout::LevelPair {
    shapes::implant_layout::LevelPair {
        upper,
        lower,
        versions_holding,
        rules_at,
        rules_near: Vec::new(),
        rules_against: Vec::new(),
        stop_short: Vec::new(),
        checks: Vec::new(),
        free_segments_below: 0,
    }
}

#[test]
fn the_first_pairs_ties_follow_exact_rules_then_versions_then_the_lower_end() {
    let rules = vec![LevelRule::EndPlusOne, LevelRule::LastTouched];
    let mut pairs = vec![
        pair(Level::T4, Level::T12, rules.clone(), 7),
        pair(Level::T4, Level::L1, rules, 6),
    ];
    assert_eq!(
        implant_layout::pairs::first_pair(&pairs).unwrap().lower,
        Level::T12
    );
    pairs[1].versions_holding = 7;
    assert_eq!(
        implant_layout::pairs::first_pair(&pairs).unwrap().lower,
        Level::L1
    );
    pairs[0].rules_at.push(LevelRule::SubstantiallyTouched);
    pairs[1].versions_holding = 8;
    assert_eq!(
        implant_layout::pairs::first_pair(&pairs).unwrap().lower,
        Level::T12
    );
}

#[test]
fn a_missing_t9_never_makes_t10_the_level_one_below_t8() {
    use shapes::implant_layout::CannotTellCode;
    let (mut input, _, _) = hand_spine();
    for vertebra in &mut input.fit.vertebrae {
        if vertebra.level >= Level::T8 {
            vertebra.angles.side_tilt_rad = 15.0_f64.to_radians();
            vertebra.angles.twist_rad = 0.0;
            vertebra.pose.rotation = [vertebra.angles.side_tilt_rad, 0.0, 0.0];
        }
    }
    input
        .fit
        .vertebrae
        .retain(|vertebra| vertebra.level != Level::T9);
    let reading = curves::measure(&input, &input.fit).unwrap();
    let major = &reading.curves[reading.major.unwrap()];
    // Tilts rise from -15 at T4 to +15 at T8, then plateau. The
    // endplate detector retains the first equal maximum, so LEV=T8.
    // EV+1 means T9 on the standard anatomical naming sequence. T10
    // is present but is two levels below; its geometry cannot stand in.
    assert_eq!(reading.vertebrae[major.lower_index].level, Level::T8);
    assert_eq!(
        level_rules::end_plus_one::evaluate(&context(&input, &reading, &[]))
            .unwrap()
            .level,
        Err(CannotTellCode::NoLevelMeetsTheRule)
    );
}
