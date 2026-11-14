//! Objective first variation about the loaded rigid state R.
//!
//! Patient axes, mm/radians and N/N·mm follow `springback`. Head multipliers
//! act ON THE ROD. Each rod uses its state-R patient placement as a material
//! reference. With rod pose `(t,Q)`, head reference point `p`, and body head
//! `X=c+R l`, the strains are `Qᵀ(X-t)-p` and, for a screw,
//! `Log(Qᵀ R R0ᵀ)`. A hook has only the first three coordinates.
//!
//! The Hessian of the rigid constraint multipliers is essential: the tissue
//! body-centre Hessian alone has the wrong curvature on a rotating loaded
//! block. This module linearises the objective complementary-energy KKT
//! equations once, then exactly condenses the six free rod-pose coordinates.
//! The result is `(Kt + Geff + Σ Bᵀ Kr B)u=-g`, without damping, added springs
//! or conditional fallback. `Geff` is derived from the order-0 head loads.
//!
//! Rod compliance has its exact rigid nullspace. Its compatibility equation
//! fixes the frame increment to the Euclidean rigid fit of the head motion;
//! this is a strain-coordinate gauge, not a restraint. A straight hook-only
//! rod's invisible roll is removed by a rank-aware six-coordinate solve.
//! The same gauge supplies the rigid-reaction part of the head multipliers.
//! Material multipliers, first-order physical transport, and finite composed
//! transport are returned separately. Only the first variation is solved;
//! the finite transported loads retain a second-order nonlinear remainder.
//! `solve_incremental` uses the same tangent for an added body load about a
//! prestressed configuration; its three head-load arrays contain increments.

use super::head_stiffness::{congruence, diagonal_scales};
use super::{RodLinearisation, SpringbackError};
use crate::chain::{Pose, Wrench};
use crate::chain_solver::{CholeskyFactor, SymmetricSpectrum};
use crate::joint_springs::log_left_derivative;
use forward_model::rotation::{matrix_from_rotation_vector, rotation_vector_from_matrix};
use nalgebra::{DMatrix, DVector, Matrix3, Vector3};

/// Project rank threshold for the six-coordinate rod-frame Gram matrix.
const FRAME_RANK_TOLERANCE: f64 = 1e-13;
/// Project relative check for the single linear solve and its KKT balance.
const RESIDUAL_TOLERANCE: f64 = 1e-9;
/// Project cap for round-off refinement of the same factored equation.
const REFINEMENT_STEPS: usize = 3;

/// One objective elastic expansion. Head order is unchanged from each rod.
/// `solve_prestressed` returns total head loads after seating;
/// `solve_incremental` returns added loads about its supplied preload.
#[derive(Clone, Debug)]
pub struct PrestressedStep {
    pub displacement: DVector<f64>,
    /// Patient-axis translation of the reference centroid, then rotation.
    pub rod_displacements: Vec<Wrench>,
    pub rod_origins_mm: Vec<Vector3<f64>>,
    /// Includes the rigid-reaction part; conjugate to material head strains.
    /// A total multiplier for seating, a multiplier increment for daily loads.
    pub material_head_wrenches: Vec<Vec<Wrench>>,
    /// Physical patient wrenches to first order, about the moving head.
    /// Use reference head positions plus their linear increments when
    /// checking first-order moment balance, retaining only first-order terms.
    pub linear_head_wrenches_on_rod: Vec<Vec<Wrench>>,
    /// Finite pose/log transport of the material loads. Incremental mode
    /// transports preload plus increment and then subtracts the preload.
    /// This differs from first-order transport by the nonlinear remainder.
    pub physical_head_wrenches_on_rod: Vec<Vec<Wrench>>,
    /// Body-chart multiplier-curvature load in the full, uncondensed KKT.
    pub body_geometric_load: DVector<f64>,
    /// Rod-frame multiplier-curvature load. It equals Rᵀ lambda after
    /// recovery, so raw material multipliers need not balance at old points.
    pub rod_geometric_loads: Vec<Wrench>,
    pub geometric_tangent: DMatrix<f64>,
    pub combined_tangent: DMatrix<f64>,
    pub relative_residual: f64,
}

struct RodGeometry {
    origin: Vector3<f64>,
    rigid: DMatrix<f64>,
    gram_inverse: DMatrix<f64>,
    frame_map: DMatrix<f64>,
    body: DMatrix<f64>,
    mixed: DMatrix<f64>,
    frame: DMatrix<f64>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Expansion {
    Seating,
    Incremental,
}

fn arm_hessian(r: Vector3<f64>, f: Vector3<f64>) -> Matrix3<f64> {
    (r * f.transpose() + f * r.transpose()) * 0.5 - Matrix3::identity() * r.dot(&f)
}

fn add_block(matrix: &mut DMatrix<f64>, row: usize, column: usize, block: &Matrix3<f64>) {
    for j in 0..3 {
        for i in 0..3 {
            matrix[(row + i, column + j)] += block[(i, j)];
        }
    }
}

fn flatten(loads: &[Wrench]) -> DVector<f64> {
    DVector::from_iterator(
        6 * loads.len(),
        loads.iter().flat_map(|w| w.iter().copied()),
    )
}

fn chunks(vector: &DVector<f64>) -> Vec<Wrench> {
    vector
        .as_slice()
        .as_chunks::<6>()
        .0
        .iter()
        .map(|w| Wrench::from_column_slice(w))
        .collect()
}

fn geometry(
    poses: &[Pose],
    rod: &RodLinearisation,
    initial: &[Wrench],
) -> Result<RodGeometry, SpringbackError> {
    let size = 6 * poses.len();
    let head_size = 6 * rod.ties.len();
    if initial.len() != rod.ties.len()
        || rod.ties.len() < 2
        || rod.kinematics.shape() != (head_size, size)
        || rod.effective_stiffness.shape() != (head_size, head_size)
    {
        return Err(SpringbackError::Dimensions);
    }
    if initial
        .iter()
        .flat_map(|w| w.iter())
        .any(|v| !v.is_finite())
    {
        return Err(SpringbackError::NonFinite);
    }
    let origin = rod
        .ties
        .iter()
        .map(|tie| tie.point_mm)
        .sum::<Vector3<f64>>()
        / rod.ties.len() as f64;
    let mut rigid = DMatrix::zeros(head_size, 6);
    let mut body = DMatrix::zeros(size, size);
    let mut mixed = DMatrix::zeros(size, 6);
    let mut frame = DMatrix::zeros(6, 6);
    for (i, (tie, load)) in rod.ties.iter().zip(initial).enumerate() {
        if tie.body >= poses.len() {
            return Err(SpringbackError::InvalidTie);
        }
        let r = tie.point_mm - poses[tie.body].centre_mm;
        let p = tie.point_mm - origin;
        let force = load.fixed_rows::<3>(0).into_owned();
        let couple = load.fixed_rows::<3>(3).into_owned();
        if !tie.locks_rotation && couple.norm() != 0.0 {
            return Err(SpringbackError::InconsistentHookRelease);
        }
        for axis in 0..3 {
            rigid[(6 * i + axis, axis)] = 1.0;
            if tie.locks_rotation {
                rigid[(6 * i + 3 + axis, 3 + axis)] = 1.0;
            }
        }
        rigid
            .view_mut((6 * i, 3), (3, 3))
            .copy_from(&(-p.cross_matrix()));

        // f·{w×(w×r)+W×(W×p)-2W×[v+w×r-V]} - m·(W×w).
        let force_cross = force.cross_matrix();
        let rotational_mixed = -force_cross * r.cross_matrix() + couple.cross_matrix() * 0.5;
        add_block(
            &mut body,
            6 * tie.body + 3,
            6 * tie.body + 3,
            &arm_hessian(r, force),
        );
        add_block(&mut frame, 3, 3, &arm_hessian(p, force));
        add_block(&mut mixed, 6 * tie.body, 3, &force_cross.transpose());
        add_block(
            &mut mixed,
            6 * tie.body + 3,
            3,
            &rotational_mixed.transpose(),
        );
        add_block(&mut frame, 3, 0, &(-force_cross));
        add_block(&mut frame, 0, 3, &force_cross);
    }
    let gram = rigid.transpose() * &rigid;
    let spectrum = SymmetricSpectrum::new(&gram, FRAME_RANK_TOLERANCE)?;
    if !spectrum.is_positive_semidefinite() || spectrum.rank() < 5 {
        return Err(SpringbackError::InvalidRodStiffness);
    }
    let mut gram_inverse = DMatrix::zeros(6, 6);
    for column in 0..6 {
        let mut unit = DVector::zeros(6);
        unit[column] = 1.0;
        gram_inverse
            .column_mut(column)
            .copy_from(&spectrum.pseudo_solve(&unit)?);
    }
    let frame_map = &gram_inverse * rigid.transpose() * &rod.kinematics;
    Ok(RodGeometry {
        origin,
        rigid,
        gram_inverse,
        frame_map,
        body,
        mixed,
        frame,
    })
}

fn condensed_geometry(local: &RodGeometry) -> DMatrix<f64> {
    let cross = &local.mixed * &local.frame_map;
    &local.body
        + &cross
        + cross.transpose()
        + local.frame_map.transpose() * &local.frame * &local.frame_map
}

/// Load-dependent geometric tangent after exact free-rod-frame condensation.
///
/// All data refer to the same patient-frame configuration. `initial_loads`
/// are material head multipliers at that configuration, ON each rod, in its
/// tie order. Add this matrix to the raw tissue tangent and mapped material
/// stiffness. Daily-case increments also need the corresponding preload
/// transport when recovering physical head forces and couples; merely using
/// the raw rod matrices and reprojecting their rigid modes omits this term.
pub fn geometric_tangent(
    poses: &[Pose],
    rods: &[RodLinearisation],
    initial_loads: &[Vec<Wrench>],
) -> Result<DMatrix<f64>, SpringbackError> {
    if poses.is_empty() || initial_loads.len() != rods.len() {
        return Err(SpringbackError::Dimensions);
    }
    let size = 6 * poses.len();
    let mut tangent = DMatrix::zeros(size, size);
    for (rod, loads) in rods.iter().zip(initial_loads) {
        tangent += condensed_geometry(&geometry(poses, rod, loads)?);
    }
    if tangent.iter().any(|value| !value.is_finite()) {
        return Err(SpringbackError::NonFinite);
    }
    Ok((&tangent + tangent.transpose()) * 0.5)
}

/// Objective elastic KKT expansion, with one SPD factorisation after exact
/// free-frame condensation. Every input uses the state-R patient frame.
pub fn solve_prestressed(
    poses: &[Pose],
    tissue_tangent: &DMatrix<f64>,
    tissue_gradient: &DVector<f64>,
    rods: &[RodLinearisation],
    initial_loads: &[Vec<Wrench>],
) -> Result<PrestressedStep, SpringbackError> {
    solve(
        poses,
        tissue_tangent,
        tissue_gradient,
        rods,
        initial_loads,
        Expansion::Seating,
    )
}

/// One added-load solve about a supplied prestressed configuration.
///
/// `body_load_increment` is the applied force/moment increment ON each body
/// centre, in patient axes (N/N·mm). Geometry, material matrices and the raw
/// tissue tangent refer to that same configuration. `preload` is its material
/// head load, ON each rod. The equation is `(Kt+Geff+ΣBᵀKrB)du=f_increment`.
/// All three returned head-load arrays contain increments; finite physical
/// increments are the transported preload-plus-increment minus the preload.
///
/// This incremental equation does not assert zeroth-order equilibrium of the
/// supplied preload, whose seated value can retain a second-order remainder.
/// It checks the full first-order body and free-frame incremental equations.
pub fn solve_incremental(
    poses: &[Pose],
    tissue_tangent: &DMatrix<f64>,
    body_load_increment: &DVector<f64>,
    rods: &[RodLinearisation],
    preload: &[Vec<Wrench>],
) -> Result<PrestressedStep, SpringbackError> {
    solve(
        poses,
        tissue_tangent,
        &(-body_load_increment),
        rods,
        preload,
        Expansion::Incremental,
    )
}

fn solve(
    poses: &[Pose],
    tissue_tangent: &DMatrix<f64>,
    tissue_gradient: &DVector<f64>,
    rods: &[RodLinearisation],
    initial_loads: &[Vec<Wrench>],
    mode: Expansion,
) -> Result<PrestressedStep, SpringbackError> {
    let size = 6 * poses.len();
    if size == 0
        || tissue_gradient.len() != size
        || tissue_tangent.shape() != (size, size)
        || initial_loads.len() != rods.len()
    {
        return Err(SpringbackError::Dimensions);
    }
    if tissue_tangent
        .iter()
        .chain(tissue_gradient.iter())
        .any(|v| !v.is_finite())
    {
        return Err(SpringbackError::NonFinite);
    }
    let mut geometric = DMatrix::zeros(size, size);
    let mut combined = tissue_tangent.clone();
    let mut geometries = Vec::with_capacity(rods.len());
    let mut initial_residual = tissue_gradient.clone();
    for (rod, loads) in rods.iter().zip(initial_loads) {
        let local = geometry(poses, rod, loads)?;
        let p0 = flatten(loads);
        initial_residual += rod.kinematics.transpose() * &p0;
        if mode == Expansion::Seating
            && (local.rigid.transpose() * &p0).norm() > RESIDUAL_TOLERANCE * p0.norm().max(1.0)
        {
            return Err(SpringbackError::InconsistentRigidLoads);
        }
        geometric += condensed_geometry(&local);
        combined += rod.kinematics.transpose() * &rod.effective_stiffness * &rod.kinematics;
        geometries.push(local);
    }
    if mode == Expansion::Seating
        && initial_residual.norm() > RESIDUAL_TOLERANCE * tissue_gradient.norm().max(1.0)
    {
        return Err(SpringbackError::InconsistentRigidLoads);
    }
    geometric = (&geometric + geometric.transpose()) * 0.5;
    combined += &geometric;
    combined = (&combined + combined.transpose()) * 0.5;
    let scales = diagonal_scales(&combined);
    let factor = CholeskyFactor::new(&congruence(&combined, &scales))
        .map_err(|_| SpringbackError::NotPositiveDefinite)?;
    let mut displacement = factor
        .solve(&(-tissue_gradient.component_mul(&scales)))?
        .component_mul(&scales);
    for _ in 0..REFINEMENT_STEPS {
        let residual = -tissue_gradient - &combined * &displacement;
        displacement += factor
            .solve(&residual.component_mul(&scales))?
            .component_mul(&scales);
    }
    let mut result = PrestressedStep {
        displacement,
        rod_displacements: Vec::with_capacity(rods.len()),
        rod_origins_mm: Vec::with_capacity(rods.len()),
        material_head_wrenches: Vec::with_capacity(rods.len()),
        linear_head_wrenches_on_rod: Vec::with_capacity(rods.len()),
        physical_head_wrenches_on_rod: Vec::with_capacity(rods.len()),
        body_geometric_load: DVector::zeros(size),
        rod_geometric_loads: Vec::with_capacity(rods.len()),
        geometric_tangent: geometric,
        combined_tangent: combined,
        relative_residual: 0.0,
    };
    let mut body_residual = tissue_gradient + tissue_tangent * &result.displacement;
    let mut largest_residual = 0.0_f64;
    let denominator = tissue_gradient.norm().max(1.0);
    for ((rod, local), initial) in rods.iter().zip(&geometries).zip(initial_loads) {
        let frame_motion = &local.frame_map * &result.displacement;
        let frame_load =
            local.mixed.transpose() * &result.displacement + &local.frame * &frame_motion;
        let body_load = &local.body * &result.displacement + &local.mixed * &frame_motion;
        let rigid_reaction = &local.rigid * &local.gram_inverse * &frame_load;
        let lambda =
            &rod.effective_stiffness * &rod.kinematics * &result.displacement + rigid_reaction;
        body_residual += &body_load + rod.kinematics.transpose() * &lambda;
        result.body_geometric_load += body_load;
        let frame_residual = local.rigid.transpose() * &lambda - &frame_load;
        largest_residual = largest_residual.max(frame_residual.norm() / denominator);
        let material = chunks(&lambda);
        let big_omega = Vector3::from_column_slice(frame_motion.rows(3, 3).as_slice());
        let q = matrix_from_rotation_vector(&big_omega);
        let mut linear_loads = Vec::with_capacity(rod.ties.len());
        let mut physical_loads = Vec::with_capacity(rod.ties.len());
        for ((tie, load), p0) in rod.ties.iter().zip(&material).zip(initial) {
            let omega = Vector3::from_column_slice(
                result.displacement.rows(6 * tie.body + 3, 3).as_slice(),
            );
            let force0 = p0.fixed_rows::<3>(0).into_owned();
            let couple0 = p0.fixed_rows::<3>(3).into_owned();
            let mut linear = *load;
            linear
                .fixed_rows_mut::<3>(0)
                .add_assign(big_omega.cross(&force0));
            linear
                .fixed_rows_mut::<3>(3)
                .add_assign((big_omega + omega).cross(&couple0) * 0.5);
            let total = if mode == Expansion::Incremental {
                load + p0
            } else {
                *load
            };
            let reference = if mode == Expansion::Incremental {
                *p0
            } else {
                Wrench::zeros()
            };
            let mut physical = Wrench::zeros();
            physical
                .fixed_rows_mut::<3>(0)
                .copy_from(&(q * total.fixed_rows::<3>(0) - reference.fixed_rows::<3>(0)));
            if tie.locks_rotation {
                let phi = rotation_vector_from_matrix(
                    &(q.transpose() * matrix_from_rotation_vector(&omega)),
                );
                let couple = q * log_left_derivative(&phi).transpose() * total.fixed_rows::<3>(3)
                    - reference.fixed_rows::<3>(3);
                physical.fixed_rows_mut::<3>(3).copy_from(&couple);
            }
            linear_loads.push(linear);
            physical_loads.push(physical);
        }
        result
            .rod_displacements
            .push(Wrench::from_column_slice(frame_motion.as_slice()));
        result.rod_origins_mm.push(local.origin);
        result
            .rod_geometric_loads
            .push(Wrench::from_column_slice(frame_load.as_slice()));
        result.material_head_wrenches.push(material);
        result.linear_head_wrenches_on_rod.push(linear_loads);
        result.physical_head_wrenches_on_rod.push(physical_loads);
    }
    largest_residual = largest_residual.max(body_residual.norm() / denominator);
    let residual = tissue_gradient + &result.combined_tangent * &result.displacement;
    result.relative_residual = largest_residual.max(residual.norm() / denominator);
    if !result.relative_residual.is_finite() || result.relative_residual > RESIDUAL_TOLERANCE {
        return Err(SpringbackError::LinearResidual);
    }
    Ok(result)
}

use core::ops::AddAssign;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rod_beams::{BeamProperties, element_stiffness};
    use crate::springback::{HeadTie, LockedRod, prepare_rods};

    fn fixture(second_locks: bool) -> (Vec<Pose>, RodLinearisation) {
        let points = [Vector3::new(0.0, -1.0, 12.0), Vector3::new(2.0, 1.0, 32.0)];
        let poses = vec![
            Pose {
                centre_mm: Vector3::new(3.0, -2.0, 10.0),
                rotation: Matrix3::identity(),
            },
            Pose {
                centre_mm: Vector3::new(5.0, 0.0, 30.0),
                rotation: Matrix3::identity(),
            },
        ];
        // Deliberately chosen small synthetic section; no evidence cell.
        let properties = BeamProperties {
            young_modulus_mpa: 10_000.0,
            shear_modulus_mpa: 4_000.0,
            diameter_mm: 2.0,
            shear_coefficient: 0.8,
        };
        let stiffness = element_stiffness(points[0], points[1], properties).unwrap();
        let locked = LockedRod {
            ties: points
                .iter()
                .enumerate()
                .map(|(body, &point_mm)| HeadTie {
                    body,
                    point_mm,
                    locks_rotation: body == 0 || second_locks,
                })
                .collect(),
            stiffness: DMatrix::from_column_slice(12, 12, stiffness.as_slice()),
        };
        let rod = prepare_rods(&poses, &[locked]).unwrap().remove(0);
        (poses, rod)
    }

    #[test]
    fn multiplier_curvature_matches_exact_finite_tie_energy() {
        let (poses, rod) = fixture(true);
        let force = Vector3::new(23.0, -11.0, 7.0);
        let couple = Vector3::new(4.0, -5.0, 2.0);
        let mut load = Wrench::zeros();
        load.fixed_rows_mut::<3>(0).copy_from(&force);
        load.fixed_rows_mut::<3>(3).copy_from(&couple);
        let local = geometry(&poses, &rod, &[load, Wrench::zeros()]).unwrap();
        let mut exact = DMatrix::zeros(18, 18);
        exact.view_mut((0, 0), (12, 12)).copy_from(&local.body);
        exact.view_mut((0, 12), (12, 6)).copy_from(&local.mixed);
        exact
            .view_mut((12, 0), (6, 12))
            .copy_from(&local.mixed.transpose());
        exact.view_mut((12, 12), (6, 6)).copy_from(&local.frame);
        let value = |x: &DVector<f64>| {
            let body = poses[0].incremented(&Wrench::from_column_slice(x.rows(0, 6).as_slice()));
            let translation = Vector3::from_column_slice(x.rows(12, 3).as_slice());
            let turn = Vector3::from_column_slice(x.rows(15, 3).as_slice());
            let q = matrix_from_rotation_vector(&turn);
            let head = body.centre_mm + body.rotation * (rod.ties[0].point_mm - poses[0].centre_mm);
            let strain = q.transpose() * (head - local.origin - translation)
                - (rod.ties[0].point_mm - local.origin);
            let rotation = rotation_vector_from_matrix(&(q.transpose() * body.rotation));
            force.dot(&strain) + couple.dot(&rotation)
        };
        let h = 3e-4;
        for i in 0..18 {
            for j in 0..18 {
                let mut pp = DVector::zeros(18);
                let mut pm = DVector::zeros(18);
                let mut mp = DVector::zeros(18);
                let mut mm = DVector::zeros(18);
                pp[i] += h;
                pp[j] += h;
                pm[i] += h;
                pm[j] -= h;
                mp[i] -= h;
                mp[j] += h;
                mm[i] -= h;
                mm[j] -= h;
                let numeric = (value(&pp) - value(&pm) - value(&mp) + value(&mm)) / (4.0 * h * h);
                assert!(
                    (numeric - exact[(i, j)]).abs() < 1e-5,
                    "({i},{j}) {numeric} != {}",
                    exact[(i, j)]
                );
            }
        }
    }

    #[test]
    fn frame_condensation_agrees_with_the_uncondensed_elastic_kkt() {
        for second_locks in [true, false] {
            let (poses, rod) = fixture(second_locks);
            let force = Vector3::new(2.0, -1.0, 0.5);
            let couple = -(rod.ties[0].point_mm - rod.ties[1].point_mm).cross(&force);
            let mut loads = vec![Wrench::zeros(); 2];
            loads[0].fixed_rows_mut::<3>(0).copy_from(&force);
            loads[0].fixed_rows_mut::<3>(3).copy_from(&couple);
            loads[1].fixed_rows_mut::<3>(0).copy_from(&(-force));
            let p0 = flatten(&loads);
            let g = -(rod.kinematics.transpose() * &p0);
            let kt = DMatrix::identity(12, 12) * 10_000.0;
            let local = geometry(&poses, &rod, &loads).unwrap();
            let active: Vec<_> = rod
                .ties
                .iter()
                .enumerate()
                .flat_map(|(head, tie)| {
                    (0..6)
                        .filter(move |axis| *axis < 3 || tie.locks_rotation)
                        .map(move |axis| 6 * head + axis)
                })
                .collect();
            let count = active.len();
            let stiffness = DMatrix::from_fn(count, count, |i, j| {
                rod.effective_stiffness[(active[i], active[j])]
            });
            let spectrum = SymmetricSpectrum::new(&stiffness, FRAME_RANK_TOLERANCE).unwrap();
            let mut compliance = DMatrix::zeros(count, count);
            for column in 0..count {
                let mut unit = DVector::zeros(count);
                unit[column] = 1.0;
                compliance
                    .column_mut(column)
                    .copy_from(&spectrum.pseudo_solve(&unit).unwrap());
            }
            let mut kkt = DMatrix::zeros(18 + count, 18 + count);
            kkt.view_mut((0, 0), (12, 12))
                .copy_from(&(&kt + &local.body));
            kkt.view_mut((0, 12), (12, 6)).copy_from(&local.mixed);
            kkt.view_mut((12, 0), (6, 12))
                .copy_from(&local.mixed.transpose());
            kkt.view_mut((12, 12), (6, 6)).copy_from(&local.frame);
            let mut j = DMatrix::zeros(count, 18);
            for (row, &coordinate) in active.iter().enumerate() {
                j.view_mut((row, 0), (1, 12))
                    .copy_from(&rod.kinematics.row(coordinate));
                j.view_mut((row, 12), (1, 6))
                    .copy_from(&(-local.rigid.row(coordinate)));
            }
            kkt.view_mut((18, 0), (count, 18)).copy_from(&j);
            kkt.view_mut((0, 18), (18, count)).copy_from(&j.transpose());
            kkt.view_mut((18, 18), (count, count))
                .copy_from(&(-&compliance));
            let mut rhs = DVector::zeros(18 + count);
            let active_p0 = DVector::from_iterator(count, active.iter().map(|&i| p0[i]));
            rhs.rows_mut(18, count)
                .copy_from(&(&compliance * &active_p0));
            let full = kkt.full_piv_lu().solve(&rhs).unwrap();
            let solved = solve_prestressed(&poses, &kt, &g, &[rod], &[loads]).unwrap();
            assert!((&solved.displacement - full.rows(0, 12)).norm() < 1e-9);
            assert!((solved.rod_displacements[0] - full.rows(12, 6)).norm() < 1e-9);
            let actual = flatten(&solved.material_head_wrenches[0]);
            for (row, &coordinate) in active.iter().enumerate() {
                assert!((actual[coordinate] - p0[coordinate] - full[18 + row]).abs() < 1e-8);
            }
        }
    }

    fn balanced_fixture() -> (Vec<Pose>, RodLinearisation, Vec<Wrench>, DVector<f64>) {
        let (poses, rod) = fixture(true);
        let force = Vector3::new(2.0, -1.0, 0.5);
        let couple = -(rod.ties[0].point_mm - rod.ties[1].point_mm).cross(&force);
        let mut loads = vec![Wrench::zeros(); 2];
        loads[0].fixed_rows_mut::<3>(0).copy_from(&force);
        loads[0].fixed_rows_mut::<3>(3).copy_from(&couple);
        loads[1].fixed_rows_mut::<3>(0).copy_from(&(-force));
        let gradient = -(rod.kinematics.transpose() * flatten(&loads));
        (poses, rod, loads, gradient)
    }

    #[test]
    fn a_rigid_block_gets_the_exact_gradient_times_path_curvature() {
        let (poses, rod, loads, g) = balanced_fixture();
        let tangent = geometric_tangent(&poses, core::slice::from_ref(&rod), &[loads]).unwrap();
        let origin = Vector3::new(1.0, 2.0, 3.0);
        let turn = Vector3::new(0.3, 0.1, -0.4);
        let translation = Vector3::new(0.2, -0.1, 0.3);
        let mut mode = DVector::zeros(12);
        let mut expected = 0.0;
        for (i, pose) in poses.iter().enumerate() {
            let r = pose.centre_mm - origin;
            mode.rows_mut(6 * i, 3)
                .copy_from(&(translation + turn.cross(&r)));
            mode.rows_mut(6 * i + 3, 3).copy_from(&turn);
            expected += g.rows(6 * i, 3).dot(&turn.cross(&turn.cross(&r)));
        }
        assert!((mode.dot(&(&tangent * &mode)) - expected).abs() < 1e-10);
        let strain = &rod.effective_stiffness * &rod.kinematics * &mode;
        assert!(strain.norm() < 1e-8);
    }

    #[test]
    fn transported_loads_balance_and_the_exact_scale_laws_hold() {
        let (poses, rod, loads, g) = balanced_fixture();
        let kt = DMatrix::identity(12, 12) * 10_000.0;
        let base = solve_prestressed(
            &poses,
            &kt,
            &g,
            core::slice::from_ref(&rod),
            core::slice::from_ref(&loads),
        )
        .unwrap();
        let u = &base.displacement;
        let mut force_balance = Vector3::zeros();
        let mut moment_balance = Vector3::zeros();
        for ((tie, load), preload) in rod
            .ties
            .iter()
            .zip(&base.linear_head_wrenches_on_rod[0])
            .zip(&loads)
        {
            let turn = Vector3::from_column_slice(u.rows(6 * tie.body + 3, 3).as_slice());
            let shift = Vector3::from_column_slice(u.rows(6 * tie.body, 3).as_slice());
            let head_shift = shift + turn.cross(&(tie.point_mm - poses[tie.body].centre_mm));
            let force = load.fixed_rows::<3>(0).into_owned();
            force_balance += force;
            moment_balance += tie.point_mm.cross(&force)
                + load.fixed_rows::<3>(3)
                + head_shift.cross(&preload.fixed_rows::<3>(0).into_owned());
        }
        assert!(force_balance.norm() < 1e-10);
        assert!(moment_balance.norm() < 1e-9);
        let factor = 3.7;
        let mut scaled_rod = rod.clone();
        scaled_rod.stiffness *= factor;
        scaled_rod.effective_stiffness *= factor;
        let scaled_loads: Vec<_> = loads.iter().map(|p| p * factor).collect();
        let scaled = solve_prestressed(
            &poses,
            &(&kt * factor),
            &(&g * factor),
            &[scaled_rod],
            &[scaled_loads],
        )
        .unwrap();
        assert!((&scaled.displacement - u).norm() < 1e-12);
        assert!(
            (flatten(&scaled.material_head_wrenches[0])
                - flatten(&base.material_head_wrenches[0]) * factor)
                .norm()
                < 1e-10
        );
        let a = 2.3;
        let b = 4.1;
        let mut rod_b = rod.clone();
        rod_b.stiffness *= b;
        rod_b.effective_stiffness *= b;
        let loads_a = loads.iter().map(|p| p * a).collect();
        let left = solve_prestressed(&poses, &(&kt * a), &(&g * a), &[rod_b], &[loads_a]).unwrap();
        let ratio = a / b;
        let loads_ratio = loads.iter().map(|p| p * ratio).collect();
        let right = solve_prestressed(
            &poses,
            &(&kt * ratio),
            &(&g * ratio),
            &[rod],
            &[loads_ratio],
        )
        .unwrap();
        assert!((left.displacement - right.displacement).norm() < 1e-12);
    }

    #[test]
    fn daily_load_increments_are_linear_and_share_load_independent_of_uniform_stiffness() {
        let (poses, rod, preload, _) = balanced_fixture();
        let kt = DMatrix::identity(12, 12) * 10_000.0;
        let mut force = DVector::zeros(12);
        force[2] = -1.3;
        force[8] = -0.7;
        let base = solve_incremental(
            &poses,
            &kt,
            &force,
            core::slice::from_ref(&rod),
            core::slice::from_ref(&preload),
        )
        .unwrap();
        let weight = 2.7;
        let heavier = solve_incremental(
            &poses,
            &kt,
            &(&force * weight),
            core::slice::from_ref(&rod),
            core::slice::from_ref(&preload),
        )
        .unwrap();
        assert!((&heavier.displacement - &base.displacement * weight).norm() < 1e-12);
        assert!(
            (flatten(&heavier.material_head_wrenches[0])
                - flatten(&base.material_head_wrenches[0]) * weight)
                .norm()
                < 1e-10
        );
        assert!(
            (flatten(&heavier.linear_head_wrenches_on_rod[0])
                - flatten(&base.linear_head_wrenches_on_rod[0]) * weight)
                .norm()
                < 1e-10
        );
        let scale = 3.7;
        let mut stiffer_rod = rod.clone();
        stiffer_rod.stiffness *= scale;
        stiffer_rod.effective_stiffness *= scale;
        let scaled_preload = preload.iter().map(|p| p * scale).collect();
        let stiffer = solve_incremental(
            &poses,
            &(&kt * scale),
            &force,
            &[stiffer_rod],
            &[scaled_preload],
        )
        .unwrap();
        assert!((&stiffer.displacement * scale - &base.displacement).norm() < 1e-12);
        assert!(
            (flatten(&stiffer.material_head_wrenches[0])
                - flatten(&base.material_head_wrenches[0]))
            .norm()
                < 1e-10
        );
        assert!(
            (flatten(&stiffer.linear_head_wrenches_on_rod[0])
                - flatten(&base.linear_head_wrenches_on_rod[0]))
            .norm()
                < 1e-10
        );
        let zero = solve_incremental(&poses, &kt, &DVector::zeros(12), &[rod], &[preload]).unwrap();
        assert_eq!(zero.displacement.norm(), 0.0);
        assert_eq!(flatten(&zero.physical_head_wrenches_on_rod[0]).norm(), 0.0);
    }
}
