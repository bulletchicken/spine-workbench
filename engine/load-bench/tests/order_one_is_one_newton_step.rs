//! First-order springback and its quadratic nonlinear remainder.

use load_bench::chain::{Chain, Ground, Pose, Wrench};
use load_bench::joint_springs::Stiffness;
use load_bench::rod_beams::{BeamProperties, CondensedRod, polyline_stations};
use load_bench::springback::{HeadTie, LockedRod, solve};
use nalgebra::{DVector, Matrix3, Vector3};
use shapes::levels::Level;
use shapes::made_up::VertebraPose;

fn case() -> (Chain, Vec<Pose>) {
    let reference: Vec<_> = [Level::L3, Level::L4, Level::L5]
        .iter()
        .enumerate()
        .map(|(i, &level)| VertebraPose {
            level,
            centre: [0.0, 0.0, 100.0 - 40.0 * i as f64],
            rotation: [0.0; 3],
            height: 20.0,
            wedge_front: 0.0,
            wedge_side: 0.0,
            top_half_width: 12.0,
            top_half_depth: 8.0,
            bottom_half_width: 12.0,
            bottom_half_depth: 8.0,
        })
        .collect();
    let k = Stiffness::new([80.0, 120.0, 200.0, 3000.0, 5000.0, 4000.0]).unwrap();
    let chain = Chain::new(
        &reference,
        &[k; 3],
        Ground {
            top_mm: Vector3::zeros(),
            rotation: Matrix3::identity(),
        },
    )
    .unwrap();
    let mut poses = chain.standing();
    // A prescribed rigid block's inner shape. Its lowest body remains at
    // rest, so every joint outside the block is at rest; a common block
    // motion cannot reduce its inner energy. This is a state-R closed form.
    poses[0] = poses[0].incremented(&Wrench::new(1.0, 0.0, 0.0, 0.0, 0.08, 0.0));
    poses[1] = poses[1].incremented(&Wrench::new(-0.8, 0.0, 0.0, 0.0, -0.03, 0.0));
    (chain, poses)
}

fn rods(poses: &[Pose], scale: f64) -> Vec<LockedRod> {
    [-9.0, 9.0]
        .iter()
        .map(|&side| {
            let points: Vec<_> = poses
                .iter()
                .map(|p| p.centre_mm + p.rotation * Vector3::new(-15.0, side, 0.0))
                .collect();
            let stations = polyline_stations(&points).unwrap();
            let properties = BeamProperties {
                young_modulus_mpa: 70_000.0 * scale,
                shear_modulus_mpa: 28_000.0 * scale,
                diameter_mm: 5.0,
                shear_coefficient: 0.85,
            };
            let beam = CondensedRod::new(&points, &stations, properties).unwrap();
            LockedRod {
                ties: points
                    .iter()
                    .enumerate()
                    .map(|(body, &point_mm)| HeadTie {
                        body,
                        point_mm,
                        locks_rotation: true,
                    })
                    .collect(),
                stiffness: beam.stiffness,
            }
        })
        .collect()
}

fn nonlinear_chart_residual(
    chain: &Chain,
    state: &load_bench::springback::SeatedState,
) -> DVector<f64> {
    use forward_model::rotation::{matrix_from_rotation_vector, rotation_vector_from_matrix};
    let count = state.rigid_poses.len();
    let mut coordinates = state.displacement.as_slice().to_vec();
    coordinates.extend(
        state
            .rod_displacements
            .iter()
            .flat_map(|w| w.iter().copied()),
    );
    // Independently differentiate the exact complementary-energy Lagrangian
    // at the single returned step. Multipliers stay fixed while varying x.
    // This includes the finite moving-rod strains, rather than freezing a
    // linear rod in the patient frame under a loaded block rotation.
    let value = |x: &[f64]| {
        let poses: Vec<_> = state
            .rigid_poses
            .iter()
            .enumerate()
            .map(|(i, p)| p.incremented(&Wrench::from_column_slice(&x[6 * i..6 * i + 6])))
            .collect();
        let mut energy = chain.evaluate(&poses).unwrap().energy_nmm;
        for (r, rod) in state.rods.iter().enumerate() {
            let offset = 6 * (count + r);
            let translation = Vector3::from_column_slice(&x[offset..offset + 3]);
            let q = matrix_from_rotation_vector(&Vector3::from_column_slice(
                &x[offset + 3..offset + 6],
            ));
            let origin = state.rod_origins_mm[r];
            for (h, tie) in rod.ties.iter().enumerate() {
                let reference = &state.rigid_poses[tie.body];
                let local = reference.rotation.transpose() * (tie.point_mm - reference.centre_mm);
                let head = poses[tie.body].centre_mm + poses[tie.body].rotation * local;
                let strain =
                    q.transpose() * (head - origin - translation) - (tie.point_mm - origin);
                let load = state.material_head_wrenches[r][h];
                energy += load.fixed_rows::<3>(0).dot(&strain);
                if tie.locks_rotation {
                    let turn = rotation_vector_from_matrix(
                        &(q.transpose()
                            * poses[tie.body].rotation
                            * reference.rotation.transpose()),
                    );
                    energy += load.fixed_rows::<3>(3).dot(&turn);
                }
            }
        }
        energy
    };
    let mut gradient = DVector::zeros(coordinates.len());
    for axis in 0..coordinates.len() {
        let step = if axis % 6 < 3 { 2e-5 } else { 2e-6 };
        let mut plus = coordinates.clone();
        let mut minus = coordinates.clone();
        plus[axis] += step;
        minus[axis] -= step;
        gradient[axis] = (value(&plus) - value(&minus)) / (2.0 * step);
    }
    gradient
}

#[test]
fn halving_compliance_quarters_the_nonlinear_remainder() {
    let (chain, poses) = case();
    let first = solve(&chain, &poses, &rods(&poses, 30.0), &poses).unwrap();
    let second = solve(&chain, &poses, &rods(&poses, 60.0), &poses).unwrap();
    assert!(first.linear_relative_residual < 1e-9 && second.linear_relative_residual < 1e-9);
    let r1 = nonlinear_chart_residual(&chain, &first).norm();
    let r2 = nonlinear_chart_residual(&chain, &second).norm();
    let ratio = r1 / r2;
    assert!(
        (3.5..4.5).contains(&ratio),
        "remainders {r1:e}, {r2:e}; ratio {ratio}"
    );
}

#[test]
fn multiplying_all_stiffnesses_preserves_the_elastic_step() {
    let (chain, poses) = case();
    let first = solve(&chain, &poses, &rods(&poses, 10.0), &poses).unwrap();
    let reference = chain.reference.clone();
    let k = Stiffness::new([80.0, 120.0, 200.0, 3000.0, 5000.0, 4000.0])
        .unwrap()
        .scaled(4.0, [1.0; 6])
        .unwrap();
    let scaled = Chain::new(
        &reference,
        &[k; 3],
        Ground {
            top_mm: Vector3::zeros(),
            rotation: Matrix3::identity(),
        },
    )
    .unwrap();
    let second = solve(&scaled, &poses, &rods(&poses, 40.0), &poses).unwrap();
    assert!((&first.displacement - &second.displacement).norm() < 1e-10);
    for (a, b) in first
        .head_wrenches_on_rod
        .iter()
        .flatten()
        .zip(second.head_wrenches_on_rod.iter().flatten())
    {
        assert!((a * 4.0 - b).norm() < 1e-7);
    }
}
