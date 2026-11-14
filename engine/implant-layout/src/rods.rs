//! Table-backed rod choices and contours on a re-posed target spine.
//!
//! The target and its distribution over joints are this project's own
//! inference. Coordinates are patient millimetres (x anterior, y left,
//! z cranial); positive sagittal tilt is anterior lean. The borrowed head
//! offset is a lumbar model parameter, not a measurement of this child.
//! Missing overhang is never added. Unsupported catalogue sizes and steel
//! alloys are not offered merely because the frozen contract names them.

use crate::LayoutError;
use crate::curves::SpineReading;
use crate::curves::geometry::sagittal_tilt_deg;
use crate::evidence::{Cell, Coordinate, PublishedValue, Table, tables};
use forward_model::endplates::Endplates;
use forward_model::rotation::{rot_z, rotation_vector_from_matrix};
use forward_model::vertebra_params::VertebraParams;
use nalgebra::{Matrix3, Vector3};
use shapes::implant_layout::{
    AnchorKind, AnchorProposal, CoronalLine, CurveSide, RodMaterial, RodProposal, ValueStatus,
};
use shapes::levels::Level;
use shapes::made_up::VertebraPose;
use shapes::spine_fit::SpineFit;
#[cfg(test)]
use shapes::spine_readings::Band;
use shapes::spine_readings::BodySide;

/// This project's own centre-line sampling interval, in millimetres.
pub const SAMPLE_STEP_MM: f64 = 2.0;
/// This project's numerical tolerance, not an anatomical threshold.
const ANGLE_TOLERANCE_DEG: f64 = 1e-8;
/// This project's finite contour-work budget, independent of anatomy.
const MAX_CONTOUR_SAMPLES: usize = 100_000;

/// A material/diameter pair with the sources that actually supply it.
/// Applying a pair to both rods is a project catalogue choice; Wang
/// studied its larger cobalt-chrome rod only on the concave side.
#[derive(Clone, Debug, PartialEq)]
pub struct RodChoice {
    pub material: RodMaterial,
    pub diameter_mm: f64,
    pub sources: Vec<String>,
}

/// Catalogue pairs supported by this crate's evidence. Sia supplies both
/// titanium alloy and cobalt chrome at its common diameter; Abe supplies
/// titanium alloy at its stated diameter; Wang supplies cobalt chrome.
/// UHSS steel does not establish a stainless-steel catalogue entry.
pub fn choices() -> Vec<RodChoice> {
    let mut result = Vec::new();
    for (group, materials) in [
        ("cobalt-chrome", &[RodMaterial::CobaltChrome][..]),
        ("ti6al7nb", &[RodMaterial::TitaniumAlloy][..]),
        (
            "sia-all",
            &[RodMaterial::TitaniumAlloy, RodMaterial::CobaltChrome][..],
        ),
    ] {
        if let Some(cell) = cell(Table::RodChoices, "diameter", group) {
            let values = match &cell.value {
                PublishedValue::Scalar { value } => vec![*value],
                PublishedValue::Grid { values } => values.clone(),
                _ => Vec::new(),
            };
            for material in materials {
                for &diameter_mm in &values {
                    add_choice(&mut result, *material, diameter_mm, &cell.source);
                }
            }
        }
    }
    result.sort_by(|a, b| {
        material_order(a.material)
            .cmp(&material_order(b.material))
            .then(a.diameter_mm.total_cmp(&b.diameter_mm))
    });
    result
}

/// The settled first choice, with its diameter read from the survey.
pub fn default_choice() -> Result<RodChoice, LayoutError> {
    let (diameter_mm, source) = scalar(Table::RodChoices, "diameter", "surgeon-survey")?;
    let mut choice = choices()
        .into_iter()
        .find(|choice| {
            choice.material == RodMaterial::CobaltChrome && choice.diameter_mm == diameter_mm
        })
        .ok_or_else(|| refused("no-sourced-default-rod"))?;
    choice.sources.push(source);
    choice.sources.sort();
    choice.sources.dedup();
    Ok(choice)
}

fn material_order(material: RodMaterial) -> u8 {
    match material {
        RodMaterial::TitaniumAlloy => 0,
        RodMaterial::CobaltChrome => 1,
        RodMaterial::StainlessSteel => 2,
    }
}

fn add_choice(choices: &mut Vec<RodChoice>, material: RodMaterial, diameter: f64, source: &str) {
    if let Some(choice) = choices
        .iter_mut()
        .find(|choice| choice.material == material && choice.diameter_mm == diameter)
    {
        choice.sources.push(source.to_owned());
        choice.sources.sort();
        choice.sources.dedup();
    } else {
        choices.push(RodChoice {
            material,
            diameter_mm: diameter,
            sources: vec![source.to_owned()],
        });
    }
}

/// One target relative rotation, in the lower vertebra's body frame.
/// Components are coronal, sagittal and axial radians, in Rx*Ry*Rz order.
/// They are relative orientations, not the bench's deformation logs.
#[derive(Clone, Debug, PartialEq)]
pub struct TargetJoint {
    pub upper: Level,
    pub lower: Level,
    pub standing_rad: [f64; 3],
    pub target_rad: [f64; 3],
}

/// Only the instrumented poses and joints, top to bottom. The lowest pose
/// remains standing; every other centre follows from its material joint.
#[derive(Clone, Debug, PartialEq)]
pub struct TargetSpine {
    pub poses: Vec<VertebraPose>,
    pub joints: Vec<TargetJoint>,
    pub target_kyphosis_deg: f64,
    pub target_kyphosis_status: ValueStatus,
    pub geometry_status: ValueStatus,
    /// Available coronal reference metadata, retained for compatibility.
    /// This line does not position or constrain the target spine.
    pub coronal_line: CoronalLine,
    pub sources: Vec<String>,
}

/// Status for positions/contour lengths on the drawn spine. Table screw
/// sizes are independent of this scale and keep their own table status.
pub fn geometry_status(fit: &SpineFit) -> ValueStatus {
    match fit.lengths {
        shapes::spine_fit::FitLengths::Measured => ValueStatus::FromTemplate,
        shapes::spine_fit::FitLengths::TypicalSize => ValueStatus::Typical,
    }
}

/// This project's joint-only target: remove each instrumented relative
/// coronal angle, retain its axial angle, and uniformly scale only its
/// T5/T6 through T11/T12 sagittal angles. The scalar objective is the shared
/// reader's projected T5-top to T12-bottom kyphosis, including bone wedges;
/// it is not the sum of finite relative Euler angles. Unchanged relative
/// joints above the span carry that part of the profile rigidly for this
/// reading. Only instrumented poses are returned.
///
/// Material points are the standing midpoint of facing endplate centres,
/// fixed in both bodies. The lowest instrumented pose stays standing;
/// reconstruction never translates a joint or places centres on a line.
/// Without measured side geometry its sagittal profile remains unscaled,
/// with Typical status. With no editable T5--T12 joint the reconstructed
/// kyphosis is retained; an unrelated lumbar joint cannot supply it.
/// The fitted profile must contain T5 and T12 to report this target.
pub fn target_spine(
    fit: &SpineFit,
    levels: &[Level],
    reading: &SpineReading,
) -> Result<TargetSpine, LayoutError> {
    let indices = selected_indices(fit, levels)?;
    let first = indices[0];
    let last = *indices.last().expect("a nonempty span");
    let measured = match reading.kyphosis_t5_t12_deg {
        Ok(value) if value.is_finite() => Some(value),
        Ok(_) => return Err(invalid("rod-target", "non-finite-kyphosis")),
        Err(_) => None,
    };
    let (minimum, source_min) = scalar(
        Table::LevelRuleThresholds,
        "normal-kyphosis-min",
        "sagittal-modifier",
    )?;
    let (maximum, source_max) = scalar(
        Table::LevelRuleThresholds,
        "normal-kyphosis-max",
        "sagittal-modifier",
    )?;
    let editable =
        (first..last).any(|i| scales_kyphosis(fit.vertebrae[i].level, fit.vertebrae[i + 1].level));
    let mut scale = 1.0;
    let mut profile = target_profile(fit, scale, first, last)?;
    let desired = measured
        .filter(|_| editable)
        .map(|value| value.clamp(minimum, maximum));
    if let Some(desired) = desired {
        for _ in 0..TARGET_SCALE_STEPS {
            let value = profile_kyphosis(&profile)?;
            if (value - desired).abs() <= ANGLE_TOLERANCE_DEG {
                break;
            }
            let trial = target_profile(fit, scale + TARGET_SCALE_DIFFERENCE, first, last)?;
            let derivative = (profile_kyphosis(&trial)? - value) / TARGET_SCALE_DIFFERENCE;
            if !derivative.is_finite() || derivative.abs() < ANGLE_TOLERANCE_DEG {
                return Err(refused("rod-target-has-no-scalable-kyphosis-profile"));
            }
            scale -= (value - desired) / derivative;
            if !scale.is_finite() {
                return Err(refused("rod-target-profile-did-not-settle"));
            }
            profile = target_profile(fit, scale, first, last)?;
        }
        if (profile_kyphosis(&profile)? - desired).abs() > ANGLE_TOLERANCE_DEG {
            return Err(refused("rod-target-profile-did-not-settle"));
        }
    }
    let actual = profile_kyphosis(&profile)?;
    let mut joints = Vec::new();
    for i in first..last {
        let standing = relative_angles(&fit.vertebrae[i].pose, &fit.vertebrae[i + 1].pose)?;
        let target = relative_angles(&profile[i], &profile[i + 1])?;
        joints.push(TargetJoint {
            upper: profile[i].level,
            lower: profile[i + 1].level,
            standing_rad: standing,
            target_rad: target,
        });
    }
    Ok(TargetSpine {
        poses: indices.into_iter().map(|i| profile[i]).collect(),
        joints,
        target_kyphosis_deg: actual,
        target_kyphosis_status: match measured {
            None => ValueStatus::Typical,
            Some(value) if (actual - value).abs() <= ANGLE_TOLERANCE_DEG => ValueStatus::Measured,
            Some(_) if desired.is_none() => ValueStatus::FromTemplate,
            Some(_) => ValueStatus::FromTables,
        },
        geometry_status: geometry_status(fit),
        coronal_line: if reading.csvl_y_mm.is_ok() {
            CoronalLine::CentreLine
        } else {
            CoronalLine::L5Vertical
        },
        sources: if desired.is_some() {
            vec![source_min, source_max]
        } else {
            Vec::new()
        },
    })
}

/// This project's deterministic scalar-solve budgets, not anatomy cutoffs.
const TARGET_SCALE_STEPS: usize = 20;
const TARGET_SCALE_DIFFERENCE: f64 = 1e-4;
/// This project's regular Euler-frame tolerance near a quarter-turn pitch.
const JOINT_FRAME_EPS: f64 = 1e-10;

fn selected_indices(fit: &SpineFit, levels: &[Level]) -> Result<Vec<usize>, LayoutError> {
    if fit.vertebrae.is_empty() {
        return Err(LayoutError::NoSpine);
    }
    if levels.is_empty() || levels.windows(2).any(|p| p[0] >= p[1]) {
        return Err(invalid(
            "instrumented-levels",
            "nonempty-increasing-levels-required",
        ));
    }
    let indices = levels
        .iter()
        .map(|level| {
            fit.vertebrae
                .iter()
                .position(|v| v.level == *level)
                .ok_or_else(|| invalid("instrumented-levels", "level-not-in-fit"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if indices.windows(2).any(|p| p[1] != p[0] + 1) {
        return Err(invalid(
            "instrumented-levels",
            "contiguous-fitted-levels-required",
        ));
    }
    for (i, v) in fit.vertebrae.iter().enumerate() {
        let params = VertebraParams::from(&v.pose);
        if v.level != v.pose.level
            || (i > 0 && fit.vertebrae[i - 1].level >= v.level)
            || params.to_numbers().iter().any(|v| !v.is_finite())
            || [
                params.height,
                params.top_half_width,
                params.top_half_depth,
                params.bottom_half_width,
                params.bottom_half_depth,
            ]
            .iter()
            .any(|v| *v <= 0.0)
            || params.wedge_front.abs() >= core::f64::consts::PI
            || params.wedge_side.abs() >= core::f64::consts::PI
        {
            return Err(invalid("fit", "invalid-rod-target-pose"));
        }
    }
    Ok(indices)
}

fn scales_kyphosis(upper: Level, lower: Level) -> bool {
    (Level::T5..=Level::T11).contains(&upper) && lower == Level::ALL[upper.order() + 1]
}

fn matrix_angles(q: &Matrix3<f64>) -> Result<[f64; 3], LayoutError> {
    let sagittal = libm::asin(q[(0, 2)].clamp(-1.0, 1.0));
    if libm::cos(sagittal).abs() < JOINT_FRAME_EPS {
        return Err(refused("rod-joint-outside-regular-angle-frame"));
    }
    Ok([
        libm::atan2(-q[(1, 2)], q[(2, 2)]),
        sagittal,
        libm::atan2(-q[(0, 1)], q[(0, 0)]),
    ])
}
fn relative_angles(upper: &VertebraPose, lower: &VertebraPose) -> Result<[f64; 3], LayoutError> {
    let ru = VertebraParams::from(upper).rotation_matrix();
    let rl = VertebraParams::from(lower).rotation_matrix();
    matrix_angles(&(rl.transpose() * ru))
}

fn target_profile(
    fit: &SpineFit,
    scale: f64,
    first: usize,
    last: usize,
) -> Result<Vec<VertebraPose>, LayoutError> {
    let mut poses: Vec<_> = fit.vertebrae.iter().map(|v| v.pose).collect();
    let standing: Vec<_> = fit
        .vertebrae
        .iter()
        .map(|v| VertebraParams::from(&v.pose))
        .collect();
    // Lowest instrumented and all lower poses are retained exactly.
    for i in (0..last).rev() {
        let ru0 = standing[i].rotation_matrix();
        let rl0 = standing[i + 1].rotation_matrix();
        let q0 = rl0.transpose() * ru0;
        let q = if i >= first {
            let [_, sagittal, axial] = matrix_angles(&q0)?;
            let sagittal = sagittal
                * if scales_kyphosis(poses[i].level, poses[i + 1].level) {
                    scale
                } else {
                    1.0
                };
            if sagittal.abs() >= core::f64::consts::FRAC_PI_2 - JOINT_FRAME_EPS {
                return Err(refused("rod-target-joint-outside-regular-angle-frame"));
            }
            forward_model::rotation::rot_y(sagittal) * rot_z(axial)
        } else {
            q0
        };
        let lower = VertebraParams::from(&poses[i + 1]);
        let rl = lower.rotation_matrix();
        let ru = rl * q;
        let point = (Endplates::of(&standing[i]).bottom.centre
            + Endplates::of(&standing[i + 1]).top.centre)
            * 0.5;
        let a = ru0.transpose() * (point - standing[i].centre);
        let b = rl0.transpose() * (point - standing[i + 1].centre);
        poses[i].centre = (lower.centre + rl * b - ru * a).into();
        poses[i].rotation = rotation_vector_from_matrix(&ru).into();
    }
    Ok(poses)
}

fn profile_kyphosis(poses: &[VertebraPose]) -> Result<f64, LayoutError> {
    let upper = poses
        .iter()
        .find(|p| p.level == Level::T5)
        .ok_or_else(|| refused("rod-target-needs-t5-and-t12"))?;
    let lower = poses
        .iter()
        .find(|p| p.level == Level::T12)
        .ok_or_else(|| refused("rod-target-needs-t5-and-t12"))?;
    Ok(
        sagittal_tilt_deg(&Endplates::of(&VertebraParams::from(upper)).top.normal)
            - sagittal_tilt_deg(&Endplates::of(&VertebraParams::from(lower)).bottom.normal),
    )
}

/// Build the first placement's left and right rods at the target, with
/// zero extra bend. The explicit overbend API remains for later models.
/// The points describe the rod before seating: only its two end heads
/// remain ties after a nonzero thoracic overbend.
pub fn propose(
    fit: &SpineFit,
    levels: &[Level],
    reading: &SpineReading,
    anchors: &[AnchorProposal],
    choice: &RodChoice,
) -> Result<Vec<RodProposal>, LayoutError> {
    propose_with_overbend(fit, levels, reading, anchors, choice, 0.0)
}

/// Build rods with an explicitly requested concave extra kyphosis. Zero
/// retains all target-head knots. A lumbar-only rod always has zero extra.
/// The borrowed head offset applies to screws; a hook holds at its entry.
pub fn propose_with_overbend(
    fit: &SpineFit,
    levels: &[Level],
    reading: &SpineReading,
    anchors: &[AnchorProposal],
    choice: &RodChoice,
    extra_deg: f64,
) -> Result<Vec<RodProposal>, LayoutError> {
    if !extra_deg.is_finite() || extra_deg < 0.0 {
        return Err(invalid("rod-overbend", "finite-nonnegative-angle-required"));
    }
    let sourced = choices()
        .into_iter()
        .find(|available| {
            available.material == choice.material && available.diameter_mm == choice.diameter_mm
        })
        .ok_or_else(|| refused("rod-choice-has-no-source"))?;
    let target = target_spine(fit, levels, reading)?;
    let (head_offset, head_source) =
        scalar(Table::RodChoices, "screw-cantilever-length", "lumbar")?;
    let extra_source = if extra_deg > 0.0 {
        Some(scalar(Table::RodChoices, "overcontour", "concave")?.1)
    } else {
        None
    };
    let mut result = Vec::new();
    for side in [BodySide::Left, BodySide::Right] {
        let mut heads = Vec::new();
        let mut head_levels = Vec::new();
        for pose in &target.poses {
            let matching: Vec<_> = anchors
                .iter()
                .filter(|a| a.level == pose.level && a.side == side)
                .collect();
            if matching.len() != 1 {
                return Err(invalid(
                    "anchors",
                    "one-anchor-per-instrumented-pedicle-required",
                ));
            }
            let anchor = matching[0];
            if anchor.kind == AnchorKind::None {
                continue;
            }
            if anchor.entry_status == ValueStatus::Missing
                || (anchor.kind == AnchorKind::Screw
                    && anchor.direction_status == ValueStatus::Missing)
                || anchor
                    .entry_mm
                    .iter()
                    .chain(&anchor.direction_unit)
                    .any(|v| !v.is_finite())
            {
                return Err(refused("rod-head-geometry-missing"));
            }
            let nominal = fit
                .vertebrae
                .iter()
                .find(|v| v.level == pose.level)
                .expect("the target contains fitted levels");
            let before = VertebraParams::from(&nominal.pose);
            let after = VertebraParams::from(pose);
            let nominal_head = Vector3::from(anchor.entry_mm)
                - Vector3::from(anchor.direction_unit)
                    * if anchor.kind == AnchorKind::Screw {
                        head_offset
                    } else {
                        0.0
                    };
            let local_head = before.rotation_matrix().transpose() * (nominal_head - before.centre);
            heads.push(after.centre + after.rotation_matrix() * local_head);
            head_levels.push(pose.level);
        }
        if heads.len() < 2 {
            return Err(refused("rod-needs-two-occupied-heads"));
        }
        let curve_side =
            reading
                .major
                .and_then(|i| reading.curves.get(i))
                .map_or(CurveSide::Outside, |curve| {
                    if side == curve.convex {
                        CurveSide::Convex
                    } else {
                        CurveSide::Concave
                    }
                });
        let overbend = if curve_side == CurveSide::Concave {
            extra_deg
        } else {
            0.0
        };
        let heads: Vec<[f64; 3]> = heads.into_iter().map(Into::into).collect();
        let contoured = contour(&heads, &head_levels, overbend)?;
        let overbend = contoured.overbend_deg;
        let points = contoured.points_mm;
        let length_mm = contoured.length_mm;
        let mut sources = target.sources.clone();
        sources.extend(sourced.sources.iter().cloned());
        for anchor in anchors
            .iter()
            .filter(|a| levels.contains(&a.level) && a.side == side && a.kind != AnchorKind::None)
        {
            sources.extend(anchor.sources.iter().cloned());
        }
        sources.extend(
            choice
                .sources
                .iter()
                .filter(|source| tables().source(source).is_some())
                .cloned(),
        );
        sources.push(head_source.clone());
        if overbend != 0.0
            && let Some(source) = &extra_source
        {
            sources.push(source.clone());
        }
        sources.sort();
        sources.dedup();
        result.push(RodProposal {
            standing_points_mm: Vec::new(),
            side,
            curve_side,
            material: choice.material,
            diameter_mm: choice.diameter_mm,
            target_kyphosis_deg: target.target_kyphosis_deg,
            target_kyphosis_status: target.target_kyphosis_status,
            coronal_line: target.coronal_line,
            overbend_deg: overbend,
            head_offset_mm: head_offset,
            length_mm,
            geometry_status: target.geometry_status,
            points_mm: points,
            sources,
        });
    }
    Ok(result)
}

/// A contoured rod before seating, plus its exact sagittal tangent at
/// each returned point. The T12 station is included when it lies in the
/// span. Length is along the continuous contour, not its chordal drawing.
#[derive(Clone, Debug, PartialEq)]
pub struct RodContour {
    pub points_mm: Vec<[f64; 3]>,
    pub sagittal_tangent_rad: Vec<f64>,
    pub thoracic_end_index: Option<usize>,
    pub length_mm: f64,
    pub overbend_deg: f64,
}

/// This project's relative geometric round-off tolerance.
const CONTOUR_EPS: f64 = 1e-11;
/// This project's bounded arc-length inversion and quadrature budgets.
const ARC_INVERSION_STEPS: usize = 40;
const QUADRATURE_DEPTH: usize = 10;

#[derive(Clone, Copy, Debug)]
struct Station {
    span: usize,
    fraction: f64,
}

/// Construct a rod from target heads and their occupied levels. No extra
/// bend keeps every head. A positive extra adds uniform sagittal tangent
/// turn over the target's reference thoracic arc, then only the thoracic
/// length adjusts to meet the end-head chord. The lumbar part is carried
/// rigidly, preserving its curvature. A final rigid sagittal placement
/// puts both end heads exactly on the contour; intermediate heads are not
/// constraints on an overbent rod. These are this project's own rules.
///
/// Straight and co-circular sagittal targets use exact line/arc geometry;
/// other targets use a cubic clamped to adjacent secants. All supplied
/// heads remain interpolation knots of the zero-overbend target. Sampling
/// includes heads, T12 and the thoracic midpoint, at <=2 mm arc intervals.
pub fn contour(
    heads: &[[f64; 3]],
    levels: &[Level],
    overbend_deg: f64,
) -> Result<RodContour, LayoutError> {
    if heads.len() != levels.len() || levels.windows(2).any(|p| p[0] >= p[1]) {
        return Err(invalid("rod-heads", "matching-increasing-levels-required"));
    }
    if !overbend_deg.is_finite() || overbend_deg < 0.0 {
        return Err(invalid("rod-overbend", "finite-nonnegative-angle-required"));
    }
    let heads: Vec<_> = heads.iter().copied().map(Vector3::from).collect();
    let reference = ReferenceCurve::new(&heads)?;
    let cut = thoracic_end(levels);
    build_contour(
        &reference,
        cut,
        if cut.is_some() { overbend_deg } else { 0.0 },
    )
}

/// Zero-overbend target interpolation, retaining its existing public
/// name. Exactly straight/circular sagittal data use their analytic
/// primitives; all other data use the clamped C2 cubic. Every head is a
/// returned knot, including noncoplanar heads.
pub fn clamped_spline(heads: &[Vector3<f64>]) -> Result<Vec<[f64; 3]>, LayoutError> {
    let reference = ReferenceCurve::new(heads)?;
    Ok(build_contour(&reference, None, 0.0)?.points_mm)
}

fn thoracic_end(levels: &[Level]) -> Option<Station> {
    if !levels.iter().any(|l| (Level::T1..=Level::T12).contains(l)) {
        return None;
    }
    let last = levels.len() - 1;
    if levels[last] <= Level::T12 {
        return Some(Station {
            span: last - 1,
            fraction: 1.0,
        });
    }
    for (span, pair) in levels.windows(2).enumerate() {
        if pair[0] <= Level::T12 && pair[1] > Level::T12 {
            // Project inference when no occupied T12 head exists: its
            // labelled-level fraction locates the station between heads.
            let order =
                |l: Level| l.order() - usize::from(l >= Level::L1 && !levels.contains(&Level::T13));
            let fraction = (order(Level::T12) - order(pair[0])) as f64
                / (order(pair[1]) - order(pair[0])) as f64;
            return (span != 0 || fraction != 0.0).then_some(Station { span, fraction });
        }
    }
    None
}

struct Placement<'a> {
    reference: &'a ReferenceCurve,
    cut: Option<Station>,
    cut_arc: f64,
    theta: f64,
    scale: f64,
    alignment: f64,
    prefix: Vec<Vector3<f64>>,
    cut_displacement: Vector3<f64>,
    cut_point: Vector3<f64>,
}
impl<'a> Placement<'a> {
    fn new(
        reference: &'a ReferenceCurve,
        cut: Option<Station>,
        theta: f64,
    ) -> Result<Self, LayoutError> {
        let mut result = Self {
            reference,
            cut,
            cut_arc: 0.0,
            theta,
            scale: 1.0,
            alignment: 0.0,
            prefix: vec![Vector3::zeros(); reference.spline.heads.len()],
            cut_displacement: Vector3::zeros(),
            cut_point: reference.spline.heads[0],
        };
        if theta == 0.0 {
            return Ok(result);
        }
        let cut = cut.expect("an overbend has a thoracic span");
        result.cut_arc = reference.arc_at(cut);
        if result.cut_arc <= CONTOUR_EPS {
            return Err(refused("rod-thoracic-arc-has-no-sagittal-length"));
        }
        for span in 0..=cut.span {
            let end = if span == cut.span { cut.fraction } else { 1.0 };
            let delta = result.integrated_turn(span, end);
            result.prefix[span + 1] = result.prefix[span] + delta;
        }
        result.cut_displacement =
            result.prefix[cut.span] + result.integrated_turn(cut.span, cut.fraction);
        result.cut_point = reference.eval(cut).0;
        let end = *reference.spline.heads.last().expect("two heads");
        let a = result.cut_displacement;
        let b = sagittal_turn(end - result.cut_point, theta);
        let target = end - reference.spline.heads[0];
        let aa = sagittal_dot(a, a);
        let bb = 2.0 * sagittal_dot(a, b);
        let cc = sagittal_dot(b, b) - sagittal_dot(target, target);
        let discriminant = bb * bb - 4.0 * aa * cc;
        if aa <= CONTOUR_EPS * CONTOUR_EPS
            || discriminant < -CONTOUR_EPS * (1.0 + bb * bb + (4.0 * aa * cc).abs())
        {
            return Err(refused("rod-contour-cannot-meet-end-heads"));
        }
        let root = libm::sqrt(discriminant.max(0.0));
        let stable = -0.5 * (bb + if bb >= 0.0 { root } else { -root });
        let candidates = if stable == 0.0 {
            [-bb / (2.0 * aa), f64::NAN]
        } else {
            [stable / aa, cc / stable]
        };
        result.scale = candidates
            .into_iter()
            .filter(|q| q.is_finite() && *q > 0.0)
            .min_by(|a, b| (a - 1.0).abs().total_cmp(&(b - 1.0).abs()))
            .ok_or_else(|| refused("rod-contour-cannot-meet-end-heads"))?;
        let raw = a * result.scale + b;
        result.alignment = sagittal_angle(target) - sagittal_angle(raw);
        Ok(result)
    }
    fn before_cut(&self, p: Station) -> bool {
        self.cut.is_some_and(|cut| {
            p.span < cut.span || (p.span == cut.span && p.fraction <= cut.fraction)
        })
    }
    fn integrated_turn(&self, span: usize, u: f64) -> Vector3<f64> {
        if u == 0.0 {
            return Vector3::zeros();
        }
        let r = self.reference;
        if let Some(turn) = r.analytic_turn(span, u) {
            let arc = r.partial_arc(span, u);
            let start = sagittal_angle(
                r.eval(Station {
                    span,
                    fraction: 0.0,
                })
                .1,
            ) + self.theta * r.sagittal_prefix[span] / self.cut_arc;
            let delta = turn + self.theta * arc / self.cut_arc;
            let mid = start + delta * 0.5;
            let distance = arc * sinc(delta * 0.5);
            return Vector3::new(distance * libm::sin(mid), 0.0, -distance * libm::cos(mid));
        }
        let component = |axis| {
            integrate(
                &|fraction| {
                    let p = Station { span, fraction };
                    let v = r.eval(p).1;
                    sagittal_turn(v, self.theta * r.arc_at(p) / self.cut_arc)[axis]
                },
                0.0,
                u,
            )
        };
        Vector3::new(component(0), 0.0, component(2))
    }
    fn value(&self, p: Station) -> (Vector3<f64>, f64) {
        let (target, derivative) = self.reference.eval(p);
        if self.theta == 0.0 {
            return (target, sagittal_angle(derivative));
        }
        let (raw, extra) = if self.before_cut(p) {
            (
                (self.prefix[p.span] + self.integrated_turn(p.span, p.fraction)) * self.scale,
                self.theta * self.reference.arc_at(p) / self.cut_arc,
            )
        } else {
            (
                self.cut_displacement * self.scale
                    + sagittal_turn(target - self.cut_point, self.theta),
                self.theta,
            )
        };
        let mut point = self.reference.spline.heads[0] + sagittal_turn(raw, self.alignment);
        point.y = target.y;
        (point, sagittal_angle(derivative) + extra + self.alignment)
    }
    fn speed(&self, p: Station) -> f64 {
        let v = self.reference.eval(p).1;
        let scale = if self.theta != 0.0 && self.before_cut(p) {
            self.scale
        } else {
            1.0
        };
        libm::sqrt(scale * scale * (v.x * v.x + v.z * v.z) + v.y * v.y)
    }
    fn length(&self, span: usize, a: f64, b: f64) -> f64 {
        integrate(&|fraction| self.speed(Station { span, fraction }), a, b)
    }
}

fn build_contour(
    reference: &ReferenceCurve,
    cut: Option<Station>,
    extra_deg: f64,
) -> Result<RodContour, LayoutError> {
    let placement = Placement::new(reference, cut, extra_deg.to_radians())?;
    let middle = if extra_deg != 0.0 {
        Some(reference.at_arc(placement.cut_arc * 0.5))
    } else {
        None
    };
    let mut points = vec![reference.spline.heads[0].into()];
    let mut angles = vec![
        placement
            .value(Station {
                span: 0,
                fraction: 0.0,
            })
            .1,
    ];
    let mut end_index = None;
    let mut length = 0.0;
    for span in 0..reference.spline.steps.len() {
        let mut breaks = vec![0.0, 1.0];
        for station in [cut, middle].into_iter().flatten() {
            if station.span == span && station.fraction > 0.0 && station.fraction < 1.0 {
                breaks.push(station.fraction);
            }
        }
        breaks.sort_by(f64::total_cmp);
        breaks.dedup();
        for interval in breaks.windows(2) {
            let (a, b) = (interval[0], interval[1]);
            let arc = placement.length(span, a, b);
            if !arc.is_finite() || arc <= 0.0 {
                return Err(refused("rod-contour-has-degenerate-span"));
            }
            length += arc;
            let count = (libm::ceil(arc / SAMPLE_STEP_MM) as usize).max(1);
            if points.len().saturating_add(count) > MAX_CONTOUR_SAMPLES {
                return Err(refused("rod-contour-work-budget"));
            }
            for index in 1..=count {
                let fraction = if index == count {
                    b
                } else if reference.analytic_turn(span, 1.0).is_some()
                    && reference
                        .spline
                        .heads
                        .iter()
                        .all(|p| p.y == reference.spline.heads[0].y)
                {
                    // A planar line/arc has constant speed in this
                    // parameter interval, including its thoracic scale.
                    a + (b - a) * index as f64 / count as f64
                } else {
                    let wanted = arc * index as f64 / count as f64;
                    let (mut low, mut high) = (a, b);
                    for _ in 0..ARC_INVERSION_STEPS {
                        let mid = (low + high) * 0.5;
                        if placement.length(span, a, mid) < wanted {
                            low = mid;
                        } else {
                            high = mid;
                        }
                    }
                    (low + high) * 0.5
                };
                let station = Station { span, fraction };
                let (mut point, angle) = placement.value(station);
                if span + 1 == reference.spline.heads.len() - 1 && fraction == 1.0 {
                    point = *reference.spline.heads.last().expect("two heads");
                }
                points.push(point.into());
                let previous = *angles.last().expect("a starting tangent");
                angles.push(previous + wrapped(angle - previous));
                if cut.is_some_and(|c| c.span == span && c.fraction == fraction) {
                    end_index = Some(points.len() - 1);
                }
            }
        }
        if cut.is_some_and(|c| c.span == span + 1 && c.fraction == 0.0) {
            end_index = Some(points.len() - 1);
        }
    }
    Ok(RodContour {
        points_mm: points,
        sagittal_tangent_rad: angles,
        thoracic_end_index: end_index,
        length_mm: length,
        overbend_deg: extra_deg,
    })
}

fn sagittal_angle(v: Vector3<f64>) -> f64 {
    libm::atan2(v.x, -v.z)
}
fn sagittal_dot(a: Vector3<f64>, b: Vector3<f64>) -> f64 {
    a.x * b.x + a.z * b.z
}
fn sagittal_turn(v: Vector3<f64>, angle: f64) -> Vector3<f64> {
    let (s, c) = (libm::sin(angle), libm::cos(angle));
    Vector3::new(c * v.x - s * v.z, v.y, s * v.x + c * v.z)
}
fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-5 {
        let x2 = x * x;
        1.0 - x2 / 6.0 + x2 * x2 / 120.0
    } else {
        libm::sin(x) / x
    }
}
fn wrapped(angle: f64) -> f64 {
    libm::atan2(libm::sin(angle), libm::cos(angle))
}
#[cfg(test)]
fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (Vector3::from(a) - Vector3::from(b)).norm()
}

// Mathematical Gauss-Legendre constants, not observations from a paper.
fn gauss(f: &impl Fn(f64) -> f64, a: f64, b: f64) -> f64 {
    const NODES: [f64; 4] = [
        0.1834346424956498,
        0.525532409916329,
        0.7966664774136267,
        0.9602898564975363,
    ];
    const WEIGHTS: [f64; 4] = [
        0.362683783378362,
        0.3137066458778873,
        0.2223810344533745,
        0.1012285362903763,
    ];
    let mid = (a + b) * 0.5;
    let half = (b - a) * 0.5;
    half * NODES
        .iter()
        .zip(WEIGHTS)
        .map(|(node, w)| w * (f(mid - half * node) + f(mid + half * node)))
        .sum::<f64>()
}
fn integrate(f: &impl Fn(f64) -> f64, a: f64, b: f64) -> f64 {
    fn refine(f: &impl Fn(f64) -> f64, a: f64, b: f64, whole: f64, depth: usize) -> f64 {
        let mid = (a + b) * 0.5;
        let left = gauss(f, a, mid);
        let right = gauss(f, mid, b);
        let split = left + right;
        if depth == 0 || (split - whole).abs() <= CONTOUR_EPS * (1.0 + split.abs()) {
            split
        } else {
            refine(f, a, mid, left, depth - 1) + refine(f, mid, b, right, depth - 1)
        }
    }
    if a == b {
        0.0
    } else {
        refine(f, a, b, gauss(f, a, b), QUADRATURE_DEPTH)
    }
}

enum SagittalShape {
    Cubic,
    Line,
    Circle {
        centre: [f64; 2],
        radius: f64,
        angles: Vec<f64>,
    },
}
struct ReferenceCurve {
    spline: CubicSpline,
    shape: SagittalShape,
    sagittal_prefix: Vec<f64>,
}
impl ReferenceCurve {
    fn new(heads: &[Vector3<f64>]) -> Result<Self, LayoutError> {
        let spline = CubicSpline::new(heads)?;
        let shape = analytic_shape(heads);
        let mut result = Self {
            spline,
            shape,
            sagittal_prefix: vec![0.0],
        };
        for span in 0..heads.len() - 1 {
            let arc = result.partial_arc(span, 1.0);
            if !arc.is_finite() || arc <= CONTOUR_EPS {
                return Err(refused("rod-target-has-no-sagittal-span"));
            }
            result
                .sagittal_prefix
                .push(result.sagittal_prefix[span] + arc);
        }
        Ok(result)
    }
    fn eval(&self, p: Station) -> (Vector3<f64>, Vector3<f64>) {
        let (mut point, mut derivative) = self.spline.eval(p);
        match &self.shape {
            SagittalShape::Cubic => {}
            SagittalShape::Line => {
                let a = self.spline.heads[p.span];
                let delta = self.spline.heads[p.span + 1] - a;
                point.x = a.x + p.fraction * delta.x;
                point.z = a.z + p.fraction * delta.z;
                derivative.x = delta.x;
                derivative.z = delta.z;
            }
            SagittalShape::Circle {
                centre,
                radius,
                angles,
            } => {
                let delta = angles[p.span + 1] - angles[p.span];
                let angle = angles[p.span] + p.fraction * delta;
                let (s, c) = (libm::sin(angle), libm::cos(angle));
                point.x = centre[0] + radius * c;
                point.z = centre[1] + radius * s;
                derivative.x = -radius * s * delta;
                derivative.z = radius * c * delta;
            }
        }
        if p.fraction == 0.0 {
            point = self.spline.heads[p.span];
        }
        if p.fraction == 1.0 {
            point = self.spline.heads[p.span + 1];
        }
        (point, derivative)
    }
    fn analytic_turn(&self, span: usize, u: f64) -> Option<f64> {
        match &self.shape {
            SagittalShape::Line => Some(0.0),
            SagittalShape::Circle { angles, .. } => Some((angles[span + 1] - angles[span]) * u),
            SagittalShape::Cubic => None,
        }
    }
    fn partial_arc(&self, span: usize, u: f64) -> f64 {
        match &self.shape {
            SagittalShape::Line => {
                let v = self.spline.heads[span + 1] - self.spline.heads[span];
                libm::sqrt(sagittal_dot(v, v)) * u
            }
            SagittalShape::Circle { radius, angles, .. } => {
                radius * (angles[span + 1] - angles[span]).abs() * u
            }
            SagittalShape::Cubic => integrate(
                &|fraction| {
                    let v = self.eval(Station { span, fraction }).1;
                    libm::sqrt(sagittal_dot(v, v))
                },
                0.0,
                u,
            ),
        }
    }
    fn arc_at(&self, p: Station) -> f64 {
        self.sagittal_prefix[p.span] + self.partial_arc(p.span, p.fraction)
    }
    fn at_arc(&self, arc: f64) -> Station {
        let span = self
            .sagittal_prefix
            .windows(2)
            .position(|p| arc <= p[1])
            .unwrap_or(self.spline.steps.len() - 1);
        let wanted = arc - self.sagittal_prefix[span];
        let (mut low, mut high) = (0.0, 1.0);
        for _ in 0..ARC_INVERSION_STEPS {
            let mid = (low + high) * 0.5;
            if self.partial_arc(span, mid) < wanted {
                low = mid;
            } else {
                high = mid;
            }
        }
        Station {
            span,
            fraction: (low + high) * 0.5,
        }
    }
}

fn analytic_shape(heads: &[Vector3<f64>]) -> SagittalShape {
    let a = heads[0];
    let chord = heads[heads.len() - 1] - a;
    let chord2 = sagittal_dot(chord, chord);
    if chord2 <= CONTOUR_EPS {
        return SagittalShape::Cubic;
    }
    let cross = |p: Vector3<f64>| p.x * chord.z - p.z * chord.x;
    if heads.iter().all(|p| {
        cross(*p - a).abs() <= CONTOUR_EPS * chord2
            && (0.0..=1.0).contains(&(sagittal_dot(*p - a, chord) / chord2))
    }) && heads
        .windows(2)
        .all(|p| sagittal_dot(p[1] - p[0], chord) > 0.0)
    {
        return SagittalShape::Line;
    }
    let Some(middle) = heads
        .iter()
        .map(|p| *p - a)
        .max_by(|a, b| cross(*a).abs().total_cmp(&cross(*b).abs()))
    else {
        return SagittalShape::Cubic;
    };
    let determinant = 2.0 * cross(middle);
    if determinant.abs() <= CONTOUR_EPS * chord2 {
        return SagittalShape::Cubic;
    }
    let mm = sagittal_dot(middle, middle);
    let centre = [
        a.x + (mm * chord.z - chord2 * middle.z) / determinant,
        a.z + (middle.x * chord2 - chord.x * mm) / determinant,
    ];
    let radius = libm::hypot(a.x - centre[0], a.z - centre[1]);
    if heads.iter().any(|p| {
        (libm::hypot(p.x - centre[0], p.z - centre[1]) - radius).abs()
            > CONTOUR_EPS * (1.0 + radius)
    }) {
        return SagittalShape::Cubic;
    }
    let mut angles = vec![libm::atan2(a.z - centre[1], a.x - centre[0])];
    for p in &heads[1..] {
        let previous = *angles.last().expect("one angle");
        angles.push(previous + wrapped(libm::atan2(p.z - centre[1], p.x - centre[0]) - previous));
    }
    let sign = (angles[1] - angles[0]).signum();
    if sign == 0.0 || angles.windows(2).any(|p| (p[1] - p[0]) * sign <= 0.0) {
        SagittalShape::Cubic
    } else {
        SagittalShape::Circle {
            centre,
            radius,
            angles,
        }
    }
}

struct CubicSpline {
    heads: Vec<Vector3<f64>>,
    steps: Vec<f64>,
    second: Vec<Vector3<f64>>,
}
impl CubicSpline {
    fn new(heads: &[Vector3<f64>]) -> Result<Self, LayoutError> {
        if heads.len() < 2 || heads.iter().any(|p| p.iter().any(|v| !v.is_finite())) {
            return Err(invalid("rod-heads", "two-finite-heads-required"));
        }
        let steps: Vec<_> = heads.windows(2).map(|p| (p[1] - p[0]).norm()).collect();
        if steps.iter().any(|s| !s.is_finite() || *s <= 0.0) {
            return Err(invalid("rod-heads", "distinct-heads-required"));
        }
        let n = heads.len();
        let mut diagonal = vec![0.0; n];
        let mut upper = vec![0.0; n];
        let mut rhs = vec![Vector3::zeros(); n];
        diagonal[0] = 2.0 * steps[0];
        upper[0] = steps[0];
        diagonal[n - 1] = 2.0 * steps[n - 2];
        for i in 1..n - 1 {
            diagonal[i] = 2.0 * (steps[i - 1] + steps[i]);
            upper[i] = steps[i];
            rhs[i] = ((heads[i + 1] - heads[i]) / steps[i]
                - (heads[i] - heads[i - 1]) / steps[i - 1])
                * 6.0;
        }
        for i in 1..n {
            let multiplier = steps[i - 1] / diagonal[i - 1];
            diagonal[i] -= multiplier * upper[i - 1];
            let previous = rhs[i - 1];
            rhs[i] -= previous * multiplier;
        }
        let mut second = vec![Vector3::zeros(); n];
        second[n - 1] = rhs[n - 1] / diagonal[n - 1];
        for i in (0..n - 1).rev() {
            second[i] = (rhs[i] - second[i + 1] * upper[i]) / diagonal[i];
        }
        Ok(Self {
            heads: heads.to_vec(),
            steps,
            second,
        })
    }
    fn eval(&self, p: Station) -> (Vector3<f64>, Vector3<f64>) {
        let b = p.fraction;
        let a = 1.0 - b;
        let scale = self.steps[p.span] * self.steps[p.span] / 6.0;
        let m = self.second[p.span];
        let n = self.second[p.span + 1];
        (
            self.heads[p.span] * a
                + self.heads[p.span + 1] * b
                + (m * (a * a * a - a) + n * (b * b * b - b)) * scale,
            self.heads[p.span + 1] - self.heads[p.span]
                + (m * (1.0 - 3.0 * a * a) + n * (3.0 * b * b - 1.0)) * scale,
        )
    }
}

fn cell(table: Table, key: &str, group: &str) -> Option<&'static Cell> {
    tables().lookup(
        table,
        &Coordinate {
            key: key.into(),
            group: Some(group.into()),
            ..Coordinate::default()
        },
    )
}
fn scalar(table: Table, key: &str, group: &str) -> Result<(f64, String), LayoutError> {
    let cell = cell(table, key, group)
        .ok_or_else(|| refused(&format!("missing-evidence:{group}:{key}")))?;
    match cell.value {
        PublishedValue::Scalar { value } => Ok((value, cell.source.clone())),
        _ => Err(refused(&format!("non-scalar-evidence:{group}:{key}"))),
    }
}
fn invalid(field: &str, reason: &str) -> LayoutError {
    LayoutError::InvalidValue {
        field: field.into(),
        reason: reason.into(),
    }
}
fn refused(reason: &str) -> LayoutError {
    LayoutError::Refused(reason.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curves::{Classification, Curve};
    use forward_model::rotation::rot_y;
    use shapes::implant_layout::{CannotTellCode, PedicleEstimate};
    use shapes::made_up::CurveRegion;
    use shapes::spine_fit::{
        FitAngles, FitRmsMm, FitStandardDeviations, FitStopReason, FittedVertebra, InferredFit,
    };

    fn fit(kyphosis: f64) -> SpineFit {
        let levels = [
            Level::T5,
            Level::T6,
            Level::T7,
            Level::T8,
            Level::T9,
            Level::T10,
            Level::T11,
            Level::T12,
            Level::L1,
            Level::L2,
            Level::L5,
        ];
        let vertebrae = levels
            .iter()
            .enumerate()
            .map(|(i, level)| {
                let lean = if i <= 7 {
                    (0.5 - i as f64 / 7.0) * kyphosis.to_radians()
                } else {
                    (-0.5 * kyphosis + (i - 7) as f64 * 3.0).to_radians()
                };
                let params = VertebraParams {
                    centre: Vector3::new(0.0, 8.0, (levels.len() - i) as f64 * 24.0),
                    rotation: rotation_vector_from_matrix(&rot_y(lean)),
                    height: 20.0,
                    wedge_front: 0.0,
                    wedge_side: 0.0,
                    top_half_width: 15.0,
                    top_half_depth: 10.0,
                    bottom_half_width: 15.0,
                    bottom_half_depth: 10.0,
                };
                FittedVertebra {
                    level: *level,
                    pose: params.pose(*level),
                    angles: FitAngles {
                        side_tilt_rad: 0.0,
                        forward_lean_rad: lean,
                        twist_rad: 0.0,
                    },
                    sd: FitStandardDeviations {
                        centre_mm: [0.0; 3],
                        angles_rad: [0.0; 3],
                    },
                    inferred: InferredFit {
                        depth: false,
                        width: false,
                        twist: false,
                    },
                    rms_mm: FitRmsMm {
                        front: Some(0.0),
                        side: Some(0.0),
                    },
                    corner_tilt_rad: None,
                }
            })
            .collect();
        SpineFit {
            vertebrae,
            side_scale: 1.0,
            side_offset_mm: 0.0,
            origin_level: Level::L5,
            iterations: 0,
            converged: true,
            stop_reason: FitStopReason::SmallStep,
            rms_mm: 0.0,
            lengths: shapes::spine_fit::FitLengths::Measured,
            typical_curve_deg: None,
            front_film: shapes::spine_fit::FilmPlacement {
                mm_per_px: [1.0; 2],
                origin_px: [0.0; 2],
                facing_assumed: false,
            },
            side_film: Some(shapes::spine_fit::FilmPlacement {
                mm_per_px: [1.0; 2],
                origin_px: [0.0; 2],
                facing_assumed: false,
            }),
        }
    }

    fn reading(kyphosis: f64) -> SpineReading {
        SpineReading {
            vertebrae: Vec::new(),
            curves: vec![Curve {
                upper_index: 0,
                lower_index: 7,
                apex_index: 3,
                region: Some(CurveRegion::MainThoracic),
                convex: BodySide::Right,
                cobb_deg: 40.0,
                signed_cobb_deg: 40.0,
                apical_deviation_mm: -10.0,
                bending_cobb_deg: Err(CannotTellCode::NoBendingFilms),
                straightening_bend: Err(CannotTellCode::NoBendingFilms),
                bending_disc_below_deg: Err(CannotTellCode::NoBendingFilms),
                structural: Ok(true),
            }],
            major: Some(0),
            csvl_y_mm: Ok(2.0),
            kyphosis_t2_t5_deg: Err(CannotTellCode::NoSideFilm),
            kyphosis_t5_t12_deg: Ok(kyphosis),
            kyphosis_t10_l2_deg: Ok(0.0),
            l4_tilt_deg: Err(CannotTellCode::NoCurve),
            classification: Classification {
                curve_type: Err(CannotTellCode::NoCurve),
                lumbar_modifier: Err(CannotTellCode::NoCurve),
                sagittal_modifier: Err(CannotTellCode::NoSideFilm),
            },
            minor_curve_minimum_deg: None,
            sources: Vec::new(),
        }
    }

    fn anchors(fit: &SpineFit) -> Vec<AnchorProposal> {
        fit.vertebrae
            .iter()
            .flat_map(|v| {
                [BodySide::Left, BodySide::Right].map(|side| {
                    let centre = Vector3::from(v.pose.centre);
                    let sign = if side == BodySide::Left { 1.0 } else { -1.0 };
                    AnchorProposal {
                        wider_than_pedicle: Vec::new(),
                        screw_here: None,
                        diameter_sources: Vec::new(),
                        length_sources: Vec::new(),
                        direction_sources: Vec::new(),
                        look_sources: Vec::new(),
                        level: v.level,
                        side,
                        kind: AnchorKind::Screw,
                        entry_mm: (centre + Vector3::new(-20.0, sign * 12.0, 0.0)).into(),
                        entry_status: ValueStatus::FromTemplate,
                        direction_unit: [1.0, 0.0, 0.0],
                        direction_status: ValueStatus::FromTables,
                        medial_angle: None,
                        medial_angles_beside: Vec::new(),
                        convergence_deg: Band {
                            p10: 0.0,
                            p50: 0.0,
                            p90: 0.0,
                        },
                        diameter_mm: Some(5.0),
                        diameter_also_seen: Vec::new(),
                        diameter_status: ValueStatus::FromTables,
                        width_ratio: None,
                        diameters_beside: Vec::new(),
                        screw_to_pedicle: None,
                        chord_mm: None,
                        chord_share: None,
                        length_mm: None,
                        length_also_seen: Vec::new(),
                        length_status: ValueStatus::Missing,
                        usual_length_mm: None,
                        pedicle: PedicleEstimate {
                            sources: Vec::new(),
                            outer_width_mm: None,
                            inner_width_mm: None,
                            versions_too_narrow: 0,
                            versions_inner_under_1_mm: 0,
                            curve_side: CurveSide::Outside,
                            status: ValueStatus::Missing,
                        },
                        looks: Vec::new(),
                        sources: Vec::new(),
                    }
                })
            })
            .collect()
    }

    #[test]
    fn every_offered_pair_has_evidence_and_unsupported_alloys_stay_absent() {
        let offered = choices();
        assert!(!offered.is_empty());
        for choice in &offered {
            assert!(!choice.sources.is_empty());
            assert!(
                choice
                    .sources
                    .iter()
                    .all(|key| tables().source(key).is_some())
            );
            assert_ne!(choice.material, RodMaterial::StainlessSteel);
        }
        let first = default_choice().unwrap();
        let survey = scalar(Table::RodChoices, "diameter", "surgeon-survey").unwrap();
        assert_eq!(first.diameter_mm, survey.0);
        assert_eq!(first.material, RodMaterial::CobaltChrome);
        assert!(first.sources.contains(&survey.1));
    }

    #[test]
    fn target_clamps_kyphosis_by_turning_joints_and_keeps_its_standing_base() {
        let maximum = scalar(
            Table::LevelRuleThresholds,
            "normal-kyphosis-max",
            "sagittal-modifier",
        )
        .unwrap()
        .0;
        let fit = fit(maximum + 12.0);
        let levels: Vec<_> = fit.vertebrae.iter().map(|v| v.level).collect();
        let target = target_spine(&fit, &levels, &reading(maximum + 12.0)).unwrap();
        assert!((profile_kyphosis(&target.poses).unwrap() - maximum).abs() < 1e-8);
        assert_eq!(target.target_kyphosis_status, ValueStatus::FromTables);
        assert_eq!(target.coronal_line, CoronalLine::CentreLine);
        assert_eq!(
            target.poses.last().unwrap(),
            &fit.vertebrae.last().unwrap().pose
        );
        for (i, pair) in target.poses.windows(2).enumerate() {
            let before =
                relative_angles(&fit.vertebrae[i].pose, &fit.vertebrae[i + 1].pose).unwrap();
            let after = relative_angles(&pair[0], &pair[1]).unwrap();
            near(after[0], 0.0, 1e-12);
            near(after[2], before[2], 1e-12);
            if pair[0].level >= Level::T12 {
                near(after[1], before[1], 1e-12);
            }
        }
    }

    #[test]
    fn retained_wedges_do_not_get_scaled_as_if_they_were_discs() {
        let minimum = scalar(
            Table::LevelRuleThresholds,
            "normal-kyphosis-min",
            "sagittal-modifier",
        )
        .unwrap()
        .0;
        let mut fit = fit(minimum / 2.0);
        for v in &mut fit.vertebrae {
            v.pose.wedge_side = 0.01;
            v.pose.wedge_front = 0.006;
        }
        let measured =
            profile_kyphosis(&fit.vertebrae.iter().map(|v| v.pose).collect::<Vec<_>>()).unwrap();
        let levels: Vec<_> = fit.vertebrae.iter().map(|v| v.level).collect();
        let target = target_spine(&fit, &levels, &reading(measured)).unwrap();
        assert!((profile_kyphosis(&target.poses).unwrap() - measured.max(minimum)).abs() < 1e-8);
        for (before, after) in fit.vertebrae.iter().zip(&target.poses) {
            assert_eq!(before.pose.wedge_side, after.wedge_side);
            assert_eq!(before.pose.wedge_front, after.wedge_front);
        }
    }

    #[test]
    fn absent_side_geometry_stays_typical_and_a_measured_unscalable_target_is_refused() {
        let fit = fit(0.0);
        let levels: Vec<_> = fit.vertebrae.iter().map(|v| v.level).collect();
        assert!(target_spine(&fit, &levels, &reading(0.0)).is_err());
        let mut measured = reading(0.0);
        measured.kyphosis_t5_t12_deg = Err(CannotTellCode::NoSideFilm);
        let target = target_spine(&fit, &levels, &measured).unwrap();
        assert_eq!(target.target_kyphosis_status, ValueStatus::Typical);
        near(target.target_kyphosis_deg, 0.0, 1e-12);
    }

    #[test]
    fn coronal_reference_metadata_never_repositions_the_target() {
        let minimum = scalar(
            Table::LevelRuleThresholds,
            "normal-kyphosis-min",
            "sagittal-modifier",
        )
        .unwrap()
        .0;
        let fit = fit(minimum);
        let levels: Vec<_> = fit.vertebrae.iter().map(|v| v.level).collect();
        let mut reading = reading(minimum);
        let original = target_spine(&fit, &levels, &reading).unwrap();
        reading.csvl_y_mm = Err(CannotTellCode::NoSacrum);
        let target = target_spine(&fit, &levels, &reading).unwrap();
        assert_eq!(target.coronal_line, CoronalLine::L5Vertical);
        assert_eq!(target.poses, original.poses);
        let mut without_l5 = fit;
        without_l5.vertebrae.pop();
        assert!(target_spine(&without_l5, &levels[..levels.len() - 1], &reading).is_ok());
    }

    #[test]
    fn a_straight_spline_has_its_independent_length_and_exact_head_ties() {
        let heads = [
            Vector3::new(-7.0, 4.0, 10.0),
            Vector3::new(-7.0, 4.0, 3.0),
            Vector3::new(-7.0, 4.0, -8.0),
        ];
        let points = clamped_spline(&heads).unwrap();
        assert!(
            (points.windows(2).map(|p| distance(p[0], p[1])).sum::<f64>() - 18.0).abs() < 1e-10
        );
        for head in heads {
            assert!(points.contains(&head.into()));
        }
        assert!(
            points
                .windows(2)
                .all(|p| distance(p[0], p[1]) <= SAMPLE_STEP_MM + 1e-10)
        );
        assert!(
            points
                .iter()
                .all(|p| (p[0] + 7.0).abs() < 1e-12 && (p[1] - 4.0).abs() < 1e-12)
        );
    }

    #[test]
    fn the_first_two_rods_carry_only_applied_sources_and_neither_is_overbent() {
        let minimum = scalar(
            Table::LevelRuleThresholds,
            "normal-kyphosis-min",
            "sagittal-modifier",
        )
        .unwrap()
        .0;
        let fit = fit(minimum + 5.0);
        let reading = reading(minimum + 5.0);
        let levels: Vec<_> = fit.vertebrae.iter().map(|v| v.level).collect();
        let choice = default_choice().unwrap();
        let anchors = anchors(&fit);
        let rods = propose(&fit, &levels, &reading, &anchors, &choice).unwrap();
        let extra = scalar(Table::RodChoices, "overcontour", "concave").unwrap();
        let offset = scalar(Table::RodChoices, "screw-cantilever-length", "lumbar").unwrap();
        assert_eq!(rods.len(), 2);
        assert_eq!(rods[0].side, BodySide::Left);
        assert_eq!(rods[0].overbend_deg, 0.0);
        assert_eq!(rods[1].overbend_deg, 0.0);
        for rod in rods {
            assert!(!rod.sources.contains(&extra.1));
            assert_eq!(rod.head_offset_mm, offset.0);
            assert!(rod.sources.contains(&offset.1));
            assert!(rod.sources.iter().all(|key| tables().source(key).is_some()));
            assert!(
                rod.points_mm
                    .windows(2)
                    .all(|p| distance(p[0], p[1]) <= SAMPLE_STEP_MM + 1e-8)
            );
            assert!(rod.length_mm > 0.0);
        }
        let mut missing = anchors;
        missing[0].entry_status = ValueStatus::Missing;
        assert!(propose(&fit, &levels, &reading, &missing, &choice).is_err());
        let unsupported = RodChoice {
            material: RodMaterial::StainlessSteel,
            diameter_mm: choice.diameter_mm,
            sources: Vec::new(),
        };
        assert!(propose(&fit, &levels, &reading, &missing, &unsupported).is_err());
    }

    #[test]
    fn a_partial_construct_scales_only_its_internal_thoracic_joints() {
        let fit = fit(30.0);
        let profile = target_profile(&fit, 2.0, 2, 5).unwrap();
        for i in 0..fit.vertebrae.len() - 1 {
            let before =
                relative_angles(&fit.vertebrae[i].pose, &fit.vertebrae[i + 1].pose).unwrap()[1];
            let after = relative_angles(&profile[i], &profile[i + 1]).unwrap()[1];
            let expected = if (2..5).contains(&i) {
                2.0 * before
            } else {
                before
            };
            assert!(
                (after - expected).abs() < 1e-8,
                "joint {i}: {after} against {expected}"
            );
        }
    }

    fn near(a: f64, b: f64, tolerance: f64) {
        assert!((a - b).abs() <= tolerance, "{a} against {b}");
    }

    fn circular_heads(chord: f64, turn: f64) -> Vec<[f64; 3]> {
        let radius = chord / (2.0 * libm::sin(turn * 0.5));
        (0..=4)
            .map(|i| {
                let angle = turn * (i as f64 / 4.0 - 0.5);
                [
                    radius * (libm::cos(turn * 0.5) - libm::cos(angle)),
                    0.0,
                    chord * 0.5 - radius * libm::sin(angle),
                ]
            })
            .collect()
    }

    #[test]
    fn a_straight_thoracic_target_gains_the_closed_form_circular_bow() {
        // Before running: c=200, theta=20deg gives h=100*tan(5deg),
        // about8.75mm posterior, and L=200*(theta/2)/sin(theta/2).
        let chord = 200.0;
        let theta = 20.0_f64.to_radians();
        let heads = [[0.0, 0.0, chord], [0.0, 0.0, 0.0]];
        let rod = contour(&heads, &[Level::T1, Level::T12], theta.to_degrees()).unwrap();
        let sag = -rod
            .points_mm
            .iter()
            .map(|p| p[0])
            .fold(f64::INFINITY, f64::min);
        near(sag, chord * 0.5 * libm::tan(theta * 0.25), 1e-9);
        near(
            rod.length_mm,
            chord * (theta * 0.5) / libm::sin(theta * 0.5),
            1e-9,
        );
        assert_eq!(rod.points_mm[0], heads[0]);
        assert_eq!(*rod.points_mm.last().unwrap(), heads[1]);
        near(
            rod.sagittal_tangent_rad.last().unwrap() - rod.sagittal_tangent_rad[0],
            theta,
            1e-9,
        );
        assert!(
            rod.points_mm[1..rod.points_mm.len() - 1]
                .iter()
                .all(|p| p[0] < 0.0)
        );
        assert!(
            rod.points_mm
                .windows(2)
                .all(|p| distance(p[0], p[1]) <= SAMPLE_STEP_MM + 1e-9)
        );
    }

    #[test]
    fn a_circular_target_gains_theta_of_signed_kyphosis_through_the_same_end_heads() {
        let chord = 200.0;
        let phi = 30.0_f64.to_radians();
        let theta = 20.0_f64.to_radians();
        let psi = phi + theta;
        let levels = [Level::T1, Level::T4, Level::T7, Level::T10, Level::T12];
        let heads = circular_heads(chord, phi);
        let rod = contour(&heads, &levels, theta.to_degrees()).unwrap();
        let radius = chord / (2.0 * libm::sin(psi * 0.5));
        let centre_x = radius * libm::cos(psi * 0.5);
        for p in &rod.points_mm {
            near(
                libm::hypot(p[0] - centre_x, p[2] - chord * 0.5),
                radius,
                1e-8,
            );
        }
        near(
            rod.length_mm,
            chord * (psi * 0.5) / libm::sin(psi * 0.5),
            1e-9,
        );
        near(
            rod.sagittal_tangent_rad.last().unwrap() - rod.sagittal_tangent_rad[0],
            psi,
            1e-9,
        );
        assert_eq!(rod.points_mm[0], heads[0]);
        assert_eq!(*rod.points_mm.last().unwrap(), *heads.last().unwrap());
        let target = contour(&heads, &levels, 0.0).unwrap();
        for head in &heads {
            assert!(target.points_mm.iter().any(|p| distance(*p, *head) < 1e-9));
        }
        near(
            target.length_mm,
            chord * (phi * 0.5) / libm::sin(phi * 0.5),
            1e-9,
        );
    }

    #[test]
    fn a_mixed_rod_rigidly_carries_the_lumbar_curve_without_changing_its_curvature() {
        let heads = [
            [0.0, 0.0, 300.0],
            [-20.0, 0.5, 230.0],
            [-15.0, 1.0, 160.0],
            [0.0, 0.0, 100.0],
            [10.0, -0.5, 50.0],
            [10.0, 0.0, 0.0],
        ];
        let levels = [
            Level::T1,
            Level::T6,
            Level::T10,
            Level::T12,
            Level::L2,
            Level::L5,
        ];
        assert_lumbar_shape_is_rigid(&heads, &levels);
        // Without a T12 head, labelled-level interpolation still inserts
        // the boundary and keeps the curve on its lumbar side rigid.
        let sparse_heads = [heads[0], heads[2], heads[4], heads[5]];
        let sparse_levels = [Level::T1, Level::T10, Level::L2, Level::L5];
        assert_lumbar_shape_is_rigid(&sparse_heads, &sparse_levels);
    }

    fn assert_lumbar_shape_is_rigid(heads: &[[f64; 3]], levels: &[Level]) {
        let target = contour(heads, levels, 0.0).unwrap();
        let rod = contour(heads, levels, 20.0).unwrap();
        let a = target.thoracic_end_index.unwrap();
        let b = rod.thoracic_end_index.unwrap();
        let target_tail = &target.points_mm[a..];
        let rod_tail = &rod.points_mm[b..];
        assert_eq!(target_tail.len(), rod_tail.len());
        for i in 0..target_tail.len() {
            near(
                distance(target_tail[0], target_tail[i]),
                distance(rod_tail[0], rod_tail[i]),
                1e-8,
            );
            near(
                distance(*target_tail.last().unwrap(), target_tail[i]),
                distance(*rod_tail.last().unwrap(), rod_tail[i]),
                1e-8,
            );
            near(
                rod.sagittal_tangent_rad[b + i] - target.sagittal_tangent_rad[a + i],
                rod.sagittal_tangent_rad[b] - target.sagittal_tangent_rad[a],
                1e-9,
            );
        }
        let target_turn = target.sagittal_tangent_rad[a] - target.sagittal_tangent_rad[0];
        let actual_turn = rod.sagittal_tangent_rad[b] - rod.sagittal_tangent_rad[0];
        near(actual_turn - target_turn, 20.0_f64.to_radians(), 1e-6);
        assert_eq!(rod.points_mm[0], heads[0]);
        assert_eq!(*rod.points_mm.last().unwrap(), *heads.last().unwrap());
    }

    #[test]
    fn lumbar_only_and_zero_thoracic_extent_do_not_gain_overbend() {
        let heads = [[0.0, 0.0, 100.0], [-10.0, 1.0, 50.0], [0.0, 0.0, 0.0]];
        for levels in [
            [Level::L1, Level::L3, Level::L5],
            [Level::T12, Level::L3, Level::L5],
        ] {
            let target = contour(&heads, &levels, 0.0).unwrap();
            let rod = contour(&heads, &levels, 20.0).unwrap();
            assert_eq!(rod.overbend_deg, 0.0);
            assert_eq!(rod.points_mm, target.points_mm);
            near(rod.length_mm, target.length_mm, 1e-9);
        }
    }
}
