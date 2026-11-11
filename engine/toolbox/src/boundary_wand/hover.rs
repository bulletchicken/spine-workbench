//! Split here, merge here, or nothing and why, answered before the click.

use super::geometry::*;
use shapes::film::FilmKind;
use shapes::film_geometry::FilmGeometry;
use shapes::pixels::Px;
use shapes::toolbox::MissReason;
use shapes::vertebra::FilmMarks;

#[derive(Clone, Copy, Debug)]
pub(super) enum Hit {
    Split(usize),
    Merge(usize),
    Miss(MissReason),
}

pub(super) fn hit(
    marks: &FilmMarks,
    at: Px,
    geometry: Option<&FilmGeometry>,
    width: u32,
    height_px: u32,
) -> Hit {
    if matches!(marks.film, FilmKind::BendLeft | FilmKind::BendRight) {
        return Hit::Miss(MissReason::NotSupportedOnFilm);
    }
    if !at.x().is_finite()
        || !at.y().is_finite()
        || at.x() < -0.5
        || at.y() < -0.5
        || at.x() > f64::from(width) - 0.5
        || at.y() > f64::from(height_px) - 0.5
    {
        return Hit::Miss(MissReason::OutsideFilm);
    }
    let mut best: Option<(usize, f64)> = None;
    for (i, pair) in marks.vertebrae.windows(2).enumerate() {
        let (a, b) = (quad(&pair[0].marks), quad(&pair[1].marks));
        let h = 0.5 * (height(a) + height(b));
        let one_mm = geometry
            .and_then(|g| g.mm_per_px)
            .filter(|v| v.is_finite() && *v > 0.0)
            .map_or(0.0, |sx| {
                let sy = geometry.and_then(|g| g.mm_per_px_vertical).unwrap_or(sx);
                let direction = unit(sub(midpoint(b[0], b[1]), midpoint(a[0], a[1])))
                    .unwrap_or(Px::new(0.0, 1.0));
                1.0 / libm::hypot(direction.x() * sx, direction.y() * sy)
            });
        let tolerance = (0.12 * h).max(one_mm);
        let gap = [a[3], a[2], b[1], b[0]];
        let d = if convex(gap) && area(gap) * area(a) > 0.0 {
            if inside(at, gap) {
                0.0
            } else {
                boundary_distance(at, gap)
            }
        } else {
            distance_segment(at, a[3], a[2]).min(distance_segment(at, b[0], b[1]))
        };
        if d <= tolerance && best.is_none_or(|(_, old)| d < old) {
            best = Some((i, d));
        }
    }
    if let Some((i, _)) = best {
        return Hit::Merge(i);
    }
    let mut body: Option<(usize, f64)> = None;
    for (i, v) in marks.vertebrae.iter().enumerate() {
        let q = quad(&v.marks);
        if inside(at, q) {
            let centre = midpoint(midpoint(q[0], q[1]), midpoint(q[3], q[2]));
            let distance = length(sub(at, centre));
            if body.is_none_or(|(_, old)| distance < old) {
                body = Some((i, distance));
            }
        }
    }
    if let Some((i, _)) = body {
        let q = quad(&marks.vertebrae[i].marks);
        if distance_segment(at, q[0], q[1]) >= 0.25 * height(q)
            && distance_segment(at, q[3], q[2]) >= 0.25 * height(q)
        {
            return Hit::Split(i);
        }
        return Hit::Miss(MissReason::NearEndplate);
    }
    if let (Some(first), Some(last)) = (marks.vertebrae.first(), marks.vertebrae.last()) {
        let a = quad(&first.marks);
        let b = quad(&last.marks);
        if dot(
            sub(at, midpoint(a[0], a[1])),
            sub(midpoint(a[3], a[2]), midpoint(a[0], a[1])),
        ) < 0.0
            || dot(
                sub(at, midpoint(b[3], b[2])),
                sub(midpoint(b[3], b[2]), midpoint(b[0], b[1])),
            ) > 0.0
        {
            return Hit::Miss(MissReason::OutsideChain);
        }
    }
    Hit::Miss(MissReason::OffSpine)
}
