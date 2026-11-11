//! A warning when the chain looks shifted by one level.

use shapes::checks::{CheckFinding, CheckKind, Severity};
use shapes::levels::Level;
use shapes::naming::NamingFlag;
use shapes::vertebra::FilmMarks;

/// Use the naming evidence recorded by the pass or an edit. Confirming L5
/// settles an end-count disagreement, but preserves the non-standard-count
/// explanation and any still-unknown names.
pub fn check(marks: &FilmMarks) -> Vec<CheckFinding> {
    let mut findings = Vec::new();
    let naming = &marks.naming;
    if !naming.l5_confirmed() && naming.flags.contains(&NamingFlag::EndsDisagree) {
        findings.push(crate::finding(
            CheckKind::LikelyOffByOne,
            Severity::Warning,
            marks.film,
            marks
                .vertebrae
                .iter()
                .find(|vertebra| vertebra.level == Level::L5),
        ));
    }
    if naming.flags.contains(&NamingFlag::NonStandardCount) {
        let counted = naming.template.levels();
        findings.push(crate::finding(
            CheckKind::NonStandardCount,
            Severity::Warning,
            marks.film,
            marks
                .vertebrae
                .iter()
                .find(|vertebra| !counted.contains(&vertebra.level)),
        ));
    }
    if naming.flags.contains(&NamingFlag::NamesUnknown) {
        findings.push(crate::finding(
            CheckKind::NamesUnknown,
            Severity::Blocking,
            marks.film,
            None,
        ));
    }
    findings
}
