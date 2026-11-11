//! Strip boundaries back to film pixels.

use crate::handoffs::{EndplateOnStrip, Polarity, Strip, StripRow};
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::FilmKind;
use shapes::film_geometry::ImageRight;
use shapes::marks::Mark;
use shapes::pixels::Px;

/// Map a fractional strip row and column to original film pixel centres.
/// The frozen strip contract is linear between rows, including the across
/// vector. No half pixel is added. Folded or out-of-range positions fail.
pub fn strip_to_film(strip: &Strip, row: f64, col: f64) -> Option<Px> {
    if row < 0.0 || row > strip.rows.checked_sub(1)? as f64 {
        return None;
    }
    mapped(strip, row, col)
}

/// The facing used to name points, including the explicit provisional
/// choice while facing is unknown. Callers save this as `named_for`.
pub fn facing(strip: &Strip) -> Result<ImageRight, EngineError> {
    let front = strip.film != FilmKind::Side;
    let facing = strip.image_right.unwrap_or(if front {
        ImageRight::PatientLeft
    } else {
        ImageRight::Anterior
    });
    if facing.suits_front_view() != front {
        return Err(refused("the facing belongs to another view"));
    }
    Ok(facing)
}

/// The plateau's supported endpoints, upright image negative then positive.
/// Without evidence on both walls the plateau is left unfound.
pub fn plateau_ends(
    strip: &Strip,
    plateau: EndplateOnStrip,
    polarity: Polarity,
) -> Option<[Mark; 2]> {
    validate(strip).ok()?;
    super::walls::plateau_ends(strip, plateau, polarity)
}

pub(super) fn refused(detail: &str) -> EngineError {
    EngineError::new(ErrorCode::PassNoSpineFound, "points could not be placed").with_detail(detail)
}

pub(super) fn validate(strip: &Strip) -> Result<(), EngineError> {
    let count = u64::from(strip.rows) * u64::from(strip.cols);
    let finite = |p: Px| p.x().is_finite() && p.y().is_finite();
    if strip.rows < 2
        || strip.cols < 3
        || strip.cols.is_multiple_of(2)
        || count != strip.values.len() as u64
        || count != strip.valid.len() as u64
        || strip.layout.len() != strip.rows as usize
        || !(strip.row_step.is_finite() && strip.row_step > 0.0)
        || !(strip.col_step.is_finite() && strip.col_step > 0.0)
        || strip.layout.iter().any(|r| {
            !finite(r.centre)
                || !finite(r.across)
                || !finite(r.along)
                || !r.curvature.is_finite()
                || !r.half_width.is_finite()
                || r.half_width < 0.0
                || r.across.x() * r.along.y() - r.across.y() * r.along.x() == 0.0
        })
        || strip
            .evidence
            .ring
            .as_ref()
            .is_some_and(|ring| ring.len() as u64 != count)
    {
        return Err(refused("the strip geometry or pixel arrays are invalid"));
    }
    facing(strip)?;
    Ok(())
}

pub(super) fn layout_at(strip: &Strip, row: f64) -> Option<StripRow> {
    if !row.is_finite() || strip.layout.len() != strip.rows as usize {
        return None;
    }
    let last = strip.rows.checked_sub(1)? as usize;
    let index = (row.max(0.0) as usize).min(last);
    let a = *strip.layout.get(index)?;
    if row < 0.0 || row > last as f64 {
        let distance = row - index as f64;
        return Some(StripRow {
            centre: Px::new(
                a.centre.x() + distance * a.along.x(),
                a.centre.y() + distance * a.along.y(),
            ),
            ..a
        });
    }
    let b = *strip.layout.get((index + 1).min(last))?;
    let t = row - index as f64;
    let mix = |x: f64, y: f64| x + t * (y - x);
    let point = |x: Px, y: Px| Px::new(mix(x.x(), y.x()), mix(x.y(), y.y()));
    Some(StripRow {
        centre: point(a.centre, b.centre),
        across: point(a.across, b.across),
        along: point(a.along, b.along),
        curvature: mix(a.curvature, b.curvature),
        half_width: mix(a.half_width, b.half_width),
        z: mix(a.z, b.z),
    })
}

/// Cropped chain rows alone may extrapolate along the end tangent. They
/// remain inferred; scalar evidence never extrapolates.
pub(super) fn mapped(strip: &Strip, row: f64, col: f64) -> Option<Px> {
    if !col.is_finite() || col < 0.0 || col > strip.cols.checked_sub(1)? as f64 {
        return None;
    }
    let layout = layout_at(strip, row)?;
    let k = col - f64::from(strip.cols - 1) / 2.0;
    if !layout.curvature.is_finite()
        || !strip.col_step.is_finite()
        || strip.col_step <= 0.0
        || 1.0 - layout.curvature * k * strip.col_step <= 0.2
    {
        return None;
    }
    let at = Px::new(
        layout.centre.x() + k * layout.across.x(),
        layout.centre.y() + k * layout.across.y(),
    );
    (at.x().is_finite() && at.y().is_finite()).then_some(at)
}

pub(super) fn sample(strip: &Strip, map: &[f32], row: f64, offset: f64) -> Option<f64> {
    let col = f64::from(strip.cols.checked_sub(1)?) / 2.0 + offset / strip.col_step;
    strip_to_film(strip, row, col)?;
    let (r, c) = (row.floor() as usize, col.floor() as usize);
    let (dr, dc) = (row - r as f64, col - c as f64);
    let mut sum = 0.0;
    for (y, wy) in [(r, 1.0 - dr), (r + 1, dr)] {
        for (x, wx) in [(c, 1.0 - dc), (c + 1, dc)] {
            let weight = wx * wy;
            if weight == 0.0 {
                continue;
            }
            let index = y.checked_mul(strip.cols as usize)?.checked_add(x)?;
            let value = f64::from(*map.get(index)?);
            if *strip.valid.get(index)? == 0 || !value.is_finite() {
                return None;
            }
            sum += weight * value;
        }
    }
    Some(sum)
}
