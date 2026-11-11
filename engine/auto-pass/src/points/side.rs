//! Side films: four corners.

use crate::handoffs::{FoundBody, Polarity, Strip};
use shapes::errors::EngineError;
use shapes::film::FilmKind;
use shapes::film_geometry::ImageRight;
use shapes::marks::SideMarks;

/// Place side-film corners, named anterior and posterior in the patient.
pub fn place(
    strip: &Strip,
    body: &FoundBody,
    polarity: Polarity,
) -> Result<SideMarks, EngineError> {
    if strip.film != FilmKind::Side {
        return Err(super::back_to_film::refused(
            "side marks require a side film",
        ));
    }
    let corners = super::walls::body_corners(strip, body, polarity)?;
    from_corners(strip, &corners)
}

pub(super) fn from_corners(
    strip: &Strip,
    corners: &super::walls::BodyCorners,
) -> Result<SideMarks, EngineError> {
    let [anterior, posterior] = if super::back_to_film::facing(strip)? == ImageRight::Anterior {
        [1, 0]
    } else {
        [0, 1]
    };
    Ok(SideMarks {
        sup_a: corners.sup[anterior],
        sup_p: corners.sup[posterior],
        inf_a: corners.inf[anterior],
        inf_p: corners.inf[posterior],
    })
}
