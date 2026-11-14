//! The geometric L3/L4 component is readable, but the complete published
//! rule also needs a Nash–Moe grade. No degree-to-grade mapping is supplied,
//! so the full rule never casts a level vote.

use super::selective::curve;
use super::support::{cell, input, output, threshold};
use super::{FusionRegion, RuleContext, RuleReading};
use crate::LayoutError;
use crate::curves::Reading;
use crate::evidence::{PublishedValue, Table};
use shapes::implant_layout::{CannotTellCode, LevelRule, Measurement, RuleEnd};
use shapes::levels::Level;
use shapes::made_up::CurveRegion;
use shapes::spine_readings::BodySide;

pub fn evaluate(ctx: &RuleContext<'_>) -> Result<RuleReading, LayoutError> {
    let mut result = geometry(ctx)?;
    if result.level.is_ok() {
        result.level = Err(CannotTellCode::NoRotationGrade);
    }
    let (grade, source) = threshold("lenke-l3", "rotation-max")?;
    result.inputs.push(input(
        ctx,
        Measurement::ApicalRotation,
        Some(Level::L3),
        Some(CurveRegion::ThoracolumbarLumbar),
        Err(CannotTellCode::NoRotationGrade),
        Some(grade),
    ));
    result.sources.push(source);
    let (grade, source) = threshold("lenke-l4", "rotation-min")?;
    result.inputs.push(input(
        ctx,
        Measurement::ApicalRotation,
        Some(Level::L4),
        Some(CurveRegion::ThoracolumbarLumbar),
        Err(CannotTellCode::NoRotationGrade),
        Some(grade),
    ));
    result.sources.push(source);
    result.sources.sort();
    result.sources.dedup();
    Ok(result)
}

/// The apex and convex-disc component alone. This is not a full rule
/// vote: callers must use [`evaluate`] when assembling rule counts.
pub fn geometry(ctx: &RuleContext<'_>) -> Result<RuleReading, LayoutError> {
    let apex_cell = cell(Table::LevelRuleThresholds, "lenke-l3", "apex-at-or-above")?;
    let PublishedValue::Disc { upper, lower } = apex_cell.value else {
        return Err(LayoutError::Refused(
            "non-disc-evidence:lenke-l3:apex-at-or-above".into(),
        ));
    };
    let (neutral, source) = threshold("lev-plus-one-5c", "bending-disc-angle")?;
    let mut inputs = Vec::new();
    let result = fused_lumbar(ctx).and_then(|()| {
        let c = curve(ctx, CurveRegion::ThoracolumbarLumbar)?;
        let apex = ctx.reading.vertebrae[c.apex_index].level;
        let upper_index = ctx
            .reading
            .vertebrae
            .iter()
            .position(|v| v.level == upper)
            .ok_or(CannotTellCode::NoLevelMeetsTheRule)?;
        let lower_index = ctx
            .reading
            .vertebrae
            .iter()
            .position(|v| v.level == lower)
            .ok_or(CannotTellCode::NoLevelMeetsTheRule)?;
        let disc_index = (upper_index + lower_index) as f64 * 0.5;
        let apex_above = disc_index - c.apex_index as f64;
        inputs.push(input(
            ctx,
            Measurement::ApexAboveDisc,
            Some(apex),
            c.region,
            Ok(apex_above),
            Some(0.0),
        ));
        let l3_index = ctx
            .reading
            .vertebrae
            .iter()
            .position(|v| v.level == Level::L3)
            .ok_or(CannotTellCode::NoLevelMeetsTheRule)?;
        if ctx.reading.vertebrae.get(l3_index + 1).map(|v| v.level) != Some(Level::L4) {
            return Err(CannotTellCode::NoLevelMeetsTheRule);
        }
        let l3 = &ctx.reading.vertebrae[l3_index];
        let disc = l3.disc_below_deg;
        inputs.push(input(
            ctx,
            Measurement::DiscAngle,
            Some(Level::L3),
            c.region,
            disc,
            Some(neutral),
        ));
        let convex_opening = disc?
            * if c.convex == BodySide::Left {
                1.0
            } else {
                -1.0
            };
        let level = if apex_above >= 0.0 && convex_opening <= neutral {
            Level::L3
        } else {
            Level::L4
        };
        ctx.reading
            .vertebrae
            .iter()
            .any(|v| v.level == level)
            .then_some(level)
            .ok_or(CannotTellCode::NoLevelMeetsTheRule)
    });
    Ok(output(
        LevelRule::LenkeL3L4,
        RuleEnd::Lower,
        result,
        inputs,
        vec![apex_cell.source.clone(), source],
    ))
}

fn fused_lumbar(ctx: &RuleContext<'_>) -> Reading<()> {
    if ctx.region == FusionRegion::Thoracic {
        return Err(CannotTellCode::NotThisCurveType);
    }
    let c = curve(ctx, CurveRegion::ThoracolumbarLumbar)?;
    if ctx.region == FusionRegion::Structural && !c.structural? {
        return Err(CannotTellCode::NotThisCurveType);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level_rules::test_spine::Fixture;

    #[test]
    fn the_apex_and_convex_disc_components_follow_the_words_but_grade_stays_missing() {
        // fusion-levels §2d: an L1 apex lies above the L1/2 disc; L2 lies
        // below it. A left-convex disc with signed angle -1 is closed,
        // zero is neutral and +1 is open. These are explicit made-up
        // readings: no Nash–Moe number is inferred from a twist angle.
        let mut fixture = Fixture::new();
        let upper = fixture
            .reading
            .vertebrae
            .iter()
            .position(|v| v.level == Level::T12)
            .unwrap();
        let lower = fixture
            .reading
            .vertebrae
            .iter()
            .position(|v| v.level == Level::L4)
            .unwrap();
        let l1 = fixture
            .reading
            .vertebrae
            .iter()
            .position(|v| v.level == Level::L1)
            .unwrap();
        let l2 = fixture
            .reading
            .vertebrae
            .iter()
            .position(|v| v.level == Level::L2)
            .unwrap();
        let l3 = fixture
            .reading
            .vertebrae
            .iter()
            .position(|v| v.level == Level::L3)
            .unwrap();
        let mut c = fixture.reading.curves[0].clone();
        c.upper_index = upper;
        c.lower_index = lower;
        c.apex_index = l1;
        c.region = Some(CurveRegion::ThoracolumbarLumbar);
        c.convex = BodySide::Left;
        c.structural = Ok(true);
        fixture.reading.curves = vec![c];
        fixture.reading.major = Some(0);
        let (neutral, _) = threshold("lev-plus-one-5c", "bending-disc-angle").unwrap();
        for (apex, disc, expected) in [
            (l1, neutral - 1.0, Level::L3),
            (l1, neutral, Level::L3),
            (l1, neutral + 1.0, Level::L4),
            (l2, neutral - 1.0, Level::L4),
        ] {
            fixture.reading.curves[0].apex_index = apex;
            fixture.reading.vertebrae[l3].disc_below_deg = Ok(disc);
            assert_eq!(geometry(&fixture.context()).unwrap().level, Ok(expected));
            assert_eq!(
                evaluate(&fixture.context()).unwrap().level,
                Err(CannotTellCode::NoRotationGrade)
            );
        }
        fixture.reading.curves[0].convex = BodySide::Right;
        fixture.reading.curves[0].apex_index = l1;
        fixture.reading.vertebrae[l3].disc_below_deg = Ok(neutral + 1.0);
        assert_eq!(geometry(&fixture.context()).unwrap().level, Ok(Level::L3));
    }
}
