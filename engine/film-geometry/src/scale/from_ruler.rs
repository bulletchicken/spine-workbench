//! Two clicks and a length; several rulers combined by inverse variance.

use film_reader::decoded_film::DecodedFilm;
use shapes::errors::{EngineError, ErrorCode};
use shapes::film_geometry::{Ruler, ScalePlane, ScaleResult, ScaleSource};
use shapes::pixels::Px;

use super::from_tags::aspect;

/// A two-click ruler at its own depth. One pixel of click error per end is
/// an experimental uncertainty; it does not include beam magnification.
pub fn from_ruler(film: &DecodedFilm, a: Px, b: Px, mm: f64) -> Result<ScaleResult, EngineError> {
    if !(1.0..=2000.0).contains(&mm) {
        return Err(EngineError::new(
            ErrorCode::ScaleBadLength,
            "the ruler length must be between 1 and 2000 mm",
        ));
    }
    if !on_film(film, a) || !on_film(film, b) {
        return Err(EngineError::new(
            ErrorCode::ToolClickOutsideFilm,
            "the ruler endpoints must lie on the film",
        ));
    }
    let ratio = aspect(film.facts.kept_tags.as_ref());
    let distance = libm::hypot(ratio * (a.x() - b.x()), a.y() - b.y());
    if distance < 50.0 {
        return Err(EngineError::new(
            ErrorCode::ScaleTooShort,
            "the ruler must span at least 50 vertical pixels",
        ));
    }
    let vertical = mm / distance;
    Ok(ScaleResult {
        mm_per_px: ratio * vertical,
        mm_per_px_vertical: (ratio != 1.0).then_some(vertical),
        source: ScaleSource::Ruler {
            rulers: vec![Ruler { a, b, mm }],
        },
        plane: ScalePlane::ObjectDepth,
        relative_uncertainty: Some(2.0_f64.sqrt() / distance),
        weak: distance < 141.0,
    })
}

/// Combine independent rulers only when every pair agrees within two
/// standard deviations. Their depth must be the same; the caller chooses it.
pub fn combine_rulers(film: &DecodedFilm, rulers: &[Ruler]) -> Result<ScaleResult, EngineError> {
    if rulers.is_empty() {
        return Err(EngineError::new(
            ErrorCode::ScaleBadLength,
            "at least one ruler is required",
        ));
    }
    let estimates = rulers
        .iter()
        .map(|ruler| from_ruler(film, ruler.a, ruler.b, ruler.mm))
        .collect::<Result<Vec<_>, _>>()?;
    let uncertainties: Vec<f64> = estimates
        .iter()
        .map(|scale| scale.mm_per_px * scale.relative_uncertainty.unwrap_or(0.0))
        .collect();
    for i in 0..estimates.len() {
        for j in 0..i {
            if (estimates[i].mm_per_px - estimates[j].mm_per_px).abs()
                > 2.0 * libm::hypot(uncertainties[i], uncertainties[j])
            {
                return Err(EngineError::new(
                    ErrorCode::ScaleDisagree,
                    "the rulers disagree by more than their click uncertainty",
                ));
            }
        }
    }
    let weight = uncertainties
        .iter()
        .map(|sigma| 1.0 / (sigma * sigma))
        .sum::<f64>();
    let horizontal = estimates
        .iter()
        .zip(&uncertainties)
        .map(|(scale, sigma)| scale.mm_per_px / (sigma * sigma))
        .sum::<f64>()
        / weight;
    let ratio = aspect(film.facts.kept_tags.as_ref());
    let relative_uncertainty = 1.0 / weight.sqrt() / horizontal;
    Ok(ScaleResult {
        mm_per_px: horizontal,
        mm_per_px_vertical: (ratio != 1.0).then_some(horizontal / ratio),
        source: ScaleSource::Ruler {
            rulers: rulers.to_vec(),
        },
        plane: ScalePlane::ObjectDepth,
        relative_uncertainty: Some(relative_uncertainty),
        weak: relative_uncertainty > 0.01,
    })
}

pub(super) fn on_film(film: &DecodedFilm, at: Px) -> bool {
    at.is_finite()
        && at.x() >= -0.5
        && at.y() >= -0.5
        && at.x() <= f64::from(film.image.width()) - 0.5
        && at.y() <= f64::from(film.image.height()) - 0.5
}
