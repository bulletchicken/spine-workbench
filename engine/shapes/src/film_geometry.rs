//! The two facts every film needs before work starts: how big a pixel is,
//! and which way the film is turned and faces. Plus what the user has
//! confirmed, and the engine's guesses that the user confirms from.
//!
//! # The parallel camera, in upright pixels
//!
//! A patient point lands on the film, shown upright (after its quarter
//! turns, before any mirroring), at
//! `u = u0 + across / su` and `v = v0 − z / sv`, where `across` is the
//! patient direction named by [`ImageRight`] (`+y` for patient left, `−y`
//! for patient right, `+x` for anterior, `−x` for posterior), and `su`,
//! `sv` are the upright pixel's width and height in mm.
//!
//! The stored spacings belong to the film as stored: `mmPerPx` is the
//! step along a film row (x), `mmPerPxVertical` (or `mmPerPx` when it is
//! `null`) the step down a film column (y). After an even number of
//! quarter turns `su = mmPerPx` and `sv` is the vertical spacing; after an
//! odd number the two swap. The point in original film pixels is the
//! upright one with the turns undone: the inverse of the app's view
//! transform with mirroring off (`uprightMatrix` in
//! `app/src/film-viewer/film-space.ts`).

use crate::pixels::{Px, PxRect};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Clockwise quarter turns, 0 to 3, that show a film upright. A view
/// transform only: the stored pixels are never turned.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(type = "0 | 1 | 2 | 3"))]
#[serde(try_from = "u8", into = "u8")]
pub struct QuarterTurns(u8);

impl QuarterTurns {
    pub const NONE: QuarterTurns = QuarterTurns(0);

    /// `None` unless `turns` is 0, 1, 2 or 3.
    pub const fn new(turns: u8) -> Option<Self> {
        if turns < 4 {
            Some(QuarterTurns(turns))
        } else {
            None
        }
    }

    pub const fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for QuarterTurns {
    type Error = String;

    fn try_from(turns: u8) -> Result<Self, Self::Error> {
        QuarterTurns::new(turns).ok_or_else(|| format!("quarter turns must be 0 to 3, not {turns}"))
    }
}

impl From<QuarterTurns> for u8 {
    fn from(turns: QuarterTurns) -> u8 {
        turns.0
    }
}

impl JsonSchema for QuarterTurns {
    fn schema_name() -> Cow<'static, str> {
        "QuarterTurns".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({ "type": "integer", "enum": [0, 1, 2, 3] })
    }
}

/// The patient direction that points to screen right once the film is
/// shown upright (after its quarter turns, before any mirroring).
///
/// Front and bending films use `patient-left` or `patient-right`; side
/// films use `anterior` or `posterior`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum ImageRight {
    PatientLeft,
    PatientRight,
    Anterior,
    Posterior,
}

impl ImageRight {
    /// True for the two directions a front or bending film can face.
    pub const fn suits_front_view(self) -> bool {
        matches!(self, ImageRight::PatientLeft | ImageRight::PatientRight)
    }

    /// The direction pointing to screen right on the same film mirrored.
    pub const fn mirrored(self) -> ImageRight {
        match self {
            ImageRight::PatientLeft => ImageRight::PatientRight,
            ImageRight::PatientRight => ImageRight::PatientLeft,
            ImageRight::Anterior => ImageRight::Posterior,
            ImageRight::Posterior => ImageRight::Anterior,
        }
    }
}

/// Where a film's scale came from. Saved with the film, so the scale can
/// always be traced and worked out again.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(
    tag = "source",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ScaleSource {
    /// From the file's spacing tags (kept in [`crate::film::KeptTags`]),
    /// by the named rule.
    File { rule: FileScaleRule },
    /// From one or more two-click rulers, combined by inverse variance.
    Ruler { rulers: Vec<Ruler> },
    /// From a round calibration object of known size: the fitted circle
    /// and the object's true diameter, so the scale is
    /// `diameterMm / (2 · radiusPx)`.
    KnownObject {
        centre: Px,
        radius_px: f64,
        diameter_mm: f64,
    },
}

/// Which spacing tags gave the scale, and therefore at which depth it is
/// true. In order of preference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum FileScaleRule {
    /// Pixel spacing calibrated on a visible fiducial.
    Fiducial,
    /// Pixel spacing from an assumed or known geometry.
    Geometry,
    /// Pixel spacing differing from the detector spacing: corrected, by an
    /// unstated method.
    CorrectedUnknownMethod,
    /// Detector spacing divided by the estimated magnification.
    DetectorOverMagnification,
    /// Detector spacing times source-to-patient over source-to-detector.
    DetectorFromDistances,
    /// Detector spacing, uncorrected: sizes read larger than in the patient.
    DetectorUncorrected,
    /// Pixel spacing with no way to tell whether it was corrected.
    UnknownCalibration,
}

/// One two-click ruler: two points on the film and the length between them.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Ruler {
    pub a: Px,
    pub b: Px,
    pub mm: f64,
}

/// The depth at which a scale is true. Lengths at other depths read
/// longer or shorter by the beam's magnification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum ScalePlane {
    FiducialDepth,
    AssumedDepth,
    TableOrBucky,
    DetectorFace,
    Isocentre,
    ObjectDepth,
    Unknown,
}

/// A scale, worked out but not yet chosen: from the file's tags, a ruler
/// or a known object.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScaleResult {
    /// Horizontal (column) spacing, in mm per pixel.
    pub mm_per_px: f64,
    /// Vertical (row) spacing, only when pixels are not square.
    pub mm_per_px_vertical: Option<f64>,
    pub source: ScaleSource,
    pub plane: ScalePlane,
    /// Relative standard uncertainty, when one is known (`0.01` = 1%).
    pub relative_uncertainty: Option<f64>,
    /// True when the result is usable but weak (for example a short ruler).
    pub weak: bool,
}

/// What the user has settled about a film. Each is set only by an explicit
/// press, never by a guess.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Confirmations {
    pub upright: bool,
    /// Front and bending films: which side is the patient's left.
    pub left_right: bool,
    /// Side films: which way the patient faces.
    pub facing: bool,
    pub scale: bool,
    /// The brightness is the right way round (bone bright).
    pub polarity: bool,
    /// The border was checked for names or IDs, and any were blanked out.
    pub privacy: bool,
}

/// A film's geometry as saved.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FilmGeometry {
    /// Horizontal (column) spacing in mm per pixel; `null` until a scale is
    /// known. Angles work without it; lengths do not.
    pub mm_per_px: Option<f64>,
    /// Vertical (row) spacing, only when pixels are not square.
    pub mm_per_px_vertical: Option<f64>,
    pub scale: Option<ScaleSource>,
    pub quarter_turns: QuarterTurns,
    pub image_right: Option<ImageRight>,
    pub confirmed: Confirmations,
}

impl FilmGeometry {
    /// Nothing known yet: no scale, no turns, no facing, nothing confirmed.
    pub const UNKNOWN: FilmGeometry = FilmGeometry {
        mm_per_px: None,
        mm_per_px_vertical: None,
        scale: None,
        quarter_turns: QuarterTurns::NONE,
        image_right: None,
        confirmed: Confirmations {
            upright: false,
            left_right: false,
            facing: false,
            scale: false,
            polarity: false,
            privacy: false,
        },
    };
}

/// How strongly the evidence supports a guess.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum Certainty {
    High,
    Medium,
    Low,
    None,
}

/// The evidence behind an upright guess.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum UprightCue {
    FileOrientation,
    ExifOrientation,
    Lungs,
    PelvisMass,
    SpineWidening,
    NeckAndHead,
    ThoracicShape,
    LumbarShape,
}

/// The evidence behind a facing guess.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum FacingCue {
    FileOrientation,
    LeadMarker,
    UpperBackBulge,
    PosteriorElements,
    Hips,
}

/// One cue and how strongly it pointed to the guess (`−1` against, `+1`
/// for), or `null` when it could not be measured.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UprightEvidence {
    pub cue: UprightCue,
    pub strength: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FacingEvidence {
    pub cue: FacingCue,
    pub strength: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UprightGuess {
    pub quarter_turns: QuarterTurns,
    pub certainty: Certainty,
    pub evidence: Vec<UprightEvidence>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageRightGuess {
    /// `null` when nothing points either way: then neither answer is
    /// pre-selected.
    pub image_right: Option<ImageRight>,
    pub certainty: Certainty,
    /// What the file's orientation tag says, when it says anything.
    pub file_says: Option<ImageRight>,
    pub evidence: Vec<FacingEvidence>,
    /// A lead letter reads mirrored: the image was flipped after it was
    /// taken.
    pub looks_mirrored: bool,
}

/// Whether the brightness looks the right way round.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum PolarityGuess {
    /// Bone already looks bright.
    AsStored,
    /// Bone looks dark: offer a flip.
    LooksInverted,
    /// No air on the film to judge by.
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum MarkerLetter {
    L,
    R,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum MarkerMaterial {
    /// A lead letter placed on the patient's side when the film was taken.
    Lead,
    /// A letter burnt in by software afterwards.
    Digital,
}

/// A left/right letter found on a film. It proposes; the user decides.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LeadMarker {
    pub letter: MarkerLetter,
    pub bounds: PxRect,
    /// Template match, `−1` to `1`.
    pub score: f64,
    /// How far ahead of the other letter's best match.
    pub margin: f64,
    pub mirrored: bool,
    /// Rotation of the letter, degrees clockwise, 0 to 355.
    pub rotation_deg: u32,
    /// A letter cut out of a lead plate, rather than a solid letter.
    pub cut_out: bool,
    pub material: MarkerMaterial,
}

/// Everything the engine proposes for a film's geometry. Never saved: the
/// user's answers are saved in [`FilmGeometry`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeometryGuess {
    pub upright: UprightGuess,
    pub image_right: ImageRightGuess,
    pub polarity: PolarityGuess,
    /// Every scale the file's tags support, best first.
    pub scales: Vec<ScaleResult>,
    pub lead_markers: Vec<LeadMarker>,
    /// Boxes that may hold burnt-in text, for the privacy check.
    pub text_hints: Vec<PxRect>,
    /// Collimation bands found along the edges, left out of statistics.
    pub collimation: Vec<PxRect>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarter_turns_refuse_four() {
        assert!(serde_json::from_str::<QuarterTurns>("3").is_ok());
        assert!(serde_json::from_str::<QuarterTurns>("4").is_err());
        assert_eq!(QuarterTurns::new(2).map(QuarterTurns::get), Some(2));
    }

    #[test]
    fn mirroring_twice_changes_nothing() {
        for right in [
            ImageRight::PatientLeft,
            ImageRight::PatientRight,
            ImageRight::Anterior,
            ImageRight::Posterior,
        ] {
            assert_eq!(right.mirrored().mirrored(), right);
            assert_ne!(right.mirrored(), right);
        }
    }
}
