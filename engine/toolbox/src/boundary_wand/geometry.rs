//! Film-space geometry for boundary hit tests and endplate cuts.

use shapes::marks::{Mark, MarkSource};
use shapes::pixels::Px;
use shapes::vertebra::VertebraMarks;

pub(super) fn add(a: Px, b: Px) -> Px {
    Px::new(a.x() + b.x(), a.y() + b.y())
}
pub(super) fn sub(a: Px, b: Px) -> Px {
    Px::new(a.x() - b.x(), a.y() - b.y())
}
pub(super) fn scale(a: Px, k: f64) -> Px {
    Px::new(a.x() * k, a.y() * k)
}
pub(super) fn dot(a: Px, b: Px) -> f64 {
    a.x() * b.x() + a.y() * b.y()
}
pub(super) fn cross(a: Px, b: Px) -> f64 {
    a.x() * b.y() - a.y() * b.x()
}
pub(super) fn length(a: Px) -> f64 {
    libm::hypot(a.x(), a.y())
}
pub(super) fn midpoint(a: Px, b: Px) -> Px {
    scale(add(a, b), 0.5)
}
pub(super) fn lerp(a: Px, b: Px, t: f64) -> Px {
    add(a, scale(sub(b, a), t))
}
pub(super) fn unit(a: Px) -> Option<Px> {
    let n = length(a);
    (n.is_finite() && n > 1e-9).then(|| scale(a, 1.0 / n))
}
pub(super) fn quad(m: &VertebraMarks) -> [Px; 4] {
    m.corners().map(|m| m.at)
}
pub(super) fn height(q: [Px; 4]) -> f64 {
    0.5 * (length(sub(q[3], q[0])) + length(sub(q[2], q[1])))
}
pub(super) fn width(q: [Px; 4]) -> f64 {
    0.5 * (length(sub(q[1], q[0])) + length(sub(q[2], q[3])))
}
pub(super) fn area(q: [Px; 4]) -> f64 {
    (0..4).map(|i| cross(q[i], q[(i + 1) % 4])).sum::<f64>() * 0.5
}
pub(super) fn distance_segment(p: Px, a: Px, b: Px) -> f64 {
    let ab = sub(b, a);
    let t = if dot(ab, ab) > 0.0 {
        (dot(sub(p, a), ab) / dot(ab, ab)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    length(sub(p, lerp(a, b, t)))
}
pub(super) fn boundary_distance(p: Px, q: [Px; 4]) -> f64 {
    (0..4)
        .map(|i| distance_segment(p, q[i], q[(i + 1) % 4]))
        .fold(f64::INFINITY, f64::min)
}
pub(super) fn inside(p: Px, q: [Px; 4]) -> bool {
    if boundary_distance(p, q) <= 1e-9 {
        return true;
    }
    let mut hit = false;
    for i in 0..4 {
        let (a, b) = (q[i], q[(i + 1) % 4]);
        if (a.y() > p.y()) != (b.y() > p.y())
            && p.x() < (b.x() - a.x()) * (p.y() - a.y()) / (b.y() - a.y()) + a.x()
        {
            hit = !hit;
        }
    }
    hit
}
pub(super) fn convex(q: [Px; 4]) -> bool {
    if q.iter().any(|p| !p.x().is_finite() || !p.y().is_finite()) {
        return false;
    }
    let sign = area(q).signum();
    sign != 0.0
        && (0..4).all(|i| {
            cross(
                sub(q[(i + 1) % 4], q[i]),
                sub(q[(i + 2) % 4], q[(i + 1) % 4]),
            ) * sign
                > 1e-9
        })
}
pub(super) fn valid(q: [Px; 4]) -> bool {
    convex(q)
        && (0..4).all(|i| {
            let a = sub(q[(i + 3) % 4], q[i]);
            let b = sub(q[(i + 1) % 4], q[i]);
            // cos(30 degrees) bounds both the acute and obtuse guards.
            dot(a, b).abs() <= 0.8660254037844386 * length(a) * length(b)
        })
}
pub(super) fn put_corners(m: &mut VertebraMarks, q: [Mark; 4]) {
    match m {
        VertebraMarks::Front { points } => {
            points.sup_l = q[0];
            points.sup_r = q[1];
            points.inf_r = q[2];
            points.inf_l = q[3];
        }
        VertebraMarks::Side { points } => {
            points.sup_a = q[0];
            points.sup_p = q[1];
            points.inf_p = q[2];
            points.inf_a = q[3];
        }
        VertebraMarks::Bending { lines } => {
            lines.sup = [q[0], q[1]];
            lines.inf = [q[3], q[2]];
        }
    }
}
pub(super) fn placed(at: Px) -> Mark {
    Mark {
        at,
        source: MarkSource::BoundaryWand,
        pinned: false,
        inferred: false,
    }
}
pub(super) fn bilinear(q: [Px; 4], u: f64, v: f64) -> Px {
    lerp(lerp(q[0], q[1], u), lerp(q[3], q[2], u), v)
}

pub(super) fn cut(q: [Px; 4], s: f64) -> Option<[Px; 2]> {
    let top = midpoint(q[0], q[1]);
    let bottom = midpoint(q[3], q[2]);
    let axis = sub(bottom, top);
    let h = length(axis);
    let u = unit(axis)?;
    let direction = unit(lerp(unit(sub(q[1], q[0]))?, unit(sub(q[2], q[3]))?, s / h))?;
    let point = add(top, scale(u, s));
    let intersection = |a, b| {
        let side = sub(b, a);
        let d = cross(direction, side);
        (d.abs() > 1e-9).then(|| add(a, scale(side, cross(sub(a, point), direction) / d)))
    };
    Some([intersection(q[0], q[3])?, intersection(q[1], q[2])?])
}
