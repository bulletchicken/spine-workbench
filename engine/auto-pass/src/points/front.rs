//! Front films: four corners and two pedicles, or a pedicle marked not
//! visible rather than guessed.

use crate::handoffs::{FoundBody, Polarity, Strip};
use shapes::errors::EngineError;
use shapes::film::FilmKind;
use shapes::film_geometry::ImageRight;
use shapes::marks::FrontMarks;

/// Place front corners and only those pedicles supported by ring evidence.
pub fn place(
    strip: &Strip,
    body: &FoundBody,
    polarity: Polarity,
) -> Result<FrontMarks, EngineError> {
    super::back_to_film::validate(strip)?;
    let evidence = super::pedicles::evidence(strip);
    place_with_rings(strip, body, polarity, evidence.as_ref())
}

pub(super) fn place_with_rings(
    strip: &Strip,
    body: &FoundBody,
    polarity: Polarity,
    evidence: Option<&super::pedicles::RingEvidence<'_>>,
) -> Result<FrontMarks, EngineError> {
    if strip.film != FilmKind::Front {
        return Err(super::back_to_film::refused(
            "front marks require a front film",
        ));
    }
    let corners = super::walls::body_corners(strip, body, polarity)?;
    from_corners(strip, body, &corners, evidence)
}

pub(super) fn from_corners(
    strip: &Strip,
    body: &FoundBody,
    corners: &super::walls::BodyCorners,
    evidence: Option<&super::pedicles::RingEvidence<'_>>,
) -> Result<FrontMarks, EngineError> {
    let [negative, positive] = super::pedicles::find(strip, body, corners, evidence);
    let [left, right] = if super::back_to_film::facing(strip)? == ImageRight::PatientLeft {
        [1, 0]
    } else {
        [0, 1]
    };
    let pedicles = [negative, positive];
    Ok(FrontMarks {
        sup_l: corners.sup[left],
        sup_r: corners.sup[right],
        inf_l: corners.inf[left],
        inf_r: corners.inf[right],
        ped_l: pedicles[left],
        ped_r: pedicles[right],
    })
}
