//! Selective thoracic fusion's six geometric criteria. Degree-based AVR
//! ratios are a project inference, without a Nash–Moe grade conversion.

use super::RuleContext;
use super::selective::{self, SelectiveVersion, criterion, curve, ratio, rotation, translation};
use crate::LayoutError;
use shapes::implant_layout::{LevelRule, Measurement};
use shapes::made_up::CurveRegion;

pub fn evaluate(ctx: &RuleContext<'_>) -> Result<SelectiveVersion, LayoutError> {
    let thoracic = curve(ctx, CurveRegion::MainThoracic);
    let lumbar = curve(ctx, CurveRegion::ThoracolumbarLumbar);
    let region = Some(CurveRegion::ThoracolumbarLumbar);
    let numerator_region = Some(CurveRegion::MainThoracic);
    let group = "selective-thoracic";
    let mut reading = selective::finish(
        LevelRule::SelectiveThoracic,
        vec![
            criterion(
                group,
                "cobb-ratio-min",
                Measurement::CobbRatio,
                numerator_region,
                ratio(thoracic.map(|c| c.cobb_deg), lumbar.map(|c| c.cobb_deg)),
                false,
                false,
            )?,
            criterion(
                group,
                "translation-ratio-min",
                Measurement::ApicalTranslationRatio,
                numerator_region,
                ratio(translation(ctx, thoracic), translation(ctx, lumbar)),
                false,
                false,
            )?,
            criterion(
                group,
                "rotation-ratio-min",
                Measurement::ApicalRotationRatio,
                numerator_region,
                ratio(rotation(ctx, thoracic), rotation(ctx, lumbar)),
                false,
                true,
            )?,
            criterion(
                group,
                "bending-cobb-max",
                Measurement::BendingCobb,
                region,
                lumbar.and_then(|c| c.bending_cobb_deg),
                true,
                false,
            )?,
            criterion(
                group,
                "kyphosis-t10-l2-max",
                Measurement::KyphosisT10L2,
                None,
                ctx.reading.kyphosis_t10_l2_deg,
                true,
                false,
            )?,
            criterion(
                group,
                "cobb-max",
                Measurement::Cobb,
                region,
                lumbar.map(|c| c.cobb_deg),
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
