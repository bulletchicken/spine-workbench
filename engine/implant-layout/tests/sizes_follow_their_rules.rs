//! Discrete-size answers worked from thresholds and catalogue entries.
//! Population comparisons are plausibility checks, never a truth test.

use implant_layout::evidence::{Coordinate, PublishedValue, Table, tables};
use implant_layout::pedicles::{CurvePosition, drawn_outer_width, width_prior};
use implant_layout::readings::{NamedSampler, PedicleDraw, Version};
use implant_layout::screws::anatomy::chord;
use implant_layout::screws::sizing::{
    GridSize, choose_diameter, choose_length, default_width_ratio, diameter_grid, length_grid,
};
use made_up_spines::patient::make_up_patient;
use shapes::implant_layout::{ChildFacts, CurveSide, LayoutInput, NamingDoubt, ValueStatus};
use shapes::levels::Level;
use shapes::made_up::RecipeName;
use shapes::spine_fit::{
    FitAngles, FitRmsMm, FitStandardDeviations, FitStopReason, FittedVertebra, InferredFit,
    SpineFit,
};
use shapes::spine_readings::BodySide;

fn value(table: Table, key: &str, group: &str) -> PublishedValue {
    tables()
        .lookup(
            table,
            &Coordinate {
                key: key.into(),
                group: Some(group.into()),
                ..Coordinate::default()
            },
        )
        .expect("the rule's source cell")
        .value
        .clone()
}

fn scalar(table: Table, key: &str, group: &str) -> f64 {
    match value(table, key, group) {
        PublishedValue::Scalar { value } => value,
        _ => panic!("a scalar rule"),
    }
}

fn range(table: Table, key: &str, group: &str) -> [f64; 2] {
    match value(table, key, group) {
        PublishedValue::Range { lower, upper } => [lower, upper],
        _ => panic!("a range rule"),
    }
}

fn fits_at_least_as_large(after: Option<f64>, before: Option<f64>) -> bool {
    match (after, before) {
        (_, None) => true,
        (Some(a), Some(b)) => a >= b,
        (None, Some(_)) => false,
    }
}

#[test]
fn exactly_one_of_ten_versions_below_the_threshold_still_qualifies() {
    let grid = diameter_grid();
    assert!(grid.len() >= 3);
    // k=1 makes the hand-worked equality boundary exact. Nine widths
    // equal the second size; one equals the first. Every larger size
    // fails in all ten versions; the second fails in exactly one.
    let mut widths = vec![grid[1].mm; 10];
    widths[0] = grid[0].mm;
    let choice = choose_diameter(&widths, &grid, 1.0, 0.1).expect("a choice");
    assert_eq!(choice.mm, Some(grid[1].mm));
    assert_eq!(choice.also_seen, vec![grid[0].mm]);
    assert_eq!(
        choose_diameter(&widths, &grid, 1.0, 0.09)
            .expect("a stricter choice")
            .mm,
        Some(grid[0].mm)
    );
}

#[test]
fn wider_pedicle_priors_never_give_smaller_screws() {
    let grid = diameter_grid();
    let (k, _) = default_width_ratio().expect("the published rule");
    let widths: Vec<_> = (0..32)
        .map(|i| (grid[0].mm + i as f64 * 0.07) / k)
        .collect();
    let first = choose_diameter(&widths, &grid, k, 0.1)
        .expect("an initial size")
        .mm;
    for increase in [0.1, 0.25, 0.75, 2.0] {
        let wider: Vec<_> = widths.iter().map(|w| w + increase).collect();
        let after = choose_diameter(&wider, &grid, k, 0.1)
            .expect("a wider prior")
            .mm;
        assert!(fits_at_least_as_large(after, first));
    }
}

#[test]
fn larger_width_ratios_never_give_smaller_screws() {
    let grid = diameter_grid();
    let widths: Vec<_> = (0..32)
        .map(|i| grid[0].mm * 0.7 + i as f64 * 0.08)
        .collect();
    let mut ratios = vec![
        scalar(
            Table::DiameterRules,
            "outer-width-ratio-max",
            "adult-cadaver",
        ),
        range(Table::DiameterRules, "outer-width-ratio", "upendra-cited")[1],
        default_width_ratio().expect("the default rule").0,
    ];
    ratios.sort_by(f64::total_cmp);
    let mut previous = None;
    for k in ratios {
        let choice = choose_diameter(&widths, &grid, k, 0.1)
            .expect("a width ratio")
            .mm;
        assert!(fits_at_least_as_large(choice, previous));
        previous = choice;
    }
}

#[test]
fn smaller_allowed_fractions_never_give_larger_screws() {
    let grid = diameter_grid();
    let (k, _) = default_width_ratio().expect("the default rule");
    let widths: Vec<_> = (0..32)
        .map(|i| (grid[0].mm * 0.8 + i as f64 * 0.1) / k)
        .collect();
    let mut previous = None;
    for alpha in [0.0, 0.05, 0.1, 0.2, 0.4] {
        let choice = choose_diameter(&widths, &grid, k, alpha)
            .expect("a permitted fraction")
            .mm;
        assert!(fits_at_least_as_large(choice, previous));
        previous = choice;
    }
}

#[test]
fn longer_chords_never_give_shorter_screws() {
    let grid = length_grid();
    let share = range(Table::LengthRules, "chord-penetration", "krag-cited")[0] / 100.0;
    let margin = scalar(Table::LengthRules, "anterior-breach-threshold", "aorta");
    let chords: Vec<_> = (0..32)
        .map(|i| grid[0].mm / share + i as f64 * 0.3)
        .collect();
    let first = choose_length(&chords, &grid, share, margin, 0.1)
        .expect("an initial length")
        .mm;
    for increase in [0.1, 0.5, 2.0, 10.0] {
        let longer: Vec<_> = chords.iter().map(|c| c + increase).collect();
        let after = choose_length(&longer, &grid, share, margin, 0.1)
            .expect("longer chords")
            .mm;
        assert!(fits_at_least_as_large(after, first));
    }
}

#[test]
fn the_anterior_margin_caps_lengths_before_rounding_down() {
    let grid = length_grid();
    let margin = scalar(Table::LengthRules, "anterior-breach-threshold", "aorta");
    let target = grid[2].mm;
    // With share=1, the margin branch is the limiting branch. A limit
    // halfway between adjacent entries must take the lower entry.
    let half_step = (grid[3].mm - grid[2].mm) * 0.5;
    let chords = vec![target + half_step + margin; 10];
    assert_eq!(
        choose_length(&chords, &grid, 1.0, margin, 0.1)
            .expect("a capped length")
            .mm,
        Some(target)
    );
    let too_short = vec![grid[0].mm * 0.5; 10];
    assert_eq!(
        choose_length(&too_short, &grid, 1.0, margin, 0.1)
            .expect("no covered length")
            .mm,
        None
    );
}

#[test]
fn the_length_count_and_also_seen_sizes_use_the_same_ten_version_boundary() {
    let grid = length_grid();
    let margin = scalar(Table::LengthRules, "anterior-breach-threshold", "aorta");
    let mut chords = vec![grid[1].mm + margin; 10];
    chords[0] = grid[0].mm + margin;
    let choice = choose_length(&chords, &grid, 1.0, margin, 0.1).expect("a choice");
    assert_eq!(choice.mm, Some(grid[1].mm));
    assert_eq!(choice.also_seen, vec![grid[0].mm]);
    assert_eq!(
        choose_length(&chords, &grid, 1.0, margin, 0.09)
            .expect("a stricter choice")
            .mm,
        Some(grid[0].mm)
    );
}

#[test]
fn every_offered_size_resolves_to_a_source_and_missing_anatomy_stays_missing() {
    for size in diameter_grid().into_iter().chain(length_grid()) {
        assert!(!size.sources.is_empty());
        assert!(
            size.sources
                .iter()
                .all(|key| tables().source(key).is_some())
        );
    }
    for level in [Level::L1, Level::L3, Level::L5] {
        assert!(width_prior(level, Some(10.0), CurveSide::Concave).is_none());
        assert!(chord(level, CurveSide::Concave, Some(10.0)).is_none());
    }
    assert!(width_prior(Level::T13, None, CurveSide::Convex).is_none());
    assert!(width_prior(Level::T4, Some(3.0), CurveSide::Convex).is_none());
}

fn made_up_fit() -> SpineFit {
    let patient = make_up_patient(RecipeName::DoubleMajor55To45, 3).expect("a made-up spine");
    let vertebrae = patient
        .standing
        .levels
        .iter()
        .copied()
        .filter(|level| (Level::T1..=Level::L5).contains(level) && *level != Level::T13)
        .map(|level| {
            let pose = patient
                .standing
                .vertebra(level)
                .expect("a fitted level")
                .pose(level);
            let rotation =
                forward_model::rotation::matrix_from_rotation_vector(&pose.rotation.into());
            FittedVertebra {
                level,
                pose,
                angles: FitAngles {
                    side_tilt_rad: libm::atan2(-rotation[(1, 2)], rotation[(2, 2)]),
                    forward_lean_rad: libm::asin(rotation[(0, 2)].clamp(-1.0, 1.0)),
                    twist_rad: libm::atan2(-rotation[(0, 1)], rotation[(0, 0)]),
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
    SpineFit {
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
    }
}

#[test]
fn a_practice_range_plausibility_comparison_preserves_conflicting_table_rows() {
    // This is a plausibility comparison of central table lengths across
    // a made-up adolescent spine's levels, not a test of anatomical truth.
    // Some published rows disagree with regional usual-use ranges. Those
    // disagreements remain visible: the usual range never clamps a size.
    let fit = made_up_fit();
    let grid = length_grid();
    let share = range(Table::LengthRules, "chord-penetration", "krag-cited")[0] / 100.0;
    let margin = scalar(Table::LengthRules, "anterior-breach-threshold", "aorta");
    let groups = [
        "balikci-2019-t1-t5",
        "balikci-2019-t6-t12",
        "balikci-2019-l1-l5",
    ];
    let mut inside = [0; 3];
    let mut outside = 0;
    for vertebra in &fit.vertebrae {
        let region = if vertebra.level <= Level::T5 {
            0
        } else if vertebra.level <= Level::T12 {
            1
        } else {
            2
        };
        let usual = range(Table::LengthRules, "usual-length", groups[region]);
        for side in [CurveSide::Concave, CurveSide::Convex] {
            let prior = chord(vertebra.level, side, None).expect("the adolescent chord table");
            let chosen = choose_length(&[prior.mean], &grid, share, margin, 0.1)
                .expect("a table length")
                .mm;
            if chosen.is_some_and(|length| (usual[0]..=usual[1]).contains(&length)) {
                inside[region] += 1;
            } else {
                outside += 1;
            }
            // Independent oracle: the grid's next larger entry must be
            // beyond at least one of the two length limits.
            if let Some(length) = chosen {
                assert!(length <= share * prior.mean && length <= prior.mean - margin);
                assert!(grid.iter().filter(|entry| entry.mm > length).all(|entry| {
                    entry.mm > share * prior.mean || entry.mm > prior.mean - margin
                }));
            }
        }
    }
    assert!(
        inside.iter().all(|count| *count > 0),
        "each region has representative usual sizes"
    );
    assert!(
        outside > 0,
        "conflicting rows are exposed rather than clamped"
    );

    // The uppermost published mean chord itself is too short for the
    // usual lower length after the chosen share. A corrected cell is
    // always read again, never pinned to a typed numerical expectation.
    let t1 = chord(Level::T1, CurveSide::Concave, None).expect("T1 chord");
    let usual = range(Table::LengthRules, "usual-length", groups[0]);
    let choice = choose_length(&[t1.mean], &grid, share, margin, 0.1).expect("T1 length");
    if share * t1.mean < usual[0] || t1.mean - margin < usual[0] {
        assert_eq!(choice.mm, None);
    }
}

#[test]
fn a_whole_millimetre_cell_adds_only_its_shared_rounding_draw() {
    let fit = made_up_fit();
    let cell = tables()
        .cells(Table::PedicleWidths)
        .iter()
        .find(|cell| cell.coordinate.group.as_deref() == Some("gao-controls") && cell.whole_mm)
        .expect("a rounded control cell");
    let level = cell.coordinate.level.expect("a level");
    let mean = match cell.value {
        PublishedValue::PlusMinus { value, .. } | PublishedValue::Scalar { value } => value,
        PublishedValue::MeanSd { mean, .. } => mean,
        _ => panic!("a width centre"),
    };
    let sampler = NamedSampler::new(7, 32).expect("an ensemble");
    let version = Version {
        index: 2,
        fit,
        z_child: 0.0,
        pedicles: [BodySide::Left, BodySide::Right]
            .into_iter()
            .map(|side| PedicleDraw {
                level,
                side,
                z_pedicle: 0.0,
            })
            .collect(),
    };
    let mut widths = Vec::new();
    for side in [BodySide::Left, BodySide::Right] {
        widths.push(
            drawn_outer_width(
                level,
                side,
                None,
                CurvePosition::OUTSIDE,
                &version,
                &sampler,
                None,
            )
            .expect("a draw")
            .expect("a table width"),
        );
    }
    assert_eq!(
        widths[0], widths[1],
        "one rounding draw per cell and version"
    );
    let half = implant_layout::pedicles::ROUNDING_HALF_MM;
    assert!((widths[0] - mean).abs() <= half);
    assert_ne!(widths[0], mean, "the rounding term is actually applied");
}

#[test]
fn malformed_ensembles_do_not_turn_into_sizes() {
    let grid: Vec<GridSize> = diameter_grid();
    for widths in [vec![], vec![f64::NAN], vec![-1.0], vec![0.0]] {
        assert!(choose_diameter(&widths, &grid, 1.0, 0.1).is_err());
    }
    assert!(choose_diameter(&[5.0], &grid, 0.0, 0.1).is_err());
    assert!(choose_diameter(&[5.0], &grid, 1.0, -0.1).is_err());
}

#[test]
fn actual_version_based_sizes_are_compared_with_practice_without_clamping() {
    // A plausibility comparison only: these published practice ranges
    // describe series and are not anatomical bounds. Default ensemble
    // rules can disagree with them, and those departures are preserved.
    let input = LayoutInput {
        fit: made_up_fit(),
        front: None,
        bend_left: None,
        bend_right: None,
        child: ChildFacts {
            age_years: Some(15.0),
            weight_kg: None,
            activity: None,
        },
        naming: NamingDoubt::AsRead,
        versions: 8,
        seed: 17,
        pedicle_child_share: None,
    };
    let draws =
        implant_layout::readings::readings(&input, 0..input.versions).expect("made-up versions");
    let mut diameter_inside = 0;
    let mut diameter_outside = 0;
    let mut length_inside = [0; 3];
    let mut length_outside = 0;
    let diameter_grid = diameter_grid();
    let length_grid = length_grid();
    for fitted in &input.fit.vertebrae {
        let level = fitted.level;
        let diameter_group = if level <= Level::T4 {
            "jiang-t1-t4"
        } else if level <= Level::T8 {
            "jiang-t5-t8"
        } else if level <= Level::T12 {
            "jiang-t9-t12"
        } else {
            "balikci"
        };
        let diameter_range = match value(Table::ScrewGrids, "diameter", diameter_group) {
            PublishedValue::Range { lower, upper } => [lower, upper],
            PublishedValue::Scalar { value } => [value, value],
            _ => panic!("a published practice diameter range"),
        };
        let region = if level <= Level::T5 {
            0
        } else if level <= Level::T12 {
            1
        } else {
            2
        };
        for side in [BodySide::Left, BodySide::Right] {
            let anchor = implant_layout::screws::anchor(
                &input,
                &draws.versions.versions,
                &draws.measured,
                level,
                side,
            )
            .expect("a candidate anchor");
            if let Some(mm) = anchor.diameter_mm {
                assert_eq!(anchor.diameter_status, ValueStatus::FromTables);
                assert!(diameter_grid.iter().any(|entry| entry.mm == mm));
                if (diameter_range[0]..=diameter_range[1]).contains(&mm) {
                    diameter_inside += 1;
                } else {
                    diameter_outside += 1;
                }
            }
            if let Some(mm) = anchor.length_mm {
                assert_eq!(anchor.length_status, ValueStatus::FromTables);
                assert!(length_grid.iter().any(|entry| entry.mm == mm));
                let usual = anchor
                    .usual_length_mm
                    .expect("the region's sourced usual range");
                if (usual[0]..=usual[1]).contains(&mm) {
                    length_inside[region] += 1;
                } else {
                    length_outside += 1;
                }
            }
            assert!(
                anchor
                    .sources
                    .iter()
                    .all(|key| tables().source(key).is_some())
            );
        }
    }
    assert!(
        diameter_inside > 0,
        "representative selected diameters occur in the series' ranges"
    );
    assert!(diameter_outside > 0, "diameter departures remain visible");
    assert!(
        length_inside.iter().all(|count| *count > 0),
        "representative selected lengths in each region"
    );
    // Length departures are permitted; their number depends on the
    // corrected source cells, without changing the rule under test.
    assert!(length_inside.iter().sum::<i32>() + length_outside > 0);
}
