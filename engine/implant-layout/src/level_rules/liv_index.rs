//! The published LIV-index check, counted over versions of a pair.

use super::RuleContext;
use super::stable_minus_one::stable;
use super::suk_neutral::neutral;
use super::support::{distance, threshold};
use crate::LayoutError;
use crate::curves::Reading;
use shapes::implant_layout::{CannotTellCode, LevelRule, PairCheck, Readable};
use shapes::levels::Level;

/// (LIV-NV)+(LIV-SV), counted caudally in anatomical levels. This is a
/// check on the supplied lower end, not another proposal for that end.
pub fn value(ctx: &RuleContext<'_>, lower: Level) -> Reading<f64> {
    ctx.reading.csvl_y_mm?;
    let liv = ctx
        .reading
        .vertebrae
        .iter()
        .position(|v| v.level == lower)
        .ok_or(CannotTellCode::NoLevelMeetsTheRule)?;
    let nv = neutral(ctx)?;
    let sv = stable(ctx)?;
    Ok(f64::from(
        distance(
            ctx,
            ctx.reading.vertebrae[nv].level,
            ctx.reading.vertebrae[liv].level,
        )? + distance(
            ctx,
            ctx.reading.vertebrae[sv].level,
            ctx.reading.vertebrae[liv].level,
        )?,
    ))
}

/// One version contributes zero or one flag. Aggregate these counts over
/// the same versions used for the level rules; an unreadable check never
/// contributes a flag.
pub fn check(ctx: &RuleContext<'_>, lower: Level) -> Result<PairCheck, LayoutError> {
    let (maximum, source) = threshold("liv-index", "liv-index-max")?;
    let flagged = match value(ctx, lower) {
        Ok(index) => Readable::Share {
            versions: u32::from(index < maximum),
        },
        Err(reason) => Readable::NotReadable { reason },
    };
    Ok(PairCheck {
        rule: LevelRule::LivIndex,
        flagged,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level_rules::test_spine::Fixture;

    #[test]
    fn the_index_is_negative_only_above_the_landmarks_midpoint() {
        let mut fixture = Fixture::new();
        let mut curve = fixture.reading.curves[fixture.reading.major.unwrap()].clone();
        curve.apex_index = 5;
        curve.lower_index = 10;
        fixture.reading.curves = vec![curve];
        fixture.reading.major = Some(0);
        for (index, vertebra) in fixture.reading.vertebrae.iter_mut().enumerate() {
            vertebra.twist_deg = Ok(if index == 14 { 0.0 } else { 5.0 });
            vertebra.centre_line_offset_mm = Ok(if index == 12 { 0.0 } else { 100.0 });
            vertebra.centre_mm[1] = if index == 12 { 0.0 } else { 100.0 };
            fixture.input.fit.vertebrae[index].pose.centre[1] = vertebra.centre_mm[1];
            fixture.input.fit.vertebrae[index].pose.rotation = [0.0; 3];
        }
        // NV=L3(index14), SV=L1(index12). At LIV L1,L2,L3 the hand
        // sums are (-2)+0=-2, (-1)+1=0, and 0+2=2. The published
        // strict-negative threshold flags only the first.
        for (index, expected, flags) in [(12, -2.0, 1), (13, 0.0, 0), (14, 2.0, 0)] {
            let level = fixture.input.fit.vertebrae[index].level;
            assert_eq!(value(&fixture.context(), level), Ok(expected));
            assert_eq!(
                check(&fixture.context(), level).unwrap().flagged,
                Readable::Share { versions: flags }
            );
        }
    }
}
