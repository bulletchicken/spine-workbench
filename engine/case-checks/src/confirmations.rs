//! Left/right, scale and L5 have been confirmed by the user.

use shapes::checks::{CheckFinding, CheckKind, Severity};
use shapes::film::{Film, FilmKind};
use shapes::vertebra::FilmMarks;

/// L5 still needs a person, and names require the facing they were given for.
pub fn naming(marks: &FilmMarks, film: Option<&Film>) -> Vec<CheckFinding> {
    let mut findings = Vec::new();
    if !marks.naming.l5_confirmed() {
        findings.push(crate::finding(
            CheckKind::L5Unconfirmed,
            Severity::Warning,
            marks.film,
            None,
        ));
    }
    if film.is_some_and(|film| film.geometry.image_right != Some(marks.named_for)) {
        findings.push(crate::finding(
            CheckKind::MarksNamedForOtherFacing,
            Severity::Blocking,
            marks.film,
            None,
        ));
    }
    findings
}

/// Report each unconfirmed fact once. Left/right on the standing front film
/// blocks using its pedicle names; the other facts remain visible warnings.
pub fn film(film: &Film) -> Vec<CheckFinding> {
    let confirmed = film.geometry.confirmed;
    let mut findings = Vec::new();
    let mut add = |kind, severity| {
        findings.push(crate::finding(kind, severity, film.kind, None));
    };
    if !confirmed.upright {
        add(CheckKind::UprightUnconfirmed, Severity::Warning);
    }
    if film.kind != FilmKind::Side && !confirmed.left_right {
        add(
            CheckKind::LeftRightUnconfirmed,
            if film.kind == FilmKind::Front {
                Severity::Blocking
            } else {
                Severity::Warning
            },
        );
    }
    if film.kind == FilmKind::Side && !confirmed.facing {
        add(CheckKind::FacingUnconfirmed, Severity::Warning);
    }
    if !confirmed.scale {
        add(CheckKind::ScaleUnconfirmed, Severity::Warning);
    }
    if !confirmed.polarity {
        add(CheckKind::PolarityUnconfirmed, Severity::Warning);
    }
    if !confirmed.privacy {
        add(CheckKind::PrivacyUnchecked, Severity::Warning);
    }
    findings
}
