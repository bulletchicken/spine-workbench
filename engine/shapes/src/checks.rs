//! What the case checks find: is the spine complete, in order and
//! confirmed?

use crate::film::FilmKind;
use crate::levels::Level;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum CheckKind {
    /// A level from T1 to L5 has no vertebra on this film.
    VertebraMissing,
    /// Levels are repeated or out of order.
    LevelsOutOfOrder,
    /// The chain looks shifted by one level.
    LikelyOffByOne,
    /// The front and side films do not have the same levels.
    FilmsDisagree,
    /// The count is not the standard one.
    NonStandardCount,
    /// Neither end of the chain could be counted from.
    NamesUnknown,
    /// Nobody has said which vertebra is L5.
    L5Unconfirmed,
    /// The film's upright turn is not confirmed.
    UprightUnconfirmed,
    /// Which side is the patient's left is not confirmed.
    LeftRightUnconfirmed,
    /// Which way the side film faces is not confirmed.
    FacingUnconfirmed,
    /// The film has no confirmed scale: lengths cannot be measured.
    ScaleUnconfirmed,
    /// The brightness has not been confirmed.
    PolarityUnconfirmed,
    /// The border has not been checked for names or IDs.
    PrivacyUnchecked,
    /// A pedicle is marked not visible.
    PedicleNotVisible,
    /// The marks' left and right (or front and back) names were given for
    /// a facing the film's `imageRight` does not state: it is still
    /// unknown, so the names rest on the engine's guess, or the file was
    /// written with names for the other facing. The app's own edits never
    /// leave a known `imageRight` and the marks' `namedFor` apart: the
    /// auto pass names for a known `imageRight`, and changing the facing
    /// renames the marks to match.
    MarksNamedForOtherFacing,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    /// Worth a look; the case can still be saved and used.
    Warning,
    /// Must be settled before anything depends on it.
    Blocking,
}

/// One finding. Each points at what it is about, so the app can fly there.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckFinding {
    pub kind: CheckKind,
    pub severity: Severity,
    pub film: Option<FilmKind>,
    pub vertebra_id: Option<u32>,
    pub level: Option<Level>,
}
