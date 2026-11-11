//! Matrix-free random walker (Grady, DOI 10.1109/TPAMI.2006.233).
//! Jacobi-preconditioned conjugate gradients use f32 vectors and f64 dot
//! products in row order; fixed iteration checkpoints make frames repeatable.

use super::local_edges::{Crop, Label};
use shapes::{
    errors::{EngineError, ErrorCode},
    watch_frames::PreviewTile,
};

pub(super) struct Solution {
    pub values: Vec<f32>,
    pub frames: Vec<PreviewTile>,
    pub iterations: u32,
    pub residual: f64,
}
struct Equation {
    pixel: usize,
    diagonal: f32,
    adjacent: [(usize, f32); 4],
    rhs: f32,
}
const SEED: usize = usize::MAX;
fn equations(crop: &Crop) -> Vec<Equation> {
    let mut indices = vec![SEED; crop.labels.len()];
    let mut count = 0;
    for (i, label) in crop.labels.iter().enumerate() {
        if *label == Label::Unknown {
            indices[i] = count;
            count += 1;
        }
    }
    let mut rows = Vec::with_capacity(count);
    for (i, label) in crop.labels.iter().enumerate() {
        if *label == Label::Unknown {
            let mut row = Equation {
                pixel: i,
                diagonal: 0.,
                adjacent: [(SEED, 0.); 4],
                rhs: 0.,
            };
            let x = i % crop.width;
            let y = i / crop.width;
            let neighbours = [
                (
                    x > 0,
                    i.wrapping_sub(1),
                    if x > 0 { crop.right[i - 1] } else { 0. },
                ),
                (x + 1 < crop.width, i + 1, crop.right[i]),
                (
                    y > 0,
                    i.wrapping_sub(crop.width),
                    if y > 0 { crop.down[i - crop.width] } else { 0. },
                ),
                (y + 1 < crop.height, i + crop.width, crop.down[i]),
            ];
            for (k, (exists, j, w)) in neighbours.into_iter().enumerate() {
                if exists {
                    row.diagonal += w;
                    row.adjacent[k] = (indices[j], w);
                    if crop.labels[j] == Label::Foreground {
                        row.rhs += w;
                    }
                }
            }
            rows.push(row);
        }
    }
    rows
}
fn multiply(rows: &[Equation], x: &[f32], result: &mut [f32]) {
    for (i, row) in rows.iter().enumerate() {
        let mut value = row.diagonal * x[i];
        for &(j, w) in &row.adjacent {
            if j != SEED {
                value -= w * x[j];
            }
        }
        result[i] = value;
    }
}
fn dot(a: &[f32], b: &[f32]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(&a, &b)| f64::from(a) * f64::from(b))
        .sum()
}
fn full(crop: &Crop, rows: &[Equation], x: &[f32]) -> Vec<f32> {
    let mut values: Vec<_> = crop
        .labels
        .iter()
        .map(|l| if *l == Label::Foreground { 1. } else { 0. })
        .collect();
    for (row, &value) in rows.iter().zip(x) {
        values[row.pixel] = value.clamp(0., 1.);
    }
    values
}
pub(super) fn solve(crop: &Crop) -> Result<Solution, EngineError> {
    solve_with(crop, 1e-3, 300)
}
fn solve_with(crop: &Crop, tolerance: f64, limit: u32) -> Result<Solution, EngineError> {
    let rows = equations(crop);
    let n = rows.len();
    let b: Vec<_> = rows.iter().map(|r| r.rhs).collect();
    let norm = dot(&b, &b).sqrt();
    let mut x: Vec<_> = rows
        .iter()
        .map(|r| if crop.old[r.pixel] { 1. } else { 0. })
        .collect();
    if norm == 0. {
        x.fill(0.);
        let values = full(crop, &rows, &x);
        return Ok(Solution {
            frames: vec![crop.tile(&values)],
            values,
            iterations: 0,
            residual: 0.,
        });
    }
    let mut ax = vec![0.; n];
    multiply(&rows, &x, &mut ax);
    let mut residual: Vec<_> = b.iter().zip(&ax).map(|(b, a)| b - a).collect();
    let mut z: Vec<_> = rows
        .iter()
        .zip(&residual)
        .map(|(row, r)| r / row.diagonal)
        .collect();
    let mut direction = z.clone();
    let mut rz = dot(&residual, &z);
    let mut iterations = 0;
    let mut frames = Vec::new();
    while dot(&residual, &residual).sqrt() > tolerance * norm && iterations < limit {
        multiply(&rows, &direction, &mut ax);
        let denominator = dot(&direction, &ax);
        if denominator <= 0. || !denominator.is_finite() {
            break;
        }
        let alpha = (rz / denominator) as f32;
        for i in 0..n {
            x[i] += alpha * direction[i];
            residual[i] -= alpha * ax[i];
            z[i] = residual[i] / rows[i].diagonal;
        }
        let next = dot(&residual, &z);
        let beta = (next / rz) as f32;
        for i in 0..n {
            direction[i] = z[i] + beta * direction[i];
        }
        rz = next;
        iterations += 1;
        if [8, 24, 64].contains(&iterations) {
            frames.push(crop.tile(&full(crop, &rows, &x)));
        }
    }
    // Recompute the true residual: recursive f32 residuals can drift.
    multiply(&rows, &x, &mut ax);
    for i in 0..n {
        residual[i] = b[i] - ax[i];
    }
    let relative = dot(&residual, &residual).sqrt() / norm;
    if !relative.is_finite() || relative > 1e-2 {
        return Err(EngineError::new(
            ErrorCode::ToolDidNotConverge,
            "the random walker residual is above 0.01",
        ));
    }
    let values = full(crop, &rows, &x);
    frames.push(crop.tile(&values));
    Ok(Solution {
        values,
        frames,
        iterations,
        residual: relative,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use shapes::pixels::Px;
    #[test]
    fn a_uniform_chain_has_linear_hitting_probabilities() {
        // A path with endpoints 1 and 0 has exact harmonic solution 1-i/10.
        let mut labels = vec![Label::Unknown; 11];
        labels[0] = Label::Foreground;
        labels[10] = Label::Background;
        let crop = Crop {
            width: 11,
            height: 1,
            origin: Px::new(0., 0.),
            factor: 1.,
            old: vec![false; 11],
            free: vec![true; 11],
            labels,
            right: vec![1.; 11],
            down: vec![0.; 11],
            warnings: vec![],
        };
        let answer = solve_with(&crop, 1e-6, 300).unwrap();
        for (i, x) in answer.values.iter().enumerate() {
            assert!(
                (f64::from(*x) - (1. - i as f64 / 10.)).abs() < 2e-6,
                "pixel {i}"
            );
        }
    }
    #[test]
    fn conductance_ratio_sets_the_single_unknown_probability() {
        // Kirchhoff's law: 3(x-1)+1(x-0)=0, hence x=3/4.
        let crop = Crop {
            width: 3,
            height: 1,
            origin: Px::new(0., 0.),
            factor: 1.,
            old: vec![false; 3],
            free: vec![true; 3],
            labels: vec![Label::Foreground, Label::Unknown, Label::Background],
            right: vec![3., 1., 0.],
            down: vec![0.; 3],
            warnings: vec![],
        };
        assert_eq!(solve(&crop).unwrap().values, [1., 0.75, 0.]);
    }
}
