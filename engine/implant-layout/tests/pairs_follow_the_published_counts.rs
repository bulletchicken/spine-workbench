//! Independent pair arithmetic and selective-fusion boundaries.

use implant_layout::curves;
use implant_layout::evidence::{Coordinate, PublishedValue, Table, tables};
use implant_layout::level_rules::selective::{CriterionReading, SelectiveVersion, summarise};
use implant_layout::level_rules::{FusionRegion, LevelReadings, RegionReadings, RuleReading};
use implant_layout::{pairs, readings};
use shapes::implant_layout::{
    CannotTellCode, ChildFacts, LayoutInput, LevelRule, Measurement, NamingDoubt, Readable,
    RuleCounts, RuleEnd, RuleVote, UnreadCount,
};
use shapes::levels::Level;
use shapes::made_up::VertebraPose;
use shapes::spine_fit::{
    FitAngles, FitRmsMm, FitStandardDeviations, FitStopReason, FittedVertebra, InferredFit,
    SpineFit,
};
use shapes::spine_readings::LevelCount;

fn threshold(group: &str, key: &str) -> (f64, String) {
    let cell = tables()
        .lookup(
            Table::LevelRuleThresholds,
            &Coordinate {
                key: key.into(),
                group: Some(group.into()),
                ..Coordinate::default()
            },
        )
        .unwrap();
    let PublishedValue::Scalar { value } = cell.value else {
        panic!("scalar rule threshold")
    };
    (value, cell.source.clone())
}

fn input() -> LayoutInput {
    LayoutInput {
        fit: SpineFit {
            vertebrae: Level::MEASURED
                .into_iter()
                .enumerate()
                .map(|(i, level)| FittedVertebra {
                    level,
                    pose: VertebraPose {
                        level,
                        centre: [0.0, 0.0, 500.0 - 25.0 * i as f64],
                        rotation: [0.0; 3],
                        height: 20.0,
                        wedge_front: 0.0,
                        wedge_side: 0.0,
                        top_half_width: 15.0,
                        bottom_half_width: 15.0,
                        top_half_depth: 10.0,
                        bottom_half_depth: 10.0,
                    },
                    angles: FitAngles {
                        side_tilt_rad: 0.0,
                        forward_lean_rad: 0.0,
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
        front: None,
        bend_left: None,
        bend_right: None,
        child: ChildFacts {
            age_years: Some(14.0),
            weight_kg: None,
            activity: None,
        },
        naming: NamingDoubt::AsRead,
        versions: 10,
        seed: 1,
        pedicle_child_share: None,
    }
}

fn vote(rule: LevelRule, end: RuleEnd, level: Level) -> RuleVote {
    RuleVote {
        rule,
        end,
        counts: RuleCounts {
            levels: vec![LevelCount {
                level,
                versions: 10,
            }],
            unread: Vec::new(),
        },
        inputs: Vec::new(),
        flip: None,
        sources: Vec::new(),
    }
}

fn row(rule: LevelRule, end: RuleEnd, level: Level) -> RuleReading {
    RuleReading {
        rule,
        end,
        level: Ok(level),
        inputs: Vec::new(),
        sources: Vec::new(),
    }
}

fn votes() -> Vec<RuleVote> {
    vec![
        vote(LevelRule::UpperEndVertebra, RuleEnd::Upper, Level::T10),
        vote(LevelRule::EndPlusOne, RuleEnd::Lower, Level::T12),
        vote(LevelRule::LastTouched, RuleEnd::Lower, Level::L1),
    ]
}

fn rows(at_t12: u32) -> Vec<Vec<RuleReading>> {
    (0..10)
        .map(|i| {
            vec![
                row(LevelRule::UpperEndVertebra, RuleEnd::Upper, Level::T10),
                row(
                    LevelRule::EndPlusOne,
                    RuleEnd::Lower,
                    if i < at_t12 { Level::T12 } else { Level::L1 },
                ),
            ]
        })
        .collect()
}

#[test]
fn anatomical_candidates_cross_twelve_to_one_and_stop_at_the_last_lumbar_body() {
    // The union of mode +/-one is computed by hand, without using Level::below
    // (which includes T13). Every member is already inside the pruning radius2.
    let input = input();
    assert_eq!(
        pairs::candidates(&input.fit, &votes(), RuleEnd::Lower),
        vec![Level::T11, Level::T12, Level::L1, Level::L2]
    );
    let mut sparse = votes();
    sparse[2] = vote(LevelRule::LastTouched, RuleEnd::Lower, Level::L4);
    assert_eq!(
        pairs::candidates(&input.fit, &sparse, RuleEnd::Lower),
        vec![
            Level::T11,
            Level::T12,
            Level::L1,
            Level::L3,
            Level::L4,
            Level::L5
        ]
    );
    assert_eq!(
        pairs::candidates(
            &input.fit,
            &[vote(LevelRule::EndPlusOne, RuleEnd::Lower, Level::L5)],
            RuleEnd::Lower
        ),
        vec![Level::L4, Level::L5]
    );
}

#[test]
fn exact_rules_then_holding_versions_then_the_lower_lower_end_break_ties() {
    let input = input();
    // Equal exact rules: six versions at T12 beat four at L1.
    let pairs = pairs::from_rules(&input.fit, &votes(), &rows(6));
    let first = pairs::first_pair(&pairs).unwrap();
    assert_eq!(
        (first.upper, first.lower, first.versions_holding),
        (Level::T10, Level::T12, 6)
    );
    // Five/five now leaves the caudal end as the tie-break: L1.
    let pairs = pairs::from_rules(&input.fit, &votes(), &rows(5));
    assert_eq!(pairs::first_pair(&pairs).unwrap().lower, Level::L1);
    // A third exact rule at T12 beats nine holding versions at L1.
    let mut more_exact = votes();
    more_exact.push(vote(LevelRule::SukNeutral, RuleEnd::Lower, Level::T12));
    let pairs = pairs::from_rules(&input.fit, &more_exact, &rows(1));
    let first = pairs::first_pair(&pairs).unwrap();
    assert_eq!(first.lower, Level::T12);
    assert_eq!(first.versions_holding, 1);
    assert!(first.rules_near.contains(&LevelRule::LastTouched));
    assert_eq!(first.free_segments_below, 6);
}

#[test]
fn unread_rules_supply_no_candidates_and_no_modal_votes() {
    let input = input();
    let mut with_unread = votes();
    let mut unread = vote(LevelRule::LenkeShoulder, RuleEnd::Upper, Level::T1);
    unread.counts.levels.clear();
    unread.counts.unread.push(UnreadCount {
        reason: CannotTellCode::ShouldersOutsideTheModel,
        versions: 10,
    });
    with_unread.push(unread);
    let mut versions = rows(5);
    for version in &mut versions {
        version.push(RuleReading {
            rule: LevelRule::LenkeShoulder,
            end: RuleEnd::Upper,
            level: Err(CannotTellCode::ShouldersOutsideTheModel),
            inputs: Vec::new(),
            sources: Vec::new(),
        });
    }
    assert_eq!(
        pairs::from_rules(&input.fit, &with_unread, &versions),
        pairs::from_rules(&input.fit, &votes(), &rows(5))
    );
}

#[test]
fn exactly_six_of_ten_holding_versions_settles_the_ends() {
    // Ends are unsettled only when strictly fewer than 0.6*N versions hold.
    // Hand rows avoid a random draw deciding whether equality is reached.
    // Geometry is irrelevant to this count.
    let input = input();
    let measured = curves::measure(&input, &input.fit).unwrap();
    for holding in [5, 6] {
        let version_rules = rows(holding);
        let counts = votes();
        let readings = LevelReadings {
            nominal: measured.clone(),
            measured: vec![measured.clone(); 10],
            versions: readings::versions(&input.fit, 1, 10, 0..10).unwrap(),
            nominal_outlines: Vec::new(),
            outlines: vec![Vec::new(); 10],
            nominal_rules: Vec::new(),
            version_rules: version_rules.clone(),
            votes: counts.clone(),
            selective: Vec::new(),
            selective_versions: Vec::new(),
            regions: vec![RegionReadings {
                region: FusionRegion::Structural,
                nominal_rules: Vec::new(),
                version_rules,
                votes: counts,
            }],
            cannot_tell: Vec::new(),
        };
        let output = pairs::build(&input, &readings).unwrap();
        assert_eq!(
            output
                .cannot_tell
                .iter()
                .any(|code| code.code == CannotTellCode::EndsUnsettled),
            holding == 5
        );
    }
}

#[test]
fn missing_criteria_never_contribute_met_counts_and_both_regions_include_the_boundaries() {
    // The review says strictly greater than its published ratio threshold:
    // equality fails. Missing is a separate result and never a met count.
    let (ratio, source) = threshold("selective-thoracic", "cobb-ratio-min");
    let mut criterion = CriterionReading {
        measurement: Measurement::CobbRatio,
        region: None,
        threshold: ratio,
        holds_below: false,
        value: Ok(ratio),
        source,
        project_inference: false,
    };
    for (step, expected) in [(-0.001, false), (0.0, false), (0.001, true)] {
        criterion.value = Ok(ratio + step);
        assert_eq!(criterion.holds(), Ok(expected));
    }
    criterion.value = Err(CannotTellCode::TwistNotMeasured);
    for met in [1, 2, 8, 9] {
        let versions: Vec<_> = (0..10)
            .map(|i| SelectiveVersion {
                rule: LevelRule::SelectiveThoracic,
                criteria: vec![criterion.clone()],
                all_hold: Ok(i < met),
            })
            .collect();
        let result = summarise(&versions);
        assert_eq!(
            result.criteria[0].holds,
            Readable::NotReadable {
                reason: CannotTellCode::TwistNotMeasured
            }
        );
        assert_eq!(result.all_hold, Readable::Share { versions: met });
        assert_eq!(result.both_regions_offered, met == 2 || met == 8);
    }
}

#[test]
fn corrected_lumbar_bending_reads_the_thoracic_curve_and_omits_missing_twist() {
    use implant_layout::curves::Curve;
    use implant_layout::level_rules::{RuleContext, selective_lumbar};
    use shapes::film::FilmKind;
    use shapes::made_up::CurveRegion;
    use shapes::spine_readings::BodySide;
    let input = input();
    let mut reading = curves::measure(&input, &input.fit).unwrap();
    // Independent example: set the lumbar/thoracic Cobb ratio
    // half a unit above its published threshold and the thoracic Cobb
    // below its printed limit. The thoracic bend crosses its own limit;
    // a much larger lumbar bend must never stand in for that reading.
    // No C7 plumb line and absent twist make those two criteria unread,
    // leaving the readable Cobb ratio, thoracic Cobb and thoracic bend.
    let (ratio_min, _) = threshold("selective-lumbar", "cobb-ratio-min");
    let (bend_max, _) = threshold("selective-lumbar", "tll-bending-cobb-max");
    let (thoracic_max, _) = threshold("selective-lumbar", "thoracic-cobb-max");
    let thoracic_cobb = thoracic_max * 0.75;
    let lumbar_cobb = thoracic_cobb * (ratio_min + 0.5);
    let thoracic = Curve {
        upper_index: 3,
        lower_index: 10,
        apex_index: 7,
        region: Some(CurveRegion::MainThoracic),
        convex: BodySide::Left,
        cobb_deg: thoracic_cobb,
        signed_cobb_deg: -thoracic_cobb,
        apical_deviation_mm: 20.0,
        bending_cobb_deg: Ok(bend_max - 0.001),
        straightening_bend: Ok(FilmKind::BendLeft),
        bending_disc_below_deg: Ok(0.0),
        structural: Ok(false),
    };
    let lumbar = Curve {
        upper_index: 10,
        lower_index: 15,
        apex_index: 13,
        region: Some(CurveRegion::ThoracolumbarLumbar),
        cobb_deg: lumbar_cobb,
        signed_cobb_deg: -lumbar_cobb,
        bending_cobb_deg: Ok(bend_max * 3.0 + 5.0),
        structural: Ok(true),
        ..thoracic.clone()
    };
    reading.curves = vec![thoracic, lumbar];
    reading.major = Some(1);
    reading.vertebrae[7].twist_deg = Err(CannotTellCode::TwistNotMeasured);
    reading.vertebrae[13].twist_deg = Err(CannotTellCode::TwistNotMeasured);
    for (step, expected) in [(-0.001, true), (0.0, false), (0.001, false)] {
        let bend = bend_max + step;
        reading.curves[0].bending_cobb_deg = Ok(bend);
        let ctx = RuleContext {
            input: &input,
            fit: &input.fit,
            reading: &reading,
            outlines: &[],
            region: FusionRegion::Lumbar,
        };
        let result = selective_lumbar::evaluate(&ctx).unwrap();
        assert_eq!(result.criteria[0].threshold, ratio_min);
        assert_eq!(result.criteria[3].threshold, bend_max);
        assert_eq!(result.criteria[4].threshold, thoracic_max);
        assert_eq!(result.criteria[3].value, Ok(bend));
        assert!(result.criteria[3].project_inference);
        assert_eq!(result.criteria[3].holds(), Ok(expected));
        assert_eq!(
            result.criteria[2].value,
            Err(CannotTellCode::TwistNotMeasured)
        );
        assert_eq!(result.all_hold, Ok(expected));
        let summary = summarise(&[result]);
        assert_eq!(
            summary.criteria[2].holds,
            Readable::NotReadable {
                reason: CannotTellCode::TwistNotMeasured
            }
        );
    }
}
