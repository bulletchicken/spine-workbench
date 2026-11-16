//! Explicit conversion of supplied layout coordinates into mechanical inputs.

use crate::BenchError;
use crate::chain::{Chain, Ground, Pose};
use crate::evidence::{Evidence, TableKind, Unit};
use crate::joint_springs::Stiffness;
use crate::rigid_rod::{Anchor, Construct, HeadKind, Rod};
use crate::rod_beams::{BeamProperties, polyline_stations};
use crate::versions::{self, cell, refusal};
use forward_model::{endplates::Endplates, vertebra_params::VertebraParams};
use nalgebra::{Matrix3, Vector3};
use shapes::implant_layout::{AnchorKind, ImplantLayout, ValueStatus};
use shapes::levels::Level;
use shapes::spine_fit::SpineFit;
use shapes::spine_readings::BodySide;

/// Project input tolerance for a supplied insertion axis being a unit vector.
const DIRECTION_UNIT_TOLERANCE: f64 = 1e-8;

pub(super) struct Placement {
    pub construct: Construct,
    /// Mechanical anchor to the complete layout's anchor array.
    pub anchor_indices: Vec<usize>,
    /// Mechanical rods are left then right, irrespective of input order.
    pub rod_indices: Vec<usize>,
}

pub(super) fn reference(fit: &SpineFit) -> Vec<shapes::made_up::VertebraPose> {
    fit.vertebrae
        .iter()
        .filter(|v| crate::daily_loads::LOAD_LEVELS.contains(&v.level))
        .map(|v| v.pose)
        .collect()
}

pub(super) fn chain(
    fit: &SpineFit,
    stiffness_scale: f64,
    rib_factors: [f64; 3],
    e: &Evidence,
) -> Result<Chain, BenchError> {
    let reference = reference(fit);
    if reference
        .iter()
        .map(|p| p.level)
        .ne(crate::daily_loads::LOAD_LEVELS)
    {
        return Err(refusal("needs-standard-t1-to-l5"));
    }
    let mut stiffness = Vec::new();
    for pose in &reference {
        let mut factors = [1.0; 6];
        if pose.level < Level::T12 {
            factors[3] *= rib_factors[1];
            factors[4] *= rib_factors[0];
            factors[5] *= rib_factors[2];
        }
        stiffness.push(
            Stiffness::from_evidence(e, pose.level == Level::L5)
                .and_then(|k| k.scaled(stiffness_scale, factors))
                .map_err(|error| refusal(&error.to_string()))?,
        );
    }
    let last = reference.last().ok_or(BenchError::NoSpine)?;
    // The fit has no sacrum pose: ground is its fitted L5 lower endplate.
    // This is the bench's boundary convention, not an inferred sacral tilt.
    let bottom = Endplates::of(&VertebraParams::from(last)).bottom.centre;
    Chain::new(
        &reference,
        &stiffness,
        Ground {
            top_mm: bottom,
            rotation: Matrix3::identity(),
        },
    )
    .map_err(|error| refusal(&error.to_string()))
}

pub(super) fn construct(
    fit: &SpineFit,
    layout: &ImplantLayout,
    e: &Evidence,
) -> Result<Placement, BenchError> {
    let reference = reference(fit);
    if reference
        .iter()
        .map(|p| p.level)
        .ne(crate::daily_loads::LOAD_LEVELS)
    {
        return Err(refusal("needs-standard-t1-to-l5"));
    }
    if layout.upper >= layout.lower
        || !reference.iter().any(|p| p.level == layout.upper)
        || !reference.iter().any(|p| p.level == layout.lower)
        || layout.rods.len() != 2
        || layout
            .released_levels
            .iter()
            .any(|l| *l < layout.upper || *l > layout.lower)
    {
        return Err(refusal("invalid-layout-span-or-rods"));
    }
    let mut result = Placement {
        construct: Construct {
            anchors: Vec::new(),
            rods: Vec::new(),
        },
        anchor_indices: Vec::new(),
        rod_indices: Vec::new(),
    };
    for side in [BodySide::Left, BodySide::Right] {
        let rods: Vec<_> = layout
            .rods
            .iter()
            .enumerate()
            .filter(|(_, r)| r.side == side)
            .collect();
        if rods.len() != 1 {
            return Err(refusal("needs-one-rod-per-side"));
        }
        let (proposal_index, rod) = rods[0];
        if !rod.head_offset_mm.is_finite() || !rod.diameter_mm.is_finite() || rod.diameter_mm <= 0.0
        {
            return Err(refusal("missing-or-invalid-rod-geometry"));
        }
        let points: Vec<Vector3<f64>> = rod.points_mm.iter().copied().map(Into::into).collect();
        let stations = polyline_stations(&points).map_err(|error| refusal(&error.to_string()))?;
        let length = *stations.last().ok_or_else(|| refusal("empty-rod"))?;
        let mut anchors: Vec<_> = layout
            .anchors
            .iter()
            .enumerate()
            .filter(|(_, a)| a.side == side && a.kind != AnchorKind::None)
            .collect();
        anchors.sort_by_key(|(_, a)| a.level);
        if anchors.len() < 2 || anchors.windows(2).any(|p| p[0].1.level == p[1].1.level) {
            return Err(refusal("needs-two-distinct-heads-per-rod"));
        }
        let bottom = anchors.last().unwrap().1.level;
        for (index, anchor) in anchors {
            if anchor.entry_status == ValueStatus::Missing
                || anchor.direction_status == ValueStatus::Missing
                || anchor.level < layout.upper
                || anchor.level > layout.lower
                || anchor
                    .entry_mm
                    .iter()
                    .chain(anchor.direction_unit.iter())
                    .any(|x| !x.is_finite())
                || (Vector3::from(anchor.direction_unit).norm() - 1.0).abs()
                    > DIRECTION_UNIT_TOLERANCE
            {
                return Err(refusal("missing-or-invalid-head-geometry"));
            }
            let body = reference
                .iter()
                .position(|p| p.level == anchor.level)
                .ok_or_else(|| refusal("head-not-fitted"))?;
            let pose = Pose::from_vertebra(&reference[body]);
            // The layout's rule: every seat, hook or screw, stands back
            // from its entry along its direction by the head offset.
            let point = Vector3::from(anchor.entry_mm)
                - Vector3::from(anchor.direction_unit) * rod.head_offset_mm;
            let station_mm = if anchor.level == bottom {
                length
            } else {
                closest_station(&points, &stations, point)
            };
            result.construct.anchors.push(Anchor {
                body,
                point_mm: pose.rotation.transpose() * (point - pose.centre_mm),
                rod: result.construct.rods.len(),
                station_mm,
                kind: match anchor.kind {
                    AnchorKind::Screw => HeadKind::Screw,
                    AnchorKind::Hook => HeadKind::Hook,
                    AnchorKind::None => unreachable!(),
                },
            });
            result.anchor_indices.push(index);
        }
        result.construct.rods.push(Rod {
            points_mm: points,
            properties: BeamProperties::from_young_modulus(
                cell(
                    e,
                    TableKind::RodMaterials,
                    versions::material_key(rod.material),
                    "young-modulus",
                    Unit::Gpa,
                )? * 1000.0,
                rod.diameter_mm,
            ),
        });
        result.rod_indices.push(proposal_index);
    }
    Ok(result)
}

// Project initialisation only: closest point on the supplied dense polyline.
// Free stations subsequently minimise on the solver's continuous contour.
fn closest_station(points: &[Vector3<f64>], stations: &[f64], point: Vector3<f64>) -> f64 {
    let mut closest = (f64::INFINITY, 0.0);
    for i in 0..points.len() - 1 {
        let delta = points[i + 1] - points[i];
        let t = ((point - points[i]).dot(&delta) / delta.norm_squared()).clamp(0.0, 1.0);
        let distance = (point - points[i] - delta * t).norm_squared();
        if distance < closest.0 {
            closest = (distance, stations[i] + t * (stations[i + 1] - stations[i]));
        }
    }
    closest.1
}
