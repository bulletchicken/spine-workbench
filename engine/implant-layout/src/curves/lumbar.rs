//! The lumbar apex and the CSVL against finite pedicle outlines. An apex
//! need not belong to a nonzero detected curve. Pedicle centres alone do
//! not locate the medial or lateral edges of a pedicle.

use super::{Reading, SpineReading};
use crate::LayoutError;
use forward_model::endplates::Endplates;
use forward_model::pedicles::pedicle_centres;
use forward_model::vertebra_params::VertebraParams;
use shapes::implant_layout::{CannotTellCode, LumbarModifier};
use shapes::levels::{Level, Region};
use shapes::made_up::CurveRegion;
use shapes::spine_fit::SpineFit;

/// The two finite pedicle silhouettes projected onto the patient y axis,
/// in millimetres, for one vertebra of this version. Each interval is
/// [lowest y, highest y], with positive extent, including its boundary.
/// Left and right name the patient's pedicles; projection may overlap or
/// reverse their order. Callers supply template geometry, never a guessed
/// width derived from the distance between centres. These intervals must
/// already reflect the version's pose and the template's shape.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PedicleOutlines {
    pub level: Level,
    pub left_y_mm: [f64; 2],
    pub right_y_mm: [f64; 2],
}

/// The lumbar apical index in the fit used for `reading`. A detected TL/L
/// curve retains its measured apex. Without one, this project's own
/// extension of the end-centre-chord definition uses the first and last
/// fitted lumbar vertebrae as regional ends. The greatest lateral
/// deviation from that chord is the apex; ties, including a straight
/// segment or a singleton, take the first level in top-to-bottom order.
/// No curve-size threshold or CSVL is needed to identify this index.
pub fn apex(reading: &SpineReading, fit: &SpineFit) -> Reading<usize> {
    if let Some(curve) = reading
        .curves
        .iter()
        .filter(|curve| curve.region == Some(CurveRegion::ThoracolumbarLumbar))
        .fold(None, |most: Option<&super::Curve>, curve| {
            if most.is_none_or(|previous| curve.cobb_deg > previous.cobb_deg) {
                Some(curve)
            } else {
                most
            }
        })
    {
        return fit
            .vertebrae
            .get(curve.apex_index)
            .map(|_| curve.apex_index)
            .ok_or(CannotTellCode::NoCurve);
    }
    let indices: Vec<_> = fit
        .vertebrae
        .iter()
        .enumerate()
        .filter_map(|(i, vertebra)| (vertebra.level.region() == Region::Lumbar).then_some(i))
        .collect();
    let first = *indices.first().ok_or(CannotTellCode::NoCurve)?;
    let last = *indices.last().expect("a lumbar vertebra exists");
    let a = fit.vertebrae[first].pose.centre;
    let b = fit.vertebrae[last].pose.centre;
    let mut index = first;
    let mut greatest = 0.0;
    for &i in &indices {
        let centre = fit.vertebrae[i].pose.centre;
        let fraction = if a[2] != b[2] {
            (centre[2] - a[2]) / (b[2] - a[2])
        } else if first != last {
            (i - first) as f64 / (last - first) as f64
        } else {
            0.0
        };
        let deviation = (centre[1] - (a[1] + fraction * (b[1] - a[1]))).abs();
        if deviation > greatest {
            index = i;
            greatest = deviation;
        }
    }
    Ok(index)
}

pub(super) fn validate(outlines: &[PedicleOutlines], fit: &SpineFit) -> Result<(), LayoutError> {
    for (index, outline) in outlines.iter().enumerate() {
        if [outline.left_y_mm, outline.right_y_mm]
            .iter()
            .any(|interval| {
                interval.iter().any(|value| !value.is_finite()) || interval[0] >= interval[1]
            })
            || outlines[..index]
                .iter()
                .any(|other| other.level == outline.level)
            || !fit
                .vertebrae
                .iter()
                .any(|vertebra| vertebra.level == outline.level)
        {
            return Err(LayoutError::InvalidValue {
                field: "pedicle-outlines".into(),
                reason: "finite-positive-intervals-at-distinct-fitted-levels-required".into(),
            });
        }
    }
    Ok(())
}

/// Missing outlines return `NoPedicleTable` when the modifier cannot be
/// resolved from the available geometry. A B reading can still
/// be established if a known pedicle centre is touched, or the body is
/// touched outside the interval between centres: neither can be A, and
/// neither requires inventing a pedicle edge. Other unresolved readings
/// stay unreadable, including lines outside a body that an unknown
/// pedicle outline might still touch.
pub(super) fn modifier(
    reading: &SpineReading,
    fit: &SpineFit,
    outlines: &[PedicleOutlines],
) -> Reading<LumbarModifier> {
    let csvl = reading.csvl_y_mm?;
    let vertebra = &fit.vertebrae[apex(reading, fit)?];
    let params = VertebraParams::from(&vertebra.pose);
    let plates = Endplates::of(&params);
    let mut body_low = f64::INFINITY;
    let mut body_high = f64::NEG_INFINITY;
    for plate in [plates.top, plates.bottom] {
        let width = plate.half_width * plate.width_axis.y;
        let depth = plate.half_depth * plate.depth_axis.y;
        let extent = libm::sqrt(width * width + depth * depth);
        body_low = body_low.min(plate.centre.y - extent);
        body_high = body_high.max(plate.centre.y + extent);
    }
    let touches_body = (body_low..=body_high).contains(&csvl);
    if let Some(outline) = outlines
        .iter()
        .find(|outline| outline.level == vertebra.level)
    {
        let [a, b] = if outline.left_y_mm[0] <= outline.right_y_mm[0] {
            [outline.left_y_mm, outline.right_y_mm]
        } else {
            [outline.right_y_mm, outline.left_y_mm]
        };
        if csvl > a[1] && csvl < b[0] {
            return Ok(LumbarModifier::BetweenPedicles);
        }
        return Ok(
            if touches_body || (a[0]..=a[1]).contains(&csvl) || (b[0]..=b[1]).contains(&csvl) {
                LumbarModifier::TouchesApex
            } else {
                LumbarModifier::ClearOfApex
            },
        );
    }
    let centres = pedicle_centres(&params);
    let low = centres[0].y.min(centres[1].y);
    let high = centres[0].y.max(centres[1].y);
    if csvl == low || csvl == high || (touches_body && (csvl < low || csvl > high)) {
        Ok(LumbarModifier::TouchesApex)
    } else {
        Err(CannotTellCode::NoPedicleTable)
    }
}
