//! Independent scalar energy and first derivatives for the comparison solves.
//! This test implementation calls no load-bench or forward-model mechanics.

#![allow(dead_code)]

use nalgebra::{Matrix3, SVector, Vector3};

#[derive(Clone, Debug)]
pub struct Body {
    pub centre: Vector3<f64>,
    pub rotation: Matrix3<f64>,
}

pub fn turn(vector: Vector3<f64>) -> Matrix3<f64> {
    let angle = vector.norm();
    if angle == 0.0 {
        return Matrix3::identity();
    }
    let axis = vector / angle;
    let skew = Matrix3::new(
        0.0, -axis.z, axis.y, axis.z, 0.0, -axis.x, -axis.y, axis.x, 0.0,
    );
    Matrix3::identity() * libm::cos(angle)
        + (axis * axis.transpose()) * (1.0 - libm::cos(angle))
        + skew * libm::sin(angle)
}

fn logarithm(matrix: Matrix3<f64>) -> Vector3<f64> {
    let sine = Vector3::new(
        matrix[(2, 1)] - matrix[(1, 2)],
        matrix[(0, 2)] - matrix[(2, 0)],
        matrix[(1, 0)] - matrix[(0, 1)],
    ) * 0.5;
    let magnitude = sine.norm();
    let cosine = (matrix.trace() - 1.0) * 0.5;
    if magnitude < 1e-14 && cosine > 0.0 {
        return sine;
    }
    sine * (libm::atan2(magnitude, cosine) / magnitude)
}

#[derive(Clone)]
struct Spring {
    upper_arm: Vector3<f64>,
    lower_arm: Vector3<f64>,
    relative_rest: Matrix3<f64>,
    diagonal: SVector<f64, 6>,
}

pub struct IndependentChain {
    springs: Vec<Spring>,
    pub ground: Body,
}

impl IndependentChain {
    /// Reference bodies and heights are top to bottom. One positive diagonal
    /// per disc, including the final disc to the supplied sacrum top.
    pub fn new(reference: &[Body], heights: &[f64], diagonals: &[[f64; 6]], ground: Body) -> Self {
        assert_eq!(reference.len(), heights.len());
        assert_eq!(reference.len(), diagonals.len());
        let springs = reference
            .iter()
            .enumerate()
            .map(|(i, upper)| {
                let lower = reference.get(i + 1).unwrap_or(&ground);
                let upper_bottom =
                    upper.centre - upper.rotation * Vector3::new(0.0, 0.0, heights[i] * 0.5);
                let lower_top = reference.get(i + 1).map_or(ground.centre, |b| {
                    b.centre + b.rotation * Vector3::new(0.0, 0.0, heights[i + 1] * 0.5)
                });
                let disc = (upper_bottom + lower_top) * 0.5;
                Spring {
                    upper_arm: upper.rotation.transpose() * (disc - upper.centre),
                    lower_arm: lower.rotation.transpose() * (disc - lower.centre),
                    relative_rest: lower.rotation.transpose() * upper.rotation,
                    diagonal: diagonals[i].into(),
                }
            })
            .collect();
        Self { springs, ground }
    }

    pub fn energy(&self, bodies: &[Body]) -> f64 {
        self.springs
            .iter()
            .enumerate()
            .map(|(i, spring)| spring.energy(&bodies[i], bodies.get(i + 1).unwrap_or(&self.ground)))
            .sum()
    }

    pub fn adjacent_energy(&self, bodies: &[Body], body: usize) -> f64 {
        let below =
            self.springs[body].energy(&bodies[body], bodies.get(body + 1).unwrap_or(&self.ground));
        if body == 0 {
            below
        } else {
            below + self.springs[body - 1].energy(&bodies[body - 1], &bodies[body])
        }
    }

    /// Analytic virtual-work derivative, used only by plain descent. There is
    /// no tangent or normal matrix in this deliberately separate algorithm.
    pub fn gradient(&self, bodies: &[Body]) -> Vec<SVector<f64, 6>> {
        let mut gradients = vec![SVector::<f64, 6>::zeros(); bodies.len()];
        for (i, spring) in self.springs.iter().enumerate() {
            let lower = bodies.get(i + 1).unwrap_or(&self.ground);
            let (up, down) = spring.gradient(&bodies[i], lower);
            gradients[i] += up;
            if let Some(next) = gradients.get_mut(i + 1) {
                *next += down;
            }
        }
        gradients
    }
}

impl Spring {
    fn deformation(&self, upper: &Body, lower: &Body) -> SVector<f64, 6> {
        let a = upper.centre + upper.rotation * self.upper_arm;
        let b = lower.centre + lower.rotation * self.lower_arm;
        let translation = lower.rotation.transpose() * (a - b);
        let rotation =
            logarithm(lower.rotation.transpose() * upper.rotation * self.relative_rest.transpose());
        SVector::<f64, 6>::new(
            translation.x,
            translation.y,
            translation.z,
            rotation.x,
            rotation.y,
            rotation.z,
        )
    }

    fn energy(&self, upper: &Body, lower: &Body) -> f64 {
        let d = self.deformation(upper, lower);
        (0..6)
            .map(|axis| 0.5 * self.diagonal[axis] * d[axis] * d[axis])
            .sum()
    }

    fn gradient(&self, upper: &Body, lower: &Body) -> (SVector<f64, 6>, SVector<f64, 6>) {
        let d = self.deformation(upper, lower);
        let response = self.diagonal.component_mul(&d);
        let f = lower.rotation * response.fixed_rows::<3>(0);
        let phi = d.fixed_rows::<3>(3).into_owned();
        let phi_squared = phi.norm_squared();
        let coefficient = if phi_squared < 1e-7 {
            1.0 / 12.0 + phi_squared / 720.0
        } else {
            let angle = phi_squared.sqrt();
            (1.0 - 0.5 * angle / libm::tan(0.5 * angle)) / phi_squared
        };
        let skew = Matrix3::new(0.0, -phi.z, phi.y, phi.z, 0.0, -phi.x, -phi.y, phi.x, 0.0);
        let derivative = Matrix3::identity() - skew * 0.5 + skew * skew * coefficient;
        let moment = lower.rotation * derivative.transpose() * response.fixed_rows::<3>(3);
        let arm = upper.rotation * self.upper_arm;
        let upper_moment = arm.cross(&f) + moment;
        let lower_moment = -(upper.centre + arm - lower.centre).cross(&f) - moment;
        (
            SVector::<f64, 6>::new(
                f.x,
                f.y,
                f.z,
                upper_moment.x,
                upper_moment.y,
                upper_moment.z,
            ),
            SVector::<f64, 6>::new(
                -f.x,
                -f.y,
                -f.z,
                lower_moment.x,
                lower_moment.y,
                lower_moment.z,
            ),
        )
    }
}

/// Exact point-preserving turn, independently parameterised by its world
/// line through a pair of heads.
pub fn about_line(body: &Body, point: Vector3<f64>, axis: Vector3<f64>, angle: f64) -> Body {
    let q = turn(axis * angle);
    Body {
        centre: point + q * (body.centre - point),
        rotation: q * body.rotation,
    }
}

/// Cyclic scalar minimisation. Each line search is a bracketed golden-section
/// search on the exact scalar energy; no gradient or tangent is shared.
pub fn settle_head_line_angles(
    chain: &IndependentChain,
    targets: &[Body],
    local_midpoint: Vector3<f64>,
) -> (Vec<Body>, Vec<f64>) {
    let midpoints: Vec<_> = targets
        .iter()
        .map(|body| body.centre + body.rotation * local_midpoint)
        .collect();
    let axes: Vec<_> = targets
        .iter()
        .map(|body| body.rotation * Vector3::y())
        .collect();
    let mut bodies = targets.to_vec();
    let mut angles = vec![0.0; targets.len()];
    let golden = (5.0_f64.sqrt() - 1.0) * 0.5;
    for _ in 0..200 {
        let mut largest = 0.0_f64;
        for i in 0..bodies.len() {
            let old = angles[i];
            let old_energy = chain.adjacent_energy(&bodies, i);
            let mut left = old - 0.25;
            let mut right = old + 0.25;
            let mut a = right - golden * (right - left);
            let mut b = left + golden * (right - left);
            bodies[i] = about_line(&targets[i], midpoints[i], axes[i], a);
            let mut fa = chain.adjacent_energy(&bodies, i);
            bodies[i] = about_line(&targets[i], midpoints[i], axes[i], b);
            let mut fb = chain.adjacent_energy(&bodies, i);
            for _ in 0..36 {
                if fa < fb {
                    right = b;
                    b = a;
                    fb = fa;
                    a = right - golden * (right - left);
                    bodies[i] = about_line(&targets[i], midpoints[i], axes[i], a);
                    fa = chain.adjacent_energy(&bodies, i);
                } else {
                    left = a;
                    a = b;
                    fa = fb;
                    b = left + golden * (right - left);
                    bodies[i] = about_line(&targets[i], midpoints[i], axes[i], b);
                    fb = chain.adjacent_energy(&bodies, i);
                }
            }
            let minimum = (left + right) * 0.5;
            // Own numerical choice: over-relax the scalar minimum only
            // when the exact adjacent-joint energy still decreases.
            let extrapolated = old + 1.6 * (minimum - old);
            bodies[i] = about_line(&targets[i], midpoints[i], axes[i], extrapolated);
            angles[i] = if chain.adjacent_energy(&bodies, i) <= old_energy {
                extrapolated
            } else {
                minimum
            };
            largest = largest.max((angles[i] - old).abs());
            bodies[i] = about_line(&targets[i], midpoints[i], axes[i], angles[i]);
        }
        if largest < 1e-7 {
            return (bodies, angles);
        }
    }
    panic!("independent angle-coordinate descent did not settle");
}

#[derive(Clone, Copy, Debug)]
pub struct PlanarPoint {
    pub x: f64,
    pub z: f64,
}
impl PlanarPoint {
    fn turned(self, angle: f64) -> Self {
        let c = libm::cos(angle);
        let s = libm::sin(angle);
        Self {
            x: c * self.x + s * self.z,
            z: -s * self.x + c * self.z,
        }
    }
    fn dot(self, b: Self) -> f64 {
        self.x * b.x + self.z * b.z
    }
    fn cross_y(self, b: Self) -> f64 {
        self.z * b.x - self.x * b.z
    }
    fn plus(self, b: Self) -> Self {
        Self {
            x: self.x + b.x,
            z: self.z + b.z,
        }
    }
    fn minus(self, b: Self) -> Self {
        Self {
            x: self.x - b.x,
            z: self.z - b.z,
        }
    }
    fn times(self, s: f64) -> Self {
        Self {
            x: self.x * s,
            z: self.z * s,
        }
    }
}

struct PlanarSpring {
    upper_z: f64,
    lower_z: f64,
    kx: f64,
    kz: f64,
    kr: f64,
}

/// Matrix-free dynamic relaxation of the same planar subset of state H.
/// The scalar implementation has its own spring and contour formulas.
/// A finite penalty is intentional: its gap and force error set the
/// independently stated comparison tolerance in the integration test.
pub struct PlanarRelaxation {
    springs: Vec<PlanarSpring>,
    ground_z: f64,
    points: Vec<PlanarPoint>,
    knots: Vec<f64>,
    slopes: Vec<PlanarPoint>,
    rod_centre: PlanarPoint,
    head_back: f64,
    stations: Vec<f64>,
    force_scale: f64,
}

pub struct Relaxed {
    pub poses: Vec<Body>,
    pub forces_on_rod: Vec<Vector3<f64>>,
    pub gap_mm: f64,
    pub gradient_equivalent: f64,
}

impl PlanarRelaxation {
    pub fn new(
        reference_z: &[f64],
        heights: &[f64],
        diagonals: &[[f64; 6]],
        ground_z: f64,
        points: &[Vector3<f64>],
        stations: &[f64],
        head_back: f64,
    ) -> Self {
        let force_scale = diagonals
            .iter()
            .flat_map(|k| [k[0], k[2], k[4] / 900.0])
            .fold(0.0_f64, f64::max);
        let springs = reference_z
            .iter()
            .enumerate()
            .map(|(i, &z)| {
                let lower_z = reference_z.get(i + 1).copied().unwrap_or(ground_z);
                let lower_top = reference_z
                    .get(i + 1)
                    .map_or(ground_z, |value| value + heights[i + 1] * 0.5);
                let disc = (z - heights[i] * 0.5 + lower_top) * 0.5;
                PlanarSpring {
                    upper_z: disc - z,
                    lower_z: disc - lower_z,
                    kx: diagonals[i][0] / force_scale,
                    kz: diagonals[i][2] / force_scale,
                    kr: diagonals[i][4] / force_scale,
                }
            })
            .collect();
        let centre = points.iter().copied().sum::<Vector3<f64>>() / points.len() as f64;
        let rod_centre = PlanarPoint {
            x: centre.x,
            z: centre.z,
        };
        let points: Vec<_> = points
            .iter()
            .map(|p| PlanarPoint {
                x: p.x - centre.x,
                z: p.z - centre.z,
            })
            .collect();
        let mut knots = vec![0.0];
        for pair in points.windows(2) {
            let d = pair[1].minus(pair[0]);
            knots.push(knots.last().copied().unwrap() + d.dot(d).sqrt());
        }
        let slopes = (0..points.len())
            .map(|i| {
                let a = i.saturating_sub(1);
                let b = (i + 1).min(points.len() - 1);
                points[b]
                    .minus(points[a])
                    .times(1.0 / (knots[b] - knots[a]))
            })
            .collect();
        Self {
            springs,
            ground_z,
            points,
            knots,
            slopes,
            rod_centre,
            head_back,
            stations: stations.to_vec(),
            force_scale,
        }
    }

    fn contour(&self, s: f64) -> (PlanarPoint, PlanarPoint) {
        let i = self
            .knots
            .partition_point(|k| *k <= s)
            .saturating_sub(1)
            .min(self.points.len() - 2);
        let h = self.knots[i + 1] - self.knots[i];
        let t = (s - self.knots[i]) / h;
        let mut p = PlanarPoint { x: 0.0, z: 0.0 };
        let mut d = p;
        for (point, basis, derivative) in [
            (
                self.points[i],
                (1.0 + 2.0 * t) * (1.0 - t) * (1.0 - t),
                6.0 * t * (t - 1.0) / h,
            ),
            (
                self.points[i + 1],
                t * t * (3.0 - 2.0 * t),
                6.0 * t * (1.0 - t) / h,
            ),
            (
                self.slopes[i],
                h * t * (1.0 - t) * (1.0 - t),
                (1.0 - t) * (1.0 - 3.0 * t),
            ),
            (
                self.slopes[i + 1],
                h * t * t * (t - 1.0),
                t * (3.0 * t - 2.0),
            ),
        ] {
            p = p.plus(point.times(basis));
            d = d.plus(point.times(derivative));
        }
        (p, d)
    }

    fn evaluate(&self, x: &[f64], penalty: f64) -> (f64, Vec<f64>, Vec<PlanarPoint>, f64) {
        let n = self.springs.len();
        let rod = 3 * n;
        let mut energy = 0.0;
        let mut gradient = vec![0.0; x.len()];
        for (i, spring) in self.springs.iter().enumerate() {
            let upper = PlanarPoint {
                x: x[3 * i],
                z: x[3 * i + 1],
            };
            let upper_angle = x[3 * i + 2];
            let (lower, lower_angle) = if i + 1 < n {
                (
                    PlanarPoint {
                        x: x[3 * (i + 1)],
                        z: x[3 * (i + 1) + 1],
                    },
                    x[3 * (i + 1) + 2],
                )
            } else {
                (
                    PlanarPoint {
                        x: 0.0,
                        z: self.ground_z,
                    },
                    0.0,
                )
            };
            let arm = PlanarPoint {
                x: 0.0,
                z: spring.upper_z,
            }
            .turned(upper_angle);
            let lower_arm = PlanarPoint {
                x: 0.0,
                z: spring.lower_z,
            }
            .turned(lower_angle);
            let relative = upper
                .plus(arm)
                .minus(lower.plus(lower_arm))
                .turned(-lower_angle);
            let angle = upper_angle - lower_angle;
            energy += 0.5
                * (spring.kx * relative.x * relative.x
                    + spring.kz * relative.z * relative.z
                    + spring.kr * angle * angle);
            let force = PlanarPoint {
                x: spring.kx * relative.x,
                z: spring.kz * relative.z,
            }
            .turned(lower_angle);
            gradient[3 * i] += force.x;
            gradient[3 * i + 1] += force.z;
            gradient[3 * i + 2] += arm.cross_y(force) + spring.kr * angle;
            if i + 1 < n {
                gradient[3 * (i + 1)] -= force.x;
                gradient[3 * (i + 1) + 1] -= force.z;
                gradient[3 * (i + 1) + 2] -=
                    upper.plus(arm).minus(lower).cross_y(force) + spring.kr * angle;
            }
        }
        let mut loads = Vec::with_capacity(n);
        let mut largest_gap = 0.0_f64;
        for i in 0..n {
            let station = if i + 1 == n {
                self.stations[i]
            } else {
                x[rod + 3 + i]
            };
            let (point, tangent) = self.contour(station);
            let arm = PlanarPoint {
                x: self.head_back,
                z: 0.0,
            }
            .turned(x[3 * i + 2]);
            let rod_arm = point.turned(x[rod + 2]);
            let gap = PlanarPoint {
                x: x[3 * i] + arm.x - x[rod] - rod_arm.x,
                z: x[3 * i + 1] + arm.z - x[rod + 1] - rod_arm.z,
            };
            largest_gap = largest_gap.max(gap.dot(gap).sqrt());
            energy += 0.5 * penalty * gap.dot(gap);
            let force = gap.times(penalty);
            gradient[3 * i] += force.x;
            gradient[3 * i + 1] += force.z;
            gradient[3 * i + 2] += arm.cross_y(force);
            gradient[rod] -= force.x;
            gradient[rod + 1] -= force.z;
            gradient[rod + 2] -= rod_arm.cross_y(force);
            if i + 1 < n {
                gradient[rod + 3 + i] -= tangent.turned(x[rod + 2]).dot(force);
            }
            loads.push(force);
        }
        (energy, gradient, loads, largest_gap)
    }

    fn masses(&self, penalty: f64) -> Vec<f64> {
        let n = self.springs.len();
        let rod = 3 * n;
        let mut masses = vec![penalty; 4 * n + 2];
        for i in 0..n {
            let mut translation = 0.0;
            let mut rotation = 0.0;
            for j in i.saturating_sub(1)..=i {
                let s = &self.springs[j];
                translation += s.kx.max(s.kz);
                let arm = if j == i { s.upper_z } else { s.lower_z };
                rotation += s.kr + s.kx.max(s.kz) * arm * arm;
            }
            masses[3 * i] = translation + penalty;
            masses[3 * i + 1] = translation + penalty;
            masses[3 * i + 2] = rotation + penalty * self.head_back * self.head_back;
        }
        masses[rod] = penalty * n as f64;
        masses[rod + 1] = penalty * n as f64;
        masses[rod + 2] = penalty
            * self
                .stations
                .iter()
                .map(|s| {
                    let (p, _) = self.contour(*s);
                    p.dot(p)
                })
                .sum::<f64>();
        masses
    }

    pub fn solve(&self, targets: &[Body]) -> Relaxed {
        let n = targets.len();
        let rod = 3 * n;
        let dimension = 4 * n + 2;
        let mut x = vec![0.0; dimension];
        for (i, body) in targets.iter().enumerate() {
            x[3 * i] = body.centre.x;
            x[3 * i + 1] = body.centre.z;
        }
        x[rod] = self.rod_centre.x;
        x[rod + 1] = self.rod_centre.z;
        x[rod + 3..].copy_from_slice(&self.stations[..n - 1]);
        let mut largest_gradient = f64::INFINITY;
        // Own numerical continuation and pseudo-time choices. Neither a
        // clock nor a tangent determines the independent iteration sequence.
        for penalty in [10.0, 100.0, 1000.0, 10_000.0] {
            let masses = self.masses(penalty);
            let mut velocity = vec![0.0; dimension];
            let mut dt = 0.2_f64;
            let mut mixing = 0.1_f64;
            let mut downhill = 0usize;
            let mut settled = false;
            for _ in 0..40_000 {
                let (_, gradient, _, _) = self.evaluate(&x, penalty);
                largest_gradient = gradient
                    .iter()
                    .enumerate()
                    .map(|(i, g)| {
                        g.abs()
                            / if i < rod && i % 3 == 2 {
                                30.0
                            } else if i == rod + 2 {
                                60.0
                            } else {
                                1.0
                            }
                    })
                    .fold(0.0_f64, f64::max);
                if largest_gradient < 1e-7 {
                    settled = true;
                    break;
                }
                for i in 0..dimension {
                    velocity[i] -= dt * gradient[i] / masses[i];
                }
                let power = velocity
                    .iter()
                    .zip(&gradient)
                    .map(|(v, g)| v * g)
                    .sum::<f64>();
                if power > 0.0 {
                    velocity.fill(0.0);
                    dt = (dt * 0.5).max(1e-5);
                    mixing = 0.1;
                    downhill = 0;
                } else {
                    let kinetic = velocity
                        .iter()
                        .zip(&masses)
                        .map(|(v, m)| v * v * m)
                        .sum::<f64>();
                    let force_squared = gradient
                        .iter()
                        .zip(&masses)
                        .map(|(g, m)| g * g / m)
                        .sum::<f64>();
                    if force_squared > 0.0 {
                        let ratio = (kinetic / force_squared).sqrt();
                        for i in 0..dimension {
                            velocity[i] = (1.0 - mixing) * velocity[i]
                                - mixing * ratio * gradient[i] / masses[i];
                        }
                    }
                    downhill += 1;
                    if downhill > 5 {
                        dt = (dt * 1.1).min(0.7);
                        mixing *= 0.99;
                    }
                }
                for i in 0..dimension {
                    x[i] += dt * velocity[i];
                }
                for i in 0..n - 1 {
                    x[rod + 3 + i] = x[rod + 3 + i].clamp(0.0, *self.knots.last().unwrap());
                }
            }
            assert!(
                settled,
                "independent dynamic relaxation cap: gradient {largest_gradient}"
            );
        }
        let (_, _, loads, gap_mm) = self.evaluate(&x, 10_000.0);
        Relaxed {
            poses: (0..n)
                .map(|i| Body {
                    centre: Vector3::new(x[3 * i], 0.0, x[3 * i + 1]),
                    rotation: turn(Vector3::y() * x[3 * i + 2]),
                })
                .collect(),
            forces_on_rod: loads
                .iter()
                .map(|f| Vector3::new(f.x * self.force_scale, 0.0, f.z * self.force_scale))
                .collect(),
            gap_mm,
            gradient_equivalent: largest_gradient,
        }
    }
}
