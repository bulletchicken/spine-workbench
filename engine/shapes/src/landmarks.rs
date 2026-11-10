//! The points placed besides the vertebrae: C7, the top of the sacrum and
//! the hip centres.
//!
//! The sacrum top is the plateau's two ends, named like the corners:
//! `sacL` and `sacR` (patient left and right) on front and bending films,
//! `sacA` and `sacP` (anterior and posterior) on side films.
//!
//! C7 is the centre of the C7 body. The chain's C7 vertebra is the
//! authority: `c7` is derived from its four corners, and it is worked out
//! again whenever those corners move (the auto pass, a wand, a pin, a
//! hand move) or the names change which vertebra is C7 (the user saying
//! which vertebra is L5, or a wand's split or merge shifting the names on
//! one side). So it is never pinned or placed on its own, and
//! [`LandmarkName`] has no name for it. [`Landmarks::c7_of`] is the one
//! definition, and the reader refuses a `c7` that disagrees with it.

use crate::levels::Level;
use crate::marks::{Mark, MarkSource};
use crate::pixels::Px;
use crate::vertebra::VertebraOnFilm;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The two ends of the sacral plateau.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(
    tag = "view",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SacrumTop {
    /// Front and bending films.
    Front { sac_l: Mark, sac_r: Mark },
    /// Side films.
    Side { sac_a: Mark, sac_p: Mark },
}

/// Which hip a hip centre is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum HipSide {
    /// The patient's left hip (front films).
    Left,
    /// The patient's right hip (front films).
    Right,
    /// One of the two femoral heads on a side film; which one is unknown.
    Unpaired,
    /// The midpoint of the two femoral heads where a side film shows them
    /// within 5 mm of each other.
    Merged,
}

/// A femoral head: its centre and, when a circle was fitted, its radius.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HipCentre {
    pub side: HipSide,
    pub centre: Mark,
    /// The fitted circle's radius in pixels; `null` when placed by hand.
    pub radius_px: Option<f64>,
}

/// Every landmark on one film. Each is `null` or empty until found.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Landmarks {
    /// The mean of the four corners (line ends, on a bending film) of the
    /// film's C7 vertebra; `null` exactly when the film has none. Never
    /// pinned, its source always `auto`, inferred when any corner is.
    pub c7: Option<Mark>,
    pub sacrum_top: Option<SacrumTop>,
    /// Front films: at most one left and one right. Side films: one or two
    /// unpaired, or one merged. Bending films: none.
    pub hips: Vec<HipCentre>,
}

impl Landmarks {
    /// No landmarks found yet.
    pub const NONE: Landmarks = Landmarks {
        c7: None,
        sacrum_top: None,
        hips: Vec::new(),
    };

    /// The `c7` a film with these vertebrae has: the mean of its C7
    /// vertebra's corners, source `auto`, not pinned, inferred when any
    /// corner is; `None` when the film has no C7 vertebra.
    pub fn c7_of(vertebrae: &[VertebraOnFilm]) -> Option<Mark> {
        let c7 = vertebrae
            .iter()
            .find(|vertebra| vertebra.level == Level::C7)?;
        let corners = c7.marks.corners();
        let mean =
            |axis: fn(Px) -> f64| corners.iter().map(|mark| axis(mark.at)).sum::<f64>() / 4.0;
        Some(Mark {
            at: Px::new(mean(Px::x), mean(Px::y)),
            source: MarkSource::Auto,
            pinned: false,
            inferred: corners.iter().any(|mark| mark.inferred),
        })
    }
}

/// How far a stored `c7` may sit from [`Landmarks::c7_of`], in pixels:
/// room for adding the corners in another order, nothing more.
pub const C7_TOLERANCE_PX: f64 = 0.001;

/// The name of one landmark point that can be moved on its own. C7 is not
/// one: it follows the C7 vertebra's corners.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum LandmarkName {
    SacL,
    SacR,
    SacA,
    SacP,
    HipLeft,
    HipRight,
    /// The first unpaired (or the merged) hip centre on a side film.
    HipFirst,
    /// The second unpaired hip centre on a side film.
    HipSecond,
}
