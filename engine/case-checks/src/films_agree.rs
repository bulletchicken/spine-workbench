//! The front and side films name the same levels.

use shapes::case_file_v1::CaseFileV1;
use shapes::checks::{CheckFinding, CheckKind, Severity};
use shapes::film::FilmKind;

/// Compare the ordered level lists only once both standing films have marks.
/// Each film's own order check explains repeated or reversed names separately.
pub fn check(case: &CaseFileV1) -> Option<CheckFinding> {
    let front = case
        .marks
        .iter()
        .find(|marks| marks.film == FilmKind::Front)?;
    let side = case
        .marks
        .iter()
        .find(|marks| marks.film == FilmKind::Side)?;
    (!front
        .vertebrae
        .iter()
        .map(|vertebra| vertebra.level)
        .eq(side.vertebrae.iter().map(|vertebra| vertebra.level)))
    .then_some(CheckFinding {
        kind: CheckKind::FilmsDisagree,
        severity: Severity::Blocking,
        film: None,
        vertebra_id: None,
        level: None,
    })
}
