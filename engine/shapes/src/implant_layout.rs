//! Placement proposals on a fitted spine: which levels to instrument, an
//! anchor at each pedicle with its entry point, direction, diameter and
//! length, and two rods.
//!
//! Every answer is counted over versions of the spine: the same spine with
//! one draw of everything the films cannot fix (fit noise, pedicle widths).
//! A range is where 8 of every 10 versions fall. Sizes come from published
//! tables, never from this spine's own pedicles, and every number that
//! comes from a paper carries its source key. The engine returns codes;
//! the app's words file turns them into sentences.
//!
//! Lengths are in millimetres and positions in the patient frame (x
//! forward, y to the patient's left, z up, origin at the centre of the
//! underside of L5). Angles shown to a person are in degrees, with `Deg`
//! in the field name.

use crate::film_geometry::FilmGeometry;
use crate::levels::Level;
use crate::made_up::CurveRegion;
use crate::spine_fit::SpineFit;
use crate::spine_readings::{Band, BodySide, FitTier, LevelCount};
use crate::vertebra::FilmMarks;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ---- What goes in

/// Everything the layout proposer reads.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayoutInput {
    pub fit: SpineFit,
    /// The front film's marks, for the top of the sacrum when it is marked.
    pub front: Option<LayoutFilm>,
    /// The bending films' endplate lines, each `null` when absent.
    pub bend_left: Option<LayoutFilm>,
    pub bend_right: Option<LayoutFilm>,
    pub child: ChildFacts,
    pub naming: NamingDoubt,
    /// How many versions to count over (32 by default).
    pub versions: u32,
    pub seed: u32,
    /// The share of a pedicle width's spread that all of one child's
    /// pedicles have in common, from 0 to 1. `null` takes this project's
    /// own default of one half; no paper gives the split. Only developer
    /// scenes change it, to show its effect.
    pub pedicle_child_share: Option<f64>,
}

/// The original proposal input and the changes a person made to one
/// placement. Pedicles not listed follow the first placement's pattern
/// over the new span.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RebuildInput {
    /// The same input the proposals were made from; same seed, same draws.
    pub layout: LayoutInput,
    pub upper: Level,
    pub lower: Level,
    /// Only the pedicles the person set.
    pub anchors: Vec<AnchorChoice>,
    /// Together with `rodDiameterMm`, one pair of `LayoutChoices.rods`; any
    /// other pair is refused.
    pub rod_material: RodMaterial,
    /// Together with `rodMaterial`, one pair of `LayoutChoices.rods`.
    pub rod_diameter_mm: f64,
}

/// One pedicle set by a person. A `null` screw size takes the size from
/// `screwHere`; a hook or an empty pedicle has no screw size.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnchorChoice {
    pub level: Level,
    pub side: BodySide,
    pub kind: AnchorKind,
    /// `null` takes `screwHere`'s size; off the grid of `LayoutChoices` is refused.
    pub diameter_mm: Option<f64>,
    /// `null` takes `screwHere`'s size; off the grid of `LayoutChoices` is refused.
    pub length_mm: Option<f64>,
}

/// One film's marks with its geometry and its stored size in pixels,
/// width then height.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayoutFilm {
    pub marks: FilmMarks,
    pub geometry: FilmGeometry,
    pub size: [u32; 2],
}

/// What a person may tell about the child. Each is `null` until given.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChildFacts {
    pub age_years: Option<f64>,
    pub weight_kg: Option<f64>,
    pub activity: Option<ActivityLevel>,
}

/// How active the child is, as five bands of steps a day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum ActivityLevel {
    VeryLow,
    Low,
    Typical,
    Active,
    VeryActive,
}

/// Whether the level names are an estimate that may be one level off.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum NamingDoubt {
    /// The names are taken as read.
    AsRead,
    /// The names may be one level off: the placement is also renamed one
    /// level up and one level down, and what changes is listed.
    MayBeOneOff,
}

// ---- What comes out

/// The readings of the spine, every rule's vote, the level pairs, the
/// first placement and the placements one step away from it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayoutProposals {
    pub tier: FitTier,
    /// How many versions every count is out of.
    pub versions: u32,
    pub curves: Vec<LayoutCurve>,
    /// The curve type with its lumbar and sagittal modifiers, counted over
    /// versions.
    pub classification: Vec<CurveTypeCount>,
    /// The two selective fusion tests, criterion by criterion.
    pub selective: Vec<SelectiveReading>,
    /// Every level rule's vote, readable or not.
    pub rules: Vec<RuleVote>,
    /// Every (upper, lower) pair the rules land on or near.
    /// For a sketch, the main-curve spans tried instead, with no rule counts.
    pub pairs: Vec<LevelPair>,
    /// `[0]` is the first placement; the rest are one step away from it.
    pub layouts: Vec<ImplantLayout>,
    /// True when the first placement is a sketch from the main curve's
    /// ends when the rules give no pair on fitted levels, or the first pair
    /// that can be laid out holds in fewer than one version in ten and the
    /// sketch can be laid out. A sketch is never shown as a placement the
    /// rules gave.
    pub first_is_sketch: bool,
    /// The first placement's upper ends over the versions, most frequent
    /// first. Empty when `firstIsSketch`.
    pub upper_ends: Vec<LevelCount>,
    /// The corresponding lower ends, most frequent first. Empty for a
    /// sketch.
    pub lower_ends: Vec<LevelCount>,
    /// The sizes and materials a person may choose from when changing a
    /// placement by hand.
    pub choices: LayoutChoices,
    /// What these films cannot tell, and where.
    pub cannot_tell: Vec<CannotTell>,
    /// When the names may be one off: the first placement renamed one
    /// level up (`shift` -1) and one level down (`shift` +1). Empty when
    /// the names are taken as read.
    pub naming_scenarios: Vec<NamingScenario>,
}

/// Where a number came from. A `from-tables` or `typical` number is never
/// shown as if measured.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum ValueStatus {
    /// Read from this spine's films.
    Measured,
    /// From a published table, for this level, side and age.
    FromTables,
    /// From the generic bone shaped to the fit.
    FromTemplate,
    /// Given by the person.
    Entered,
    /// From a typical spine, because these films do not show it.
    Typical,
    /// No source gives it; nothing stands in its place.
    Missing,
}

/// A test counted over versions, or why the films cannot read it. A test
/// that is not readable never counts as met.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Readable {
    /// The versions in which the test holds.
    Share {
        versions: u32,
    },
    NotReadable {
        reason: CannotTellCode,
    },
}

// ---- Curves and their type

/// One curve as read over the versions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayoutCurve {
    pub region: CurveRegion,
    /// The side the curve bulges toward.
    pub convex: BodySide,
    /// The versions in which this curve was found.
    pub versions_found: u32,
    /// The versions in which it is the largest curve.
    pub versions_major: u32,
    pub upper_end: Vec<LevelCount>,
    pub lower_end: Vec<LevelCount>,
    pub apex: Vec<LevelCount>,
    pub cobb_deg: Band,
    /// The smaller of the two bending films' angles over the same end
    /// vertebrae; `null` without bending films.
    pub bending_cobb_deg: Option<Band>,
    /// Whether a minor curve is structural. The largest curve always is.
    pub structural: Readable,
}

/// One reading of the curve type, with the versions that gave it. A part
/// is `null` in versions where it cannot be read (the reasons are in
/// `cannotTell`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CurveTypeCount {
    pub curve_type: Option<CurveType>,
    pub lumbar_modifier: Option<LumbarModifier>,
    pub sagittal_modifier: Option<SagittalModifier>,
    pub versions: u32,
}

/// The six curve types of the Lenke classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum CurveType {
    /// Type 1.
    MainThoracic,
    /// Type 2.
    DoubleThoracic,
    /// Type 3.
    DoubleMajor,
    /// Type 4.
    TripleMajor,
    /// Type 5.
    ThoracolumbarLumbar,
    /// Type 6.
    ThoracolumbarLumbarMainThoracic,
}

/// Where the centre line falls against the lumbar apex.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum LumbarModifier {
    /// A: between the apex's pedicles.
    BetweenPedicles,
    /// B: touching the apex's pedicle or body.
    TouchesApex,
    /// C: clear of the apex.
    ClearOfApex,
}

/// T5 to T12 kyphosis in three bands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum SagittalModifier {
    /// "-": under +10 degrees.
    Hypokyphotic,
    /// "N": +10 to +40 degrees.
    Normal,
    /// "+": over +40 degrees.
    Hyperkyphotic,
}

// ---- Rules

/// Every level rule and check, by its id.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum LevelRule {
    /// Lower end: the lower end vertebra plus one.
    EndPlusOne,
    /// Lower end: from the neutral vertebra, where twist crosses zero.
    SukNeutral,
    /// Lower end: the last vertebra the centre line touches.
    LastTouched,
    /// Lower end: one above the stable vertebra (at it for modifier C).
    StableMinusOne,
    /// Lower end: from the substantially touched vertebra.
    SubstantiallyTouched,
    /// Lower end of a fused thoracolumbar or lumbar curve: L3 or L4.
    #[serde(rename = "lenke-l3-l4")]
    LenkeL3L4,
    /// Lower end in type 5C: the lower end vertebra or one below it.
    #[serde(rename = "lev-plus-one-5c")]
    LevPlusOne5C,
    /// A check on a lower end: the neutral and stable vertebrae against
    /// it, flagging adding-on.
    LivIndex,
    /// Lower end: the first vertebra, going up from the sacrum, that lies
    /// more than 10 mm from the centre line.
    DeviatedVertebra,
    /// Upper end: the upper end vertebra of the highest fused curve.
    UpperEndVertebra,
    /// Upper end in type 5C: the upper end vertebra, or one above it.
    #[serde(rename = "uev-5c")]
    Uev5C,
    /// Upper end from the shoulders: never readable here, since the
    /// shoulders are outside the model.
    LenkeShoulder,
    /// Fusing the main thoracic curve alone.
    SelectiveThoracic,
    /// Fusing the thoracolumbar or lumbar curve alone.
    SelectiveLumbar,
}

/// Which end of the fusion a rule proposes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum RuleEnd {
    Upper,
    Lower,
}

/// Something a rule reads off the spine. The unit follows the kind: an
/// angle in degrees, a distance in millimetres, a ratio with none.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum Measurement {
    /// A vertebra's upper endplate tilt in the frontal plane, degrees,
    /// positive when it falls toward the patient's right.
    CoronalTilt,
    /// A vertebra's named twist, degrees.
    Twist,
    /// A curve's Cobb angle on the standing front film, degrees.
    Cobb,
    /// A curve's angle on the bending film that straightens it most,
    /// degrees.
    BendingCobb,
    /// The angle of the disc below a vertebra on the standing front film,
    /// degrees, positive when the disc opens toward the patient's left.
    DiscAngle,
    /// The same disc on the bending film that straightens its curve most.
    BendingDiscAngle,
    /// Kyphosis from T2 to T5, degrees, positive when kyphotic.
    KyphosisT2T5,
    /// Kyphosis from T5 to T12, degrees.
    KyphosisT5T12,
    /// Kyphosis from T10 to L2, degrees.
    KyphosisT10L2,
    /// A vertebra's body centre from the centre line, millimetres,
    /// positive toward the patient's left.
    CentreLineOffset,
    /// A curve's apex from the centre line, millimetres.
    ApicalTranslation,
    /// A curve's apical twist, degrees.
    ApicalRotation,
    /// A curve's Cobb angle over the other main curve's.
    CobbRatio,
    /// A curve's apical translation over the other main curve's.
    ApicalTranslationRatio,
    /// A curve's apical rotation over the other main curve's.
    ApicalRotationRatio,
    /// How far a curve's apex sits above the disc below a vertebra, in
    /// levels; an apex on a disc counts as a half.
    ApexAboveDisc,
    /// (lower end - neutral vertebra) + (lower end - stable vertebra), in
    /// levels.
    LivIndex,
}

/// One rule's vote: per version a level, or why it could not read one,
/// with the inputs it read on the spine as fitted.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuleVote {
    pub rule: LevelRule,
    pub end: RuleEnd,
    pub counts: RuleCounts,
    pub inputs: Vec<RuleInput>,
    /// The smallest change of one input that moves the level; `null` when
    /// no single input moves it or the rule cannot be read.
    pub flip: Option<RuleFlip>,
    /// The source keys of the rule, and of any source that contests it.
    pub sources: Vec<String>,
}

/// A rule's levels over the versions, and the versions it could not read,
/// with why. The counts add up to the number of versions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuleCounts {
    pub levels: Vec<LevelCount>,
    pub unread: Vec<UnreadCount>,
}

/// How many versions a rule could not read, for one reason.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnreadCount {
    pub reason: CannotTellCode,
    pub versions: u32,
}

/// One input a rule read, on the spine as fitted.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuleInput {
    pub measurement: Measurement,
    /// The vertebra it belongs to, for a measurement of one vertebra or
    /// one disc.
    pub level: Option<Level>,
    /// The curve it belongs to, for a measurement of one curve.
    pub region: Option<CurveRegion>,
    /// `null` when the films cannot give it.
    pub value: Option<f64>,
    /// One standard deviation, when the fit gives one.
    pub sd: Option<f64>,
    /// The value the rule compares it with, when it has one.
    pub threshold: Option<f64>,
    pub status: ValueStatus,
}

/// The smallest change of one input, in that input's own standard
/// deviations, that moves the rule's level.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuleFlip {
    pub measurement: Measurement,
    pub level: Option<Level>,
    pub region: Option<CurveRegion>,
    pub change_in_sd: f64,
    /// The same change in the measurement's own unit.
    pub change_value: f64,
    pub moves_to: Level,
}

/// One selective fusion test: each criterion, and whether all the readable
/// ones hold together.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectiveReading {
    /// `selective-thoracic` or `selective-lumbar`.
    pub rule: LevelRule,
    pub criteria: Vec<SelectiveCriterion>,
    /// The versions in which every readable criterion holds; criteria that
    /// cannot be read are left out, never counted as met.
    pub all_hold: Readable,
    /// True when the share is between 0.2 and 0.8, so pairs for both the
    /// selective and the longer fusion are proposed.
    pub both_regions_offered: bool,
    pub sources: Vec<String>,
}

/// One criterion of a selective fusion test.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectiveCriterion {
    pub measurement: Measurement,
    pub region: Option<CurveRegion>,
    pub threshold: f64,
    /// True when the criterion holds below the threshold, false above it.
    pub holds_below: bool,
    /// `null` when it cannot be read.
    pub value: Option<Band>,
    pub holds: Readable,
}

// ---- Level pairs

/// One (upper, lower) pair of instrumented vertebrae.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LevelPair {
    pub upper: Level,
    pub lower: Level,
    /// The versions in which this is the pair most rules give.
    pub versions_holding: u32,
    /// Rules whose most frequent level is exactly this pair's end.
    pub rules_at: Vec<LevelRule>,
    /// Rules whose most frequent level is one away.
    pub rules_near: Vec<LevelRule>,
    /// Rules whose most frequent level is further away.
    pub rules_against: Vec<LevelRule>,
    /// When the lower end stops one above a landmark: the published odds
    /// of adding-on for stopping short of it, a population figure.
    pub stop_short: Vec<StopShortOdds>,
    /// The checks that score this pair's lower end.
    pub checks: Vec<PairCheck>,
    /// Free segments between the lower end and the sacrum.
    pub free_segments_below: u32,
}

/// The published odds of adding-on when the lower end stops one above the
/// landmark a rule names.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StopShortOdds {
    pub rule: LevelRule,
    /// The landmark's level on the spine as fitted.
    pub landmark: Level,
    pub odds_ratio: f64,
    pub source: String,
}

/// A check on a pair's lower end: the versions in which it flags.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PairCheck {
    /// `liv-index`.
    pub rule: LevelRule,
    pub flagged: Readable,
    pub source: String,
}

// ---- Placements

/// One placement: its ends, an anchor at each pedicle of every
/// instrumented level, and two rods.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImplantLayout {
    /// Unique within one set of proposals and one comparison.
    pub id: String,
    pub change: LayoutChange,
    pub upper: Level,
    pub lower: Level,
    /// Both pedicles of every level from `upper` to `lower`, top to
    /// bottom, left before right; a pedicle left empty has kind `none`.
    pub anchors: Vec<AnchorProposal>,
    /// Left, then right.
    pub rods: Vec<RodProposal>,
    /// The joints whose capsules the screws take away, each named by the
    /// vertebra above the disc.
    pub released_levels: Vec<Level>,
}

/// How a placement differs from the first placement.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum LayoutChange {
    /// The first placement itself.
    First {},
    /// The lower end moved to `level`.
    EndsAt { level: Level },
    /// The upper end moved to `level`.
    StartsAt { level: Level },
    /// Screws at the key levels only.
    FewerScrews {},
    /// Cobalt chrome rods of 6.0 mm.
    StifferRods {},
    /// Titanium alloy rods of 5.5 mm.
    SofterRods {},
    /// Hooks in place of screws at the upper end.
    HooksAtTop {},
    /// Changed by a person.
    ChangedByHand {},
}

/// What sits in one pedicle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum AnchorKind {
    Screw,
    Hook,
    /// Nothing: the pedicle is left empty.
    None,
}

/// One pedicle's anchor. For a hook, `entryMm` is where it holds the
/// bone, and the screw sizes are `null`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnchorProposal {
    pub level: Level,
    pub side: BodySide,
    pub kind: AnchorKind,
    /// The entry point implied by the table chord on the generic bone
    /// shaped to the fit, patient frame, mm. A screw with a printed chord
    /// spread then slides along its own direction, by at most one printed
    /// SD, so the rod seats line up; its entry plus `chordMm` along
    /// `directionUnit` is then not the body's front exit.
    pub entry_mm: [f64; 3],
    pub entry_status: ValueStatus,
    /// The screw's direction in the patient frame, a unit vector pointing
    /// into the bone.
    pub direction_unit: [f64; 3],
    pub direction_status: ValueStatus,
    /// The medial angle the direction uses, from its table; `null` for a
    /// hook or an empty pedicle.
    pub medial_angle: Option<AngleFromTable>,
    /// Other published medial angles for this level, shown beside it.
    pub medial_angles_beside: Vec<AngleFromTable>,
    /// The angle between the direction and the patient's mid-sagittal
    /// plane, over the versions.
    pub convergence_deg: Band,
    pub diameter_mm: Option<f64>,
    /// Every other diameter chosen in at least 1 of 10 versions.
    pub diameter_also_seen: Vec<f64>,
    pub diameter_status: ValueStatus,
    /// The largest screw diameter allowed per millimetre of outer pedicle
    /// width, in the rule the diameter follows.
    pub width_ratio: Option<f64>,
    /// The diameter other published width rules would give, shown beside
    /// it.
    pub diameters_beside: Vec<DiameterRule>,
    /// Screw diameter over outer pedicle width, over the versions.
    pub screw_to_pedicle: Option<Band>,
    /// The pedicle's chord from its table (the printed mean), mm; `null`
    /// when no table covers this level and age. The entry may sit up to one
    /// printed SD off it.
    pub chord_mm: Option<f64>,
    /// The share of the chord the length aims for.
    pub chord_share: Option<f64>,
    pub length_mm: Option<f64>,
    /// The grid lengths the other published chord shares give; empty when
    /// the tables give one share.
    pub length_also_seen: Vec<f64>,
    pub length_status: ValueStatus,
    /// The lengths used in practice in this region, shortest and longest,
    /// shown and never imposed.
    pub usual_length_mm: Option<[f64; 2]>,
    pub pedicle: PedicleEstimate,
    /// For each diameter of `LayoutChoices.screwDiametersMm`, the versions
    /// in which the drawn outer width is under that diameter divided by
    /// the width ratio the diameter rule uses (the same test as
    /// `versionsTooNarrow`), on every anchor whatever its kind. Empty
    /// without a width table; these are population draws, never measured
    /// pedicle widths.
    pub wider_than_pedicle: Vec<DiameterCount>,
    /// The screw size at this pedicle before the pattern chooses its
    /// kind, including for a hook or an empty place. `null` when no size
    /// can be found; a screw repeats its own size here.
    pub screw_here: Option<ScrewHere>,
    pub looks: Vec<LookCode>,
    /// Source keys behind the chosen diameter; empty without a source.
    pub diameter_sources: Vec<String>,
    /// Source keys behind the chosen length; empty without a source.
    pub length_sources: Vec<String>,
    /// Source keys behind the direction; empty without a source.
    pub direction_sources: Vec<String>,
    /// One source list per entry of `looks`, in the same order.
    pub look_sources: Vec<LookSources>,
    /// The union of this anchor's source keys.
    pub sources: Vec<String>,
}

/// One offered screw diameter against the population pedicle draws.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiameterCount {
    pub diameter_mm: f64,
    /// Versions in which the drawn outer width is too narrow for this diameter.
    pub versions: u32,
}

/// The screw a pedicle could take, whether the pattern places a screw,
/// a hook or nothing there. Sizes come from tables, never a measurement
/// of this child's pedicle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScrewHere {
    pub diameter_mm: f64,
    /// Every other diameter chosen in at least 1 of 10 versions.
    pub diameter_also_seen: Vec<f64>,
    pub length_mm: f64,
    /// The grid lengths the other published chord shares give.
    pub length_also_seen: Vec<f64>,
    /// Screw diameter over outer width; `null` without a width band.
    pub screw_to_pedicle: Option<Band>,
    /// The direction's published medial angle; `null` without a table.
    pub medial_angle: Option<AngleFromTable>,
}

/// The source keys behind one reason an anchor needs a look.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LookSources {
    pub code: LookCode,
    /// Empty when the reason rests only on the films.
    pub sources: Vec<String>,
}

/// A published angle and its source key.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AngleFromTable {
    pub angle_deg: f64,
    pub source: String,
}

/// A published rule for screw diameter against pedicle width, and the
/// diameter it gives here.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiameterRule {
    pub width_ratio: f64,
    /// `null` when no diameter on the grid qualifies.
    pub diameter_mm: Option<f64>,
    pub source: String,
}

/// A pedicle's width from the population tables, over the versions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PedicleEstimate {
    /// `null` when no table covers this level and age.
    pub outer_width_mm: Option<Band>,
    /// Outer width less both walls.
    pub inner_width_mm: Option<Band>,
    /// Versions with an outer width under 3.2 mm.
    pub versions_too_narrow: u32,
    pub versions_inner_under_1_mm: u32,
    pub curve_side: CurveSide,
    pub status: ValueStatus,
    /// Source keys behind the width draws; empty without a width table.
    pub sources: Vec<String>,
}

/// Where a pedicle sits against the curve its vertebra belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum CurveSide {
    Concave,
    Convex,
    /// The vertebra is in no curve.
    Outside,
}

/// One rod.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RodProposal {
    pub side: BodySide,
    /// Whether the rod runs on the concave or the convex side of the main
    /// curve.
    pub curve_side: CurveSide,
    pub material: RodMaterial,
    pub diameter_mm: f64,
    /// The T5 to T12 kyphosis the contour aims for; 0 with typical status
    /// when the fit lacks T5 or T12, so there is no such reading.
    pub target_kyphosis_deg: f64,
    pub target_kyphosis_status: ValueStatus,
    /// The line the rod is straightened to in the frontal plane.
    pub coronal_line: CoronalLine,
    /// Extra kyphosis bent into the rod beyond the target, over its
    /// thoracic span; 0 for a rod contoured to the target.
    pub overbend_deg: f64,
    /// How far each rod seat, at a screw head or a hook, stands back from
    /// its entry along the anchor's direction.
    pub head_offset_mm: f64,
    /// Along the contour from the top head to the bottom head, with no
    /// overhang.
    pub length_mm: f64,
    /// Where the rod's points and length come from: the drawn spine at a
    /// measured scale (`from-template`) or at a typical size (`typical`).
    pub geometry_status: ValueStatus,
    /// The rod's centre line as contoured, before it is seated: top to
    /// bottom, every 2 mm, patient frame. A rod with no overbend passes
    /// through the screw heads of the spine at its target shape. An
    /// overbent rod passes through its two end heads there and bows
    /// further back between them.
    pub points_mm: Vec<[f64; 3]>,
    /// The centre line through the standing spine's heads, in the same
    /// order and with one point per `pointsMm` point. For drawing before
    /// load only; the bench does not read it. Empty when unavailable.
    /// `geometryStatus` covers both lines: typical geometry must never
    /// be shown as measured millimetres.
    pub standing_points_mm: Vec<[f64; 3]>,
    pub sources: Vec<String>,
}

/// The line a rod is straightened to in the frontal plane.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum CoronalLine {
    /// The centre line from the top of the sacrum.
    CentreLine,
    /// The vertical through the centre of L5's lower endplate, standing in
    /// when the sacrum is not marked.
    L5Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum RodMaterial {
    TitaniumAlloy,
    CobaltChrome,
    StainlessSteel,
}

/// The published sizes and materials a person may choose from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayoutChoices {
    pub screw_diameters_mm: Vec<f64>,
    pub screw_lengths_mm: Vec<f64>,
    pub rod_diameters_mm: Vec<f64>,
    pub rod_materials: Vec<RodMaterial>,
    /// Sourced material and diameter pairs, in the proposer's order.
    pub rods: Vec<RodChoice>,
    pub sources: Vec<String>,
}

/// One sourced rod material and diameter that a person may choose.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RodChoice {
    pub material: RodMaterial,
    pub diameter_mm: f64,
}

// ---- What needs a look

/// Why one anchor needs a look.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum LookCode {
    /// A concave pedicle at the apex or next to it, T3 to T9: a CT would
    /// show it.
    ConcaveApexNeedsCt,
    /// Too narrow in more than 1 of 10 versions, or no diameter qualifies.
    MayBeTooNarrow,
    /// A level where published scans place the aorta close in a right
    /// thoracic curve.
    NearTheAorta,
    /// The direction cannot be read from films: the convergence range is
    /// wider than 30 degrees, or the twist is over 30 degrees.
    DirectionUnreadable,
    /// The length is shorter than the tables cover.
    ShorterThanTables,
    /// No table covers this level at this age.
    NoTableForThisAge,
    /// The chord comes from the nearest age band, since no table covers
    /// this age.
    ChordFromNearestAge,
}

/// Something these films cannot tell, and the levels it concerns (empty
/// when it concerns the whole spine).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CannotTell {
    pub code: CannotTellCode,
    pub levels: Vec<Level>,
}

/// Why something cannot be read or settled from these films.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum CannotTellCode {
    /// The first pair holds in fewer than 6 of 10 versions. Its `levels`
    /// names only the ends whose most frequent level is below that share.
    /// When the pair is unsettled but neither end is on its own, `levels`
    /// names both ends of the first pair.
    EndsUnsettled,
    /// The level names may be one off.
    NamesMayBeOneOff,
    /// The top of the sacrum is not marked, so there is no centre line.
    NoSacrum,
    /// No bending films.
    NoBendingFilms,
    /// The bending films were taken standing; the rules were derived on
    /// bends taken lying down.
    StandingBends,
    /// Twist is not measured where no pedicles were marked.
    TwistNotMeasured,
    /// No side film, so kyphosis is typical, not measured.
    NoSideFilm,
    /// The shoulders are outside the model.
    ShouldersOutsideTheModel,
    /// No source maps degrees of twist to a rotation grade.
    NoRotationGrade,
    /// The rule is for another curve type.
    NotThisCurveType,
    /// No curve was found for the rule to read.
    NoCurve,
    /// No fitted vertebra meets the rule's test.
    NoLevelMeetsTheRule,
    /// No published table gives this pedicle's width at this age, so a
    /// reading that needs the pedicle's outline cannot be made.
    NoPedicleTable,
    /// The film carries no scale, so the spine is drawn at a typical size
    /// and a rule that compares a distance on it with millimetres cannot
    /// be read.
    NoFilmScale,
}

// ---- When the names may be one off

/// The first placement with every name moved by `shift` levels (-1 up, +1
/// down), and what that changes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NamingScenario {
    pub shift: i32,
    pub changes: Vec<AnchorChange>,
}

/// One anchor whose kind, size or looks change when the names move.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnchorChange {
    /// The vertebra by its name as read.
    pub level: Level,
    pub side: BodySide,
    /// The same vertebra by its moved name.
    pub renamed_as: Level,
    pub kind_from: AnchorKind,
    pub kind_to: AnchorKind,
    pub diameter_from_mm: Option<f64>,
    pub diameter_to_mm: Option<f64>,
    pub length_from_mm: Option<f64>,
    pub length_to_mm: Option<f64>,
    pub looks_added: Vec<LookCode>,
    pub looks_removed: Vec<LookCode>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::film::FilmKind;
    use crate::film_geometry::ImageRight;
    use crate::landmarks::Landmarks;
    use crate::naming::{ChosenBy, LevelNaming, Template};
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

    fn input() -> LayoutInput {
        LayoutInput {
            fit: empty_fit(),
            front: Some(LayoutFilm {
                marks: FilmMarks {
                    film: FilmKind::Front,
                    vertebrae: Vec::new(),
                    landmarks: Landmarks::NONE,
                    naming: LevelNaming {
                        template: Template::Standard,
                        chosen_by: ChosenBy::Engine,
                        alternatives: Vec::new(),
                        from_top: None,
                        from_bottom: None,
                        flags: Vec::new(),
                        l5_confirmed_on: None,
                    },
                    named_for: ImageRight::PatientLeft,
                },
                geometry: FilmGeometry::UNKNOWN,
                size: [2000, 4000],
            }),
            bend_left: None,
            bend_right: None,
            child: ChildFacts {
                age_years: Some(13.0),
                weight_kg: None,
                activity: Some(ActivityLevel::VeryActive),
            },
            naming: NamingDoubt::MayBeOneOff,
            versions: 32,
            seed: 7,
            pedicle_child_share: None,
        }
    }

    fn band(p50: f64) -> Band {
        Band {
            p10: p50 - 1.0,
            p50,
            p90: p50 + 1.0,
        }
    }

    fn anchor() -> AnchorProposal {
        AnchorProposal {
            level: Level::T8,
            side: BodySide::Left,
            kind: AnchorKind::Screw,
            entry_mm: [-30.0, 12.0, 250.0],
            entry_status: ValueStatus::FromTemplate,
            direction_unit: [1.0, 0.0, 0.0],
            direction_status: ValueStatus::Measured,
            medial_angle: Some(AngleFromTable {
                angle_deg: 12.0,
                source: "upendra-2010".to_string(),
            }),
            medial_angles_beside: vec![AngleFromTable {
                angle_deg: 6.0,
                source: "zheng-2009".to_string(),
            }],
            convergence_deg: band(10.0),
            diameter_mm: Some(4.5),
            diameter_also_seen: vec![4.0, 5.0],
            diameter_status: ValueStatus::FromTables,
            width_ratio: Some(1.25),
            diameters_beside: vec![DiameterRule {
                width_ratio: 0.8,
                diameter_mm: None,
                source: "misenhimer-1989".to_string(),
            }],
            screw_to_pedicle: Some(band(1.1)),
            chord_mm: Some(38.0),
            chord_share: Some(0.8),
            length_mm: Some(30.0),
            length_also_seen: Vec::new(),
            length_status: ValueStatus::FromTables,
            usual_length_mm: Some([30.0, 35.0]),
            pedicle: PedicleEstimate {
                outer_width_mm: Some(band(4.0)),
                inner_width_mm: None,
                versions_too_narrow: 5,
                versions_inner_under_1_mm: 9,
                curve_side: CurveSide::Concave,
                status: ValueStatus::FromTables,
                sources: Vec::new(),
            },
            wider_than_pedicle: Vec::new(),
            screw_here: None,
            looks: vec![LookCode::ConcaveApexNeedsCt],
            diameter_sources: Vec::new(),
            length_sources: Vec::new(),
            direction_sources: Vec::new(),
            look_sources: Vec::new(),
            sources: vec!["gao-2017".to_string()],
        }
    }

    fn proposals() -> LayoutProposals {
        let first = ImplantLayout {
            id: "first".to_string(),
            change: LayoutChange::First {},
            upper: Level::T4,
            lower: Level::L1,
            anchors: vec![anchor()],
            rods: vec![RodProposal {
                side: BodySide::Left,
                curve_side: CurveSide::Concave,
                material: RodMaterial::CobaltChrome,
                diameter_mm: 5.5,
                target_kyphosis_deg: 25.0,
                target_kyphosis_status: ValueStatus::Measured,
                coronal_line: CoronalLine::L5Vertical,
                overbend_deg: 20.0,
                head_offset_mm: 15.0,
                length_mm: 300.0,
                geometry_status: ValueStatus::FromTemplate,
                points_mm: vec![[0.0, 0.0, 400.0], [0.0, 0.0, 398.0]],
                standing_points_mm: Vec::new(),
                sources: vec!["wilke-2024".to_string()],
            }],
            released_levels: vec![Level::T4],
        };
        let mut lower = first.clone();
        lower.id = "ends-at-L2".to_string();
        lower.change = LayoutChange::EndsAt { level: Level::L2 };
        LayoutProposals {
            tier: FitTier::TwoFilms,
            versions: 32,
            curves: vec![LayoutCurve {
                region: CurveRegion::MainThoracic,
                convex: BodySide::Right,
                versions_found: 32,
                versions_major: 32,
                upper_end: vec![LevelCount {
                    level: Level::T5,
                    versions: 32,
                }],
                lower_end: vec![LevelCount {
                    level: Level::T12,
                    versions: 32,
                }],
                apex: vec![LevelCount {
                    level: Level::T8,
                    versions: 32,
                }],
                cobb_deg: band(40.0),
                bending_cobb_deg: None,
                structural: Readable::Share { versions: 32 },
            }],
            classification: vec![CurveTypeCount {
                curve_type: Some(CurveType::MainThoracic),
                lumbar_modifier: None,
                sagittal_modifier: Some(SagittalModifier::Normal),
                versions: 32,
            }],
            selective: vec![SelectiveReading {
                rule: LevelRule::SelectiveThoracic,
                criteria: vec![SelectiveCriterion {
                    measurement: Measurement::CobbRatio,
                    region: Some(CurveRegion::MainThoracic),
                    threshold: 1.2,
                    holds_below: false,
                    value: Some(band(1.5)),
                    holds: Readable::NotReadable {
                        reason: CannotTellCode::NoBendingFilms,
                    },
                }],
                all_hold: Readable::Share { versions: 20 },
                both_regions_offered: true,
                sources: vec!["lenke-2003".to_string()],
            }],
            rules: vec![RuleVote {
                rule: LevelRule::LevPlusOne5C,
                end: RuleEnd::Lower,
                counts: RuleCounts {
                    levels: vec![LevelCount {
                        level: Level::L1,
                        versions: 30,
                    }],
                    unread: vec![UnreadCount {
                        reason: CannotTellCode::NotThisCurveType,
                        versions: 2,
                    }],
                },
                inputs: vec![RuleInput {
                    measurement: Measurement::CoronalTilt,
                    level: Some(Level::T12),
                    region: None,
                    value: Some(12.0),
                    sd: Some(1.6),
                    threshold: None,
                    status: ValueStatus::Measured,
                }],
                flip: Some(RuleFlip {
                    measurement: Measurement::CoronalTilt,
                    level: Some(Level::T12),
                    region: None,
                    change_in_sd: 1.5,
                    change_value: 2.4,
                    moves_to: Level::L2,
                }),
                sources: vec!["liu-2020".to_string()],
            }],
            pairs: vec![LevelPair {
                upper: Level::T4,
                lower: Level::L1,
                versions_holding: 20,
                rules_at: vec![LevelRule::EndPlusOne],
                rules_near: vec![LevelRule::Uev5C],
                rules_against: vec![LevelRule::LenkeL3L4],
                stop_short: vec![StopShortOdds {
                    rule: LevelRule::StableMinusOne,
                    landmark: Level::L2,
                    odds_ratio: 2.6,
                    source: "liu-2020".to_string(),
                }],
                checks: vec![PairCheck {
                    rule: LevelRule::LivIndex,
                    flagged: Readable::Share { versions: 3 },
                    source: "chen-2025".to_string(),
                }],
                free_segments_below: 4,
            }],
            layouts: vec![first, lower],
            first_is_sketch: false,
            upper_ends: Vec::new(),
            lower_ends: Vec::new(),
            choices: LayoutChoices {
                screw_diameters_mm: vec![4.0, 4.5],
                screw_lengths_mm: vec![25.0, 27.5],
                rod_diameters_mm: vec![5.5, 6.0],
                rod_materials: vec![RodMaterial::TitaniumAlloy],
                rods: vec![RodChoice {
                    material: RodMaterial::TitaniumAlloy,
                    diameter_mm: 5.5,
                }],
                sources: vec!["balikci-2019".to_string()],
            },
            cannot_tell: vec![CannotTell {
                code: CannotTellCode::NamesMayBeOneOff,
                levels: Vec::new(),
            }],
            naming_scenarios: vec![NamingScenario {
                shift: -1,
                changes: vec![AnchorChange {
                    level: Level::T8,
                    side: BodySide::Left,
                    renamed_as: Level::T7,
                    kind_from: AnchorKind::Screw,
                    kind_to: AnchorKind::Hook,
                    diameter_from_mm: Some(4.5),
                    diameter_to_mm: None,
                    length_from_mm: Some(30.0),
                    length_to_mm: None,
                    looks_added: vec![LookCode::MayBeTooNarrow],
                    looks_removed: Vec::new(),
                }],
            }],
        }
    }

    fn rebuild() -> RebuildInput {
        RebuildInput {
            layout: input(),
            upper: Level::T4,
            lower: Level::L1,
            anchors: vec![AnchorChoice {
                level: Level::T8,
                side: BodySide::Left,
                kind: AnchorKind::Screw,
                diameter_mm: None,
                length_mm: Some(30.0),
            }],
            rod_material: RodMaterial::CobaltChrome,
            rod_diameter_mm: 5.5,
        }
    }

    fn proposals_with_pedicle_details() -> LayoutProposals {
        let mut proposals = proposals();
        proposals.upper_ends = vec![LevelCount {
            level: Level::T4,
            versions: 32,
        }];
        proposals.lower_ends = vec![LevelCount {
            level: Level::L1,
            versions: 32,
        }];
        let anchor = &mut proposals.layouts[0].anchors[0];
        anchor.wider_than_pedicle = vec![DiameterCount {
            diameter_mm: 4.5,
            versions: 5,
        }];
        anchor.screw_here = Some(ScrewHere {
            diameter_mm: 4.5,
            diameter_also_seen: vec![4.0, 5.0],
            length_mm: 30.0,
            length_also_seen: vec![32.5],
            screw_to_pedicle: Some(band(1.1)),
            medial_angle: anchor.medial_angle.clone(),
        });
        anchor.pedicle.sources = vec!["gao-2017".to_string()];
        anchor.diameter_sources = vec!["sato-2022".to_string()];
        anchor.length_sources = vec!["upendra-2010".to_string()];
        anchor.direction_sources = vec!["upendra-2010".to_string()];
        anchor.look_sources = vec![LookSources {
            code: LookCode::ConcaveApexNeedsCt,
            sources: vec!["gao-2017".to_string()],
        }];
        proposals.layouts[0].rods[0].standing_points_mm =
            vec![[1.0, 2.0, 400.0], [1.0, 2.0, 398.0]];
        proposals
    }

    #[test]
    fn proposals_spell_codes_and_units_and_read_back() {
        let value = serde_json::to_value(proposals()).expect("the proposals serialise");
        assert_eq!(value["tier"], "two-films");
        assert_eq!(value["firstIsSketch"], false);
        assert_eq!(
            value["choices"]["rods"],
            json!([{ "material": "titanium-alloy", "diameterMm": 5.5 }])
        );
        assert_eq!(value["upperEnds"], json!([]));
        assert_eq!(value["lowerEnds"], json!([]));
        assert_eq!(value["layouts"][0]["change"], json!({"kind": "first"}));
        assert_eq!(
            value["layouts"][1]["change"],
            json!({"kind": "ends-at", "level": "L2"})
        );
        let anchor = &value["layouts"][0]["anchors"][0];
        assert_eq!(anchor["entryMm"], json!([-30.0, 12.0, 250.0]));
        assert_eq!(anchor["kind"], "screw");
        assert_eq!(anchor["pedicle"]["innerWidthMm"], serde_json::Value::Null);
        assert_eq!(anchor["pedicle"]["versionsInnerUnder1Mm"], 9);
        assert_eq!(anchor["looks"], json!(["concave-apex-needs-ct"]));
        assert_eq!(anchor["widerThanPedicle"], json!([]));
        assert_eq!(anchor["screwHere"], serde_json::Value::Null);
        assert_eq!(anchor["pedicle"]["sources"], json!([]));
        for field in [
            "diameterSources",
            "lengthSources",
            "directionSources",
            "lookSources",
        ] {
            assert_eq!(anchor[field], json!([]));
        }
        assert_eq!(value["layouts"][0]["rods"][0]["coronalLine"], "l5-vertical");
        assert_eq!(
            value["layouts"][0]["rods"][0]["standingPointsMm"],
            json!([])
        );
        let rule = &value["rules"][0];
        assert_eq!(rule["rule"], "lev-plus-one-5c");
        assert_eq!(rule["counts"]["unread"][0]["reason"], "not-this-curve-type");
        assert_eq!(value["pairs"][0]["rulesNear"], json!(["uev-5c"]));
        assert_eq!(value["pairs"][0]["rulesAgainst"], json!(["lenke-l3-l4"]));
        assert_eq!(
            value["selective"][0]["criteria"][0]["holds"],
            json!({"kind": "not-readable", "reason": "no-bending-films"})
        );
        assert_eq!(
            value["curves"][0]["bendingCobbDeg"],
            serde_json::Value::Null
        );
        assert_eq!(
            value["classification"][0]["lumbarModifier"],
            serde_json::Value::Null
        );
        assert_eq!(value["namingScenarios"][0]["shift"], -1);
        assert_eq!(
            serde_json::from_value::<LayoutProposals>(value).expect("the proposals read back"),
            proposals()
        );
    }

    #[test]
    fn the_input_writes_absent_films_as_null_and_reads_back() {
        let value = serde_json::to_value(input()).expect("the input serialises");
        assert_eq!(value["bendLeft"], serde_json::Value::Null);
        assert_eq!(value["pedicleChildShare"], serde_json::Value::Null);
        assert_eq!(value["naming"], "may-be-one-off");
        assert_eq!(
            value["child"],
            json!({"ageYears": 13.0, "weightKg": null, "activity": "very-active"})
        );
        assert_eq!(value["front"]["size"], json!([2000, 4000]));
        assert_eq!(
            serde_json::from_value::<LayoutInput>(value).expect("the input reads back"),
            input()
        );
    }

    #[test]
    fn hand_changes_and_pedicle_details_keep_their_wire_names() {
        let value = serde_json::to_value(rebuild()).expect("the changes serialise");
        assert_eq!(value["rodMaterial"], "cobalt-chrome");
        assert_eq!(value["rodDiameterMm"], 5.5);
        assert_eq!(
            value["anchors"][0],
            json!({"level": "T8", "side": "left", "kind": "screw", "diameterMm": null, "lengthMm": 30.0})
        );
        assert_eq!(
            serde_json::from_value::<RebuildInput>(value).expect("the changes read back"),
            rebuild()
        );

        let proposals = proposals_with_pedicle_details();
        let value = serde_json::to_value(&proposals).expect("the details serialise");
        assert_eq!(value["upperEnds"], json!([{"level": "T4", "versions": 32}]));
        assert_eq!(value["lowerEnds"], json!([{"level": "L1", "versions": 32}]));
        let anchor = &value["layouts"][0]["anchors"][0];
        assert_eq!(
            anchor["widerThanPedicle"],
            json!([{"diameterMm": 4.5, "versions": 5}])
        );
        assert_eq!(anchor["screwHere"]["diameterMm"], 4.5);
        assert_eq!(anchor["screwHere"]["lengthAlsoSeen"], json!([32.5]));
        assert_eq!(anchor["screwHere"]["medialAngle"]["source"], "upendra-2010");
        assert_eq!(
            anchor["lookSources"],
            json!([{"code": "concave-apex-needs-ct", "sources": ["gao-2017"]}])
        );
        assert_eq!(
            value["layouts"][0]["rods"][0]["standingPointsMm"][0],
            json!([1.0, 2.0, 400.0])
        );
        assert_eq!(
            serde_json::from_value::<LayoutProposals>(value).expect("the details read back"),
            proposals
        );
    }

    #[test]
    fn unknown_fields_are_refused_at_each_new_boundary() {
        let value = serde_json::to_value(rebuild()).expect("the changes serialise");
        for path in ["", "/layout", "/anchors/0"] {
            let mut changed = value.clone();
            changed
                .pointer_mut(path)
                .and_then(serde_json::Value::as_object_mut)
                .expect("an object")
                .insert("extra".to_string(), json!(1));
            assert!(
                serde_json::from_value::<RebuildInput>(changed).is_err(),
                "{path}"
            );
        }

        let value =
            serde_json::to_value(proposals_with_pedicle_details()).expect("the details serialise");
        for path in [
            "/upperEnds/0",
            "/lowerEnds/0",
            "/layouts/0/anchors/0/widerThanPedicle/0",
            "/layouts/0/anchors/0/screwHere",
            "/layouts/0/anchors/0/screwHere/medialAngle",
            "/layouts/0/anchors/0/lookSources/0",
        ] {
            let mut changed = value.clone();
            changed
                .pointer_mut(path)
                .and_then(serde_json::Value::as_object_mut)
                .expect("an object")
                .insert("extra".to_string(), json!(1));
            assert!(
                serde_json::from_value::<LayoutProposals>(changed).is_err(),
                "{path}"
            );
        }

        let value = serde_json::to_value(input()).expect("the input serialises");
        for path in ["", "/front", "/child"] {
            let mut changed = value.clone();
            changed
                .pointer_mut(path)
                .and_then(serde_json::Value::as_object_mut)
                .expect("an object")
                .insert("extra".to_string(), json!(1));
            assert!(
                serde_json::from_value::<LayoutInput>(changed).is_err(),
                "{path}"
            );
        }

        let value = serde_json::to_value(proposals()).expect("the proposals serialise");
        for path in [
            "",
            "/curves/0",
            "/curves/0/upperEnd/0",
            "/curves/0/cobbDeg",
            "/curves/0/structural",
            "/classification/0",
            "/selective/0",
            "/selective/0/criteria/0",
            "/selective/0/criteria/0/holds",
            "/rules/0",
            "/rules/0/counts",
            "/rules/0/counts/unread/0",
            "/rules/0/inputs/0",
            "/rules/0/flip",
            "/pairs/0",
            "/pairs/0/stopShort/0",
            "/pairs/0/checks/0",
            "/layouts/0",
            "/layouts/0/change",
            "/layouts/1/change",
            "/layouts/0/anchors/0",
            "/layouts/0/anchors/0/medialAngle",
            "/layouts/0/anchors/0/diametersBeside/0",
            "/layouts/0/anchors/0/pedicle",
            "/layouts/0/rods/0",
            "/choices",
            "/choices/rods/0",
            "/cannotTell/0",
            "/namingScenarios/0",
            "/namingScenarios/0/changes/0",
        ] {
            let mut changed = value.clone();
            changed
                .pointer_mut(path)
                .and_then(serde_json::Value::as_object_mut)
                .expect("an object")
                .insert("extra".to_string(), json!(1));
            assert!(
                serde_json::from_value::<LayoutProposals>(changed).is_err(),
                "{path}"
            );
        }
    }
}
