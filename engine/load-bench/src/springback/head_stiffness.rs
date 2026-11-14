//! Head kinematics and hook releases, retaining free rigid motions.

use super::{LockedRod, RodLinearisation, SpringbackError};
use crate::chain::Pose;
use crate::chain_solver::SymmetricSpectrum;
use nalgebra::{DMatrix, DVector, Vector3};

/// Project relative rank threshold after diagonal equilibration.
const RANK_TOLERANCE: f64 = 1e-13;
/// Project relative consistency tolerance for releasing a zero-energy mode.
const RELEASE_TOLERANCE: f64 = 1e-10;
/// Project round-off threshold in a rigid-motion Gram-Schmidt basis.
const BASIS_TOLERANCE: f64 = 1e-12;

/// Build mapped head matrices without changing the supplied raw stiffness.
/// Every free rigid mode is explicitly projected from round-off residue;
/// this changes no exact elastic stiffness or physical boundary condition.
pub fn prepare_rods(
    poses: &[Pose],
    rods: &[LockedRod],
) -> Result<Vec<RodLinearisation>, SpringbackError> {
    let mut prepared = Vec::with_capacity(rods.len());
    for rod in rods {
        let size = 6 * rod.ties.len();
        if rod.ties.len() < 2 || rod.stiffness.shape() != (size, size) {
            return Err(SpringbackError::Dimensions);
        }
        if rod.stiffness.iter().any(|v| !v.is_finite()) {
            return Err(SpringbackError::NonFinite);
        }
        let largest = rod.stiffness.abs().max();
        if (&rod.stiffness - rod.stiffness.transpose()).abs().max() > largest * RELEASE_TOLERANCE {
            return Err(SpringbackError::InvalidRodStiffness);
        }
        if rod
            .ties
            .iter()
            .any(|tie| tie.body >= poses.len() || tie.point_mm.iter().any(|x| !x.is_finite()))
        {
            return Err(SpringbackError::InvalidTie);
        }
        let points: Vec<_> = rod.ties.iter().map(|tie| tie.point_mm).collect();
        let all: Vec<_> = (0..size).collect();
        let full_projector = projector(&points, &all);
        let raw = &full_projector
            * ((&rod.stiffness + rod.stiffness.transpose()) * 0.5)
            * &full_projector;
        let active: Vec<_> = rod
            .ties
            .iter()
            .enumerate()
            .flat_map(|(i, tie)| {
                (0..6)
                    .filter(move |axis| *axis < 3 || tie.locks_rotation)
                    .map(move |axis| 6 * i + axis)
            })
            .collect();
        let released: Vec<_> = rod
            .ties
            .iter()
            .enumerate()
            .filter(|(_, tie)| !tie.locks_rotation)
            .flat_map(|(i, _)| (3..6).map(move |axis| 6 * i + axis))
            .collect();
        let mut active_stiffness = submatrix(&raw, &active, &active);
        if !released.is_empty() {
            let rr = submatrix(&raw, &released, &released);
            let ra = submatrix(&raw, &released, &active);
            let scales = diagonal_scales(&rr);
            let equilibrated = congruence(&rr, &scales);
            let spectrum = SymmetricSpectrum::new(&equilibrated, RANK_TOLERANCE)?;
            if !spectrum.is_positive_semidefinite() {
                return Err(SpringbackError::InvalidRodStiffness);
            }
            let mut response = DMatrix::zeros(released.len(), active.len());
            for column in 0..active.len() {
                let rhs = ra.column(column).component_mul(&scales);
                let solution = spectrum
                    .solve_consistent(&rhs, RELEASE_TOLERANCE)
                    .map_err(|_| SpringbackError::InconsistentHookRelease)?;
                response
                    .column_mut(column)
                    .copy_from(&solution.component_mul(&scales));
            }
            active_stiffness -= ra.transpose() * response;
        }
        let rigid_projector = projector(&points, &active);
        active_stiffness = &rigid_projector * active_stiffness * &rigid_projector;
        active_stiffness = (&active_stiffness + active_stiffness.transpose()) * 0.5;
        let mut effective = DMatrix::zeros(size, size);
        for (i, &row) in active.iter().enumerate() {
            for (j, &column) in active.iter().enumerate() {
                effective[(row, column)] = active_stiffness[(i, j)];
            }
        }
        let mut kinematics = DMatrix::zeros(size, 6 * poses.len());
        for (i, tie) in rod.ties.iter().enumerate() {
            let arm = tie.point_mm - poses[tie.body].centre_mm;
            for axis in 0..3 {
                kinematics[(6 * i + axis, 6 * tie.body + axis)] = 1.0;
                if tie.locks_rotation {
                    kinematics[(6 * i + 3 + axis, 6 * tie.body + 3 + axis)] = 1.0;
                }
            }
            kinematics
                .view_mut((6 * i, 6 * tie.body + 3), (3, 3))
                .copy_from(&(-arm.cross_matrix()));
        }
        prepared.push(RodLinearisation {
            ties: rod.ties.clone(),
            head_points_mm: points,
            stiffness: rod.stiffness.clone(),
            effective_stiffness: effective,
            kinematics,
            active,
            rigid_projector,
        });
    }
    Ok(prepared)
}

fn submatrix(matrix: &DMatrix<f64>, rows: &[usize], columns: &[usize]) -> DMatrix<f64> {
    DMatrix::from_fn(rows.len(), columns.len(), |i, j| {
        matrix[(rows[i], columns[j])]
    })
}

/// Diagonal congruence factors, preserving exactly zero coordinates.
pub(super) fn diagonal_scales(matrix: &DMatrix<f64>) -> DVector<f64> {
    DVector::from_fn(matrix.nrows(), |i, _| {
        let diagonal = matrix[(i, i)].abs();
        if diagonal > 0.0 {
            1.0 / diagonal.sqrt()
        } else {
            1.0
        }
    })
}

pub(super) fn congruence(matrix: &DMatrix<f64>, scales: &DVector<f64>) -> DMatrix<f64> {
    DMatrix::from_fn(matrix.nrows(), matrix.ncols(), |i, j| {
        matrix[(i, j)] * scales[i] * scales[j]
    })
}

fn projector(points: &[Vector3<f64>], active: &[usize]) -> DMatrix<f64> {
    let centre = points.iter().copied().sum::<Vector3<f64>>() / points.len() as f64;
    let modes = DMatrix::from_fn(active.len(), 6, |row, column| {
        let coordinate = active[row];
        let head = coordinate / 6;
        let axis = coordinate % 6;
        if column < 3 {
            f64::from(axis == column)
        } else if axis < 3 {
            (-(points[head] - centre).cross_matrix())[(axis, column - 3)]
        } else {
            f64::from(axis - 3 == column - 3)
        }
    });
    let mut basis: Vec<DVector<f64>> = Vec::new();
    for candidate in modes.column_iter() {
        let mut residual = candidate.into_owned();
        for _ in 0..2 {
            for vector in &basis {
                residual -= vector * vector.dot(&residual);
            }
        }
        let norm = residual.norm();
        if norm > BASIS_TOLERANCE * candidate.norm().max(1.0) {
            basis.push(residual / norm);
        }
    }
    let basis = DMatrix::from_columns(&basis);
    DMatrix::identity(active.len(), active.len()) - &basis * basis.transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rod_beams::{BeamProperties, CondensedRod, polyline_stations};
    use crate::springback::HeadTie;
    use nalgebra::Matrix3;

    #[test]
    fn a_hook_releases_its_couple_without_grounding_the_rod() {
        let points = vec![
            Vector3::zeros(),
            Vector3::new(3.0, 0.0, 20.0),
            Vector3::new(0.0, 0.0, 40.0),
        ];
        let poses: Vec<_> = points
            .iter()
            .map(|&centre_mm| Pose {
                centre_mm,
                rotation: Matrix3::identity(),
            })
            .collect();
        let stations = polyline_stations(&points).unwrap();
        let beam = CondensedRod::new(
            &points,
            &stations,
            BeamProperties {
                young_modulus_mpa: 60_000.0,
                shear_modulus_mpa: 25_000.0,
                diameter_mm: 3.0,
                shear_coefficient: 0.8,
            },
        )
        .unwrap();
        let rod = LockedRod {
            ties: points
                .iter()
                .enumerate()
                .map(|(body, &point_mm)| HeadTie {
                    body,
                    point_mm,
                    locks_rotation: body != 1,
                })
                .collect(),
            stiffness: beam.stiffness,
        };
        let prepared = prepare_rods(&poses, &[rod]).unwrap();
        let rod = &prepared[0];
        let u = DVector::from_fn(18, |i, _| (i % 7) as f64 * 0.001);
        let p = &rod.effective_stiffness * &rod.kinematics * u;
        assert_eq!(p.rows(9, 3).norm(), 0.0);
        let mut force = Vector3::zeros();
        let mut moment = Vector3::zeros();
        for (i, point) in points.iter().enumerate() {
            let f = Vector3::from_column_slice(p.rows(6 * i, 3).as_slice());
            force += f;
            moment += point.cross(&f) + p.rows(6 * i + 3, 3);
        }
        assert!(force.norm() < 1e-10, "{force}");
        assert!(moment.norm() < 1e-9, "{moment}");
    }

    #[test]
    fn releasing_every_hook_matches_the_translation_only_rod() {
        let points = vec![Vector3::zeros(), Vector3::z() * 20.0, Vector3::z() * 40.0];
        let poses: Vec<_> = points
            .iter()
            .map(|&centre_mm| Pose {
                centre_mm,
                rotation: Matrix3::identity(),
            })
            .collect();
        let stations = polyline_stations(&points).unwrap();
        let beam = CondensedRod::new(
            &points,
            &stations,
            BeamProperties {
                young_modulus_mpa: 60_000.0,
                shear_modulus_mpa: 25_000.0,
                diameter_mm: 3.0,
                shear_coefficient: 0.8,
            },
        )
        .unwrap();
        let rod = LockedRod {
            ties: points
                .iter()
                .enumerate()
                .map(|(body, &point_mm)| HeadTie {
                    body,
                    point_mm,
                    locks_rotation: false,
                })
                .collect(),
            stiffness: beam.stiffness,
        };
        let prepared = prepare_rods(&poses, &[rod]).unwrap();
        let indices: Vec<_> = (0..3)
            .flat_map(|i| (0..3).map(move |axis| 6 * i + axis))
            .collect();
        let released = submatrix(&prepared[0].effective_stiffness, &indices, &indices);
        assert!((&released - &beam.translations).norm() < 1e-9 * beam.translations.norm());
    }
}
