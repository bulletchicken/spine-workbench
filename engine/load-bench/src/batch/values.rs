//! Read fixed endpoints and join head loads to the outward stop values.

#[cfg(test)]
mod tests;

mod stresses;

use super::assembly::Placement;
use crate::chain::{Chain, Pose};
use crate::daily_loads::{DailyCases, DailyResponse};
use crate::evidence::Evidence;
use crate::rigid_rod::RigidState;
use crate::springback::SeatedState;
use crate::versions::{Draw, refusal};
use crate::{BenchError, anchor_demand};
use forward_model::{endplates::Endplates, vertebra_params::VertebraParams};
use nalgebra::Vector3;
use shapes::implant_layout::ImplantLayout;
use shapes::levels::Level;
use shapes::load_bench::{BenchStop, MainCurve, StopValues};
use shapes::made_up::VertebraPose;
use shapes::spine_fit::SpineFit;

pub(super) struct Readings {
    pub(super) main_curve: Option<MainCurve>,
    upper: usize,
    lower: usize,
    apex: usize,
}
impl Readings {
    pub(super) fn new(fit: &SpineFit, _layout: &ImplantLayout) -> Result<Self, BenchError> {
        let mut nominal = fit.clone();
        nominal
            .vertebrae
            .retain(|v| crate::daily_loads::LOAD_LEVELS.contains(&v.level));
        let input = shapes::implant_layout::LayoutInput {
            fit: nominal.clone(),
            front: None,
            bend_left: None,
            bend_right: None,
            child: shapes::implant_layout::ChildFacts {
                age_years: None,
                weight_kg: None,
                activity: None,
            },
            naming: shapes::implant_layout::NamingDoubt::AsRead,
            versions: 1,
            seed: 0,
            pedicle_child_share: None,
        };
        let reading = implant_layout::curves::measure(&input, &nominal)
            .map_err(|e| refusal(&e.to_string()))?;
        // All placements keep the same nominal main endpoints and apex.
        // For an exactly straight spine there is no curve to name.
        Ok(match reading.major.and_then(|i| reading.curves.get(i)) {
            Some(curve) => Self {
                main_curve: Some(MainCurve {
                    upper: nominal.vertebrae[curve.upper_index].level,
                    apex: nominal.vertebrae[curve.apex_index].level,
                    lower: nominal.vertebrae[curve.lower_index].level,
                }),
                upper: curve.upper_index,
                lower: curve.lower_index,
                apex: curve.apex_index,
            },
            None => Self {
                main_curve: None,
                upper: 0,
                lower: nominal.vertebrae.len() - 1,
                apex: 0,
            },
        })
    }
    fn angles(&self, fit: &SpineFit, poses: &[VertebraPose]) -> [f64; 3] {
        let cobb =
            (front(fit, &poses[self.lower], false) - front(fit, &poses[self.upper], true)).abs();
        // These are model-scenario angles; a front-only sagittal/twist
        // scenario retains typical geometry and is never a film reading.
        let t5 = poses.iter().find(|p| p.level == Level::T5).unwrap();
        let t12 = poses.iter().find(|p| p.level == Level::T12).unwrap();
        let kyphosis = sagittal(t5, true) - sagittal(t12, false);
        let rotation = Pose::from_vertebra(&poses[self.apex]).rotation;
        let twist = libm::atan2(-rotation[(0, 1)], rotation[(0, 0)])
            .to_degrees()
            .abs();
        [cobb, kyphosis, twist]
    }
}
fn projected(pose: &VertebraPose, top: bool, sagittal: bool) -> f64 {
    let plates = Endplates::of(&VertebraParams::from(pose));
    let normal = if top {
        plates.top.normal
    } else {
        plates.bottom.normal
    };
    if sagittal {
        libm::atan2(normal.x, normal.z).to_degrees()
    } else {
        libm::atan2(-normal.y, normal.z).to_degrees()
    }
}
fn sagittal(pose: &VertebraPose, top: bool) -> f64 {
    projected(pose, top, true)
}
fn front(fit: &SpineFit, pose: &VertebraPose, top: bool) -> f64 {
    let original = fit
        .vertebrae
        .iter()
        .find(|v| v.level == pose.level)
        .unwrap();
    match original.corner_tilt_rad {
        Some(corner) => {
            corner.to_degrees()
                + (if top { 0.5 } else { -0.5 }) * pose.wedge_front.to_degrees()
                + projected(pose, top, false)
                - projected(&original.pose, top, false)
        }
        None => projected(pose, top, false),
    }
}

pub(super) fn before(
    fit: &SpineFit,
    chain: &Chain,
    layout: &ImplantLayout,
    readings: &Readings,
) -> Result<StopValues, BenchError> {
    let angles = readings.angles(fit, &chain.reference);
    let spans: usize = layout
        .rods
        .iter()
        .map(|r| {
            layout
                .anchors
                .iter()
                .filter(|a| a.side == r.side && a.kind != shapes::implant_layout::AnchorKind::None)
                .count()
                .saturating_sub(1)
        })
        .sum();
    Ok(StopValues {
        stop: BenchStop::Before,
        anchor_force_n: vec![0.0; layout.anchors.len()],
        anchor_demand: vec![0.0; layout.anchors.len()],
        rod_span_use: vec![0.0; spans],
        junction_above: None,
        junction_below: None,
        main_cobb_deg: angles[0],
        kyphosis_deg: angles[1],
        apical_twist_deg: angles[2],
        poses: Some(chain.reference.clone()),
        rod_swing: None,
        cycles_per_year: None,
    })
}

/// Produce loaded stops with no level-above or level-below comparison.
/// Compression follows the column through the discs, so every disc outside
/// the construct has zero couple with or without rods to first order. These
/// measures are explicitly absent; no division of numerical moments decides it.
#[allow(clippy::too_many_arguments)]
pub(super) fn loaded(
    layout: &ImplantLayout,
    draw: &Draw,
    evidence: &Evidence,
    placement: &Placement,
    readings: &Readings,
    chain: &Chain,
    rigid: &RigidState,
    seated: &SeatedState,
    daily: &DailyCases,
) -> Result<Vec<StopValues>, BenchError> {
    let poses: Vec<_> = seated
        .poses
        .iter()
        .zip(&chain.reference)
        .map(|(p, r)| p.to_vertebra(r))
        .collect();
    let angles = readings.angles(&draw.geometry.fit, &poses);
    let mut result = Vec::new();
    for (stop, response) in [
        (BenchStop::RodsIn, None),
        (BenchStop::Standing, Some(&daily.standing)),
        (BenchStop::BendingForward, Some(&daily.bending_forward)),
        (BenchStop::YearOfWalking, Some(&daily.walking_peak)),
    ] {
        let (forces, demands) =
            anchors(layout, draw, evidence, placement, rigid, seated, response)?;
        let stresses = stresses::at_stop(
            layout,
            draw,
            evidence,
            placement,
            rigid,
            seated,
            response,
            daily,
            stop == BenchStop::YearOfWalking,
        )?;
        result.push(StopValues {
            stop,
            anchor_force_n: forces,
            anchor_demand: demands,
            rod_span_use: stresses.uses,
            junction_above: None,
            junction_below: None,
            main_cobb_deg: angles[0],
            kyphosis_deg: angles[1],
            apical_twist_deg: angles[2],
            poses: (stop == BenchStop::RodsIn).then(|| poses.clone()),
            rod_swing: stresses.swing,
            cycles_per_year: stresses.cycles,
        });
    }
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
fn anchors(
    layout: &ImplantLayout,
    draw: &Draw,
    e: &Evidence,
    p: &Placement,
    rigid: &RigidState,
    seated: &SeatedState,
    response: Option<&DailyResponse>,
) -> Result<(Vec<f64>, Vec<f64>), BenchError> {
    let mut forces = vec![0.0; layout.anchors.len()];
    let mut demands = forces.clone();
    for (r, order) in rigid.anchor_order.iter().enumerate() {
        for (h, &mechanical) in order.iter().enumerate() {
            let index = p.anchor_indices[mechanical];
            let mut force = seated.head_wrenches_on_rod[r][h]
                .fixed_rows::<3>(0)
                .into_owned();
            if let Some(response) = response {
                let body = seated.corrected_rods[r].ties[h].body;
                force = seated.poses[body].rotation.transpose() * force
                    + response.head_wrenches_in_body_frame[r][h].fixed_rows::<3>(0);
            }
            let anchor = &layout.anchors[index];
            let demand = anchor_demand::at_head(
                e,
                anchor.level,
                anchor.kind,
                force,
                draw.values.bone_factor,
                draw.anchor_quantile,
            )
            .map_err(|e| refusal(&e.to_string()))?;
            forces[index] = demand.force_n;
            demands[index] = demand.demand;
        }
    }
    Ok((forces, demands))
}
