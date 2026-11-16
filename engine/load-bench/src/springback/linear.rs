//! The single elastic solve and its rigid-limit least-energy dual.

use super::head_stiffness::{congruence, diagonal_scales};
use super::{LinearStep, RodLinearisation, SpringbackError};
use crate::chain::Wrench;
use crate::chain_solver::{CholeskyFactor, SymmetricSpectrum};
use crate::rigid_rod::LENGTH_MM;
use nalgebra::{DMatrix, DVector};

/// Project relative threshold after diagonal congruence for rigid null modes.
const RANK_TOLERANCE: f64 = 1e-13;
/// Project relative tolerance for both physical balance and linear residuals.
const RESIDUAL_TOLERANCE: f64 = 1e-9;
/// Project numerical tolerance for round-off in the rigid-limit balance, as
/// a share of the load the tissue's stiffness scale makes over [`LENGTH_MM`].
/// The rigid solve runs on the chain scaled by one over that stiffness,
/// with translations in steps of [`LENGTH_MM`], so its absolute
/// stationarity tolerance ([`crate::rigid_rod::minimise::Options`]) comes
/// back as that tolerance times stiffness over [`LENGTH_MM`] in newtons and
/// times stiffness in newton-millimetres. This floor, applied to one norm
/// over both, is ten times [`LENGTH_MM`] squared (4000) times the first and
/// ten times [`LENGTH_MM`] (200) times the second, so what the rigid solve
/// may leave unbalanced sits well under it. A straight rod through two
/// freely sliding heads holds nothing, so the tissue gradient it leaves is
/// round-off: a gradient under this floor needs no rod load and a residual
/// under it is accepted, instead of dividing round-off by round-off. It is
/// tied to the tissue, which makes the gradient, not to the rods, so stiffer
/// rods never loosen it.
const ROUND_OFF_LOAD_TOLERANCE: f64 = 1e-11;
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
/// `tissue_stiffness` is the chain's stiffness scale in N/mm
/// ([`crate::chain::Chain::stiffness_scale`]); it sets the round-off floor.
pub fn least_energy_loads(
    tissue_gradient: &DVector<f64>,
    rods: &[RodLinearisation],
    tissue_stiffness: f64,
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
    let round_off = round_off_load(tissue_stiffness);
    if tissue_gradient.norm() <= round_off {
        // Nothing beyond round-off to carry, so the least-energy loads are
        // zero; solving for them would only work on round-off.
        return Ok(rods
            .iter()
            .map(|rod| vec![Wrench::zeros(); rod.kinematics.nrows() / 6])
            .collect());
    }
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
    // Relative to the tissue gradient, but never below round-off: a gradient
    // that is itself round-off cannot judge a round-off residual.
    let allowed = (RESIDUAL_TOLERANCE * tissue_gradient.norm()).max(round_off);
    if residual.norm() > allowed || !residual.norm().is_finite() {
        return Err(SpringbackError::InconsistentRigidLoads);
    }
    Ok(loads)
}

/// The tissue load under which a rigid-limit gradient or residual is
/// round-off; `tissue_stiffness` as for [`least_energy_loads`].
pub(crate) fn round_off_load(tissue_stiffness: f64) -> f64 {
    ROUND_OFF_LOAD_TOLERANCE * tissue_stiffness.abs() * LENGTH_MM
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rigid_rod::minimise::Options;

    /// The floor once claimed ten times the rigid solve's tolerance; set to
    /// that, it would sit 400 times lower and two-head rods could stop
    /// settling again. Pin the margins its doc comment states.
    #[test]
    fn the_round_off_floor_keeps_its_margin_over_the_rigid_solve() {
        let tolerance = Options::default().absolute_stationarity_tolerance;
        for stiffness in [0.5, 1.0, 800.0] {
            let floor = round_off_load(stiffness);
            let force = tolerance * stiffness / LENGTH_MM;
            let moment = tolerance * stiffness;
            for (ratio, expected) in [(floor / force, 4000.0), (floor / moment, 200.0)] {
                assert!((ratio / expected - 1.0).abs() < 1e-9, "{ratio}");
            }
        }
    }
}
