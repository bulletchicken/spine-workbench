//! The solids a film is painted from, and how far a ray travels inside each:
//! the line integral a parallel beam sees.
//!
//! Every length is exact for its solid, in closed form: a quadratic for
//! the curved surface, a linear bound for each flat face. No solid is
//! sampled or marched, so painting costs one small solve per pixel per
//! solid it covers.
//!
//! - [`Ellipsoid`]: any orientation; spheres and flattened plates too.
//! - [`Frustum`]: an elliptic cone (a cylinder when its taper is zero)
//!   whose cross-section keeps its shape, cut by any number of flat faces
//!   and optionally trimmed by an ellipsoid. Vertebral bodies, pedicles
//!   and the mitred pieces of a rib are frustums.
//! - [`Capsule`]: a cylinder with round ends, for processes and long bones.
//!
//! On a parallel film every pixel's ray has the same direction and starts
//! at a point that moves evenly with the pixel. So a solid is first
//! [prepared](Solid::prepare) for a film's [`RayGrid`]: everything that
//! depends only on the direction is worked out once, and what depends on
//! the start becomes a value plus a step per pixel across and down. Each
//! pixel then costs a few multiplications and one square root.

use nalgebra::{Matrix3, Vector3};

/// A line through the patient: `origin + t · direction`, `direction`
/// a unit vector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    pub origin: Vector3<f64>,
    pub direction: Vector3<f64>,
}

/// The rays of a film: pixel `(x, y)`'s ray starts at
/// `origin + x · across + y · down` and runs along `direction`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayGrid {
    pub origin: Vector3<f64>,
    pub across: Vector3<f64>,
    pub down: Vector3<f64>,
    pub direction: Vector3<f64>,
}

impl RayGrid {
    /// One ray alone, as pixel `(0, 0)`.
    pub fn single(ray: &Ray) -> Self {
        RayGrid {
            origin: ray.origin,
            across: Vector3::zeros(),
            down: Vector3::zeros(),
            direction: ray.direction,
        }
    }
}

type Triple = [f64; 3];

fn dot(a: &Triple, b: &Triple) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// A point that moves evenly with the pixel: `base + x · step_x + y ·
/// step_y`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Moving {
    base: Triple,
    step_x: Triple,
    step_y: Triple,
}

impl Moving {
    /// The start of each ray in the frame `rows · (p − centre)`, each row
    /// scaled by `scale`.
    fn local(
        grid: &RayGrid,
        centre: &Vector3<f64>,
        rows: &Matrix3<f64>,
        scale: &Vector3<f64>,
    ) -> Self {
        let map = |v: Vector3<f64>| -> Triple {
            let m = (rows * v).component_div(scale);
            [m.x, m.y, m.z]
        };
        Moving {
            base: map(grid.origin - centre),
            step_x: map(grid.across),
            step_y: map(grid.down),
        }
    }

    fn at(&self, x: f64, y: f64) -> Triple {
        [
            self.base[0] + x * self.step_x[0] + y * self.step_y[0],
            self.base[1] + x * self.step_x[1] + y * self.step_y[1],
            self.base[2] + x * self.step_x[2] + y * self.step_y[2],
        ]
    }
}

/// A direction in the same frame.
fn local_direction(grid: &RayGrid, rows: &Matrix3<f64>, scale: &Vector3<f64>) -> Triple {
    let m = (rows * grid.direction).component_div(scale);
    [m.x, m.y, m.z]
}

/// A stretch of a ray, `from ≤ t ≤ to`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Span {
    from: f64,
    to: f64,
}

impl Span {
    const ALL: Span = Span {
        from: f64::NEG_INFINITY,
        to: f64::INFINITY,
    };

    fn meet(self, other: Span) -> Option<Span> {
        let span = Span {
            from: self.from.max(other.from),
            to: self.to.min(other.to),
        };
        (span.from < span.to).then_some(span)
    }

    fn length(self) -> f64 {
        self.to - self.from
    }
}

/// The values of `t` where `a·t² + b·t + c ≤ 0`, as at most two spans.
fn quadratic_inside(a: f64, b: f64, c: f64) -> [Option<Span>; 2] {
    let scale = a.abs().max(b.abs()).max(c.abs());
    if scale == 0.0 {
        return [Some(Span::ALL), None];
    }
    if a.abs() <= 1e-12 * scale {
        // Linear: b·t + c ≤ 0.
        if b.abs() <= 1e-12 * scale {
            return [(c <= 0.0).then_some(Span::ALL), None];
        }
        let root = -c / b;
        let span = if b > 0.0 {
            Span {
                from: f64::NEG_INFINITY,
                to: root,
            }
        } else {
            Span {
                from: root,
                to: f64::INFINITY,
            }
        };
        return [Some(span), None];
    }
    let discriminant = b * b - 4.0 * a * c;
    if discriminant <= 0.0 {
        // No real roots: everywhere inside when the parabola opens down.
        return [(a < 0.0).then_some(Span::ALL), None];
    }
    // The stable pair of roots.
    let q = -0.5 * (b + b.signum() * discriminant.sqrt());
    let (r1, r2) = (q / a, c / q);
    let (low, high) = if r1 < r2 { (r1, r2) } else { (r2, r1) };
    if a > 0.0 {
        [
            Some(Span {
                from: low,
                to: high,
            }),
            None,
        ]
    } else {
        [
            Some(Span {
                from: f64::NEG_INFINITY,
                to: low,
            }),
            Some(Span {
                from: high,
                to: f64::INFINITY,
            }),
        ]
    }
}

/// A flat face: points with `normal · p ≤ offset` are inside.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HalfSpace {
    pub normal: Vector3<f64>,
    pub offset: f64,
}

impl HalfSpace {
    /// The half-space behind the plane through `point` facing `outward`.
    pub fn behind(point: Vector3<f64>, outward: Vector3<f64>) -> Self {
        HalfSpace {
            normal: outward,
            offset: outward.dot(&point),
        }
    }

    /// The same face moved inward by `depth`.
    pub fn moved_in(&self, depth: f64) -> Self {
        HalfSpace {
            normal: self.normal,
            offset: self.offset - depth * self.normal.norm(),
        }
    }

    fn prepare(&self, grid: &RayGrid) -> PreparedFace {
        PreparedFace {
            start: [
                self.normal.dot(&grid.origin) - self.offset,
                self.normal.dot(&grid.across),
                self.normal.dot(&grid.down),
            ],
            rate: self.normal.dot(&grid.direction),
        }
    }
}

/// A face prepared for a grid: how far inside the start is, moving with
/// the pixel, and how fast the ray leaves.
#[derive(Clone, Copy, Debug, PartialEq)]
struct PreparedFace {
    start: Triple,
    rate: f64,
}

impl PreparedFace {
    fn span(&self, x: f64, y: f64) -> Option<Span> {
        let start = self.start[0] + x * self.start[1] + y * self.start[2];
        if self.rate.abs() < 1e-15 {
            return (start <= 0.0).then_some(Span::ALL);
        }
        let root = -start / self.rate;
        Some(if self.rate > 0.0 {
            Span {
                from: f64::NEG_INFINITY,
                to: root,
            }
        } else {
            Span {
                from: root,
                to: f64::INFINITY,
            }
        })
    }
}

/// Narrows `bounds` by `span`, `None` once nothing is left.
fn narrowed(bounds: Span, span: Option<Span>) -> Option<Span> {
    span.and_then(|s| bounds.meet(s))
}

/// An ellipsoid: `centre`, its unit axes as the columns of `axes`, and its
/// half-lengths along them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ellipsoid {
    pub centre: Vector3<f64>,
    pub axes: Matrix3<f64>,
    pub radii: Vector3<f64>,
}

impl Ellipsoid {
    pub fn sphere(centre: Vector3<f64>, radius: f64) -> Self {
        Ellipsoid {
            centre,
            axes: Matrix3::identity(),
            radii: Vector3::repeat(radius),
        }
    }

    /// The same ellipsoid with every half-length shorter by `depth`.
    pub fn shrunk(&self, depth: f64) -> Self {
        Ellipsoid {
            radii: self.radii.map(|r| (r - depth).max(0.0)),
            ..*self
        }
    }

    fn prepare(&self, grid: &RayGrid) -> PreparedEllipsoid {
        let rows = self.axes.transpose();
        let direction = local_direction(grid, &rows, &self.radii);
        PreparedEllipsoid {
            start: Moving::local(grid, &self.centre, &rows, &self.radii),
            a: dot(&direction, &direction),
            direction,
        }
    }

    fn corners(&self) -> [Vector3<f64>; 8] {
        box_corners(&self.centre, &self.axes, &self.radii)
    }
}

/// An ellipsoid prepared for a grid, in the frame where it is the unit
/// sphere.
#[derive(Clone, Copy, Debug, PartialEq)]
struct PreparedEllipsoid {
    start: Moving,
    direction: Triple,
    a: f64,
}

impl PreparedEllipsoid {
    fn span(&self, x: f64, y: f64) -> Option<Span> {
        let o = self.start.at(x, y);
        let [inside, _] =
            quadratic_inside(self.a, 2.0 * dot(&o, &self.direction), dot(&o, &o) - 1.0);
        inside
    }
}

/// The corners of the box about `centre` with unit axes the columns of
/// `axes` and half-lengths `half` along them.
fn box_corners(
    centre: &Vector3<f64>,
    axes: &Matrix3<f64>,
    half: &Vector3<f64>,
) -> [Vector3<f64>; 8] {
    let mut corners = [*centre; 8];
    for (i, corner) in corners.iter_mut().enumerate() {
        let sign = |bit: usize| if i & bit == 0 { -1.0 } else { 1.0 };
        *corner += axes * Vector3::new(sign(1) * half.x, sign(2) * half.y, sign(4) * half.z);
    }
    corners
}

/// An elliptic frustum: about `centre`, with unit axes the columns of
/// `axes` (the third along the frustum), the cross-section at the centre
/// having half-lengths `half` along the first two axes and growing by the
/// factor `1 + taper · z` at height `z` along the third. It is cut by
/// `faces` and, when given, kept inside `trim`, and lies within
/// `half_length` of its centre along its axis.
#[derive(Clone, Debug, PartialEq)]
pub struct Frustum {
    pub centre: Vector3<f64>,
    pub axes: Matrix3<f64>,
    pub half: [f64; 2],
    pub taper: f64,
    pub faces: Vec<HalfSpace>,
    pub trim: Option<Ellipsoid>,
    pub half_length: f64,
}

impl Frustum {
    fn prepare(&self, grid: &RayGrid) -> PreparedFrustum {
        let rows = self.axes.transpose();
        let one = Vector3::repeat(1.0);
        let [a0, b0] = self.half;
        let k = self.taper;
        let d = local_direction(grid, &rows, &one);
        let mut faces: Vec<PreparedFace> = self.faces.iter().map(|f| f.prepare(grid)).collect();
        if k != 0.0 {
            // Only the nappe the frustum grows from: 1 + k·z ≥ 0.
            let axis: Vector3<f64> = self.axes.column(2).into();
            faces.push(
                HalfSpace {
                    normal: axis * (-k.signum()),
                    offset: -k.signum() * axis.dot(&self.centre) + 1.0 / k.abs(),
                }
                .prepare(grid),
            );
        }
        PreparedFrustum {
            start: Moving::local(grid, &self.centre, &rows, &one),
            direction: d,
            inverse_squares: [1.0 / (a0 * a0), 1.0 / (b0 * b0)],
            taper: k,
            a: d[0] * d[0] / (a0 * a0) + d[1] * d[1] / (b0 * b0) - (k * d[2]) * (k * d[2]),
            faces,
            trim: self.trim.map(|t| t.prepare(grid)),
        }
    }
}

/// A frustum prepared for a grid.
#[derive(Clone, Debug, PartialEq)]
struct PreparedFrustum {
    start: Moving,
    direction: Triple,
    inverse_squares: [f64; 2],
    taper: f64,
    a: f64,
    faces: Vec<PreparedFace>,
    trim: Option<PreparedEllipsoid>,
}

impl PreparedFrustum {
    fn length(&self, x: f64, y: f64) -> f64 {
        let mut bounds = Span::ALL;
        for face in &self.faces {
            match narrowed(bounds, face.span(x, y)) {
                Some(span) => bounds = span,
                None => return 0.0,
            }
        }
        if let Some(trim) = &self.trim {
            match narrowed(bounds, trim.span(x, y)) {
                Some(span) => bounds = span,
                None => return 0.0,
            }
        }
        let o = self.start.at(x, y);
        let d = &self.direction;
        let [ia, ib] = self.inverse_squares;
        let k = self.taper;
        let grow = 1.0 + k * o[2];
        let b = 2.0 * (o[0] * d[0] * ia + o[1] * d[1] * ib - k * d[2] * grow);
        let c = o[0] * o[0] * ia + o[1] * o[1] * ib - grow * grow;
        quadratic_inside(self.a, b, c)
            .iter()
            .flatten()
            .filter_map(|span| span.meet(bounds))
            .map(Span::length)
            .sum()
    }
}

/// A capsule: every point within `radius` of the segment from `start` to
/// `end`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Capsule {
    pub start: Vector3<f64>,
    pub end: Vector3<f64>,
    pub radius: f64,
}

impl Capsule {
    fn prepare(&self, grid: &RayGrid) -> PreparedCapsule {
        let axis = self.end - self.start;
        let long = axis.norm();
        let tube = (long > 0.0).then(|| {
            let u = axis / long;
            // Drop each ray's start and direction onto the plane across
            // the axis.
            let across = |v: Vector3<f64>| -> Triple {
                let w = v - u * v.dot(&u);
                [w.x, w.y, w.z]
            };
            let direction = across(grid.direction);
            PreparedTube {
                start: Moving {
                    base: across(grid.origin - self.start),
                    step_x: across(grid.across),
                    step_y: across(grid.down),
                },
                a: dot(&direction, &direction),
                direction,
                ends: [
                    HalfSpace::behind(self.end, u).prepare(grid),
                    HalfSpace::behind(self.start, -u).prepare(grid),
                ],
            }
        });
        PreparedCapsule {
            caps: [
                Ellipsoid::sphere(self.start, self.radius).prepare(grid),
                Ellipsoid::sphere(self.end, self.radius).prepare(grid),
            ],
            tube,
            radius_squared: self.radius * self.radius,
        }
    }
}

/// The straight part of a capsule, prepared for a grid.
#[derive(Clone, Copy, Debug, PartialEq)]
struct PreparedTube {
    start: Moving,
    direction: Triple,
    a: f64,
    ends: [PreparedFace; 2],
}

/// A capsule prepared for a grid.
#[derive(Clone, Copy, Debug, PartialEq)]
struct PreparedCapsule {
    caps: [PreparedEllipsoid; 2],
    tube: Option<PreparedTube>,
    radius_squared: f64,
}

impl PreparedCapsule {
    fn length(&self, x: f64, y: f64) -> f64 {
        let mut from = f64::INFINITY;
        let mut to = f64::NEG_INFINITY;
        let mut take = |span: Option<Span>| {
            if let Some(span) = span {
                from = from.min(span.from);
                to = to.max(span.to);
            }
        };
        take(self.caps[0].span(x, y));
        take(self.caps[1].span(x, y));
        if let Some(tube) = &self.tube {
            let o = tube.start.at(x, y);
            let [round, _] = quadratic_inside(
                tube.a,
                2.0 * dot(&o, &tube.direction),
                dot(&o, &o) - self.radius_squared,
            );
            let ends = narrowed(Span::ALL, tube.ends[0].span(x, y))
                .and_then(|s| narrowed(s, tube.ends[1].span(x, y)));
            if let (Some(round), Some(ends)) = (round, ends) {
                take(round.meet(ends));
            }
        }
        // The capsule is convex, so its pieces' spans join into one.
        if to > from { to - from } else { 0.0 }
    }
}

/// Any solid.
#[derive(Clone, Debug, PartialEq)]
pub enum Solid {
    Ellipsoid(Ellipsoid),
    Frustum(Frustum),
    Capsule(Capsule),
}

/// A solid prepared for one film's rays.
#[derive(Clone, Debug, PartialEq)]
pub struct Prepared(PreparedShape);

#[derive(Clone, Debug, PartialEq)]
enum PreparedShape {
    Ellipsoid(PreparedEllipsoid),
    Frustum(PreparedFrustum),
    Capsule(PreparedCapsule),
}

impl Prepared {
    /// How far pixel `(x, y)`'s ray travels inside the solid, mm.
    pub fn chord(&self, x: f64, y: f64) -> f64 {
        match &self.0 {
            PreparedShape::Ellipsoid(e) => e.span(x, y).map_or(0.0, Span::length),
            PreparedShape::Frustum(f) => f.length(x, y),
            PreparedShape::Capsule(c) => c.length(x, y),
        }
    }
}

impl Solid {
    /// The solid prepared for a film's rays.
    pub fn prepare(&self, grid: &RayGrid) -> Prepared {
        Prepared(match self {
            Solid::Ellipsoid(e) => PreparedShape::Ellipsoid(e.prepare(grid)),
            Solid::Frustum(f) => PreparedShape::Frustum(f.prepare(grid)),
            Solid::Capsule(c) => PreparedShape::Capsule(c.prepare(grid)),
        })
    }

    /// How far `ray` travels inside the solid, mm.
    pub fn chord(&self, ray: &Ray) -> f64 {
        self.prepare(&RayGrid::single(ray)).chord(0.0, 0.0)
    }

    /// The corners of a box that holds the solid.
    pub fn corners(&self) -> [Vector3<f64>; 8] {
        match self {
            Solid::Ellipsoid(e) => e.corners(),
            Solid::Frustum(f) => {
                let grow = 1.0 + f.taper.abs() * f.half_length;
                let half = Vector3::new(f.half[0] * grow, f.half[1] * grow, f.half_length);
                box_corners(&f.centre, &f.axes, &half)
            }
            Solid::Capsule(c) => {
                let low = c.start.inf(&c.end).add_scalar(-c.radius);
                let high = c.start.sup(&c.end).add_scalar(c.radius);
                box_corners(
                    &(0.5 * (low + high)),
                    &Matrix3::identity(),
                    &(0.5 * (high - low)),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: f64) -> f64 {
        x * x
    }
    use forward_model::rotation::{rot_x, rot_y, rot_z};

    fn along_x(y: f64, z: f64) -> Ray {
        Ray {
            origin: Vector3::new(-500.0, y, z),
            direction: Vector3::x(),
        }
    }

    /// The slow, plainly right answer: march along the ray in tiny steps
    /// and count the steps inside.
    fn marched(inside: impl Fn(&Vector3<f64>) -> bool, ray: &Ray) -> f64 {
        let step = 0.002;
        (0..500_000)
            .filter(|&i| inside(&(ray.origin + ray.direction * (f64::from(i) * step + 0.5 * step))))
            .count() as f64
            * step
    }

    #[test]
    fn a_sphere_chord_is_twice_the_half_chord() {
        // Worked by hand: 2·sqrt(r² − ρ²).
        let sphere = Solid::Ellipsoid(Ellipsoid::sphere(Vector3::new(3.0, 1.0, 2.0), 10.0));
        for rho in [0.0, 3.0, 6.0, 9.9] {
            let chord = sphere.chord(&along_x(1.0 + rho, 2.0));
            assert!(
                (chord - 2.0 * (100.0 - rho * rho).sqrt()).abs() < 1e-9,
                "{rho}"
            );
        }
        assert_eq!(sphere.chord(&along_x(12.0, 2.0)), 0.0);
    }

    #[test]
    fn a_turned_ellipsoid_matches_a_march() {
        let axes = rot_x(0.4) * rot_y(-0.7) * rot_z(1.1);
        let e = Ellipsoid {
            centre: Vector3::new(1.0, -2.0, 3.0),
            axes,
            radii: Vector3::new(12.0, 5.0, 8.0),
        };
        let inside = |p: &Vector3<f64>| {
            let q = (axes.transpose() * (p - e.centre)).component_div(&e.radii);
            q.norm_squared() <= 1.0
        };
        for (y, z) in [(0.0, 0.0), (-4.0, 5.0), (2.0, -3.0), (-9.0, 3.0)] {
            let ray = along_x(y, z);
            let exact = Solid::Ellipsoid(e).chord(&ray);
            assert!((exact - marched(inside, &ray)).abs() < 0.005, "{y} {z}");
        }
    }

    fn body() -> Frustum {
        let axes = rot_x(0.3) * rot_z(0.5);
        let up: Vector3<f64> = axes.column(2).into();
        let centre = Vector3::new(0.0, 0.0, 0.0);
        let top_normal = (axes * Vector3::new(0.05, -0.08, 1.0)).normalize();
        let bottom_normal = (axes * Vector3::new(-0.05, 0.08, 1.0)).normalize();
        Frustum {
            centre,
            axes,
            half: [10.0, 15.0],
            taper: 0.004,
            faces: vec![
                HalfSpace::behind(centre + up * 10.0, top_normal),
                HalfSpace::behind(centre - up * 10.0, -bottom_normal),
            ],
            trim: Some(Ellipsoid {
                centre,
                axes,
                radii: Vector3::new(11.9, 17.85, 16.6),
            }),
            half_length: 14.0,
        }
    }

    #[test]
    fn a_cut_trimmed_frustum_matches_a_march() {
        let f = body();
        let inside = |p: &Vector3<f64>| {
            let q = f.axes.transpose() * (p - f.centre);
            let grow = 1.0 + f.taper * q.z;
            let in_section = square(q.x / f.half[0]) + square(q.y / f.half[1]) <= grow * grow;
            let in_faces = f.faces.iter().all(|face| face.normal.dot(p) <= face.offset);
            let trim = f.trim.expect("a trim");
            let t = (trim.axes.transpose() * (p - trim.centre)).component_div(&trim.radii);
            in_section && in_faces && t.norm_squared() <= 1.0 && grow >= 0.0
        };
        for (y, z) in [
            (0.0, 0.0),
            (5.0, 4.0),
            (-12.0, -6.0),
            (14.0, 9.0),
            (3.0, -10.5),
        ] {
            let ray = along_x(y, z);
            let exact = Solid::Frustum(f.clone()).chord(&ray);
            assert!(
                (exact - marched(inside, &ray)).abs() < 0.005,
                "{y} {z}: {exact}"
            );
        }
    }

    #[test]
    fn a_ray_along_a_tube_crosses_only_its_wall() {
        // A hollow tube seen end on: the wall's length where the ray meets
        // it, nothing in the hollow. Worked by hand for a tube 12 long.
        let tube = |radius: f64| Frustum {
            centre: Vector3::zeros(),
            axes: rot_y(core::f64::consts::FRAC_PI_2),
            half: [radius, radius],
            taper: 0.0,
            faces: vec![
                HalfSpace::behind(Vector3::new(6.0, 0.0, 0.0), Vector3::x()),
                HalfSpace::behind(Vector3::new(-6.0, 0.0, 0.0), -Vector3::x()),
            ],
            trim: None,
            half_length: 6.0,
        };
        let wall = |y: f64| {
            Solid::Frustum(tube(4.0)).chord(&along_x(y, 0.0))
                - Solid::Frustum(tube(3.0)).chord(&along_x(y, 0.0))
        };
        assert!(wall(0.0).abs() < 1e-9);
        assert!((wall(3.5) - 12.0).abs() < 1e-9);
        assert!(wall(4.5).abs() < 1e-9);
    }

    #[test]
    fn mitred_tube_pieces_meet_without_gap_or_overlap() {
        // Two tube pieces bent 40° at a joint, each cut by the bisecting
        // plane: together they hold exactly what one bent tube holds.
        let joint = Vector3::zeros();
        let first: Vector3<f64> = Vector3::new(1.0, 0.0, 0.0);
        let second: Vector3<f64> = rot_z(0.7) * first;
        let bisector = (first + second).normalize();
        let piece =
            |dir: Vector3<f64>, start: Vector3<f64>, end: Vector3<f64>, keep_after: bool| {
                let axes = {
                    let z = dir;
                    let x = Vector3::z().cross(&z).normalize();
                    let y = z.cross(&x);
                    Matrix3::from_columns(&[x, y, z])
                };
                let cut = if keep_after {
                    HalfSpace::behind(joint, -bisector)
                } else {
                    HalfSpace::behind(joint, bisector)
                };
                let far = if keep_after {
                    HalfSpace::behind(end, dir)
                } else {
                    HalfSpace::behind(start, -dir)
                };
                Solid::Frustum(Frustum {
                    centre: 0.5 * (start + end),
                    axes,
                    half: [2.0, 2.0],
                    taper: 0.0,
                    faces: vec![cut, far],
                    trim: None,
                    half_length: 32.0,
                })
            };
        let a = piece(first, -first * 30.0, joint, false);
        let b = piece(second, joint, second * 30.0, true);
        let inside = |p: &Vector3<f64>| {
            let before = p.dot(&bisector) <= 0.0;
            let (dir, from) = if before {
                (first, -first * 30.0)
            } else {
                (second, Vector3::zeros())
            };
            let s = (p - from).dot(&dir);
            let off = (p - from - dir * s).norm();
            off <= 2.0 && (0.0..=30.0).contains(&s)
        };
        for y in [-1.5, 0.0, 0.8, 1.9] {
            let ray = Ray {
                origin: Vector3::new(-0.5, y, -100.0),
                direction: Vector3::z(),
            };
            let tilted = Ray {
                origin: Vector3::new(-40.0, y - 5.0, 1.0),
                direction: Vector3::new(1.0, 0.2, 0.0).normalize(),
            };
            for r in [ray, tilted] {
                let exact = a.chord(&r) + b.chord(&r);
                assert!((exact - marched(inside, &r)).abs() < 0.005, "{y}");
            }
        }
    }

    #[test]
    fn every_solid_lies_inside_its_box() {
        let f = body();
        let solids = [
            Solid::Frustum(f.clone()),
            Solid::Ellipsoid(Ellipsoid {
                centre: Vector3::new(1.0, 2.0, 3.0),
                axes: rot_x(0.3) * rot_z(-0.8),
                radii: Vector3::new(4.0, 9.0, 2.0),
            }),
            Solid::Capsule(Capsule {
                start: Vector3::new(-3.0, 1.0, 0.0),
                end: Vector3::new(5.0, -2.0, 7.0),
                radius: 2.0,
            }),
        ];
        for solid in &solids {
            let corners = solid.corners();
            let low = corners
                .iter()
                .fold(Vector3::repeat(f64::MAX), |a, c| a.inf(c));
            let high = corners
                .iter()
                .fold(Vector3::repeat(f64::MIN), |a, c| a.sup(c));
            // Rays along x just outside the box in y or z miss the solid.
            for (y, z) in [
                (low.y - 0.01, 0.5 * (low.z + high.z)),
                (high.y + 0.01, 0.0),
                (0.0, high.z + 0.01),
                (1.0, low.z - 0.01),
            ] {
                assert_eq!(solid.chord(&along_x(y, z)), 0.0, "{solid:?} at {y} {z}");
            }
        }
    }

    #[test]
    fn a_prepared_solid_gives_each_pixel_its_own_ray() {
        // A grid whose pixels step 0.5 mm across and 0.25 mm down: pixel
        // (x, y) must answer as the ray from its own start.
        let grid = RayGrid {
            origin: Vector3::new(-200.0, -15.0, -12.0),
            across: Vector3::new(0.0, 0.5, 0.0),
            down: Vector3::new(0.0, 0.0, 0.25),
            direction: Vector3::x(),
        };
        let solids = [
            Solid::Frustum(body()),
            Solid::Capsule(Capsule {
                start: Vector3::new(0.0, -8.0, -5.0),
                end: Vector3::new(4.0, 9.0, 6.0),
                radius: 3.0,
            }),
        ];
        for solid in &solids {
            let prepared = solid.prepare(&grid);
            for (x, y) in [(0.0, 0.0), (30.0, 48.0), (41.0, 50.0), (17.0, 90.0)] {
                let ray = Ray {
                    origin: grid.origin + grid.across * x + grid.down * y,
                    direction: grid.direction,
                };
                assert!((prepared.chord(x, y) - solid.chord(&ray)).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn a_capsule_is_a_cylinder_with_round_ends() {
        let capsule = Solid::Capsule(Capsule {
            start: Vector3::new(0.0, 0.0, -10.0),
            end: Vector3::new(0.0, 0.0, 10.0),
            radius: 3.0,
        });
        // Across the middle: 2·sqrt(9 − ρ²); across an end cap: the
        // sphere's chord.
        assert!((capsule.chord(&along_x(1.0, 0.0)) - 2.0 * 8f64.sqrt()).abs() < 1e-9);
        assert!((capsule.chord(&along_x(0.0, 12.0)) - 2.0 * 5f64.sqrt()).abs() < 1e-9);
        assert_eq!(capsule.chord(&along_x(0.0, 13.5)), 0.0);
        // Along its axis: the whole length plus both caps.
        let down = Ray {
            origin: Vector3::new(0.5, 0.0, 100.0),
            direction: -Vector3::z(),
        };
        let expected = 20.0 + 2.0 * (9.0f64 - 0.25).sqrt();
        assert!((capsule.chord(&down) - expected).abs() < 1e-9);
    }
}
