//! Recover elastic strain before multiplying by a free rod's stiffness.
//!
//! A common rigid motion carries no strain. Subtract it at body centres
//! before applying head arms, rather than subtracting two large head motions
//! after multiplication by the material matrix. The equations are unchanged.

use super::RodGeometry;
use crate::chain::Pose;
use crate::springback::RodLinearisation;
use nalgebra::{DVector, Vector3};

pub(super) fn material_load(
    poses: &[Pose],
    rod: &RodLinearisation,
    local: &RodGeometry,
    displacement: &DVector<f64>,
) -> DVector<f64> {
    let frame = &local.frame_map * displacement;
    let translation = Vector3::from_column_slice(frame.rows(0, 3).as_slice());
    let turn = Vector3::from_column_slice(frame.rows(3, 3).as_slice());
    let mut relative = displacement.clone();
    for (i, pose) in poses.iter().enumerate() {
        let shift = Vector3::from_column_slice(displacement.rows(6 * i, 3).as_slice())
            - translation
            - turn.cross(&(pose.centre_mm - local.origin));
        let rotation =
            Vector3::from_column_slice(displacement.rows(6 * i + 3, 3).as_slice()) - turn;
        relative.rows_mut(6 * i, 3).copy_from(&shift);
        relative.rows_mut(6 * i + 3, 3).copy_from(&rotation);
    }
    let material = &rod.effective_stiffness * (&rod.kinematics * relative);
    // The exact material matrix annihilates all free frame modes. Remove
    // only their round-off residue after multiplication as well.
    &material - &local.rigid * &local.gram_inverse * (local.rigid.transpose() * &material)
}

/// Action of the same condensed tangent, retaining its body/frame terms.
pub(super) fn tangent_load(
    poses: &[Pose],
    tissue_load: DVector<f64>,
    rods: &[RodLinearisation],
    geometries: &[RodGeometry],
    displacement: &DVector<f64>,
) -> DVector<f64> {
    let mut result = tissue_load;
    for (rod, local) in rods.iter().zip(geometries) {
        let frame = &local.frame_map * displacement;
        let frame_load = local.mixed.transpose() * displacement + &local.frame * &frame;
        let body_load = &local.body * displacement + &local.mixed * &frame;
        let load = material_load(poses, rod, local, displacement)
            + &local.rigid * &local.gram_inverse * frame_load;
        result += body_load + rod.kinematics.transpose() * load;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::Wrench;
    use crate::rod_beams::{BeamProperties, element_stiffness};
    use crate::springback::{HeadTie, LockedRod, prepare_rods, solve_incremental};
    use nalgebra::{DMatrix, Matrix3};

    #[test]
    fn a_small_fixed_end_bend_keeps_its_load_under_a_large_common_motion() {
        let points = [Vector3::zeros(), Vector3::new(0.0, 0.0, 100.0)];
        let poses: Vec<_> = points
            .iter()
            .map(|&centre_mm| Pose {
                centre_mm,
                rotation: Matrix3::identity(),
            })
            .collect();
        // Synthetic 2 mm section: A=pi mm², I=pi/4 mm⁴. Fixed end
        // translation delta=0.01 mm gives F=12EI*delta/[L³(1+phi)]
        // and My=FL/2 at both ends, phi=12EI/(kappa*GA*L²).
        let properties = BeamProperties {
            young_modulus_mpa: 100_000.0,
            shear_modulus_mpa: 40_000.0,
            diameter_mm: 2.0,
            shear_coefficient: 0.8,
        };
        let element = element_stiffness(points[0], points[1], properties).unwrap();
        let rod = prepare_rods(
            &poses,
            &[LockedRod {
                ties: points
                    .iter()
                    .enumerate()
                    .map(|(body, &point_mm)| HeadTie {
                        body,
                        point_mm,
                        locks_rotation: true,
                    })
                    .collect(),
                stiffness: DMatrix::from_column_slice(12, 12, element.as_slice()),
            }],
        )
        .unwrap()
        .remove(0);
        let local = super::super::geometry(&poses, &rod, &[Wrench::zeros(); 2]).unwrap();
        let turn = Vector3::new(0.3, -0.2, 0.4);
        let translation = Vector3::new(1e6, -2e6, 3e6);
        let mut motion = DVector::zeros(12);
        for (i, pose) in poses.iter().enumerate() {
            motion
                .rows_mut(6 * i, 3)
                .copy_from(&(translation + turn.cross(&(pose.centre_mm - local.origin))));
            motion.rows_mut(6 * i + 3, 3).copy_from(&turn);
        }
        assert!(material_load(&poses, &rod, &local, &motion).norm() < 1e-7);
        motion[0] += 0.01;
        let force = 12.0 * 100_000.0 * core::f64::consts::PI / 4.0 * 0.01
            / (100.0 * 100.0 * 100.0 * (1.0 + 0.0009375));
        let mut expected = DVector::zeros(12);
        expected[0] = force;
        expected[6] = -force;
        expected[4] = force * 50.0;
        expected[10] = force * 50.0;
        let actual = material_load(&poses, &rod, &local, &motion);
        assert!((&actual - &expected).norm() < 1e-7, "{actual:?}");

        // The same independent answer supplies an ill-conditioned solve:
        // a 1000 mm common shift plus the 0.01 mm bend, with weak tissue.
        // These are linear algebra coordinates, not a finite pose proposal.
        let tissue = DMatrix::identity(12, 12) * 0.001;
        let mut answer = DVector::zeros(12);
        answer[0] = 1000.01;
        answer[6] = 1000.0;
        let applied = &tissue * &answer + expected;
        let solved = solve_incremental(
            &poses,
            &tissue,
            &applied,
            &[rod],
            &[vec![Wrench::zeros(); 2]],
        )
        .unwrap();
        assert!(solved.relative_residual < 1e-9);
        assert!((&solved.displacement - answer).norm() < 1e-6);
    }

    #[test]
    fn a_negative_incremental_mode_is_still_refused() {
        let poses = [Pose {
            centre_mm: Vector3::zeros(),
            rotation: Matrix3::identity(),
        }];
        let mut tissue = DMatrix::identity(6, 6);
        // The unit x displacement has independently negative energy -1/2.
        tissue[(0, 0)] = -1.0;
        let result = solve_incremental(&poses, &tissue, &DVector::zeros(6), &[], &[]);
        assert_eq!(
            result.unwrap_err(),
            crate::springback::SpringbackError::NotPositiveDefinite
        );
    }
}
