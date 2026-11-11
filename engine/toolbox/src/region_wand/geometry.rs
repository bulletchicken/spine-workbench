//! Polygon geometry in original film pixels; no view transform is involved.

use shapes::pixels::Px;

pub(super) fn add(a: Px, b: Px) -> Px {
    Px::new(a.x() + b.x(), a.y() + b.y())
}
pub(super) fn sub(a: Px, b: Px) -> Px {
    Px::new(a.x() - b.x(), a.y() - b.y())
}
pub(super) fn scale(a: Px, s: f64) -> Px {
    Px::new(a.x() * s, a.y() * s)
}
pub(super) fn dot(a: Px, b: Px) -> f64 {
    a.x() * b.x() + a.y() * b.y()
}
pub(super) fn cross(a: Px, b: Px) -> f64 {
    a.x() * b.y() - a.y() * b.x()
}
pub(super) fn distance(a: Px, b: Px) -> f64 {
    let d = sub(a, b);
    dot(d, d).sqrt()
}
pub(super) fn height(q: &[Px; 4]) -> f64 {
    (distance(q[0], q[3]) + distance(q[1], q[2])) * 0.5
}
pub(super) fn width(q: &[Px; 4]) -> f64 {
    (distance(q[0], q[1]) + distance(q[3], q[2])) * 0.5
}
pub(super) fn centre(q: &[Px; 4]) -> Px {
    scale(q.iter().copied().fold(Px::new(0., 0.), add), 0.25)
}
pub(super) fn area(q: &[Px]) -> f64 {
    (0..q.len())
        .map(|i| cross(q[i], q[(i + 1) % q.len()]))
        .sum::<f64>()
        * 0.5
}
pub(super) fn segment_distance(p: Px, a: Px, b: Px) -> f64 {
    let v = sub(b, a);
    let t = (dot(sub(p, a), v) / dot(v, v).max(1e-30)).clamp(0., 1.);
    distance(p, add(a, scale(v, t)))
}
pub(super) fn boundary_distance(p: Px, q: &[Px; 4]) -> f64 {
    (0..4)
        .map(|i| segment_distance(p, q[i], q[(i + 1) % 4]))
        .fold(f64::INFINITY, f64::min)
}
/// Half-open scan conversion: top/left edges are inside, bottom/right outside.
pub(super) fn inside(p: Px, q: &[Px]) -> bool {
    let mut hit = false;
    for i in 0..q.len() {
        let (a, b) = (q[i], q[(i + 1) % q.len()]);
        if (a.y() > p.y()) != (b.y() > p.y())
            && p.x() < (b.x() - a.x()) * (p.y() - a.y()) / (b.y() - a.y()) + a.x()
        {
            hit = !hit;
        }
    }
    hit
}
pub(super) fn valid_quad(q: &[Px; 4], orientation: f64) -> bool {
    if !q.iter().all(|p| p.is_finite()) || area(q) * orientation <= 0. {
        return false;
    }
    (0..4).all(|i| {
        let a = sub(q[(i + 3) % 4], q[i]);
        let b = sub(q[(i + 1) % 4], q[i]);
        let lengths = (dot(a, a) * dot(b, b)).sqrt();
        lengths > 1e-12
            && cross(b, a) * orientation > 0.
            && (dot(a, b) / lengths).abs() <= libm::cos(std::f64::consts::PI / 6.)
    })
}
pub(super) fn bilinear(q: &[Px; 4], u: f64, v: f64) -> Px {
    add(
        scale(add(scale(q[0], 1. - u), scale(q[1], u)), 1. - v),
        scale(add(scale(q[3], 1. - u), scale(q[2], u)), v),
    )
}
/// Convex polygon clipping, used only for overlap warnings.
pub(super) fn overlap(a: &[Px; 4], b: &[Px; 4]) -> f64 {
    let orientation = area(b).signum();
    let mut polygon = a.to_vec();
    for i in 0..4 {
        let origin = b[i];
        let edge = sub(b[(i + 1) % 4], origin);
        let old = std::mem::take(&mut polygon);
        if old.is_empty() {
            break;
        }
        for j in 0..old.len() {
            let p = old[j];
            let q = old[(j + 1) % old.len()];
            let dp = cross(edge, sub(p, origin)) * orientation;
            let dq = cross(edge, sub(q, origin)) * orientation;
            if dp >= 0. {
                polygon.push(p);
            }
            if (dp >= 0.) != (dq >= 0.) {
                polygon.push(add(p, scale(sub(q, p), dp / (dp - dq))));
            }
        }
    }
    area(&polygon).abs()
}
