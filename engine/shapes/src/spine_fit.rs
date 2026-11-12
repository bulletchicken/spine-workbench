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

/// How far a placed corner strays, in two parts: one standard deviation of
/// each corner's own error, and the part of the error that all four corners
/// of one vertebra share (0 means none). Inferred corners keep their own
/// larger allowance, and a pedicle's spread is twice a corner's.
///
/// The shared part adds in quadrature to the reported standard deviation of
/// that vertebra's centre across the film and up it (centre y and centre z
/// on the front film, centre x and centre z on the side film) and changes
/// nothing else: not the fitted poses, not any other spread. A box that sits
/// off as a whole moves its centre and nothing else, and the fit, which
/// takes each corner's error as its own, cannot see it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum MarkSpread {
    /// The same spread for every corner, in millimetres, and the shared
    /// part, in millimetres.
    Millimetres { sd_mm: f64, shared_sd_mm: f64 },
    /// A share of that vertebra's own height on the film, measured from
    /// its corners, and the shared part as a share of the same height.
    ShareOfHeight { share: f64, shared_share: f64 },
}

/// One film's marks in original pixels, its geometry, its stored size in
/// pixels (width then height) and how far its corners may stray.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FilmToFit {
    pub marks: FilmMarks,
    pub geometry: FilmGeometry,
    pub size: [u32; 2],
    pub mark_spread: MarkSpread,
}

/// A twist measured outside the fit, with one standard deviation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TwistReading {
    pub level: Level,
    pub twist_rad: f64,
    pub sd_rad: f64,
}

/// Where the values the films cannot show come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum TypicalSource {
    /// Nothing: every value comes from the films.
    None,
    /// The typical spine compiled into the engine, with its spread.
    BuiltIn,
}

/// One front film, or a front and side pair, to fit as one spine.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FitFromFilms {
    pub front: FilmToFit,
    pub side: Option<FilmToFit>,
    pub typical: TypicalSource,
    pub twist_readings: Vec<TwistReading>,
}

/// Whether lengths are millimetres from a stated film scale or drawn at
/// a typical size because the film has none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum FitLengths {
    Measured,
    TypicalSize,
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
    /// The tilt read straight from the front film's four corners, in
    /// radians: the line from the middle of the body's right side to the
    /// middle of its left side, in the patient's own sense so that it
    /// compares with `sideTiltRad`. Positive when the patient's left side
    /// is higher, as `rotX(+θ)` raises it, whichever way the film faces.
    /// `null` when the vertebra has no front marks. Unlike `sideTiltRad`,
    /// which moves with the lean and twist the fit gives the body, this is
    /// what the film shows.
    pub corner_tilt_rad: Option<f64>,
}

/// Where a film stands against the fitted spine: the scale the fit used and
/// the pixel where the patient origin (the underside of `originLevel`, where
/// every reported centre is measured from) lands. With the film's own
/// facing, quarter turns and stored size, these rebuild the parallel camera
/// the fit projected through.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FilmPlacement {
    /// Millimetres per pixel along a row, then down a column, of the film
    /// as stored, as a film geometry's `mmPerPx` and `mmPerPxVertical`
    /// give them: the film's stated scale; for a front film without one,
    /// the provisional scale the fit drew it at (the same both ways); for
    /// a side film, its stated scale times `sideScale`.
    pub mm_per_px: [f64; 2],
    /// The upright pixel where the patient origin lands: the film shown
    /// after its quarter turns, before any mirroring, x right and y down,
    /// with pixel centres on whole numbers as
    /// `docs/conventions/pixel-space.md` says. A side film's includes
    /// `sideOffsetMm`.
    pub origin_px: [f64; 2],
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
    /// Side-film scale factor relative to its stated pixel spacing; 1 when
    /// there is no side film.
    pub side_scale: f64,
    /// Side-film vertical registration in the provisional frame, mm; 0 when
    /// there is no side film.
    pub side_offset_mm: f64,
    /// L5 when fitted, otherwise the lowest fitted level.
    pub origin_level: Level,
    pub iterations: u32,
    pub converged: bool,
    pub stop_reason: FitStopReason,
    /// Root mean square error of the placed marks, before uncertainty weights.
    pub rms_mm: f64,
    /// Measured from a stated scale, or drawn at a typical size.
    pub lengths: FitLengths,
    /// The curve size measured to choose the typical side profile, in
    /// degrees, or `null` when none was measured.
    pub typical_curve_deg: Option<f64>,
    /// Where the front film stands.
    pub front_film: FilmPlacement,
    /// Where the side film stands, or `null` when there is none.
    pub side_film: Option<FilmPlacement>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::film::FilmKind;
    use crate::film_geometry::ImageRight;
    use crate::landmarks::Landmarks;
    use crate::naming::{ChosenBy, LevelNaming, Template};
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
                corner_tilt_rad: Some(0.02),
            }],
            side_scale: 1.0,
            side_offset_mm: 0.0,
            origin_level: Level::L5,
            iterations: 3,
            converged: true,
            stop_reason: FitStopReason::SmallStep,
            rms_mm: 0.5,
            lengths: FitLengths::TypicalSize,
            typical_curve_deg: None,
            front_film: FilmPlacement {
                mm_per_px: [0.3, 0.3],
                origin_px: [512.0, 1800.5],
            },
            side_film: None,
        }
    }

    fn one_film() -> FitFromFilms {
        let marks = FilmMarks {
            film: FilmKind::Front,
            vertebrae: Vec::new(),
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
            named_for: ImageRight::PatientLeft,
        };
        FitFromFilms {
            front: FilmToFit {
                marks,
                geometry: FilmGeometry::UNKNOWN,
                size: [100, 200],
                mark_spread: MarkSpread::ShareOfHeight {
                    share: 0.1,
                    shared_share: 0.08,
                },
            },
            side: None,
            typical: TypicalSource::BuiltIn,
            twist_readings: vec![TwistReading {
                level: Level::T8,
                twist_rad: 0.1,
                sd_rad: 0.05,
            }],
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
        assert_eq!(value["lengths"], "typical-size");
        assert_eq!(value["typicalCurveDeg"], json!(null));
        let vertebra = &value["vertebrae"][0];
        assert_eq!(
            vertebra["angles"],
            json!({"sideTiltRad": 0.0, "forwardLeanRad": 0.0, "twistRad": 0.0})
        );
        assert_eq!(vertebra["sd"]["centreMm"], json!([1.0, 2.0, 3.0]));
        assert_eq!(vertebra["sd"]["anglesRad"], json!([0.01, 0.02, 0.03]));
        assert_eq!(vertebra["rmsMm"], json!({"front": 0.5, "side": null}));
        assert_eq!(vertebra["cornerTiltRad"], 0.02);
        assert_eq!(
            value["frontFilm"],
            json!({"mmPerPx": [0.3, 0.3], "originPx": [512.0, 1800.5]})
        );
        assert_eq!(value["sideFilm"], json!(null));
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
            "/frontFilm",
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

    #[test]
    fn a_one_film_input_names_its_spread_and_typical_source() {
        let input = one_film();
        let value = serde_json::to_value(&input).expect("the input serialises");
        assert_eq!(value["side"], json!(null));
        assert_eq!(value["typical"], "built-in");
        assert_eq!(
            value["front"]["markSpread"],
            json!({"kind": "share-of-height", "share": 0.1, "sharedShare": 0.08})
        );
        assert_eq!(
            value["twistReadings"][0],
            json!({"level": "T8", "twistRad": 0.1, "sdRad": 0.05})
        );
        assert_eq!(
            serde_json::to_value(MarkSpread::Millimetres {
                sd_mm: 1.0,
                shared_sd_mm: 0.0
            })
            .expect("serialises"),
            json!({"kind": "millimetres", "sdMm": 1.0, "sharedSdMm": 0.0})
        );
        assert_eq!(
            serde_json::from_value::<FitFromFilms>(value).expect("the input reads back"),
            input
        );
    }

    #[test]
    fn unknown_fields_are_refused_in_a_one_film_input() {
        let value = serde_json::to_value(one_film()).expect("the input serialises");
        for path in ["", "/front", "/front/markSpread", "/twistReadings/0"] {
            let mut changed = value.clone();
            changed
                .pointer_mut(path)
                .and_then(serde_json::Value::as_object_mut)
                .expect("an object")
                .insert("extra".to_string(), json!(1));
            assert!(
                serde_json::from_value::<FitFromFilms>(changed).is_err(),
                "{path}"
            );
        }
    }
}
