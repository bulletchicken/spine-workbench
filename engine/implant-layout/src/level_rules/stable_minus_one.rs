//! The stable vertebra, or one above it for lumbar modifiers A and B.

use super::support::{cell, distance, input, lower_curve, output, shift};
use super::{RuleContext, RuleReading};
use crate::LayoutError;
use crate::curves::Reading;
use crate::evidence::{PublishedValue, Table};
use forward_model::pedicles::pedicle_centres;
use forward_model::vertebra_params::VertebraParams;
use shapes::implant_layout::{CannotTellCode, LevelRule, LumbarModifier, Measurement, RuleEnd};

fn centre_line_between_pedicle_centres(ctx: &RuleContext<'_>, index: usize) -> Reading<bool> {
    let params = VertebraParams::from(&ctx.fit.vertebrae[index].pose);
    let centres = pedicle_centres(&params);
    let low = centres[0].y.min(centres[1].y) - params.centre.y;
    let high = centres[0].y.max(centres[1].y) - params.centre.y;
    ctx.reading.vertebrae[index]
        .centre_line_offset_mm
        .map(|offset| (low..=high).contains(&-offset))
}

/// The first fitted vertebra at or below the lower end whose two
/// projected pedicle centres bracket the CSVL, with boundaries included.
/// This is the stated project reading of "most closely bisected".
pub fn stable(ctx: &RuleContext<'_>) -> Reading<usize> {
    ctx.reading.csvl_y_mm?;
    let lower = lower_curve(ctx)?.lower_index;
    for index in lower..ctx.reading.vertebrae.len() {
        if index > lower
            && distance(
                ctx,
                ctx.reading.vertebrae[index - 1].level,
                ctx.reading.vertebrae[index].level,
            )? != 1
        {
            return Err(CannotTellCode::NoLevelMeetsTheRule);
        }
        if centre_line_between_pedicle_centres(ctx, index)? {
            return Ok(index);
        }
    }
    Err(CannotTellCode::NoLevelMeetsTheRule)
}

/// The modifier has already compared the centre line with this version's
/// finite table-derived pedicle outlines, requiring a physical scale.
fn centre_line_against_lumbar_pedicle_outlines(ctx: &RuleContext<'_>) -> Reading<LumbarModifier> {
    if ctx.fit.lengths == shapes::spine_fit::FitLengths::TypicalSize {
        return Err(CannotTellCode::NoFilmScale);
    }
    ctx.reading.classification.lumbar_modifier
}

pub fn evaluate(ctx: &RuleContext<'_>) -> Result<RuleReading, LayoutError> {
    let published = cell(Table::LevelRuleThresholds, "lenke-ab", "liv-above-stable")?;
    let PublishedValue::Range { lower: offset, .. } = published.value else {
        return Err(LayoutError::Refused(
            "non-range-evidence:lenke-ab:liv-above-stable".into(),
        ));
    };
    let mut inputs = Vec::new();
    if let Ok(apex) = crate::curves::lumbar::apex(ctx.reading, ctx.fit) {
        let vertebra = &ctx.reading.vertebrae[apex];
        let interval = super::last_touched::body_interval(ctx, apex);
        let outline = ctx
            .outlines
            .iter()
            .find(|outline| outline.level == vertebra.level);
        let low = outline.map_or(interval[0], |o| {
            interval[0].min(o.left_y_mm[0].min(o.right_y_mm[0]) - vertebra.centre_mm[1])
        });
        let high = outline.map_or(interval[1], |o| {
            interval[1].max(o.left_y_mm[1].max(o.right_y_mm[1]) - vertebra.centre_mm[1])
        });
        inputs.push(input(
            ctx,
            Measurement::CentreLineOffset,
            Some(vertebra.level),
            Some(shapes::made_up::CurveRegion::ThoracolumbarLumbar),
            vertebra.centre_line_offset_mm,
            vertebra
                .centre_line_offset_mm
                .ok()
                .map(|value| if value < 0.0 { high } else { -low }),
        ));
    }
    let level = ctx.reading.csvl_y_mm.and_then(|_| {
        let curve = lower_curve(ctx)?;
        for (index, vertebra) in ctx
            .reading
            .vertebrae
            .iter()
            .enumerate()
            .skip(curve.lower_index)
        {
            let params = VertebraParams::from(&ctx.fit.vertebrae[index].pose);
            let centres = pedicle_centres(&params);
            let low = centres[0].y.min(centres[1].y) - params.centre.y;
            let high = centres[0].y.max(centres[1].y) - params.centre.y;
            let offset = vertebra.centre_line_offset_mm;
            inputs.push(input(
                ctx,
                Measurement::CentreLineOffset,
                Some(vertebra.level),
                curve.region,
                offset,
                offset
                    .ok()
                    .map(|offset| if offset < 0.0 { high } else { -low }),
            ));
            if centre_line_between_pedicle_centres(ctx, index)? {
                break;
            }
        }
        let sv = stable(ctx)?;
        let modifier = centre_line_against_lumbar_pedicle_outlines(ctx)?;
        if modifier == LumbarModifier::ClearOfApex {
            Ok(ctx.reading.vertebrae[sv].level)
        } else {
            // The level rule takes SV-1 from the published range
            // SV-1 or SV-2. Its lower endpoint is read, never retyped.
            shift(ctx, ctx.reading.vertebrae[sv].level, -(offset as i32))
        }
    });
    Ok(output(
        LevelRule::StableMinusOne,
        RuleEnd::Lower,
        level,
        inputs,
        vec![published.source.clone()],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level_rules::test_spine::Fixture;
    use shapes::levels::Level;

    #[test]
    fn pedicle_bracketing_and_modifier_follow_the_rule_words() {
        let mut fixture = Fixture::new();
        let mut curve = fixture.reading.curves[fixture.reading.major.unwrap()].clone();
        curve.lower_index = 10;
        fixture.reading.curves = vec![curve];
        fixture.reading.major = Some(0);
        fixture.reading.classification.lumbar_modifier = Ok(LumbarModifier::BetweenPedicles);
        for index in 10..fixture.input.fit.vertebrae.len() {
            fixture.input.fit.vertebrae[index].pose.rotation = [0.0; 3];
            fixture.reading.vertebrae[index].centre_line_offset_mm = Ok(0.0);
        }
        let params = VertebraParams::from(&fixture.input.fit.vertebrae[10].pose);
        let edge = pedicle_centres(&params)[0].y - params.centre.y;
        // At T11, an upright centre-line offset no larger than the
        // pedicle-centre half-span is bisected. Just outside it, T12 is
        // SV instead. Modifier A gives SV-1; C gives SV itself.
        for (offset, expected_sv) in [(edge - 0.1, 10), (edge, 10), (edge + 0.1, 11)] {
            fixture.reading.vertebrae[10].centre_line_offset_mm = Ok(offset);
            fixture.reading.vertebrae[10].centre_mm[1] = offset;
            fixture.input.fit.vertebrae[10].pose.centre[1] = offset;
            assert_eq!(stable(&fixture.context()), Ok(expected_sv));
            assert_eq!(
                evaluate(&fixture.context()).unwrap().level,
                Ok(fixture.input.fit.vertebrae[expected_sv - 1].level)
            );
        }
        fixture.reading.classification.lumbar_modifier = Ok(LumbarModifier::ClearOfApex);
        assert_eq!(evaluate(&fixture.context()).unwrap().level, Ok(Level::T12));
    }
}
