//! Preliminary constrained iterations in exact joint-deformation coordinates.
//!
//! Joint translations are mm in the current lower frame; joint rotations
//! are the principal relative log in radians. Forward kinematics preserves
//! the original tissue energy exactly, so its coordinate Hessian is diagonal.
//! Newton polish uses exact point-constraint curvature and returns only a starting
//! point for the body-coordinate solve, which still checks the full KKT
//! residual and the true reduced Lagrangian stiffness.
//!
//! The curvature transform is specific to H: body constraints are sums of
//! head-point positions, and their rod/slide terms are additive. In particular,
//! this adapter must not be used for R's relative rotational lock equations.

use super::minimise::{self, Evaluation, FirstOrder, Model, Options};
use super::{HeldGuess, LENGTH_MM};
use crate::chain::{Chain, Wrench};
use nalgebra::{DMatrix, DVector, Matrix3, Vector3};
use std::ops::AddAssign;

/// Project cap and tolerances for preliminary iterations, not acceptance of
/// the final physical state or its loads.
const WARM_ITERATIONS: u32 = 20;
const WARM_CONSTRAINT_TOLERANCE: f64 = 1e-7;
const WARM_ABSOLUTE_STATIONARITY: f64 = 1e-9;
/// Project handoff level for Newton polish: once nearby, the strict body
/// solve is cheaper and supplies the final physical acceptance criteria.
const NEWTON_HANDOFF_STATIONARITY_TOLERANCE: f64 = 1e-2;
/// Project numerical switch for the cancellation-free SO(3) series.
const JACOBIAN_SERIES_SQUARED_ANGLE: f64 = 0.01;

pub(super) struct WarmStart {
    pub state: HeldGuess,
    pub iterations: u32,
    pub constraint_max: f64,
    pub stationarity_max: f64,
}

#[derive(Clone)]
struct JointGuess {
    deformation: Vec<Wrench>,
    held: HeldGuess,
}

struct Coordinates<'a, M> {
    model: &'a M,
    chain: &'a Chain,
    stiffness: Vec<Wrench>,
    dimension: usize,
}

/// Differential of Exp(phi) under additive changes of phi, expressed as a
/// left rotation increment: dExp(phi) = [J_l(phi) dphi] Exp(phi).
fn left_jacobian(phi: &Vector3<f64>) -> Matrix3<f64> {
    let squared = phi.norm_squared();
    let (a, b) = if squared < JACOBIAN_SERIES_SQUARED_ANGLE {
        (
            0.5 - squared / 24.0 + squared * squared / 720.0
                - squared * squared * squared / 40320.0,
            1.0 / 6.0 - squared / 120.0 + squared * squared / 5040.0
                - squared * squared * squared / 362880.0,
        )
    } else {
        let angle = squared.sqrt();
        let half = angle * 0.5;
        let sinc_half = libm::sin(half) / half;
        (
            0.5 * sinc_half * sinc_half,
            (1.0 - libm::sin(angle) / angle) / squared,
        )
    };
    let cross = phi.cross_matrix();
    Matrix3::identity() + cross * a + cross * cross * b
}

impl<M: Model<State = HeldGuess>> Coordinates<'_, M> {
    fn energy(&self, state: &JointGuess) -> f64 {
        state
            .deformation
            .iter()
            .zip(&self.stiffness)
            .map(|(q, k)| {
                (0..6)
                    .map(|axis| 0.5 * k[axis] * q[axis] * q[axis])
                    .sum::<f64>()
            })
            .sum()
    }

    fn energy_gradient(&self, state: &JointGuess) -> (f64, DVector<f64>) {
        let mut gradient = DVector::zeros(self.dimension);
        for (joint, (q, stiffness)) in state.deformation.iter().zip(&self.stiffness).enumerate() {
            for axis in 0..6 {
                let scale = if axis < 3 { LENGTH_MM } else { 1.0 };
                gradient[6 * joint + axis] = stiffness[axis] * q[axis] * scale;
            }
        }
        (self.energy(state), gradient)
    }

    fn energy_derivatives(&self, state: &JointGuess) -> (f64, DVector<f64>, DMatrix<f64>) {
        let (energy, gradient) = self.energy_gradient(state);
        let mut hessian = DMatrix::zeros(self.dimension, self.dimension);
        for (joint, stiffness) in self.stiffness.iter().enumerate() {
            for axis in 0..6 {
                let scale = if axis < 3 { LENGTH_MM } else { 1.0 };
                hessian[(6 * joint + axis, 6 * joint + axis)] = stiffness[axis] * scale * scale;
            }
        }
        (energy, gradient, hessian)
    }

    fn constraint_derivatives(&self, state: &JointGuess) -> Option<(DVector<f64>, DMatrix<f64>)> {
        let (constraint, body_jacobian) = self.model.constraint_data(&state.held)?;
        if body_jacobian.ncols() != self.dimension || body_jacobian.nrows() != constraint.len() {
            return None;
        }
        let count = state.deformation.len();
        let frames = self.chain.joint_motion_frames(&state.held.poses).ok()?;
        let rows = constraint.len();
        let mut jacobian = DMatrix::zeros(rows, self.dimension);
        let source = body_jacobian.as_slice();
        let target = jacobian.as_mut_slice();
        let suffix = 6 * count * rows;
        target[suffix..].copy_from_slice(&source[suffix..]);
        let rotation_maps: Vec<_> = frames
            .iter()
            .enumerate()
            .map(|(joint, frame)| {
                frame.lower_rotation
                    * left_jacobian(&state.deformation[joint].fixed_rows::<3>(3).into_owned())
            })
            .collect();
        // H has sparse point rows. Read each body block once, and expand only
        // its nonzero rows through the ancestors using contiguous matrix data.
        for body in 0..count {
            let base = 6 * body * rows;
            for row in 0..rows {
                let translation = [
                    source[base + row],
                    source[base + rows + row],
                    source[base + 2 * rows + row],
                ];
                let rotation = [
                    source[base + 3 * rows + row],
                    source[base + 4 * rows + row],
                    source[base + 5 * rows + row],
                ];
                if translation == [0.0; 3] && rotation == [0.0; 3] {
                    continue;
                }
                for joint in body..count {
                    let frame = &frames[joint];
                    let arm = state.held.poses[body].centre_mm - frame.upper_point_mm;
                    let angular = [
                        (arm.y * translation[2] - arm.z * translation[1]) / LENGTH_MM + rotation[0],
                        (arm.z * translation[0] - arm.x * translation[2]) / LENGTH_MM + rotation[1],
                        (arm.x * translation[1] - arm.y * translation[0]) / LENGTH_MM + rotation[2],
                    ];
                    let linear_map = frame.lower_rotation.as_slice();
                    let angular_map = rotation_maps[joint].as_slice();
                    for axis in 0..3 {
                        let offset = 3 * axis;
                        target[(6 * joint + axis) * rows + row] += translation[0]
                            * linear_map[offset]
                            + translation[1] * linear_map[offset + 1]
                            + translation[2] * linear_map[offset + 2];
                        target[(6 * joint + 3 + axis) * rows + row] += angular[0]
                            * angular_map[offset]
                            + angular[1] * angular_map[offset + 1]
                            + angular[2] * angular_map[offset + 2];
                    }
                }
            }
        }
        Some((constraint, jacobian))
    }

    fn body_increment(&self, state: &JointGuess, increment: &DVector<f64>) -> Option<DVector<f64>> {
        let frames = self.chain.joint_motion_frames(&state.held.poses).ok()?;
        let mut result = increment.clone();
        result.rows_mut(0, 6 * state.deformation.len()).fill(0.0);
        for (joint, frame) in frames.iter().enumerate() {
            let translation = frame.lower_rotation * increment.fixed_rows::<3>(6 * joint);
            let turn = frame.lower_rotation
                * left_jacobian(&state.deformation[joint].fixed_rows::<3>(3).into_owned())
                * increment.fixed_rows::<3>(6 * joint + 3);
            for body in 0..=joint {
                let arm = state.held.poses[body].centre_mm - frame.upper_point_mm;
                let movement = translation + turn.cross(&arm) / LENGTH_MM;
                for axis in 0..3 {
                    result[6 * body + axis] += movement[axis];
                    result[6 * body + 3 + axis] += turn[axis];
                }
            }
        }
        Some(result)
    }
}

impl<M: Model<State = HeldGuess>> Coordinates<'_, M> {
    /// Recover each point-force arm matrix from its physical force, moment
    /// and symmetric body rotational Hessian, then accumulate subtree resultants.
    /// This costs quadratic work in joint count and never forms a tissue tangent.
    fn exact_constraint_hessian(
        &self,
        state: &JointGuess,
        lambda: &DVector<f64>,
        energy_hessian: &DMatrix<f64>,
    ) -> Option<DMatrix<f64>> {
        let (constraint, body_jacobian) = self.model.constraint_data(&state.held)?;
        let body_gradient = body_jacobian.transpose() * lambda;
        let data = Evaluation {
            energy: 0.0,
            gradient: DVector::zeros(self.dimension),
            hessian: DMatrix::zeros(self.dimension, self.dimension),
            constraint,
            jacobian: body_jacobian,
        };
        let body_hessian = self.model.lagrangian_hessian(&state.held, lambda, &data)?;
        let count = state.deformation.len();
        let prefix_size = 6 * count;
        let frames = self.chain.joint_motion_frames(&state.held.poses).ok()?;
        let maps: Vec<_> = frames
            .iter()
            .enumerate()
            .map(|(j, f)| {
                f.lower_rotation
                    * left_jacobian(&state.deformation[j].fixed_rows::<3>(3).into_owned())
            })
            .collect();
        let mut result = energy_hessian.clone();
        if self.dimension > prefix_size {
            result
                .view_mut(
                    (prefix_size, prefix_size),
                    (self.dimension - prefix_size, self.dimension - prefix_size),
                )
                .add_assign(body_hessian.view(
                    (prefix_size, prefix_size),
                    (self.dimension - prefix_size, self.dimension - prefix_size),
                ));
        }
        // For a subtree above pivot p: F=sum f, M=sum((c-p) cross f+m),
        // A=sum((c-p) f^T-I*(c-p).f + S-.5[m]x).  The antisymmetric part
        // restores the ordered angular derivative before the log-map chain rule.
        let mut force = Vector3::zeros();
        let mut moment = Vector3::zeros();
        let mut arm_matrix = Matrix3::zeros();
        for joint in 0..count {
            let f = body_gradient.fixed_rows::<3>(6 * joint).into_owned() / LENGTH_MM;
            let m = body_gradient.fixed_rows::<3>(6 * joint + 3).into_owned();
            let c = state.held.poses[joint].centre_mm;
            let symmetric = body_hessian
                .fixed_view::<3, 3>(6 * joint + 3, 6 * joint + 3)
                .into_owned();
            force += f;
            moment += c.cross(&f) + m;
            arm_matrix += c * f.transpose() - Matrix3::identity() * c.dot(&f) + symmetric
                - m.cross_matrix() * 0.5;
            let pivot = frames[joint].upper_point_mm;
            let a =
                arm_matrix - pivot * force.transpose() + Matrix3::identity() * pivot.dot(&force);
            let local_moment =
                frames[joint].lower_rotation.transpose() * (moment - pivot.cross(&force));
            let derivatives = left_jacobian_derivatives(
                &state.deformation[joint].fixed_rows::<3>(3).into_owned(),
            );
            let mut diagonal = maps[joint].transpose() * a * maps[joint];
            for (axis, derivative) in derivatives.iter().enumerate() {
                diagonal
                    .column_mut(axis)
                    .add_assign(derivative.transpose() * local_moment);
            }
            result
                .view_mut((6 * joint + 3, 6 * joint + 3), (3, 3))
                .add_assign(diagonal);
            let rotation_left = maps[joint].transpose() * a.transpose();
            let mixed_left =
                frames[joint].lower_rotation.transpose() * force.cross_matrix() * LENGTH_MM;
            for (ancestor, map) in maps.iter().enumerate().skip(joint + 1) {
                let rotation = rotation_left * map;
                result
                    .view_mut((6 * joint + 3, 6 * ancestor + 3), (3, 3))
                    .add_assign(rotation);
                result
                    .view_mut((6 * ancestor + 3, 6 * joint + 3), (3, 3))
                    .add_assign(rotation.transpose());
                let mixed = mixed_left * map;
                result
                    .view_mut((6 * joint, 6 * ancestor + 3), (3, 3))
                    .add_assign(mixed);
                result
                    .view_mut((6 * ancestor + 3, 6 * joint), (3, 3))
                    .add_assign(mixed.transpose());
            }
        }
        let dimension = result.nrows();
        let values = result.as_mut_slice();
        for column in 0..dimension {
            for row in 0..column {
                let average =
                    (values[column * dimension + row] + values[row * dimension + column]) * 0.5;
                values[column * dimension + row] = average;
                values[row * dimension + column] = average;
            }
        }
        Some(result)
    }
}

fn left_jacobian_derivatives(phi: &Vector3<f64>) -> [Matrix3<f64>; 3] {
    let squared = phi.norm_squared();
    let (a, b, da, db) = if squared < JACOBIAN_SERIES_SQUARED_ANGLE {
        let s = squared;
        (
            0.5 - s / 24.0 + s * s / 720.0 - s * s * s / 40320.0,
            1.0 / 6.0 - s / 120.0 + s * s / 5040.0 - s * s * s / 362880.0,
            -1.0 / 24.0 + s / 360.0 - s * s / 13440.0,
            -1.0 / 120.0 + s / 2520.0 - s * s / 120960.0,
        )
    } else {
        let angle = squared.sqrt();
        let sinc = libm::sin(angle) / angle;
        let cos = libm::cos(angle);
        let a = (1.0 - cos) / squared;
        let b = (1.0 - sinc) / squared;
        (
            a,
            b,
            (0.5 * angle * libm::sin(angle) - (1.0 - cos)) / (squared * squared),
            (1.5 * sinc - 0.5 * cos - 1.0) / (squared * squared),
        )
    };
    let cross = phi.cross_matrix();
    core::array::from_fn(|axis| {
        let mut unit = Vector3::zeros();
        unit[axis] = 1.0;
        let e = unit.cross_matrix();
        cross * (2.0 * phi[axis] * da)
            + e * a
            + cross * cross * (2.0 * phi[axis] * db)
            + (e * cross + cross * e) * b
    })
}

impl<M: Model<State = HeldGuess>> Model for Coordinates<'_, M> {
    type State = JointGuess;

    fn lagrangian_hessian(
        &self,
        state: &JointGuess,
        multipliers: &DVector<f64>,
        evaluation: &Evaluation,
    ) -> Option<DMatrix<f64>> {
        self.exact_constraint_hessian(state, multipliers, &evaluation.hessian)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn evaluate(&self, state: &JointGuess) -> Option<Evaluation> {
        let (energy, gradient, hessian) = self.energy_derivatives(state);
        let (constraint, jacobian) = self.constraint_derivatives(state)?;
        Some(Evaluation {
            energy,
            gradient,
            hessian,
            constraint,
            jacobian,
        })
    }

    fn first_order(&self, state: &JointGuess) -> Option<FirstOrder> {
        let (energy, gradient) = self.energy_gradient(state);
        let (constraint, jacobian) = self.constraint_derivatives(state)?;
        Some(FirstOrder {
            energy,
            gradient,
            constraint,
            jacobian,
        })
    }

    fn value(&self, state: &JointGuess) -> Option<(f64, DVector<f64>)> {
        let energy = self.energy(state);
        let (constraint, _) = self.model.constraint_data(&state.held)?;
        Some((energy, constraint))
    }

    fn constraint_data(&self, state: &JointGuess) -> Option<(DVector<f64>, DMatrix<f64>)> {
        self.constraint_derivatives(state)
    }

    fn step_measure(&self, state: &JointGuess, increment: &DVector<f64>) -> f64 {
        self.body_increment(state, increment)
            .map(|body_increment| self.model.step_measure(&state.held, &body_increment))
            .unwrap_or(f64::INFINITY)
    }

    fn step(&self, state: &JointGuess, increment: &DVector<f64>) -> Option<JointGuess> {
        let mut next = state.clone();
        for (joint, q) in next.deformation.iter_mut().enumerate() {
            for axis in 0..6 {
                q[axis] += increment[6 * joint + axis] * if axis < 3 { LENGTH_MM } else { 1.0 };
            }
            if q.fixed_rows::<3>(3).norm() >= core::f64::consts::PI {
                return None;
            }
        }
        let mut suffix = increment.clone();
        suffix.rows_mut(0, 6 * next.deformation.len()).fill(0.0);
        next.held = self.model.step(&state.held, &suffix)?;
        next.held.poses = self.chain.poses_from_deformations(&next.deformation).ok()?;
        Some(next)
    }
}

/// Polish an already nearby H state using its exact joint-coordinate
/// Lagrangian Hessian. The body's strict constrained solve must still establish
/// settled status, physical multipliers and reduced stability.
pub(super) fn newton_polish<M: Model<State = HeldGuess>>(
    model: &M,
    chain: &Chain,
    initial: HeldGuess,
) -> Option<WarmStart> {
    let count = chain.reference.len();
    if model.dimension() < 6 * count || initial.poses.len() != count {
        return None;
    }
    let stiffness = chain
        .joint_stiffnesses()
        .into_iter()
        .map(|k| k.0 / chain.stiffness_scale())
        .collect();
    let coordinates = Coordinates {
        model,
        chain,
        stiffness,
        dimension: model.dimension(),
    };
    let deformation = chain.evaluate(&initial.poses).ok()?.deformation;
    let mut held = initial;
    held.poses = chain.poses_from_deformations(&deformation).ok()?;
    let result = minimise::minimise(
        &coordinates,
        JointGuess { deformation, held },
        Options {
            iteration_limit: WARM_ITERATIONS,
            constraint_tolerance: WARM_CONSTRAINT_TOLERANCE,
            stationarity_tolerance: NEWTON_HANDOFF_STATIONARITY_TOLERANCE,
            absolute_stationarity_tolerance: WARM_ABSOLUTE_STATIONARITY,
            ..Options::default()
        },
    );
    Some(WarmStart {
        state: result.state.held,
        iterations: result.iterations,
        constraint_max: result.constraint_max,
        stationarity_max: result.stationarity_max,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::Ground;
    use crate::joint_springs::Stiffness;
    use forward_model::rotation::rot_y;
    use shapes::levels::Level;
    use shapes::made_up::VertebraPose;

    struct Points {
        chain: Chain,
    }
    impl Points {
        fn constraints(&self, state: &HeldGuess) -> (DVector<f64>, DMatrix<f64>) {
            let mut value = DVector::zeros(3 * state.poses.len());
            let mut jacobian = DMatrix::zeros(value.len(), 6 * state.poses.len());
            let local = Vector3::new(-13.0, 4.0, 2.0);
            for (body, pose) in state.poses.iter().enumerate() {
                let arm = pose.rotation * local;
                value
                    .rows_mut(3 * body, 3)
                    .copy_from(&((pose.centre_mm + arm) / LENGTH_MM));
                jacobian
                    .view_mut((3 * body, 6 * body), (3, 3))
                    .copy_from(&Matrix3::identity());
                jacobian
                    .view_mut((3 * body, 6 * body + 3), (3, 3))
                    .copy_from(&(-arm.cross_matrix() / LENGTH_MM));
            }
            (value, jacobian)
        }
    }
    impl Model for Points {
        type State = HeldGuess;
        fn lagrangian_hessian(
            &self,
            state: &HeldGuess,
            multipliers: &DVector<f64>,
            evaluation: &Evaluation,
        ) -> Option<DMatrix<f64>> {
            let mut hessian = evaluation.hessian.clone();
            let local = Vector3::new(-13.0, 4.0, 2.0);
            for (body, pose) in state.poses.iter().enumerate() {
                let arm = pose.rotation * local;
                let force = multipliers.fixed_rows::<3>(3 * body).into_owned() / LENGTH_MM;
                let curvature = (arm * force.transpose() + force * arm.transpose()) * 0.5
                    - Matrix3::identity() * arm.dot(&force);
                hessian
                    .view_mut((6 * body + 3, 6 * body + 3), (3, 3))
                    .add_assign(curvature);
            }
            Some(hessian)
        }

        fn dimension(&self) -> usize {
            6 * self.chain.reference.len()
        }
        fn evaluate(&self, state: &HeldGuess) -> Option<Evaluation> {
            let value = self.chain.evaluate(&state.poses).ok()?;
            let (constraint, jacobian) = self.constraints(state);
            Some(Evaluation {
                energy: value.energy_nmm / self.chain.stiffness_scale(),
                gradient: DVector::zeros(self.dimension()),
                hessian: DMatrix::zeros(self.dimension(), self.dimension()),
                constraint,
                jacobian,
            })
        }
        fn constraint_data(&self, state: &HeldGuess) -> Option<(DVector<f64>, DMatrix<f64>)> {
            Some(self.constraints(state))
        }
        fn step(&self, state: &HeldGuess, delta: &DVector<f64>) -> Option<HeldGuess> {
            let poses = state
                .poses
                .iter()
                .enumerate()
                .map(|(body, p)| {
                    let mut increment =
                        Wrench::from_column_slice(&delta.as_slice()[6 * body..6 * body + 6]);
                    for axis in 0..3 {
                        increment[axis] *= LENGTH_MM;
                    }
                    p.incremented(&increment)
                })
                .collect();
            Some(HeldGuess {
                poses,
                ..state.clone()
            })
        }
    }

    fn setup() -> (Points, JointGuess) {
        let reference: Vec<_> = [Level::L3, Level::L4, Level::L5]
            .iter()
            .enumerate()
            .map(|(i, &level)| VertebraPose {
                level,
                centre: [2.0 * i as f64, -(i as f64), 70.0 - 30.0 * i as f64],
                rotation: [0.04 * i as f64, -0.03, 0.02],
                height: 20.0,
                wedge_front: 0.0,
                wedge_side: 0.0,
                top_half_width: 15.0,
                top_half_depth: 10.0,
                bottom_half_width: 15.0,
                bottom_half_depth: 10.0,
            })
            .collect();
        let stiffness = Stiffness::new([7.0, 9.0, 13.0, 17.0, 19.0, 23.0]).unwrap();
        let chain = Chain::new(
            &reference,
            &[stiffness; 3],
            Ground {
                top_mm: Vector3::new(0.0, 0.0, -4.0),
                rotation: rot_y(0.12),
            },
        )
        .unwrap();
        let deformation = vec![
            Wrench::new(0.4, -0.2, 0.1, 0.3, -0.2, 0.1),
            Wrench::new(-0.2, 0.1, 0.3, -0.15, 0.22, 0.11),
            Wrench::new(0.1, -0.3, 0.2, 0.17, 0.12, -0.19),
        ];
        let poses = chain.poses_from_deformations(&deformation).unwrap();
        let held = HeldGuess {
            poses,
            rod_poses: Vec::new(),
            stations_mm: Vec::new(),
        };
        (Points { chain }, JointGuess { deformation, held })
    }

    #[test]
    fn exact_joint_energy_and_its_gradient_match_the_body_chain() {
        let (model, state) = setup();
        let coordinates = Coordinates {
            model: &model,
            chain: &model.chain,
            stiffness: model
                .chain
                .joint_stiffnesses()
                .into_iter()
                .map(|k| k.0 / model.chain.stiffness_scale())
                .collect(),
            dimension: model.dimension(),
        };
        let e = coordinates.evaluate(&state).unwrap();
        let physical = model.chain.evaluate(&state.held.poses).unwrap();
        assert!((e.energy - physical.energy_nmm / model.chain.stiffness_scale()).abs() < 1e-12);
        for column in 0..model.dimension() {
            let mut increment = DVector::zeros(model.dimension());
            increment[column] = 1e-6;
            let plus = coordinates.step(&state, &increment).unwrap();
            let minus = coordinates.step(&state, &(-increment)).unwrap();
            let difference =
                (coordinates.value(&plus).unwrap().0 - coordinates.value(&minus).unwrap().0) / 2e-6;
            assert!(
                (difference - e.gradient[column]).abs() < 1e-8,
                "{column}: {difference} {}",
                e.gradient[column]
            );
        }
    }

    #[test]
    fn tree_jacobian_matches_independent_forward_kinematics_differences() {
        let (model, state) = setup();
        let coordinates = Coordinates {
            model: &model,
            chain: &model.chain,
            stiffness: model
                .chain
                .joint_stiffnesses()
                .into_iter()
                .map(|k| k.0 / model.chain.stiffness_scale())
                .collect(),
            dimension: model.dimension(),
        };
        let (_, jacobian) = coordinates.constraint_derivatives(&state).unwrap();
        for column in 0..model.dimension() {
            let mut increment = DVector::zeros(model.dimension());
            increment[column] = 1e-6;
            let plus = coordinates.step(&state, &increment).unwrap();
            let minus = coordinates.step(&state, &(-increment)).unwrap();
            let difference =
                (model.constraints(&plus.held).0 - model.constraints(&minus.held).0) / 2e-6;
            assert!(
                (&difference - jacobian.column(column)).norm() < 2e-9,
                "column {column}: {}",
                (&difference - jacobian.column(column)).norm()
            );
        }
    }
    #[test]
    fn exact_joint_lagrangian_hessian_matches_independent_gradient_differences() {
        let (model, state) = setup();
        let coordinates = Coordinates {
            model: &model,
            chain: &model.chain,
            stiffness: model
                .chain
                .joint_stiffnesses()
                .into_iter()
                .map(|k| k.0 / model.chain.stiffness_scale())
                .collect(),
            dimension: model.dimension(),
        };
        let evaluation = coordinates.evaluate(&state).unwrap();
        let multipliers = DVector::from_fn(evaluation.constraint.len(), |i, _| {
            ((i * 7 % 11) as f64 - 5.0) * 0.3
        });
        let hessian = coordinates
            .lagrangian_hessian(&state, &multipliers, &evaluation)
            .unwrap();
        let mut differences = DMatrix::zeros(model.dimension(), model.dimension());
        for column in 0..model.dimension() {
            let mut increment = DVector::zeros(model.dimension());
            increment[column] = 1e-6;
            let plus = coordinates.step(&state, &increment).unwrap();
            let minus = coordinates.step(&state, &(-increment)).unwrap();
            let ep = coordinates.evaluate(&plus).unwrap();
            let em = coordinates.evaluate(&minus).unwrap();
            differences.set_column(
                column,
                &((ep.gradient + ep.jacobian.transpose() * &multipliers
                    - em.gradient
                    - em.jacobian.transpose() * &multipliers)
                    / 2e-6),
            );
        }
        let error = (&differences - &hessian).abs().max();
        assert!(
            error < 2e-7,
            "joint-coordinate Lagrangian Hessian error {error}"
        );
    }
}
