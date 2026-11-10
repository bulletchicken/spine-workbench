//! Made-up spines: the recipes, the answer key that comes with every
//! made-up case, and how a set of marks compares with it.
//!
//! Accuracy on made-up films is never clinical accuracy.

use crate::film::{FilmKind, FilmPng};
use crate::film_geometry::FilmGeometry;
use crate::levels::Level;
use crate::naming::Template;
use crate::pixels::Px;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The named recipes. Each draws everything it leaves open from the seed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum RecipeName {
    /// Every part of the spine drawn from the seed: curve type, size,
    /// count anomalies.
    Random,
    Straight,
    #[serde(rename = "right-thoracic-40")]
    RightThoracic40,
    #[serde(rename = "double-major-55-45")]
    DoubleMajor55To45,
    #[serde(rename = "lumbar-left-30")]
    LumbarLeft30,
    #[serde(rename = "severe-90")]
    Severe90,
    SixLumbar,
    ElevenRibs,
    UpsideDown,
    MirroredFront,
    SideFacingLeft,
    HeavyShoulders,
}

/// How hard the films are to read, from clean drawings (`t0`) to films
/// with noise, scatter, markers, implants and crops (`t4`).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum Tier {
    T0,
    T1,
    T2,
    T3,
    T4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MadeUpRecipe {
    pub name: RecipeName,
    pub seed: u32,
    pub tier: Tier,
}

/// One vertebra's true pose: the thirteen numbers of the shared vertebra
/// model, in the patient frame (mm and radians; x forward, y to the
/// patient's left, z up; origin at the centre of the underside of L5).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VertebraPose {
    pub level: Level,
    pub centre: [f64; 3],
    /// Rotation vector taking the vertebra's own axes to patient axes.
    pub rotation: [f64; 3],
    /// Distance between the endplate centres.
    pub height: f64,
    /// Front-view wedge, positive when the patient's left side is taller.
    pub wedge_front: f64,
    /// Side-view wedge, positive when the front is shorter.
    pub wedge_side: f64,
    pub top_half_width: f64,
    pub top_half_depth: f64,
    pub bottom_half_width: f64,
    pub bottom_half_depth: f64,
}

/// One true point on a made-up film, and whether the film shows it.
///
/// Its coordinates are kept even when it is not visible: a point off the
/// film or outside the collimated field, a pedicle hidden by the body, an
/// endplate seen face on. Scoring leaves invisible points out of corner
/// errors; the vertebra still counts as found and named.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrueMark {
    pub at: Px,
    pub visible: bool,
}

/// The true marks of one vertebra on one film, named as the placed marks
/// are.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum TrueMarks {
    Front {
        sup_l: TrueMark,
        sup_r: TrueMark,
        inf_l: TrueMark,
        inf_r: TrueMark,
        ped_l: TrueMark,
        ped_r: TrueMark,
    },
    Side {
        sup_a: TrueMark,
        sup_p: TrueMark,
        inf_a: TrueMark,
        inf_p: TrueMark,
    },
    /// Each line's ends patient left, then right.
    Bending {
        sup: [TrueMark; 2],
        inf: [TrueMark; 2],
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrueVertebra {
    pub level: Level,
    pub marks: TrueMarks,
}

/// The true landmarks on one film.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrueLandmarks {
    /// The mean of the true C7 vertebra's four corners on this film (a
    /// bending film's line ends), the same definition as the placed
    /// `c7` ([`Landmarks::c7_of`](crate::landmarks::Landmarks::c7_of)),
    /// so the scorer compares like with like. Visible exactly when all
    /// four corners are.
    pub c7: TrueMark,
    /// The top of the sacrum (the S1 endplate): its patient-left (or
    /// anterior) corner, its centre, then its patient-right (or
    /// posterior) corner.
    pub sacrum_top: [TrueMark; 3],
    /// The femoral head centres, patient left, then right.
    pub hips: [TrueMark; 2],
}

/// The truth for one made-up film.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FilmAnswer {
    pub kind: FilmKind,
    /// The film's true geometry, as if every fact had been confirmed.
    pub geometry: FilmGeometry,
    pub vertebrae: Vec<TrueVertebra>,
    pub landmarks: TrueLandmarks,
    /// Bending films only: the spine as bent for this film, top to bottom.
    /// `null` on front and side films, which show
    /// [`AnswerKey::poses`].
    pub bent_poses: Option<Vec<VertebraPose>>,
}

/// Where a curve sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum CurveRegion {
    ProximalThoracic,
    MainThoracic,
    ThoracolumbarLumbar,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CurveAngle {
    pub region: CurveRegion,
    pub degrees: f64,
}

/// Angles measured from the made-up spine's geometry, never copied from
/// its recipe.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TruthAngles {
    pub frontal_cobb: Vec<CurveAngle>,
    pub apical_twist: Vec<CurveAngle>,
    pub kyphosis_degrees: f64,
    pub lordosis_degrees: f64,
}

/// Everything true about a made-up case.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnswerKey {
    pub recipe: MadeUpRecipe,
    /// The true vertebra count.
    pub template: Template,
    /// The standing spine, top to bottom: what the front and side films
    /// show.
    pub poses: Vec<VertebraPose>,
    pub films: Vec<FilmAnswer>,
    pub angles: TruthAngles,
}

/// A made-up case as the engine hands it over: each film's PNG and the
/// answer key.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MadeUpSpine {
    pub films: Vec<FilmPng>,
    pub answer_key: AnswerKey,
}

/// How one placed vertebra compares with the truth.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VertebraComparison {
    /// The true level.
    pub level: Level,
    /// The placed vertebra matched to it, if any.
    pub vertebra_id: Option<u32>,
    /// The placed vertebra carries the true level's name.
    pub level_right: bool,
    /// Mean distance from each placed corner to its true corner, over the
    /// corners the film shows.
    pub mean_corner_error_px: Option<f64>,
    pub max_corner_error_px: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FilmComparison {
    pub film: FilmKind,
    pub expected: u32,
    pub found: u32,
    pub levels_right: u32,
    pub mean_corner_error_px: Option<f64>,
    /// The same in mm, when the film has a scale.
    pub mean_corner_error_mm: Option<f64>,
    pub vertebrae: Vec<VertebraComparison>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnswerKeyComparison {
    pub films: Vec<FilmComparison>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipe_names_match_their_spelling() {
        let cases = [
            (RecipeName::RightThoracic40, "right-thoracic-40"),
            (RecipeName::DoubleMajor55To45, "double-major-55-45"),
            (RecipeName::LumbarLeft30, "lumbar-left-30"),
            (RecipeName::Severe90, "severe-90"),
            (RecipeName::SideFacingLeft, "side-facing-left"),
        ];
        for (name, spelling) in cases {
            assert_eq!(
                serde_json::to_string(&name).expect("a recipe name serialises"),
                format!("\"{spelling}\"")
            );
        }
    }
}
