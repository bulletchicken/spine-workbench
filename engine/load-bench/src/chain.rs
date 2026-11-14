//! Vertebrae connected by objective six-axis springs at shared disc points.
//!
//! Patient x is anterior, y patient-left, z up; mm, N, N·mm and radians.
//! Pose rotations map body axes onto patient axes. Increments compose on the
//! LEFT, about patient axes. +Mx tips up toward −y, +My tips up toward +x,
//! +Mz turns anterior toward +y. `gradient` is the holding wrench at each
//! body centre; the joint wrench ON the vertebra is its negative. The force
//! from bone on a screw is opposite the screw's force on its vertebra.
//! Rod-on-screw force equals screw-on-vertebra force for a massless screw.
//! Positive pull-out is rod-on-screw force opposite its inward insertion axis.
//!
//! Standing poses define zero energy. Each reference disc point is the
//! midpoint of facing endplate centres. Its two material attachments move
//! with their bodies; translations and exact rotation logs are measured in
//! the lower body's current frame. This preserves free-body balance at
//! finite rotations, including translational spring lever arms. The fixed
//! sacrum has an explicit frame and top point; no sacrum geometry is guessed.
//!
//! Both derivatives are analytic. The tangent is the exact Hessian of
//! `E(c + v, Exp(omega) R)` at zero increments. A derivative of spatial
//! moments alone has a moving-basis skew; the exponential-chart correction
//! is `+0.5 [moment]x` on each rotational diagonal. Force-dependent geometric
//! stiffness is retained, even when it is negative away from a stable state.
//! Only neighbouring bodies couple, so the exact blocks feed the bordered
//! chain factorisation without a dense chain solve.

use crate::joint_springs::{
    Stiffness, log_left_derivative, rotation_deformation,
    rotation_response_tangent_with_material_scale,
};
use forward_model::endplates::Endplates;
use forward_model::rotation::{matrix_from_rotation_vector, rotation_vector_from_matrix};
use forward_model::vertebra_params::VertebraParams;
use nalgebra::{Matrix3, Matrix6, SVector, Vector3};
use shapes::made_up::VertebraPose;

/// A body's centre (mm) and orientation (body axes to patient axes).
#[derive(Clone, Debug, PartialEq)]
pub struct Pose {
    pub centre_mm: Vector3<f64>,
    pub rotation: Matrix3<f64>,
}

impl Pose {
    pub fn from_vertebra(pose: &VertebraPose) -> Self {
        Self {
            centre_mm: pose.centre.into(),
            rotation: matrix_from_rotation_vector(&pose.rotation.into()),
        }
    }

    /// Compose a translation and patient-frame rotation at the body centre.
    pub fn incremented(&self, step: &SVector<f64, 6>) -> Self {
        Self {
            centre_mm: self.centre_mm + step.fixed_rows::<3>(0),
            rotation: matrix_from_rotation_vector(&step.fixed_rows::<3>(3).into_owned())
                * self.rotation,
        }
    }

    /// Carry this pose back onto a vertebra's unchanged sizes and wedges.
    pub fn to_vertebra(&self, reference: &VertebraPose) -> VertebraPose {
        VertebraPose {
            centre: self.centre_mm.into(),
            rotation: rotation_vector_from_matrix(&self.rotation).into(),
            ..*reference
        }
    }

    fn usable(&self) -> bool {
        self.centre_mm
            .iter()
            .chain(self.rotation.iter())
            .all(|x| x.is_finite())
            && (self.rotation.transpose() * self.rotation - Matrix3::identity())
                .abs()
                .max()
                < 1e-10
            && (self.rotation.determinant() - 1.0).abs() < 1e-10
    }
}

/// Ground pose: its centre is the sacrum top, and its rotation is the
/// sacrum frame. Both remain fixed while the chain moves.
#[derive(Clone, Debug)]
pub struct Ground {
    pub top_mm: Vector3<f64>,
    pub rotation: Matrix3<f64>,
}

/// Force and moment about the body's centre, in patient axes.
pub type Wrench = SVector<f64, 6>;

/// Bad plain inputs are refused before any assembly.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ChainError {
    #[error("chain-needs-one-stiffness-per-disc-including-sacrum")]
    Dimensions,
    #[error("chain-pose-is-not-finite-or-rigid")]
    InvalidPose,
    #[error("chain-stiffness-is-not-finite-and-positive")]
    InvalidStiffness,
}

#[derive(Clone, Debug)]
struct Joint {
    reference_upper_centre_mm: Vector3<f64>,
    reference_lower_centre_mm: Vector3<f64>,
    upper_attachment_mm: Vector3<f64>,
    lower_attachment_mm: Vector3<f64>,
    reference_upper: Matrix3<f64>,
    reference_lower: Matrix3<f64>,
    stiffness: Stiffness,
}

/// Reference chain, top to bottom; joint i is under body i. The last
/// joint connects the lowest body to ground.
#[derive(Clone, Debug)]
pub struct Chain {
    pub reference: Vec<VertebraPose>,
    pub ground: Pose,
    joints: Vec<Joint>,
}

/// Joint energy and holding wrenches. Summing negatives gives the joint
/// loads on each body. The ground wrench is the applied support reaction.
#[derive(Clone, Debug)]
pub struct ChainEvaluation {
    pub energy_nmm: f64,
    pub gradient: Vec<Wrench>,
    /// Holding wrench on the ground at its top point. The equal opposite
    /// is the wrench the chain transmits to the sacrum.
    pub ground_reaction: Wrench,
    pub deformation: Vec<Wrench>,
}

/// Symmetric chain tangent, lower[i] couples body i+1 to body i.
#[derive(Clone, Debug)]
pub struct ChainLinearisation {
    pub diagonal: Vec<Matrix6<f64>>,
    pub lower: Vec<Matrix6<f64>>,
    pub evaluation: ChainEvaluation,
}

/// A joint's lower frame and moving upper attachment, in patient axes.
/// Rotation increments at this joint turn every body above about that point.
#[derive(Clone, Debug)]
pub struct JointMotionFrame {
    pub lower_rotation: Matrix3<f64>,
    pub upper_point_mm: Vector3<f64>,
}

impl Chain {
    pub fn new(
        reference: &[VertebraPose],
        stiffness: &[Stiffness],
        ground: Ground,
    ) -> Result<Self, ChainError> {
        if reference.is_empty() || reference.len() != stiffness.len() {
            return Err(ChainError::Dimensions);
        }
        let ground = Pose {
            centre_mm: ground.top_mm,
            rotation: ground.rotation,
        };
        if !ground.usable() {
            return Err(ChainError::InvalidPose);
        }
        let mut joints = Vec::with_capacity(reference.len());
        for (i, pose) in reference.iter().enumerate() {
            let params = VertebraParams::from(pose);
            let numbers = params.to_numbers();
            if numbers.iter().any(|x| !x.is_finite())
                || params.height <= 0.0
                || [
                    params.top_half_width,
                    params.top_half_depth,
                    params.bottom_half_width,
                    params.bottom_half_depth,
                ]
                .iter()
                .any(|x| *x <= 0.0)
                || params.wedge_front.abs() >= core::f64::consts::PI
                || params.wedge_side.abs() >= core::f64::consts::PI
            {
                return Err(ChainError::InvalidPose);
            }
            if stiffness[i].0.iter().any(|x| !x.is_finite() || *x <= 0.0) {
                return Err(ChainError::InvalidStiffness);
            }
            let upper = Pose::from_vertebra(pose);
            let lower = reference
                .get(i + 1)
                .map(Pose::from_vertebra)
                .unwrap_or_else(|| ground.clone());
            let lower_top = reference
                .get(i + 1)
                .map(|p| Endplates::of(&VertebraParams::from(p)).top.centre)
                .unwrap_or(ground.centre_mm);
            let point = (Endplates::of(&params).bottom.centre + lower_top) * 0.5;
            joints.push(Joint {
                reference_upper_centre_mm: upper.centre_mm,
                reference_lower_centre_mm: lower.centre_mm,
                upper_attachment_mm: upper.rotation.transpose() * (point - upper.centre_mm),
                lower_attachment_mm: lower.rotation.transpose() * (point - lower.centre_mm),
                reference_upper: upper.rotation,
                reference_lower: lower.rotation,
                stiffness: stiffness[i],
            });
        }
        Ok(Self {
            reference: reference.to_vec(),
            ground,
            joints,
        })
    }

    /// A positive tissue scale, carried analytically by the correction solve.
    pub fn stiffness_scale(&self) -> f64 {
        self.joints[0].stiffness.0[0]
    }

    /// Multiply all six stiffnesses of every joint, preserving rest geometry.
    pub fn scaled(&self, factor: f64) -> Result<Self, ChainError> {
        if !factor.is_finite() || factor <= 0.0 {
            return Err(ChainError::InvalidStiffness);
        }
        let mut result = self.clone();
        for joint in &mut result.joints {
            joint.stiffness.0 *= factor;
        }
        Ok(result)
    }

    /// Solver-only elimination of energy terms constant under a rigid group
    /// motion. Physical wrenches are always evaluated on the original chain.
    pub(crate) fn for_group_motion(&self, groups: &[usize]) -> Self {
        let mut result = self.clone();
        for i in 0..groups.len().saturating_sub(1) {
            if groups[i] == groups[i + 1] {
                result.joints[i].stiffness.0.fill(0.0);
            }
        }
        result
    }

    pub fn standing(&self) -> Vec<Pose> {
        self.reference.iter().map(Pose::from_vertebra).collect()
    }

    /// Copy the plain six-axis joint stiffnesses, in disc order.
    pub fn joint_stiffnesses(&self) -> Vec<Stiffness> {
        self.joints.iter().map(|j| j.stiffness).collect()
    }

    /// Exact forward kinematics from joint translations (mm, lower axes)
    /// and principal relative rotation logs (rad, lower axes). The fixed
    /// sacrum and reference attachments are retained.
    pub fn poses_from_deformations(&self, deformation: &[Wrench]) -> Result<Vec<Pose>, ChainError> {
        if deformation.len() != self.joints.len() {
            return Err(ChainError::Dimensions);
        }
        if deformation.iter().any(|q| {
            q.iter().any(|v| !v.is_finite()) || q.fixed_rows::<3>(3).norm() >= core::f64::consts::PI
        }) {
            return Err(ChainError::InvalidPose);
        }
        if deformation.iter().all(|q| q.iter().all(|v| *v == 0.0)) {
            return Ok(self.standing());
        }
        let mut poses = vec![self.ground.clone(); self.joints.len()];
        for i in (0..self.joints.len()).rev() {
            let lower = poses.get(i + 1).unwrap_or(&self.ground);
            let joint = &self.joints[i];
            let relative = joint.reference_lower.transpose() * joint.reference_upper;
            let rotation = lower.rotation
                * matrix_from_rotation_vector(&deformation[i].fixed_rows::<3>(3).into_owned())
                * relative;
            let centre_mm = lower.centre_mm
                + lower.rotation * (joint.lower_attachment_mm + deformation[i].fixed_rows::<3>(0))
                - rotation * joint.upper_attachment_mm;
            poses[i] = Pose {
                centre_mm,
                rotation,
            };
        }
        Ok(poses)
    }

    /// Frames for a tree Jacobian of head positions with respect to exact
    /// joint coordinates. Body i is affected by joints j >= i.
    pub fn joint_motion_frames(&self, poses: &[Pose]) -> Result<Vec<JointMotionFrame>, ChainError> {
        if poses.len() != self.joints.len() {
            return Err(ChainError::Dimensions);
        }
        if poses.iter().any(|p| !p.usable()) {
            return Err(ChainError::InvalidPose);
        }
        Ok(self
            .joints
            .iter()
            .enumerate()
            .map(|(i, joint)| JointMotionFrame {
                lower_rotation: poses.get(i + 1).unwrap_or(&self.ground).rotation,
                upper_point_mm: poses[i].centre_mm + poses[i].rotation * joint.upper_attachment_mm,
            })
            .collect())
    }

    pub fn evaluate(&self, poses: &[Pose]) -> Result<ChainEvaluation, ChainError> {
        if poses.len() != self.joints.len() {
            return Err(ChainError::Dimensions);
        }
        if poses.iter().any(|p| !p.usable()) {
            return Err(ChainError::InvalidPose);
        }
        let mut evaluation = ChainEvaluation {
            energy_nmm: 0.0,
            gradient: vec![Wrench::zeros(); poses.len()],
            ground_reaction: Wrench::zeros(),
            deformation: Vec::with_capacity(poses.len()),
        };
        for (i, joint) in self.joints.iter().enumerate() {
            if joint.stiffness.0.iter().all(|v| *v == 0.0) {
                evaluation.deformation.push(Wrench::zeros());
                continue;
            }
            let lower = poses.get(i + 1).unwrap_or(&self.ground);
            let (energy, deformation, upper_wrench, lower_wrench) =
                joint.evaluate(&poses[i], lower);
            evaluation.energy_nmm += energy;
            evaluation.deformation.push(deformation);
            evaluation.gradient[i] += upper_wrench;
            if i + 1 < poses.len() {
                evaluation.gradient[i + 1] += lower_wrench;
            } else {
                evaluation.ground_reaction = lower_wrench;
            }
        }
        Ok(evaluation)
    }

    /// Apply the exact chart tangent through relative joint motions. This
    /// avoids subtracting large body-centre matrix products for a nearly
    /// rigid column. It is the same Hessian assembled by `linearise`.
    pub(crate) fn tangent_load(
        &self,
        poses: &[Pose],
        motion: &[Wrench],
    ) -> Result<Vec<Wrench>, ChainError> {
        self.tangent_load_with_material_scale(poses, motion, 1.0)
    }

    /// Apply a tangent whose material stiffness changes without changing the
    /// seated forces used by its geometric terms.
    pub(crate) fn tangent_load_with_material_scale(
        &self,
        poses: &[Pose],
        motion: &[Wrench],
        material_scale: f64,
    ) -> Result<Vec<Wrench>, ChainError> {
        if !material_scale.is_finite() || material_scale <= 0.0 {
            return Err(ChainError::InvalidStiffness);
        }
        let evaluation = self.evaluate(poses)?;
        if motion.len() != poses.len()
            || motion.iter().flat_map(|u| u.iter()).any(|v| !v.is_finite())
        {
            return Err(ChainError::Dimensions);
        }
        let zero = Wrench::zeros();
        let mut result = vec![Wrench::zeros(); poses.len()];
        for (i, joint) in self.joints.iter().enumerate() {
            let upper = &poses[i];
            let lower = poses.get(i + 1).unwrap_or(&self.ground);
            let up = &motion[i];
            let low = motion.get(i + 1).unwrap_or(&zero);
            let wu = up.fixed_rows::<3>(3).into_owned();
            let wl = low.fixed_rows::<3>(3).into_owned();
            let arm = upper.rotation * joint.upper_attachment_mm;
            let point_arm = upper.centre_mm + arm - lower.centre_mm;
            let relative = up.fixed_rows::<3>(0) - low.fixed_rows::<3>(0) + wu.cross(&arm)
                - wl.cross(&point_arm);
            let q = &evaluation.deformation[i];
            let force = lower.rotation
                * joint
                    .stiffness
                    .0
                    .fixed_rows::<3>(0)
                    .component_mul(&q.fixed_rows::<3>(0));
            let mut material_change = lower.rotation
                * joint
                    .stiffness
                    .0
                    .fixed_rows::<3>(0)
                    .component_mul(&(lower.rotation.transpose() * relative));
            if material_scale != 1.0 {
                material_change *= material_scale;
            }
            let force_change = wl.cross(&force) + material_change;
            let (local_couple, tangent) = rotation_response_tangent_with_material_scale(
                &q.fixed_rows::<3>(3).into_owned(),
                &joint.stiffness,
                material_scale,
            );
            let couple = lower.rotation * local_couple;
            let couple_change = wl.cross(&couple)
                + lower.rotation
                    * (tangent - local_couple.cross_matrix() * 0.5)
                    * (lower.rotation.transpose() * (wu - wl));
            let mut upper_change = Wrench::zeros();
            upper_change.fixed_rows_mut::<3>(0).copy_from(&force_change);
            upper_change.fixed_rows_mut::<3>(3).copy_from(
                &(wu.cross(&arm).cross(&force) + arm.cross(&force_change) + couple_change),
            );
            let mut lower_change = Wrench::zeros();
            lower_change
                .fixed_rows_mut::<3>(0)
                .copy_from(&(-force_change));
            lower_change.fixed_rows_mut::<3>(3).copy_from(
                &(-(up.fixed_rows::<3>(0) - low.fixed_rows::<3>(0) + wu.cross(&arm)).cross(&force)
                    - point_arm.cross(&force_change)
                    - couple_change),
            );
            result[i] += upper_change;
            if i + 1 < result.len() {
                result[i + 1] += lower_change;
            }
        }
        for (i, load) in result.iter_mut().enumerate() {
            let correction = evaluation.gradient[i]
                .fixed_rows::<3>(3)
                .cross(&motion[i].fixed_rows::<3>(3))
                * 0.5;
            for axis in 0..3 {
                load[axis + 3] += correction[axis];
            }
        }
        Ok(result)
    }

    /// Exact local-retraction energy Hessian, assembled joint by joint.
    /// Translation increments are mm at each body centre; rotation increments
    /// are patient-axis radians composed on the left.
    pub fn linearise(&self, poses: &[Pose]) -> Result<ChainLinearisation, ChainError> {
        self.linearise_with_material_scale(poses, 1.0)
    }

    /// Material stiffness about the seated strain may be multiplied, while
    /// the seated force and couple still determine all geometric stiffness.
    pub(crate) fn linearise_with_material_scale(
        &self,
        poses: &[Pose],
        material_scale: f64,
    ) -> Result<ChainLinearisation, ChainError> {
        if !material_scale.is_finite() || material_scale <= 0.0 {
            return Err(ChainError::InvalidStiffness);
        }
        let evaluation = self.evaluate(poses)?;
        let n = poses.len();
        let mut diagonal = vec![Matrix6::zeros(); n];
        let mut lower_blocks = vec![Matrix6::zeros(); n - 1];
        for (i, joint) in self.joints.iter().enumerate() {
            if joint.stiffness.0.iter().all(|v| *v == 0.0) {
                continue;
            }
            let lower = poses.get(i + 1).unwrap_or(&self.ground);
            let (upper_hessian, lower_hessian, coupling) =
                joint.tangent(&poses[i], lower, &evaluation.deformation[i], material_scale);
            diagonal[i] += upper_hessian;
            if i + 1 < n {
                diagonal[i + 1] += lower_hessian;
                lower_blocks[i] += coupling;
            }
        }
        Ok(ChainLinearisation {
            diagonal,
            lower: lower_blocks,
            evaluation,
        })
    }
}

impl Joint {
    /// Exact local-retraction energy Hessian, upper/lower diagonal and
    /// lower-by-upper coupling. The force terms are geometric stiffness;
    /// prestress can make them negative and they must not be clipped.
    fn tangent(
        &self,
        upper: &Pose,
        lower: &Pose,
        deformation: &Wrench,
        material_scale: f64,
    ) -> (Matrix6<f64>, Matrix6<f64>, Matrix6<f64>) {
        let r = upper.rotation * self.upper_attachment_mm;
        let q = upper.centre_mm - lower.centre_mm + r;
        let r_cross = r.cross_matrix();
        let q_cross = q.cross_matrix();
        let translation = deformation.fixed_rows::<3>(0).into_owned();
        let rotation = deformation.fixed_rows::<3>(3).into_owned();
        let translation_weights =
            Matrix3::from_diagonal(&self.stiffness.0.fixed_rows::<3>(0).into_owned());
        let a = lower.rotation * translation_weights * lower.rotation.transpose();
        let mut a = (a + a.transpose()) * 0.5;
        if material_scale != 1.0 {
            a *= material_scale;
        }
        let force = lower.rotation * (translation_weights * translation);
        let f_cross = force.cross_matrix();
        let (local_couple, local_tangent) = rotation_response_tangent_with_material_scale(
            &rotation,
            &self.stiffness,
            material_scale,
        );
        let couple = lower.rotation * local_couple;
        let s = lower.rotation * local_tangent * lower.rotation.transpose();
        let ar = a * r_cross;
        let aq = a * q_cross;
        let fr = f_cross * r_cross;
        let qf = q_cross * f_cross;
        let upper_rotation = -r_cross * ar + (fr + fr.transpose()) * 0.5 + s;
        let lower_rotation = -q_cross * aq + (qf + qf.transpose()) * 0.5 + s;
        let mut upper_hessian = Matrix6::zeros();
        let mut lower_hessian = Matrix6::zeros();
        let mut coupling = Matrix6::zeros();
        upper_hessian.fixed_view_mut::<3, 3>(0, 0).copy_from(&a);
        upper_hessian.fixed_view_mut::<3, 3>(0, 3).copy_from(&(-ar));
        upper_hessian
            .fixed_view_mut::<3, 3>(3, 0)
            .copy_from(&(-ar).transpose());
        upper_hessian
            .fixed_view_mut::<3, 3>(3, 3)
            .copy_from(&((upper_rotation + upper_rotation.transpose()) * 0.5));
        lower_hessian.fixed_view_mut::<3, 3>(0, 0).copy_from(&a);
        let lower_cross = -aq + f_cross;
        lower_hessian
            .fixed_view_mut::<3, 3>(0, 3)
            .copy_from(&lower_cross);
        lower_hessian
            .fixed_view_mut::<3, 3>(3, 0)
            .copy_from(&lower_cross.transpose());
        lower_hessian
            .fixed_view_mut::<3, 3>(3, 3)
            .copy_from(&((lower_rotation + lower_rotation.transpose()) * 0.5));
        coupling.fixed_view_mut::<3, 3>(0, 0).copy_from(&(-a));
        coupling.fixed_view_mut::<3, 3>(0, 3).copy_from(&ar);
        coupling
            .fixed_view_mut::<3, 3>(3, 0)
            .copy_from(&(f_cross - q_cross * a));
        coupling
            .fixed_view_mut::<3, 3>(3, 3)
            .copy_from(&(q_cross * ar - fr - s + couple.cross_matrix() * 0.5));
        (upper_hessian, lower_hessian, coupling)
    }

    fn evaluate(&self, upper: &Pose, lower: &Pose) -> (f64, Wrench, Wrench, Wrench) {
        // Exactly reused reference poses are the mathematical zero of this
        // spring, without round-off from multiplying RᵀR or its attachments.
        if upper.centre_mm == self.reference_upper_centre_mm
            && lower.centre_mm == self.reference_lower_centre_mm
            && upper.rotation == self.reference_upper
            && lower.rotation == self.reference_lower
        {
            return (0.0, Wrench::zeros(), Wrench::zeros(), Wrench::zeros());
        }
        let arm = upper.rotation * self.upper_attachment_mm;
        let upper_point = upper.centre_mm + arm;
        let lower_point = lower.centre_mm + lower.rotation * self.lower_attachment_mm;
        let translation = lower.rotation.transpose() * (upper_point - lower_point);
        let rotation = rotation_deformation(
            &self.reference_upper,
            &self.reference_lower,
            &upper.rotation,
            &lower.rotation,
        );
        let deformation = Wrench::new(
            translation.x,
            translation.y,
            translation.z,
            rotation.x,
            rotation.y,
            rotation.z,
        );
        let response = self.stiffness.response(&deformation);
        let force = lower.rotation * response.fixed_rows::<3>(0);
        let couple = lower.rotation
            * log_left_derivative(&rotation).transpose()
            * response.fixed_rows::<3>(3);
        let upper_moment = arm.cross(&force) + couple;
        let lower_moment = -(upper_point - lower.centre_mm).cross(&force) - couple;
        let mut up = Wrench::zeros();
        up.fixed_rows_mut::<3>(0).copy_from(&force);
        up.fixed_rows_mut::<3>(3).copy_from(&upper_moment);
        let mut low = Wrench::zeros();
        low.fixed_rows_mut::<3>(0).copy_from(&(-force));
        low.fixed_rows_mut::<3>(3).copy_from(&lower_moment);
        (self.stiffness.energy(&deformation), deformation, up, low)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forward_model::rotation::{rot_x, rot_y, rot_z};
    use shapes::levels::Level;

    fn reference() -> Vec<VertebraPose> {
        [Level::L3, Level::L4, Level::L5]
            .iter()
            .enumerate()
            .map(|(i, &level)| VertebraPose {
                level,
                centre: [0.0, 0.0, 70.0 - 30.0 * i as f64],
                rotation: [0.0; 3],
                height: 20.0,
                wedge_front: 0.0,
                wedge_side: 0.0,
                top_half_width: 15.0,
                top_half_depth: 10.0,
                bottom_half_width: 15.0,
                bottom_half_depth: 10.0,
            })
            .collect()
    }

    fn chain() -> Chain {
        let k = Stiffness::new([100.0, 200.0, 300.0, 4000.0, 5000.0, 6000.0])
            .expect("made-up stiffness");
        Chain::new(
            &reference(),
            &[k; 3],
            Ground {
                top_mm: Vector3::new(0.0, 0.0, -10.0),
                rotation: Matrix3::identity(),
            },
        )
        .expect("chain")
    }

    #[test]
    fn analytic_holding_wrenches_are_the_energy_derivative() {
        let chain = chain();
        let mut poses = chain.standing();
        poses[0] = poses[0].incremented(&Wrench::new(1.0, 2.0, -1.0, 0.6, -0.2, 0.3));
        poses[1] = poses[1].incremented(&Wrench::new(-1.0, 1.0, 2.0, -0.2, 0.1, 0.2));
        let value = chain.evaluate(&poses).expect("energy");
        for i in 0..poses.len() {
            for axis in 0..6 {
                let h = if axis < 3 { 1e-4 } else { 1e-6 };
                let mut step = Wrench::zeros();
                step[axis] = h;
                let mut plus = poses.clone();
                plus[i] = poses[i].incremented(&step);
                let mut minus = poses.clone();
                minus[i] = poses[i].incremented(&(-step));
                let difference = (chain.evaluate(&plus).expect("plus").energy_nmm
                    - chain.evaluate(&minus).expect("minus").energy_nmm)
                    / (2.0 * h);
                let exact = value.gradient[i][axis];
                assert!(
                    (difference - exact).abs() < 2e-5 + exact.abs() * 2e-8,
                    "{i}/{axis}: {difference} {exact}"
                );
            }
        }
    }

    #[test]
    fn joint_coordinates_have_the_closed_form_large_angle_geometry_and_energy() {
        let chain = chain();
        let mut deformation = vec![Wrench::zeros(); 3];
        deformation[2][2] = 2.0;
        deformation[2][4] = 0.8;
        let poses = chain.poses_from_deformations(&deformation).unwrap();
        // Reference disc point is z=-5, lower attachment z=+5 from ground
        // at -10. A 2 mm joint translation puts the rotation pivot at -3.
        for (i, pose) in poses.iter().enumerate() {
            let arm = chain.reference[i].centre[2] + 5.0;
            let expected = Vector3::new(arm * libm::sin(0.8), 0.0, -3.0 + arm * libm::cos(0.8));
            assert!((pose.centre_mm - expected).norm() < 1e-12);
            assert!((pose.rotation - rot_y(0.8)).norm() < 1e-14);
        }
        let evaluated = chain.evaluate(&poses).unwrap();
        // Two exact scalar spring energies: 1/2*300*2²+1/2*5000*.8².
        assert!((evaluated.energy_nmm - 2200.0).abs() < 1e-9);
        for (a, b) in evaluated.deformation.iter().zip(&deformation) {
            assert!((a - b).norm() < 1e-12);
        }
    }

    #[test]
    fn a_chain_and_its_ground_balance_at_finite_rotations() {
        let chain = chain();
        let mut poses = chain.standing();
        poses[0] = poses[0].incremented(&Wrench::new(1.0, 2.0, -1.0, 0.6, -0.2, 0.3));
        poses[1] = poses[1].incremented(&Wrench::new(-1.0, 1.0, 2.0, -0.2, 0.1, 0.2));
        let value = chain.evaluate(&poses).expect("energy");
        let mut force = value.ground_reaction.fixed_rows::<3>(0).into_owned();
        let mut moment = value.ground_reaction.fixed_rows::<3>(3).into_owned()
            + chain.ground.centre_mm.cross(&force);
        for (pose, wrench) in poses.iter().zip(&value.gradient) {
            force += wrench.fixed_rows::<3>(0);
            moment += wrench.fixed_rows::<3>(3)
                + pose
                    .centre_mm
                    .cross(&wrench.fixed_rows::<3>(0).into_owned());
        }
        assert!(force.norm() < 1e-10);
        assert!(moment.norm() < 1e-9, "{moment}");
    }

    #[test]
    fn rigidly_turning_the_reference_and_state_turns_the_wrenches() {
        let chain = chain();
        let mut poses = chain.standing();
        poses[0] = poses[0].incremented(&Wrench::new(1.0, -2.0, 0.5, 0.4, -0.3, 0.6));
        let before = chain.evaluate(&poses).expect("energy");
        let q = rot_x(0.5) * rot_y(-0.7) * rot_z(0.8);
        let t = Vector3::new(2.0, -4.0, 1.0);
        let reference: Vec<_> = chain
            .reference
            .iter()
            .map(|p| {
                let original = Pose::from_vertebra(p);
                Pose {
                    centre_mm: q * original.centre_mm + t,
                    rotation: q * original.rotation,
                }
                .to_vertebra(p)
            })
            .collect();
        let stiffness: Vec<_> = chain.joints.iter().map(|j| j.stiffness).collect();
        let turned = Chain::new(
            &reference,
            &stiffness,
            Ground {
                top_mm: q * chain.ground.centre_mm + t,
                rotation: q * chain.ground.rotation,
            },
        )
        .expect("turned chain");
        let poses: Vec<_> = poses
            .iter()
            .map(|p| Pose {
                centre_mm: q * p.centre_mm + t,
                rotation: q * p.rotation,
            })
            .collect();
        let after = turned.evaluate(&poses).expect("energy");
        assert!((before.energy_nmm - after.energy_nmm).abs() < 1e-8);
        for (a, b) in before.gradient.iter().zip(&after.gradient) {
            assert!((q * a.fixed_rows::<3>(0) - b.fixed_rows::<3>(0)).norm() < 1e-9);
            assert!((q * a.fixed_rows::<3>(3) - b.fixed_rows::<3>(3)).norm() < 1e-8);
        }
    }
    fn dense_tangent(value: &ChainLinearisation) -> nalgebra::DMatrix<f64> {
        let mut result =
            nalgebra::DMatrix::zeros(6 * value.diagonal.len(), 6 * value.diagonal.len());
        for (i, block) in value.diagonal.iter().enumerate() {
            result.view_mut((6 * i, 6 * i), (6, 6)).copy_from(block);
        }
        for (i, block) in value.lower.iter().enumerate() {
            result.view_mut((6 * i + 6, 6 * i), (6, 6)).copy_from(block);
            result
                .view_mut((6 * i, 6 * i + 6), (6, 6))
                .copy_from(&block.transpose());
        }
        result
    }
    fn chart_gradient(
        chain: &Chain,
        base: &[Pose],
        body: usize,
        axis: usize,
        h: f64,
    ) -> nalgebra::DVector<f64> {
        let mut step = Wrench::zeros();
        step[axis] = h;
        let mut poses = base.to_vec();
        poses[body] = poses[body].incremented(&step);
        let mut value = chain.evaluate(&poses).unwrap();
        if axis >= 3 {
            // Independent small-step exponential derivative, expressed in the
            // one fixed chart Exp(omega)*R_base rather than the moving chart.
            let omega = step.fixed_rows::<3>(3).into_owned();
            let cross = omega.cross_matrix();
            let left = Matrix3::identity()
                + cross * (0.5 - h * h / 24.0)
                + cross * cross * (1.0 / 6.0 - h * h / 120.0);
            let moment = left.transpose() * value.gradient[body].fixed_rows::<3>(3);
            value.gradient[body]
                .fixed_rows_mut::<3>(3)
                .copy_from(&moment);
        }
        nalgebra::DVector::from_iterator(
            6 * base.len(),
            value
                .gradient
                .into_iter()
                .flat_map(|g| g.as_slice().to_vec()),
        )
    }
    #[test]
    fn exact_tangent_matches_independent_fixed_chart_gradient_differences_at_large_deformations() {
        let mut reference = reference();
        for (i, p) in reference.iter_mut().enumerate() {
            p.rotation = [[0.31, -0.13, 0.21], [-0.2, 0.17, 0.05], [0.07, 0.14, -0.15]][i];
        }
        let stiffness = Stiffness::new([100.0, 200.0, 300.0, 4000.0, 5000.0, 6000.0]).unwrap();
        let chain = Chain::new(
            &reference,
            &[stiffness; 3],
            Ground {
                top_mm: Vector3::new(1.0, -2.0, -10.0),
                rotation: rot_x(0.31) * rot_y(-0.27),
            },
        )
        .unwrap();
        for rotations in [
            [[1.3, -0.6, 0.4], [-0.7, 0.9, -0.5], [0.5, -0.3, 0.8]],
            [[1e-8, -2e-8, 3e-8], [0.0, 0.0, 0.0], [0.12, -0.21, 0.17]],
            [[2.8, 0.0, 0.0], [-0.1, 0.3, -0.4], [0.07, -0.11, 0.08]],
        ] {
            let deformation: Vec<_> = rotations
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    Wrench::new(
                        [7.0, -8.0, -3.0][i],
                        [-11.0, 4.0, 2.0][i],
                        [15.0, -9.0, 5.0][i],
                        p[0],
                        p[1],
                        p[2],
                    )
                })
                .collect();
            let poses = chain.poses_from_deformations(&deformation).unwrap();
            let linear = chain.linearise(&poses).unwrap();
            let tangent = dense_tangent(&linear);
            assert_eq!((&tangent - tangent.transpose()).abs().max(), 0.0);
            for column in 0..6 * poses.len() {
                let body = column / 6;
                let axis = column % 6;
                let h = if axis < 3 { 1e-4 } else { 2e-6 };
                let mut motion = vec![Wrench::zeros(); poses.len()];
                motion[body][axis] = 1.0;
                let action = chain.tangent_load(&poses, &motion).unwrap();
                let difference = (chart_gradient(&chain, &poses, body, axis, h)
                    - chart_gradient(&chain, &poses, body, axis, -h))
                    / (2.0 * h);
                for row in 0..6 * poses.len() {
                    let exact = tangent[(row, column)];
                    let error = (difference[row] - exact).abs();
                    assert!(
                        (difference[row] - action[row / 6][row % 6]).abs()
                            < 2e-5 + 2e-8 * exact.abs()
                    );
                    assert!(
                        error < 2e-5 + 2e-8 * exact.abs(),
                        "({row},{column}) exact={exact} fd={} error={error}",
                        difference[row]
                    );
                }
            }
        }
    }
    #[test]
    fn compression_can_make_the_exact_geometric_tangent_negative() {
        let mut reference = reference()[2];
        reference.centre = [0.0, 0.0, 10.0];
        let stiffness = Stiffness::new([100.0, 100.0, 100.0, 4000.0, 5000.0, 6000.0]).unwrap();
        let chain = Chain::new(
            &[reference],
            &[stiffness],
            Ground {
                top_mm: Vector3::zeros(),
                rotation: Matrix3::identity(),
            },
        )
        .unwrap();
        let poses =
            vec![chain.standing()[0].incremented(&Wrench::new(0.0, 0.0, -30.0, 0.0, 0.0, 0.0))];
        let tangent = chain.linearise(&poses).unwrap().diagonal[0][(3, 3)];
        let expected = stiffness.0[3] + stiffness.0[0] * 100.0 - stiffness.0[2] * 30.0 * 10.0;
        assert_eq!(tangent, expected);
        assert!(tangent < 0.0);
        let h = 1e-4;
        let step = Wrench::new(0.0, 0.0, 0.0, h, 0.0, 0.0);
        let plus = chain
            .evaluate(&[poses[0].incremented(&step)])
            .unwrap()
            .energy_nmm;
        let minus = chain
            .evaluate(&[poses[0].incremented(&(-step))])
            .unwrap()
            .energy_nmm;
        let base = chain.evaluate(&poses).unwrap().energy_nmm;
        let scalar_curvature = (plus + minus - 2.0 * base) / (h * h);
        assert!(
            (scalar_curvature - tangent).abs() < 0.01,
            "scalar {scalar_curvature} exact {tangent}"
        );
    }

    #[test]
    fn material_stiffening_keeps_the_seated_compression_and_its_geometric_term() {
        let mut reference = reference()[2];
        reference.centre = [0.0, 0.0, 10.0];
        let stiffness = Stiffness::new([100.0, 100.0, 100.0, 4000.0, 5000.0, 6000.0]).unwrap();
        let chain = Chain::new(
            &[reference],
            &[stiffness],
            Ground {
                top_mm: Vector3::zeros(),
                rotation: Matrix3::identity(),
            },
        )
        .unwrap();
        let poses =
            vec![chain.standing()[0].incremented(&Wrench::new(0.0, 0.0, -3.0, 0.0, 0.0, 0.0))];
        let ordinary = chain.linearise(&poses).unwrap();
        let stiffened = chain.linearise_with_material_scale(&poses, 2.0).unwrap();
        assert_eq!(stiffened.evaluation.gradient, ordinary.evaluation.gradient);
        assert_eq!(stiffened.evaluation.gradient[0][2], -300.0);
        // The head is 10 mm from the disc: k_x*10²+k_rx is material;
        // the existing 300 N compression contributes -300*10 at either scale.
        assert_eq!(
            ordinary.diagonal[0][(3, 3)],
            100.0 * 100.0 + 4000.0 - 3000.0
        );
        assert_eq!(
            stiffened.diagonal[0][(3, 3)],
            2.0 * (100.0 * 100.0 + 4000.0) - 3000.0
        );
        assert_eq!(stiffened.diagonal[0][(2, 2)], 200.0);
    }

    #[test]
    fn the_stiffened_tangent_is_the_energy_curvature_at_unchanged_seated_stress() {
        let chain = chain();
        let deformation = [
            Wrench::new(0.6, -0.3, -0.8, 0.4, -0.2, 0.3),
            Wrench::new(-0.2, 0.5, -0.4, -0.1, 0.3, 0.2),
            Wrench::new(0.1, -0.2, -0.6, 0.2, 0.1, -0.2),
        ];
        let poses = chain.poses_from_deformations(&deformation).unwrap();
        let seated = chain.evaluate(&poses).unwrap();
        let factor = 1.9;
        let linear = chain.linearise_with_material_scale(&poses, factor).unwrap();
        let tangent = dense_tangent(&linear);
        assert_eq!(linear.evaluation.gradient, seated.gradient);
        let motion = [
            Wrench::new(0.7, -0.4, 0.3, 0.1, -0.2, 0.05),
            Wrench::new(-0.2, 0.3, -0.1, -0.04, 0.1, -0.08),
            Wrench::new(0.1, -0.2, 0.2, 0.03, -0.02, 0.07),
        ];
        let vector =
            nalgebra::DVector::from_iterator(18, motion.iter().flat_map(|u| u.iter().copied()));
        let action = chain
            .tangent_load_with_material_scale(&poses, &motion, factor)
            .unwrap();
        let applied =
            nalgebra::DVector::from_iterator(18, action.iter().flat_map(|w| w.iter().copied()));
        assert!((&tangent * &vector - applied).norm() < 1e-8);
        // Independent potential: the original energy plus extra quadratic
        // strain energy centred on the seated strain. Its gradient there is
        // unchanged, while its constitutive tangent is multiplied by factor.
        let energy = |step: f64| {
            let moved: Vec<_> = poses
                .iter()
                .zip(&motion)
                .map(|(pose, u)| pose.incremented(&(u * step)))
                .collect();
            let evaluated = chain.evaluate(&moved).unwrap();
            evaluated.energy_nmm
                + evaluated
                    .deformation
                    .iter()
                    .zip(&seated.deformation)
                    .zip(chain.joint_stiffnesses())
                    .map(|((now, start), k)| (factor - 1.0) * k.energy(&(now - start)))
                    .sum::<f64>()
        };
        let h = 1e-4;
        let difference = (energy(h) + energy(-h) - 2.0 * energy(0.0)) / (h * h);
        let exact = vector.dot(&(&tangent * &vector));
        assert!((difference - exact).abs() < 0.01 + exact.abs() * 1e-6);
    }
}
