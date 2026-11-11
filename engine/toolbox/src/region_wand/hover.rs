//! One hit test for both region previews and clicks. An inside click trims;
//! an outside click grows the nearest outline, unless the choice is ambiguous.

use super::geometry::{boundary_distance, centre, distance, height, inside};
use film_reader::decoded_film::DecodedFilm;
use shapes::{
    errors::{EngineError, ErrorCode},
    pixels::Px,
    toolbox::{MissReason, ToolPreview},
    vertebra::FilmMarks,
};

pub fn preview(
    film: &DecodedFilm,
    marks: &FilmMarks,
    at: Px,
    case_version: u32,
) -> Result<ToolPreview, EngineError> {
    if film.facts.kind != marks.film || marks.vertebrae.iter().any(|v| !v.marks.suits(marks.film)) {
        return Err(EngineError::new(
            ErrorCode::ToolRefused,
            "marks do not belong to this film",
        ));
    }
    let none = |reason| {
        Ok(ToolPreview::Nothing {
            case_version,
            reason,
        })
    };
    if marks.film.is_bending() {
        return none(MissReason::NotSupportedOnFilm);
    }
    if !at.is_finite()
        || at.x() < 0.
        || at.y() < 0.
        || at.x() >= f64::from(film.image.width())
        || at.y() >= f64::from(film.image.height())
    {
        return none(MissReason::OutsideFilm);
    }
    let mut within = Vec::new();
    let mut nearby = Vec::new();
    for (index, v) in marks.vertebrae.iter().enumerate() {
        let q = v.marks.corners().map(|m| m.at);
        let h = height(&q);
        if !q.iter().all(|p| p.is_finite()) || !h.is_finite() {
            return Err(EngineError::new(
                ErrorCode::ToolRefused,
                "outline is not finite",
            ));
        }
        if inside(at, &q) {
            within.push((distance(at, centre(&q)), index));
        } else {
            let d = boundary_distance(at, &q);
            if d <= 0.6 * h {
                nearby.push((d, index));
            }
        }
    }
    let compare = |a: &(f64, usize), b: &(f64, usize)| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1));
    within.sort_by(compare);
    nearby.sort_by(compare);
    let subtract = !within.is_empty();
    let candidates = if subtract { &within } else { &nearby };
    let Some(&(d, index)) = candidates.first() else {
        return none(MissReason::OffSpine);
    };
    let v = &marks.vertebrae[index];
    let corners = v.marks.corners();
    let h = height(&corners.map(|m| m.at));
    if !subtract
        && candidates
            .get(1)
            .is_some_and(|other| other.0 - d <= 2.0_f64.max(0.03 * h))
    {
        return none(MissReason::Ambiguous);
    }
    if h < 16. {
        return none(MissReason::TooSmall);
    }
    if corners.iter().all(|m| m.pinned) {
        return none(MissReason::AllPinned);
    }
    Ok(if subtract {
        ToolPreview::Subtract {
            case_version,
            vertebra: v.id,
        }
    } else {
        ToolPreview::Add {
            case_version,
            vertebra: v.id,
        }
    })
}
