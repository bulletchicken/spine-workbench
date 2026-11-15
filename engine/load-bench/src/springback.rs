//! Locked free rods: one elastic step about the released rigid state R.
//!
//! Patient axes are x anterior, y patient-left, z superior, right-handed.
//! Positions/translations are mm, rotations radians, force N, moment N·mm,
//! and stiffness maps those displacements to their conjugate loads. Positive
//! Mx tips an up axis toward −y, My toward +x, Mz turns anterior toward +y.
//! Head wrenches returned here act ON THE ROD; their negatives act on the
//! vertebra. Bone-on-screw reaction is opposite screw-on-bone force. Pull-out
//! is the rod-on-screw force opposite the inward insertion axis; push-in is
//! negative. Head moments exist, although the bench's anchor-force contract
//! reports only the resultant force, not those moments.
//!
//! At state R the rods have their stress-free contours and the tissue has
//! gradient g and exact tangent Kt. The objective head strains are measured
//! in a free moving frame for each rod. Their loaded rigid multipliers p0
//! contribute a constraint/load-transport Hessian. Exact condensation of
//! those frames gives one solve `(Kt + Geff + Σ Bᵀ Kr B) u = −g`.
//! Omitting Geff can give negative curvature on a stable rigid-block turn;
//! increasing rod stiffness cannot fix a term in the rods' rigid nullspace.
//! No physical stiffness or numerical damping is added. Each screw ties
//! all six freedoms at its locked
//! placement; hook rotations are released by static condensation. Every rod
//! retains all six free rigid motions; there is no holding torque in R or
//! springback. This is one linear step, not a nonlinear elastic equilibrium.
//! Its complete linear KKT residual is checked; its nonlinear remainder is
//! second order
//! only while the state-R tangent and constraints are regular.
//!
//! The same released stiffness supplies the unique order-0 head wrenches:
//! minimise rod strain energy under body and free-rod balance. The dual solve
//! uses `Σ Bᵀ Kr B`, with no tissue stiffness and no initial iterate. Null
//! coordinates are exposed by a bounded spectral decomposition, never by
//! adding springs. Inconsistent force balance is refused.
//!
//! Released balance loads are applied at body centres as dead loads, adding
//! no stiffness. `solve_balanced` additionally keeps T1's horizontal centre
//! increments zero in this linear step and recovers the condition reaction.
//! Height and rotations stay free; daily increments use the full tangent.

mod corotational;
mod head_stiffness;
mod linear;

use crate::chain::{Chain, ChainError, Pose, Wrench};
use crate::chain_solver::LinearError;
use nalgebra::{DMatrix, DVector, Vector3};

pub(crate) use corotational::solve_incremental_with_tissue_load;

pub use corotational::{PrestressedStep, geometric_tangent, solve_incremental, solve_prestressed};
pub use head_stiffness::prepare_rods;
pub use linear::{least_energy_loads, solve_linear};

/// Project choice converting a rotation to a length for the reported
/// displacement norm. It is not a tissue or implant material property.
pub const ROTATION_LENGTH_MM: f64 = 20.0;
/// Project numerical zero for the correction norm, in mm-equivalent units.
const CORRECTION_ZERO_MM: f64 = 1e-12;

/// One locked head, located in patient coordinates of the rigid state R.
#[derive(Clone, Debug)]
pub struct HeadTie {
    pub body: usize,
    pub point_mm: Vector3<f64>,
    /// Screws lock rotation; hooks leave it free.
    pub locks_rotation: bool,
}

/// Full six-freedom head stiffness in patient axes, at the state-R contour.
/// Its head order is the tie order. Free overhangs are already condensed.
#[derive(Clone, Debug)]
pub struct LockedRod {
    pub ties: Vec<HeadTie>,
    pub stiffness: DMatrix<f64>,
}

/// A rod and its head-to-body kinematics, for springback and daily cases.
#[derive(Clone, Debug)]
pub struct RodLinearisation {
    pub ties: Vec<HeadTie>,
    pub head_points_mm: Vec<Vector3<f64>>,
    /// Raw supplied six-freedom head stiffness, before releasing hooks.
    pub stiffness: DMatrix<f64>,
    /// Six-freedom representation with hook rotation rows/columns released.
    pub effective_stiffness: DMatrix<f64>,
    /// Head increments from body-centre increments, six coordinates per body.
    /// Hook rotation rows are zero; their translations still include the arm.
    pub kinematics: DMatrix<f64>,
    pub(crate) active: Vec<usize>,
    pub(crate) rigid_projector: DMatrix<f64>,
}

/// A complete linear solve. Wrenches are ordered rod, then head.
#[derive(Clone, Debug)]
pub struct LinearStep {
    pub displacement: DVector<f64>,
    /// Patient-axis force and moment ON THE ROD, moment about that head.
    pub head_wrenches_on_rod: Vec<Vec<Wrench>>,
    pub combined_tangent: DMatrix<f64>,
    pub relative_residual: f64,
}

/// The value later load cases build on. `tissue_tangent` is the state-R
/// tangent used by this expansion. Poses are R plus the single step;
/// `rods` retains the locked placements and full head matrices at R.
#[derive(Clone, Debug)]
pub struct SeatedState {
    pub poses: Vec<Pose>,
    pub rigid_poses: Vec<Pose>,
    pub held_poses: Vec<Pose>,
    pub displacement: DVector<f64>,
    /// Total applied dead load plus the linear T1-condition reaction,
    /// in body-centre patient coordinates, N/N·mm. No stiffness is added.
    pub applied_loads: DVector<f64>,
    /// Tissue gradient minus the applied load supplied at R. The linear
    /// balance also includes the increment of `applied_loads` from R.
    pub tissue_gradient: DVector<f64>,
    pub tissue_tangent: DMatrix<f64>,
    /// Constraint/load-transport curvature, exactly condensed through the
    /// free rod frames. Add this to tissue and material rod tangents.
    pub geometric_tangent: DMatrix<f64>,
    pub combined_tangent: DMatrix<f64>,
    pub rigid_head_wrenches_on_rod: Vec<Vec<Wrench>>,
    /// Conjugate multipliers in each state-R material frame.
    pub material_head_wrenches: Vec<Vec<Wrench>>,
    pub rod_origins_mm: Vec<Vector3<f64>>,
    pub rod_displacements: Vec<Wrench>,
    /// Terms in the uncondensed, first-order body equilibrium equation.
    pub body_geometric_load: DVector<f64>,
    pub rod_geometric_loads: Vec<Wrench>,
    /// First-order physical loads; linear moment balance also includes
    /// movement of their points of application under the rigid-state loads.
    pub linear_head_wrenches_on_rod: Vec<Vec<Wrench>>,
    pub rods: Vec<RodLinearisation>,
    /// Tissue tangent evaluated at the composed output poses, for daily cases.
    pub corrected_tissue_tangent: DMatrix<f64>,
    pub corrected_geometric_tangent: DMatrix<f64>,
    /// Daily-case head geometry, transported with each locked vertebra.
    /// Material matrices undergo their common rod-frame rotation, and their
    /// rigid null modes are projected onto the transported head geometry.
    /// This is a first-order re-reference, not a nonlinear beam update.
    pub corrected_rods: Vec<RodLinearisation>,
    /// Finite pose/log transport of the one-step material head loads,
    /// ON THE ROD with moment about the moved head. Its nonlinear free-body
    /// remainder is second order; use `linear_head_wrenches_on_rod` and
    /// the moved-application-point term for exact first-order statics.
    /// Keep RigidState::anchor_order to map rod/head order to input anchors.
    pub head_wrenches_on_rod: Vec<Vec<Wrench>>,
    /// Norm of the springback step divided by the standing-to-R correction.
    /// The norm weights each rotation by ROTATION_LENGTH_MM. It is not
    /// clamped to [0,1] and does not select a particular clinical angle.
    pub stiff_rod_share: f64,
    pub linear_relative_residual: f64,
    /// True only when the one linear solve is finite, positive definite on
    /// its free coordinates, and satisfies its residual check. The balanced
    /// step does not certify the full tangent used by daily increments;
    /// those solves retain their own positivity and residual checks.
    pub settled: bool,
}

/// No state is returned from a singular or inconsistent linear system.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SpringbackError {
    #[error("springback-dimensions")]
    Dimensions,
    #[error("springback-invalid-tie")]
    InvalidTie,
    #[error("springback-non-finite")]
    NonFinite,
    #[error("springback-invalid-rod-stiffness")]
    InvalidRodStiffness,
    #[error("springback-hook-release-is-inconsistent")]
    InconsistentHookRelease,
    #[error("springback-not-positive-definite")]
    NotPositiveDefinite,
    #[error("springback-rigid-loads-do-not-balance")]
    InconsistentRigidLoads,
    #[error("springback-linear-residual")]
    LinearResidual,
    #[error("springback-chain: {0}")]
    Chain(#[from] ChainError),
    #[error("springback-linear: {0}")]
    Linear(#[from] LinearError),
}

/// One elastic solve about R. Every input pose must use the same patient
/// frame as the head points and rod stiffness. The returned poses compose
/// the rotation step on the left about each body centre.
pub fn solve(
    chain: &Chain,
    rigid_poses: &[Pose],
    rods: &[LockedRod],
    held_poses: &[Pose],
) -> Result<SeatedState, SpringbackError> {
    solve_loaded(
        chain,
        rigid_poses,
        rods,
        held_poses,
        &DVector::zeros(6 * rigid_poses.len()),
    )
}

/// One elastic step with a constant body-centre applied load, with no added
/// stiffness or T1 restraint. Every load uses patient axes and N/N·mm.
pub fn solve_loaded(
    chain: &Chain,
    rigid_poses: &[Pose],
    rods: &[LockedRod],
    held_poses: &[Pose],
    applied_loads: &DVector<f64>,
) -> Result<SeatedState, SpringbackError> {
    solve_with_load(chain, rigid_poses, rods, held_poses, applied_loads, false)
}

/// Released-state springback: carry its applied load as a dead load and
/// retain its two horizontal T1 rows in the linear step. Because translation
/// is about each body centre, zero x/y increments keep T1 exactly in place.
pub fn solve_balanced(
    chain: &Chain,
    rigid_poses: &[Pose],
    rods: &[LockedRod],
    held_poses: &[Pose],
    applied_loads: &DVector<f64>,
) -> Result<SeatedState, SpringbackError> {
    solve_with_load(chain, rigid_poses, rods, held_poses, applied_loads, true)
}

fn solve_with_load(
    chain: &Chain,
    rigid_poses: &[Pose],
    rods: &[LockedRod],
    held_poses: &[Pose],
    applied_loads: &DVector<f64>,
    hold_horizontal: bool,
) -> Result<SeatedState, SpringbackError> {
    if held_poses.len() != rigid_poses.len() {
        return Err(SpringbackError::Dimensions);
    }
    if applied_loads.len() != 6 * rigid_poses.len() {
        return Err(SpringbackError::Dimensions);
    }
    if applied_loads.iter().any(|value| !value.is_finite()) {
        return Err(SpringbackError::NonFinite);
    }
    let tissue = chain.linearise(rigid_poses)?;
    let count = rigid_poses.len();
    let tangent = tangent_matrix(&tissue.diagonal, &tissue.lower);
    let gradient = DVector::from_iterator(
        6 * count,
        tissue
            .evaluation
            .gradient
            .iter()
            .flat_map(|w| w.iter().copied()),
    ) - applied_loads;
    let rods = prepare_rods(rigid_poses, rods)?;
    let rigid_loads = least_energy_loads(&gradient, &rods)?;
    let step = if hold_horizontal {
        corotational::solve_balanced_prestressed(
            rigid_poses,
            &tangent,
            &gradient,
            &rods,
            &rigid_loads,
        )?
    } else {
        solve_prestressed(rigid_poses, &tangent, &gradient, &rods, &rigid_loads)?
    };
    let poses: Vec<_> = rigid_poses
        .iter()
        .enumerate()
        .map(|(i, pose)| {
            pose.incremented(&Wrench::from_column_slice(
                step.displacement.rows(6 * i, 6).as_slice(),
            ))
        })
        .collect();
    let standing = chain.standing();
    let mut correction_squared = 0.0;
    let mut springback_squared = 0.0;
    for i in 0..count {
        let correction_rotation = forward_model::rotation::rotation_vector_from_matrix(
            &(rigid_poses[i].rotation * standing[i].rotation.transpose()),
        );
        correction_squared += (rigid_poses[i].centre_mm - standing[i].centre_mm).norm_squared()
            + ROTATION_LENGTH_MM * ROTATION_LENGTH_MM * correction_rotation.norm_squared();
        springback_squared += step.displacement.rows(6 * i, 3).norm_squared()
            + ROTATION_LENGTH_MM
                * ROTATION_LENGTH_MM
                * step.displacement.rows(6 * i + 3, 3).norm_squared();
    }
    let correction = correction_squared.sqrt();
    let change = springback_squared.sqrt();
    let stiff_rod_share = if correction <= CORRECTION_ZERO_MM && change <= CORRECTION_ZERO_MM {
        0.0
    } else if correction > 0.0 {
        change / correction
    } else {
        return Err(SpringbackError::LinearResidual);
    };
    let corrected = chain.linearise(&poses)?;
    let corrected_tissue_tangent = tangent_matrix(&corrected.diagonal, &corrected.lower);
    let corrected_input: Vec<_> = rods
        .iter()
        .enumerate()
        .map(|(r, rod)| LockedRod {
            ties: rod
                .ties
                .iter()
                .map(|tie| {
                    let reference = &rigid_poses[tie.body];
                    let local =
                        reference.rotation.transpose() * (tie.point_mm - reference.centre_mm);
                    HeadTie {
                        body: tie.body,
                        point_mm: poses[tie.body].centre_mm + poses[tie.body].rotation * local,
                        locks_rotation: tie.locks_rotation,
                    }
                })
                .collect(),
            stiffness: rotate_head_stiffness(
                &rod.stiffness,
                &forward_model::rotation::matrix_from_rotation_vector(
                    &step.rod_displacements[r].fixed_rows::<3>(3).into_owned(),
                ),
            ),
        })
        .collect();
    let corrected_rods = prepare_rods(&poses, &corrected_input)?;
    let corrected_geometric_tangent =
        geometric_tangent(&poses, &corrected_rods, &step.physical_head_wrenches_on_rod)?;
    Ok(SeatedState {
        poses,
        rigid_poses: rigid_poses.to_vec(),
        held_poses: held_poses.to_vec(),
        displacement: step.displacement,
        applied_loads: applied_loads + &step.applied_load_increment,
        tissue_gradient: gradient,
        tissue_tangent: tangent,
        geometric_tangent: step.geometric_tangent,
        combined_tangent: step.combined_tangent,
        rigid_head_wrenches_on_rod: rigid_loads,
        material_head_wrenches: step.material_head_wrenches,
        rod_origins_mm: step.rod_origins_mm,
        rod_displacements: step.rod_displacements,
        body_geometric_load: step.body_geometric_load,
        rod_geometric_loads: step.rod_geometric_loads,
        linear_head_wrenches_on_rod: step.linear_head_wrenches_on_rod,
        rods,
        corrected_tissue_tangent,
        corrected_geometric_tangent,
        corrected_rods,
        head_wrenches_on_rod: step.physical_head_wrenches_on_rod,
        stiff_rod_share,
        linear_relative_residual: step.relative_residual,
        settled: true,
    })
}

/// Common rod-frame rotation of the material matrix. This is a block
/// congruence, not an update of the linear beam's constitutive law.
fn rotate_head_stiffness(matrix: &DMatrix<f64>, q: &nalgebra::Matrix3<f64>) -> DMatrix<f64> {
    let mut result = DMatrix::zeros(matrix.nrows(), matrix.ncols());
    for row in 0..matrix.nrows() / 3 {
        for column in 0..matrix.ncols() / 3 {
            let block = q * matrix.fixed_view::<3, 3>(3 * row, 3 * column) * q.transpose();
            result
                .fixed_view_mut::<3, 3>(3 * row, 3 * column)
                .copy_from(&block);
        }
    }
    result
}

fn tangent_matrix(
    diagonal: &[nalgebra::Matrix6<f64>],
    lower: &[nalgebra::Matrix6<f64>],
) -> DMatrix<f64> {
    let mut tangent = DMatrix::zeros(6 * diagonal.len(), 6 * diagonal.len());
    for i in 0..diagonal.len() {
        tangent
            .view_mut((6 * i, 6 * i), (6, 6))
            .copy_from(&diagonal[i]);
        if i + 1 < diagonal.len() {
            tangent
                .view_mut((6 * (i + 1), 6 * i), (6, 6))
                .copy_from(&lower[i]);
            tangent
                .view_mut((6 * i, 6 * (i + 1)), (6, 6))
                .copy_from(&lower[i].transpose());
        }
    }
    tangent
}
