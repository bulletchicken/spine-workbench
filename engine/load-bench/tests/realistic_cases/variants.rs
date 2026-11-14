//! Explicit contour probes; sourced layout proposals remain untouched.
use super::{AnchorProposal, LayoutInput, Level, Pose, RodProposal, SpineReading};
use forward_model::rotation::{matrix_from_rotation_vector, rotation_vector_from_matrix};
use implant_layout::rods::{self, RodChoice};
use nalgebra::Vector3;

/// Requested probe fraction, not a calibrated correction rule or default.
const PARTIAL_TARGET_FRACTION: f64 = 0.7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RodVariant {
    LayoutDefault,
    FullTarget,
    SeventyPercentTarget,
}

impl RodVariant {
    pub fn name(self) -> &'static str {
        match self {
            Self::LayoutDefault => "current-default-no-overbend",
            Self::FullTarget => "explicit-zero-extra-target",
            Self::SeventyPercentTarget => "historical-seventy-percent-target-probe",
        }
    }

    pub fn target_fraction(self) -> f64 {
        match self {
            Self::SeventyPercentTarget => PARTIAL_TARGET_FRACTION,
            _ => 1.0,
        }
    }
}

pub const VARIANTS: [RodVariant; 3] = [
    RodVariant::LayoutDefault,
    RodVariant::FullTarget,
    RodVariant::SeventyPercentTarget,
];

pub(super) fn proposals(
    variant: RodVariant,
    input: &LayoutInput,
    levels: &[Level],
    reading: &SpineReading,
    anchors: &[AnchorProposal],
    choice: &RodChoice,
) -> Result<Vec<RodProposal>, String> {
    match variant {
        RodVariant::LayoutDefault => rods::propose(&input.fit, levels, reading, anchors, choice),
        _ => rods::propose_with_overbend(&input.fit, levels, reading, anchors, choice, 0.0),
    }
    .map_err(|e| format!("rod variant: {e}"))
}

/// Instrumented centres interpolate linearly. Orientations follow the
/// SO(3) geodesic Exp(f Log(R_target R_standing^T)) R_standing, composed
/// on the left in patient axes. Outside the span, poses stay standing.
pub(super) fn target_poses(
    variant: RodVariant,
    input: &LayoutInput,
    levels: &[Level],
    reading: &SpineReading,
) -> Result<Vec<Pose>, String> {
    let target = rods::target_spine(&input.fit, levels, reading)
        .map_err(|e| format!("target variant: {e}"))?;
    let mut result: Vec<_> = input
        .fit
        .vertebrae
        .iter()
        .map(|v| Pose::from_vertebra(&v.pose))
        .collect();
    let fraction = variant.target_fraction();
    for target in &target.poses {
        let i = input
            .fit
            .vertebrae
            .iter()
            .position(|v| v.level == target.level)
            .ok_or("target variant level absent from fit")?;
        let before = &result[i];
        let after = Pose::from_vertebra(target);
        let turn = rotation_vector_from_matrix(&(after.rotation * before.rotation.transpose()));
        result[i] = Pose {
            centre_mm: before.centre_mm + (after.centre_mm - before.centre_mm) * fraction,
            rotation: matrix_from_rotation_vector(&(turn * fraction)) * before.rotation,
        };
    }
    Ok(result)
}

/// Variant iii alone replaces the plain Construct's contours with curves
/// through the explicitly interpolated heads. RodProposal remains the
/// actual zero-extra layout return, including all source metadata.
pub(super) fn points(
    variant: RodVariant,
    input: &LayoutInput,
    levels: &[Level],
    anchors: &[AnchorProposal],
    proposal: &RodProposal,
    target: &[Pose],
) -> Result<Vec<Vector3<f64>>, String> {
    if variant != RodVariant::SeventyPercentTarget {
        return Ok(proposal
            .points_mm
            .iter()
            .copied()
            .map(Vector3::from)
            .collect());
    }
    let mut heads = Vec::new();
    for &level in levels {
        let i = input
            .fit
            .vertebrae
            .iter()
            .position(|v| v.level == level)
            .ok_or("partial target level absent from fit")?;
        let anchor = anchors
            .iter()
            .find(|a| a.level == level && a.side == proposal.side)
            .ok_or("partial target head absent")?;
        let before = Pose::from_vertebra(&input.fit.vertebrae[i].pose);
        let standing_head = Vector3::from(anchor.entry_mm)
            - Vector3::from(anchor.direction_unit) * proposal.head_offset_mm;
        let local = before.rotation.transpose() * (standing_head - before.centre_mm);
        heads.push((target[i].centre_mm + target[i].rotation * local).into());
    }
    rods::contour(&heads, levels, 0.0)
        .map(|c| c.points_mm.into_iter().map(Vector3::from).collect())
        .map_err(|e| format!("partial target contour: {e}"))
}
