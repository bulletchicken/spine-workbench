//! A provisional scale for a front film that states none: the one that
//! makes the median ratio of each body's width on the film to its typical
//! width equal one. A film without spacing is taken to have square pixels;
//! non-square pixels must carry their spacing to be fitted correctly.

use crate::FitError;
use crate::typical_spine::TypicalSpine;
use shapes::film::FilmKind;
use shapes::film_geometry::FilmGeometry;
use shapes::spine_fit::{FilmToFit, FitLengths};

/// The front film's geometry as fitted, and whether its lengths are
/// measured. A film with no spacing needs the typical spine and is fitted
/// as square pixels. Non-square pixels must carry their spacing.
pub fn front_geometry(
    film: &FilmToFit,
    typical: Option<&TypicalSpine>,
) -> Result<(FilmGeometry, FitLengths), FitError> {
    if film.geometry.mm_per_px.is_some() {
        return Ok((film.geometry.clone(), FitLengths::Measured));
    }
    let typical = typical.ok_or(FitError::MissingScale(FilmKind::Front))?;
    let mut ratios: Vec<f64> = film
        .marks
        .vertebrae
        .iter()
        .filter_map(|v| {
            // Quarter turns keep distances between square pixels.
            let [a, b, c, d] = v.marks.corners().map(|m| (m.at.x(), m.at.y()));
            let length = |p: (f64, f64), q: (f64, f64)| libm::hypot(p.0 - q.0, p.1 - q.1);
            let half_width_px = 0.25 * (length(a, b) + length(d, c));
            let ratio = typical.half_width_mm(v.level) / half_width_px;
            (ratio.is_finite() && ratio > 0.0).then_some(ratio)
        })
        .collect();
    if ratios.is_empty() {
        return Err(FitError::InvalidMarks(FilmKind::Front));
    }
    ratios.sort_by(f64::total_cmp);
    let middle = ratios.len() / 2;
    let mm_per_px = if ratios.len().is_multiple_of(2) {
        0.5 * (ratios[middle - 1] + ratios[middle])
    } else {
        ratios[middle]
    };
    Ok((
        FilmGeometry {
            mm_per_px: Some(mm_per_px),
            mm_per_px_vertical: None,
            ..film.geometry.clone()
        },
        FitLengths::TypicalSize,
    ))
}

/// Start an unscaled Side from Front's stored row spacing, with square
/// pixels and Side's own quarter turns. The fit estimates the relative
/// ratio; neither estimate becomes a stated calibration. Non-square
/// pixels must carry their spacing.
pub fn side_geometry(film: &FilmToFit, front: &FilmGeometry) -> FilmGeometry {
    if film.geometry.mm_per_px.is_some() {
        return film.geometry.clone();
    }
    let row = front.mm_per_px.expect("an effective Front scale");
    FilmGeometry {
        mm_per_px: Some(row),
        mm_per_px_vertical: None,
        ..film.geometry.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shapes::film_geometry::QuarterTurns;

    #[test]
    fn the_side_estimate_assumes_square_pixels_and_keeps_its_own_turns() {
        let original = crate::answer_keys::upright_input();
        let film = FilmToFit {
            marks: original.side,
            geometry: FilmGeometry::UNKNOWN,
            size: original.side_size,
            mark_spread: shapes::spine_fit::MarkSpread::Millimetres {
                sd_mm: 1.0,
                shared_sd_mm: 0.0,
            },
            facing_assumed: true,
        };
        let front = FilmGeometry {
            mm_per_px: Some(0.4),
            mm_per_px_vertical: Some(0.8),
            quarter_turns: QuarterTurns::new(1).expect("one turn"),
            ..FilmGeometry::UNKNOWN
        };
        let geometry = side_geometry(&film, &front);
        assert_eq!(geometry.mm_per_px, Some(0.4));
        assert_eq!(geometry.mm_per_px_vertical, None);
        assert_eq!(geometry.quarter_turns, film.geometry.quarter_turns);
        assert_eq!(film.geometry, FilmGeometry::UNKNOWN);
    }
}
