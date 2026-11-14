use super::*;
use shapes::implant_layout::{
    AnchorKind, AnchorProposal, CurveSide, LayoutChange, PedicleEstimate, ValueStatus,
};
use shapes::levels::Level;
use shapes::spine_readings::{Band, BodySide};

fn anchor(side: BodySide) -> AnchorProposal {
    AnchorProposal {
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
    assert_eq!(force.mostly_from, SampledUnknown::StiffnessScale);
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
    assert_eq!(count.sorted_by, SampledUnknown::StiffnessScale);
    assert_eq!(count.order, vec![1, 0, 2]);
    assert_eq!((count.lower, count.higher, count.ties), (0, 2, 1));
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
    let mut value = run("first", 0, 20.0, 10.0).stops.remove(0);
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
