//! Wang's first deviated vertebra, counted cranially from the sacrum.

use super::support::{distance, input, output, template, threshold};
use super::{RuleContext, RuleReading};
use crate::LayoutError;
use crate::curves::Reading;
use shapes::implant_layout::{CannotTellCode, LevelRule, Measurement, RuleEnd};

/// Compare the spine's lateral distance with the published millimetre
/// threshold in one place. A typical drawing has no physical scale.
pub fn deviation_exceeds_mm(ctx: &RuleContext<'_>, index: usize, minimum_mm: f64) -> Reading<bool> {
    if ctx.fit.lengths == shapes::spine_fit::FitLengths::TypicalSize {
        return Err(CannotTellCode::NoFilmScale);
    }
    ctx.reading.vertebrae[index]
        .centre_line_offset_mm
        .map(|offset| offset.abs() > minimum_mm)
}

pub fn evaluate(ctx: &RuleContext<'_>) -> Result<RuleReading, LayoutError> {
    let (minimum, source) = threshold("deviated-vertebra", "first-cephalad-deviation-min")?;
    let mut inputs = Vec::new();
    let level = ctx.reading.csvl_y_mm.and_then(|_| {
        let last = template(ctx)
            .levels()
            .last()
            .copied()
            .ok_or(CannotTellCode::NoLevelMeetsTheRule)?;
        if ctx.reading.vertebrae.last().is_none_or(|v| v.level < last) {
            return Err(CannotTellCode::NoLevelMeetsTheRule);
        }
        for (index, vertebra) in ctx.reading.vertebrae.iter().enumerate().rev() {
            if index + 1 < ctx.reading.vertebrae.len()
                && distance(ctx, vertebra.level, ctx.reading.vertebrae[index + 1].level)? != 1
            {
                return Err(CannotTellCode::NoLevelMeetsTheRule);
            }
            inputs.push(input(
                ctx,
                Measurement::CentreLineOffset,
                Some(vertebra.level),
                None,
                vertebra.centre_line_offset_mm,
                Some(minimum),
            ));
            if deviation_exceeds_mm(ctx, index, minimum)? {
                return Ok(vertebra.level);
            }
        }
        Err(CannotTellCode::NoLevelMeetsTheRule)
    });
    Ok(output(
        LevelRule::DeviatedVertebra,
        RuleEnd::Lower,
        level,
        inputs,
        vec![source],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level_rules::test_spine::Fixture;
    use shapes::levels::Level;

    #[test]
    fn the_cranial_scan_skips_values_at_or_below_the_published_minimum() {
        let mut fixture = Fixture::new();
        let (minimum, _) = threshold("deviated-vertebra", "first-cephalad-deviation-min").unwrap();
        for vertebra in &mut fixture.reading.vertebrae {
            vertebra.centre_line_offset_mm = Ok(minimum - 1.0);
            vertebra.centre_mm[1] = minimum - 1.0;
        }
        fixture.reading.vertebrae[10].centre_line_offset_mm = Ok(minimum + 1.0);
        fixture.reading.vertebrae[10].centre_mm[1] = minimum + 1.0;
        // Going up from the sacrum reaches L2 before T11. Strictly more
        // than the printed minimum picks L2; exactly at or below it
        // skips L2 and reaches the deliberately displaced T11.
        for (offset, expected) in [
            (minimum - 1.0, Level::T11),
            (minimum, Level::T11),
            (minimum + 1.0, Level::L2),
        ] {
            fixture.reading.vertebrae[13].centre_line_offset_mm = Ok(offset);
            fixture.reading.vertebrae[13].centre_mm[1] = offset;
            assert_eq!(evaluate(&fixture.context()).unwrap().level, Ok(expected));
        }
    }
}
