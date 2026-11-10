//! The known answer for renaming a film's marks after its facing changes:
//! `proving-ground/known-answers/side-swap.json` gives marks before, the
//! facing, and the marks after. Every point keeps its position and flags;
//! only the left/right (or anterior/posterior) names change hands. The
//! app's case reducer, which mirrors `FilmMarks::renamed_for`, must give
//! the same answers.

// Reading the fixture file is the point of this test; the engine itself
// never touches files.
#![allow(clippy::disallowed_methods)]

use serde::Deserialize;
use shapes::film_geometry::ImageRight;
use shapes::vertebra::FilmMarks;
use std::path::Path;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KnownAnswers {
    cases: Vec<Case>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    what: String,
    facing: ImageRight,
    before: FilmMarks,
    after: FilmMarks,
}

fn cases() -> Vec<Case> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../proving-ground/known-answers/side-swap.json");
    let text = std::fs::read_to_string(path).expect("side-swap.json is readable");
    let answers: KnownAnswers = serde_json::from_str(&text).expect("side-swap.json is well formed");
    answers.cases
}

#[test]
fn renaming_gives_the_known_answer() {
    let cases = cases();
    assert!(!cases.is_empty());
    for case in cases {
        assert_eq!(
            case.before.renamed_for(case.facing),
            case.after,
            "{}",
            case.what
        );
    }
}

#[test]
fn renaming_back_restores_the_marks() {
    for case in cases() {
        assert_eq!(
            case.after.renamed_for(case.before.named_for),
            case.before,
            "{}",
            case.what
        );
    }
}
