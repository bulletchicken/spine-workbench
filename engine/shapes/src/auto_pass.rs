//! The auto pass as the app sees it: what goes in, what comes back, and the
//! steps it reports while it runs.
//!
//! The pass runs in two parts. A **film pass** (stages evidence, centre
//! line and strip) runs on each film's own worker and returns a small
//! [`FilmPassResult`]. One **chain pass** (stages chain, naming and
//! confidence) takes every film's result on the spine worker and returns a
//! [`ChainAnswer`]. Each film's worker then places the points.
//!
//! What travels between the stages inside the engine (the unrolled strip,
//! the chain boundaries) is the auto pass's own business: it rides here as
//! opaque bytes that only the engine reads, so the app forwards it without
//! looking inside.
//!
//! Like every tool request, the pass carries the case version it was
//! computed from, and its answer echoes it. The hand-offs between workers
//! ([`FilmPassResult`], [`ChainAnswer`], [`FilmChain`]) never reach the
//! case store, so they carry no version: the client that runs the pass
//! stamps [`AutoPassResult::case_version`] from
//! [`AutoPassInput::case_version`] itself.

use crate::film::{Film, FilmKind};
use crate::levels::Level;
use crate::naming::{LevelNaming, Template};
use crate::pixels::PxRect;
use crate::vertebra::FilmMarks;
use crate::watch_frames::WatchFrame;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The six stages, in the order they run.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum Stage {
    Evidence,
    CentreLine,
    Strip,
    Chain,
    Naming,
    Confidence,
}

/// What one bounded slice of work did. A pass is a state machine: the
/// worker calls `advance(workUnits)` until `done`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Step {
    pub done: bool,
    pub stage: Stage,
    /// 0 to 1 within the current stage.
    pub fraction: f64,
    /// Frames produced during this slice, at fixed checkpoints that never
    /// depend on the slice size.
    pub frames: Vec<WatchFrame>,
}

/// Whether the front and side films were taken at the same moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum Acquisition {
    /// Both views at once (a biplanar slot scanner).
    Simultaneous,
    /// Two separate exposures.
    Separate,
    /// Not known: treated as separate.
    Unknown,
}

/// Whose body sizes the height priors assume.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum Population {
    Adolescent,
    Adult,
}

/// Choices for the chain pass.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChainOptions {
    pub acquisition: Acquisition,
    pub population: Population,
    /// A count the user chose in the naming panel; `null` lets the engine
    /// choose from the evidence.
    pub template: Option<Template>,
}

/// Everything the auto pass needs from the app.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutoPassInput {
    /// The case store's version (the count every applied edit raises)
    /// that the films and their records were taken from. Not
    /// `CASE_VERSION`, the case file format's version.
    pub case_version: u32,
    /// The films to run, each as the case holds it. A film's pass uses its
    /// geometry, leaves its blanked boxes out of every map, and reads its
    /// pixels with the brightness `values` describes, the user's flip
    /// included.
    pub films: Vec<Film>,
    pub options: ChainOptions,
    /// The case's `nextVertebraId` at `caseVersion`. New vertebrae take
    /// ids from here upward, film after film in the order of `films`, so
    /// no id the case already holds is reused. If the case gives out any
    /// of these ids before the answer lands, each film's edit that uses
    /// one is dropped (the rules in `app/src/case-state/case-edits.ts`),
    /// so one id never ends up on two vertebrae.
    pub first_vertebra_id: u32,
}

/// What a film pass hands to the chain pass.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FilmPassResult {
    pub film: FilmKind,
    /// The unrolled strip and its maps back to film pixels, in the auto
    /// pass's own encoding.
    #[serde(with = "serde_bytes")]
    #[schemars(with = "Vec<u8>")]
    #[cfg_attr(feature = "typescript", ts(type = "Uint8Array"))]
    pub strip: Vec<u8>,
    /// The film had no scale, so one was assumed from the spine's width:
    /// nothing from this film may be reported in millimetres.
    pub scale_assumed: bool,
    pub warnings: Vec<PassWarning>,
}

/// One film's solved chain, before its points are placed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FilmChain {
    pub film: FilmKind,
    pub naming: LevelNaming,
    /// The chain's boundaries, levels and confidence, in the auto pass's
    /// own encoding; the film's worker turns them into points.
    #[serde(with = "serde_bytes")]
    #[schemars(with = "Vec<u8>")]
    #[cfg_attr(feature = "typescript", ts(type = "Uint8Array"))]
    pub chain: Vec<u8>,
    pub warnings: Vec<PassWarning>,
}

/// The unit a length is reported in. Pixels when a film has no scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum LengthUnit {
    Mm,
    Px,
}

/// How well the front and side chains agree on each level's height.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LevelResidual {
    pub level: Level,
    pub residual: f64,
}

/// The joint front and side solve.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TwoFilmCheck {
    /// Upper bound minus lower bound of the joint energy: how far from the
    /// best possible the joint answer could be.
    pub certified_gap: f64,
    /// The gap is small enough to call the joint answer exact.
    pub exact: bool,
    /// The films disagree: check the facing, the scale, or that both films
    /// are the same patient and visit.
    pub disagree: bool,
    pub unit: LengthUnit,
    pub residuals: Vec<LevelResidual>,
}

/// What the chain pass hands back.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChainAnswer {
    pub films: Vec<FilmChain>,
    /// Present when a front and a side film were solved together.
    pub two_films: Option<TwoFilmCheck>,
}

/// What the whole auto pass returns to the screens.
///
/// The app applies it as one `replace-marks` edit per entry of `marks`,
/// in order, each with cause `auto-pass` and `basedOn` set to
/// `caseVersion`. Each edit is kept or dropped on its own: dropped if its
/// own film changed since that version, or if it uses a vertebra id the
/// case may have given out since.
///
/// When only some of the edits are kept, the screens show `twoFilms` only
/// if every film's edit was kept, and each warning only if its film's
/// edit was kept, so nothing shown describes marks the case does not
/// hold.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutoPassResult {
    /// The input's `caseVersion`, echoed.
    pub case_version: u32,
    /// One film's marks per film the pass ran, in the order of the
    /// input's `films`.
    pub marks: Vec<FilmMarks>,
    pub two_films: Option<TwoFilmCheck>,
    pub warnings: Vec<PassWarning>,
}

/// Something a pass noticed that the user should know.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PassWarning {
    pub code: PassWarningCode,
    pub film: FilmKind,
    /// The level it concerns, when it concerns one.
    pub level: Option<Level>,
    /// The stretch of film it concerns, when it concerns one.
    pub region: Option<PxRect>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum PassWarningCode {
    // The film as a whole
    /// No scale: lengths are in assumed units only.
    ScaleAssumed,
    /// The spine's own rhythm gave only a rough scale.
    ScaleGuess,
    PolarityUncertain,
    /// Which way the film faces is not settled.
    FacingUnknown,
    /// Collimator edges could not be found; the whole film was used.
    CollimationUnclear,
    /// The patient's size could not be judged.
    SizeUnknown,
    /// Too few pixels carry evidence.
    LowEvidence,
    /// A stitching seam crosses the film.
    SeamsFound,
    /// A marker or burnt-in text lies over the spine.
    MarkersOverSpine,
    /// An edge band looks like a shutter that was not found.
    ShutterSuspected,
    /// The film is too coarse for the pedicle map.
    CoarseFilm,
    /// JPEG block edges were damped.
    JpegBlocks,
    /// The working resolution was lowered to fit in memory.
    BudgetReduced,

    // The centre line and strip
    TopCropped,
    BottomCropped,
    PelvisAsymmetric,
    WeakEvidence,
    Tortuous,
    TightCurve,
    NearMaskEdge,
    /// The curve is so severe the line was searched without its direction
    /// penalty.
    SevereCurve,
    /// The spine's width fell back to an estimate.
    WidthFallback,
    /// The strip folds over itself on a tight curve; part was masked.
    StripFolded,
    PossibleImplant,
    /// The facing cues were looked for in guessed places.
    FacingWindowsGuessed,

    // The chain
    MetalNearby,
    ManyUnobservedRows,
    TiltInconsistent,
    TiltRangeWidened,
    FilmsDisagree,
    /// A bending film was solved without the front film's heights.
    BendingCouplingDropped,
}
