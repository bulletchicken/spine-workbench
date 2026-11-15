//! Equality-constrained energy minimisation in caller-scaled local coordinates.
//!
//! Each iteration reveals independent constraints by bounded rook-pivot
//! elimination in scaled coordinates. A complete search checks a small
//! candidate against the entire remaining block, and supplies the fallback
//! when rook search reaches its cap. Only the smaller null basis is then
//! twice orthonormalised in contiguous columns. A particular feasibility
//! step and that null space reduce the quadratic model to a positive-definite
//! solve. Consistent redundant rows are retained in the reported residual;
//! they are not mistaken for a mechanism. At a stationary point the actual
//! reduced Lagrangian Hessian must be positive definite without damping.
//!
//! Trial damping uses identity in the twice-orthonormalised null basis and bounds the
//! main increment in the caller's coordinates. It adds no physical springs;
//! the final reduced stiffness is always undamped.
//!
//! All tolerances and caps below are this project's numerical choices.
//! Coordinates, constraint rows and energy must be scaled by the caller.
//! `gradient + jacobian.transpose() * multipliers = 0` fixes the multiplier
//! sign. Redundant rows receive a deterministic representative multiplier;
//! this is not a minimum-multiplier-norm choice and does not select the
//! separate least-rod-strain-energy force set.

use crate::chain_solver::CholeskyFactor;
use nalgebra::{DMatrix, DVector};

/// Energy and derivatives in the coordinates accepted by [`Model::step`].
#[derive(Clone, Debug)]
pub struct Evaluation {
    pub energy: f64,
    pub gradient: DVector<f64>,
    pub hessian: DMatrix<f64>,
    pub constraint: DVector<f64>,
    pub jacobian: DMatrix<f64>,
}

/// Exact objective and constraint first derivatives, without assembling a
/// tangent. Preliminary augmented iterations may reuse a separate tangent.
pub struct FirstOrder {
    pub energy: f64,
    pub gradient: DVector<f64>,
    pub constraint: DVector<f64>,
    pub jacobian: DMatrix<f64>,
}

/// An energy and equality constraints on an arbitrary retractable state.
pub trait Model {
    type State: Clone;

    fn dimension(&self) -> usize;
    fn evaluate(&self, state: &Self::State) -> Option<Evaluation>;
    fn step(&self, state: &Self::State, increment: &DVector<f64>) -> Option<Self::State>;

    /// Largest individual objective term in the gradient's scaled coordinates,
    /// before balanced contributions cancel. The default adds no scale.
    fn balanced_term_scale(&self, _state: &Self::State) -> Option<f64> {
        Some(0.0)
    }

    fn first_order(&self, state: &Self::State) -> Option<FirstOrder> {
        let e = self.evaluate(state)?;
        Some(FirstOrder {
            energy: e.energy,
            gradient: e.gradient,
            constraint: e.constraint,
            jacobian: e.jacobian,
        })
    }

    /// Trust measure in caller-scaled deformation coordinates. Models may
    /// omit harmless frame translation while bounding relative strains.
    fn step_measure(&self, _state: &Self::State, increment: &DVector<f64>) -> f64 {
        max_abs(increment)
    }

    /// Override when feasibility projection can avoid energy derivatives.
    fn constraint_data(&self, state: &Self::State) -> Option<(DVector<f64>, DMatrix<f64>)> {
        let evaluation = self.evaluate(state)?;
        Some((evaluation.constraint, evaluation.jacobian))
    }

    /// Override when line-search values can avoid derivative assembly.
    fn value(&self, state: &Self::State) -> Option<(f64, DVector<f64>)> {
        let value = self.evaluate(state)?;
        Some((value.energy, value.constraint))
    }

    /// The true local Hessian of energy plus multiplier-weighted constraints.
    /// Linear constraints need only the energy Hessian supplied in evaluate.
    fn lagrangian_hessian(
        &self,
        _state: &Self::State,
        _multipliers: &DVector<f64>,
        evaluation: &Evaluation,
    ) -> Option<DMatrix<f64>> {
        Some(evaluation.hessian.clone())
    }
}

/// Fixed work limits and nondimensional convergence criteria.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub iteration_limit: u32,
    pub line_search_limit: u32,
    pub damping_limit: u32,
    pub constraint_tolerance: f64,
    pub stationarity_tolerance: f64,
    pub absolute_stationarity_tolerance: f64,
    pub rank_tolerance: f64,
    pub maximum_step: f64,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            iteration_limit: 80,
            line_search_limit: 24,
            damping_limit: 14,
            constraint_tolerance: 2e-12,
            stationarity_tolerance: 2e-11,
            absolute_stationarity_tolerance: 1e-12,
            rank_tolerance: 1e-11,
            maximum_step: 0.5,
        }
    }
}

/// Why a solve finished. A work cap is never reported as settled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    Converged,
    IterationLimit,
    LineSearch,
    Inconsistent,
    Singular,
    Invalid,
}

/// Last usable state, explicit convergence information and multiplier signs.
#[derive(Clone, Debug)]
pub struct Solution<S> {
    pub state: S,
    pub multipliers: DVector<f64>,
    pub settled: bool,
    pub reason: Reason,
    pub iterations: u32,
    pub constraint_max: f64,
    pub stationarity_max: f64,
    pub rank: usize,
}

/// Project work cap for alternating row/column pivot maxima. The complete
/// fallback still certifies rank when this inexpensive search stalls.
const ROOK_SEARCH_LIMIT: usize = 16;

struct RowSpace {
    dimension: usize,
    row_count: usize,
    factor: Vec<f64>,
    permutation: Vec<usize>,
    variables: Vec<usize>,
    selected: Vec<usize>,
    null: DMatrix<f64>,
}

impl RowSpace {
    fn new(jacobian: &DMatrix<f64>, tolerance: f64) -> Option<Self> {
        Self::factor(jacobian, tolerance, true)
    }

    fn factor(jacobian: &DMatrix<f64>, tolerance: f64, build_null: bool) -> Option<Self> {
        let rows = jacobian.nrows();
        let columns = jacobian.ncols();
        let mut factor = Vec::with_capacity(rows * columns);
        for row in 0..rows {
            for column in 0..columns {
                factor.push(jacobian[(row, column)]);
            }
        }
        let largest = factor.iter().fold(0.0_f64, |a, b| a.max(b.abs()));
        let mut permutation: Vec<_> = (0..rows).collect();
        let mut variables: Vec<_> = (0..columns).collect();
        let mut rank = 0;
        for pivot in 0..rows.min(columns) {
            let mut pivot_row = pivot;
            let mut pivot_column = pivot;
            let mut magnitude = 0.0;
            let mut rook_settled = false;
            for _ in 0..ROOK_SEARCH_LIMIT {
                let offset = pivot_row * columns;
                let mut row_magnitude = 0.0;
                for (column, &entry) in factor[offset + pivot..offset + columns].iter().enumerate()
                {
                    let value = entry.abs();
                    if value > row_magnitude {
                        row_magnitude = value;
                        pivot_column = column + pivot;
                    }
                }
                let mut next_row = pivot_row;
                let mut column_magnitude = 0.0;
                for row in pivot..rows {
                    let value = factor[row * columns + pivot_column].abs();
                    if value > column_magnitude {
                        column_magnitude = value;
                        next_row = row;
                    }
                }
                magnitude = row_magnitude;
                if next_row == pivot_row {
                    rook_settled = true;
                    break;
                }
                pivot_row = next_row;
            }
            if !rook_settled || magnitude <= tolerance * largest {
                magnitude = 0.0;
                for row in pivot..rows {
                    let offset = row * columns;
                    for (column, &entry) in
                        factor[offset + pivot..offset + columns].iter().enumerate()
                    {
                        let value = entry.abs();
                        if value > magnitude {
                            magnitude = value;
                            pivot_row = row;
                            pivot_column = column + pivot;
                        }
                    }
                }
            }
            if magnitude <= tolerance * largest {
                break;
            }
            if !magnitude.is_finite() {
                return None;
            }
            if pivot_row != pivot {
                for column in 0..columns {
                    factor.swap(pivot * columns + column, pivot_row * columns + column);
                }
                permutation.swap(pivot, pivot_row);
            }
            if pivot_column != pivot {
                for row in 0..rows {
                    factor.swap(row * columns + pivot, row * columns + pivot_column);
                }
                variables.swap(pivot, pivot_column);
            }
            let diagonal = factor[pivot * columns + pivot];
            let (earlier, later) = factor.split_at_mut((pivot + 1) * columns);
            let pivot_values = &earlier[pivot * columns..(pivot + 1) * columns];
            for values in later.chunks_exact_mut(columns) {
                let coefficient = values[pivot] / diagonal;
                values[pivot] = coefficient;
                if coefficient != 0.0 {
                    for (value, &source) in values[pivot + 1..]
                        .iter_mut()
                        .zip(&pivot_values[pivot + 1..])
                    {
                        *value -= coefficient * source;
                    }
                }
            }
            rank += 1;
        }
        let selected = permutation[..rank].to_vec();
        let mut null = DMatrix::zeros(columns, if build_null { columns - rank } else { 0 });
        for free in rank..if build_null { columns } else { rank } {
            let mut coordinates = vec![0.0; rank];
            for row in (0..rank).rev() {
                let mut value = -factor[row * columns + free];
                let (entries, entry_tail) =
                    factor[row * columns + row + 1..row * columns + rank].as_chunks::<4>();
                let (coordinate_chunks, coordinate_tail) = coordinates[row + 1..].as_chunks::<4>();
                for (a, b) in entries.iter().zip(coordinate_chunks) {
                    value -= a[0] * b[0];
                    value -= a[1] * b[1];
                    value -= a[2] * b[2];
                    value -= a[3] * b[3];
                }
                for (&entry, &coordinate) in entry_tail.iter().zip(coordinate_tail) {
                    value -= entry * coordinate;
                }
                coordinates[row] = value / factor[row * columns + row];
            }
            null[(variables[free], free - rank)] = 1.0;
            for row in 0..rank {
                null[(variables[row], free - rank)] = coordinates[row];
            }
        }
        if !orthonormalise_columns(&mut null) {
            return None;
        }
        null.iter().all(|x| x.is_finite()).then_some(Self {
            dimension: columns,
            row_count: rows,
            factor,
            permutation,
            variables,
            selected,
            null,
        })
    }

    fn particular(&self, constraint: &DVector<f64>) -> Option<DVector<f64>> {
        let rank = self.selected.len();
        let mut work: Vec<_> = self
            .permutation
            .iter()
            .map(|&row| -constraint[row])
            .collect();
        for row in 0..self.row_count {
            let mut value = work[row];
            for (column, &coordinate) in work.iter().enumerate().take(row.min(rank)) {
                value -= self.factor[row * self.dimension + column] * coordinate;
            }
            work[row] = value;
        }
        for row in (0..rank).rev() {
            let mut value = work[row];
            for (column, &coordinate) in work.iter().enumerate().take(rank).skip(row + 1) {
                value -= self.factor[row * self.dimension + column] * coordinate;
            }
            work[row] = value / self.factor[row * self.dimension + row];
        }
        let mut result = DVector::zeros(self.dimension);
        for row in 0..rank {
            result[self.variables[row]] = work[row];
        }
        result.iter().all(|x| x.is_finite()).then_some(result)
    }

    fn multipliers(&self, gradient: &DVector<f64>, row_count: usize) -> DVector<f64> {
        let rank = self.selected.len();
        let mut work = vec![0.0; rank];
        for row in 0..rank {
            let mut value = -gradient[self.variables[row]];
            for (column, &coordinate) in work.iter().enumerate().take(row) {
                value -= self.factor[column * self.dimension + row] * coordinate;
            }
            work[row] = value / self.factor[row * self.dimension + row];
        }
        for row in (0..rank).rev() {
            let mut value = work[row];
            for (column, &coordinate) in work.iter().enumerate().take(rank).skip(row + 1) {
                value -= self.factor[column * self.dimension + row] * coordinate;
            }
            work[row] = value;
        }
        let mut result = DVector::zeros(row_count);
        for (row, &selected) in self.selected.iter().enumerate() {
            result[selected] = work[row];
        }
        result
    }
}

/// Twice-orthogonalise only the small null basis, in contiguous columns.
/// The elimination's free-coordinate identity guarantees independence.
fn orthonormalise_columns(matrix: &mut DMatrix<f64>) -> bool {
    let dimension = matrix.nrows();
    let count = matrix.ncols();
    let data = matrix.as_mut_slice();
    for column in 0..count {
        let (earlier, remaining) = data.split_at_mut(column * dimension);
        let current = &mut remaining[..dimension];
        for _ in 0..2 {
            for previous in earlier.chunks_exact(dimension) {
                let mut dot = 0.0;
                let (left, left_tail) = current.as_chunks::<4>();
                let (right, right_tail) = previous.as_chunks::<4>();
                for (a, b) in left.iter().zip(right) {
                    dot += a[0] * b[0];
                    dot += a[1] * b[1];
                    dot += a[2] * b[2];
                    dot += a[3] * b[3];
                }
                for (&a, &b) in left_tail.iter().zip(right_tail) {
                    dot += a * b;
                }
                let (left, left_tail) = current.as_chunks_mut::<4>();
                for (a, b) in left.iter_mut().zip(right) {
                    a[0] -= dot * b[0];
                    a[1] -= dot * b[1];
                    a[2] -= dot * b[2];
                    a[3] -= dot * b[3];
                }
                for (a, &b) in left_tail.iter_mut().zip(right_tail) {
                    *a -= dot * b;
                }
            }
        }
        let mut squared = 0.0;
        let (chunks, tail) = current.as_chunks::<4>();
        for a in chunks {
            squared += a[0] * a[0];
            squared += a[1] * a[1];
            squared += a[2] * a[2];
            squared += a[3] * a[3];
        }
        for &a in tail {
            squared += a * a;
        }
        let norm = squared.sqrt();
        if !norm.is_finite() || norm <= 0.0 {
            return false;
        }
        let (chunks, tail) = current.as_chunks_mut::<4>();
        for a in chunks {
            a[0] /= norm;
            a[1] /= norm;
            a[2] /= norm;
            a[3] /= norm;
        }
        for a in tail {
            *a /= norm;
        }
    }
    true
}

fn max_abs(vector: &DVector<f64>) -> f64 {
    vector
        .iter()
        .fold(0.0_f64, |largest, value| largest.max(value.abs()))
}

/// Project a symmetric tangent without multiplying its structural zeroes.
/// Joint-coordinate energy is diagonal; body-coordinate chain tangents are
/// sparse. Both cases retain exactly the same quadratic form.
fn projected_hessian(hessian: &DMatrix<f64>, null: &DMatrix<f64>) -> DMatrix<f64> {
    let n = hessian.nrows();
    let q = null.ncols();
    let mut entries = Vec::new();
    let mut diagonal = true;
    for j in 0..n {
        for i in 0..n {
            let value = hessian[(i, j)];
            if value != 0.0 {
                entries.push((i, j, value));
                diagonal &= i == j;
            }
        }
    }
    if diagonal {
        let weights: Vec<_> = (0..n).map(|i| hessian[(i, i)]).collect();
        let mut result = DMatrix::zeros(q, q);
        for j in 0..q {
            let b = &null.as_slice()[j * n..(j + 1) * n];
            for i in 0..=j {
                let a = &null.as_slice()[i * n..(i + 1) * n];
                let mut value = 0.0;
                for k in 0..n {
                    value += weights[k] * a[k] * b[k];
                }
                result[(i, j)] = value;
                result[(j, i)] = value;
            }
        }
        return result;
    }
    if entries.len() * 2 <= n * n {
        let mut product = vec![0.0; n * q];
        for k in 0..q {
            let column = &null.as_slice()[k * n..(k + 1) * n];
            let target = &mut product[k * n..(k + 1) * n];
            for &(i, j, value) in &entries {
                target[i] += value * column[j];
            }
        }
        let mut result = DMatrix::zeros(q, q);
        for j in 0..q {
            let b = &product[j * n..(j + 1) * n];
            for i in 0..=j {
                let a = &null.as_slice()[i * n..(i + 1) * n];
                let mut value = 0.0;
                for k in 0..n {
                    value += a[k] * b[k];
                }
                result[(i, j)] = value;
                result[(j, i)] = value;
            }
        }
        return result;
    }
    let result = null.transpose() * hessian * null;
    (&result + result.transpose()) * 0.5
}

fn finite_evaluation(evaluation: &Evaluation, dimension: usize) -> bool {
    evaluation.energy.is_finite()
        && evaluation.gradient.len() == dimension
        && evaluation.hessian.shape() == (dimension, dimension)
        && evaluation.jacobian.shape() == (evaluation.constraint.len(), dimension)
        && evaluation
            .gradient
            .iter()
            .chain(evaluation.hessian.iter())
            .chain(evaluation.constraint.iter())
            .chain(evaluation.jacobian.iter())
            .all(|x| x.is_finite())
}

fn finish<S>(
    state: S,
    multipliers: DVector<f64>,
    reason: Reason,
    iterations: u32,
    constraint_max: f64,
    stationarity_max: f64,
    rank: usize,
) -> Solution<S> {
    Solution {
        state,
        multipliers,
        settled: reason == Reason::Converged,
        reason,
        iterations,
        constraint_max,
        stationarity_max,
        rank,
    }
}

fn next_damping(value: f64) -> f64 {
    if value == 0.0 { 1e-8 } else { value * 10.0 }
}

/// Project numerical cap for restoring a trial to its curved constraints.
const FEASIBILITY_PROJECTION_STEPS: u32 = 6;
/// Project choice limiting residual-based polishing to small deformations.
const POLISH_STEP_LIMIT: f64 = 1e-4;
/// Project allowance for floating merit comparisons within feasible rows.
const MERIT_ROUNDING_MULTIPLIER: f64 = 64.0;

struct MeritBound {
    penalty: f64,
    value: f64,
    correct: bool,
    rank_tolerance: f64,
    constraint_tolerance: f64,
    stationarity_max: f64,
}

fn usable_value(value: &(f64, DVector<f64>), constraint_count: usize) -> bool {
    value.0.is_finite()
        && value.1.len() == constraint_count
        && value.1.iter().all(|x| x.is_finite())
}

fn polishing_candidate<M: Model>(
    model: &M,
    candidate: &M::State,
    measure: f64,
    energy: f64,
    constraint: &DVector<f64>,
    bound: &MeritBound,
) -> bool {
    if measure > POLISH_STEP_LIMIT || max_abs(constraint) > bound.constraint_tolerance {
        return false;
    }
    let tolerance = MERIT_ROUNDING_MULTIPLIER * f64::EPSILON * energy.abs().max(1.0)
        + bound.penalty * constraint.len() as f64 * bound.constraint_tolerance;
    if energy + bound.penalty * constraint.abs().sum() > bound.value + tolerance {
        return false;
    }
    let Some(evaluation) = model
        .evaluate(candidate)
        .filter(|e| finite_evaluation(e, model.dimension()))
    else {
        return false;
    };
    let Some(rows) = RowSpace::factor(&evaluation.jacobian, bound.rank_tolerance, false) else {
        return false;
    };
    let multipliers = rows.multipliers(&evaluation.gradient, evaluation.constraint.len());
    let residual = &evaluation.gradient + evaluation.jacobian.transpose() * multipliers;
    max_abs(&residual) < 0.5 * bound.stationarity_max
}

fn trial<M: Model>(
    model: &M,
    state: &M::State,
    increment: &DVector<f64>,
    _rows: &RowSpace,
    constraint_count: usize,
    bound: MeritBound,
) -> Option<M::State> {
    let step_measure = model.step_measure(state, increment);
    if !step_measure.is_finite() || step_measure < 0.0 {
        return None;
    }
    let mut candidate = model.step(state, increment)?;
    let (mut energy, mut constraint) = model
        .value(&candidate)
        .filter(|v| usable_value(v, constraint_count))?;
    if energy + bound.penalty * constraint.abs().sum() <= bound.value
        || polishing_candidate(model, &candidate, step_measure, energy, &constraint, &bound)
    {
        return Some(candidate);
    }
    if !bound.correct || max_abs(&constraint) <= bound.constraint_tolerance {
        return None;
    }
    // A new Jacobian at the trial restores its curved feasible manifold;
    // reusing the old Newton-point Jacobian leaves a third-order merit error.
    let (_, jacobian) = model.constraint_data(&candidate)?;
    if jacobian.shape() != (constraint_count, increment.len())
        || jacobian.iter().any(|x| !x.is_finite())
    {
        return None;
    }
    let mut rows = RowSpace::new(&jacobian, bound.rank_tolerance)?;
    let mut correction_sum = 0.0;
    for iteration in 0..FEASIBILITY_PROJECTION_STEPS {
        if iteration == FEASIBILITY_PROJECTION_STEPS / 2 {
            let (_, jacobian) = model.constraint_data(&candidate)?;
            if jacobian.shape() != (constraint_count, increment.len())
                || jacobian.iter().any(|x| !x.is_finite())
            {
                return None;
            }
            rows = RowSpace::new(&jacobian, bound.rank_tolerance)?;
        }
        let mut correction = rows.particular(&constraint)?;
        correction -= &rows.null * (rows.null.transpose() * &correction);
        correction_sum += model.step_measure(&candidate, &correction);
        if !correction_sum.is_finite() || correction_sum > step_measure {
            return None;
        }
        candidate = model.step(&candidate, &correction)?;
        (energy, constraint) = model
            .value(&candidate)
            .filter(|v| usable_value(v, constraint_count))?;
        if energy + bound.penalty * constraint.abs().sum() <= bound.value
            || polishing_candidate(model, &candidate, step_measure, energy, &constraint, &bound)
        {
            return Some(candidate);
        }
        if max_abs(&constraint) <= bound.constraint_tolerance {
            return None;
        }
    }
    None
}

/// Minimise from the supplied initial state. No initial state is guessed.
pub fn minimise<M: Model>(model: &M, initial: M::State, options: Options) -> Solution<M::State> {
    let dimension = model.dimension();
    let mut state = initial;
    let mut last_multipliers = DVector::zeros(0);
    let mut last_constraint = f64::INFINITY;
    let mut last_stationarity = f64::INFINITY;
    let mut last_rank = 0;
    if dimension == 0
        || options.iteration_limit == 0
        || options.line_search_limit == 0
        || options.damping_limit == 0
        || [
            options.constraint_tolerance,
            options.stationarity_tolerance,
            options.absolute_stationarity_tolerance,
            options.rank_tolerance,
            options.maximum_step,
        ]
        .iter()
        .any(|v| !v.is_finite() || *v <= 0.0)
        || options.rank_tolerance >= 1.0
    {
        return finish(
            state,
            last_multipliers,
            Reason::Invalid,
            0,
            last_constraint,
            last_stationarity,
            last_rank,
        );
    }
    for iteration in 0..=options.iteration_limit {
        let Some((evaluation, balanced_term_scale)) = model
            .evaluate(&state)
            .and_then(|e| model.balanced_term_scale(&state).map(|scale| (e, scale)))
            .filter(|(e, scale)| {
                finite_evaluation(e, dimension) && scale.is_finite() && *scale >= 0.0
            })
        else {
            return finish(
                state,
                last_multipliers,
                Reason::Invalid,
                iteration,
                last_constraint,
                last_stationarity,
                last_rank,
            );
        };
        let Some(rows) = RowSpace::new(&evaluation.jacobian, options.rank_tolerance) else {
            return finish(
                state,
                last_multipliers,
                Reason::Invalid,
                iteration,
                last_constraint,
                last_stationarity,
                last_rank,
            );
        };
        let multipliers = rows.multipliers(&evaluation.gradient, evaluation.constraint.len());
        let constraint_force = evaluation.jacobian.transpose() * &multipliers;
        let stationarity = &evaluation.gradient + &constraint_force;
        last_constraint = max_abs(&evaluation.constraint);
        last_stationarity = max_abs(&stationarity);
        last_rank = rows.selected.len();
        last_multipliers = multipliers.clone();
        let scale = max_abs(&evaluation.gradient)
            .max(max_abs(&constraint_force))
            .max(balanced_term_scale);
        let stationary = last_stationarity
            <= options.absolute_stationarity_tolerance + options.stationarity_tolerance * scale;
        let feasible = last_constraint <= options.constraint_tolerance;
        let Some(hessian) = model
            .lagrangian_hessian(&state, &multipliers, &evaluation)
            .filter(|h| h.shape() == (dimension, dimension) && h.iter().all(|v| v.is_finite()))
        else {
            return finish(
                state,
                multipliers,
                Reason::Invalid,
                iteration,
                last_constraint,
                last_stationarity,
                last_rank,
            );
        };
        let hessian = (&hessian + hessian.transpose()) * 0.5;
        let reduced = projected_hessian(&hessian, &rows.null);
        if feasible && stationary {
            let reason = if CholeskyFactor::new(&reduced).is_ok() {
                Reason::Converged
            } else {
                Reason::Singular
            };
            return finish(
                state,
                multipliers,
                reason,
                iteration,
                last_constraint,
                last_stationarity,
                last_rank,
            );
        }
        if iteration == options.iteration_limit {
            break;
        }
        let Some(mut particular) = rows.particular(&evaluation.constraint) else {
            return finish(
                state,
                multipliers,
                Reason::Invalid,
                iteration,
                last_constraint,
                last_stationarity,
                last_rank,
            );
        };
        // Two-pass orthonormalisation makes the null-coordinate metric I.
        // Remove round-off in the normal correction with two projections;
        // no dense Gram factorisation is needed for this orthonormal basis.
        for _ in 0..2 {
            particular -= &rows.null * (rows.null.transpose() * &particular);
        }
        let normal_projection = rows.null.transpose() * &particular;
        let incompatibility = &evaluation.jacobian * &particular + &evaluation.constraint;
        // A curved redundant equality need not have a consistent linearisation
        // away from its feasible manifold. Restore the selected independent
        // rows first; refuse excluded residuals only if restoration stalls.
        let incompatible = max_abs(&incompatibility)
            > 20.0 * options.constraint_tolerance + 1e-8 * last_constraint;
        let reduced_rhs = -rows.null.transpose() * (&evaluation.gradient + &hessian * &particular);
        let damping_scale = hessian
            .iter()
            .fold(0.0_f64, |a, b| a.max(b.abs()))
            .max(1e-12);
        let mut accepted = None;
        let mut damping = 0.0;
        for _ in 0..options.damping_limit {
            let regularisation = damping * damping_scale;
            let mut damped = reduced.clone();
            for i in 0..damped.nrows() {
                damped[(i, i)] += regularisation;
            }
            let damped_rhs = &reduced_rhs - &normal_projection * regularisation;
            if let Ok(factor) = CholeskyFactor::new(&damped)
                && let Ok(tangent_step) = factor.solve(&damped_rhs)
            {
                let mut increment = &particular + &rows.null * tangent_step;
                let largest = model.step_measure(&state, &increment);
                if largest > options.maximum_step
                    && model.step_measure(&state, &particular) <= 0.8 * options.maximum_step
                {
                    // A weak mode must not scale down every stiff direction.
                    // Raise intrinsic damping until the full local step fits
                    // the trust bound. No trial stiffness enters final loads.
                    damping = next_damping(damping);
                    continue;
                }
                if largest > options.maximum_step {
                    // A large normal correction cannot fit the local bound
                    // in one iteration; reduce its feasibility gap in stages.
                    increment *= options.maximum_step / largest;
                }
                let qp_multipliers = rows.multipliers(
                    &(&evaluation.gradient + &hessian * &increment),
                    evaluation.constraint.len(),
                );
                let direction_energy = evaluation.gradient.dot(&increment);
                let constraint_increment = &evaluation.jacobian * &increment;
                let direction_constraint: f64 = evaluation
                    .constraint
                    .iter()
                    .zip(constraint_increment.iter())
                    .map(|(&c, &d)| {
                        if c > 0.0 {
                            d
                        } else if c < 0.0 {
                            -d
                        } else {
                            d.abs()
                        }
                    })
                    .sum();
                let mut penalty = (1.5 * max_abs(&qp_multipliers)).max(1.0);
                if direction_energy + penalty * direction_constraint >= 0.0
                    && direction_constraint < 0.0
                {
                    penalty = penalty
                        .max(1.5 * (direction_energy.abs() + 1e-12) / (-direction_constraint));
                }
                let directional = direction_energy + penalty * direction_constraint;
                if directional < 0.0 {
                    let merit = evaluation.energy + penalty * evaluation.constraint.abs().sum();
                    let mut length = 1.0;
                    for _ in 0..options.line_search_limit {
                        let local = &increment * length;
                        let bound = MeritBound {
                            penalty,
                            value: merit + 1e-4 * length * directional,
                            correct: length >= 0.25,
                            rank_tolerance: options.rank_tolerance,
                            constraint_tolerance: options.constraint_tolerance,
                            stationarity_max: last_stationarity,
                        };
                        if let Some(candidate) = trial(
                            model,
                            &state,
                            &local,
                            &rows,
                            evaluation.constraint.len(),
                            bound,
                        ) {
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
            damping = next_damping(damping);
        }
        if let Some(candidate) = accepted {
            state = candidate;
        } else {
            return finish(
                state,
                multipliers,
                if incompatible {
                    Reason::Inconsistent
                } else {
                    Reason::LineSearch
                },
                iteration + 1,
                last_constraint,
                last_stationarity,
                last_rank,
            );
        }
    }
    finish(
        state,
        last_multipliers,
        Reason::IterationLimit,
        options.iteration_limit,
        last_constraint,
        last_stationarity,
        last_rank,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Circle;
    impl Model for Circle {
        type State = DVector<f64>;
        fn dimension(&self) -> usize {
            2
        }
        fn evaluate(&self, x: &Self::State) -> Option<Evaluation> {
            Some(Evaluation {
                energy: 0.5 * ((x[0] - 2.0) * (x[0] - 2.0) + (x[1] - 1.0) * (x[1] - 1.0)),
                gradient: DVector::from_column_slice(&[x[0] - 2.0, x[1] - 1.0]),
                hessian: DMatrix::identity(2, 2),
                constraint: DVector::from_element(1, x.dot(x) - 1.0),
                jacobian: DMatrix::from_row_slice(1, 2, &[2.0 * x[0], 2.0 * x[1]]),
            })
        }
        fn step(&self, x: &Self::State, d: &DVector<f64>) -> Option<Self::State> {
            Some(x + d)
        }
        fn lagrangian_hessian(
            &self,
            _: &Self::State,
            lambda: &DVector<f64>,
            _: &Evaluation,
        ) -> Option<DMatrix<f64>> {
            Some(DMatrix::identity(2, 2) * (1.0 + 2.0 * lambda[0]))
        }
    }

    #[test]
    fn a_curved_constraint_reaches_its_closed_form_minimum() {
        let result = minimise(
            &Circle,
            DVector::from_column_slice(&[0.2, 0.8]),
            Options::default(),
        );
        assert!(
            result.settled,
            "{:?}, {} {}",
            result.reason, result.constraint_max, result.stationarity_max
        );
        let expected = DVector::from_column_slice(&[2.0 / 5.0_f64.sqrt(), 1.0 / 5.0_f64.sqrt()]);
        assert!((&result.state - expected).norm() < 1e-11);
        assert!(result.constraint_max < 2e-12);
    }

    struct Quadratic {
        singular: bool,
        inconsistent: bool,
    }
    impl Model for Quadratic {
        type State = DVector<f64>;
        fn dimension(&self) -> usize {
            2
        }
        fn evaluate(&self, x: &Self::State) -> Option<Evaluation> {
            Some(Evaluation {
                energy: if self.singular {
                    0.5 * x[0] * x[0]
                } else {
                    0.5 * x.dot(x)
                },
                gradient: if self.singular {
                    DVector::from_column_slice(&[x[0], 0.0])
                } else {
                    x.clone()
                },
                hessian: DMatrix::from_diagonal(&DVector::from_column_slice(&[
                    1.0,
                    if self.singular { 0.0 } else { 1.0 },
                ])),
                constraint: DVector::from_column_slice(&[
                    x[0] - 1.0,
                    2.0 * x[0] - if self.inconsistent { 3.0 } else { 2.0 },
                ]),
                jacobian: DMatrix::from_row_slice(2, 2, &[1.0, 0.0, 2.0, 0.0]),
            })
        }
        fn step(&self, x: &Self::State, d: &DVector<f64>) -> Option<Self::State> {
            Some(x + d)
        }
    }

    #[test]
    fn dependent_rows_do_not_make_a_regular_minimum_singular() {
        let result = minimise(
            &Quadratic {
                singular: false,
                inconsistent: false,
            },
            DVector::from_column_slice(&[0.0, 0.5]),
            Options::default(),
        );
        assert!(result.settled, "{:?}", result.reason);
        assert_eq!(result.rank, 1);
        assert!((result.state - DVector::from_column_slice(&[1.0, 0.0])).norm() < 1e-12);
    }

    #[test]
    fn an_actual_reduced_mechanism_and_inconsistent_equalities_are_refused() {
        let singular = minimise(
            &Quadratic {
                singular: true,
                inconsistent: false,
            },
            DVector::from_column_slice(&[1.0, 0.0]),
            Options::default(),
        );
        assert_eq!(singular.reason, Reason::Singular);
        assert!(!singular.settled);
        let inconsistent = minimise(
            &Quadratic {
                singular: false,
                inconsistent: true,
            },
            DVector::zeros(2),
            Options::default(),
        );
        assert_eq!(inconsistent.reason, Reason::Inconsistent);
        assert!(!inconsistent.settled);
    }

    #[test]
    fn a_stationary_maximum_is_not_reported_as_settled() {
        let direction = DVector::from_column_slice(&[-2.0 / 5.0_f64.sqrt(), -1.0 / 5.0_f64.sqrt()]);
        let result = minimise(&Circle, direction, Options::default());
        assert_eq!(result.reason, Reason::Singular);
        assert!(!result.settled);
    }

    #[test]
    fn reaching_a_fixed_work_cap_is_not_convergence() {
        let options = Options {
            iteration_limit: 1,
            maximum_step: 0.01,
            ..Options::default()
        };
        let result = minimise(&Circle, DVector::from_column_slice(&[0.2, 0.8]), options);
        assert_eq!(result.reason, Reason::IterationLimit);
        assert!(!result.settled);
        assert_eq!(result.iterations, 1);
    }

    #[test]
    fn pivoted_mixed_constraints_keep_their_known_energy_minimum() {
        struct Mixed;
        impl Model for Mixed {
            type State = DVector<f64>;
            fn dimension(&self) -> usize {
                4
            }
            fn evaluate(&self, x: &Self::State) -> Option<Evaluation> {
                let diagonal = DVector::from_column_slice(&[1.0, 2.0, 3.0, 4.0]);
                let hessian = DMatrix::from_diagonal(&diagonal);
                let jacobian = DMatrix::from_row_slice(
                    3,
                    4,
                    &[1.0, 2.0, 0.0, 1.0, 0.0, 1.0, 1.0, -1.0, 1.0, 3.0, 1.0, 0.0],
                );
                let answer = DVector::from_column_slice(&[0.2, -0.1, 0.3, 0.4]);
                let dual = DVector::from_column_slice(&[0.3, -0.2, 0.0]);
                let rest = &answer + (jacobian.transpose() * dual).component_div(&diagonal);
                let displacement = x - rest;
                let gradient = &hessian * &displacement;
                Some(Evaluation {
                    energy: 0.5 * displacement.dot(&gradient),
                    gradient,
                    hessian,
                    constraint: &jacobian * (x - answer),
                    jacobian,
                })
            }
            fn step(&self, x: &Self::State, d: &DVector<f64>) -> Option<Self::State> {
                Some(x + d)
            }
        }
        let result = minimise(&Mixed, DVector::zeros(4), Options::default());
        assert!(
            result.settled,
            "{:?}, {} {}",
            result.reason, result.constraint_max, result.stationarity_max
        );
        assert_eq!(result.rank, 2);
        assert!((result.state - DVector::from_column_slice(&[0.2, -0.1, 0.3, 0.4])).norm() < 1e-11);
    }

    #[test]
    fn curved_redundant_rows_can_restore_a_feasible_manifold() {
        struct RedundantCircle;
        impl Model for RedundantCircle {
            type State = DVector<f64>;
            fn dimension(&self) -> usize {
                2
            }
            fn evaluate(&self, x: &Self::State) -> Option<Evaluation> {
                let c = x.dot(x) - 1.0;
                Some(Evaluation {
                    energy: 0.5 * ((x[0] - 2.0) * (x[0] - 2.0) + (x[1] - 1.0) * (x[1] - 1.0)),
                    gradient: DVector::from_column_slice(&[x[0] - 2.0, x[1] - 1.0]),
                    hessian: DMatrix::identity(2, 2),
                    constraint: DVector::from_column_slice(&[c, 2.0 * c + c * c]),
                    jacobian: DMatrix::from_row_slice(
                        2,
                        2,
                        &[
                            2.0 * x[0],
                            2.0 * x[1],
                            4.0 * (1.0 + c) * x[0],
                            4.0 * (1.0 + c) * x[1],
                        ],
                    ),
                })
            }
            fn step(&self, x: &Self::State, d: &DVector<f64>) -> Option<Self::State> {
                Some(x + d)
            }
            fn lagrangian_hessian(
                &self,
                x: &Self::State,
                lambda: &DVector<f64>,
                _: &Evaluation,
            ) -> Option<DMatrix<f64>> {
                let c = x.dot(x) - 1.0;
                Some(
                    DMatrix::identity(2, 2) * (1.0 + 2.0 * lambda[0] + 4.0 * (1.0 + c) * lambda[1])
                        + x * x.transpose() * (8.0 * lambda[1]),
                )
            }
        }
        let result = minimise(
            &RedundantCircle,
            DVector::from_column_slice(&[0.2, 0.8]),
            Options::default(),
        );
        assert!(
            result.settled,
            "{:?}, {} {}",
            result.reason, result.constraint_max, result.stationarity_max
        );
        assert_eq!(result.rank, 1);
        let expected = DVector::from_column_slice(&[2.0 / 5.0_f64.sqrt(), 1.0 / 5.0_f64.sqrt()]);
        assert!((result.state - expected).norm() < 1e-11);
    }

    #[test]
    fn a_weak_mode_does_not_clip_every_stiff_direction() {
        struct Weak;
        impl Model for Weak {
            type State = DVector<f64>;
            fn dimension(&self) -> usize {
                2
            }
            fn evaluate(&self, x: &Self::State) -> Option<Evaluation> {
                let d = x - DVector::from_column_slice(&[1.0, 100.0]);
                let hessian = DMatrix::from_diagonal(&DVector::from_column_slice(&[1.0, 1e-10]));
                let gradient = &hessian * &d;
                Some(Evaluation {
                    energy: 0.5 * d.dot(&gradient),
                    gradient,
                    hessian,
                    constraint: DVector::zeros(0),
                    jacobian: DMatrix::zeros(0, 2),
                })
            }
            fn step(&self, x: &Self::State, d: &DVector<f64>) -> Option<Self::State> {
                Some(x + d)
            }
        }
        let result = minimise(
            &Weak,
            DVector::zeros(2),
            Options {
                iteration_limit: 1,
                ..Options::default()
            },
        );
        assert_eq!(result.reason, Reason::IterationLimit);
        assert!(result.state[0] >= 0.4, "{}", result.state[0]);
        assert!(result.state[1].abs() <= 0.5);
    }

    #[test]
    fn multiplier_estimates_balance_selected_independent_coordinates() {
        let jacobian = DMatrix::from_row_slice(2, 2, &[1.0, 3.0, 2.0, 6.0]);
        let rows = RowSpace::new(&jacobian, 1e-11).expect("row space");
        let gradient = DVector::from_column_slice(&[2.0, -1.0]);
        let lambda = rows.multipliers(&gradient, 2);
        assert_eq!(lambda[0], 0.0);
        assert!((lambda[1] - 1.0 / 6.0).abs() < 1e-14);
        let residual = &gradient + jacobian.transpose() * lambda;
        assert!(residual[1].abs() < 1e-14);
        assert!((rows.null.transpose() * rows.null - DMatrix::identity(1, 1)).norm() < 1e-14);
    }

    #[test]
    fn an_unbalanced_free_direction_remains_in_the_full_kkt_residual() {
        // J fixes dx + 3dy. The admissible unit direction (3,-1)/sqrt(10)
        // has work 7/sqrt(10) under g=(2,-1), irrespective of multipliers.
        // Balancing a selected coordinate therefore cannot imply convergence.
        let jacobian = DMatrix::from_row_slice(1, 2, &[1.0, 3.0]);
        let rows = RowSpace::new(&jacobian, 1e-11).expect("row space");
        let gradient = DVector::from_column_slice(&[2.0, -1.0]);
        let lambda = rows.multipliers(&gradient, 1);
        let residual = &gradient + jacobian.transpose() * lambda;
        let tangent = DVector::from_column_slice(&[3.0, -1.0]) / 10.0_f64.sqrt();
        assert!((tangent.dot(&residual) - 7.0 / 10.0_f64.sqrt()).abs() < 1e-14);
        assert!(max_abs(&residual) > 1.0);
    }

    #[test]
    fn floating_merit_noise_does_not_prevent_stationarity_polishing() {
        struct FineCircle;
        impl Model for FineCircle {
            type State = DVector<f64>;
            fn dimension(&self) -> usize {
                2
            }
            fn evaluate(&self, x: &Self::State) -> Option<Evaluation> {
                let angle = x[0] / 1e-8;
                Some(Evaluation {
                    energy: 1.0 + 0.5 * x.dot(x),
                    gradient: x.clone(),
                    hessian: DMatrix::identity(2, 2),
                    constraint: DVector::from_element(1, x[1] + 1e-15 * libm::sin(angle)),
                    jacobian: DMatrix::from_row_slice(1, 2, &[1e-7 * libm::cos(angle), 1.0]),
                })
            }
            fn step(&self, x: &Self::State, d: &DVector<f64>) -> Option<Self::State> {
                Some(x + d)
            }
        }
        let start = DVector::from_column_slice(&[1e-8, -1e-15 * libm::sin(1.0)]);
        let result = minimise(&FineCircle, start, Options::default());
        assert!(
            result.settled,
            "{:?}, {}",
            result.reason, result.stationarity_max
        );
        assert!(result.iterations <= 3, "{}", result.iterations);
        assert!(result.state[0].abs() < 1e-12);
    }

    #[test]
    fn projecting_diagonal_sparse_and_dense_quadratic_forms_keeps_their_work() {
        // Columns (1,1,0) and (0,1,1). With D=(2,3,5), their
        // self-works are 5 and 8, and their mutual work is 3.
        let basis = DMatrix::from_row_slice(3, 2, &[1.0, 0.0, 1.0, 1.0, 0.0, 1.0]);
        let diagonal = DMatrix::from_diagonal(&DVector::from_column_slice(&[2.0, 3.0, 5.0]));
        let expected = DMatrix::from_row_slice(2, 2, &[5.0, 3.0, 3.0, 8.0]);
        assert_eq!(projected_hessian(&diagonal, &basis), expected);
        // A separate sparse four-coordinate form has energy
        // x0² + 2x1² + 3x2² + 4x3² + x0*x3. Substituting
        // x=(a,b,a,b) gives 4a²+6b²+a*b.
        let mut sparse = DMatrix::from_diagonal(&DVector::from_column_slice(&[2.0, 4.0, 6.0, 8.0]));
        sparse[(0, 3)] = 1.0;
        sparse[(3, 0)] = 1.0;
        let basis = DMatrix::from_row_slice(4, 2, &[1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0]);
        assert_eq!(
            projected_hessian(&sparse, &basis),
            DMatrix::from_row_slice(2, 2, &[8.0, 1.0, 1.0, 12.0])
        );
        // Dense H=I+vvᵀ, v=(1,2,3,4): substitution adds
        // (4a+6b)² to 2a²+2b².
        let v = DVector::from_column_slice(&[1.0, 2.0, 3.0, 4.0]);
        let dense = DMatrix::identity(4, 4) + &v * v.transpose();
        assert_eq!(
            projected_hessian(&dense, &basis),
            DMatrix::from_row_slice(2, 2, &[18.0, 24.0, 24.0, 38.0])
        );
    }
}

#[cfg(test)]
mod balanced_term_tests;
