//! Levels run in order, top to bottom, with no repeats.

use shapes::checks::{CheckFinding, CheckKind, Severity};
use shapes::vertebra::FilmMarks;

/// Report the film once if any adjacent pair repeats or reverses a level.
pub fn check(marks: &FilmMarks) -> Option<CheckFinding> {
    marks
        .vertebrae
        .windows(2)
        .any(|pair| pair[0].level >= pair[1].level)
        .then(|| {
            crate::finding(
                CheckKind::LevelsOutOfOrder,
                Severity::Blocking,
                marks.film,
                None,
            )
        })
}
