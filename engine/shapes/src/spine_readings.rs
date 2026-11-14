//! Angles on a fitted spine, with the reading on the fit separate from the
//! uncertainty band. Positions use x forward, y patient left, z up;
//! every angle here is in degrees. Typical values stay in not-measured.

use crate::levels::Level;
use crate::made_up::CurveRegion;
use crate::spine_fit::{FilmToFit, SpineFit};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Which films the fit stands on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum FitTier {
    /// Front and side films, with or without a length scale.
    TwoFilms,
    /// A front film alone: forward lean, depth, side wedge and twist are
    /// typical unless independently measured, and never vote as measurements.
    FrontFilmOnly,
}

/// The 10th, 50th and 90th percentiles, in the named unit. Front-plane
/// angles and side-plane plate angles use closed-form normal percentiles
/// from independent plate spreads; lean and twist use sampled percentiles.
/// Bending angles keep singleton bands; bending-mark uncertainty is not
/// propagated.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Band {
    pub p10: f64,
    pub p50: f64,
    pub p90: f64,
}

/// How many versions gave one level.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LevelCount {
    pub level: Level,
    pub versions: u32,
}

/// The patient's left or right.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum BodySide {
    Left,
    Right,
}

/// The fit, films when at hand, and 1 to 256 repeatable versions. Plate
/// spreads use the supplied marks and their stated own spread; absent films
/// or levels, coincident plate corners or zero body height use sqrt(2) times
/// the fit's reported tilt or lean spread, separately for each plate.
/// That fallback gives narrower bands; placement and load-bench version
/// ensembles use it. Each inferred corner has three times the stated spread.
/// A share of height uses the marked body height. Millimetres use the film's
/// stated spacing when present; otherwise the placement spacing, with the
/// side scale removed for the side film. Shared
/// translations cancel from a plate's direction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadingsInput {
    pub fit: SpineFit,
    pub front: Option<FilmToFit>,
    pub side: Option<FilmToFit>,
    pub bend_left: Option<FilmToFit>,
    pub bend_right: Option<FilmToFit>,
    pub versions: u32,
    pub seed: u32,
}

/// Angles on the spine as fitted and their uncertainty bands. Repeated-mark
/// coverage has been measured on made-up spines with independent noise;
/// a real finder's errors may not be independent. See the crate README
/// for which kinds of reading were measured and their coverage figures.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpineReadings {
    pub tier: FitTier,
    pub versions: u32,
    pub vertebrae: Vec<VertebraReading>,
    pub sagittal: SagittalReadings,
    pub curves: Vec<CurveReading>,
    pub between: BetweenReadings,
}

/// One vertebra, top to bottom; typical twist and lean remain separate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VertebraReading {
    pub level: Level,
    pub coronal_tilt: Reading,
    pub forward_lean: Reading,
    pub twist: Reading,
}

/// Upper-to-lower endplate angles, positive for the named bend. Plate
/// spreads are independent in the side plane. L1-S1 also uses the marked
/// sacrum's millimetre spread; with a share of height, the sacrum has no
/// body height to supply its spread and the band holds that line fixed.
/// With independent 1 mm side-corner noise on three made-up spines, bands
/// held exact-mark angles 74–78% of the time for kyphoses and 65–71% for
/// lordoses, below eight in ten. Front-film mark error enters fitted side
/// tilts but is absent from bands built from the side film's spread alone.
/// Do not label these readings with the front-plane eight-in-ten sentence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SagittalReadings {
    pub kyphosis_t2_t5: Reading,
    pub kyphosis_t5_t12: Reading,
    pub kyphosis_t10_l2: Reading,
    pub lordosis_l1_l5: Reading,
    pub lordosis_l1_s1: Reading,
}

/// A front-plane curve between the most tilted end vertebrae of the fitted
/// spine. Cobb is the angle on that fitted spine between those named ends.
/// For the spread the marks were given, the band held the angle exact marks
/// would give about three times in four on the two made-up thoracic curves
/// (73% and 76%) and about eight times in ten on the lumbar one (84%). A
/// second independent marking falls inside the band less often; main-Cobb
/// second-marking coverage has not been measured. The band does not cover
/// which vertebrae are the ends; their version counts describe that separately.
/// On noisy marks, picking the most tilted plates with the same noise that
/// is measured gives a measured bias of +3.1 to +3.9 degrees at the trained
/// finder's stated noise on made-up thoracic spines. Noise can also split
/// one real curve into shorter curves. The fixed-end band does not include
/// choices of ends or curve splitting.
/// Coverage was measured on three made-up main curves with independent
/// noise, not minor curves; a real finder's errors may not be independent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CurveReading {
    pub region: CurveRegion,
    pub convex: BodySide,
    pub upper_end: Level,
    pub lower_end: Level,
    pub apex: Level,
    pub upper_end_seen: Vec<LevelCount>,
    pub lower_end_seen: Vec<LevelCount>,
    pub apex_seen: Vec<LevelCount>,
    /// Versions in which these same two end vertebrae reach the evidence floor.
    /// Endpoint counts separately describe the ends selected in each version.
    pub versions_found: u32,
    pub cobb: Reading,
    pub apex_twist: Reading,
    pub bending_cobb: Reading,
}

/// Every upper/lower pair, ordered by upper then lower.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BetweenReadings {
    pub levels: Vec<Level>,
    pub front: Vec<PairReading>,
    pub side: Vec<PairReading>,
}

/// The upper vertebra’s upper endplate against the lower vertebra’s lower endplate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PairReading {
    pub upper: Level,
    pub lower: Level,
    pub angle: Reading,
}

/// A fitted value and its uncertainty band, or what is absent and the typical
/// value used by the drawing, when one exists. For a curve, the value is its
/// Cobb angle between the fitted spine's most tilted plates in their turning
/// neighbourhoods. Every other fitted reading uses its named levels; bending
/// Cobb uses marked bending lines at the standing curve's fixed ends.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Reading {
    Measured {
        value_deg: f64,
        band: Band,
    },
    NotMeasured {
        reason: NotMeasured,
        typical_deg: Option<f64>,
    },
}

/// What would be needed to measure this angle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum NotMeasured {
    NoSideFilm,
    NotOnSideFilm,
    TwistNotMeasured,
    NoSacrum,
    NoBendingFilms,
    NoCurve,
    NotFitted,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spine_fit::{FilmPlacement, FitLengths, FitStopReason};
    use serde_json::{Value, json};

    fn measured() -> Reading {
        Reading::Measured {
            value_deg: -31.0,
            band: Band {
                p10: -33.0,
                p50: -31.0,
                p90: -29.0,
            },
        }
    }

    fn output() -> SpineReadings {
        let missing = Reading::NotMeasured {
            reason: NotMeasured::NoSideFilm,
            typical_deg: Some(12.0),
        };
        SpineReadings {
            tier: FitTier::FrontFilmOnly,
            versions: 32,
            vertebrae: vec![VertebraReading {
                level: Level::T5,
                coronal_tilt: measured(),
                forward_lean: missing,
                twist: Reading::NotMeasured {
                    reason: NotMeasured::TwistNotMeasured,
                    typical_deg: None,
                },
            }],
            sagittal: SagittalReadings {
                kyphosis_t2_t5: missing,
                kyphosis_t5_t12: missing,
                kyphosis_t10_l2: missing,
                lordosis_l1_l5: missing,
                lordosis_l1_s1: missing,
            },
            curves: vec![CurveReading {
                region: CurveRegion::MainThoracic,
                convex: BodySide::Left,
                upper_end: Level::T5,
                lower_end: Level::T10,
                apex: Level::T8,
                upper_end_seen: vec![LevelCount {
                    level: Level::T5,
                    versions: 32,
                }],
                lower_end_seen: vec![],
                apex_seen: vec![],
                versions_found: 32,
                cobb: measured(),
                apex_twist: missing,
                bending_cobb: missing,
            }],
            between: BetweenReadings {
                levels: vec![Level::T5, Level::T10],
                front: vec![PairReading {
                    upper: Level::T5,
                    lower: Level::T10,
                    angle: measured(),
                }],
                side: vec![PairReading {
                    upper: Level::T5,
                    lower: Level::T10,
                    angle: missing,
                }],
            },
        }
    }

    fn input() -> ReadingsInput {
        ReadingsInput {
            fit: SpineFit {
                vertebrae: vec![],
                side_scale: 1.0,
                side_offset_mm: 0.0,
                origin_level: Level::L5,
                iterations: 0,
                converged: true,
                stop_reason: FitStopReason::SmallStep,
                rms_mm: 0.0,
                lengths: FitLengths::TypicalSize,
                typical_curve_deg: None,
                front_film: FilmPlacement {
                    mm_per_px: [1.0; 2],
                    origin_px: [0.0; 2],
                    facing_assumed: false,
                },
                side_film: None,
            },
            front: None,
            side: None,
            bend_left: None,
            bend_right: None,
            versions: 32,
            seed: 1,
        }
    }

    #[test]
    fn readings_round_trip_with_exact_names_and_explicit_nulls() {
        let value = serde_json::to_value(output()).unwrap();
        assert_eq!(value["tier"], "front-film-only");
        assert_eq!(value["sagittal"]["kyphosisT2T5"]["typicalDeg"], 12.0);
        assert_eq!(value["vertebrae"][0]["coronalTilt"]["valueDeg"], -31.0);
        assert_eq!(value["vertebrae"][0]["twist"]["typicalDeg"], Value::Null);
        assert_eq!(
            serde_json::from_value::<SpineReadings>(value).unwrap(),
            output()
        );
        let value = serde_json::to_value(input()).unwrap();
        for name in ["front", "side", "bendLeft", "bendRight"] {
            assert_eq!(value[name], Value::Null);
        }
        assert_eq!(
            serde_json::from_value::<ReadingsInput>(value).unwrap(),
            input()
        );
    }

    #[test]
    fn unknown_fields_are_refused_at_every_reading_boundary() {
        let value = serde_json::to_value(output()).unwrap();
        for path in [
            "",
            "/vertebrae/0",
            "/vertebrae/0/coronalTilt",
            "/vertebrae/0/coronalTilt/band",
            "/vertebrae/0/forwardLean",
            "/sagittal",
            "/curves/0",
            "/curves/0/upperEndSeen/0",
            "/between",
            "/between/front/0",
            "/between/side/0/angle",
        ] {
            let mut changed = value.clone();
            changed
                .pointer_mut(path)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert("extra".into(), json!(1));
            assert!(
                serde_json::from_value::<SpineReadings>(changed).is_err(),
                "{path}"
            );
        }
        let mut value = serde_json::to_value(input()).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("extra".into(), json!(1));
        assert!(serde_json::from_value::<ReadingsInput>(value).is_err());
    }
}
