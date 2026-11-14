//! Independent line-tilt spreads from the supplied marks, or the fit.

use crate::ReadingsError;
use crate::curves::geometry::upright;
use shapes::film::FilmKind;
use shapes::landmarks::SacrumTop;
use shapes::levels::Level;
use shapes::marks::Mark;
use shapes::spine_fit::{FilmPlacement, FilmToFit, MarkSpread, SpineFit};
use shapes::spine_readings::ReadingsInput;

#[derive(Clone, Copy)]
pub(crate) struct PlateSpread {
    pub front: [f64; 2],
    pub side: [f64; 2],
}

pub(crate) fn from_fit(fit: &SpineFit) -> Vec<PlateSpread> {
    fit.vertebrae
        .iter()
        .map(|v| PlateSpread {
            front: [core::f64::consts::SQRT_2 * v.sd.angles_rad[0]; 2],
            side: [core::f64::consts::SQRT_2 * v.sd.angles_rad[1]; 2],
        })
        .collect()
}

pub(crate) fn from_input(input: &ReadingsInput) -> Result<Vec<PlateSpread>, ReadingsError> {
    for (film, kind) in [
        (&input.front, FilmKind::Front),
        (&input.side, FilmKind::Side),
    ] {
        if let Some(film) = film {
            if film.marks.film != kind {
                return Err(ReadingsError::InvalidValue {
                    field: format!("{}.marks.film", kind.name()),
                    reason: "wrong-film-kind".into(),
                });
            }
            own_sd(film, None)?;
        }
    }
    let mut spreads = from_fit(&input.fit);
    for (i, v) in input.fit.vertebrae.iter().enumerate() {
        if let Some(film) = &input.front
            && let Some(sds) = on_film(film, spacing(film, input.fit.front_film, 1.0), v.level)?
        {
            for (sd, given) in spreads[i].front.iter_mut().zip(sds) {
                if let Some(given) = given {
                    *sd = given;
                }
            }
        }
        if let (Some(film), Some(placement)) = (&input.side, input.fit.side_film)
            && let Some(sds) = on_film(
                film,
                spacing(film, placement, input.fit.side_scale),
                v.level,
            )?
        {
            for (sd, given) in spreads[i].side.iter_mut().zip(sds) {
                if let Some(given) = given {
                    *sd = given;
                }
            }
        }
    }
    Ok(spreads)
}

fn spacing(film: &FilmToFit, placement: FilmPlacement, side_scale: f64) -> [f64; 2] {
    if matches!(film.mark_spread, MarkSpread::Millimetres { .. }) {
        if let Some(row) = film.geometry.mm_per_px {
            return [row, film.geometry.mm_per_px_vertical.unwrap_or(row)];
        }
        // Recover the provisional spacing used for the fit's mark weights;
        // the placement includes the fitted side magnification and gauge.
        return placement.mm_per_px.map(|s| s / side_scale);
    }
    placement.mm_per_px
}

fn point(film: &FilmToFit, spacing: [f64; 2], mark: Mark) -> [f64; 2] {
    let [u, v] = upright(film, mark.at);
    let [su, sv] = if film.geometry.quarter_turns.get().is_multiple_of(2) {
        spacing
    } else {
        [spacing[1], spacing[0]]
    };
    [u * su, v * sv]
}

fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    libm::hypot(a[0] - b[0], a[1] - b[1])
}

fn corner_factor(mark: Mark) -> f64 {
    if mark.inferred { 3.0 } else { 1.0 }
}

fn own_sd(film: &FilmToFit, height: Option<f64>) -> Result<Option<f64>, ReadingsError> {
    let (own, shared, unit) = match film.mark_spread {
        MarkSpread::Millimetres {
            sd_mm,
            shared_sd_mm,
        } => (sd_mm, shared_sd_mm, Some(1.0)),
        MarkSpread::ShareOfHeight {
            share,
            shared_share,
        } => (share, shared_share, height),
    };
    if !own.is_finite() || own < 0.0 || !shared.is_finite() || shared < 0.0 {
        return Err(ReadingsError::InvalidValue {
            field: format!("{}.markSpread", film.marks.film.name()),
            reason: "must-be-finite-and-nonnegative".into(),
        });
    }
    // A common shift cancels from a line's direction.
    Ok(unit.map(|unit| own * unit))
}

pub(crate) fn on_film(
    film: &FilmToFit,
    spacing: [f64; 2],
    level: Level,
) -> Result<Option<[Option<f64>; 2]>, ReadingsError> {
    own_sd(film, None)?;
    let Some(v) = film.marks.vertebrae.iter().find(|v| v.level == level) else {
        return Ok(None);
    };
    if !v.marks.suits(film.marks.film) {
        return Err(ReadingsError::InvalidValue {
            field: format!("{}.marks.vertebrae", film.marks.film.name()),
            reason: "wrong-mark-kind".into(),
        });
    }
    let corners = v.marks.corners();
    let [a, b, c, d] = corners.map(|m| point(film, spacing, m));
    let height = distance(
        [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5],
        [(c[0] + d[0]) * 0.5, (c[1] + d[1]) * 0.5],
    );
    let sigma = own_sd(film, Some(height))?.expect("body height supplied");
    let widths = [distance(a, b), distance(c, d)];
    let plate = |width: f64, ka, kb| {
        (width.is_finite() && width > 0.0 && height.is_finite() && height > 0.0)
            .then(|| sigma * libm::hypot(ka, kb) / width)
    };
    let [a, b, c, d] = corners.map(corner_factor);
    Ok(Some([plate(widths[0], a, b), plate(widths[1], c, d)]))
}

/// A sacral line has two marks but no body height. A millimetre spread
/// therefore gives its tilt spread; a share of height cannot supply one.
pub(crate) fn sacrum_sd(input: &ReadingsInput) -> Result<Option<f64>, ReadingsError> {
    let (Some(film), Some(placement)) = (&input.side, input.fit.side_film) else {
        return Ok(None);
    };
    let Some(SacrumTop::Side { sac_a, sac_p }) = film.marks.landmarks.sacrum_top else {
        return Ok(None);
    };
    let Some(sigma) = own_sd(film, None)? else {
        return Ok(None);
    };
    let spacing = spacing(film, placement, input.fit.side_scale);
    let width = distance(point(film, spacing, sac_a), point(film, spacing, sac_p));
    if !width.is_finite() || width <= 0.0 {
        return Ok(None);
    }
    Ok(Some(
        sigma * libm::hypot(corner_factor(sac_a), corner_factor(sac_p)) / width,
    ))
}
