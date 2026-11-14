//! The lower end vertebra plus the published caudal offset.

use super::support::{input, lower_curve, output, shift, threshold};
use super::{RuleContext, RuleReading};
use crate::LayoutError;
use shapes::implant_layout::{LevelRule, Measurement, RuleEnd};

pub fn evaluate(ctx: &RuleContext<'_>) -> Result<RuleReading, LayoutError> {
    let (offset, source) = threshold("end-plus-one", "liv-below-end")?;
    let mut inputs = Vec::new();
    let level = lower_curve(ctx).and_then(|curve| {
        let end = &ctx.reading.vertebrae[curve.lower_index];
        inputs.push(input(
            ctx,
            Measurement::CoronalTilt,
            Some(end.level),
            curve.region,
            Ok(end.upper_tilt_deg),
            None,
        ));
        shift(ctx, end.level, offset as i32)
    });
    Ok(output(
        LevelRule::EndPlusOne,
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

    #[test]
    fn each_lower_end_moves_to_the_published_next_level() {
        let mut fixture = Fixture::new();
        let curve = fixture.reading.curves[fixture.reading.major.unwrap()].clone();
        fixture.reading.curves = vec![curve];
        fixture.reading.major = Some(0);
        let (offset, _) = threshold("end-plus-one", "liv-below-end").unwrap();
        // The note says EV+1. Move the made-up EV through T10,T11,T12;
        // counting down by the printed offset gives T11,T12,L1.
        for end in 9..=11 {
            fixture.reading.curves[0].lower_index = end;
            let expected = fixture.input.fit.vertebrae[end + offset as usize].level;
            assert_eq!(evaluate(&fixture.context()).unwrap().level, Ok(expected));
        }
    }

    #[test]
    fn an_end_at_t8_cannot_skip_the_absent_t9() {
        let mut f = Fixture::new();
        let mut curve = f.reading.curves[f.reading.major.unwrap()].clone();
        curve.lower_index = 7;
        curve.apex_index = 5;
        f.reading.curves = vec![curve];
        f.reading.major = Some(0);
        f.input.fit.vertebrae.remove(8);
        f.reading.vertebrae.remove(8);
        f.outlines.remove(8);
        // The definition's one below T8 is T9. T10 is fitted but is two
        // anatomical levels below; no missing body is silently crossed.
        assert_eq!(
            evaluate(&f.context()).unwrap().level,
            Err(shapes::implant_layout::CannotTellCode::NoLevelMeetsTheRule)
        );
    }
}
