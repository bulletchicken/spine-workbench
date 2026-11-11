//! A contour becomes four corners through a minimum-area rectangle and
//! two rounds of robust side fits. Pinned corners constrain both adjacent
//! lines exactly. Pedicles follow the quad's bilinear coordinates.
//!
//! Tukey reweighting uses the usual 4.685 cutoff and MAD scale (Beaton and
//! Tukey, Technometrics 16(2), 1974); the quarter-working-pixel scale floor
//! keeps perfectly straight contours well defined.

use super::geometry::{
    add, area, bilinear, cross, distance, dot, height, scale, segment_distance, sub, valid_quad,
};
use shapes::errors::{EngineError, ErrorCode};
use shapes::marks::{Mark, MarkSource, PedicleMark};
use shapes::pixels::Px;
use shapes::vertebra::{VertebraMarks, VertebraOnFilm};

/// Refit in film pixels; `factor` is the crop's film-pixels-per-cell step.
pub(super) fn fit(contour: &[Px], old: &[Mark; 4], factor: f64) -> Result<[Px; 4], EngineError> {
    let before = old.map(|mark| mark.at);
    let orientation = area(&before);
    if contour.len() < 24
        || !contour.iter().all(|p| p.is_finite())
        || !factor.is_finite()
        || factor <= 0.0
        || !valid_quad(&before, orientation)
    {
        return Err(failed());
    }
    let start = minimum_rectangle(contour, &before).ok_or_else(failed)?;
    let mut fitted = start;
    for _ in 0..2 {
        let groups = side_points(contour, &fitted);
        let lines = (0..4)
            .map(|side| line_fit(&groups[side], old[side], old[(side + 1) % 4], factor))
            .collect::<Option<Vec<_>>>();
        let Some(lines) = lines else {
            fitted = start;
            break;
        };
        let corners = (0..4)
            .map(|i| {
                intersection(lines[(i + 3) % 4], lines[i])
                    .map(|point| if old[i].pinned { old[i].at } else { point })
            })
            .collect::<Option<Vec<_>>>();
        let Some(corners) = corners else {
            fitted = start;
            break;
        };
        fitted.copy_from_slice(&corners);
        if guards(contour, &fitted, orientation, height(&before)) {
            // The second assignment uses the newly fitted side lines.
            continue;
        }
        fitted = start;
        break;
    }
    restore_pins(&mut fitted, old);
    if guards(contour, &fitted, orientation, height(&before)) {
        return Ok(fitted);
    }
    let mut fallback = start;
    restore_pins(&mut fallback, old);
    if guards(contour, &fallback, orientation, height(&before)) {
        Ok(fallback)
    } else {
        Err(failed())
    }
}

fn failed() -> EngineError {
    EngineError::new(
        ErrorCode::ToolRefitFailed,
        "the region could not be fitted with four corners",
    )
}

fn restore_pins(quad: &mut [Px; 4], old: &[Mark; 4]) {
    for (point, mark) in quad.iter_mut().zip(old) {
        if mark.pinned {
            *point = mark.at;
        }
    }
}

fn guards(contour: &[Px], quad: &[Px; 4], orientation: f64, old_height: f64) -> bool {
    valid_quad(quad, orientation)
        && quad.iter().all(|corner| {
            contour
                .iter()
                .any(|p| distance(*corner, *p) <= 0.5 * old_height)
        })
        && side_points(contour, quad)
            .iter()
            .all(|points| points.len() >= 6)
}

fn side_points(contour: &[Px], quad: &[Px; 4]) -> [Vec<Px>; 4] {
    let mut groups: [Vec<Px>; 4] = std::array::from_fn(|_| Vec::new());
    for &point in contour {
        let side = (0..4)
            .min_by(|&a, &b| {
                segment_distance(point, quad[a], quad[(a + 1) % 4])
                    .total_cmp(&segment_distance(point, quad[b], quad[(b + 1) % 4]))
                    .then(a.cmp(&b))
            })
            .unwrap_or(0);
        let direction = sub(quad[(side + 1) % 4], quad[side]);
        let projection = dot(sub(point, quad[side]), direction) / dot(direction, direction);
        if (0.15..=0.85).contains(&projection) {
            groups[side].push(point);
        }
    }
    groups
}

#[derive(Clone, Copy)]
struct Line {
    origin: Px,
    direction: Px,
}

fn line_fit(points: &[Px], a: Mark, b: Mark, factor: f64) -> Option<Line> {
    if points.len() < 6 {
        return None;
    }
    if a.pinned && b.pinned {
        let length = distance(a.at, b.at);
        return (length > 0.0).then(|| Line {
            origin: a.at,
            direction: scale(sub(b.at, a.at), 1.0 / length),
        });
    }
    let anchor = if a.pinned {
        Some(a.at)
    } else if b.pinned {
        Some(b.at)
    } else {
        None
    };
    let mut weights = vec![1.0; points.len()];
    let mut line = None;
    for iteration in 0..5 {
        let total: f64 = weights.iter().sum();
        if total <= 1e-12 {
            return None;
        }
        let origin = anchor.unwrap_or_else(|| {
            scale(
                points
                    .iter()
                    .zip(&weights)
                    .fold(Px::new(0.0, 0.0), |sum, (&point, &weight)| {
                        add(sum, scale(point, weight))
                    }),
                1.0 / total,
            )
        });
        let (mut xx, mut xy, mut yy) = (0.0, 0.0, 0.0);
        for (&point, &weight) in points.iter().zip(&weights) {
            let offset = sub(point, origin);
            xx += weight * offset.x() * offset.x();
            xy += weight * offset.x() * offset.y();
            yy += weight * offset.y() * offset.y();
        }
        if xx + yy <= 1e-12 {
            return None;
        }
        let angle = 0.5 * libm::atan2(2.0 * xy, xx - yy);
        let fitted = Line {
            origin,
            direction: Px::new(libm::cos(angle), libm::sin(angle)),
        };
        line = Some(fitted);
        if iteration == 4 {
            break;
        }
        let residuals: Vec<f64> = points
            .iter()
            .map(|&point| cross(fitted.direction, sub(point, origin)))
            .collect();
        let centre = median(residuals.clone());
        let mad = median(
            residuals
                .iter()
                .map(|value| (value - centre).abs())
                .collect(),
        );
        let cutoff = 4.685 * (1.4826 * mad).max(0.25 * factor);
        for (weight, residual) in weights.iter_mut().zip(residuals) {
            let ratio = residual / cutoff;
            *weight = if ratio.abs() < 1.0 {
                let remaining = 1.0 - ratio * ratio;
                remaining * remaining
            } else {
                0.0
            };
        }
    }
    line
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[middle - 1] + values[middle]) * 0.5
    } else {
        values[middle]
    }
}

fn intersection(a: Line, b: Line) -> Option<Px> {
    let determinant = cross(a.direction, b.direction);
    if determinant.abs() < 0.2 {
        return None;
    }
    Some(add(
        a.origin,
        scale(
            a.direction,
            cross(sub(b.origin, a.origin), b.direction) / determinant,
        ),
    ))
}

fn minimum_rectangle(contour: &[Px], old: &[Px; 4]) -> Option<[Px; 4]> {
    let hull = convex_hull(contour);
    if hull.len() < 3 {
        return None;
    }
    let mut best = None;
    let mut best_area = f64::INFINITY;
    let mut best_distance = f64::INFINITY;
    for i in 0..hull.len() {
        let edge = sub(hull[(i + 1) % hull.len()], hull[i]);
        let along = scale(edge, 1.0 / dot(edge, edge).sqrt());
        let across = Px::new(-along.y(), along.x());
        let (mut lo_x, mut lo_y, mut hi_x, mut hi_y) = (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        );
        for &point in &hull {
            let x = dot(point, along);
            let y = dot(point, across);
            lo_x = lo_x.min(x);
            hi_x = hi_x.max(x);
            lo_y = lo_y.min(y);
            hi_y = hi_y.max(y);
        }
        let rectangle_area = (hi_x - lo_x) * (hi_y - lo_y);
        let point = |x, y| add(scale(along, x), scale(across, y));
        let mut rectangle = [
            point(lo_x, lo_y),
            point(hi_x, lo_y),
            point(hi_x, hi_y),
            point(lo_x, hi_y),
        ];
        if area(&rectangle) * area(old) < 0.0 {
            rectangle.reverse();
        }
        for shift in 0..4 {
            let candidate = std::array::from_fn(|j| rectangle[(j + shift) % 4]);
            let displacement = candidate
                .iter()
                .zip(old)
                .map(|(&a, &b)| {
                    let d = sub(a, b);
                    dot(d, d)
                })
                .sum::<f64>();
            if rectangle_area < best_area
                || (rectangle_area == best_area && displacement < best_distance)
            {
                best_area = rectangle_area;
                best_distance = displacement;
                best = Some(candidate);
            }
        }
    }
    best
}

fn convex_hull(contour: &[Px]) -> Vec<Px> {
    let mut points = contour.to_vec();
    points.sort_by(|a, b| a.x().total_cmp(&b.x()).then(a.y().total_cmp(&b.y())));
    points.dedup();
    if points.len() < 3 {
        return points;
    }
    let mut hull: Vec<Px> = Vec::new();
    for &point in &points {
        while hull.len() >= 2
            && cross(
                sub(hull[hull.len() - 1], hull[hull.len() - 2]),
                sub(point, hull[hull.len() - 1]),
            ) <= 0.0
        {
            hull.pop();
        }
        hull.push(point);
    }
    let lower = hull.len();
    for &point in points.iter().rev().skip(1) {
        while hull.len() > lower
            && cross(
                sub(hull[hull.len() - 1], hull[hull.len() - 2]),
                sub(point, hull[hull.len() - 1]),
            ) <= 0.0
        {
            hull.pop();
        }
        hull.push(point);
    }
    hull.pop();
    hull
}

/// Write refitted corners and carry visible, unpinned pedicles with them.
pub(super) fn apply(vertebra: &mut VertebraOnFilm, new: [Px; 4]) {
    let old = vertebra.marks.corners();
    let before = old.map(|mark| mark.at);
    let mut after = new;
    restore_pins(&mut after, &old);
    let corners = std::array::from_fn(|i| {
        if old[i].pinned {
            old[i]
        } else {
            Mark {
                at: after[i],
                source: MarkSource::RegionWand,
                pinned: false,
                inferred: false,
            }
        }
    });
    let [a, b, c, d] = corners;
    match &mut vertebra.marks {
        VertebraMarks::Front { points } => {
            points.sup_l = a;
            points.sup_r = b;
            points.inf_r = c;
            points.inf_l = d;
            if !same_quad(&before, &after) {
                move_pedicle(&mut points.ped_l, &before, &after);
                move_pedicle(&mut points.ped_r, &before, &after);
            }
        }
        VertebraMarks::Side { points } => {
            points.sup_a = a;
            points.sup_p = b;
            points.inf_p = c;
            points.inf_a = d;
        }
        VertebraMarks::Bending { lines } => {
            lines.sup = [a, b];
            lines.inf = [d, c];
        }
    }
}

fn same_quad(a: &[Px; 4], b: &[Px; 4]) -> bool {
    a.iter()
        .zip(b)
        .all(|(a, b)| a.x().to_bits() == b.x().to_bits() && a.y().to_bits() == b.y().to_bits())
}

fn move_pedicle(pedicle: &mut PedicleMark, old: &[Px; 4], new: &[Px; 4]) {
    if let PedicleMark::Visible {
        at,
        source,
        pinned: false,
        inferred,
        ring_score,
    } = pedicle
    {
        if let Some((u, v)) = inverse_bilinear(old, *at) {
            *at = bilinear(new, u, v);
        }
        *source = MarkSource::RegionWand;
        *inferred = true;
        *ring_score = None;
    }
}

/// Solve p = q0 + b*u + c*v + d*u*v by eliminating v. The stable
/// quadratic form avoids cancellation for nearly parallelogram quads.
fn inverse_bilinear(quad: &[Px; 4], point: Px) -> Option<(f64, f64)> {
    let b = sub(quad[1], quad[0]);
    let c = sub(quad[3], quad[0]);
    let d = add(sub(quad[0], quad[1]), sub(quad[2], quad[3]));
    let p = sub(point, quad[0]);
    let aa = cross(b, d);
    let bb = cross(b, c) - cross(p, d);
    let cc = -cross(p, c);
    let magnitude = aa.abs().max(bb.abs()).max(cc.abs()).max(1.0);
    let mut roots = Vec::with_capacity(2);
    if aa.abs() <= 1e-12 * magnitude {
        if bb.abs() <= 1e-12 * magnitude {
            return None;
        }
        roots.push(-cc / bb);
    } else {
        let discriminant = bb * bb - 4.0 * aa * cc;
        if discriminant < 0.0 {
            return None;
        }
        let q = -0.5 * (bb + discriminant.sqrt().copysign(bb));
        if q == 0.0 {
            roots.push(-bb / (2.0 * aa));
        } else {
            roots.extend([q / aa, cc / q]);
        }
    }
    roots.sort_by(f64::total_cmp);
    roots
        .into_iter()
        .filter_map(|u| {
            let down = add(c, scale(d, u));
            let length = dot(down, down);
            if length <= 1e-24 {
                return None;
            }
            let v = dot(sub(p, scale(b, u)), down) / length;
            if (-0.5..=1.5).contains(&u) && (-0.5..=1.5).contains(&v) {
                Some((u, v, distance(bilinear(quad, u, v), point)))
            } else {
                None
            }
        })
        .min_by(|a, b| a.2.total_cmp(&b.2))
        .map(|(u, v, _)| (u, v))
}

#[cfg(test)]
mod tests {
    use super::*;
    use shapes::confidence::{Confidence, ConfidenceBand};
    use shapes::levels::Level;
    use shapes::marks::FrontMarks;

    fn contour(quad: [Px; 4]) -> Vec<Px> {
        // 40 equally spaced points on each analytically defined side.
        (0..4)
            .flat_map(|side| {
                (0..40).map(move |i| {
                    let t = f64::from(i) / 40.0;
                    add(scale(quad[side], 1.0 - t), scale(quad[(side + 1) % 4], t))
                })
            })
            .collect()
    }

    #[test]
    fn robust_sides_recover_a_known_trapezoid() {
        let truth = [
            Px::new(10.0, 10.0),
            Px::new(50.0, 14.0),
            Px::new(54.0, 50.0),
            Px::new(6.0, 46.0),
        ];
        let old = [
            Px::new(8.0, 8.0),
            Px::new(52.0, 8.0),
            Px::new(52.0, 52.0),
            Px::new(8.0, 52.0),
        ]
        .map(Mark::auto);
        let result = fit(&contour(truth), &old, 1.0).expect("four supported lines");
        for (actual, expected) in result.iter().zip(truth) {
            // Exact straight sides; 1e-8 px allows only eigensolve/intersection rounding.
            assert!(
                distance(*actual, expected) < 1e-8,
                "{actual:?} versus {expected:?}"
            );
        }
    }

    #[test]
    fn isolated_outliers_do_not_pull_a_straight_side() {
        let truth = [
            Px::new(0.0, 0.0),
            Px::new(40.0, 0.0),
            Px::new(40.0, 40.0),
            Px::new(0.0, 40.0),
        ];
        let mut points = contour(truth);
        points.extend([Px::new(19.0, -3.0), Px::new(21.0, -3.0)]);
        let result = fit(&points, &truth.map(Mark::auto), 1.0).expect("outliers rejected");
        // All genuine side samples are exact; rounding is the only tolerated error.
        for (actual, expected) in result.iter().zip(truth) {
            assert!(distance(*actual, expected) < 1e-8);
        }
    }

    #[test]
    fn a_pinned_corner_is_bitwise_unchanged_and_orientation_is_kept() {
        let truth = [
            Px::new(50.0, 10.0),
            Px::new(10.0, 10.0),
            Px::new(10.0, 50.0),
            Px::new(50.0, 50.0),
        ];
        let mut old = truth.map(Mark::auto);
        old[0].pinned = true;
        old[0].source = MarkSource::Pin;
        old[1].at = Px::new(8.0, 8.0);
        let result = fit(&contour(truth), &old, 1.0).expect("pinned fit");
        assert_eq!(result[0].x().to_bits(), old[0].at.x().to_bits());
        assert_eq!(result[0].y().to_bits(), old[0].at.y().to_bits());
        assert!(area(&result) < 0.0);
    }

    #[test]
    fn an_unsupported_side_is_refused() {
        let quad = [
            Px::new(0.0, 0.0),
            Px::new(40.0, 0.0),
            Px::new(40.0, 40.0),
            Px::new(0.0, 40.0),
        ];
        let points: Vec<Px> = (0..30).map(|i| Px::new(f64::from(i), 0.0)).collect();
        assert_eq!(
            fit(&points, &quad.map(Mark::auto), 1.0).unwrap_err().code,
            ErrorCode::ToolRefitFailed
        );
    }

    fn front(quad: [Px; 4], ped_l: PedicleMark, ped_r: PedicleMark) -> VertebraOnFilm {
        let [sup_l, sup_r, inf_r, inf_l] = quad.map(Mark::auto);
        VertebraOnFilm {
            id: 1,
            level: Level::L1,
            marks: VertebraMarks::Front {
                points: FrontMarks {
                    sup_l,
                    sup_r,
                    inf_l,
                    inf_r,
                    ped_l,
                    ped_r,
                },
            },
            confidence: Confidence {
                score: 0.9,
                band: ConfidenceBand::High,
                reasons: vec![],
                calibrated: false,
                endplate_spread_px: None,
                margin: None,
            },
            outline: None,
        }
    }

    fn pedicle(at: Px, pinned: bool) -> PedicleMark {
        PedicleMark::Visible {
            at,
            source: MarkSource::Auto,
            pinned,
            inferred: false,
            ring_score: Some(0.9),
        }
    }

    #[test]
    fn nonlinear_bilinear_coordinates_follow_the_changed_quad() {
        let old = [
            Px::new(0.0, 0.0),
            Px::new(20.0, 0.0),
            Px::new(24.0, 40.0),
            Px::new(0.0, 40.0),
        ];
        let new = [
            Px::new(10.0, 5.0),
            Px::new(40.0, 5.0),
            Px::new(48.0, 65.0),
            Px::new(10.0, 65.0),
        ];
        // (u,v)=(1/4,3/5): old x=(20+4*3/5)/4=5.6, y=24.
        // New x=10+(30+8*3/5)/4=18.7, y=5+60*3/5=41.
        let mut body = front(
            old,
            pedicle(Px::new(5.6, 24.0), false),
            pedicle(Px::new(18.0, 12.0), true),
        );
        apply(&mut body, new);
        let VertebraMarks::Front { points } = body.marks else {
            unreachable!()
        };
        let moved = points.ped_l.mark().unwrap();
        assert!(distance(moved.at, Px::new(18.7, 41.0)) < 1e-10);
        assert!(moved.inferred);
        assert_eq!(points.ped_r, pedicle(Px::new(18.0, 12.0), true));
    }

    #[test]
    fn quadratic_inverse_selects_the_root_inside_the_quad() {
        let quad = [
            Px::new(0.0, 0.0),
            Px::new(20.0, 0.0),
            Px::new(24.0, 44.0),
            Px::new(0.0, 40.0),
        ];
        // The cross term is (4,4): at u=1/4,v=3/5, p=(5.6,24.6).
        let (u, v) = inverse_bilinear(&quad, Px::new(5.6, 24.6)).expect("valid root");
        assert!((u - 0.25).abs() < 1e-12);
        assert!((v - 0.6).abs() < 1e-12);
        assert!(inverse_bilinear(&quad, Px::new(1000.0, 1000.0)).is_none());
    }

    #[test]
    fn unchanged_quads_and_invisible_pedicles_stay_unchanged() {
        let quad = [
            Px::new(0.0, 0.0),
            Px::new(20.0, 0.0),
            Px::new(20.0, 40.0),
            Px::new(0.0, 40.0),
        ];
        let visible = pedicle(Px::new(4.0, 12.0), false);
        let hidden = PedicleMark::NotVisible {
            source: MarkSource::Hand,
        };
        let mut body = front(quad, visible, hidden);
        apply(&mut body, quad);
        let VertebraMarks::Front { points } = body.marks else {
            unreachable!()
        };
        assert_eq!(points.ped_l, visible);
        assert_eq!(points.ped_r, hidden);
    }
}
