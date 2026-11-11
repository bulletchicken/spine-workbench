//! Exact coordinates for the frozen strip layout. Between two rows the
//! centre and the across vector both interpolate linearly. This makes each
//! cell a bilinear patch; the inverse solves that patch, not a different
//! smooth curve fitted through it.

use crate::handoffs::{Strip, StripRow};
use shapes::pixels::Px;

/// The original film pixel at a fractional strip row and column. Folded
/// locations and coordinates outside the strip have no map.
pub fn strip_to_film(strip: &Strip, row: f64, column: f64) -> Option<Px> {
    let layout = layout_at(strip, row)?;
    if !column.is_finite() || column < 0.0 || column > f64::from(strip.cols.checked_sub(1)?) {
        return None;
    }
    let offset = column - f64::from(strip.cols - 1) / 2.0;
    if !unfolded(layout.curvature, offset * strip.col_step) {
        return None;
    }
    Some(add(layout.centre, scale(layout.across, offset)))
}

/// Invert every intersected bilinear row patch. If a severely curved strip
/// covers the same film point twice, choose the solution nearest its centre
/// line, breaking ties toward the head end. The calculation uses only the
/// frozen layout, so it also works after a strip has crossed workers.
pub fn film_to_strip(strip: &Strip, point: Px) -> Option<(f64, f64)> {
    if !point.x().is_finite()
        || !point.y().is_finite()
        || strip.layout.len() != strip.rows as usize
        || strip.rows < 2
        || strip.cols == 0
        || !strip.col_step.is_finite()
        || strip.col_step <= 0.0
    {
        return None;
    }
    let middle = f64::from(strip.cols - 1) / 2.0;
    let mut best: Option<(f64, f64, f64)> = None;
    for (row, pair) in strip.layout.windows(2).enumerate() {
        let (a, b) = (pair[0], pair[1]);
        let delta_centre = subtract(b.centre, a.centre);
        let delta_across = subtract(b.across, a.across);
        let from_centre = subtract(point, a.centre);
        let quadratic = -cross(delta_centre, delta_across);
        let linear = cross(from_centre, delta_across) - cross(delta_centre, a.across);
        let constant = cross(from_centre, a.across);
        for fraction in roots(quadratic, linear, constant).into_iter().flatten() {
            if !fraction.is_finite() || !(-1e-10..=1.0 + 1e-10).contains(&fraction) {
                continue;
            }
            let fraction = fraction.clamp(0.0, 1.0);
            let centre = add(a.centre, scale(delta_centre, fraction));
            let across = add(a.across, scale(delta_across, fraction));
            let offset = dot(subtract(point, centre), across) / dot(across, across);
            if !offset.is_finite() || offset < -middle - 1e-9 || offset > middle + 1e-9 {
                continue;
            }
            let curvature = a.curvature + fraction * (b.curvature - a.curvature);
            if !unfolded(curvature, offset * strip.col_step) {
                continue;
            }
            let reproduced = add(centre, scale(across, offset));
            let error = subtract(reproduced, point);
            if dot(error, error) > 1e-12 {
                continue;
            }
            let distance = offset.abs();
            if best.is_none_or(|(_, _, previous)| distance < previous) {
                best = Some((
                    row as f64 + fraction,
                    (offset + middle).clamp(0.0, 2.0 * middle),
                    distance,
                ));
            }
        }
    }
    best.map(|(row, column, _)| (row, column))
}

/// Physical area multiplier of the *stored* piecewise-linear strip map.
/// Unlike `1 - curvature * offset`, this includes its sub-row interpolation
/// exactly. At whole rows the derivative of the following patch is used
/// (the preceding patch at the final row).
pub fn area_scale(strip: &Strip, row: f64, column: f64) -> Option<f64> {
    let layout = layout_at(strip, row)?;
    strip_to_film(strip, row, column)?;
    if strip.rows < 2 {
        return None;
    }
    let first = (row.floor() as usize).min(strip.layout.len() - 2);
    let a = strip.layout[first];
    let b = strip.layout[first + 1];
    let offset = column - f64::from(strip.cols - 1) / 2.0;
    let derivative = add(
        subtract(b.centre, a.centre),
        scale(subtract(b.across, a.across), offset),
    );
    // At a whole row the two physical unit directions have a fixed area
    // in film pixels. Interpolating both directions would shrink that
    // reference on a curve and would incorrectly cancel part of the map's
    // true area change.
    let reference = cross(a.along, a.across).abs();
    (reference.is_finite() && reference > 0.0)
        .then_some(cross(derivative, layout.across).abs() / reference)
}

pub(crate) fn layout_at(strip: &Strip, row: f64) -> Option<StripRow> {
    if strip.layout.len() != strip.rows as usize
        || strip.rows == 0
        || !row.is_finite()
        || row < 0.0
        || row > f64::from(strip.rows - 1)
    {
        return None;
    }
    let first = row.floor() as usize;
    let second = (first + 1).min(strip.layout.len() - 1);
    let a = strip.layout[first];
    let b = strip.layout[second];
    let fraction = row - first as f64;
    let mix = |x, y| x + fraction * (y - x);
    let mix_px = |x: Px, y: Px| Px::new(mix(x.x(), y.x()), mix(x.y(), y.y()));
    Some(StripRow {
        centre: mix_px(a.centre, b.centre),
        across: mix_px(a.across, b.across),
        along: mix_px(a.along, b.along),
        curvature: mix(a.curvature, b.curvature),
        half_width: mix(a.half_width, b.half_width),
        z: mix(a.z, b.z),
    })
}

pub(crate) fn unfolded(curvature: f64, offset: f64) -> bool {
    // A rounded 0.8 / curvature is allowed to land on the 0.2 limit.
    curvature.is_finite() && offset.is_finite() && 1.0 - curvature * offset >= 0.2 - 1e-12
}

fn roots(a: f64, b: f64, c: f64) -> [Option<f64>; 2] {
    if a.abs() <= 1e-14 * b.abs().max(1.0) {
        return [if b.abs() > 1e-15 { Some(-c / b) } else { None }, None];
    }
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return [None, None];
    }
    let q = -0.5 * (b + discriminant.sqrt().copysign(b));
    if q == 0.0 {
        [Some(-b / (2.0 * a)), None]
    } else {
        [Some(q / a), Some(c / q)]
    }
}

pub(crate) fn add(a: Px, b: Px) -> Px {
    Px::new(a.x() + b.x(), a.y() + b.y())
}

pub(crate) fn subtract(a: Px, b: Px) -> Px {
    Px::new(a.x() - b.x(), a.y() - b.y())
}

pub(crate) fn scale(point: Px, scale: f64) -> Px {
    Px::new(point.x() * scale, point.y() * scale)
}

fn dot(a: Px, b: Px) -> f64 {
    a.x() * b.x() + a.y() * b.y()
}

fn cross(a: Px, b: Px) -> f64 {
    a.x() * b.y() - a.y() * b.x()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handoffs::StripEvidence;
    use shapes::film::FilmKind;

    fn circle() -> Strip {
        let layout: Vec<_> = (0..=400)
            .map(|i| {
                let angle = f64::from(i) * 0.5 / 80.0;
                // Circle curving right: C=(80(1-cos a),80 sin a),
                // T=(sin a,cos a), N=(cos a,-sin a), curvature=1/80.
                StripRow {
                    centre: Px::new(80.0 * (1.0 - libm::cos(angle)), 80.0 * libm::sin(angle)),
                    across: Px::new(0.5 * libm::cos(angle), -0.5 * libm::sin(angle)),
                    along: Px::new(0.5 * libm::sin(angle), 0.5 * libm::cos(angle)),
                    curvature: 1.0 / 80.0,
                    half_width: 20.0,
                    z: -0.5 * f64::from(i),
                }
            })
            .collect();
        Strip {
            film: FilmKind::Front,
            rows: layout.len() as u32,
            cols: 161,
            row_step: 0.5,
            col_step: 0.5,
            scale_assumed: false,
            values: vec![],
            valid: vec![],
            layout,
            image_right: None,
            polarity_hint: None,
            evidence: StripEvidence::default(),
        }
    }

    #[test]
    fn fractional_maps_invert_the_stored_bilinear_layout() {
        let strip = circle();
        for row in [0.0, 0.25, 37.7, 100.0, 284.3, 400.0] {
            for column in [0.0, 9.2, 80.0, 150.8, 160.0] {
                let film = strip_to_film(&strip, row, column).expect("unfolded");
                let back = film_to_strip(&strip, film).expect("invertible");
                // Quadratic solve in f64, well away from a fold: the spec's
                // tolerance is 10^-6 strip pixels.
                assert!((back.0 - row).abs() < 1e-6, "{row} -> {}", back.0);
                assert!((back.1 - column).abs() < 1e-6);
            }
        }
        // At the origin of this circle the normal is +film x.
        assert_eq!(strip_to_film(&strip, 0.0, 100.0), Some(Px::new(10.0, 0.0)));
        assert_eq!(strip_to_film(&strip, -1.0, 80.0), None);
    }

    #[test]
    fn area_matches_a_finite_difference_of_the_mapping() {
        let strip = circle();
        let (row, column, delta) = (100.25, 120.0, 1e-4);
        let dr = scale(
            subtract(
                strip_to_film(&strip, row + delta, column).unwrap(),
                strip_to_film(&strip, row - delta, column).unwrap(),
            ),
            0.5 / delta,
        );
        let dc = scale(
            subtract(
                strip_to_film(&strip, row, column + delta).unwrap(),
                strip_to_film(&strip, row, column - delta).unwrap(),
            ),
            0.5 / delta,
        );
        // Circle fixture film pixels are square millimetres; one strip
        // cell is 0.5 by 0.5 mm before its curvature changes the area.
        let numerical = cross(dr, dc).abs() / 0.25;
        assert!((area_scale(&strip, row, column).unwrap() - numerical).abs() < 1e-6);
    }

    #[test]
    fn inverse_rejects_a_point_beyond_either_end() {
        let strip = circle();
        assert_eq!(film_to_strip(&strip, Px::new(0.0, -10.0)), None);
    }
}
