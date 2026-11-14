//! Daily disc-couple increments beside the construct, against the same
//! load without it. Locked-in correction couples are excluded: these are
//! ratios of a single linear case and are independent of its size.
//!
//! Patient x anterior, y left, z up; moments N·mm. Each vector is the
//! holding couple on the upper side, at its moving material disc point.
//! Comparing vector norms removes the arbitrary choice of action side.

use crate::chain::{Chain, ChainError, Pose, Wrench};
use crate::daily_loads::{DailyResponse, tangent_matrix};
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

/// Differentiate the spatial moment balance of the chain prefix, retaining
/// moving body centres and the moving disc point. The energy Hessian's
/// rotational chart term is removed to recover spatial holding-wrench
/// derivatives. Thus a translational spring lever arm is never counted as
/// an intrinsic disc couple, even at a prestressed seated pose.
pub fn moment_increments(
    chain: &Chain,
    poses: &[Pose],
    displacement: &DVector<f64>,
) -> Result<Vec<Vector3<f64>>, ChainError> {
    if displacement.len() != 6 * poses.len() || displacement.iter().any(|v| !v.is_finite()) {
        return Err(ChainError::Dimensions);
    }
    let linear = chain.linearise(poses)?;
    let derivative = tangent_matrix(&linear) * displacement;
    let frames = chain.joint_motion_frames(poses)?;
    let motions: Vec<_> = displacement
        .as_slice()
        .as_chunks::<6>()
        .0
        .iter()
        .map(|u| Wrench::from_column_slice(u))
        .collect();
    let spatial: Vec<_> = linear
        .evaluation
        .gradient
        .iter()
        .enumerate()
        .map(|(i, g)| {
            let mut dg = Wrench::from_column_slice(derivative.rows(6 * i, 6).as_slice());
            let correction = g.fixed_rows::<3>(3).cross(&motions[i].fixed_rows::<3>(3)) * 0.5;
            dg.fixed_rows_mut::<3>(3).add_assign(-correction);
            dg
        })
        .collect();
    let mut moments = Vec::with_capacity(poses.len());
    for (joint, frame) in frames.iter().enumerate() {
        let point = frame.upper_point_mm;
        let point_motion = motions[joint].fixed_rows::<3>(0).into_owned()
            + motions[joint]
                .fixed_rows::<3>(3)
                .cross(&(point - poses[joint].centre_mm));
        let mut moment = Vector3::zeros();
        for i in 0..=joint {
            let gradient = &linear.evaluation.gradient[i];
            moment += (motions[i].fixed_rows::<3>(0) - point_motion)
                .cross(&gradient.fixed_rows::<3>(0))
                + (poses[i].centre_mm - point).cross(&spatial[i].fixed_rows::<3>(0))
                + spatial[i].fixed_rows::<3>(3);
        }
        moments.push(moment);
    }
    Ok(moments)
}

use core::ops::AddAssign;
