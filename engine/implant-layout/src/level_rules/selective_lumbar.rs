//! Selective lumbar fusion's five geometric criteria. This project's
//! correction reads the thoracic bend against the table's printed TL/L
//! bending threshold; the per-version criterion marks that inference.

use super::RuleContext;
use super::selective::{self, SelectiveVersion, criterion, curve, ratio, rotation, translation};
use crate::LayoutError;
use shapes::implant_layout::{LevelRule, Measurement};
use shapes::made_up::CurveRegion;

pub fn evaluate(ctx: &RuleContext<'_>) -> Result<SelectiveVersion, LayoutError> {
    let thoracic = curve(ctx, CurveRegion::MainThoracic);
    let lumbar = curve(ctx, CurveRegion::ThoracolumbarLumbar);
    let region = Some(CurveRegion::MainThoracic);
    let numerator_region = Some(CurveRegion::ThoracolumbarLumbar);
    let group = "selective-lumbar";
    let mut reading = selective::finish(
        LevelRule::SelectiveLumbar,
        vec![
            criterion(
                group,
                "cobb-ratio-min",
                Measurement::CobbRatio,
                numerator_region,
                ratio(lumbar.map(|c| c.cobb_deg), thoracic.map(|c| c.cobb_deg)),
                false,
                false,
            )?,
            criterion(
                group,
                "translation-ratio-min",
                Measurement::ApicalTranslationRatio,
                numerator_region,
                ratio(translation(ctx, lumbar), translation(ctx, thoracic)),
                false,
                false,
            )?,
            criterion(
                group,
                "rotation-ratio-min",
                Measurement::ApicalRotationRatio,
                numerator_region,
                ratio(rotation(ctx, lumbar), rotation(ctx, thoracic)),
                false,
                true,
            )?,
            criterion(
                group,
                "tll-bending-cobb-max",
                Measurement::BendingCobb,
                region,
                thoracic.and_then(|c| c.bending_cobb_deg),
                true,
                true,
            )?,
            criterion(
                group,
                "thoracic-cobb-max",
                Measurement::Cobb,
                region,
                thoracic.map(|c| c.cobb_deg),
                true,
                false,
            )?,
        ],
    );
    if let Err(reason) = thoracic.and(lumbar) {
        reading.all_hold = Err(reason);
    }
    Ok(reading)
}
