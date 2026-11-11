//! Penalised cubic splines, parameterised by physical arc length.

/// A point in the upright plane, in length units.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn distance(self, other: Self) -> f64 {
        libm::hypot(self.x - other.x, self.y - other.y)
    }
}

/// Position, unit tangent and signed curvature toward upright image right.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Sample {
    pub arc: f64,
    pub point: Point,
    pub tangent: Point,
    pub curvature: f64,
}

/// Cubic cardinal B-spline with 10-unit knots and a second-difference penalty.
struct Spline {
    x: Vec<f64>,
    y: Vec<f64>,
    length: f64,
}

const KNOT: f64 = 10.0;

fn basis(u: f64) -> ([f64; 4], [f64; 4], [f64; 4]) {
    let u2 = u * u;
    let u3 = u2 * u;
    (
        [
            ((1.0 - u) * (1.0 - u) * (1.0 - u)) / 6.0,
            (3.0 * u3 - 6.0 * u2 + 4.0) / 6.0,
            (-3.0 * u3 + 3.0 * u2 + 3.0 * u + 1.0) / 6.0,
            u3 / 6.0,
        ],
        [
            -0.5 * ((1.0 - u) * (1.0 - u)),
            1.5 * u2 - 2.0 * u,
            -1.5 * u2 + u + 0.5,
            0.5 * u2,
        ],
        [1.0 - u, 3.0 * u - 2.0, 1.0 - 3.0 * u, u],
    )
}

fn solve_banded(mut a: Vec<[f64; 4]>, mut b: Vec<f64>) -> Vec<f64> {
    let n = a.len();
    for i in 0..n {
        for j in i.saturating_sub(3)..=i {
            let mut sum = a[i][i - j];
            for k in i.saturating_sub(3)..j {
                if j - k <= 3 {
                    sum -= a[i][i - k] * a[j][j - k];
                }
            }
            a[i][i - j] = if i == j {
                sum.max(1e-12).sqrt()
            } else {
                sum / a[j][0]
            };
        }
    }
    for i in 0..n {
        for j in i.saturating_sub(3)..i {
            b[i] -= a[i][i - j] * b[j];
        }
        b[i] /= a[i][0];
    }
    for i in (0..n).rev() {
        for j in i + 1..(i + 4).min(n) {
            b[i] -= a[j][j - i] * b[j];
        }
        b[i] /= a[i][0];
    }
    b
}

impl Spline {
    fn fit(points: &[Point], weights: &[f64], cutoff: f64) -> Self {
        let length = (points.len() - 1) as f64;
        let n = (length / KNOT).floor() as usize + 4;
        let mut matrix = vec![[0.0; 4]; n];
        let mut bx = vec![0.0; n];
        let mut by = vec![0.0; n];
        for (i, (point, &weight)) in points.iter().zip(weights).enumerate() {
            let t = i as f64 / KNOT;
            let start = t.floor() as usize;
            let (b, _, _) = basis(t - t.floor());
            for j in 0..4 {
                bx[start + j] += weight * b[j] * point.x;
                by[start + j] += weight * b[j] * point.y;
                for k in 0..=j {
                    matrix[start + j][j - k] += weight * b[j] * b[k];
                }
            }
        }
        let mean_weight = weights.iter().sum::<f64>() / weights.len() as f64;
        let lambda = KNOT * mean_weight
            / libm::pow(2.0 * libm::sin(core::f64::consts::PI * KNOT / cutoff), 4.0);
        let difference = [1.0, -2.0, 1.0];
        for i in 0..n - 2 {
            for j in 0..3 {
                for k in 0..=j {
                    matrix[i + j][j - k] += lambda * difference[j] * difference[k];
                }
            }
        }
        Self {
            x: solve_banded(matrix.clone(), bx),
            y: solve_banded(matrix, by),
            length,
        }
    }

    fn evaluate(&self, parameter: f64) -> (Point, Point, Point) {
        let t = parameter.clamp(0.0, self.length) / KNOT;
        let start = t.floor() as usize;
        let (b, first, second) = basis(t - t.floor());
        let mut point = Point::default();
        let mut derivative = Point::default();
        let mut curvature = Point::default();
        for j in 0..4 {
            point.x += b[j] * self.x[start + j];
            point.y += b[j] * self.y[start + j];
            derivative.x += first[j] * self.x[start + j] / KNOT;
            derivative.y += first[j] * self.y[start + j] / KNOT;
            curvature.x += second[j] * self.x[start + j] / (KNOT * KNOT);
            curvature.y += second[j] * self.y[start + j] / (KNOT * KNOT);
        }
        (point, derivative, curvature)
    }

    fn span_length(&self, a: f64, b: f64) -> f64 {
        // Five-point Gauss-Legendre quadrature; the derivative is quadratic.
        const NODES: [f64; 5] = [
            -0.906_179_845_938_664,
            -0.538_469_310_105_683_1,
            0.0,
            0.538_469_310_105_683_1,
            0.906_179_845_938_664,
        ];
        const WEIGHTS: [f64; 5] = [
            0.236_926_885_056_189_1,
            0.478_628_670_499_366_5,
            0.568_888_888_888_888_9,
            0.478_628_670_499_366_5,
            0.236_926_885_056_189_1,
        ];
        let middle = 0.5 * (a + b);
        let half = 0.5 * (b - a);
        half * NODES
            .iter()
            .zip(WEIGHTS)
            .map(|(&node, weight)| {
                let (_, tangent, _) = self.evaluate(middle + half * node);
                weight * libm::hypot(tangent.x, tangent.y)
            })
            .sum::<f64>()
    }
}

/// Equally spaced points on a polyline, including its start, at one unit.
pub(crate) fn resample(points: &[Point]) -> Vec<Point> {
    let mut output = vec![points[0]];
    let mut cumulative = 0.0;
    let mut next = 1.0;
    for segment in points.windows(2) {
        let length = segment[0].distance(segment[1]);
        if length <= 1e-12 {
            continue;
        }
        while next <= cumulative + length {
            let fraction = (next - cumulative) / length;
            output.push(Point {
                x: segment[0].x + fraction * (segment[1].x - segment[0].x),
                y: segment[0].y + fraction * (segment[1].y - segment[0].y),
            });
            next += 1.0;
        }
        cumulative += length;
    }
    output
}

/// Fit at one-unit data spacing, then sample at a fixed physical arc step.
pub(crate) fn fit(points: &[Point], weights: &[f64], cutoff: f64, step: f64) -> Vec<Sample> {
    let spline = Spline::fit(points, weights, cutoff);
    let intervals = (spline.length / 0.05).ceil() as usize;
    let mut arcs = vec![0.0];
    let delta = spline.length / intervals as f64;
    for i in 0..intervals {
        let length = spline.span_length(i as f64 * delta, (i + 1) as f64 * delta);
        arcs.push(arcs[i] + length);
    }
    let total = arcs[intervals];
    let count = (total / step).floor() as usize + 1;
    let mut samples = Vec::with_capacity(count);
    let mut index = 0;
    for i in 0..count {
        let arc = i as f64 * step;
        while index + 1 < intervals && arcs[index + 1] < arc {
            index += 1;
        }
        let fraction = (arc - arcs[index]) / (arcs[index + 1] - arcs[index]).max(1e-12);
        let parameter = (index as f64 + fraction) * delta;
        let (point, derivative, second) = spline.evaluate(parameter);
        let speed = libm::hypot(derivative.x, derivative.y).max(1e-12);
        samples.push(Sample {
            arc,
            point,
            tangent: Point {
                x: derivative.x / speed,
                y: derivative.y / speed,
            },
            curvature: (second.x * derivative.y - second.y * derivative.x)
                / (speed * speed * speed),
        });
    }
    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_oblique_line_keeps_physical_arc_and_unit_tangents() {
        // A 3-4-5 direction has exactly unit tangent (0.6, 0.8).
        let points: Vec<_> = (0..201)
            .map(|s| Point {
                x: 3.0 + 0.6 * s as f64,
                y: 7.0 + 0.8 * s as f64,
            })
            .collect();
        let samples = fit(&points, &vec![1.0; points.len()], 50.0, 0.25);
        for sample in samples {
            assert!((sample.point.x - 3.0 - 0.6 * sample.arc).abs() < 1e-7);
            assert!((sample.point.y - 7.0 - 0.8 * sample.arc).abs() < 1e-7);
            assert!((sample.tangent.x - 0.6).abs() < 1e-8);
            assert!((sample.tangent.y - 0.8).abs() < 1e-8);
            assert!(sample.curvature.abs() < 1e-8);
        }
    }
}
