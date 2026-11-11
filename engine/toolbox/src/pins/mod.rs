//! Pins: a dragged mark stays exactly where it was put. Corner moves
//! jointly refit the other unpinned corners and neighbouring vertebrae;
//! pedicle and landmark moves do not reshape the chain. Requests carry
//! all edit state and answers echo the request's case version.

mod geometry;
mod pedicles;
pub mod re_solve;
mod targets;

use auto_pass::handoffs::FilmPassCache;
use film_reader::decoded_film::DecodedFilm;
use shapes::confidence::ConfidenceReason;
use shapes::errors::{EngineError, ErrorCode};
use shapes::film_geometry::FilmGeometry;
use shapes::landmarks::Landmarks;
use shapes::toolbox::{PinAction, PinInput, PinOutput};

/// Move or release one pin, preserving every other pin exactly.
///
/// Film evidence is deliberately not part of the linear shape/disc
/// energy. The cache argument keeps the same doorway as the wands.
pub fn apply(
    film: &DecodedFilm,
    geometry: Option<&FilmGeometry>,
    _cache: Option<&FilmPassCache>,
    input: &PinInput,
) -> Result<PinOutput, EngineError> {
    if input.marks.film != film.facts.kind
        || input
            .marks
            .vertebrae
            .iter()
            .any(|v| !v.marks.suits(input.marks.film))
    {
        return Err(EngineError::new(
            ErrorCode::ToolPinInvalid,
            "the pin's marks do not belong to this film",
        ));
    }
    if let PinAction::Move { to, .. } = input.action
        && (!to.is_finite()
            || to.x() < -0.5
            || to.y() < -0.5
            || to.x() > f64::from(film.image.width()) - 0.5
            || to.y() > f64::from(film.image.height()) - 0.5)
    {
        return Err(EngineError::new(
            ErrorCode::ToolClickOutsideFilm,
            "the pin lies outside the film",
        ));
    }
    let mut marks = input.marks.clone();
    let target = targets::apply(&mut marks, input.action)?;
    if let Some((index, corner)) = target.corner
        && let PinAction::Move { to, .. } = input.action
    {
        let old = input.marks.vertebrae[index].marks.corners().map(|m| m.at);
        let direct = marks.vertebrae[index].marks.corners().map(|m| m.at);
        if !geometry::valid(direct, old) {
            return Err(EngineError::new(
                ErrorCode::ToolPinInvalid,
                "the pin would fold the vertebra or make an extreme corner angle",
            ));
        }
        if old[corner] != to {
            re_solve::around(&input.marks, &mut marks, index, geometry)?;
        }
    }
    let mut changed = Vec::new();
    for (index, (before, after)) in input
        .marks
        .vertebrae
        .iter()
        .zip(&mut marks.vertebrae)
        .enumerate()
    {
        if Some(index) == target.vertebra {
            if !after
                .confidence
                .reasons
                .contains(&ConfidenceReason::EditedByUser)
            {
                after
                    .confidence
                    .reasons
                    .push(ConfidenceReason::EditedByUser);
            }
        } else if before.marks != after.marks {
            after
                .confidence
                .reasons
                .retain(|reason| *reason != ConfidenceReason::EditedByUser);
        }
        if before != after {
            changed.push(after.id);
        }
    }
    marks.landmarks.c7 = Landmarks::c7_of(&marks.vertebrae);
    Ok(PinOutput {
        case_version: input.case_version,
        marks,
        changed,
        warnings: Vec::new(),
    })
}
