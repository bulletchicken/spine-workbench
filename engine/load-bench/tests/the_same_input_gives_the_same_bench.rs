//! Global version identities, both film tiers and the complete bench wire shape.

mod realistic_cases;

use forward_model::project::{front_marks, side_marks};
use load_bench::{compare_layouts, run_load_bench};
use made_up_spines::film_layout::FilmLayout;
use made_up_spines::patient::make_up_patient;
use made_up_spines::recipes::film_recipes;
use serde_json::Value;
use shapes::confidence::{Confidence, ConfidenceBand};
use shapes::film::FilmKind;
use shapes::implant_layout::{
    ActivityLevel, AnchorKind, AnchorProposal, ChildFacts, CoronalLine, CurveSide, ImplantLayout,
    LayoutChange, LayoutFilm, LayoutInput, NamingDoubt, PedicleEstimate, RodMaterial, RodProposal,
    ValueStatus,
};
use shapes::landmarks::Landmarks;
use shapes::levels::Level;
use shapes::load_bench::{BenchBatch, BenchCompareInput, BenchInput, BenchResult, BenchStop};
use shapes::made_up::{MadeUpRecipe, RecipeName, Tier, VertebraPose};
use shapes::marks::{MarkSource, PedicleMark};
use shapes::naming::{ChosenBy, LevelNaming, Template};
use shapes::spine_fit::{FilmToFit, FitFromFilms, FitLengths, MarkSpread, TypicalSource};
use shapes::spine_readings::{Band, BodySide};
use shapes::vertebra::{FilmMarks, VertebraMarks, VertebraOnFilm};

fn child() -> ChildFacts {
    ChildFacts {
        age_years: Some(15.0),
        weight_kg: Some(40.0),
        activity: Some(ActivityLevel::Typical),
    }
}

fn flat_band(value: f64) -> Band {
    Band {
        p10: value,
        p50: value,
        p90: value,
    }
}

// Explicit synthetic geometry isolates batching from radiographic fitting
// and catalogue decisions. Every standing head lies exactly on its rod,
// so nonnegative tissue energy has the independent global minimum zero.
fn resting_input(versions: u32) -> BenchInput {
    let levels = load_bench::daily_loads::LOAD_LEVELS;
    let poses: Vec<_> = levels
        .into_iter()
        .enumerate()
        .map(|(i, level)| VertebraPose {
            level,
            centre: [0.0, 0.0, 660.0 - 40.0 * i as f64],
            rotation: [0.0; 3],
            height: 20.0,
            wedge_front: 0.0,
            wedge_side: 0.0,
            top_half_width: 12.0,
            top_half_depth: 8.0,
            bottom_half_width: 12.0,
            bottom_half_depth: 8.0,
        })
        .collect();
    let mut anchors = Vec::new();
    for level in [Level::T6, Level::T8] {
        let pose = poses.iter().find(|p| p.level == level).unwrap();
        for (side, y) in [(BodySide::Left, 9.0), (BodySide::Right, -9.0)] {
            anchors.push(AnchorProposal {
                wider_than_pedicle: Vec::new(),
                screw_here: None,
                diameter_sources: Vec::new(),
                length_sources: Vec::new(),
                direction_sources: Vec::new(),
                look_sources: Vec::new(),
                level,
                side,
                kind: AnchorKind::Screw,
                entry_mm: [-15.0, y, pose.centre[2]],
                entry_status: ValueStatus::Typical,
                direction_unit: [1.0, 0.0, 0.0],
                direction_status: ValueStatus::Typical,
                medial_angle: None,
                medial_angles_beside: Vec::new(),
                convergence_deg: flat_band(0.0),
                diameter_mm: Some(4.0),
                diameter_also_seen: Vec::new(),
                diameter_status: ValueStatus::Typical,
                width_ratio: None,
                diameters_beside: Vec::new(),
                screw_to_pedicle: None,
                chord_mm: None,
                chord_share: None,
                length_mm: Some(25.0),
                length_also_seen: Vec::new(),
                length_status: ValueStatus::Typical,
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
                looks: Vec::new(),
                sources: Vec::new(),
            });
        }
    }
    let rods = [BodySide::Left, BodySide::Right]
        .into_iter()
        .map(|side| {
            let points_mm: Vec<_> = anchors
                .iter()
                .filter(|a| a.side == side)
                .map(|a| a.entry_mm)
                .collect();
            RodProposal {
                standing_points_mm: Vec::new(),
                side,
                curve_side: CurveSide::Outside,
                material: RodMaterial::CobaltChrome,
                diameter_mm: 5.5,
                target_kyphosis_deg: 0.0,
                target_kyphosis_status: ValueStatus::Typical,
                coronal_line: CoronalLine::L5Vertical,
                overbend_deg: 0.0,
                head_offset_mm: 0.0,
                length_mm: 80.0,
                geometry_status: ValueStatus::FromTemplate,
                points_mm,
                sources: Vec::new(),
            }
        })
        .collect();
    BenchInput {
        fit: realistic_cases::truth_fit(&poses),
        layouts: vec![ImplantLayout {
            id: "first".into(),
            change: LayoutChange::First {},
            upper: Level::T6,
            lower: Level::T8,
            anchors,
            rods,
            released_levels: Vec::new(),
        }],
        child: child(),
        seed: 19,
        versions,
        version_from: 0,
        version_count: versions,
    }
}

fn fitted_input(front_only: bool) -> BenchInput {
    let recipe = MadeUpRecipe {
        name: RecipeName::RightThoracic40,
        seed: 1,
        tier: Tier::T0,
    };
    let patient = make_up_patient(recipe.name, recipe.seed).unwrap();
    let recipes = film_recipes(&recipe, u64::from(recipe.seed));
    let film = |kind| {
        let recipe = recipes.iter().find(|r| r.kind == kind).unwrap();
        let layout = FilmLayout::new(recipe, &patient.standing);
        let vertebrae = patient
            .standing
            .levels
            .iter()
            .zip(&patient.standing.vertebrae)
            .filter(|(level, _)| (Level::T1..=Level::L5).contains(level))
            .enumerate()
            .map(|(i, (&level, v))| {
                let marks = if kind == FilmKind::Front {
                    let mut points = front_marks(v, &layout.camera).unwrap();
                    if front_only {
                        points.ped_l = PedicleMark::NotVisible {
                            source: MarkSource::Hand,
                        };
                        points.ped_r = PedicleMark::NotVisible {
                            source: MarkSource::Hand,
                        };
                    }
                    VertebraMarks::Front { points }
                } else {
                    VertebraMarks::Side {
                        points: side_marks(v, &layout.camera).unwrap(),
                    }
                };
                VertebraOnFilm {
                    id: i as u32 + 1,
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
        FilmToFit {
            marks: FilmMarks {
                film: kind,
                vertebrae,
                landmarks: Landmarks::NONE,
                naming: LevelNaming {
                    template: Template::Standard,
                    chosen_by: ChosenBy::User,
                    alternatives: Vec::new(),
                    from_top: None,
                    from_bottom: None,
                    flags: Vec::new(),
                    l5_confirmed_on: None,
                },
                named_for: layout.geometry.image_right.unwrap(),
            },
            geometry: layout.geometry,
            size: layout.size,
            mark_spread: MarkSpread::Millimetres {
                sd_mm: 0.1,
                shared_sd_mm: 0.0,
            },
            facing_assumed: false,
        }
    };
    let mut front = film(FilmKind::Front);
    if front_only {
        front.geometry.mm_per_px = None;
        front.geometry.mm_per_px_vertical = None;
        front.geometry.scale = None;
        front.geometry.confirmed.scale = false;
    }
    let fit = joint_fit::fit_from_films(&FitFromFilms {
        front: front.clone(),
        side: (!front_only).then(|| film(FilmKind::Side)),
        typical: TypicalSource::BuiltIn,
        twist_readings: Vec::new(),
    })
    .unwrap();
    assert_eq!(fit.side_film.is_none(), front_only);
    assert_eq!(
        fit.lengths,
        if front_only {
            FitLengths::TypicalSize
        } else {
            FitLengths::Measured
        }
    );
    let layout_input = LayoutInput {
        fit: fit.clone(),
        front: Some(LayoutFilm {
            marks: front.marks,
            geometry: front.geometry,
            size: front.size,
        }),
        bend_left: None,
        bend_right: None,
        child: child(),
        naming: NamingDoubt::AsRead,
        versions: 1,
        seed: 1,
        pedicle_child_share: None,
    };
    let proposals = implant_layout::propose_layouts(&layout_input).unwrap();
    let first = proposals
        .layouts
        .first()
        .expect("a first placement")
        .clone();
    BenchInput {
        fit,
        layouts: vec![first],
        child: child(),
        seed: 1,
        versions: 1,
        version_from: 0,
        version_count: 1,
    }
}

fn keys(value: &Value, expected: &[&str]) {
    let actual: std::collections::BTreeSet<_> = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(actual, expected.iter().copied().collect());
}

fn walk_contract(input: &BenchInput, batch: &BenchBatch) {
    assert_eq!(batch.middle.len(), 1);
    let middle = &batch.middle[0];
    assert_eq!(middle.layout_id, input.layouts[0].id);
    assert_eq!(
        middle.before,
        input
            .fit
            .vertebrae
            .iter()
            .filter(|v| load_bench::daily_loads::LOAD_LEVELS.contains(&v.level))
            .map(|v| v.pose)
            .collect::<Vec<_>>()
    );
    assert_eq!(middle.rods_in.is_some(), middle.rods.is_some());
    let poses = middle
        .rods_in
        .as_ref()
        .expect("the fixture's middle run settles");
    assert_heads_on_lines(input, poses, middle.rods.as_ref().unwrap());
    let json = serde_json::to_value(batch).unwrap();
    keys(
        &json,
        &["versionFrom", "versionCount", "runs", "middle", "mainCurve"],
    );
    assert_eq!(
        serde_json::from_value::<BenchBatch>(json.clone()).unwrap(),
        *batch
    );
    for (run, value) in batch.runs.iter().zip(json["runs"].as_array().unwrap()) {
        keys(
            value,
            &[
                "layoutId",
                "version",
                "settled",
                "stiffRodShare",
                "draws",
                "stops",
            ],
        );
        keys(
            &value["draws"],
            &[
                "stiffnessScale",
                "ribFactors",
                "boneFactor",
                "weightKg",
                "stepsPerDay",
                "walkingMultiple",
                "rodYieldMpa",
                "preloadFactor",
                "sliceOffsetMm",
                "contourKnockDown",
            ],
        );
        assert!(run.settled, "version {} did not settle", run.version);
        assert!(run.stiff_rod_share.is_finite());
        assert_eq!(run.stops.len(), 5);
        for (stop, value) in run.stops.iter().zip(value["stops"].as_array().unwrap()) {
            keys(
                value,
                &[
                    "stop",
                    "anchorForceN",
                    "anchorDemand",
                    "rodSpanUse",
                    "junctionAbove",
                    "junctionBelow",
                    "mainCobbDeg",
                    "kyphosisDeg",
                    "apicalTwistDeg",
                    "poses",
                    "rodSwing",
                    "cyclesPerYear",
                ],
            );
            assert_eq!(stop.anchor_force_n.len(), input.layouts[0].anchors.len());
            assert_eq!(stop.anchor_demand.len(), input.layouts[0].anchors.len());
            assert!(
                stop.anchor_force_n
                    .iter()
                    .chain(&stop.anchor_demand)
                    .chain(&stop.rod_span_use)
                    .all(|v| v.is_finite() && *v >= 0.0)
            );
            assert!(
                stop.main_cobb_deg.is_finite()
                    && stop.kyphosis_deg.is_finite()
                    && stop.apical_twist_deg.is_finite()
            );
            assert_eq!(
                stop.poses.is_some(),
                matches!(stop.stop, BenchStop::Before | BenchStop::RodsIn)
            );
            if stop.stop == BenchStop::YearOfWalking {
                assert_eq!(stop.cycles_per_year, Some(run.draws.steps_per_day * 365.0));
                let swings = stop.rod_swing.as_ref().unwrap();
                assert_eq!(swings.len(), input.layouts[0].rods.len());
                for swing in value["rodSwing"].as_array().unwrap() {
                    keys(swing, &["side", "ratio"]);
                }
            } else {
                assert!(value["cyclesPerYear"].is_null() && value["rodSwing"].is_null());
            }
            if matches!(stop.stop, BenchStop::Before | BenchStop::RodsIn) {
                assert!(value["junctionAbove"].is_null() && value["junctionBelow"].is_null());
            }
        }
    }
    // An identical second placement independently has a zero paired
    // difference in every version. Reusing the computed run here isolates
    // the comparison contract from another physical solve.
    let mut layouts = input.layouts.clone();
    let mut same = layouts[0].clone();
    same.id = "same-placement".into();
    same.change = LayoutChange::ChangedByHand {};
    layouts.push(same);
    let mut comparison_batch = batch.clone();
    comparison_batch
        .runs
        .extend(batch.runs.iter().cloned().map(|mut run| {
            run.layout_id = "same-placement".into();
            run
        }));
    let result = compare_layouts(&BenchCompareInput {
        layouts,
        batches: vec![comparison_batch],
    })
    .unwrap();
    let evidence = load_bench::evidence::Evidence::read().unwrap();
    for layout in &result.layouts {
        for stop in &layout.stops {
            for key in stop
                .cycles_sources
                .iter()
                .chain(stop.estimates.iter().flat_map(|e| &e.sources))
                .chain(stop.parts.iter().flat_map(|p| &p.sources))
            {
                assert!(evidence.source(key).is_some(), "unknown source: {key}");
            }
            if stop.stop == BenchStop::Before {
                assert!(stop.parts.is_empty());
                assert!(stop.estimates.iter().all(|e| matches!(
                    e.measure,
                    shapes::load_bench::Measure::MainCobb
                        | shapes::load_bench::Measure::Kyphosis
                        | shapes::load_bench::Measure::ApicalTwist
                )));
                assert!(stop.estimates.iter().all(|e| e.sources.is_empty()));
            }
            for part in &stop.parts {
                let occupied = match part.part {
                    shapes::load_bench::PartRef::Anchor { level, side } => input.layouts[0]
                        .anchors
                        .iter()
                        .any(|a| a.level == level && a.side == side && a.kind != AnchorKind::None),
                    _ => false,
                };
                assert_eq!(part.demand.is_some(), occupied);
            }
        }
        for count in &layout.counts {
            assert_eq!(count.sorted_by, None);
            assert_eq!(
                count.outcomes,
                vec![shapes::load_bench::PairOutcome::Tie; input.versions as usize]
            );
        }
    }
    let value = serde_json::to_value(&result).unwrap();
    keys(
        &value,
        &["versions", "firstId", "layouts", "scales", "mainCurve"],
    );
    assert_eq!(
        value["mainCurve"],
        serde_json::to_value(batch.main_curve).unwrap()
    );
    for layout in value["layouts"].as_array().unwrap() {
        keys(
            layout,
            &["layoutId", "versionsUnsettled", "stops", "counts"],
        );
        for stop in layout["stops"].as_array().unwrap() {
            keys(
                stop,
                &[
                    "stop",
                    "estimates",
                    "parts",
                    "ghosts",
                    "cyclesPerYear",
                    "outsideStiffRod",
                    "middle",
                    "rods",
                    "cyclesSources",
                ],
            );
            for estimate in stop["estimates"].as_array().unwrap() {
                keys(
                    estimate,
                    &["measure", "band", "tag", "mostlyFrom", "sources"],
                );
                keys(&estimate["band"], &["p10", "p50", "p90"]);
            }
            for part in stop["parts"].as_array().unwrap() {
                keys(
                    part,
                    &[
                        "part",
                        "share",
                        "steady",
                        "versionsPastStrength",
                        "demand",
                        "sources",
                    ],
                );
                keys(&part["share"], &["p10", "p50", "p90"]);
                let expected: &[&str] = match part["part"]["kind"].as_str().unwrap() {
                    "anchor" => &["kind", "level", "side"],
                    "rod-span" => &["kind", "side", "from", "to"],
                    "rod" => &["kind", "side"],
                    "junction" => &["kind", "which"],
                    _ => panic!("known physical part"),
                };
                keys(&part["part"], expected);
            }
            if let Some(ghosts) = stop["ghosts"].as_array() {
                assert_eq!(ghosts.len(), 2);
                for pose in ghosts.iter().flat_map(|spine| spine.as_array().unwrap()) {
                    keys(
                        pose,
                        &[
                            "level",
                            "centre",
                            "rotation",
                            "height",
                            "wedgeFront",
                            "wedgeSide",
                            "topHalfWidth",
                            "topHalfDepth",
                            "bottomHalfWidth",
                            "bottomHalfDepth",
                        ],
                    );
                }
            }
        }
        for count in layout["counts"].as_array().unwrap() {
            keys(
                count,
                &[
                    "measure", "stop", "lower", "higher", "ties", "order", "sortedBy", "outcomes",
                    "clearAt",
                ],
            );
            assert_eq!(count["lower"], 0);
            assert_eq!(count["higher"], 0);
            assert_eq!(count["ties"], input.versions);
            assert_eq!(
                count["order"].as_array().unwrap().len(),
                input.versions as usize
            );
        }
    }
    for scale in value["scales"].as_array().unwrap() {
        keys(scale, &["measure", "lowest", "highest"]);
    }
    assert_eq!(
        serde_json::from_value::<BenchResult>(value).unwrap(),
        result
    );
}

#[test]
fn thirty_two_versions_keep_their_bits_across_two_and_uneven_splits() {
    let input = resting_input(32);
    let whole = run_load_bench(&input).unwrap();
    let bytes = serde_json::to_vec(&whole.runs).unwrap();
    let fingerprint = bytes.iter().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    });
    assert_eq!(fingerprint, 6_582_547_120_467_459_952);
    let split = |cuts: &[u32]| {
        let mut runs = Vec::new();
        for pair in cuts.windows(2) {
            let batch = run_load_bench(&BenchInput {
                version_from: pair[0],
                version_count: pair[1] - pair[0],
                ..input.clone()
            })
            .unwrap();
            runs.extend(batch.runs);
        }
        runs
    };
    assert_eq!(
        serde_json::to_vec(&whole.runs).unwrap(),
        serde_json::to_vec(&split(&[0, 16, 32])).unwrap()
    );
    assert_eq!(
        serde_json::to_vec(&whole.runs).unwrap(),
        serde_json::to_vec(&split(&[0, 3, 19, 20, 32])).unwrap()
    );
    for run in &whole.runs {
        assert!(run.settled);
        let before = &run.stops[0];
        let rods_in = &run.stops[1];
        assert!(rods_in.anchor_force_n.iter().all(|f| f.abs() < 1e-8));
        assert_eq!(before.poses, rods_in.poses);
        assert_eq!(run.stiff_rod_share, 0.0);
    }
}

#[test]
fn a_front_film_without_scale_runs_from_fit_through_comparison() {
    let input = fitted_input(true);
    let batch = run_load_bench(&input).unwrap();
    walk_contract(&input, &batch);
}

#[test]
fn two_films_run_from_fit_through_comparison() {
    let input = fitted_input(false);
    let batch = run_load_bench(&input).unwrap();
    walk_contract(&input, &batch);
    assert_ne!(
        batch.middle[0].rods_in.as_ref().unwrap(),
        &batch.middle[0].before
    );
}

#[test]
fn a_corrected_spine_keeps_the_rods_straight_top_continuations() {
    use nalgebra::Vector3;
    let mut input = fitted_input(false);
    input.version_count = 0;
    for rod in &mut input.layouts[0].rods {
        let top = Vector3::from(rod.points_mm[0]);
        let tangent = (top - Vector3::from(rod.points_mm[1])).normalize();
        rod.points_mm.insert(0, (top + tangent * 5.0).into());
        rod.points_mm.insert(0, (top + tangent * 10.0).into());
    }
    let batch = run_load_bench(&input).unwrap();
    let middle = &batch.middle[0];
    let poses = middle
        .rods_in
        .as_ref()
        .expect("the fitted middle run settles");
    let movement = poses
        .iter()
        .zip(&middle.before)
        .map(|(a, b)| (Vector3::from(a.centre) - Vector3::from(b.centre)).norm())
        .fold(0.0, f64::max);
    assert!(movement > 0.1, "the spine must really move: {movement} mm");
    let rods = middle.rods.as_ref().unwrap();
    assert_heads_on_lines(&input, poses, rods);
    for line in rods {
        let rod = input.layouts[0]
            .rods
            .iter()
            .find(|r| r.side == line.side)
            .unwrap();
        let top_anchor = input.layouts[0]
            .anchors
            .iter()
            .filter(|a| a.side == rod.side && a.kind != AnchorKind::None)
            .min_by_key(|a| a.level)
            .unwrap();
        let head = moved_head(&input, poses, rod, top_anchor);
        let top_head_knot = line
            .points_mm
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                (Vector3::from(**a) - head)
                    .norm_squared()
                    .total_cmp(&(Vector3::from(**b) - head).norm_squared())
            })
            .unwrap()
            .0;
        assert!(
            top_head_knot >= 2,
            "both extra knots must precede the upper head"
        );
        // An affine head-translation field retains the straight top segment's
        // collinearity, including the extrapolated knots above the upper head.
        let a = Vector3::from(line.points_mm[0]);
        let b = Vector3::from(line.points_mm[1]);
        let c = Vector3::from(line.points_mm[2]);
        let distance = (a - b).cross(&(c - b)).norm() / (c - b).norm();
        assert!(distance < 1e-8, "the top continuation bends: {distance} mm");
    }
}

#[test]
fn an_empty_first_slice_does_not_check_unused_load_facts() {
    let mut input = resting_input(1);
    input.version_count = 0;
    let original = run_load_bench(&input).unwrap();
    assert!(original.middle[0].rods_in.is_some());
    for (activity, weight) in [
        (ActivityLevel::VeryLow, 0.0),
        (ActivityLevel::VeryActive, -1.0),
    ] {
        input.child.activity = Some(activity);
        input.child.weight_kg = Some(weight);
        assert_eq!(run_load_bench(&input).unwrap(), original);
    }
}

#[test]
fn a_failed_middle_solve_keeps_the_version_runs_and_standing_poses() {
    let mut input = resting_input(2);
    // Finite supplied dimensions can still overflow the beam stiffness.
    // No drawing or version solve can settle such a rod.
    for rod in &mut input.layouts[0].rods {
        rod.diameter_mm = f64::MAX;
    }
    let batch = run_load_bench(&input).unwrap();
    assert_eq!(batch.runs.len(), 2);
    assert_eq!(
        batch.runs.iter().map(|r| r.version).collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert!(batch.runs.iter().all(|r| !r.settled && r.stops.len() == 1));
    let middle = &batch.middle[0];
    assert_eq!(
        middle.before,
        batch.runs[0].stops[0].poses.as_ref().unwrap().clone()
    );
    assert!(middle.rods_in.is_none() && middle.rods.is_none());
}

#[test]
fn a_drawing_preparation_overflow_keeps_the_empty_slice() {
    let mut input = resting_input(1);
    input.version_count = 0;
    // The finite rotation vector overflows while locating the ground at
    // the lower endplate. Preparing the mechanical chain must fail locally.
    input.fit.vertebrae.last_mut().unwrap().pose.rotation[0] = f64::MAX;
    let batch = run_load_bench(&input).unwrap();
    assert!(batch.runs.is_empty());
    assert_eq!(batch.middle.len(), 1);
    assert_eq!(
        batch.middle[0].before.last().unwrap(),
        &input.fit.vertebrae.last().unwrap().pose
    );
    assert!(batch.middle[0].rods_in.is_none() && batch.middle[0].rods.is_none());
}

#[test]
fn an_unavailable_stainless_fatigue_level_is_written_as_null() {
    let mut input = resting_input(1);
    for rod in &mut input.layouts[0].rods {
        rod.material = RodMaterial::StainlessSteel;
    }
    let batch = run_load_bench(&input).unwrap();
    let year = batch.runs[0].stops.last().unwrap();
    assert_eq!(year.stop, BenchStop::YearOfWalking);
    assert!(
        year.rod_swing
            .as_ref()
            .unwrap()
            .iter()
            .all(|r| r.ratio.is_none())
    );
    walk_contract(&input, &batch);
}

#[test]
fn open_activity_bands_keep_their_missing_endpoints() {
    for activity in [ActivityLevel::VeryLow, ActivityLevel::VeryActive] {
        let mut input = resting_input(1);
        input.child.activity = Some(activity);
        assert_eq!(
            run_load_bench(&input),
            Err(load_bench::BenchError::Refused(
                "open-ended-activity-band".into()
            ))
        );
    }
}

fn moved_head(
    input: &BenchInput,
    poses: &[VertebraPose],
    rod: &RodProposal,
    anchor: &AnchorProposal,
) -> nalgebra::Vector3<f64> {
    use load_bench::chain::Pose;
    use nalgebra::Vector3;
    let original = Pose::from_vertebra(
        &input
            .fit
            .vertebrae
            .iter()
            .find(|v| v.level == anchor.level)
            .unwrap()
            .pose,
    );
    let moved = Pose::from_vertebra(poses.iter().find(|p| p.level == anchor.level).unwrap());
    let head = Vector3::from(anchor.entry_mm)
        - Vector3::from(anchor.direction_unit)
            * if anchor.kind == AnchorKind::Hook {
                0.0
            } else {
                rod.head_offset_mm
            };
    let local = original.rotation.transpose() * (head - original.centre_mm);
    moved.centre_mm + moved.rotation * local
}

fn assert_heads_on_lines(
    input: &BenchInput,
    poses: &[VertebraPose],
    lines: &[shapes::load_bench::RodLine],
) {
    use nalgebra::Vector3;
    assert_eq!(lines.len(), input.layouts[0].rods.len());
    for rod in &input.layouts[0].rods {
        let line = lines.iter().find(|line| line.side == rod.side).unwrap();
        assert!(line.points_mm.len() >= rod.points_mm.len());
        for anchor in input.layouts[0]
            .anchors
            .iter()
            .filter(|a| a.side == rod.side && a.kind != AnchorKind::None)
        {
            let current = moved_head(input, poses, rod, anchor);
            let distance = line
                .points_mm
                .windows(2)
                .map(|pair| {
                    let a = Vector3::from(pair[0]);
                    let d = Vector3::from(pair[1]) - a;
                    let t = ((current - a).dot(&d) / d.norm_squared()).clamp(0.0, 1.0);
                    (current - a - d * t).norm()
                })
                .reduce(f64::min)
                .unwrap();
            let tolerance = 3.0_f64.sqrt()
                * load_bench::rigid_rod::minimise::Options::default().constraint_tolerance
                * load_bench::rigid_rod::LENGTH_MM
                + 64.0 * f64::EPSILON * current.norm().max(1.0);
            assert!(
                distance <= tolerance,
                "{:?}/{:?}: {distance} mm, tolerance {tolerance}",
                anchor.level,
                anchor.side
            );
        }
    }
}

#[test]
fn the_middle_keeps_the_fit_and_the_full_length_of_a_rod_that_fits() {
    let mut input = resting_input(2);
    // The contour extends above the upper head; a head-only line loses that end.
    for rod in &mut input.layouts[0].rods {
        rod.points_mm[0][2] += 20.0;
        rod.points_mm[1][2] -= 20.0;
        rod.length_mm = 120.0;
    }
    // Pin the lower head at the physical lower end, as the bench requires.
    for anchor in &mut input.layouts[0].anchors {
        if anchor.level == Level::T8 {
            anchor.entry_mm[2] -= 20.0;
        }
    }
    let first = run_load_bench(&BenchInput {
        version_count: 1,
        ..input.clone()
    })
    .unwrap();
    let middle = &first.middle[0];
    assert_eq!(
        middle.before.iter().map(|p| p.level).collect::<Vec<_>>(),
        load_bench::daily_loads::LOAD_LEVELS
    );
    let poses = middle
        .rods_in
        .as_ref()
        .expect("the resting drawing settles");
    assert_heads_on_lines(&input, poses, middle.rods.as_ref().unwrap());
    for line in middle.rods.as_ref().unwrap() {
        let length: f64 = line
            .points_mm
            .windows(2)
            .map(|p| {
                p[0].iter()
                    .zip(p[1])
                    .map(|(a, b)| (a - b) * (a - b))
                    .sum::<f64>()
                    .sqrt()
            })
            .sum();
        assert!((length - 120.0).abs() < 1e-8, "full rod length: {length}");
    }
    let later = run_load_bench(&BenchInput {
        version_from: 1,
        version_count: 1,
        ..input.clone()
    })
    .unwrap();
    assert!(later.middle.is_empty());
    assert_eq!(first.main_curve, later.main_curve);
    let without = compare_layouts(&BenchCompareInput {
        layouts: input.layouts.clone(),
        batches: vec![later.clone()],
    })
    .unwrap();
    assert!(without.main_curve.is_none());
    assert!(
        without.layouts[0]
            .stops
            .iter()
            .all(|s| s.middle.is_none() && s.rods.is_none())
    );
    let with = compare_layouts(&BenchCompareInput {
        layouts: input.layouts,
        batches: vec![later, first.clone()],
    })
    .unwrap();
    assert_eq!(with.layouts[0].stops[0].middle, Some(middle.before.clone()));
    assert_eq!(with.layouts[0].stops[1].middle, middle.rods_in);
    assert_eq!(with.layouts[0].stops[1].rods, middle.rods);
}

#[test]
fn the_main_curve_matches_the_right_thoracic_answer_key() {
    use forward_model::{endplates::Endplates, vertebra_params::VertebraParams};
    use shapes::load_bench::MainCurve;
    use shapes::made_up::CurveRegion;
    let key = made_up_spines::make_up_answer_key(&MadeUpRecipe {
        name: RecipeName::RightThoracic40,
        seed: 1,
        tier: Tier::T0,
    })
    .unwrap();
    assert_eq!(key.angles.frontal_cobb[0].region, CurveRegion::MainThoracic);
    let fit = realistic_cases::truth_fit(
        &key.poses
            .iter()
            .copied()
            .filter(|p| load_bench::daily_loads::LOAD_LEVELS.contains(&p.level))
            .collect::<Vec<_>>(),
    );
    let poses: Vec<_> = fit.vertebrae.iter().map(|v| v.pose).collect();
    let plates: Vec<_> = poses
        .iter()
        .map(|p| Endplates::of(&VertebraParams::from(p)))
        .collect();
    let tilt = |n: nalgebra::Vector3<f64>| libm::atan2(-n.y, n.z);
    let mut most = (0.0, 0, 0);
    for upper in 0..plates.len() {
        for lower in upper + 1..plates.len() {
            let cobb = (tilt(plates[upper].top.normal) - tilt(plates[lower].bottom.normal)).abs();
            if cobb > most.0 {
                most = (cobb, upper, lower);
            }
        }
    }
    assert!((most.0.to_degrees() - key.angles.frontal_cobb[0].degrees).abs() < 1e-10);
    // The answer key carries exact centres: use distance from the end-to-end
    // coronal chord, rather than rounding the recipe's continuous apex.
    let a = poses[most.1].centre;
    let b = poses[most.2].centre;
    let apex_index = (most.1..=most.2)
        .max_by(|&i, &j| {
            let area =
                |p: [f64; 3]| ((p[1] - a[1]) * (b[2] - a[2]) - (p[2] - a[2]) * (b[1] - a[1])).abs();
            area(poses[i].centre).total_cmp(&area(poses[j].centre))
        })
        .unwrap();
    let expected = MainCurve {
        upper: poses[most.1].level,
        apex: poses[apex_index].level,
        lower: poses[most.2].level,
    };
    let proposals = implant_layout::propose_layouts(&LayoutInput {
        fit: fit.clone(),
        front: None,
        bend_left: None,
        bend_right: None,
        child: child(),
        naming: NamingDoubt::AsRead,
        versions: 1,
        seed: 1,
        pedicle_child_share: None,
    })
    .unwrap();
    let input = BenchInput {
        fit,
        layouts: vec![proposals.layouts[0].clone()],
        child: child(),
        seed: 1,
        versions: 1,
        version_from: 0,
        version_count: 0,
    };
    let batch = run_load_bench(&input).unwrap();
    assert_eq!(batch.main_curve, Some(expected));
    let later = run_load_bench(&BenchInput {
        version_from: 1,
        ..input
    })
    .unwrap();
    assert_eq!(later.main_curve, Some(expected));
    assert!(later.middle.is_empty());
}

#[test]
fn fit_noise_and_seed_never_move_the_middle_drawing() {
    let mut input = resting_input(2);
    input.version_count = 0;
    let original = run_load_bench(&input).unwrap();
    for v in &mut input.fit.vertebrae {
        v.sd.centre_mm = [5.0; 3];
        v.sd.angles_rad = [0.1; 3];
    }
    input.seed = 1234;
    let changed = run_load_bench(&input).unwrap();
    assert_eq!(original.middle, changed.middle);
    assert_eq!(
        original.middle[0].rods_in.as_ref().unwrap(),
        &original.middle[0].before
    );
}
