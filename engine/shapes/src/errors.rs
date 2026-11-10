//! Expected failures, as they cross from the engine to the app.
//!
//! `message` is English for developers. Users read the wording the app
//! keeps for each [`ErrorCode`]; an exhaustive switch there means a new
//! code without wording fails the TypeScript build.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Every way an engine call can fail on purpose.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum ErrorCode {
    // Reading a film
    /// Not a PNG, JPEG or DICOM file (judged by its bytes, never its name).
    FilmUnknownFormat,
    /// A DICOM directory file rather than an image.
    FilmIsADirectory,
    /// A DICOM file with no image in it.
    FilmNoPixels,
    /// A compression this app cannot decode.
    FilmUnsupportedCompression,
    /// A bit depth this app cannot read.
    FilmUnsupportedBits,
    /// A colour layout this app cannot read.
    FilmUnsupportedColour,
    /// The file ends early.
    FilmTruncated,
    /// The image data is damaged.
    FilmCorrupt,
    /// The header and the image data disagree on the size.
    FilmSizeMismatch,
    /// Too many pixels, or too big a file.
    FilmTooLarge,

    // Scale and geometry
    /// The ruler's two points are too close together.
    ScaleTooShort,
    /// The length typed for a ruler or object is out of range.
    ScaleBadLength,
    /// No circle could be fitted to the calibration object.
    ScaleFitFailed,
    /// Two rulers disagree by more than their uncertainty allows.
    ScaleDisagree,
    /// No spine found while sensing the geometry.
    GeometryNoSpineFound,

    // The auto pass
    PassNoSpineFound,
    PassCancelled,
    /// The film is too small or too coarse to search.
    PassFilmTooSmall,
    /// The film has no contrast.
    PassFlatFilm,
    /// Too little of the film is usable (shutters, markers, borders).
    PassTooLittleUsable,
    /// The spine shown is too short for the vertebra count.
    PassStripTooShort,

    // The adjusting tools
    ToolClickOutsideFilm,
    /// The hit test found nothing the tool can do here; the detail says why.
    ToolRefused,
    /// No usable contrast around the click.
    ToolNoContrast,
    /// The edit would leave the vertebra with no area.
    ToolWouldRemoveVertebra,
    /// The solver did not settle.
    ToolDidNotConverge,
    /// The new outline could not be turned into four corners.
    ToolRefitFailed,
    /// These two vertebrae cannot be merged.
    ToolMergeInvalid,
    /// No split of the chain fits here.
    ToolSplitInfeasible,
    /// The pin cannot go there.
    ToolPinInvalid,
    /// The pins and gaps asked for cannot all hold at once.
    ToolConstraintsConflict,
    /// The edit names a vertebra the film does not have.
    ToolUnknownVertebra,

    // Case files
    CaseNotAZip,
    CaseUnknownEntry,
    CaseWrongFormat,
    /// Made by a newer version of the app.
    CaseNewerVersion,
    CaseUnknownField,
    CaseMissingFilm,
    CaseFilmSizeMismatch,
    CaseTooLarge,
    CaseInvalidValue,

    // Made-up spines
    /// No spine could be built from the recipe.
    MadeUpUnsatisfiable,

    // The engine itself
    EngineOutOfMemory,
    EngineCrashed,
    EngineOutOfSync,
    EngineLoadFailed,
}

impl ErrorCode {
    /// The code as written in JSON and TypeScript.
    pub fn name(self) -> String {
        serde_json::to_value(self)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_default()
    }
}

/// An expected failure.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EngineError {
    pub code: ErrorCode,
    /// English, for developers.
    pub message: String,
    /// Where or what exactly, when known: a JSON path, a codec name, a
    /// reason code.
    pub detail: Option<String>,
}

impl EngineError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        EngineError {
            code,
            message: message.into(),
            detail: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.name(), self.message)?;
        if let Some(detail) = &self.detail {
            write!(f, " ({detail})")?;
        }
        Ok(())
    }
}

impl std::error::Error for EngineError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_kebab_case() {
        assert_eq!(ErrorCode::CaseNewerVersion.name(), "case-newer-version");
        assert_eq!(
            ErrorCode::FilmUnsupportedCompression.name(),
            "film-unsupported-compression"
        );
    }

    #[test]
    fn the_message_shows_code_and_detail() {
        let error = EngineError::new(ErrorCode::CaseUnknownField, "unknown field `x`")
            .with_detail("films[0].x");
        assert_eq!(
            error.to_string(),
            "case-unknown-field: unknown field `x` (films[0].x)"
        );
    }
}
