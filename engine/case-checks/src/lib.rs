//! Is the spine complete and in order? Each check returns findings, each a
//! warning or blocking, pointing at the film and vertebra concerned.
//!
//! | Module | Holds |
//! |---|---|
//! | [`presence`] | every vertebra present |
//! | [`order`] | levels in order, top to bottom |
//! | [`off_by_one`] | a likely off-by-one level |
//! | [`films_agree`] | front and side level sets match |
//! | [`confirmations`] | left/right, scale and L5 confirmed |

#![forbid(unsafe_code)]

pub mod confirmations;
pub mod films_agree;
pub mod off_by_one;
pub mod order;
pub mod presence;

use shapes::case_file_v1::CaseFileV1;
use shapes::checks::{CheckFinding, CheckKind, Severity};
use shapes::film::FilmKind;
use shapes::vertebra::VertebraOnFilm;

/// Check the live case without changing it. Findings follow the case's film
/// order, then the comparison between front and side. A film not yet searched
/// has no missing-level findings; its unconfirmed facts are still reported.
///
/// This is separate from strict saved-file validation: a live edit may leave
/// levels out of order, and that must produce a finding rather than a refusal.
pub fn check_case(case: &CaseFileV1) -> Vec<CheckFinding> {
    let mut findings = Vec::new();
    let mut kinds = Vec::new();
    for kind in case
        .films
        .iter()
        .map(|film| film.kind)
        .chain(case.marks.iter().map(|marks| marks.film))
    {
        if !kinds.contains(&kind) {
            kinds.push(kind);
        }
    }
    for kind in kinds {
        let film = case.films.iter().find(|film| film.kind == kind);
        let marks = case.marks.iter().find(|marks| marks.film == kind);
        if let Some(marks) = marks {
            findings.extend(presence::check(marks));
            findings.extend(order::check(marks));
            findings.extend(off_by_one::check(marks));
            findings.extend(confirmations::naming(marks, film));
        }
        if let Some(film) = film {
            findings.extend(confirmations::film(film));
        }
        if let Some(marks) = marks {
            findings.extend(presence::pedicles(marks));
        }
    }
    findings.extend(films_agree::check(case));
    findings
}

fn finding(
    kind: CheckKind,
    severity: Severity,
    film: FilmKind,
    vertebra: Option<&VertebraOnFilm>,
) -> CheckFinding {
    CheckFinding {
        kind,
        severity,
        film: Some(film),
        vertebra_id: vertebra.map(|vertebra| vertebra.id),
        level: vertebra.map(|vertebra| vertebra.level),
    }
}

#[cfg(test)]
mod known_cases;
