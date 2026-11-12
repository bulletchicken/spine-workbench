//! Covariance without LM damping, propagated to reported coordinates.

use crate::FitError;
use crate::levenberg_marquardt::Solution;
use crate::parameters::{COUNT, angle_jacobian};
use crate::patient_origin;
use nalgebra::{DMatrix, DVector};
use shapes::spine_fit::FitStandardDeviations;

/// With `rescale`, the inverse normal matrix is scaled by the residual
/// variance. Without it, the weights are taken as standard deviations in
/// real units, so a value only a prior decides keeps that prior's spread.
pub fn covariance(solution: &Solution, rescale: bool) -> Result<DMatrix<f64>, FitError> {
    let normal = &solution.normal;
    let cholesky = normal
        .matrix
        .clone()
        .cholesky()
        .ok_or(FitError::Underconstrained)?;
    let dof = normal
        .residual_count
        .checked_sub(solution.numbers.len())
        .filter(|&n| n > 0)
        .ok_or(FitError::Underconstrained)?;
    let covariance = if rescale {
        cholesky.inverse() * (normal.cost / dof as f64)
    } else {
        cholesky.inverse()
    };
    if covariance.iter().any(|x| !x.is_finite()) {
        return Err(FitError::NumericalFailure);
    }
    Ok(covariance)
}

pub fn standard_deviations(
    numbers: &[f64],
    covariance: &DMatrix<f64>,
    index: usize,
    origin: usize,
) -> FitStandardDeviations {
    let origin_jacobian = patient_origin::jacobian(numbers, origin);
    let angle_jacobian = angle_jacobian(&numbers[index * COUNT..(index + 1) * COUNT]);
    let mut centre_mm = [0.0; 3];
    let mut angles_rad = [0.0; 3];
    for axis in 0..3 {
        let mut derivative = DVector::zeros(numbers.len());
        for j in 0..COUNT {
            derivative[origin * COUNT + j] = -origin_jacobian[(axis, j)];
        }
        derivative[index * COUNT + axis] += 1.0;
        centre_mm[axis] = standard_deviation(covariance, &derivative);
        derivative.fill(0.0);
        for j in 0..3 {
            derivative[index * COUNT + 3 + j] = angle_jacobian[(axis, j)];
        }
        angles_rad[axis] = standard_deviation(covariance, &derivative);
    }
    FitStandardDeviations {
        centre_mm,
        angles_rad,
    }
}

fn standard_deviation(covariance: &DMatrix<f64>, derivative: &DVector<f64>) -> f64 {
    libm::sqrt(derivative.dot(&(covariance * derivative)).max(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moving_the_origin_cancels_its_centre_error() {
        let mut numbers = vec![0.0; COUNT + 2];
        numbers[6] = libm::log(20.0);
        let covariance = DMatrix::identity(numbers.len(), numbers.len());
        let sd = standard_deviations(&numbers, &covariance, 0, 0);
        // The reported centre of the origin vertebra is R*z*h/2. Its
        // original centre error cancels exactly. At identity and h=20,
        // d/d(rx,ry,log h) moves it by (10,10,10) mm in magnitude.
        for value in sd.centre_mm {
            assert!((value - 10.0).abs() < 1e-7);
        }
        for value in sd.angles_rad {
            assert!((value - 1.0).abs() < 1e-10);
        }
    }
}
