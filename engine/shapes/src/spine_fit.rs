//! Marks on the front and side films fitted together as one spine, with
//! uncertainty and the dimensions that had to come from neighbouring levels.

use crate::film_geometry::FilmGeometry;
use crate::levels::Level;
use crate::made_up::VertebraPose;
use crate::vertebra::FilmMarks;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Both films' marks in original pixels, with their known geometries and
/// stored sizes in pixels, width then height.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FitInput {
    pub front: FilmMarks,
    pub side: FilmMarks,
    pub front_geometry: FilmGeometry,
    pub side_geometry: FilmGeometry,
    pub front_size: [u32; 2],
    pub side_size: [u32; 2],
}

/// The named angles of `R = rotX(sideTilt) * rotY(forwardLean) * rotZ(twist)`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FitAngles {
    pub side_tilt_rad: f64,
    pub forward_lean_rad: f64,
    pub twist_rad: f64,
}

/// One standard deviation in the patient frame and the named angles.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FitStandardDeviations {
    /// x forward, y patient left, z up, in millimetres.
    pub centre_mm: [f64; 3],
    /// Side tilt, forward lean and twist, in radians.
    pub angles_rad: [f64; 3],
}

/// Quantities inferred from neighbouring vertebrae because their marks
/// are absent. Depth includes front-to-back position and forward lean;
/// width includes side-to-side position and side tilt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InferredFit {
    pub depth: bool,
    pub width: bool,
    pub twist: bool,
}

/// Root mean square mark error in millimetres on each film. An absent
/// view has `null`, rather than a zero that would suggest a perfect fit.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FitRmsMm {
    pub front: Option<f64>,
    pub side: Option<f64>,
}

/// A vertebra fitted from all its placed marks and the chain's priors.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FittedVertebra {
    pub level: Level,
    pub pose: VertebraPose,
    pub angles: FitAngles,
    pub sd: FitStandardDeviations,
    pub inferred: InferredFit,
    pub rms_mm: FitRmsMm,
}

/// Which stopping rule ended the solve.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum FitStopReason {
    /// Relative decrease in cost was below 1e-10.
    RelativeCost,
    /// The step was below 1e-8.
    SmallStep,
    /// The solve reached 100 iterations without either convergence rule.
    IterationLimit,
}

/// The joint fit, with centres translated to the underside of `originLevel`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpineFit {
    /// Top to bottom, matched by level name across the films.
    pub vertebrae: Vec<FittedVertebra>,
    /// Side-film scale factor relative to its stated pixel spacing.
    pub side_scale: f64,
    /// Side-film vertical registration in the provisional frame, mm.
    pub side_offset_mm: f64,
    /// L5 when fitted, otherwise the lowest fitted level.
    pub origin_level: Level,
    pub iterations: u32,
    pub converged: bool,
    pub stop_reason: FitStopReason,
    /// Root mean square error of the placed marks, before uncertainty weights.
    pub rms_mm: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn answer() -> SpineFit {
        SpineFit {
            vertebrae: vec![FittedVertebra {
                level: Level::L5,
                pose: VertebraPose {
                    level: Level::L5,
                    centre: [0.0, 0.0, 10.0],
                    rotation: [0.0; 3],
                    height: 20.0,
                    wedge_front: 0.0,
                    wedge_side: 0.0,
                    top_half_width: 15.0,
                    top_half_depth: 10.0,
                    bottom_half_width: 15.0,
                    bottom_half_depth: 10.0,
                },
                angles: FitAngles {
                    side_tilt_rad: 0.0,
                    forward_lean_rad: 0.0,
                    twist_rad: 0.0,
                },
                sd: FitStandardDeviations {
                    centre_mm: [1.0, 2.0, 3.0],
                    angles_rad: [0.01, 0.02, 0.03],
                },
                inferred: InferredFit {
                    depth: true,
                    width: false,
                    twist: false,
                },
                rms_mm: FitRmsMm {
                    front: Some(0.5),
                    side: None,
                },
            }],
            side_scale: 1.0,
            side_offset_mm: 0.0,
            origin_level: Level::L5,
            iterations: 3,
            converged: true,
            stop_reason: FitStopReason::SmallStep,
            rms_mm: 0.5,
        }
    }

    #[test]
    fn the_result_uses_named_angles_and_explicit_absent_views() {
        let result = answer();
        let value = serde_json::to_value(&result).expect("the result serialises");
        assert_eq!(value["sideScale"], 1.0);
        assert_eq!(value["sideOffsetMm"], 0.0);
        assert_eq!(value["originLevel"], "L5");
        assert_eq!(value["stopReason"], "small-step");
        let vertebra = &value["vertebrae"][0];
        assert_eq!(
            vertebra["angles"],
            json!({"sideTiltRad": 0.0, "forwardLeanRad": 0.0, "twistRad": 0.0})
        );
        assert_eq!(vertebra["sd"]["centreMm"], json!([1.0, 2.0, 3.0]));
        assert_eq!(vertebra["sd"]["anglesRad"], json!([0.01, 0.02, 0.03]));
        assert_eq!(vertebra["rmsMm"], json!({"front": 0.5, "side": null}));
        assert_eq!(
            serde_json::from_value::<SpineFit>(value).expect("the result reads back"),
            result
        );
    }

    #[test]
    fn unknown_fields_are_refused_at_each_new_boundary() {
        let value = serde_json::to_value(answer()).expect("the result serialises");
        for path in [
            "",
            "/vertebrae/0",
            "/vertebrae/0/angles",
            "/vertebrae/0/sd",
            "/vertebrae/0/inferred",
            "/vertebrae/0/rmsMm",
        ] {
            let mut changed = value.clone();
            changed
                .pointer_mut(path)
                .and_then(serde_json::Value::as_object_mut)
                .expect("an object")
                .insert("extra".to_string(), json!(1));
            assert!(
                serde_json::from_value::<SpineFit>(changed).is_err(),
                "{path}"
            );
        }
    }
}
