//! Every vertebra the film's count expects is present.

use shapes::checks::{CheckFinding, CheckKind, Severity};
use shapes::confidence::ConfidenceReason;
use shapes::levels::Level;
use shapes::marks::PedicleMark;
use shapes::vertebra::{FilmMarks, VertebraMarks};

/// Report an unresolved tall outline and each missing measured level. Bending
/// films may hold only the levels being compared, so their gaps are not errors.
/// C7 is an anchor, and levels below L5 are outside the measured T1–L5 chain.
pub fn check(marks: &FilmMarks) -> Vec<CheckFinding> {
    let mut findings = Vec::new();
    for vertebra in &marks.vertebrae {
        if vertebra
            .confidence
            .reasons
            .contains(&ConfidenceReason::OddHeight)
        {
            findings.push(crate::finding(
                CheckKind::VertebraMissing,
                Severity::Blocking,
                marks.film,
                Some(vertebra),
            ));
        }
    }
    if !marks.film.is_bending() {
        for level in marks.naming.template.levels() {
            if (Level::T1..=Level::L5).contains(&level)
                && !marks
                    .vertebrae
                    .iter()
                    .any(|vertebra| vertebra.level == level)
            {
                findings.push(CheckFinding {
                    kind: CheckKind::VertebraMissing,
                    severity: Severity::Blocking,
                    film: Some(marks.film),
                    vertebra_id: None,
                    level: Some(level),
                });
            }
        }
    }
    findings
}

/// A hidden pedicle is a fact to display, not a missing vertebra. The frozen
/// finding shape identifies its vertebra; two hidden pedicles give two findings.
pub fn pedicles(marks: &FilmMarks) -> Vec<CheckFinding> {
    let mut findings = Vec::new();
    for vertebra in &marks.vertebrae {
        if let VertebraMarks::Front { points } = &vertebra.marks {
            for pedicle in [points.ped_l, points.ped_r] {
                if matches!(pedicle, PedicleMark::NotVisible { .. }) {
                    findings.push(crate::finding(
                        CheckKind::PedicleNotVisible,
                        Severity::Warning,
                        marks.film,
                        Some(vertebra),
                    ));
                }
            }
        }
    }
    findings
}
