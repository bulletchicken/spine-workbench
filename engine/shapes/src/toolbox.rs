//! The adjusting tools as the app sees them: hover previews, the region
//! wand, the boundary wand and pins.
//!
//! The engine keeps no edit state. Every request carries the film's current
//! [`FilmMarks`] and the case version it was computed from; every answer
//! echoes that version, so the app drops an answer that arrives after its
//! film changed, or after the case gave out the new vertebra id it uses
//! (the rules in `app/src/case-state/case-edits.ts`). Nothing happens on
//! a click that the hover did not show.

use crate::landmarks::LandmarkName;
use crate::levels::Level;
use crate::marks::{EndplateLine, LineEnd, MarkName};
use crate::pixels::Px;
use crate::vertebra::FilmMarks;
use crate::watch_frames::PreviewTile;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The two tools that preview on hover.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum WandKind {
    RegionWand,
    BoundaryWand,
}

/// What a click would do at the pointer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ToolPreviewInput {
    /// The case store's version this request was computed from, which
    /// the answer echoes. Not `CASE_VERSION`, the case file format's.
    pub case_version: u32,
    pub tool: WandKind,
    pub at: Px,
    pub marks: FilmMarks,
    /// The id a new vertebra would get if a click here splits one: the
    /// case's `nextVertebraId`, as the boundary wand's click will carry.
    pub new_vertebra_id: u32,
}

/// Why a click here would do nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum MissReason {
    OutsideFilm,
    OffSpine,
    NearEndplate,
    OutsideChain,
    Ambiguous,
    NotSupportedOnFilm,
    TooSmall,
    AllPinned,
    /// A split here would push a name past C1 or L7 (see
    /// [`Template::names`](crate::naming::Template::names)): the count
    /// needs settling in the naming panel first.
    NamesRunOut,
}

/// A vertebra's level after an edit.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VertebraLevel {
    pub vertebra: u32,
    pub level: Level,
}

/// The hover answer: the target and the action, drawn before any click.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(
    tag = "action",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ToolPreview {
    /// The region wand would grow this vertebra.
    Add { case_version: u32, vertebra: u32 },
    /// The region wand would carve this vertebra back.
    Subtract { case_version: u32, vertebra: u32 },
    /// The boundary wand would split this vertebra in two along `line`.
    Split {
        case_version: u32,
        vertebra: u32,
        line: [Px; 2],
        /// Every level that would change, including the new vertebra's,
        /// under the input's `newVertebraId`.
        levels_after: Vec<VertebraLevel>,
    },
    /// The boundary wand would merge these two vertebrae.
    Merge {
        case_version: u32,
        upper: u32,
        lower: u32,
        levels_after: Vec<VertebraLevel>,
    },
    /// Nothing, and why.
    #[serde(rename = "none")]
    Nothing {
        case_version: u32,
        reason: MissReason,
    },
}

/// Which way a region wand click pushes the outline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum WandPolarity {
    /// Bone the outline missed.
    Add,
    /// Part of the outline that is not this bone.
    Subtract,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WandClick {
    pub at: Px,
    pub polarity: WandPolarity,
}

/// A region wand click on one vertebra, with the clicks before it on the
/// same vertebra (the session the app keeps).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegionWandInput {
    /// The case store's version this request was computed from, which
    /// the answer echoes. Not `CASE_VERSION`, the case file format's.
    pub case_version: u32,
    pub marks: FilmMarks,
    pub vertebra: u32,
    /// Oldest first; the last one is the new click.
    pub clicks: Vec<WandClick>,
}

/// Something about an accepted edit worth showing on the vertebra.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(
    tag = "code",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ToolWarning {
    /// The outline's area changed a lot.
    AreaChange { ratio: f64 },
    /// The outline now overlaps a neighbour.
    NeighbourOverlap { with: u32, fraction: f64 },
    /// The solver settled loosely.
    LowConfidence { residual: f64 },
    /// A very bright or dark object (metal, gas) is inside the outline.
    BrightOrDarkObject { fraction: f64 },
    /// The outline reached the edge of what this click may change.
    ReachedLimit,
    /// The region came apart; the largest piece was kept.
    Disconnected,
    /// Pins on merged corners were removed.
    PinsRemoved { count: u32 },
    /// The two level counts now disagree.
    NamingConflict,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegionWandOutput {
    pub case_version: u32,
    /// The film's marks after the edit.
    pub marks: FilmMarks,
    /// Ids of the vertebrae that changed.
    pub changed: Vec<u32>,
    /// The region found, as a closed outline.
    pub outline: Vec<Px>,
    /// Up to four snapshots of the region growing or shrinking.
    pub growth: Vec<PreviewTile>,
    pub solver_iterations: u32,
    pub solver_residual: f64,
    pub warnings: Vec<ToolWarning>,
}

/// A boundary wand click: split the vertebra under it, or merge the two
/// on either side of the gap under it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BoundaryWandInput {
    /// The case store's version this request was computed from, which
    /// the answer echoes. Not `CASE_VERSION`, the case file format's.
    pub case_version: u32,
    pub marks: FilmMarks,
    pub at: Px,
    /// The id a new vertebra gets if this is a split: the case's
    /// `nextVertebraId` at `caseVersion`. If the case gives that id out
    /// before the answer lands, the answer is dropped, so one id never
    /// ends up on two vertebrae.
    pub new_vertebra_id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum BoundaryAction {
    Split,
    Merge,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BoundaryWandOutput {
    pub case_version: u32,
    pub action: BoundaryAction,
    pub marks: FilmMarks,
    pub changed: Vec<u32>,
    /// The new vertebra's id after a split.
    pub new_vertebra: Option<u32>,
    /// The vertebra removed by a merge.
    pub removed_vertebra: Option<u32>,
    pub warnings: Vec<ToolWarning>,
}

/// The one point a pin, a drag or a hand placement moves.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(
    tag = "on",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum PinTarget {
    /// A named mark of a vertebra on a front or side film.
    Vertebra {
        vertebra: u32,
        mark: MarkName,
    },
    /// One end of an endplate line of a vertebra on a bending film.
    Endplate {
        vertebra: u32,
        line: EndplateLine,
        end: LineEnd,
    },
    Landmark {
        landmark: LandmarkName,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(
    tag = "action",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum PinAction {
    /// Drag the point here and pin it.
    Move { target: PinTarget, to: Px },
    /// Release the pin, leaving the point where it is.
    Unpin { target: PinTarget },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PinInput {
    /// The case store's version this request was computed from, which
    /// the answer echoes. Not `CASE_VERSION`, the case file format's.
    pub case_version: u32,
    pub marks: FilmMarks,
    pub action: PinAction,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PinOutput {
    pub case_version: u32,
    pub marks: FilmMarks,
    /// The pinned vertebra and any neighbours that re-solved around it.
    pub changed: Vec<u32>,
    pub warnings: Vec<ToolWarning>,
}
