//! Deterministic damped normal equations, assembled from small residual blocks.

use crate::problem::Problem;
use crate::residuals::Row;
use crate::{
    FitError, gauge, parameters, priors, residuals, stacking, twist_readings, typical_rows,
};
use nalgebra::{DMatrix, DVector};
use shapes::spine_fit::FitStopReason;

pub struct Normal {
    pub matrix: DMatrix<f64>,
    pub gradient: DVector<f64>,
    pub cost: f64,
    pub residual_count: usize,
}

pub struct Solution {
    pub numbers: DVector<f64>,
    pub normal: Normal,
    pub iterations: u32,
    pub stop_reason: FitStopReason,
}

fn all_rows(problem: &Problem, numbers: &[f64], differentiate: bool) -> Option<Vec<Row>> {
    if !parameters::usable(numbers) {
        return None;
    }
    let mut rows = residuals::rows(problem, numbers, differentiate)?;
    rows.extend(priors::rows(problem, numbers, differentiate));
    rows.extend(typical_rows::rows(problem, numbers, differentiate));
    rows.extend(stacking::rows(problem, numbers, differentiate));
    rows.extend(twist_readings::rows(problem, numbers, differentiate));
    rows.extend(gauge::rows(problem, numbers, differentiate));
    rows.iter()
        .all(|r| r.value.is_finite() && r.derivatives.iter().all(|x| x.1.is_finite()))
        .then_some(rows)
}

pub fn normal(problem: &Problem, numbers: &[f64]) -> Result<Normal, FitError> {
    let rows = all_rows(problem, numbers, true).ok_or(FitError::NumericalFailure)?;
    let mut result = Normal {
        matrix: DMatrix::zeros(numbers.len(), numbers.len()),
        gradient: DVector::zeros(numbers.len()),
        cost: 0.0,
        residual_count: rows.len(),
    };
    for row in rows {
        result.cost += row.value * row.value;
        for &(i, a) in &row.derivatives {
            result.gradient[i] += a * row.value;
            for &(j, b) in &row.derivatives {
                result.matrix[(i, j)] += a * b;
            }
        }
    }
    Ok(result)
}

fn cost(problem: &Problem, numbers: &[f64]) -> Option<f64> {
    Some(
        all_rows(problem, numbers, false)?
            .iter()
            .map(|r| r.value * r.value)
            .sum(),
    )
}

pub fn solve(problem: &Problem, mut numbers: DVector<f64>) -> Result<Solution, FitError> {
    let mut equations = normal(problem, numbers.as_slice())?;
    let mut damping = 1e-3;
    for iteration in 1..=100 {
        let mut matrix = equations.matrix.clone();
        for i in 0..numbers.len() {
            matrix[(i, i)] += damping * equations.matrix[(i, i)].max(1e-12);
        }
        let Some(cholesky) = matrix.cholesky() else {
            damping *= 10.0;
            continue;
        };
        let step = cholesky.solve(&(-&equations.gradient));
        let candidate = &numbers + &step;
        let candidate_cost = cost(problem, candidate.as_slice());
        if let Some(new_cost) = candidate_cost.filter(|value| *value <= equations.cost) {
            let relative = (equations.cost - new_cost) / equations.cost.max(f64::MIN_POSITIVE);
            let small = step.norm() < 1e-8;
            numbers = candidate;
            equations = normal(problem, numbers.as_slice())?;
            if small || relative < 1e-10 {
                return Ok(Solution {
                    numbers,
                    normal: equations,
                    iterations: iteration,
                    stop_reason: if small {
                        FitStopReason::SmallStep
                    } else {
                        FitStopReason::RelativeCost
                    },
                });
            }
            damping = (damping / 3.0).max(1e-12);
        } else {
            // A rejected step never establishes convergence; only a step
            // accepted against the actual stacked objective may stop early.
            damping = (damping * 10.0).min(1e20);
        }
    }
    Ok(Solution {
        numbers,
        normal: equations,
        iterations: 100,
        stop_reason: FitStopReason::IterationLimit,
    })
}
