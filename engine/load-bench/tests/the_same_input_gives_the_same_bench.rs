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
    let json = serde_json::to_value(batch).unwrap();
    keys(&json, &["versionFrom", "versionCount", "runs"]);
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
    let value = serde_json::to_value(&result).unwrap();
    keys(
        &value,
        &["versions", "firstId", "layouts", "scales", "mainCurve"],
    );
    assert!(value["mainCurve"].is_null());
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
                assert_eq!(estimate["sources"], serde_json::json!([]));
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
                assert!(part["demand"].is_null());
                assert_eq!(part["sources"], serde_json::json!([]));
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
