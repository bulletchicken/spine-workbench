//! The single elastic solve and its rigid-limit least-energy dual.

use super::head_stiffness::{congruence, diagonal_scales};
use super::{LinearStep, RodLinearisation, SpringbackError};
use crate::chain::Wrench;
use crate::chain_solver::{CholeskyFactor, SymmetricSpectrum};
use nalgebra::{DMatrix, DVector};

/// Project relative threshold after diagonal congruence for rigid null modes.
const RANK_TOLERANCE: f64 = 1e-13;
/// Project relative tolerance for both physical balance and linear residuals.
const RESIDUAL_TOLERANCE: f64 = 1e-9;
/// Project fixed cap for round-off residual refinement, not nonlinear steps.
const REFINEMENT_STEPS: usize = 3;

/// Solve the stated linear springback equation once, after equilibrating
/// coordinates by diagonal congruence. No damping or stiffness is added.
pub fn solve_linear(
    tissue_tangent: &DMatrix<f64>,
    tissue_gradient: &DVector<f64>,
    rods: &[RodLinearisation],
) -> Result<LinearStep, SpringbackError> {
    let size = tissue_gradient.len();
    check_dimensions(size, tissue_tangent, rods)?;
    let mut combined = tissue_tangent.clone();
    for rod in rods {
        combined += rod.kinematics.transpose() * &rod.effective_stiffness * &rod.kinematics;
    }
    combined = (&combined + combined.transpose()) * 0.5;
    let scales = diagonal_scales(&combined);
    let scaled = congruence(&combined, &scales);
    let factor = CholeskyFactor::new(&scaled).map_err(|_| SpringbackError::NotPositiveDefinite)?;
    let mut displacement = factor
        .solve(&(-tissue_gradient.component_mul(&scales)))?
        .component_mul(&scales);
    for _ in 0..REFINEMENT_STEPS {
        let residual = -tissue_gradient - &combined * &displacement;
        let step = factor
            .solve(&residual.component_mul(&scales))?
            .component_mul(&scales);
        displacement += step;
    }
    let residual = tissue_gradient + &combined * &displacement;
    let denominator = tissue_gradient
        .norm()
        .max((&combined * &displacement).norm());
    let relative_residual = if denominator > 0.0 {
        residual.norm() / denominator
    } else {
        residual.norm()
    };
    if !relative_residual.is_finite() || relative_residual > RESIDUAL_TOLERANCE {
        return Err(SpringbackError::LinearResidual);
    }
    let loads = loads_for_displacement(rods, &displacement);
    Ok(LinearStep {
        displacement,
        head_wrenches_on_rod: loads,
        combined_tangent: combined,
        relative_residual,
    })
}

/// Unique least-rod-strain-energy head loads in the rigid limit. Loads act
/// on rods, so their mapped body loads equal minus the tissue gradient.
/// A solution exists only when the supplied state-R gradient balances every
/// free rigid motion. This function has no iteration starting point.
pub fn least_energy_loads(
    tissue_gradient: &DVector<f64>,
    rods: &[RodLinearisation],
) -> Result<Vec<Vec<Wrench>>, SpringbackError> {
    let size = tissue_gradient.len();
    check_dimensions(size, &DMatrix::zeros(size, size), rods)?;
    if tissue_gradient.iter().any(|x| !x.is_finite()) {
        return Err(SpringbackError::NonFinite);
    }
    let mut stiffness = DMatrix::zeros(size, size);
    for rod in rods {
        stiffness += rod.kinematics.transpose() * &rod.effective_stiffness * &rod.kinematics;
    }
    stiffness = (&stiffness + stiffness.transpose()) * 0.5;
    let scales = diagonal_scales(&stiffness);
    let scaled = congruence(&stiffness, &scales);
    let spectrum = SymmetricSpectrum::new(&scaled, RANK_TOLERANCE)?;
    if !spectrum.is_positive_semidefinite() {
        return Err(SpringbackError::InvalidRodStiffness);
    }
    let rhs = -tissue_gradient.component_mul(&scales);
    // Congruence conditions the solve, not the physical balance criterion.
    // Uninstrumented rows have zero rod stiffness and retain scale one;
    // comparing their round-off load with the suppressed instrumented RHS
    // can falsely reject an otherwise balanced rigid state. Project here,
    // refine, then check the original physical equations below, at the same
    // residual tolerance. A genuine unsupported load is still refused there.
    let mut dual = spectrum.pseudo_solve(&rhs)?.component_mul(&scales);
    for _ in 0..REFINEMENT_STEPS {
        let residual = -tissue_gradient - &stiffness * &dual;
        let step = spectrum
            .pseudo_solve(&residual.component_mul(&scales))?
            .component_mul(&scales);
        dual += step;
    }
    let loads = loads_for_displacement(rods, &dual);
    let mut residual = tissue_gradient.clone();
    for (rod, head_loads) in rods.iter().zip(&loads) {
        let p = DVector::from_iterator(
            6 * head_loads.len(),
            head_loads.iter().flat_map(|w| w.iter().copied()),
        );
        residual += rod.kinematics.transpose() * p;
    }
    let relative = if tissue_gradient.norm() > 0.0 {
        residual.norm() / tissue_gradient.norm()
    } else {
        residual.norm()
    };
    if relative > RESIDUAL_TOLERANCE || !relative.is_finite() {
        return Err(SpringbackError::InconsistentRigidLoads);
    }
    Ok(loads)
}

fn check_dimensions(
    size: usize,
    tangent: &DMatrix<f64>,
    rods: &[RodLinearisation],
) -> Result<(), SpringbackError> {
    if size == 0
        || !size.is_multiple_of(6)
        || tangent.shape() != (size, size)
        || rods.iter().any(|r| {
            r.kinematics.ncols() != size
                || r.kinematics.nrows() != r.effective_stiffness.nrows()
                || !r.effective_stiffness.is_square()
        })
    {
        return Err(SpringbackError::Dimensions);
    }
    if tangent.iter().any(|x| !x.is_finite()) {
        return Err(SpringbackError::NonFinite);
    }
    Ok(())
}

fn loads_for_displacement(
    rods: &[RodLinearisation],
    displacement: &DVector<f64>,
) -> Vec<Vec<Wrench>> {
    rods.iter()
        .map(|rod| {
            let p = &rod.effective_stiffness * &rod.kinematics * displacement;
            // Project the final rounding residue out of the free rigid modes,
            // while preserving exactly zero hook moments.
            let active = DVector::from_iterator(rod.active.len(), rod.active.iter().map(|&i| p[i]));
            let active = &rod.rigid_projector * active;
            let mut cleaned = DVector::zeros(p.len());
            for (i, &coordinate) in rod.active.iter().enumerate() {
                cleaned[coordinate] = active[i];
            }
            cleaned
                .as_slice()
                .as_chunks::<6>()
                .0
                .iter()
                .map(|chunk| Wrench::from_column_slice(chunk))
                .collect()
        })
        .collect()
}
