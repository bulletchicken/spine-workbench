//! Suk's lower end from the neutral vertebra. A measured twist zero
//! crossing replaces an unavailable degree-to-Nash–Moe conversion.

use super::support::{distance, input, lower_curve, output, shift, template, threshold};
use super::{RuleContext, RuleReading};
use crate::LayoutError;
use crate::curves::Reading;
use shapes::implant_layout::{CannotTellCode, LevelRule, Measurement, RuleEnd};

/// Find the caudal-most measured zero or crossing below the curve apex.
/// Searching beyond the lower end is this project's interpretation:
/// otherwise the published NV >= EV+2 branch could never be evaluated.
/// A crossing belongs to its nearer-zero member; equal magnitudes take
/// the caudal member. Unknown caudal twist prevents locating the last one.
pub fn neutral(ctx: &RuleContext<'_>) -> Reading<usize> {
    let apex = lower_curve(ctx)?.apex_index;
    let vertebrae = &ctx.reading.vertebrae;
    let last = template(ctx)
        .levels()
        .last()
        .copied()
        .ok_or(CannotTellCode::NoLevelMeetsTheRule)?;
    if vertebrae.last().is_none_or(|v| v.level < last) {
        return Err(CannotTellCode::NoLevelMeetsTheRule);
    }
    for index in (apex..vertebrae.len()).rev() {
        let current = vertebrae[index].twist_deg?;
        if current == 0.0 {
            return Ok(index);
        }
        if index > apex {
            if distance(ctx, vertebrae[index - 1].level, vertebrae[index].level)? != 1 {
                return Err(CannotTellCode::NoLevelMeetsTheRule);
            }
            let above = vertebrae[index - 1].twist_deg?;
            if above == 0.0 || (above < 0.0) != (current < 0.0) {
                return Ok(if above.abs() < current.abs() {
                    index - 1
                } else {
                    index
                });
            }
        }
    }
    Err(CannotTellCode::NoLevelMeetsTheRule)
}

pub fn evaluate(ctx: &RuleContext<'_>) -> Result<RuleReading, LayoutError> {
    let (near, source) = threshold("suk-neutral", "nv-ev-max")?;
    let (far, far_source) = threshold("suk-neutral", "nv-ev-min")?;
    let (offset, offset_source) = threshold("suk-neutral", "liv-above-neutral")?;
    let mut inputs = Vec::new();
    let level = lower_curve(ctx).and_then(|curve| {
        for vertebra in &ctx.reading.vertebrae[curve.apex_index..] {
            inputs.push(input(
                ctx,
                Measurement::Twist,
                Some(vertebra.level),
                curve.region,
                vertebra.twist_deg,
                Some(0.0),
            ));
        }
        let nv = neutral(ctx)?;
        let difference = f64::from(distance(
            ctx,
            ctx.reading.vertebrae[curve.lower_index].level,
            ctx.reading.vertebrae[nv].level,
        )?);
        if difference <= near {
            Ok(ctx.reading.vertebrae[nv].level)
        } else if difference >= far {
            shift(ctx, ctx.reading.vertebrae[nv].level, -(offset as i32))
        } else {
            Err(CannotTellCode::NoLevelMeetsTheRule)
        }
    });
    Ok(output(
        LevelRule::SukNeutral,
        RuleEnd::Lower,
        level,
        inputs,
        vec![source, far_source, offset_source],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level_rules::test_spine::Fixture;

    #[test]
    fn the_two_suk_branches_meet_on_the_printed_level_thresholds() {
        let mut fixture = Fixture::new();
        let mut curve = fixture.reading.curves[fixture.reading.major.unwrap()].clone();
        curve.apex_index = 5;
        fixture.reading.curves = vec![curve];
        fixture.reading.major = Some(0);
        for vertebra in &mut fixture.reading.vertebrae {
            vertebra.twist_deg = Ok(5.0);
        }
        let nv = 14;
        fixture.reading.vertebrae[nv].twist_deg = Ok(0.0);
        let (near, _) = threshold("suk-neutral", "nv-ev-max").unwrap();
        let (far, _) = threshold("suk-neutral", "nv-ev-min").unwrap();
        let (offset, _) = threshold("suk-neutral", "liv-above-neutral").unwrap();
        // With NV fixed at L3, EV one level above gives NV; EV two
        // above gives NV-1, exactly the two rows of Suk's definition.
        fixture.reading.curves[0].lower_index = nv - near as usize;
        assert_eq!(
            evaluate(&fixture.context()).unwrap().level,
            Ok(fixture.input.fit.vertebrae[nv].level)
        );
        fixture.reading.curves[0].lower_index = nv - far as usize;
        assert_eq!(
            evaluate(&fixture.context()).unwrap().level,
            Ok(fixture.input.fit.vertebrae[nv - offset as usize].level)
        );
        fixture.reading.vertebrae[nv + 1].twist_deg = Err(CannotTellCode::TwistNotMeasured);
        assert_eq!(
            neutral(&fixture.context()),
            Err(CannotTellCode::TwistNotMeasured)
        );
    }

    #[test]
    fn a_sign_crossing_takes_the_nearer_zero_member_and_then_the_caudal_tie() {
        let mut fixture = Fixture::new();
        let mut curve = fixture.reading.curves[fixture.reading.major.unwrap()].clone();
        curve.apex_index = 5;
        fixture.reading.curves = vec![curve];
        fixture.reading.major = Some(0);
        for (i, vertebra) in fixture.reading.vertebrae.iter_mut().enumerate() {
            vertebra.twist_deg = Ok(if i < 14 { -3.0 } else { 2.0 });
        }
        // At L2=-3 and L3=+2, L3 is nearer zero. Moving L2 to -1
        // makes L2 nearer; equal magnitudes use the caudal tied level.
        assert_eq!(neutral(&fixture.context()), Ok(14));
        fixture.reading.vertebrae[13].twist_deg = Ok(-1.0);
        assert_eq!(neutral(&fixture.context()), Ok(13));
        fixture.reading.vertebrae[13].twist_deg = Ok(-2.0);
        assert_eq!(neutral(&fixture.context()), Ok(14));
    }

    #[test]
    fn a_twist_sign_change_across_a_missing_t9_is_not_a_measured_crossing() {
        let mut f = Fixture::new();
        let mut curve = f.reading.curves[f.reading.major.unwrap()].clone();
        curve.apex_index = 5;
        curve.lower_index = 7;
        f.reading.curves = vec![curve];
        f.reading.major = Some(0);
        for (i, vertebra) in f.reading.vertebrae.iter_mut().enumerate() {
            vertebra.twist_deg = Ok(if i <= 7 { -1.0 } else { 1.0 });
        }
        f.input.fit.vertebrae.remove(8);
        f.reading.vertebrae.remove(8);
        f.outlines.remove(8);
        // T8=-1 and T10=+1 do not tell the missing T9's twist, so they
        // cannot be interpreted as neighbouring endpoints of a crossing.
        assert_eq!(
            neutral(&f.context()),
            Err(CannotTellCode::NoLevelMeetsTheRule)
        );
    }
}
