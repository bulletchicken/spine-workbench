//! How sure the engine is about each vertebra, and the band thresholds the
//! app and the scorekeeper share.
//!
//! The score answers "is this outline on the right bone?": it is the chain
//! search's posterior that both endplates sit within a third of a body
//! height of where they were placed. An off-by-one shift is what it
//! catches; millimetre precision is the separate endplate spread.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Scores at or above this are [`ConfidenceBand::High`].
pub const HIGH_FROM: f64 = 0.90;

/// Scores at or above this (and below [`HIGH_FROM`]) are
/// [`ConfidenceBand::Medium`]; anything lower is [`ConfidenceBand::Low`].
pub const MEDIUM_FROM: f64 = 0.60;

/// The colour a vertebra is drawn in. Ordered low to high.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum ConfidenceBand {
    Low,
    Medium,
    High,
}

impl ConfidenceBand {
    /// The band a score earns before any cap.
    pub fn for_score(score: f64) -> ConfidenceBand {
        if score >= HIGH_FROM {
            ConfidenceBand::High
        } else if score >= MEDIUM_FROM {
            ConfidenceBand::Medium
        } else {
            ConfidenceBand::Low
        }
    }
}

/// Why a vertebra's confidence is what it is. Several reasons cap the band
/// at medium whatever the score.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum ConfidenceReason {
    /// The gap above or below is faint.
    WeakGap,
    /// The height is unusual next to its neighbours.
    OddHeight,
    /// The front and side films place it differently.
    FilmsDisagree,
    /// Ribs cross it.
    RibOverlap,
    /// A pedicle could not be seen.
    PedicleHidden,
    /// It is close to the edge of the film.
    NearFilmEdge,
    /// Its level name is in doubt.
    LevelUncertain,
    /// The user changed it.
    EditedByUser,
    /// It sits where the film shows nothing (cropped off), or the trained
    /// finder's chain filled it in between its neighbours without seeing
    /// it: placed from the chain alone.
    NoEvidence,
    /// Its endplate tilts disagree with its neighbours'.
    TiltInconsistent,
    /// Metal lies over it.
    MetalNearby,
    /// The film's brightness may be the wrong way round.
    PolarityUncertain,
    /// The film was stored with lossy compression.
    LossyFilm,
}

/// Confidence for one vertebra.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Confidence {
    /// 0 to 1: how likely the outline is on the right bone.
    /// For the trained side finder, 0 is a conservative unavailable-score
    /// sentinel, not a probability estimate. Its band is Low, its reasons
    /// are [LevelUncertain], calibrated is false, and endplateSpreadPx and
    /// margin are null.
    #[schemars(description = "0 to 1: how likely the outline is on the right bone.")]
    pub score: f64,
    /// Never higher than [`ConfidenceBand::for_score`] gives; caps can
    /// lower it.
    pub band: ConfidenceBand,
    pub reasons: Vec<ConfidenceReason>,
    /// False until the score has been checked against real films; the app
    /// then labels the colours as uncalibrated.
    pub calibrated: bool,
    /// How far each endplate could plausibly move, one standard deviation
    /// in film pixels; `null` when not computed by the chain search.
    pub endplate_spread_px: Option<EndplateSpread>,
    /// Energy gap to the best placement outside the right-bone box; `null`
    /// when not computed by the chain search.
    pub margin: Option<f64>,
}

/// One standard deviation of each endplate's position, in film pixels.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EndplateSpread {
    pub sup: f64,
    pub inf: f64,
}

impl Confidence {
    /// True when the band does not claim more than the score earns.
    pub fn band_fits_score(&self) -> bool {
        self.band <= ConfidenceBand::for_score(self.score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bands_split_at_the_shared_thresholds() {
        assert_eq!(ConfidenceBand::for_score(0.90), ConfidenceBand::High);
        assert_eq!(ConfidenceBand::for_score(0.8999), ConfidenceBand::Medium);
        assert_eq!(ConfidenceBand::for_score(0.60), ConfidenceBand::Medium);
        assert_eq!(ConfidenceBand::for_score(0.5999), ConfidenceBand::Low);
    }

    #[test]
    fn a_cap_may_lower_the_band_but_never_raise_it() {
        let mut confidence = Confidence {
            score: 0.95,
            band: ConfidenceBand::Medium,
            reasons: vec![ConfidenceReason::NoEvidence],
            calibrated: false,
            endplate_spread_px: None,
            margin: None,
        };
        assert!(confidence.band_fits_score());
        confidence.score = 0.5;
        assert!(!confidence.band_fits_score());
    }
}
