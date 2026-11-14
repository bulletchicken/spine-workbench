//! The load bench: placements put under load on versions of the spine, and
//! compared.
//!
//! Films cannot show how stiff a spine is, so no placement gets a single
//! number. Every placement is run on the same versions of the spine (the
//! versions the placement proposals counted over, plus a draw of tissue
//! stiffness, bone strength, rod yield, weight and steps a day), and
//! placements are compared by paired counts and ranges. Forces scale with
//! the stiffness the films cannot show, so absolute forces are tagged
//! assumed. Lengths are in millimetres, forces in newtons, stresses in
//! megapascals, angles in degrees, positions in the patient frame.

use crate::implant_layout::{ChildFacts, ImplantLayout};
use crate::levels::Level;
use crate::made_up::VertebraPose;
use crate::spine_fit::SpineFit;
use crate::spine_readings::{Band, BodySide};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ---- One batch of runs

/// A slice of versions to run every placement on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BenchInput {
    pub fit: SpineFit,
    pub layouts: Vec<ImplantLayout>,
    pub child: ChildFacts,
    /// The same seed and number of versions the placement proposals used,
    /// so version k is the same spine in both.
    pub seed: u32,
    pub versions: u32,
    /// The first version of this slice, from 0.
    pub version_from: u32,
    pub version_count: u32,
}

/// Every placement run on one slice of versions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BenchBatch {
    pub version_from: u32,
    pub version_count: u32,
    /// One per placement and version, placement by placement.
    pub runs: Vec<BenchRun>,
}

/// One placement on one version.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BenchRun {
    pub layout_id: String,
    pub version: u32,
    /// The solve met its stopping rule within its fixed limits.
    pub settled: bool,
    /// How far letting the rods flex moved the shape, over how far the
    /// rigid rods moved it.
    pub stiff_rod_share: f64,
    pub draws: VersionDraws,
    /// One per stop, in the order of [`BenchStop`].
    pub stops: Vec<StopValues>,
}

/// What this version drew for everything the films cannot show (its
/// geometry and pedicle widths aside).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionDraws {
    /// Every joint's stiffness over the published table.
    pub stiffness_scale: f64,
    /// The rib cage's stiffening of the thoracic joints: flexion, lateral
    /// bending, axial rotation.
    pub rib_factors: [f64; 3],
    /// The share of published anchor strength this bone holds.
    pub bone_factor: f64,
    /// As given, or drawn when not given.
    pub weight_kg: f64,
    pub steps_per_day: f64,
    /// Walking's peak load over standing.
    pub walking_multiple: f64,
    /// The yield stress of this placement's rod material.
    pub rod_yield_mpa: f64,
    /// The joints' stiffening under standing load.
    pub preload_factor: f64,
    /// How far in front of each body centre the trunk's weight acts.
    pub slice_offset_mm: f64,
    /// The share of a rod's fatigue level left after contouring.
    pub contour_knock_down: f64,
}

/// Where the story of a placement stands.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum BenchStop {
    /// The standing spine, before the rods.
    Before,
    /// The rods in and locked, with no weight on the spine, as on the
    /// table.
    RodsIn,
    /// The locked-in loads and the standing load.
    Standing,
    /// The locked-in loads and the standing load made bigger.
    BendingForward,
    /// A year of steps against the level rods survived on the test bench.
    YearOfWalking,
}

/// One placement on one version at one stop.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StopValues {
    pub stop: BenchStop,
    /// The resultant force between bone and anchor, one per anchor in the
    /// placement's order; 0 for an empty pedicle.
    pub anchor_force_n: Vec<f64>,
    /// Each anchor's force over its published strength, the bone factor
    /// included.
    pub anchor_demand: Vec<f64>,
    /// Each rod span's peak von Mises stress over the rod's yield: the
    /// left rod's spans top to bottom, then the right rod's.
    pub rod_span_use: Vec<f64>,
    /// The joint moment between the upper end and the vertebra above it,
    /// over the same load on the spine before the rods; `null` with no
    /// load to compare (before, rods in) or no vertebra above.
    pub junction_above: Option<f64>,
    /// The same between the lower end and the level below it.
    pub junction_below: Option<f64>,
    pub main_cobb_deg: f64,
    /// T5 to T12.
    pub kyphosis_deg: f64,
    pub apical_twist_deg: f64,
    /// Every vertebra's pose, top to bottom: at the before and rods-in
    /// stops; `null` at stops where the shape does not move.
    pub poses: Option<Vec<VertebraPose>>,
    /// At a year of walking, each rod's swing; `null` at other stops.
    pub rod_swing: Option<Vec<RodSwing>>,
    /// At a year of walking, the steps in the year; `null` at other stops.
    pub cycles_per_year: Option<f64>,
}

/// One rod's Goodman-equivalent stress swing over the level rods of its
/// material survived on the test bench, contouring included.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RodSwing {
    pub side: BodySide,
    /// `null` for a material with no published test-bench level.
    pub ratio: Option<f64>,
}

// ---- Comparing placements

/// Every batch for a set of placements, to be compared. `layouts[0]` is
/// the placement the others are compared with.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BenchCompareInput {
    pub layouts: Vec<ImplantLayout>,
    pub batches: Vec<BenchBatch>,
}

/// Ranges, tags, shares and paired counts for every placement.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BenchResult {
    pub versions: u32,
    /// The placement the others are compared with.
    pub first_id: String,
    pub layouts: Vec<LayoutResult>,
    /// One scale per measure, shared by every placement's ranges.
    pub scales: Vec<MeasureScale>,
}

/// One placement over every version.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayoutResult {
    pub layout_id: String,
    /// The versions whose solve did not settle.
    pub versions_unsettled: u32,
    pub stops: Vec<StopResult>,
    /// Against the first placement; empty for the first placement itself.
    pub counts: Vec<PairedCount>,
}

/// One placement at one stop.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StopResult {
    pub stop: BenchStop,
    pub estimates: Vec<Estimate>,
    pub parts: Vec<PartResult>,
    /// Two spines: the versions at the 10th and the 90th percentile of the
    /// main curve; `null` at stops where the shape does not move.
    pub ghosts: Option<Vec<Vec<VertebraPose>>>,
    /// At a year of walking, the steps in a year; `null` at other stops.
    pub cycles_per_year: Option<Band>,
    /// Versions where the rods are too soft for the rigid-rod method.
    pub outside_stiff_rod: u32,
}

/// One estimate over the versions.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Estimate {
    pub measure: Measure,
    pub band: Band,
    pub tag: EstimateTag,
    /// The drawn unknown it moves with most, by rank correlation.
    pub mostly_from: SampledUnknown,
}

/// What a part carries.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PartResult {
    pub part: PartRef,
    /// Its load over the heaviest load any part of its kind carries, at
    /// any stop, in any placement compared, per version.
    pub share: Band,
    /// The share holds still across the versions.
    pub steady: bool,
    /// Versions where it goes past its published strength (for a rod at
    /// a year of walking, past the test-bench level).
    pub versions_past_strength: u32,
}

/// A part of a placement.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum PartRef {
    Anchor {
        level: Level,
        side: BodySide,
    },
    /// The rod between two neighbouring heads.
    RodSpan {
        side: BodySide,
        from: Level,
        to: Level,
    },
    /// A whole rod, for its swing at a year of walking.
    Rod {
        side: BodySide,
    },
    /// The joint next to the construct.
    Junction {
        which: JunctionSide,
    },
}

/// Which end of the construct a junction is at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum JunctionSide {
    Above,
    Below,
}

/// A placement against the first placement on the same versions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PairedCount {
    pub measure: Measure,
    pub stop: BenchStop,
    /// Versions where this placement is lower by more than the margin.
    pub lower: u32,
    /// Versions where it is higher by more than the margin.
    pub higher: u32,
    /// Versions within the margin.
    pub ties: u32,
    /// The versions in the order the dots are drawn.
    pub order: Vec<u32>,
    /// The drawn unknown the order follows.
    pub sorted_by: SampledUnknown,
}

/// How far the films settle an estimate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum EstimateTag {
    /// Narrow whatever the unknowns.
    SetByTheFilms,
    /// Wide, but its difference from the first placement keeps its sign
    /// in at least 29 of 32 versions.
    ComparisonOnly,
    /// Rests on stated priors.
    Assumed,
}

/// Something each version draws that the films cannot show.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum SampledUnknown {
    /// Fit noise in each vertebra's place and angles.
    Geometry,
    StiffnessScale,
    RibCage,
    Bone,
    Weight,
    Steps,
    Walking,
    RodYield,
    Preload,
    SliceOffset,
    ContourKnockDown,
}

/// What the bench estimates and compares.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum Measure {
    /// The main curve's Cobb angle, degrees.
    MainCobb,
    /// T5 to T12 kyphosis, degrees.
    Kyphosis,
    /// The main curve's apical twist, degrees.
    ApicalTwist,
    /// The largest anchor force, newtons.
    HeaviestAnchorForce,
    /// The largest anchor force over its published strength.
    HeaviestAnchorDemand,
    /// The largest anchor force over the median anchor force.
    HeaviestAnchorOverMedian,
    /// The largest rod span use.
    HeaviestRodUse,
    /// The junction ratio above the construct.
    JunctionAbove,
    /// The junction ratio below the construct.
    JunctionBelow,
    /// The largest rod swing ratio at a year of walking.
    HeaviestRodSwing,
}

/// One scale for a measure's ranges, shared by every placement compared:
/// the lowest 10th and the highest 90th percentile over every stop.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeasureScale {
    pub measure: Measure,
    pub lowest: f64,
    pub highest: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::implant_layout::{ActivityLevel, LayoutChange};
    use crate::spine_fit::FitStopReason;
    use crate::spine_fit::{FilmPlacement, FitLengths};
    use serde_json::json;

    /// The one place this file builds a fit: no vertebrae.
    fn empty_fit() -> SpineFit {
        SpineFit {
            vertebrae: Vec::new(),
            side_scale: 1.0,
            side_offset_mm: 0.0,
            origin_level: Level::L5,
            iterations: 0,
            converged: true,
            stop_reason: FitStopReason::SmallStep,
            rms_mm: 0.0,
            lengths: FitLengths::Measured,
            typical_curve_deg: None,
            front_film: FilmPlacement {
                mm_per_px: [0.2, 0.2],
                origin_px: [512.0, 1800.0],
                facing_assumed: false,
            },
            side_film: None,
        }
    }

    fn layout() -> ImplantLayout {
        ImplantLayout {
            id: "first".to_string(),
            change: LayoutChange::First {},
            upper: Level::T4,
            lower: Level::L1,
            anchors: Vec::new(),
            rods: Vec::new(),
            released_levels: Vec::new(),
        }
    }

    fn input() -> BenchInput {
        BenchInput {
            fit: empty_fit(),
            layouts: vec![layout()],
            child: ChildFacts {
                age_years: None,
                weight_kg: Some(42.0),
                activity: Some(ActivityLevel::Typical),
            },
            seed: 7,
            versions: 32,
            version_from: 4,
            version_count: 4,
        }
    }

    fn pose() -> VertebraPose {
        VertebraPose {
            level: Level::L5,
            centre: [0.0, 0.0, 15.0],
            rotation: [0.0; 3],
            height: 30.0,
            wedge_front: 0.0,
            wedge_side: 0.0,
            top_half_width: 25.0,
            top_half_depth: 18.0,
            bottom_half_width: 25.0,
            bottom_half_depth: 18.0,
        }
    }

    fn batch() -> BenchBatch {
        BenchBatch {
            version_from: 4,
            version_count: 1,
            runs: vec![BenchRun {
                layout_id: "first".to_string(),
                version: 4,
                settled: true,
                stiff_rod_share: 0.1,
                draws: VersionDraws {
                    stiffness_scale: 2.0,
                    rib_factors: [1.5, 1.6, 2.0],
                    bone_factor: 1.0,
                    weight_kg: 42.0,
                    steps_per_day: 9000.0,
                    walking_multiple: 1.4,
                    rod_yield_mpa: 900.0,
                    preload_factor: 1.8,
                    slice_offset_mm: 28.0,
                    contour_knock_down: 0.7,
                },
                stops: vec![
                    StopValues {
                        stop: BenchStop::RodsIn,
                        anchor_force_n: vec![120.0, 0.0],
                        anchor_demand: vec![0.2, 0.0],
                        rod_span_use: vec![0.3],
                        junction_above: None,
                        junction_below: None,
                        main_cobb_deg: 15.0,
                        kyphosis_deg: 25.0,
                        apical_twist_deg: 8.0,
                        poses: Some(vec![pose()]),
                        rod_swing: None,
                        cycles_per_year: None,
                    },
                    StopValues {
                        stop: BenchStop::YearOfWalking,
                        anchor_force_n: vec![150.0, 0.0],
                        anchor_demand: vec![0.25, 0.0],
                        rod_span_use: vec![0.4],
                        junction_above: Some(1.2),
                        junction_below: Some(1.1),
                        main_cobb_deg: 15.0,
                        kyphosis_deg: 25.0,
                        apical_twist_deg: 8.0,
                        poses: None,
                        rod_swing: Some(vec![RodSwing {
                            side: BodySide::Left,
                            ratio: None,
                        }]),
                        cycles_per_year: Some(3_285_000.0),
                    },
                ],
            }],
        }
    }

    fn band(p50: f64) -> Band {
        Band {
            p10: p50 - 1.0,
            p50,
            p90: p50 + 1.0,
        }
    }

    fn result() -> BenchResult {
        BenchResult {
            versions: 32,
            first_id: "first".to_string(),
            layouts: vec![LayoutResult {
                layout_id: "fewer-screws".to_string(),
                versions_unsettled: 0,
                stops: vec![StopResult {
                    stop: BenchStop::YearOfWalking,
                    estimates: vec![Estimate {
                        measure: Measure::HeaviestAnchorForce,
                        band: band(200.0),
                        tag: EstimateTag::Assumed,
                        mostly_from: SampledUnknown::StiffnessScale,
                    }],
                    parts: vec![
                        PartResult {
                            part: PartRef::Anchor {
                                level: Level::T8,
                                side: BodySide::Left,
                            },
                            share: band(0.8),
                            steady: true,
                            versions_past_strength: 3,
                        },
                        PartResult {
                            part: PartRef::RodSpan {
                                side: BodySide::Right,
                                from: Level::T8,
                                to: Level::T9,
                            },
                            share: band(0.5),
                            steady: false,
                            versions_past_strength: 0,
                        },
                        PartResult {
                            part: PartRef::Junction {
                                which: JunctionSide::Above,
                            },
                            share: band(0.4),
                            steady: true,
                            versions_past_strength: 0,
                        },
                        PartResult {
                            part: PartRef::Rod {
                                side: BodySide::Left,
                            },
                            share: band(0.6),
                            steady: true,
                            versions_past_strength: 2,
                        },
                    ],
                    ghosts: Some(vec![vec![pose()], vec![pose()]]),
                    cycles_per_year: Some(band(3_000_000.0)),
                    outside_stiff_rod: 5,
                }],
                counts: vec![PairedCount {
                    measure: Measure::HeaviestAnchorForce,
                    stop: BenchStop::Standing,
                    lower: 26,
                    higher: 4,
                    ties: 2,
                    order: (0..32).collect(),
                    sorted_by: SampledUnknown::StiffnessScale,
                }],
            }],
            scales: vec![MeasureScale {
                measure: Measure::MainCobb,
                lowest: 12.0,
                highest: 44.0,
            }],
        }
    }

    #[test]
    fn a_batch_names_its_units_and_writes_every_stop_value() {
        let value = serde_json::to_value(batch()).expect("the batch serialises");
        let run = &value["runs"][0];
        assert_eq!(run["draws"]["rodYieldMpa"], 900.0);
        assert_eq!(run["draws"]["ribFactors"], json!([1.5, 1.6, 2.0]));
        let rods_in = &run["stops"][0];
        assert_eq!(rods_in["stop"], "rods-in");
        assert_eq!(rods_in["anchorForceN"], json!([120.0, 0.0]));
        assert_eq!(rods_in["junctionAbove"], serde_json::Value::Null);
        assert_eq!(rods_in["rodSwing"], serde_json::Value::Null);
        let year = &run["stops"][1];
        assert_eq!(year["stop"], "year-of-walking");
        assert_eq!(year["rodSwing"], json!([{"side": "left", "ratio": null}]));
        assert_eq!(year["poses"], serde_json::Value::Null);
        assert_eq!(
            serde_json::from_value::<BenchBatch>(value).expect("the batch reads back"),
            batch()
        );
        let value = serde_json::to_value(input()).expect("the input serialises");
        assert_eq!(value["versionFrom"], 4);
        assert_eq!(
            serde_json::from_value::<BenchInput>(value).expect("the input reads back"),
            input()
        );
    }

    #[test]
    fn a_result_tags_parts_and_counts_and_reads_back() {
        let value = serde_json::to_value(result()).expect("the result serialises");
        let stop = &value["layouts"][0]["stops"][0];
        assert_eq!(stop["estimates"][0]["tag"], "assumed");
        assert_eq!(stop["estimates"][0]["mostlyFrom"], "stiffness-scale");
        assert_eq!(
            stop["parts"][1]["part"],
            json!({"kind": "rod-span", "side": "right", "from": "T8", "to": "T9"})
        );
        assert_eq!(
            stop["parts"][2]["part"],
            json!({"kind": "junction", "which": "above"})
        );
        assert_eq!(
            stop["parts"][3]["part"],
            json!({"kind": "rod", "side": "left"})
        );
        let count = &value["layouts"][0]["counts"][0];
        assert_eq!(count["measure"], "heaviest-anchor-force");
        assert_eq!(count["sortedBy"], "stiffness-scale");
        let compare = BenchCompareInput {
            layouts: vec![layout()],
            batches: vec![batch()],
        };
        let compare_value = serde_json::to_value(&compare).expect("the comparison serialises");
        assert_eq!(
            serde_json::from_value::<BenchCompareInput>(compare_value)
                .expect("the comparison reads back"),
            compare
        );
        assert_eq!(
            serde_json::from_value::<BenchResult>(value).expect("the result reads back"),
            result()
        );
    }

    #[test]
    fn unknown_fields_are_refused_at_each_new_boundary() {
        let value = serde_json::to_value(input()).expect("the input serialises");
        for path in ["", "/child", "/layouts/0"] {
            let mut changed = value.clone();
            changed
                .pointer_mut(path)
                .and_then(serde_json::Value::as_object_mut)
                .expect("an object")
                .insert("extra".to_string(), json!(1));
            assert!(
                serde_json::from_value::<BenchInput>(changed).is_err(),
                "{path}"
            );
        }

        let value = serde_json::to_value(batch()).expect("the batch serialises");
        for path in [
            "",
            "/runs/0",
            "/runs/0/draws",
            "/runs/0/stops/0",
            "/runs/0/stops/1/rodSwing/0",
        ] {
            let mut changed = value.clone();
            changed
                .pointer_mut(path)
                .and_then(serde_json::Value::as_object_mut)
                .expect("an object")
                .insert("extra".to_string(), json!(1));
            assert!(
                serde_json::from_value::<BenchBatch>(changed).is_err(),
                "{path}"
            );
        }

        let value = serde_json::to_value(BenchCompareInput {
            layouts: vec![layout()],
            batches: vec![batch()],
        })
        .expect("the comparison serialises");
        let mut changed = value.clone();
        changed
            .as_object_mut()
            .expect("an object")
            .insert("extra".to_string(), json!(1));
        assert!(serde_json::from_value::<BenchCompareInput>(changed).is_err());

        let value = serde_json::to_value(result()).expect("the result serialises");
        for path in [
            "",
            "/layouts/0",
            "/layouts/0/stops/0",
            "/layouts/0/stops/0/estimates/0",
            "/layouts/0/stops/0/estimates/0/band",
            "/layouts/0/stops/0/parts/0",
            "/layouts/0/stops/0/parts/0/part",
            "/layouts/0/stops/0/parts/1/part",
            "/layouts/0/stops/0/parts/2/part",
            "/layouts/0/stops/0/parts/3/part",
            "/layouts/0/counts/0",
            "/scales/0",
        ] {
            let mut changed = value.clone();
            changed
                .pointer_mut(path)
                .and_then(serde_json::Value::as_object_mut)
                .expect("an object")
                .insert("extra".to_string(), json!(1));
            assert!(
                serde_json::from_value::<BenchResult>(changed).is_err(),
                "{path}"
            );
        }
    }
}
