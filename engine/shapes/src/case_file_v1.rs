//! `CaseFileV1`: the documented data file, `case.json`, inside a saved case.
//!
//! A saved case is a zip holding exactly `case.json`, one
//! `films/<kind>.png` per film present, and a short `README.txt`. The
//! films are lossless grayscale PNGs re-encoded from the decoded pixels;
//! the original file's bytes are never stored, so no tag or EXIF can leak.
//!
//! `case.json` holds no dates, no names, no free-text notes and no
//! measurements. It is read strictly (see [`crate::reading`]): unknown
//! fields, a wrong format, a newer version, non-finite numbers, points far
//! outside their film and levels out of order are all refused.

use crate::film::{Film, FilmPng};
use crate::vertebra::FilmMarks;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The whole case.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaseFileV1 {
    /// Always `"spine-case"`.
    #[schemars(extend("const" = "spine-case"))]
    #[cfg_attr(feature = "typescript", ts(type = "'spine-case'"))]
    pub format: String,
    /// Always `1`.
    #[schemars(extend("const" = 1))]
    #[cfg_attr(feature = "typescript", ts(type = "1"))]
    pub version: u32,
    pub saved_with: SavedWith,
    /// Only the films present, each kind at most once.
    pub films: Vec<Film>,
    /// One entry per film that has marks.
    pub marks: Vec<FilmMarks>,
    /// The id the next new vertebra gets. Only ever goes up, so ids are
    /// never reused.
    pub next_vertebra_id: u32,
}

/// A case file's contents: the record and each film's stored PNG, one per
/// film the record lists. Unpacking a `.spinecase` gives one; packing
/// takes one, and checks the record strictly before anything is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenedCase {
    pub case: CaseFileV1,
    pub films: Vec<FilmPng>,
}

/// Which app wrote the file: its version, and nothing else.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavedWith {
    /// A version such as `0.1.0`: letters, digits, `.`, `+` and `-` only,
    /// at most 32 characters, so it cannot carry anything else.
    pub app: String,
}

impl CaseFileV1 {
    /// A case with no films yet, written by app version `app`.
    pub fn new(app: impl Into<String>) -> Self {
        CaseFileV1 {
            format: crate::CASE_FORMAT.to_owned(),
            version: crate::CASE_VERSION,
            saved_with: SavedWith { app: app.into() },
            films: Vec::new(),
            marks: Vec::new(),
            next_vertebra_id: 1,
        }
    }
}
