//! Daily disc-couple increments beside the construct, against the same
//! load without it. Locked-in correction couples are excluded: these are
//! ratios of a single linear case and are independent of its size.
//!
//! Patient x anterior, y left, z up; moments N·mm. Each vector is the
//! holding couple on the upper side, at its moving material disc point.
//! Comparing vector norms removes the arbitrary choice of action side.

use crate::chain::{Chain, ChainError, Pose};
use crate::daily_loads::DailyResponse;
use crate::joint_springs::rotation_response_tangent_with_material_scale;
use nalgebra::{DVector, Vector3};
use shapes::levels::Level;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JunctionRatios {
    pub above: Option<f64>,
    pub below: Option<f64>,
}

/// Joint i is under body i. No body above the upper end or exactly zero
/// reference moment gives an absent ratio. The final joint is L5/sacrum,
/// so its ratio remains available for a construct ending at L5.
pub fn ratios(
    levels: &[Level],
    upper: Level,
    lower: Level,
    with_construct: &DailyResponse,
    without_construct: &DailyResponse,
) -> Result<JunctionRatios, ChainError> {
    let n = levels.len();
    if n == 0
        || levels.windows(2).any(|p| p[0] >= p[1])
        || with_construct.joint_moments_nmm.len() != n
        || without_construct.joint_moments_nmm.len() != n
    {
        return Err(ChainError::Dimensions);
    }
    let top = levels
        .iter()
        .position(|l| *l == upper)
        .ok_or(ChainError::Dimensions)?;
    let bottom = levels
        .iter()
        .position(|l| *l == lower)
        .ok_or(ChainError::Dimensions)?;
    if top > bottom {
        return Err(ChainError::Dimensions);
    }
    let ratio = |i: usize| {
        let before = without_construct.joint_moments_nmm[i].norm();
        let after = with_construct.joint_moments_nmm[i].norm();
        if before > 0.0 && before.is_finite() && after.is_finite() {
            let value = after / before;
            value.is_finite().then_some(value)
        } else {
            None
        }
    };
    Ok(JunctionRatios {
        above: top.checked_sub(1).and_then(ratio),
        below: ratio(bottom),
    })
}

/// Differentiate each disc's intrinsic log-spring couple directly.
/// This is the same spatial holding-couple derivative as a chain-prefix
/// balance, without cancellation between large force-arm and body-chart
/// terms. Translation spring forces act through the material joint point
/// and therefore add no intrinsic couple there.
pub fn moment_increments(
    chain: &Chain,
    poses: &[Pose],
    displacement: &DVector<f64>,
) -> Result<Vec<Vector3<f64>>, ChainError> {
    moment_increments_with_material_scale(chain, poses, displacement, 1.0)
}

/// Use the same material increment and unchanged seated couple as the daily
/// chain tangent, including the log's force-dependent curvature.
pub(crate) fn moment_increments_with_material_scale(
    chain: &Chain,
    poses: &[Pose],
    displacement: &DVector<f64>,
    material_scale: f64,
) -> Result<Vec<Vector3<f64>>, ChainError> {
    if !material_scale.is_finite() || material_scale <= 0.0 {
        return Err(ChainError::InvalidStiffness);
    }
    if displacement.len() != 6 * poses.len() || displacement.iter().any(|v| !v.is_finite()) {
        return Err(ChainError::Dimensions);
    }
    let evaluation = chain.evaluate(poses)?;
    let stiffness = chain.joint_stiffnesses();
    let frames = chain.joint_motion_frames(poses)?;
    Ok(frames
        .iter()
        .enumerate()
        .map(|(i, frame)| {
            let upper_turn = Vector3::from_column_slice(displacement.rows(6 * i + 3, 3).as_slice());
            let lower_turn = if i + 1 < poses.len() {
                Vector3::from_column_slice(displacement.rows(6 * (i + 1) + 3, 3).as_slice())
            } else {
                Vector3::zeros()
            };
            let rotation = evaluation.deformation[i].fixed_rows::<3>(3).into_owned();
            let (couple, tangent) = rotation_response_tangent_with_material_scale(
                &rotation,
                &stiffness[i],
                material_scale,
            );
            let relative_turn = frame.lower_rotation.transpose() * (upper_turn - lower_turn);
            // Remove the exponential chart's +1/2[couple]x to get the
            // spatial derivative, then transport the lower frame once.
            let local = (tangent - couple.cross_matrix() * 0.5) * relative_turn;
            lower_turn.cross(&(frame.lower_rotation * couple)) + frame.lower_rotation * local
        })
        .collect())
}
