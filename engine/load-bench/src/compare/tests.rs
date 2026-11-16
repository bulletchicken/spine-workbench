use super::*;
use shapes::implant_layout::{
    AnchorKind, AnchorProposal, CoronalLine, CurveSide, LayoutChange, PedicleEstimate, RodMaterial,
    RodProposal, ValueStatus,
};
use shapes::levels::Level;
use shapes::spine_readings::{Band, BodySide};

fn anchor(side: BodySide) -> AnchorProposal {
    AnchorProposal {
        wider_than_pedicle: Vec::new(),
        screw_here: None,
        diameter_sources: Vec::new(),
        length_sources: Vec::new(),
        direction_sources: Vec::new(),
        look_sources: Vec::new(),
        level: Level::T8,
        side,
        kind: AnchorKind::Screw,
        entry_mm: [0.0; 3],
        entry_status: ValueStatus::FromTables,
        direction_unit: [1.0, 0.0, 0.0],
        direction_status: ValueStatus::FromTables,
        medial_angle: None,
        medial_angles_beside: vec![],
        convergence_deg: Band {
            p10: 0.0,
            p50: 0.0,
            p90: 0.0,
        },
        diameter_mm: Some(5.0),
        diameter_also_seen: vec![],
        diameter_status: ValueStatus::FromTables,
        width_ratio: None,
        diameters_beside: vec![],
        screw_to_pedicle: None,
        chord_mm: Some(40.0),
        chord_share: None,
        length_mm: Some(30.0),
        length_also_seen: vec![],
        length_status: ValueStatus::FromTables,
        usual_length_mm: None,
        pedicle: PedicleEstimate {
            sources: Vec::new(),
            outer_width_mm: None,
            inner_width_mm: None,
            versions_too_narrow: 0,
            versions_inner_under_1_mm: 0,
            curve_side: CurveSide::Outside,
            status: ValueStatus::Missing,
        },
        looks: vec![],
        sources: vec![],
    }
}

fn layout(id: &str) -> ImplantLayout {
    ImplantLayout {
        id: id.to_string(),
        change: LayoutChange::ChangedByHand {},
        upper: Level::T8,
        lower: Level::T8,
        anchors: vec![anchor(BodySide::Left), anchor(BodySide::Right)],
        rods: vec![],
        released_levels: vec![],
    }
}

fn sourced_layout() -> ImplantLayout {
    let mut layout = layout("first");
    layout.lower = Level::L1;
    layout.anchors[1].level = Level::L1;
    layout.anchors[1].side = BodySide::Left;
    layout.anchors[1].kind = AnchorKind::Hook;
    layout.rods.push(RodProposal {
        standing_points_mm: Vec::new(),
        side: BodySide::Left,
        curve_side: CurveSide::Outside,
        material: RodMaterial::CobaltChrome,
        diameter_mm: 5.5,
        target_kyphosis_deg: 25.0,
        target_kyphosis_status: ValueStatus::Typical,
        coronal_line: CoronalLine::L5Vertical,
        overbend_deg: 0.0,
        head_offset_mm: 10.0,
        length_mm: 0.0,
        geometry_status: ValueStatus::FromTemplate,
        points_mm: vec![],
        sources: vec![],
    });
    layout
}

#[test]
fn loaded_sources_keep_the_rows_behind_each_number() {
    let evidence = crate::evidence::Evidence::read().unwrap();
    let layout = sourced_layout();
    // The seated angles use springs, ribs and rod modulus. Gravity,
    // preload and anchor capacity do not enter those angles.
    assert_eq!(
        sources::estimate(&evidence, &layout, BenchStop::RodsIn, Measure::MainCobb),
        [
            "mannen-2018",
            "prasad-2017",
            "project-assumptions",
            "tachi-2021"
        ]
    );
    // Standing adds preload and gravity; the T8 screw and L1 hook read
    // distinct capacity rows, both with the bone-factor assumption.
    assert_eq!(
        sources::estimate(
            &evidence,
            &layout,
            BenchStop::Standing,
            Measure::HeaviestAnchorDemand,
        ),
        [
            "gardner-morse-2004",
            "lehman-2003",
            "liljenqvist-2001",
            "mannen-2018",
            "prasad-2017",
            "project-assumptions",
            "tachi-2021",
            "winter-via-keenan-2015",
        ]
    );
    // The swing adds walking, ultimate strength, contour loss and the
    // construct run-out; annual step counts do not enter its ratio.
    assert_eq!(
        sources::estimate(
            &evidence,
            &layout,
            BenchStop::YearOfWalking,
            Measure::HeaviestRodSwing,
        ),
        [
            "gardner-morse-2004",
            "mannen-2018",
            "prasad-2017",
            "project-assumptions",
            "rohlmann-2002",
            "rohlmann-2014-gait",
            "slivka-2013",
            "tachi-2021",
            "vieweg-2023",
            "winter-via-keenan-2015",
        ]
    );

    let mut run = run("first", 0, 20.0, 4.0);
    for stop in &mut run.stops {
        stop.rod_span_use = vec![0.5];
    }
    let result = compare(&BenchCompareInput {
        layouts: vec![layout],
        batches: vec![BenchBatch {
            middle: Vec::new(),
            main_curve: None,
            version_from: 0,
            version_count: 1,
            runs: vec![run],
        }],
    })
    .unwrap();
    // The screw's share uses force, so the hook's capacity is absent.
    // Its denominator also reads bending and walking loads at other stops.
    assert_eq!(
        result.layouts[0].stops[2].parts[0].sources,
        [
            "gardner-morse-2004",
            "lehman-2003",
            "mannen-2018",
            "prasad-2017",
            "project-assumptions",
            "rohlmann-2002",
            "rohlmann-2014-gait",
            "tachi-2021",
            "winter-via-keenan-2015",
        ]
    );
}

fn run(id: &str, version: u32, cobb: f64, force: f64) -> BenchRun {
    BenchRun {
        layout_id: id.to_string(),
        version,
        settled: true,
        stiff_rod_share: 0.0,
        draws: VersionDraws {
            stiffness_scale: 1.0,
            rib_factors: [1.0; 3],
            bone_factor: 1.0,
            weight_kg: 40.0,
            steps_per_day: 8000.0,
            walking_multiple: 1.5,
            rod_yield_mpa: 900.0,
            preload_factor: 1.0,
            slice_offset_mm: 25.0,
            contour_knock_down: 0.8,
        },
        stops: STOPS
            .into_iter()
            .map(|stop| StopValues {
                stop,
                anchor_force_n: vec![force, force / 2.0],
                anchor_demand: vec![force / 10.0, force / 20.0],
                rod_span_use: vec![],
                junction_above: None,
                junction_below: None,
                main_cobb_deg: cobb,
                kyphosis_deg: 25.0,
                apical_twist_deg: 5.0,
                poses: None,
                rod_swing: None,
                cycles_per_year: if stop == BenchStop::YearOfWalking {
                    Some(2_920_000.0)
                } else {
                    None
                },
            })
            .collect(),
    }
}

fn input(runs: Vec<BenchRun>, count: u32) -> BenchCompareInput {
    BenchCompareInput {
        layouts: vec![layout("first"), layout("other")],
        batches: vec![BenchBatch {
            middle: Vec::new(),
            main_curve: None,
            version_from: 0,
            version_count: count,
            runs,
        }],
    }
}

fn cobb_count(result: &BenchResult) -> &PairedCount {
    result.layouts[1]
        .counts
        .iter()
        .find(|count| count.stop == BenchStop::RodsIn && count.measure == Measure::MainCobb)
        .unwrap()
}

#[test]
fn pairing_uses_version_numbers_and_changes_only_when_versions_are_mismatched() {
    let mut runs = Vec::new();
    for version in 0..4 {
        runs.push(run("first", version, 10.0 * version as f64, 1.0));
        runs.push(run("other", version, 10.0 * version as f64 + 3.0, 1.0));
    }
    let original = compare(&input(runs.clone(), 4)).unwrap();
    assert_eq!(
        (
            cobb_count(&original).lower,
            cobb_count(&original).higher,
            cobb_count(&original).ties
        ),
        (0, 4, 0)
    );
    runs.reverse();
    assert_eq!(compare(&input(runs.clone(), 4)).unwrap(), original);
    for run in runs.iter_mut().filter(|run| run.layout_id == "other") {
        run.version = (run.version + 3) % 4;
    }
    let mismatched = compare(&input(runs, 4)).unwrap();
    assert_eq!(
        (
            cobb_count(&mismatched).lower,
            cobb_count(&mismatched).higher,
            cobb_count(&mismatched).ties
        ),
        (1, 3, 0)
    );
}

#[test]
fn the_clear_count_rounds_up_three_quarters_of_the_paired_dots() {
    let mut runs = Vec::new();
    for version in 0..6 {
        runs.push(run("first", version, 20.0, 1.0));
        let mut other = run("other", version, 23.0, 1.0);
        other.settled = version != 5;
        runs.push(other);
    }
    let result = compare(&input(runs, 6)).unwrap();
    let count = cobb_count(&result);
    // Five paired dots need four votes; the unpaired sixth does not count.
    assert_eq!(count.order.len(), 5);
    assert_eq!(count.clear_at, 4);
}

#[test]
fn global_shares_have_one_denominator_and_strength_counts_keep_their_units() {
    let mut other_run = run("other", 0, 20.0, 16.0);
    other_run.stops[3].anchor_force_n = vec![32.0, 16.0];
    let result = compare(&input(vec![run("first", 0, 20.0, 4.0), other_run], 1)).unwrap();
    let first = &result.layouts[0].stops[1];
    let other = &result.layouts[1].stops[1];
    assert_eq!(first.parts[0].share.p50, 0.125);
    assert_eq!(first.parts[1].share.p50, 0.0625);
    assert_eq!(other.parts[0].share.p50, 0.5);
    assert_eq!(other.parts[0].versions_past_strength, 1);
    assert_eq!(other.parts[1].versions_past_strength, 0);
    assert!(first.parts.iter().all(|part| part.steady));
}

#[test]
fn part_sources_include_other_settled_placements_only_on_used_versions() {
    let mut input = input(
        vec![run("first", 0, 20.0, 4.0), run("other", 0, 20.0, 16.0)],
        1,
    );
    input.layouts[1].released_levels = vec![Level::T8];
    let has_release_source = |result: &BenchResult| {
        result.layouts[0].stops[1].parts[0]
            .sources
            .iter()
            .any(|source| source == "wang-2017")
    };
    assert!(has_release_source(&compare(&input).unwrap()));
    input.batches[0].runs[1].settled = false;
    assert!(!has_release_source(&compare(&input).unwrap()));

    // The other placement's extra settled version cannot contribute a
    // denominator when this placement has no settled numerator there.
    input.batches[0].version_count = 2;
    let mut unsettled = run("first", 1, 20.0, 4.0);
    unsettled.settled = false;
    input.batches[0]
        .runs
        .extend([unsettled, run("other", 1, 20.0, 16.0)]);
    assert!(!has_release_source(&compare(&input).unwrap()));
}

#[test]
fn a_missing_junction_cannot_borrow_sources_from_its_version() {
    let mut input = input(
        vec![
            run("first", 0, 20.0, 4.0),
            run("first", 1, 20.0, 4.0),
            run("other", 0, 20.0, 16.0),
            run("other", 1, 20.0, 16.0),
        ],
        2,
    );
    input.layouts[1].released_levels = vec![Level::T8];
    let runs = &mut input.batches[0].runs;
    runs[2].settled = false;
    for (index, run) in runs.iter_mut().enumerate() {
        run.stops[2].junction_above = (index != 1).then_some(0.5);
        run.stops[2].junction_below = Some(0.5);
    }
    let result = compare(&input).unwrap();
    let parts = &result.layouts[0].stops[2].parts;
    for (which, includes_release) in [(JunctionSide::Above, false), (JunctionSide::Below, true)] {
        let part = parts
            .iter()
            .find(|part| part.part == PartRef::Junction { which })
            .unwrap();
        assert_eq!(
            part.sources.iter().any(|source| source == "wang-2017"),
            includes_release,
        );
    }
}

#[test]
fn margins_and_tags_follow_their_numeric_thresholds() {
    let narrow = Band {
        p10: 9.0,
        p50: 10.0,
        p90: 11.0,
    };
    let broad = Band {
        p10: 1.0,
        p50: 10.0,
        p90: 19.0,
    };
    assert_eq!(
        tag(Measure::MainCobb, narrow, false, false),
        EstimateTag::SetByTheFilms
    );
    assert_eq!(
        tag(Measure::MainCobb, broad, true, false),
        EstimateTag::ComparisonOnly
    );
    assert_eq!(
        tag(Measure::MainCobb, broad, false, false),
        EstimateTag::Assumed
    );
    assert_eq!(
        tag(Measure::HeaviestAnchorForce, narrow, true, false),
        EstimateTag::Assumed
    );
    assert_eq!(
        tag(Measure::Kyphosis, narrow, true, false),
        EstimateTag::Assumed
    );
    assert_eq!(
        tag(Measure::JunctionAbove, narrow, true, true),
        EstimateTag::Assumed
    );
    let result = compare(&input(
        vec![run("first", 0, 20.0, 10.0), run("other", 0, 22.0, 11.0)],
        1,
    ))
    .unwrap();
    assert_eq!(cobb_count(&result).ties, 1);
    let force = result.layouts[1]
        .counts
        .iter()
        .find(|count| count.measure == Measure::HeaviestAnchorForce)
        .unwrap();
    assert_eq!(force.ties, 1);
    assert_eq!(CLEAR_COUNT_32, 24);
}

#[test]
fn unknowns_follow_tied_rank_correlation_and_unsettled_runs_do_not_vote() {
    let mut runs = Vec::new();
    for (version, scale) in [2.0, 1.0, 3.0].into_iter().enumerate() {
        for id in ["first", "other"] {
            let mut value = run(id, version as u32, 20.0, 10.0 * scale);
            value.draws.stiffness_scale = scale;
            value.settled = version != 1;
            runs.push(value);
        }
    }
    let result = compare(&input(runs, 3)).unwrap();
    assert_eq!(result.layouts[0].versions_unsettled, 1);
    let force = result.layouts[0].stops[1]
        .estimates
        .iter()
        .find(|value| value.measure == Measure::HeaviestAnchorForce)
        .unwrap();
    assert_eq!(force.mostly_from, Some(SampledUnknown::StiffnessScale));
    assert_eq!(
        force.band,
        Band {
            p10: 21.0,
            p50: 25.0,
            p90: 29.0
        }
    );
    assert_eq!(cobb_count(&result).ties, 2);
    assert!(result.layouts[0].counts.is_empty());
}

#[test]
fn duplicate_or_unshared_versions_are_refused() {
    assert!(compare(&input(vec![run("first", 0, 20.0, 1.0)], 1)).is_err());
    assert!(
        compare(&input(
            vec![
                run("first", 0, 20.0, 1.0),
                run("other", 0, 20.0, 1.0),
                run("other", 0, 20.0, 1.0)
            ],
            1
        ))
        .is_err()
    );
}

#[test]
fn dot_order_follows_the_paired_difference_and_ghosts_are_actual_versions() {
    let mut runs = Vec::new();
    for (version, scale) in [2.0, 1.0, 3.0].into_iter().enumerate() {
        for id in ["first", "other"] {
            let mut value = run(id, version as u32, 20.0, 1.0);
            value.draws.stiffness_scale = scale;
            if id == "other" {
                value.stops[1].main_cobb_deg += 2.0 * scale - 1.0;
            }
            runs.push(value);
        }
    }
    let result = compare(&input(runs, 3)).unwrap();
    let count = cobb_count(&result);
    assert_eq!(count.sorted_by, Some(SampledUnknown::StiffnessScale));
    assert_eq!(count.order, vec![1, 0, 2]);
    assert_eq!((count.lower, count.higher, count.ties), (0, 2, 1));
    assert_eq!(
        count.outcomes,
        vec![PairOutcome::Tie, PairOutcome::Higher, PairOutcome::Higher]
    );
    let mut runs = Vec::new();
    for version in 0..11 {
        let mut value = run("first", version, (10 - version) as f64, 1.0);
        value.stops[0].poses = Some(vec![shapes::made_up::VertebraPose {
            level: Level::T8,
            centre: [version as f64, 0.0, 0.0],
            rotation: [0.0; 3],
            height: 20.0,
            wedge_front: 0.0,
            wedge_side: 0.0,
            top_half_width: 10.0,
            top_half_depth: 8.0,
            bottom_half_width: 10.0,
            bottom_half_depth: 8.0,
        }]);
        runs.push(value);
    }
    let result = compare(&BenchCompareInput {
        layouts: vec![layout("first")],
        batches: vec![BenchBatch {
            middle: Vec::new(),
            main_curve: None,
            version_from: 0,
            version_count: 11,
            runs,
        }],
    })
    .unwrap();
    let ghosts = result.layouts[0].stops[0].ghosts.as_ref().unwrap();
    assert_eq!(ghosts[0][0].centre[0], 9.0);
    assert_eq!(ghosts[1][0].centre[0], 1.0);
    assert!(result.layouts[0].stops[2].ghosts.is_none());
}

#[test]
fn an_empty_pedicle_does_not_lower_the_median_occupied_anchor_force() {
    let mut layout = layout("first");
    let mut empty = anchor(BodySide::Right);
    empty.kind = AnchorKind::None;
    layout.anchors.push(empty);
    let mut value = run("first", 0, 20.0, 10.0).stops.remove(1);
    value.anchor_force_n = vec![10.0, 0.0, 0.0];
    // Occupied forces are 10 and 0, whose median is 5; the empty pedicle
    // must not change that median to zero.
    assert_eq!(
        measure(&value, &layout, Measure::HeaviestAnchorOverMedian),
        Some(2.0)
    );
}

#[test]
fn comparison_only_needs_twenty_nine_of_thirty_two_signed_pairs() {
    let make = |positive: u32| {
        let mut runs = Vec::new();
        for version in 0..32 {
            let baseline = version as f64 + 10.0;
            let difference = if version < positive { 3.0 } else { -3.0 };
            runs.push(run("first", version, baseline, 1.0));
            runs.push(run("other", version, baseline + difference, 1.0));
        }
        compare(&input(runs, 32)).unwrap()
    };
    let tag_for = |result: &BenchResult| {
        result.layouts[1].stops[1]
            .estimates
            .iter()
            .find(|estimate| estimate.measure == Measure::MainCobb)
            .unwrap()
            .tag
    };
    assert_eq!(tag_for(&make(29)), EstimateTag::ComparisonOnly);
    assert_eq!(tag_for(&make(28)), EstimateTag::Assumed);
}

#[test]
fn changed_draws_and_geometry_are_refused_before_pairing() {
    let mut changed = run("other", 0, 20.0, 1.0);
    changed.draws.stiffness_scale = 2.0;
    assert_eq!(
        compare(&input(vec![run("first", 0, 20.0, 1.0), changed], 1)),
        Err(refused("draws-not-shared"))
    );
    let mut first = run("first", 0, 20.0, 1.0);
    first.stops[0].poses = Some(vec![]);
    let mut changed = run("other", 0, 20.0, 1.0);
    changed.stops[0].poses = Some(vec![shapes::made_up::VertebraPose {
        level: Level::T8,
        centre: [0.0; 3],
        rotation: [0.0; 3],
        height: 20.0,
        wedge_front: 0.0,
        wedge_side: 0.0,
        top_half_width: 10.0,
        top_half_depth: 8.0,
        bottom_half_width: 10.0,
        bottom_half_depth: 8.0,
    }]);
    assert_eq!(
        compare(&input(vec![first, changed], 1)),
        Err(refused("geometry-not-shared"))
    );
}

#[test]
fn yield_draws_match_for_the_same_material_and_can_differ_between_materials() {
    use shapes::implant_layout::{CoronalLine, RodMaterial, RodProposal};
    let rod = RodProposal {
        standing_points_mm: Vec::new(),
        side: BodySide::Left,
        curve_side: CurveSide::Outside,
        material: RodMaterial::CobaltChrome,
        diameter_mm: 5.5,
        target_kyphosis_deg: 25.0,
        target_kyphosis_status: ValueStatus::Typical,
        coronal_line: CoronalLine::L5Vertical,
        overbend_deg: 0.0,
        head_offset_mm: 10.0,
        length_mm: 0.0,
        geometry_status: ValueStatus::FromTemplate,
        points_mm: vec![],
        sources: vec![],
    };
    let mut changed = run("other", 0, 20.0, 1.0);
    changed.draws.rod_yield_mpa += 1.0;
    let mut input = input(vec![run("first", 0, 20.0, 1.0), changed], 1);
    for layout in &mut input.layouts {
        layout.rods.push(rod.clone());
    }
    assert_eq!(
        compare(&input),
        Err(refused("rod-strength-draws-not-shared"))
    );
    input.layouts[1].rods[0].material = RodMaterial::TitaniumAlloy;
    assert!(compare(&input).is_ok());
}

#[test]
fn a_missing_rod_comparison_does_not_become_a_known_peak() {
    let layout = layout("first");
    let mut value = run("first", 0, 20.0, 1.0).stops.remove(4);
    value.rod_swing = Some(vec![
        RodSwing {
            side: BodySide::Left,
            ratio: Some(0.5),
        },
        RodSwing {
            side: BodySide::Right,
            ratio: None,
        },
    ]);
    assert_eq!(measure(&value, &layout, Measure::HeaviestRodSwing), None);
    let mut computed = run("first", 0, 20.0, 1.0);
    computed.stops[4] = value;
    let result = compare(&BenchCompareInput {
        layouts: vec![layout],
        batches: vec![BenchBatch {
            middle: Vec::new(),
            main_curve: None,
            version_from: 0,
            version_count: 1,
            runs: vec![computed],
        }],
    })
    .unwrap();
    assert!(
        result.layouts[0].stops[4]
            .parts
            .iter()
            .all(|part| !matches!(part.part, PartRef::Rod { .. }))
    );
}

#[test]
fn missing_solves_cannot_set_a_narrow_estimate_or_a_steady_share() {
    let mut missing = run("first", 1, 20.0, 1.0);
    missing.settled = false;
    let result = compare(&BenchCompareInput {
        layouts: vec![layout("first")],
        batches: vec![BenchBatch {
            middle: Vec::new(),
            main_curve: None,
            version_from: 0,
            version_count: 2,
            runs: vec![run("first", 0, 20.0, 1.0), missing],
        }],
    })
    .unwrap();
    let estimate = result.layouts[0].stops[1]
        .estimates
        .iter()
        .find(|e| e.measure == Measure::MainCobb)
        .unwrap();
    assert_eq!(estimate.tag, EstimateTag::Assumed);
    assert!(
        result.layouts[0].stops[1]
            .parts
            .iter()
            .all(|part| !part.steady)
    );
}

#[test]
fn anchor_demand_uses_settled_ratios_and_before_has_only_shape_estimates() {
    let mut missing = run("first", 2, 20.0, 9990.0);
    missing.settled = false;
    let result = compare(&BenchCompareInput {
        layouts: vec![layout("first")],
        batches: vec![BenchBatch {
            middle: Vec::new(),
            main_curve: None,
            version_from: 0,
            version_count: 3,
            runs: vec![
                run("first", 0, 20.0, 10.0),
                run("first", 1, 20.0, 30.0),
                missing,
            ],
        }],
    })
    .unwrap();
    let stops = &result.layouts[0].stops;
    assert_eq!(
        stops[0]
            .estimates
            .iter()
            .map(|e| e.measure)
            .collect::<Vec<_>>(),
        vec![Measure::MainCobb, Measure::Kyphosis, Measure::ApicalTwist]
    );
    assert!(stops[0].parts.is_empty());
    let demand = stops[1].parts[0].demand.unwrap();
    assert_eq!(
        demand,
        Band {
            p10: 1.2,
            p50: 2.0,
            p90: 2.8
        }
    );
    assert_eq!(
        stops[1].parts[1].demand.unwrap(),
        Band {
            p10: 0.6,
            p50: 1.0,
            p90: 1.4
        }
    );
}

#[test]
fn constant_and_uncorrelated_values_have_no_drawn_driver() {
    let mut runs = Vec::new();
    // Centred force ranks [−1, 1, 1, −1] are orthogonal to
    // stiffness ranks [−1.5, −0.5, 0.5, 1.5]. Other draws are constant.
    for (version, force) in [10.0, 20.0, 20.0, 10.0].into_iter().enumerate() {
        for id in ["first", "other"] {
            let mut value = run(id, version as u32, 20.0, force);
            value.draws.stiffness_scale = version as f64 + 1.0;
            runs.push(value);
        }
    }
    let result = compare(&input(runs, 4)).unwrap();
    assert!(
        result
            .layouts
            .iter()
            .flat_map(|l| &l.stops)
            .flat_map(|s| &s.estimates)
            .all(|estimate| estimate.mostly_from.is_none())
    );
    for count in &result.layouts[1].counts {
        assert_eq!(count.sorted_by, None);
        assert_eq!(count.order, vec![0, 1, 2, 3]);
        assert_eq!(count.outcomes, vec![PairOutcome::Tie; 4]);
    }
}

#[test]
fn outcomes_keep_their_versions_when_dot_order_changes() {
    let mut runs = Vec::new();
    for (version, (scale, difference)) in [(3.0, -3.0), (1.0, 2.0), (2.0, 3.0)]
        .into_iter()
        .enumerate()
    {
        for id in ["first", "other"] {
            let mut value = run(id, version as u32, 20.0, 1.0);
            value.draws.stiffness_scale = scale;
            if id == "other" {
                value.stops[1].main_cobb_deg += difference;
            }
            runs.push(value);
        }
    }
    let result = compare(&input(runs, 3)).unwrap();
    let count = cobb_count(&result);
    assert_eq!(count.order, vec![1, 2, 0]);
    assert_eq!(
        count.outcomes,
        vec![PairOutcome::Tie, PairOutcome::Higher, PairOutcome::Lower]
    );
    for count in &result.layouts[1].counts {
        assert_eq!(count.outcomes.len(), count.order.len());
        for (outcome, total) in [
            (PairOutcome::Lower, count.lower),
            (PairOutcome::Higher, count.higher),
            (PairOutcome::Tie, count.ties),
        ] {
            assert_eq!(
                count.outcomes.iter().filter(|o| **o == outcome).count(),
                total as usize
            );
        }
        assert_eq!(count.clear_at, 3);
    }
}

#[test]
fn drawing_runs_copy_only_to_shape_stops_and_never_change_summaries() {
    let mut input = input(
        vec![run("first", 0, 20.0, 4.0), run("other", 0, 15.0, 3.0)],
        1,
    );
    let baseline = compare(&input).unwrap();
    let pose = shapes::made_up::VertebraPose {
        level: Level::T8,
        centre: [0.0, 0.0, 100.0],
        rotation: [0.0; 3],
        height: 20.0,
        wedge_front: 0.0,
        wedge_side: 0.0,
        top_half_width: 12.0,
        top_half_depth: 8.0,
        bottom_half_width: 12.0,
        bottom_half_depth: 8.0,
    };
    let middle = MiddleRun {
        layout_id: "first".into(),
        before: vec![pose],
        rods_in: Some(vec![shapes::made_up::VertebraPose {
            centre: [1.0, 0.0, 100.0],
            ..pose
        }]),
        rods: Some(vec![RodLine {
            side: BodySide::Left,
            points_mm: vec![[1.0, 2.0, 100.0], [1.0, 2.0, 80.0]],
        }]),
    };
    input.batches[0].middle = vec![middle.clone()];
    input.batches[0].main_curve = Some(MainCurve {
        upper: Level::T5,
        apex: Level::T8,
        lower: Level::T12,
    });
    let mut with = compare(&input).unwrap();
    assert_eq!(with.main_curve, input.batches[0].main_curve);
    assert_eq!(with.layouts[0].stops[0].middle, Some(middle.before));
    assert_eq!(with.layouts[0].stops[1].middle, middle.rods_in);
    assert_eq!(with.layouts[0].stops[1].rods, middle.rods);
    assert!(
        with.layouts[0]
            .stops
            .iter()
            .skip(2)
            .all(|stop| stop.middle.is_none() && stop.rods.is_none())
    );
    assert!(with.layouts[0].stops[0].rods.is_none());
    assert!(
        with.layouts[1]
            .stops
            .iter()
            .all(|stop| stop.middle.is_none() && stop.rods.is_none())
    );
    for layout in &mut with.layouts {
        for stop in &mut layout.stops {
            stop.middle = None;
            stop.rods = None;
        }
    }
    with.main_curve = None;
    assert_eq!(
        serde_json::to_vec(&with).unwrap(),
        serde_json::to_vec(&baseline).unwrap()
    );

    input.batches[0].middle[0].rods_in = None;
    input.batches[0].middle[0].rods = None;
    let unsettled = compare(&input).unwrap();
    assert!(unsettled.layouts[0].stops[0].middle.is_some());
    assert!(unsettled.layouts[0].stops[1].middle.is_none());
    assert!(unsettled.layouts[0].stops[1].rods.is_none());

    // A later slice still carries the fitted curve, but cannot supply drawing data.
    input.batches[0].version_from = 1;
    input.batches[0].middle.clear();
    for run in &mut input.batches[0].runs {
        run.version = 1;
    }
    let without = compare(&input).unwrap();
    assert!(without.main_curve.is_none());
    assert!(
        without
            .layouts
            .iter()
            .flat_map(|l| &l.stops)
            .all(|s| s.middle.is_none() && s.rods.is_none())
    );
}

#[test]
fn batches_must_agree_on_the_fitted_curve_including_its_absence() {
    let mut input = input(
        vec![run("first", 0, 20.0, 4.0), run("other", 0, 15.0, 3.0)],
        1,
    );
    let mut later = input.batches[0].clone();
    later.version_from = 1;
    for run in &mut later.runs {
        run.version = 1;
    }
    input.batches.push(later);
    let curve = MainCurve {
        upper: Level::T5,
        apex: Level::T8,
        lower: Level::T12,
    };
    input.batches[0].main_curve = Some(curve);
    assert_eq!(compare(&input), Err(refused("main-curves-not-shared")));
    input.batches[1].main_curve = Some(MainCurve {
        apex: Level::T9,
        ..curve
    });
    assert_eq!(compare(&input), Err(refused("main-curves-not-shared")));
    input.batches[1].main_curve = Some(curve);
    assert!(compare(&input).is_ok());
}

#[test]
fn bending_remains_assumed_even_when_all_versions_are_identical() {
    let result = compare(&input(
        vec![run("first", 0, 20.0, 4.0), run("other", 0, 20.0, 4.0)],
        1,
    ))
    .unwrap();
    for layout in &result.layouts {
        let bending = &layout.stops[3];
        assert_eq!(bending.stop, BenchStop::BendingForward);
        assert!(!bending.estimates.is_empty());
        assert!(
            bending
                .estimates
                .iter()
                .all(|row| row.tag == EstimateTag::Assumed)
        );
        assert!(!bending.parts.is_empty());
        assert!(bending.parts.iter().all(|part| !part.steady));
        let standing = &layout.stops[2];
        assert!(standing.parts.iter().all(|part| part.steady));
    }
}

#[test]
fn an_unused_slice_offset_cannot_be_named_as_a_range_driver() {
    let runs: Vec<_> = (0..3)
        .map(|version| {
            let mut value = run("first", version, 20.0, f64::from(version + 1));
            value.draws.slice_offset_mm = f64::from(version + 1) * 10.0;
            value
        })
        .collect();
    let by_version = runs.iter().map(|run| (run.version, run)).collect();
    let samples = vec![(0, 1.0), (1, 2.0), (2, 3.0)];
    assert_eq!(ranks::driver(&by_version, &samples).0, None);
}

#[test]
fn the_fit_gets_one_column_like_every_other_unknown() {
    // A single pose coordinate that happens to follow the result exactly
    // must not name the fit: the fit is read through Before main Cobb
    // alone, which here does not move. Stiffness follows the result closely.
    let runs: Vec<_> = (0..6u32)
        .map(|version| {
            let mut value = run("first", version, 20.0, 1.0);
            value.draws.stiffness_scale = [2.0, 1.0, 3.0, 4.0, 6.0, 5.0][version as usize];
            value.stops[0].poses = Some(vec![shapes::made_up::VertebraPose {
                level: Level::T8,
                centre: [f64::from(version), 0.0, 0.0],
                rotation: [0.0; 3],
                height: 20.0,
                wedge_front: 0.0,
                wedge_side: 0.0,
                top_half_width: 10.0,
                top_half_depth: 8.0,
                bottom_half_width: 10.0,
                bottom_half_depth: 8.0,
            }]);
            value
        })
        .collect();
    let by_version = runs.iter().map(|run| (run.version, run)).collect();
    let samples: Vec<_> = (0..6u32).map(|v| (v, f64::from(v) + 1.0)).collect();
    assert_eq!(
        ranks::driver(&by_version, &samples).0,
        Some(SampledUnknown::StiffnessScale)
    );
}
