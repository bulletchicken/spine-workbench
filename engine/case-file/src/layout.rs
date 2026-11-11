//! The entries a case file may hold: `case.json`, `films/<kind>.png` and
//! `README.txt`, and nothing else.

use shapes::film::FilmKind;

pub const CASE_JSON: &str = "case.json";
pub const README: &str = "README.txt";
pub const README_TEXT: &str = "Spine Workbench case, version 1.\n\ncase.json describes the films, geometry and marks.\nfilms/ contains lossless greyscale PNGs under fixed names.\nThe format is documented in docs/contracts/case-file-v1.md in the\nSpine Workbench source repository.\n\nNot a medical device.\n";

/// Only literal, fixed entry names are accepted; no path is normalised.
pub fn is_allowed(name: &[u8]) -> bool {
    name == CASE_JSON.as_bytes()
        || name == README.as_bytes()
        || FilmKind::ALL
            .iter()
            .any(|kind| name == kind.png_path().as_bytes())
}
