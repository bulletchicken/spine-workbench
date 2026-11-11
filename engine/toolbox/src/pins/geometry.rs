//! Geometric guards for a pin's local fit.

use shapes::pixels::Px;

pub(super) fn distance(a: Px, b: Px) -> f64 {
    libm::hypot(a.x() - b.x(), a.y() - b.y())
}

pub(super) fn height(quad: [Px; 4]) -> f64 {
    0.5 * (distance(quad[0], quad[3]) + distance(quad[1], quad[2]))
}

fn turn(a: Px, b: Px, c: Px) -> f64 {
    (b.x() - a.x()) * (c.y() - b.y()) - (b.y() - a.y()) * (c.x() - b.x())
}

/// Strictly convex, same orientation, with angles in [30°, 150°].
pub(super) fn valid(quad: [Px; 4], old: [Px; 4]) -> bool {
    let sign = turn(old[0], old[1], old[2]);
    if !sign.is_finite() || sign == 0.0 || !quad.iter().all(|p| p.is_finite()) {
        return false;
    }
    (0..4).all(|i| {
        let (a, b, c) = (quad[(i + 3) % 4], quad[i], quad[(i + 1) % 4]);
        let dot = (a.x() - b.x()) * (c.x() - b.x()) + (a.y() - b.y()) * (c.y() - b.y());
        let lengths = distance(a, b) * distance(b, c);
        turn(a, b, c) * sign > 0.0
            && lengths > 1e-12
            && dot.abs() <= 0.866_025_403_784_438_6 * lengths
    })
}
