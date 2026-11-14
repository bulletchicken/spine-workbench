//! A six-freedom chain with a dense border, factored without filling the
//! chain's zero blocks. Unknowns and loads retain the caller's units and
//! order: six per vertebra, then the border. `lower[i]` is block `(i+1,i)`.
//! The chain is eliminated first; only its Schur complement is dense.
//!
//! No damping is hidden here. A non-positive pivot is a failure, so the
//! nonlinear caller decides whether a damped retry is appropriate. The
//! separate symmetric spectrum exposes null directions rather than turning
//! a mechanism into a small spring.

use nalgebra::{DMatrix, DVector, Matrix6, linalg::SymmetricEigen};

/// The project's relative round-off threshold for Cholesky pivots.
const PIVOT_TOLERANCE: f64 = 1e-14;
/// The project's symmetry check, relative to the largest matrix entry.
const SYMMETRY_TOLERANCE: f64 = 1e-12;
/// The project's fixed cap for the small dense spectral decomposition.
const SPECTRUM_ITERATION_LIMIT: usize = 8192;

/// A linear system was unusable; no partial solution is returned.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum LinearError {
    #[error("linear-dimensions")]
    Dimensions,
    #[error("linear-non-finite")]
    NonFinite,
    #[error("linear-not-symmetric")]
    NotSymmetric,
    #[error("linear-not-positive-definite")]
    NotPositiveDefinite,
    #[error("linear-spectrum-iteration-limit")]
    SpectrumIterationLimit,
    #[error("linear-inconsistent")]
    Inconsistent,
}

/// Symmetric block tridiagonal chain and its dense border.
#[derive(Clone, Debug)]
pub struct BorderedSystem {
    pub diagonal: Vec<Matrix6<f64>>,
    pub lower: Vec<Matrix6<f64>>,
    /// Chain rows by border columns.
    pub coupling: DMatrix<f64>,
    pub border: DMatrix<f64>,
}

/// Reusable factorisation; a solve does not refactor the matrix.
#[derive(Clone, Debug)]
pub struct BorderedFactor {
    diagonal: Vec<Matrix6<f64>>,
    lower: Vec<Matrix6<f64>>,
    coupling: DMatrix<f64>,
    chain_response: DMatrix<f64>,
    border: CholeskyFactor,
}

impl BorderedSystem {
    /// Factor an SPD system. Empty chain or border blocks are allowed.
    pub fn factor(&self) -> Result<BorderedFactor, LinearError> {
        let count = self.diagonal.len();
        let chain_size = 6 * count;
        let border_size = self.border.nrows();
        if self.lower.len() != count.saturating_sub(1)
            || self.coupling.shape() != (chain_size, border_size)
            || self.border.ncols() != border_size
        {
            return Err(LinearError::Dimensions);
        }
        if !self.coupling.iter().all(|x| x.is_finite())
            || !self
                .lower
                .iter()
                .flat_map(|m| m.iter())
                .all(|x| x.is_finite())
        {
            return Err(LinearError::NonFinite);
        }
        check_symmetric(&self.border)?;
        let mut diagonal: Vec<Matrix6<f64>> = Vec::with_capacity(count);
        let mut lower: Vec<Matrix6<f64>> = Vec::with_capacity(count.saturating_sub(1));
        for i in 0..count {
            let mut block = self.diagonal[i];
            check_symmetric(&DMatrix::from_column_slice(6, 6, block.as_slice()))?;
            if i > 0 {
                // H L_previous^T = A_(i,i-1), solved a row at a time.
                let mut h = self.lower[i - 1];
                for row in 0..6 {
                    for column in 0..6 {
                        let mut value = h[(row, column)];
                        for k in 0..column {
                            value -= diagonal[i - 1][(column, k)] * h[(row, k)];
                        }
                        h[(row, column)] = value / diagonal[i - 1][(column, column)];
                    }
                }
                block -= h * h.transpose();
                lower.push(h);
            }
            let factor = CholeskyFactor::new(&DMatrix::from_column_slice(6, 6, block.as_slice()))?;
            diagonal.push(Matrix6::from_row_slice(&factor.lower));
        }
        let chain_response = solve_chain(&diagonal, &lower, &self.coupling);
        let schur = &self.border - self.coupling.transpose() * &chain_response;
        // Round-off in B^T A^-1 B need not be bitwise symmetric.
        let schur = (&schur + schur.transpose()) * 0.5;
        let border = CholeskyFactor::new(&schur)?;
        Ok(BorderedFactor {
            diagonal,
            lower,
            coupling: self.coupling.clone(),
            chain_response,
            border,
        })
    }
}

impl BorderedFactor {
    pub fn dimension(&self) -> usize {
        6 * self.diagonal.len() + self.border.dimension()
    }

    /// Solve one load vector, ordered chain first and border last.
    pub fn solve(&self, rhs: &DVector<f64>) -> Result<DVector<f64>, LinearError> {
        let result =
            self.solve_matrix(&DMatrix::from_column_slice(rhs.len(), 1, rhs.as_slice()))?;
        Ok(result.column(0).into_owned())
    }

    /// Solve several independent loads with the same factorisation.
    pub fn solve_matrix(&self, rhs: &DMatrix<f64>) -> Result<DMatrix<f64>, LinearError> {
        if rhs.nrows() != self.dimension() {
            return Err(LinearError::Dimensions);
        }
        if !rhs.iter().all(|x| x.is_finite()) {
            return Err(LinearError::NonFinite);
        }
        let chain_size = 6 * self.diagonal.len();
        let chain_rhs = rhs.rows(0, chain_size).into_owned();
        let y = solve_chain(&self.diagonal, &self.lower, &chain_rhs);
        let border_rhs = rhs.rows(chain_size, self.border.dimension()).into_owned()
            - self.coupling.transpose() * &y;
        let z = self.border.solve_matrix(&border_rhs)?;
        let x = y - &self.chain_response * &z;
        let mut result = DMatrix::zeros(self.dimension(), rhs.ncols());
        result.rows_mut(0, chain_size).copy_from(&x);
        result.rows_mut(chain_size, z.nrows()).copy_from(&z);
        finite_result(result)
    }
}

fn solve_chain(
    diagonal: &[Matrix6<f64>],
    lower: &[Matrix6<f64>],
    rhs: &DMatrix<f64>,
) -> DMatrix<f64> {
    let mut result = rhs.clone();
    for column in 0..rhs.ncols() {
        for (block, l) in diagonal.iter().enumerate() {
            for row in 0..6 {
                let index = block * 6 + row;
                let mut value = result[(index, column)];
                if block > 0 {
                    for k in 0..6 {
                        value -= lower[block - 1][(row, k)] * result[((block - 1) * 6 + k, column)];
                    }
                }
                for k in 0..row {
                    value -= l[(row, k)] * result[(block * 6 + k, column)];
                }
                result[(index, column)] = value / l[(row, row)];
            }
        }
        for block in (0..diagonal.len()).rev() {
            let l = &diagonal[block];
            for row in (0..6).rev() {
                let index = block * 6 + row;
                let mut value = result[(index, column)];
                if block + 1 < diagonal.len() {
                    for k in 0..6 {
                        value -= lower[block][(k, row)] * result[((block + 1) * 6 + k, column)];
                    }
                }
                for k in row + 1..6 {
                    value -= l[(k, row)] * result[(block * 6 + k, column)];
                }
                result[(index, column)] = value / l[(row, row)];
            }
        }
    }
    result
}

/// Hand-written dense Cholesky for the border and small reduced systems.
#[derive(Clone, Debug)]
pub struct CholeskyFactor {
    // Row-major storage keeps each Cholesky dot product contiguous. It is
    // private: callers always supply and receive ordinary nalgebra matrices.
    lower: Vec<f64>,
    size: usize,
}

impl CholeskyFactor {
    pub fn new(matrix: &DMatrix<f64>) -> Result<Self, LinearError> {
        check_symmetric(matrix)?;
        let size = matrix.nrows();
        let mut lower = vec![0.0; size * size];
        let input = matrix.as_slice();
        for row in 0..size {
            for column in 0..=row {
                let mut value = input[column * size + row];
                for (&a, &b) in lower[row * size..row * size + column]
                    .iter()
                    .zip(&lower[column * size..column * size + column])
                {
                    value -= a * b;
                }
                if row == column {
                    if value <= PIVOT_TOLERANCE * input[row * size + row].abs()
                        || !value.is_finite()
                    {
                        return Err(LinearError::NotPositiveDefinite);
                    }
                    lower[row * size + column] = value.sqrt();
                } else {
                    lower[row * size + column] = value / lower[column * size + column];
                }
            }
        }
        Ok(Self { lower, size })
    }

    pub fn dimension(&self) -> usize {
        self.size
    }

    pub fn solve(&self, rhs: &DVector<f64>) -> Result<DVector<f64>, LinearError> {
        let result =
            self.solve_matrix(&DMatrix::from_column_slice(rhs.len(), 1, rhs.as_slice()))?;
        Ok(result.column(0).into_owned())
    }

    pub fn solve_matrix(&self, rhs: &DMatrix<f64>) -> Result<DMatrix<f64>, LinearError> {
        if rhs.nrows() != self.dimension() {
            return Err(LinearError::Dimensions);
        }
        if !rhs.iter().all(|x| x.is_finite()) {
            return Err(LinearError::NonFinite);
        }
        let mut result = rhs.clone();
        let size = self.size;
        if size == 0 {
            return Ok(result);
        }
        for values in result.as_mut_slice().chunks_exact_mut(size) {
            for row in 0..size {
                let mut value = values[row];
                for (&a, &b) in self.lower[row * size..row * size + row]
                    .iter()
                    .zip(&values[..row])
                {
                    value -= a * b;
                }
                values[row] = value / self.lower[row * size + row];
            }
            for row in (0..size).rev() {
                let mut value = values[row];
                for (k, &coordinate) in values.iter().enumerate().skip(row + 1) {
                    value -= self.lower[k * size + row] * coordinate;
                }
                values[row] = value / self.lower[row * size + row];
            }
        }
        finite_result(result)
    }
}

fn finite_result(result: DMatrix<f64>) -> Result<DMatrix<f64>, LinearError> {
    if result.iter().all(|x| x.is_finite()) {
        Ok(result)
    } else {
        Err(LinearError::NonFinite)
    }
}

fn check_symmetric(matrix: &DMatrix<f64>) -> Result<(), LinearError> {
    if !matrix.is_square() {
        return Err(LinearError::Dimensions);
    }
    if !matrix.iter().all(|x| x.is_finite()) {
        return Err(LinearError::NonFinite);
    }
    let scale = matrix
        .iter()
        .fold(0.0_f64, |scale, value| scale.max(value.abs()));
    for row in 0..matrix.nrows() {
        for column in 0..row {
            if (matrix[(row, column)] - matrix[(column, row)]).abs() > SYMMETRY_TOLERANCE * scale {
                return Err(LinearError::NotSymmetric);
            }
        }
    }
    Ok(())
}

/// Rank-aware symmetric decomposition for constraint redundancies. It does
/// not apply damping or replace a null eigenvalue with a small stiffness.
#[derive(Clone, Debug)]
pub struct SymmetricSpectrum {
    vectors: DMatrix<f64>,
    values: DVector<f64>,
    cutoff: f64,
}

impl SymmetricSpectrum {
    /// `relative_tolerance` is a caller-chosen rank threshold against the
    /// largest absolute eigenvalue; no absolute unit-dependent floor is used.
    pub fn new(matrix: &DMatrix<f64>, relative_tolerance: f64) -> Result<Self, LinearError> {
        check_symmetric(matrix)?;
        if !relative_tolerance.is_finite() || relative_tolerance <= 0.0 || relative_tolerance >= 1.0
        {
            return Err(LinearError::Dimensions);
        }
        if matrix.nrows() == 0 {
            return Ok(Self {
                vectors: DMatrix::zeros(0, 0),
                values: DVector::zeros(0),
                cutoff: 0.0,
            });
        }
        let eigen = SymmetricEigen::try_new(matrix.clone(), f64::EPSILON, SPECTRUM_ITERATION_LIMIT)
            .ok_or(LinearError::SpectrumIterationLimit)?;
        if !eigen.eigenvalues.iter().all(|x| x.is_finite())
            || !eigen.eigenvectors.iter().all(|x| x.is_finite())
        {
            return Err(LinearError::NonFinite);
        }
        let largest = eigen
            .eigenvalues
            .iter()
            .fold(0.0_f64, |a, b| a.max(b.abs()));
        Ok(Self {
            vectors: eigen.eigenvectors,
            values: eigen.eigenvalues,
            cutoff: relative_tolerance * largest,
        })
    }

    pub fn rank(&self) -> usize {
        self.values.iter().filter(|v| v.abs() > self.cutoff).count()
    }

    pub fn is_positive_semidefinite(&self) -> bool {
        self.values.iter().all(|value| *value >= -self.cutoff)
    }

    /// Columns span the numerical null space and are orthonormal.
    pub fn null_space(&self) -> DMatrix<f64> {
        let indices: Vec<_> = self
            .values
            .iter()
            .enumerate()
            .filter_map(|(i, value)| (value.abs() <= self.cutoff).then_some(i))
            .collect();
        DMatrix::from_fn(self.values.len(), indices.len(), |row, column| {
            self.vectors[(row, indices[column])]
        })
    }

    /// Moore-Penrose solution; any null-space part of `rhs` is projected out.
    pub fn pseudo_solve(&self, rhs: &DVector<f64>) -> Result<DVector<f64>, LinearError> {
        if rhs.len() != self.values.len() {
            return Err(LinearError::Dimensions);
        }
        if !rhs.iter().all(|x| x.is_finite()) {
            return Err(LinearError::NonFinite);
        }
        let mut transformed = self.vectors.transpose() * rhs;
        for i in 0..transformed.len() {
            transformed[i] = if self.values[i].abs() > self.cutoff {
                transformed[i] / self.values[i]
            } else {
                0.0
            };
        }
        let solution = &self.vectors * transformed;
        if solution.iter().all(|x| x.is_finite()) {
            Ok(solution)
        } else {
            Err(LinearError::NonFinite)
        }
    }

    /// As [`Self::pseudo_solve`], but refuses a relative null-space load above
    /// `relative_tolerance`; this detects an inconsistent constraint system.
    pub fn solve_consistent(
        &self,
        rhs: &DVector<f64>,
        relative_tolerance: f64,
    ) -> Result<DVector<f64>, LinearError> {
        if !relative_tolerance.is_finite() || !(0.0..1.0).contains(&relative_tolerance) {
            return Err(LinearError::Dimensions);
        }
        let solution = self.pseudo_solve(rhs)?;
        let null_load = self.null_space().transpose() * rhs;
        if null_load.norm() > relative_tolerance * rhs.norm() {
            Err(LinearError::Inconsistent)
        } else {
            Ok(solution)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elimination_matches_an_independently_assembled_dense_system() {
        // A diagonally dominant spring graph: chain neighbours couple and
        // every chain coordinate couples to each of two grounded border DOFs.
        let count = 4;
        let mut full = DMatrix::zeros(6 * count + 2, 6 * count + 2);
        for i in 0..full.nrows() {
            full[(i, i)] = 2.0;
        }
        for i in 0..6 * (count - 1) {
            full[(i, i)] += 3.0;
            full[(i + 6, i + 6)] += 3.0;
            full[(i, i + 6)] -= 3.0;
            full[(i + 6, i)] -= 3.0;
        }
        for i in 0..6 * count {
            for j in 0..2 {
                let k = (i + j + 1) as f64 / 20.0;
                full[(i, i)] += k;
                full[(24 + j, 24 + j)] += k;
                full[(i, 24 + j)] -= k;
                full[(24 + j, i)] -= k;
            }
        }
        let system = BorderedSystem {
            diagonal: (0..count)
                .map(|i| Matrix6::from_fn(|r, c| full[(6 * i + r, 6 * i + c)]))
                .collect(),
            lower: (0..count - 1)
                .map(|i| Matrix6::from_fn(|r, c| full[(6 * (i + 1) + r, 6 * i + c)]))
                .collect(),
            coupling: full.view((0, 24), (24, 2)).into_owned(),
            border: full.view((24, 24), (2, 2)).into_owned(),
        };
        let rhs = DMatrix::from_fn(26, 3, |i, j| ((i + 1) * (j + 2)) as f64);
        let factor = system.factor().unwrap();
        let result = factor.solve_matrix(&rhs).unwrap();
        assert!((&full * &result - &rhs).norm() < 1e-12);
        let dense = full.cholesky().unwrap().solve(&rhs);
        assert!((result - dense).norm() < 1e-12);
    }

    #[test]
    fn a_mechanism_and_non_finite_values_are_refused() {
        assert_eq!(
            CholeskyFactor::new(&DMatrix::zeros(2, 2)).unwrap_err(),
            LinearError::NotPositiveDefinite
        );
        assert_eq!(
            CholeskyFactor::new(&DMatrix::from_element(1, 1, f64::NAN)).unwrap_err(),
            LinearError::NonFinite
        );
        assert_eq!(
            CholeskyFactor::new(&DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 0.0, 1.0])).unwrap_err(),
            LinearError::NotSymmetric
        );
    }

    #[test]
    fn mixed_coordinates_and_an_empty_border_keep_the_scale_law() {
        // Prescribe a block lower-bidiagonal factor with non-diagonal blocks,
        // then independently multiply it out. Its product is SPD by construction.
        let mut l = DMatrix::zeros(18, 18);
        for block in 0..3 {
            for row in 0..6 {
                for column in 0..=row {
                    l[(6 * block + row, 6 * block + column)] = if row == column {
                        2.0 + (row + block) as f64
                    } else {
                        (row + column + 1) as f64 / 10.0
                    };
                }
                if block > 0 {
                    for column in 0..6 {
                        l[(6 * block + row, 6 * (block - 1) + column)] =
                            (row + 2 * column + 1) as f64 / 20.0;
                    }
                }
            }
        }
        let matrix = &l * l.transpose();
        let answer = DVector::from_fn(18, |i, _| (i as f64 - 9.0) / 3.0);
        let rhs = &matrix * &answer;
        for scale in [1e-8, 1.0, 1e8] {
            let system = BorderedSystem {
                diagonal: (0..3)
                    .map(|i| Matrix6::from_fn(|r, c| matrix[(6 * i + r, 6 * i + c)] * scale))
                    .collect(),
                lower: (0..2)
                    .map(|i| Matrix6::from_fn(|r, c| matrix[(6 * (i + 1) + r, 6 * i + c)] * scale))
                    .collect(),
                coupling: DMatrix::zeros(18, 0),
                border: DMatrix::zeros(0, 0),
            };
            let result = system.factor().unwrap().solve(&(&rhs * scale)).unwrap();
            assert!((result - &answer).norm() < 2e-14);
        }
    }

    #[test]
    fn redundant_constraints_preserve_their_null_space() {
        // Two identical constraint rows, with stiffness 2 in their range.
        let spectrum = SymmetricSpectrum::new(&DMatrix::from_element(2, 2, 1.0), 1e-12).unwrap();
        assert_eq!(spectrum.rank(), 1);
        let solution = spectrum
            .solve_consistent(&DVector::from_element(2, 4.0), 1e-12)
            .unwrap();
        assert!((solution - DVector::from_element(2, 2.0)).norm() < 1e-14);
        assert_eq!(
            spectrum
                .solve_consistent(&DVector::from_column_slice(&[1.0, -1.0]), 1e-12)
                .unwrap_err(),
            LinearError::Inconsistent
        );
        assert!((spectrum.null_space().column(0).norm() - 1.0).abs() < 1e-14);
    }
}
