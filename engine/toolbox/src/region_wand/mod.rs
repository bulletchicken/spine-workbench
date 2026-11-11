//! The region wand: outside clicks add missed bone; inside clicks trim an
//! outline. Hit testing is shared by preview and commit. Click history lives
//! in the request, never in the engine. Segmentation follows a bounded
//! random-walker solve, then robust side fits with exact pin constraints.

pub mod carve;
mod distance_map;
mod geometry;
pub mod grow;
pub mod hover;
pub mod local_edges;
pub mod refit;

use auto_pass::handoffs::FilmPassCache;
use film_reader::decoded_film::DecodedFilm;
use geometry::*;
use shapes::{
    confidence::ConfidenceReason,
    errors::{EngineError, ErrorCode},
    film_geometry::FilmGeometry,
    landmarks::Landmarks,
    toolbox::{
        RegionWandInput, RegionWandOutput, ToolPreview, ToolWarning, WandClick, WandPolarity,
    },
};

/// Apply the newest click, after checking the same target and polarity shown
/// by hover. The optional phase edge multiplier remains off by default; the
/// raw local pixels are sufficient, including before the first film pass.
pub fn apply(
    film: &DecodedFilm,
    _geometry: Option<&FilmGeometry>,
    _cache: Option<&FilmPassCache>,
    input: &RegionWandInput,
) -> Result<RegionWandOutput, EngineError> {
    let Some(last) = input.clicks.last() else {
        return Err(EngineError::new(
            ErrorCode::ToolRefused,
            "a region request needs a click",
        ));
    };
    let preview = hover::preview(film, &input.marks, last.at, input.case_version)?;
    let hit = match preview {
        ToolPreview::Add { vertebra, .. } => Some((vertebra, WandPolarity::Add)),
        ToolPreview::Subtract { vertebra, .. } => Some((vertebra, WandPolarity::Subtract)),
        ToolPreview::Nothing { reason, .. } => {
            return Err(EngineError::new(
                if reason == shapes::toolbox::MissReason::OutsideFilm {
                    ErrorCode::ToolClickOutsideFilm
                } else {
                    ErrorCode::ToolRefused
                },
                "the preview has no region action",
            )
            .with_detail(match reason {
                shapes::toolbox::MissReason::NotSupportedOnFilm => "not-supported-on-film",
                shapes::toolbox::MissReason::AllPinned => "all-pinned",
                shapes::toolbox::MissReason::TooSmall => "too-small",
                shapes::toolbox::MissReason::Ambiguous => "ambiguous",
                _ => "off-spine",
            }));
        }
        _ => None,
    };
    if hit != Some((input.vertebra, last.polarity)) {
        return Err(EngineError::new(
            ErrorCode::ToolRefused,
            "the click target or polarity does not match its preview",
        ));
    }
    let index = input
        .marks
        .vertebrae
        .iter()
        .position(|v| v.id == input.vertebra)
        .ok_or_else(|| {
            EngineError::new(
                ErrorCode::ToolUnknownVertebra,
                "the requested vertebra is missing",
            )
        })?;
    let old = input.marks.vertebrae[index].marks.corners();
    let q = old.map(|m| m.at);
    if !valid_quad(&q, area(&q)) {
        return Err(EngineError::new(
            ErrorCode::ToolRefitFailed,
            "the old outline is not a valid convex quad",
        ));
    }
    let clicks = session(&input.clicks, height(&q), film)?;
    let neighbours: Vec<_> = input
        .marks
        .vertebrae
        .iter()
        .filter(|v| v.id != input.vertebra)
        .map(|v| v.marks.corners().map(|m| m.at))
        .collect();
    let crop = local_edges::build(film, &q, &neighbours, &clicks)?;
    let solution = grow::solve(&crop)?;
    let mut warnings = crop.warnings.clone();
    if solution.residual > 1e-3 {
        warnings.push(ToolWarning::LowConfidence {
            residual: solution.residual,
        });
    }
    let outline = carve::contour(&crop, &solution.values, &mut warnings)?;
    let fitted = refit::fit(&outline, &old, crop.factor)?;
    let ratio = area(&fitted).abs() / area(&q).abs();
    if !(0.4..=2.5).contains(&ratio) {
        warnings.push(ToolWarning::AreaChange { ratio });
    }
    for neighbour in input
        .marks
        .vertebrae
        .iter()
        .filter(|v| v.id != input.vertebra)
    {
        let other = neighbour.marks.corners().map(|m| m.at);
        let smaller = area(&other).abs().min(area(&fitted).abs());
        if smaller > 0. {
            let fraction = overlap(&fitted, &other) / smaller;
            if fraction > 0.1 {
                warnings.push(ToolWarning::NeighbourOverlap {
                    with: neighbour.id,
                    fraction,
                });
            }
        }
    }
    let mut marks = input.marks.clone();
    let target = &mut marks.vertebrae[index];
    refit::apply(target, fitted);
    target.outline = Some(outline.clone());
    if !target
        .confidence
        .reasons
        .contains(&ConfidenceReason::EditedByUser)
    {
        target
            .confidence
            .reasons
            .push(ConfidenceReason::EditedByUser);
    }
    marks.landmarks.c7 = Landmarks::c7_of(&marks.vertebrae);
    Ok(RegionWandOutput {
        case_version: input.case_version,
        marks,
        changed: vec![input.vertebra],
        outline,
        growth: solution.frames,
        solver_iterations: solution.iterations,
        solver_residual: solution.residual,
        warnings,
    })
}
fn session(
    clicks: &[WandClick],
    h: f64,
    film: &DecodedFilm,
) -> Result<Vec<WandClick>, EngineError> {
    let mut kept: Vec<WandClick> = Vec::new();
    for click in clicks {
        if !click.at.is_finite()
            || click.at.x() < 0.
            || click.at.y() < 0.
            || click.at.x() >= f64::from(film.image.width())
            || click.at.y() >= f64::from(film.image.height())
        {
            return Err(EngineError::new(
                ErrorCode::ToolClickOutsideFilm,
                "a session click is outside the film",
            ));
        }
        kept.retain(|older| {
            older.polarity == click.polarity || distance(older.at, click.at) >= 0.15 * h
        });
        kept.push(*click);
        if kept.len() > 20 {
            kept.remove(0);
        }
    }
    Ok(kept)
}
