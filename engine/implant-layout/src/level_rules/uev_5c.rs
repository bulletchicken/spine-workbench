//! In type 5C, posterior fusion permits the UEV or one above it. The
//! primary vote is the UEV; the other permitted end remains explicit.

use super::selective::curve;
use super::support::{input, output, shift, threshold};
use super::{RuleContext, RuleReading};
use crate::LayoutError;
use crate::curves::Reading;
use shapes::implant_layout::{
    CannotTellCode, CurveType, LevelRule, LumbarModifier, Measurement, RuleEnd,
};
use shapes::levels::Level;
use shapes::made_up::CurveRegion;

pub(super) fn applicable(ctx: &RuleContext<'_>) -> Reading<()> {
    if ctx.reading.classification.curve_type? != CurveType::ThoracolumbarLumbar
        || ctx.reading.classification.lumbar_modifier? != LumbarModifier::ClearOfApex
    {
        return Err(CannotTellCode::NotThisCurveType);
    }
    Ok(())
}

pub fn evaluate(ctx: &RuleContext<'_>) -> Result<RuleReading, LayoutError> {
    let (_, source) = threshold("uev-5c", "uiv-above-uev")?;
    let selected = applicable(ctx).and_then(|()| curve(ctx, CurveRegion::ThoracolumbarLumbar));
    let level = selected.map(|c| ctx.reading.vertebrae[c.upper_index].level);
    let inputs = selected.map_or_else(
        |_| Vec::new(),
        |c| {
            vec![input(
                ctx,
                Measurement::CoronalTilt,
                Some(ctx.reading.vertebrae[c.upper_index].level),
                c.region,
                Ok(ctx.reading.vertebrae[c.upper_index].upper_tilt_deg),
                None,
            )]
        },
    );
    Ok(output(
        LevelRule::Uev5C,
        RuleEnd::Upper,
        level,
        inputs,
        vec![source],
    ))
}

/// The other published posterior option. No second vote is added for the
/// same rule; pair generation can include this explicitly permitted end.
pub fn posterior_alternative(ctx: &RuleContext<'_>) -> Result<Reading<Level>, LayoutError> {
    let (offset, _) = threshold("uev-5c", "uiv-above-uev")?;
    let level = applicable(ctx)
        .and_then(|()| curve(ctx, CurveRegion::ThoracolumbarLumbar))
        .and_then(|c| {
            shift(
                ctx,
                ctx.reading.vertebrae[c.upper_index].level,
                -(offset as i32),
            )
        });
    Ok(level)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level_rules::test_spine::Fixture;

    #[test]
    fn a_type_five_c_curve_keeps_both_published_posterior_upper_ends() {
        // A made-up TL/L curve starts at T12. The review's posterior
        // options are T12 (UEV) and the preceding fitted body, T11.
        let mut f = Fixture::new();
        let mut c = f.reading.curves[0].clone();
        c.upper_index = 11;
        c.lower_index = 15;
        c.apex_index = 13;
        c.region = Some(CurveRegion::ThoracolumbarLumbar);
        f.reading.curves = vec![c];
        f.reading.major = Some(0);
        f.reading.classification.curve_type = Ok(CurveType::ThoracolumbarLumbar);
        f.reading.classification.lumbar_modifier = Ok(LumbarModifier::ClearOfApex);
        assert_eq!(evaluate(&f.context()).unwrap().level, Ok(Level::T12));
        assert_eq!(posterior_alternative(&f.context()).unwrap(), Ok(Level::T11));
        f.reading.classification.lumbar_modifier = Ok(LumbarModifier::TouchesApex);
        assert_eq!(
            evaluate(&f.context()).unwrap().level,
            Err(CannotTellCode::NotThisCurveType)
        );
    }
}
