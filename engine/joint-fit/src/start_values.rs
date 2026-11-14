//! Initial poses from corner midpoints, endplate slopes, pedicle offsets,
//! twist readings and, with no side film, the typical spine.

use crate::parameters::{COUNT, SIZES};
use crate::problem::{Problem, millimetres};
use crate::typical_spine::Typical;
use forward_model::cameras::parallel::ParallelCamera;
use forward_model::rotation::{rot_x, rot_y, rot_z, rotation_vector_from_matrix};
use nalgebra::{DVector, Vector2};
use shapes::marks::Mark;

struct View {
    centre: Vector2<f64>,
    height: Vector2<f64>,
    half_sizes: [f64; 2],
    slopes: [f64; 2],
}

fn view(camera: &ParallelCamera, corners: [Mark; 4]) -> View {
    let [a, b, c, d] = corners.map(|m| millimetres(camera, m));
    let top = (a + b) * 0.5;
    let bottom = (c + d) * 0.5;
    View {
        centre: (top + bottom) * 0.5,
        height: top - bottom,
        half_sizes: [(a - b).norm() * 0.5, (d - c).norm() * 0.5],
        slopes: [
            libm::atan2(a.y - b.y, a.x - b.x),
            libm::atan2(d.y - c.y, d.x - c.x),
        ],
    }
}

pub fn start(problem: &Problem) -> DVector<f64> {
    let n = problem.observations.len();
    let fronts: Vec<_> = problem
        .observations
        .iter()
        .map(|v| v.front.map(|m| view(&problem.front, m.corners())))
        .collect();
    let mut sides: Vec<_> = problem
        .observations
        .iter()
        .map(|v| {
            let camera = problem.side.as_ref()?;
            v.side.map(|m| view(camera, m.corners()))
        })
        .collect();
    // Shared centres register the films before fitting poses. Starting at
    // equal pixel spacing can send a noisy pair into a slow local solve.
    let shared: Vec<_> = fronts
        .iter()
        .zip(&sides)
        .filter_map(|(f, s)| Some((f.as_ref()?.centre.y, s.as_ref()?.centre.y)))
        .collect();
    let count = shared.len().max(1) as f64;
    let front_mean = shared.iter().map(|p| p.0).sum::<f64>() / count;
    let side_mean = shared.iter().map(|p| p.1).sum::<f64>() / count;
    let variance = shared
        .iter()
        .map(|p| (p.1 - side_mean) * (p.1 - side_mean))
        .sum::<f64>();
    let ratio = shared
        .iter()
        .map(|p| (p.0 - front_mean) * (p.1 - side_mean))
        .sum::<f64>()
        / variance;
    let ratio = if ratio.is_finite() && ratio > 0.0 && libm::log(ratio).abs() < 14.0 {
        ratio
    } else {
        1.0
    };
    let offset = if shared.is_empty() {
        0.0
    } else {
        front_mean - ratio * side_mean
    };
    for view in sides.iter_mut().flatten() {
        view.centre *= ratio;
        view.height *= ratio;
        view.half_sizes = view.half_sizes.map(|s| s * ratio);
    }
    // Until the rotation matrices are composed, slots 3..6 are named angles.
    let mut poses = vec![[0.0; COUNT]; n];
    for i in 0..n {
        let p = &mut poses[i];
        let f = fronts[i].as_ref();
        let s = sides[i].as_ref();
        if let Some(f) = f {
            p[1] = f.centre.x;
            p[2] = f.centre.y;
            p[3] = (f.slopes[0] + f.slopes[1]) * 0.5;
            p[7] = f.slopes[0] - f.slopes[1];
            p[9] = f.half_sizes[0].max(1e-3);
            p[11] = f.half_sizes[1].max(1e-3);
        }
        if let Some(s) = s {
            p[0] = s.centre.x;
            if f.is_none() {
                p[2] = s.centre.y + offset;
            }
            p[4] = -(s.slopes[0] + s.slopes[1]) * 0.5;
            p[8] = s.slopes[1] - s.slopes[0];
            p[10] = s.half_sizes[0].max(1e-3);
            p[12] = s.half_sizes[1].max(1e-3);
        }
        p[6] = match (f, s) {
            (Some(f), Some(s)) => libm::sqrt(
                f.height.x * f.height.x
                    + s.height.x * s.height.x
                    + 0.25 * (f.height.y + s.height.y) * (f.height.y + s.height.y),
            ),
            (Some(v), None) | (None, Some(v)) => v.height.norm(),
            _ => unreachable!("a level has at least one view"),
        }
        .max(1e-3);
    }
    for fields in [[0, 4, 10, 12], [1, 3, 9, 11]] {
        let seen: Vec<_> = if fields[0] == 0 {
            sides.iter().map(Option::is_some).collect()
        } else {
            fronts.iter().map(Option::is_some).collect()
        };
        for i in 0..n {
            if !seen[i] {
                for field in fields {
                    poses[i][field] = interpolate(&poses, &seen, i, field);
                }
            }
        }
    }
    if let Some(typical) = &problem.typical
        && sides.iter().all(Option::is_none)
    {
        typical_side_view(&mut poses, typical);
    }
    for (i, observation) in problem.observations.iter().enumerate() {
        if let Some(front) = observation.front {
            let p = &mut poses[i];
            let mut sum = 0.0;
            let mut count = 0;
            for (pedicle, sign) in [(front.ped_l, 1.0), (front.ped_r, -1.0)] {
                if let Some(mark) = pedicle.mark() {
                    let y = millimetres(&problem.front, mark).x;
                    sum += y - p[1] - sign * 0.78 * 0.5 * (p[9] + p[11]) * libm::cos(p[3])
                        + 0.18 * p[6] * libm::sin(p[3]);
                    count += 1;
                }
            }
            if count > 0 {
                p[5] = libm::asin(
                    (-sum / f64::from(count) / (1.25 * 0.5 * (p[10] + p[12]))).clamp(-0.8, 0.8),
                );
            }
        }
        if let Some(reading) = observation.twist_reading {
            poses[i][5] = reading.twist_rad;
        }
    }
    let seen: Vec<_> = problem
        .observations
        .iter()
        .map(|v| v.has_pedicle() || v.twist_reading.is_some())
        .collect();
    for i in 0..n {
        if !seen[i] {
            poses[i][5] = interpolate(&poses, &seen, i, 5);
        }
    }
    let mut values = DVector::zeros(n * COUNT + 2);
    for (i, p) in poses.iter_mut().enumerate() {
        let r = rotation_vector_from_matrix(&(rot_x(p[3]) * rot_y(p[4]) * rot_z(p[5])));
        p[3..6].copy_from_slice(r.as_slice());
        for j in SIZES {
            p[j] = libm::log(p[j]);
        }
        values.as_mut_slice()[i * COUNT..(i + 1) * COUNT].copy_from_slice(p);
    }
    values[n * COUNT] = libm::log(ratio);
    values[n * COUNT + 1] = offset;
    values
}

/// With no side view anywhere: typical lean, side wedge and depths, the
/// height the projected one implies at that lean, and front-to-back places
/// stacked up from the lowest vertebra.
fn typical_side_view(poses: &mut [[f64; COUNT]], typical: &Typical) {
    for (p, level) in poses.iter_mut().zip(&typical.levels) {
        p[4] = level.lean_rad.mean;
        p[8] = level.side_wedge_rad.mean;
        p[6] /= libm::cos(p[4]);
        let ratio = libm::exp(level.log_depth_to_width.mean);
        p[10] = ratio * p[9];
        p[12] = ratio * p[11];
    }
    for i in (0..poses.len().saturating_sub(1)).rev() {
        let rise = poses[i][2] - poses[i + 1][2];
        let lean = 0.5 * (poses[i][4] + poses[i + 1][4]);
        poses[i][0] = poses[i + 1][0] + rise * libm::tan(lean);
    }
}

fn interpolate(poses: &[[f64; COUNT]], seen: &[bool], i: usize, field: usize) -> f64 {
    let above = (0..i).rev().find(|&j| seen[j]);
    let below = (i + 1..poses.len()).find(|&j| seen[j]);
    match (above, below) {
        (Some(a), Some(b)) => {
            poses[a][field] + (poses[b][field] - poses[a][field]) * (i - a) as f64 / (b - a) as f64
        }
        (Some(a), None) | (None, Some(a)) => poses[a][field],
        _ => 0.0,
    }
}
