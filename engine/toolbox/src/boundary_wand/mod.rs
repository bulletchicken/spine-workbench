//! The boundary wand shares one prepared edit between preview and commit.
//! It changes the count only by an explicit split or merge, keeps stable
//! ids, and returns a complete replacement for one film's current marks.

mod geometry;
pub mod hover;
pub mod merge;
pub mod renumber;
pub mod split;
mod window;

use auto_pass::handoffs::FilmPassCache;
use film_reader::decoded_film::DecodedFilm;
use shapes::confidence::ConfidenceReason;
use shapes::errors::{EngineError, ErrorCode};
use shapes::film_geometry::FilmGeometry;
use shapes::landmarks::Landmarks;
use shapes::toolbox::{
    BoundaryAction, BoundaryWandInput, BoundaryWandOutput, MissReason, ToolPreview,
    ToolPreviewInput,
};
use shapes::vertebra::FilmMarks;

pub(super) fn refused(reason: &str, message: &str) -> EngineError {
    EngineError::new(ErrorCode::ToolRefused, message).with_detail(reason)
}

pub(super) fn count_after(count: usize, action: BoundaryAction) -> usize {
    match action {
        BoundaryAction::Split => count + 1,
        BoundaryAction::Merge => count - 1,
    }
}

/// Resolve the same geometric and naming edit a click will commit.
pub fn preview(
    film: &DecodedFilm,
    geometry: Option<&FilmGeometry>,
    cache: Option<&FilmPassCache>,
    input: &ToolPreviewInput,
) -> Result<ToolPreview, EngineError> {
    prepare(
        film,
        geometry,
        cache,
        &BoundaryWandInput {
            case_version: input.case_version,
            marks: input.marks.clone(),
            at: input.at,
            new_vertebra_id: input.new_vertebra_id,
        },
    )
    .map(|(preview, _)| preview)
}

/// Commit the prepared split or merge without retaining any edit state.
pub fn apply(
    film: &DecodedFilm,
    geometry: Option<&FilmGeometry>,
    cache: Option<&FilmPassCache>,
    input: &BoundaryWandInput,
) -> Result<BoundaryWandOutput, EngineError> {
    let (preview, answer) = prepare(film, geometry, cache, input)?;
    answer.ok_or_else(|| match preview {
        ToolPreview::Nothing {
            reason: MissReason::OutsideFilm,
            ..
        } => EngineError::new(
            ErrorCode::ToolClickOutsideFilm,
            "The click is outside the film",
        ),
        ToolPreview::Nothing { reason, .. } => refused(
            "no-action",
            &format!("The boundary wand has no action here: {reason:?}"),
        ),
        _ => refused("no-action", "The boundary wand has no action here"),
    })
}

fn validate(
    film: &DecodedFilm,
    marks: &FilmMarks,
    cache: Option<&FilmPassCache>,
) -> Result<(), EngineError> {
    if marks.film != film.facts.kind || cache.is_some_and(|cache| cache.evidence.film != marks.film)
    {
        return Err(refused(
            "film-mismatch",
            "The marks and cached evidence must belong to this film",
        ));
    }
    for (i, v) in marks.vertebrae.iter().enumerate() {
        if !v.marks.suits(marks.film)
            || !geometry::valid(geometry::quad(&v.marks))
            || marks.vertebrae[..i].iter().any(|old| old.id == v.id)
        {
            return Err(refused(
                "invalid-marks",
                "The film needs distinct ids and valid outlines",
            ));
        }
    }
    Ok(())
}

fn prepare(
    film: &DecodedFilm,
    geometry: Option<&FilmGeometry>,
    cache: Option<&FilmPassCache>,
    input: &BoundaryWandInput,
) -> Result<(ToolPreview, Option<BoundaryWandOutput>), EngineError> {
    let case_version = input.case_version;
    // A bending film deliberately has no polygon editing semantics.
    if matches!(
        input.marks.film,
        shapes::film::FilmKind::BendLeft | shapes::film::FilmKind::BendRight
    ) {
        return Ok((
            ToolPreview::Nothing {
                case_version,
                reason: MissReason::NotSupportedOnFilm,
            },
            None,
        ));
    }
    validate(film, &input.marks, cache)?;
    let hit = hover::hit(
        &input.marks,
        input.at,
        geometry,
        film.image.width(),
        film.image.height(),
    );
    let (action, index) = match hit {
        hover::Hit::Miss(reason) => {
            return Ok((
                ToolPreview::Nothing {
                    case_version,
                    reason,
                },
                None,
            ));
        }
        hover::Hit::Split(index) => (BoundaryAction::Split, index),
        hover::Hit::Merge(index) => (BoundaryAction::Merge, index),
    };
    if action == BoundaryAction::Split
        && input
            .marks
            .vertebrae
            .iter()
            .any(|v| v.id == input.new_vertebra_id)
    {
        return Err(refused(
            "id-already-used",
            "The new vertebra id is already present",
        ));
    }
    let mut marks = input.marks.clone();
    let mut warnings = Vec::new();
    let parent = marks.vertebrae[index].id;
    let (line, new_vertebra, removed_vertebra) = match action {
        BoundaryAction::Split => (
            Some(split::apply(
                film,
                cache,
                &mut marks,
                index,
                input.at,
                input.new_vertebra_id,
            )?),
            Some(input.new_vertebra_id),
            None,
        ),
        BoundaryAction::Merge => (
            None,
            None,
            Some(merge::apply(&mut marks, index, &mut warnings)?),
        ),
    };
    debug_assert_eq!(
        marks.vertebrae.len(),
        count_after(input.marks.vertebrae.len(), action)
    );
    let levels_after = match renumber::apply(&input.marks, &mut marks, &mut warnings) {
        Ok(levels) => levels,
        Err(reason) => {
            return Ok((
                ToolPreview::Nothing {
                    case_version,
                    reason,
                },
                None,
            ));
        }
    };
    for body in &mut marks.vertebrae {
        if (body.id == parent || Some(body.id) == new_vertebra)
            && !body
                .confidence
                .reasons
                .contains(&ConfidenceReason::EditedByUser)
        {
            body.confidence.reasons.push(ConfidenceReason::EditedByUser);
        }
    }
    marks.landmarks.c7 = Landmarks::c7_of(&marks.vertebrae);
    let changed = marks
        .vertebrae
        .iter()
        .filter(|v| input.marks.vertebrae.iter().find(|old| old.id == v.id) != Some(*v))
        .map(|v| v.id)
        .collect();
    let preview = match action {
        BoundaryAction::Split => ToolPreview::Split {
            case_version,
            vertebra: parent,
            line: line.expect("a split has a line"),
            levels_after,
        },
        BoundaryAction::Merge => ToolPreview::Merge {
            case_version,
            upper: parent,
            lower: removed_vertebra.expect("a merge removes its lower id"),
            levels_after,
        },
    };
    Ok((
        preview,
        Some(BoundaryWandOutput {
            case_version,
            action,
            marks,
            changed,
            new_vertebra,
            removed_vertebra,
            warnings,
        }),
    ))
}
