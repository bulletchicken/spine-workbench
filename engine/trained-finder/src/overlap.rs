//! How much two outlines overlap.
//!
//! The overlap of two quads is the area they share divided by the area
//! either covers. Each quad is first replaced by its convex hull, so a
//! corner order that crosses over itself still has an area, and the shared
//! part is the first hull clipped to the second. Every step is double
//! precision in the reference's order, so a near tie at one half comes out
//! the same way it did there.

use crate::outline::{Point, Quad};

/// The area two quads share divided by the area either covers, 0 to 1; 0
/// when either has no area.
pub fn overlap(a: &Quad, b: &Quad) -> f64 {
    let (first, second) = (hull(a), hull(b));
    if first.len() < 3 || second.len() < 3 {
        return 0.0;
    }
    let shared = area(&clip(&first, &second));
    let either = area(&first) + area(&second) - shared;
    if either > 0.0 { shared / either } else { 0.0 }
}

/// How far `b` turns left of `a` as seen from `origin`: positive for a
/// left turn, zero when the three are in line.
fn cross(origin: Point, a: Point, b: Point) -> f64 {
    (a[0] - origin[0]) * (b[1] - origin[1]) - (a[1] - origin[1]) * (b[0] - origin[0])
}

/// The convex hull, going round anticlockwise in picture terms from the
/// leftmost point (Andrew's monotone chain). Repeated points are dropped
/// first and points in line are dropped as it goes; fewer than three
/// distinct points come back as they are.
fn hull(quad: &Quad) -> Vec<Point> {
    let mut points = quad.to_vec();
    points.sort_by(|p, q| p[0].total_cmp(&q[0]).then(p[1].total_cmp(&q[1])));
    points.dedup();
    if points.len() < 3 {
        return points;
    }
    let mut lower: Vec<Point> = Vec::with_capacity(points.len());
    for &point in &points {
        while lower.len() >= 2
            && cross(lower[lower.len() - 2], lower[lower.len() - 1], point) <= 0.0
        {
            lower.pop();
        }
        lower.push(point);
    }
    let mut upper: Vec<Point> = Vec::with_capacity(points.len());
    for &point in points.iter().rev() {
        while upper.len() >= 2
            && cross(upper[upper.len() - 2], upper[upper.len() - 1], point) <= 0.0
        {
            upper.pop();
        }
        upper.push(point);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

/// The shoelace area; 0 for fewer than three points.
fn area(polygon: &[Point]) -> f64 {
    if polygon.len() < 3 {
        return 0.0;
    }
    let mut twice = 0.0;
    for (i, point) in polygon.iter().enumerate() {
        let next = polygon[(i + 1) % polygon.len()];
        twice += point[0] * next[1] - next[0] * point[1];
    }
    twice.abs() / 2.0
}

/// `subject` cut down to the part inside the convex `clipper`
/// (Sutherland and Hodgman): each edge of the clipper in turn keeps what
/// lies on its inner side, adding a point where an edge of what is left
/// crosses it. Nothing comes back once fewer than three points are left.
fn clip(subject: &[Point], clipper: &[Point]) -> Vec<Point> {
    let mut kept = subject.to_vec();
    for (i, &a) in clipper.iter().enumerate() {
        let b = clipper[(i + 1) % clipper.len()];
        let side = |p: Point| (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
        let source = std::mem::take(&mut kept);
        for (j, &p) in source.iter().enumerate() {
            let q = source[(j + 1) % source.len()];
            let (at_p, at_q) = (side(p), side(q));
            if at_p >= 0.0 {
                kept.push(p);
            }
            if (at_p >= 0.0) != (at_q >= 0.0) {
                let t = at_p / (at_p - at_q);
                kept.push([p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])]);
            }
        }
        if kept.len() < 3 {
            return Vec::new();
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(left: f64, top: f64, side: f64) -> Quad {
        [
            [left, top],
            [left + side, top],
            [left + side, top + side],
            [left, top + side],
        ]
    }

    #[test]
    fn a_quad_overlaps_itself_wholly() {
        assert_eq!(overlap(&square(0.0, 0.0, 2.0), &square(0.0, 0.0, 2.0)), 1.0);
    }

    #[test]
    fn half_shifted_squares_share_a_third() {
        // Each square has area 4; they share a 1 by 2 strip.
        let shared = overlap(&square(0.0, 0.0, 2.0), &square(1.0, 0.0, 2.0));
        assert!((shared - 2.0 / 6.0).abs() < 1e-15);
    }

    #[test]
    fn apart_or_flat_quads_share_nothing() {
        assert_eq!(overlap(&square(0.0, 0.0, 1.0), &square(5.0, 5.0, 1.0)), 0.0);
        let flat = [[0.0, 0.0], [1.0, 0.0], [2.0, 0.0], [1.0, 0.0]];
        assert_eq!(overlap(&flat, &square(0.0, 0.0, 1.0)), 0.0);
        let point = [[0.5, 0.5]; 4];
        assert_eq!(overlap(&point, &square(0.0, 0.0, 1.0)), 0.0);
    }

    #[test]
    fn a_crossed_corner_order_is_read_as_its_hull() {
        // Upper left, upper right, lower left, lower right: a bow tie whose
        // hull is the whole square.
        let crossed = [[0.0, 0.0], [2.0, 0.0], [0.0, 2.0], [2.0, 2.0]];
        assert_eq!(overlap(&crossed, &square(0.0, 0.0, 2.0)), 1.0);
    }

    #[test]
    fn a_small_square_inside_a_big_one_overlaps_by_its_share_of_the_area() {
        let shared = overlap(&square(1.0, 1.0, 1.0), &square(0.0, 0.0, 4.0));
        assert_eq!(shared, 1.0 / 16.0);
        assert_eq!(
            overlap(&square(0.0, 0.0, 4.0), &square(1.0, 1.0, 1.0)),
            1.0 / 16.0
        );
    }

    #[test]
    fn the_hull_drops_points_in_line_and_repeats() {
        let quad = [[0.0, 0.0], [1.0, 0.0], [2.0, 0.0], [0.0, 0.0]];
        assert_eq!(hull(&quad), vec![[0.0, 0.0], [2.0, 0.0]]);
        let triangle = [[0.0, 0.0], [2.0, 0.0], [1.0, 0.0], [0.0, 2.0]];
        assert_eq!(hull(&triangle), vec![[0.0, 0.0], [2.0, 0.0], [0.0, 2.0]]);
    }
}
