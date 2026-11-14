//! Bordered augmented-energy iterations used only to initialise a hard solve.
//!
//! The first six coordinates per body form a block-tridiagonal chain; the
//! remaining coordinates form its dense border. Constraint rows may couple
//! one body to the border, but must not add off-band body blocks. All scales
//! and caps here are project numerical choices. This routine returns a warm
//! state, never physical loads or a claim of constrained convergence. The
//! hard minimiser must still check every equality, stationarity and reduced
//! positive definiteness.
//!
//! Six multiplier updates allow eight accepted steps each. Each update uses
//! one fresh tissue Hessian, exact current first derivatives, multiplier
//! curvature and the Gauss--Newton penalty tangent. The actual augmented
//! energy supplies the line search; this tangent is only a preconditioner.
//! Scalar independent slide coordinates are removed by an exact Schur
//! complement before the existing bordered chain factorisation. The returned
//! state minimises the recorded approximate merit over the outer iterates;
//! its multiplier estimate belongs to that same iterate.

use super::minimise::{Evaluation, FirstOrder, Model};
use crate::chain_solver::BorderedSystem;
use nalgebra::{DMatrix, DVector, Matrix6};

/// Project work cap for multiplier updates.
const OUTER_LIMIT: u32 = 6;
/// Project work cap for each augmented-energy minimum.
const INNER_LIMIT: u32 = 8;
/// Project backtracking and damped-factor retry caps.
const LINE_LIMIT: u32 = 16;
const DAMPING_LIMIT: u32 = 12;
/// Project initial equality penalty in the caller's scaled units.
const INITIAL_PENALTY: f64 = 10_000.0;
/// Project upper penalty bound; the hard solve supplies the final precision.
const MAXIMUM_PENALTY: f64 = 1_000_000.0;
/// Project approximate stopping levels for a warm state only.
const WARM_CONSTRAINT: f64 = 1e-7;
const WARM_STATIONARITY: f64 = 1e-6;
/// Project augmented Armijo fraction and local deformation bound.
const ARMIJO: f64 = 1e-4;
const MAXIMUM_STEP: f64 = 0.5;
/// Project damping schedule and dimensionless warm-state merit weights.
const INITIAL_DAMPING: f64 = 1e-3;
const MINIMUM_DAMPING: f64 = 1e-10;
const MAXIMUM_DAMPING: f64 = 1e12;
const DAMPING_INCREASE: f64 = 10.0;
const DAMPING_DECREASE: f64 = 3.0;
const CONSTRAINT_MERIT_WEIGHT: f64 = 1000.0;
const REQUIRED_CONSTRAINT_PROGRESS: f64 = 0.4;
const PENALTY_INCREASE: f64 = 10.0;

/// Diagnostics describe the warm iterations, not a settled physical state.
#[derive(Clone, Debug)]
pub struct WarmStart<S> {
    pub state: S,
    pub iterations: u32,
    /// Multiplier estimate belonging to the returned best state.
    pub multipliers: DVector<f64>,
    pub constraint_max: f64,
    pub stationarity_max: f64,
}

fn largest(vector: &DVector<f64>) -> f64 {
    vector.iter().fold(0.0_f64, |a, b| a.max(b.abs()))
}

fn usable(e: &FirstOrder, dimension: usize) -> bool {
    e.energy.is_finite()
        && e.gradient.len() == dimension
        && e.jacobian.shape() == (e.constraint.len(), dimension)
        && e.gradient
            .iter()
            .chain(e.constraint.iter())
            .chain(e.jacobian.iter())
            .all(|v| v.is_finite())
}

fn augmented_value(
    energy: f64,
    constraint: &DVector<f64>,
    lambda: &DVector<f64>,
    penalty: f64,
) -> f64 {
    energy + lambda.dot(constraint) + 0.5 * penalty * constraint.norm_squared()
}

fn equations<M: Model>(
    model: &M,
    state: &M::State,
    e: &FirstOrder,
    tangent: &DMatrix<f64>,
    lambda: &DVector<f64>,
    penalty: f64,
) -> Option<(DMatrix<f64>, DVector<f64>)> {
    let weighted = lambda + &e.constraint * penalty;
    let data = Evaluation {
        energy: e.energy,
        gradient: e.gradient.clone(),
        hessian: tangent.clone(),
        constraint: e.constraint.clone(),
        jacobian: e.jacobian.clone(),
    };
    let mut matrix = model.lagrangian_hessian(state, lambda, &data)?;
    if matrix.shape() != (e.gradient.len(), e.gradient.len())
        || matrix.iter().any(|v| !v.is_finite())
    {
        return None;
    }
    let mut gradient = e.gradient.clone();
    let size = gradient.len();
    let rows = e.constraint.len();
    let jacobian = e.jacobian.as_slice();
    let matrix_values = matrix.as_mut_slice();
    let gradient_values = gradient.as_mut_slice();
    // Keep multiplier-weighted constraint curvature, but leave the penalty's
    // residual-weighted curvature out of this preliminary tangent. The exact
    // augmented gradient and actual augmented energy still govern each trial.
    // Head rows are sparse, preserving the chain's structural zero blocks.
    for row in 0..rows {
        let entries: Vec<_> = (0..size)
            .filter_map(|column| {
                let value = jacobian[row + column * rows];
                (value != 0.0).then_some((column, value))
            })
            .collect();
        for &(i, a) in &entries {
            gradient_values[i] += a * weighted[row];
            for &(j, b) in &entries {
                matrix_values[i + j * size] += penalty * a * b;
            }
        }
    }
    for column in 0..size {
        for row in 0..=column {
            let a = row + column * size;
            let b = column + row * size;
            let symmetric = (matrix_values[a] + matrix_values[b]) * 0.5;
            matrix_values[a] = symmetric;
            matrix_values[b] = symmetric;
        }
    }
    Some((matrix, gradient))
}

fn bordered(matrix: &DMatrix<f64>, body_blocks: usize) -> Option<BorderedSystem> {
    let chain_size = body_blocks * 6;
    let size = matrix.nrows();
    if matrix.ncols() != size || chain_size > size {
        return None;
    }
    let values = matrix.as_slice();
    let diagonal = (0..body_blocks)
        .map(|i| Matrix6::from_fn(|r, c| values[6 * i + r + (6 * i + c) * size]))
        .collect();
    let lower = (0..body_blocks.saturating_sub(1))
        .map(|i| Matrix6::from_fn(|r, c| values[6 * (i + 1) + r + (6 * i + c) * size]))
        .collect();
    // Do not silently discard a coupling that the caller's ordering promised
    // would be zero. A different graph needs a different factorisation.
    for i in 0..body_blocks {
        for j in 0..i.saturating_sub(1) {
            for r in 0..6 {
                for c in 0..6 {
                    if values[6 * i + r + (6 * j + c) * size] != 0.0 {
                        return None;
                    }
                }
            }
        }
    }
    Some(BorderedSystem {
        diagonal,
        lower,
        coupling: matrix
            .view((0, chain_size), (chain_size, size - chain_size))
            .into_owned(),
        border: matrix
            .view(
                (chain_size, chain_size),
                (size - chain_size, size - chain_size),
            )
            .into_owned(),
    })
}

/// Slide coordinates have a diagonal mutual block and touch only one body.
/// Eliminate them exactly before constructing the much smaller rod border.
fn step_from_border(
    matrix: &DMatrix<f64>,
    gradient: &DVector<f64>,
    body_blocks: usize,
    damping: f64,
) -> Option<DVector<f64>> {
    let size = matrix.nrows();
    let chain_size = body_blocks * 6;
    if matrix.ncols() != size || gradient.len() != size || chain_size > size {
        return None;
    }
    let mut damped = matrix.clone();
    let damped_values = damped.as_mut_slice();
    for i in 0..size {
        let diagonal = &mut damped_values[i * (size + 1)];
        *diagonal += damping * diagonal.abs().max(1.0);
    }
    let mut eliminated: Vec<usize> = Vec::new();
    for column in (chain_size..size).rev() {
        let support = (0..body_blocks)
            .filter(|&body| (0..6).any(|r| damped_values[body * 6 + r + column * size] != 0.0))
            .count();
        if support <= 1
            && eliminated
                .iter()
                .all(|&j| damped_values[column + j * size] == 0.0)
            && damped_values[column * (size + 1)] > 0.0
        {
            eliminated.push(column);
        }
    }
    let kept: Vec<_> = (0..size).filter(|i| !eliminated.contains(i)).collect();
    let reduced_size = kept.len();
    let mut reduced = DMatrix::zeros(reduced_size, reduced_size);
    let reduced_values = reduced.as_mut_slice();
    if reduced_size > 0
        && kept
            .iter()
            .enumerate()
            .all(|(i, &coordinate)| i == coordinate)
    {
        for (column, destination) in reduced_values.chunks_exact_mut(reduced_size).enumerate() {
            destination
                .copy_from_slice(&damped_values[column * size..column * size + reduced_size]);
        }
    } else if reduced_size > 0 {
        for (&column, destination) in kept
            .iter()
            .zip(reduced_values.chunks_exact_mut(reduced_size))
        {
            for (&row, value) in kept.iter().zip(destination) {
                *value = damped_values[row + column * size];
            }
        }
    }
    let mut rhs = DVector::from_iterator(kept.len(), kept.iter().map(|&i| -gradient[i]));
    let rhs_values = rhs.as_mut_slice();
    for &column in &eliminated {
        let inverse = 1.0 / damped_values[column * (size + 1)];
        let entries: Vec<_> = kept
            .iter()
            .enumerate()
            .filter_map(|(i, &row)| {
                let v = damped_values[row + column * size];
                (v != 0.0).then_some((i, v))
            })
            .collect();
        for &(i, a) in &entries {
            rhs_values[i] += a * inverse * gradient[column];
            for &(j, b) in &entries {
                reduced_values[i + j * reduced_size] -= a * inverse * b;
            }
        }
    }
    let system = bordered(&reduced, body_blocks)?;
    let solution = system.factor().ok()?.solve(&rhs).ok()?;
    let mut full = DVector::zeros(size);
    for (i, &coordinate) in kept.iter().enumerate() {
        full[coordinate] = solution[i];
    }
    for &column in &eliminated {
        let mut force = gradient[column];
        for &row in &kept {
            force += damped_values[column + row * size] * full[row];
        }
        full[column] = -force / damped_values[column * (size + 1)];
    }
    full.iter().all(|x| x.is_finite()).then_some(full)
}

/// Obtain a deterministic warm state. Invalid graphs or failed trials leave
/// the last usable state for the authoritative hard constrained solve.
pub fn warm_start<M: Model>(
    model: &M,
    initial: M::State,
    body_blocks: usize,
) -> WarmStart<M::State> {
    let dimension = model.dimension();
    let mut state = initial;
    let Some(first) = model.first_order(&state).filter(|e| usable(e, dimension)) else {
        return WarmStart {
            state,
            iterations: 0,
            multipliers: DVector::zeros(0),
            constraint_max: f64::INFINITY,
            stationarity_max: f64::INFINITY,
        };
    };
    let mut lambda = DVector::zeros(first.constraint.len());
    let mut penalty = INITIAL_PENALTY;
    let mut damping = INITIAL_DAMPING;
    let mut iterations = 0;
    let mut previous_constraint = f64::INFINITY;
    let mut best = state.clone();
    let mut best_score = f64::INFINITY;
    let mut best_multipliers = lambda.clone();
    let mut best_constraint = largest(&first.constraint);
    let mut best_stationarity = largest(&first.gradient);
    for _ in 0..OUTER_LIMIT {
        let Some(tangent) = model
            .evaluate(&state)
            .map(|e| e.hessian)
            .filter(|h| h.shape() == (dimension, dimension) && h.iter().all(|v| v.is_finite()))
        else {
            break;
        };
        for _ in 0..INNER_LIMIT {
            let Some(e) = model
                .first_order(&state)
                .filter(|e| usable(e, dimension) && e.constraint.len() == lambda.len())
            else {
                break;
            };
            let Some((matrix, gradient)) = equations(model, &state, &e, &tangent, &lambda, penalty)
            else {
                break;
            };
            let reference = largest(&e.gradient).max(1.0);
            if largest(&gradient) <= WARM_STATIONARITY * reference {
                break;
            }
            let energy = augmented_value(e.energy, &e.constraint, &lambda, penalty);
            let mut accepted = None;
            for _ in 0..DAMPING_LIMIT {
                if let Some(mut increment) =
                    step_from_border(&matrix, &gradient, body_blocks, damping)
                {
                    let measure = model.step_measure(&state, &increment);
                    if !measure.is_finite() {
                        break;
                    }
                    if measure > MAXIMUM_STEP {
                        increment *= MAXIMUM_STEP / measure;
                    }
                    let directional = gradient.dot(&increment);
                    if directional < 0.0 {
                        let mut length = 1.0;
                        for _ in 0..LINE_LIMIT {
                            if let Some(candidate) = model.step(&state, &(&increment * length))
                                && let Some((f, c)) = model.value(&candidate)
                                && c.len() == lambda.len()
                                && f.is_finite()
                                && c.iter().all(|x| x.is_finite())
                                && augmented_value(f, &c, &lambda, penalty)
                                    <= energy + ARMIJO * length * directional
                            {
                                accepted = Some(candidate);
                                break;
                            }
                            length *= 0.5;
                        }
                    }
                }
                if accepted.is_some() {
                    break;
                }
                damping = (damping * DAMPING_INCREASE).min(MAXIMUM_DAMPING);
            }
            let Some(candidate) = accepted else {
                break;
            };
            state = candidate;
            iterations += 1;
            damping = (damping / DAMPING_DECREASE).max(MINIMUM_DAMPING);
        }
        let Some(e) = model
            .first_order(&state)
            .filter(|e| usable(e, dimension) && e.constraint.len() == lambda.len())
        else {
            break;
        };
        lambda += &e.constraint * penalty;
        let stationarity = largest(&(&e.gradient + e.jacobian.transpose() * &lambda));
        let constraint = largest(&e.constraint);
        let score = (constraint * CONSTRAINT_MERIT_WEIGHT)
            .max(stationarity / largest(&e.gradient).max(1.0));
        if score < best_score {
            best_score = score;
            best = state.clone();
            best_multipliers = lambda.clone();
            best_constraint = constraint;
            best_stationarity = stationarity;
        }
        if constraint < WARM_CONSTRAINT
            && stationarity < WARM_STATIONARITY * largest(&e.gradient).max(1.0)
        {
            break;
        }
        if constraint > REQUIRED_CONSTRAINT_PROGRESS * previous_constraint {
            penalty = (penalty * PENALTY_INCREASE).min(MAXIMUM_PENALTY);
        }
        previous_constraint = constraint;
    }
    WarmStart {
        state: best,
        iterations,
        multipliers: best_multipliers,
        constraint_max: best_constraint,
        stationarity_max: best_stationarity,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Springs;
    impl Model for Springs {
        type State = DVector<f64>;
        fn dimension(&self) -> usize {
            13
        }
        fn evaluate(&self, x: &Self::State) -> Option<Evaluation> {
            let mut h = DMatrix::identity(13, 13);
            h[(0, 6)] = -0.2;
            h[(6, 0)] = -0.2;
            let mut j = DMatrix::zeros(2, 13);
            j[(0, 0)] = 1.0;
            j[(0, 12)] = -1.0;
            j[(1, 6)] = 1.0;
            j[(1, 12)] = -1.0;
            let c = &j * x - DVector::from_element(2, 1.0);
            Some(Evaluation {
                energy: 0.5 * x.dot(&(&h * x)),
                gradient: &h * x,
                hessian: h,
                constraint: c,
                jacobian: j,
            })
        }
        fn step(&self, x: &Self::State, p: &DVector<f64>) -> Option<Self::State> {
            Some(x + p)
        }
    }
    struct SlidingSprings;
    impl Model for SlidingSprings {
        type State = DVector<f64>;
        fn dimension(&self) -> usize {
            15
        }
        fn evaluate(&self, x: &Self::State) -> Option<Evaluation> {
            let mut h = DMatrix::identity(15, 15);
            h[(0, 6)] = -0.2;
            h[(6, 0)] = -0.2;
            let mut j = DMatrix::zeros(2, 15);
            j[(0, 0)] = 1.0;
            j[(0, 12)] = -1.0;
            j[(0, 13)] = -1.0;
            j[(1, 6)] = 1.0;
            j[(1, 12)] = -1.0;
            j[(1, 14)] = -1.0;
            Some(Evaluation {
                energy: 0.5 * x.dot(&(&h * x)),
                gradient: &h * x,
                hessian: h,
                constraint: &j * x - DVector::from_element(2, 1.0),
                jacobian: j,
            })
        }
        fn step(&self, x: &Self::State, p: &DVector<f64>) -> Option<Self::State> {
            Some(x + p)
        }
    }
    #[test]
    fn independent_slides_are_released_without_changing_the_spring_minimum() {
        // Symmetry gives x0=x6=a. The border and two slides share a-1
        // as r=2s, so E=.8a²+(a-1)²/3 and a=5/17.
        let result = warm_start(&SlidingSprings, DVector::zeros(15), 2);
        assert!((result.state[0] - 5.0 / 17.0).abs() < 1e-6);
        assert!((result.state[6] - 5.0 / 17.0).abs() < 1e-6);
        assert!((result.state[12] + 8.0 / 17.0).abs() < 1e-6);
        assert!((result.state[13] + 4.0 / 17.0).abs() < 1e-6);
        assert!((result.state[14] + 4.0 / 17.0).abs() < 1e-6);
        assert!(result.constraint_max < 1e-7);
    }

    #[test]
    fn a_bordered_spring_minimum_is_a_useful_warm_start() {
        // Two identical body coordinates take a; the free border coordinate
        // takes a-1. Energy=.8a²+.5(a-1)², so a=1/2.6.
        let result = warm_start(&Springs, DVector::zeros(13), 2);
        let a = 1.0 / 2.6;
        assert!((result.state[0] - a).abs() < 1e-6);
        assert!((result.state[6] - a).abs() < 1e-6);
        assert!((result.state[12] - (a - 1.0)).abs() < 1e-6);
        assert!(result.constraint_max < 1e-7);
    }
}
