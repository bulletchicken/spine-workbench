//! Fixed-seed three-point RANSAC, followed by a geometric circle fit.

use nalgebra::{Matrix3, Vector3};

#[derive(Clone, Copy)]
pub(super) struct Circle {
    pub x: f64,
    pub y: f64,
    pub radius: f64,
}

impl Circle {
    fn residual(self, point: [f64; 2]) -> f64 {
        libm::hypot(point[0] - self.x, point[1] - self.y) - self.radius
    }
}

/// At least half the 180 rays must support one rim. All draws and ties
/// have a fixed order so the native and browser fits choose the same rim.
pub(super) fn fit(points: &[[f64; 2]]) -> Option<(Circle, usize, f64)> {
    if points.len() < 90 {
        return None;
    }
    let mut seed = 42u32;
    let mut draw = || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        ((u64::from(seed) * points.len() as u64) >> 32) as usize
    };
    let mut best = None;
    let mut best_count = 0;
    let mut best_error = f64::INFINITY;
    for _ in 0..200 {
        let indices = [draw(), draw(), draw()];
        if indices[0] == indices[1] || indices[0] == indices[2] || indices[1] == indices[2] {
            continue;
        }
        let Some(circle) = through_three(indices.map(|index| points[index])) else {
            continue;
        };
        let mut count = 0;
        let mut error = 0.0;
        for point in points {
            let residual = circle.residual(*point);
            if residual.abs() <= 1.5 {
                count += 1;
                error += residual * residual;
            }
        }
        if count > best_count || (count == best_count && error < best_error) {
            best = Some(circle);
            best_count = count;
            best_error = error;
        }
    }
    let best = best?;
    let inliers: Vec<_> = points
        .iter()
        .copied()
        .filter(|point| best.residual(*point).abs() <= 1.5)
        .collect();
    if inliers.len() < 90 {
        return None;
    }
    let fitted = refine(kasa(&inliers)?, &inliers);
    let rms = (squared_error(fitted, &inliers) / inliers.len() as f64).sqrt();
    Some((fitted, inliers.len(), rms))
}

fn through_three([a, b, c]: [[f64; 2]; 3]) -> Option<Circle> {
    let (bx, by, cx, cy) = (b[0] - a[0], b[1] - a[1], c[0] - a[0], c[1] - a[1]);
    let determinant = 2.0 * (bx * cy - by * cx);
    if determinant.abs() < 1e-8 {
        return None;
    }
    let (b2, c2) = (bx * bx + by * by, cx * cx + cy * cy);
    let (x, y) = (
        (cy * b2 - by * c2) / determinant,
        (bx * c2 - cx * b2) / determinant,
    );
    let radius = libm::hypot(x, y);
    (radius.is_finite() && radius > 0.0).then_some(Circle {
        x: x + a[0],
        y: y + a[1],
        radius,
    })
}

fn kasa(points: &[[f64; 2]]) -> Option<Circle> {
    let mean_x = points.iter().map(|p| p[0]).sum::<f64>() / points.len() as f64;
    let mean_y = points.iter().map(|p| p[1]).sum::<f64>() / points.len() as f64;
    let mut matrix = Matrix3::zeros();
    let mut rhs = Vector3::zeros();
    for p in points {
        let (x, y) = (p[0] - mean_x, p[1] - mean_y);
        let row = Vector3::new(x, y, 1.0);
        matrix += row * row.transpose();
        rhs += row * (x * x + y * y);
    }
    let solution = matrix.lu().solve(&rhs)?;
    let (x, y) = (solution[0] / 2.0, solution[1] / 2.0);
    let radius = (solution[2] + x * x + y * y).sqrt();
    radius.is_finite().then_some(Circle {
        x: x + mean_x,
        y: y + mean_y,
        radius,
    })
}

fn squared_error(circle: Circle, points: &[[f64; 2]]) -> f64 {
    points
        .iter()
        .map(|point| {
            let r = circle.residual(*point);
            r * r
        })
        .sum()
}

fn refine(mut circle: Circle, points: &[[f64; 2]]) -> Circle {
    let mut damping = 0.001;
    for _ in 0..20 {
        let mut matrix = Matrix3::zeros();
        let mut gradient = Vector3::zeros();
        for point in points {
            let (dx, dy) = (circle.x - point[0], circle.y - point[1]);
            let distance = libm::hypot(dx, dy);
            if distance <= f64::EPSILON {
                continue;
            }
            let jacobian = Vector3::new(dx / distance, dy / distance, -1.0);
            matrix += jacobian * jacobian.transpose();
            gradient += jacobian * (distance - circle.radius);
        }
        for i in 0..3 {
            matrix[(i, i)] *= 1.0 + damping;
        }
        let Some(step) = matrix.lu().solve(&(-gradient)) else {
            break;
        };
        let candidate = Circle {
            x: circle.x + step[0],
            y: circle.y + step[1],
            radius: circle.radius + step[2],
        };
        if candidate.radius > 0.0
            && squared_error(candidate, points) < squared_error(circle, points)
        {
            circle = candidate;
            damping *= 0.1;
            if step.norm() < 1e-7 {
                break;
            }
        } else {
            damping *= 10.0;
        }
    }
    circle
}
