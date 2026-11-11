//! Bending films: the two endplate lines of each vertebra.

use crate::handoffs::{FoundBody, Polarity, Strip};
use shapes::errors::EngineError;
use shapes::film::FilmKind;
use shapes::film_geometry::ImageRight;
use shapes::marks::EndplateLines;

/// The two lines, with each pair ordered patient left then patient right.
pub fn place(
    strip: &Strip,
    body: &FoundBody,
    polarity: Polarity,
) -> Result<EndplateLines, EngineError> {
    if !matches!(strip.film, FilmKind::BendLeft | FilmKind::BendRight) {
        return Err(super::back_to_film::refused(
            "endplate lines require a bending film",
        ));
    }
    let corners = super::walls::body_corners(strip, body, polarity)?;
    let [left, right] = if super::back_to_film::facing(strip)? == ImageRight::PatientLeft {
        [1, 0]
    } else {
        [0, 1]
    };
    Ok(EndplateLines {
        sup: [corners.sup[left], corners.sup[right]],
        inf: [corners.inf[left], corners.inf[right]],
    })
}
