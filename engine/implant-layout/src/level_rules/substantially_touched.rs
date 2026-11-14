//! Qin's LTV branch and the most proximal substantial pedicle contact.
//! A non-substantial LTV gives LTV+1; otherwise the most proximal STV
//! supplies the lower end, without an end-vertebra search bound.

use super::last_touched::{body_interval, body_touched};
use super::support::{distance, input, output, shift, template, threshold};
use super::{RuleContext, RuleReading};
use crate::LayoutError;
use crate::curves::Reading;
use shapes::implant_layout::{CannotTellCode, LevelRule, Measurement, RuleEnd, RuleInput};
use shapes::levels::Level;

/// Compare the centre line with finite projected pedicle outlines in one
/// place. Between the two outer edges it either crosses an outline or is
/// medial to it. The later fit-length gate belongs here because these
/// outlines incorporate published millimetre widths.
pub fn centre_line_meets_or_is_medial_to_pedicle(
    ctx: &RuleContext<'_>,
    index: usize,
) -> Reading<bool> {
    let vertebra = &ctx.reading.vertebrae[index];
    let offset = vertebra.centre_line_offset_mm?;
    let outline = ctx
        .outlines
        .iter()
        .find(|outline| outline.level == vertebra.level)
        .ok_or(CannotTellCode::NoPedicleTable)?;
    let low = outline.left_y_mm[0].min(outline.right_y_mm[0]);
    let high = outline.left_y_mm[1].max(outline.right_y_mm[1]);
    let line = vertebra.centre_mm[1] - offset;
    Ok((low..=high).contains(&line))
}

fn body_input(ctx: &RuleContext<'_>, index: usize) -> RuleInput {
    let vertebra = &ctx.reading.vertebrae[index];
    let interval = body_interval(ctx, index);
    input(
        ctx,
        Measurement::CentreLineOffset,
        Some(vertebra.level),
        None,
        vertebra.centre_line_offset_mm,
        vertebra.centre_line_offset_mm.ok().map(|value| {
            if value < 0.0 {
                interval[1]
            } else {
                -interval[0]
            }
        }),
    )
}

fn pedicle_input(ctx: &RuleContext<'_>, index: usize) -> RuleInput {
    let vertebra = &ctx.reading.vertebrae[index];
    let edge = ctx
        .outlines
        .iter()
        .find(|outline| outline.level == vertebra.level)
        .map(|outline| {
            let low = outline.left_y_mm[0].min(outline.right_y_mm[0]);
            let high = outline.left_y_mm[1].max(outline.right_y_mm[1]);
            if vertebra
                .centre_line_offset_mm
                .is_ok_and(|value| value < 0.0)
            {
                high - vertebra.centre_mm[1]
            } else {
                vertebra.centre_mm[1] - low
            }
        });
    input(
        ctx,
        Measurement::CentreLineOffset,
        Some(vertebra.level),
        None,
        vertebra.centre_line_offset_mm,
        edge,
    )
}

/// Distinguish a completely observed TL/L region with no body contact
/// from an unobserved candidate. Only the former can continue to the
/// independent proximal-STV definition without inventing an LTV.
fn first_ltv(ctx: &RuleContext<'_>, inputs: &mut Vec<RuleInput>) -> Reading<Option<usize>> {
    for level in template(ctx)
        .levels()
        .into_iter()
        .filter(|level| (Level::T12..=Level::L5).contains(level))
    {
        let index = ctx
            .reading
            .vertebrae
            .iter()
            .position(|v| v.level == level)
            .ok_or(CannotTellCode::NoLevelMeetsTheRule)?;
        inputs.push(body_input(ctx, index));
        if body_touched(ctx, index)? {
            return Ok(Some(index));
        }
    }
    Ok(None)
}

/// One decision path for both public APIs. The source names two domains:
/// LTV is the first TL/L body contact; STV is the most proximal substantial
/// contact across the fitted anatomical chain. The explicit nSTV LTV
/// branch takes precedence over a later STV. A non-substantial contact
/// outside the LTV domain never manufactures an nSTV+1 branch.
fn decision(ctx: &RuleContext<'_>, offset: i32, inputs: &mut Vec<RuleInput>) -> Reading<usize> {
    ctx.reading.csvl_y_mm?;
    if let Some(ltv) = first_ltv(ctx, inputs)? {
        inputs.push(pedicle_input(ctx, ltv));
        if !centre_line_meets_or_is_medial_to_pedicle(ctx, ltv)? {
            let level = shift(ctx, ctx.reading.vertebrae[ltv].level, offset)?;
            return ctx
                .reading
                .vertebrae
                .iter()
                .position(|v| v.level == level)
                .ok_or(CannotTellCode::NoLevelMeetsTheRule);
        }
    }
    for index in 0..ctx.reading.vertebrae.len() {
        if index > 0
            && distance(
                ctx,
                ctx.reading.vertebrae[index - 1].level,
                ctx.reading.vertebrae[index].level,
            )? != 1
        {
            return Err(CannotTellCode::NoLevelMeetsTheRule);
        }
        inputs.push(pedicle_input(ctx, index));
        if centre_line_meets_or_is_medial_to_pedicle(ctx, index)? {
            return Ok(index);
        }
    }
    Err(CannotTellCode::NoLevelMeetsTheRule)
}

/// The rule's effective landmark: STV, or one below its non-substantial
/// LTV. This is the same decision returned by [`evaluate`], as a fit index.
pub fn landmark(ctx: &RuleContext<'_>) -> Reading<usize> {
    let (offset, _) = threshold("substantially-touched", "liv-below-nstv")
        .map_err(|_| CannotTellCode::NoLevelMeetsTheRule)?;
    decision(ctx, offset as i32, &mut Vec::new())
}

pub fn evaluate(ctx: &RuleContext<'_>) -> Result<RuleReading, LayoutError> {
    let (offset, source) = threshold("substantially-touched", "liv-below-nstv")?;
    let mut inputs = Vec::new();
    let level =
        decision(ctx, offset as i32, &mut inputs).map(|index| ctx.reading.vertebrae[index].level);
    Ok(output(
        LevelRule::SubstantiallyTouched,
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

    fn clear_contacts(fixture: &mut Fixture) {
        for index in 0..fixture.reading.vertebrae.len() {
            let y = 100.0;
            fixture.reading.vertebrae[index].centre_mm[1] = y;
            fixture.reading.vertebrae[index].centre_line_offset_mm = Ok(y);
            fixture.input.fit.vertebrae[index].pose.centre[1] = y;
            fixture.input.fit.vertebrae[index].pose.rotation = [0.0; 3];
            fixture.outlines[index].left_y_mm = [y + 8.0, y + 12.0];
            fixture.outlines[index].right_y_mm = [y - 12.0, y - 8.0];
        }
    }

    fn contact(fixture: &mut Fixture, index: usize, y: f64) {
        fixture.reading.vertebrae[index].centre_mm[1] = y;
        fixture.reading.vertebrae[index].centre_line_offset_mm = Ok(y);
        fixture.input.fit.vertebrae[index].pose.centre[1] = y;
        fixture.outlines[index].left_y_mm = [y + 8.0, y + 12.0];
        fixture.outlines[index].right_y_mm = [y - 12.0, y - 8.0];
    }

    fn assert_both(fixture: &Fixture, expected: Reading<Level>) {
        let ctx = fixture.context();
        assert_eq!(evaluate(&ctx).unwrap().level, expected);
        assert_eq!(
            landmark(&ctx).map(|index| fixture.reading.vertebrae[index].level),
            expected
        );
    }

    #[test]
    fn outer_pedicle_contact_is_substantial_and_lateral_body_contact_is_not() {
        let mut fixture = Fixture::new();
        let mut curve = fixture.reading.curves[fixture.reading.major.unwrap()].clone();
        curve.lower_index = 10;
        fixture.reading.curves = vec![curve];
        fixture.reading.major = Some(0);
        clear_contacts(&mut fixture);
        for index in 10..fixture.reading.vertebrae.len() {
            let v = &mut fixture.reading.vertebrae[index];
            v.centre_mm[1] = 0.0;
            v.centre_line_offset_mm = Ok(0.0);
            fixture.input.fit.vertebrae[index].pose.centre[1] = 0.0;
            fixture.input.fit.vertebrae[index].pose.rotation = [0.0; 3];
            fixture.outlines[index].left_y_mm = [8.0, 12.0];
            fixture.outlines[index].right_y_mm = [-12.0, -8.0];
        }
        // Synthetic pedicle outlines have outer edges +/-12 mm. A line
        // at 11.9,12 is substantial at T11; at 12.1 it touches only the
        // body there, so the first substantial body is T12 below it.
        for (line, expected) in [(11.9, Level::T11), (12.0, Level::T11), (12.1, Level::T12)] {
            fixture.reading.vertebrae[10].centre_line_offset_mm = Ok(-line);
            fixture.reading.vertebrae[10].centre_mm[1] = -line;
            fixture.input.fit.vertebrae[10].pose.centre[1] = -line;
            fixture.outlines[10].left_y_mm = [8.0 - line, 12.0 - line];
            fixture.outlines[10].right_y_mm = [-12.0 - line, -8.0 - line];
            assert_eq!(evaluate(&fixture.context()).unwrap().level, Ok(expected));
        }
        fixture.outlines.remove(10);
        assert_eq!(
            evaluate(&fixture.context()).unwrap().level,
            Err(CannotTellCode::NoPedicleTable)
        );
    }

    #[test]
    fn only_a_non_substantial_touch_gives_its_published_next_level() {
        let mut fixture = Fixture::new();
        let mut curve = fixture.reading.curves[fixture.reading.major.unwrap()].clone();
        curve.lower_index = 10;
        fixture.reading.curves = vec![curve];
        fixture.reading.major = Some(0);
        clear_contacts(&mut fixture);
        for index in 10..fixture.reading.vertebrae.len() {
            let v = &mut fixture.reading.vertebrae[index];
            v.centre_mm[1] = if index == 11 { -13.0 } else { -100.0 };
            v.centre_line_offset_mm = Ok(v.centre_mm[1]);
            fixture.input.fit.vertebrae[index].pose.centre[1] = v.centre_mm[1];
            fixture.input.fit.vertebrae[index].pose.rotation = [0.0; 3];
            fixture.outlines[index].left_y_mm = [8.0 + v.centre_mm[1], 12.0 + v.centre_mm[1]];
            fixture.outlines[index].right_y_mm = [-12.0 + v.centre_mm[1], -8.0 + v.centre_mm[1]];
        }
        // All proximal contacts are explicitly clear. Only T12's body
        // touches (13 is between its 12 mm pedicle edge and 15 mm body
        // edge), so LTV is nSTV and Qin gives its published next level.
        let (offset, _) = threshold("substantially-touched", "liv-below-nstv").unwrap();
        assert_eq!(
            evaluate(&fixture.context()).unwrap().level,
            Ok(fixture.input.fit.vertebrae[11 + offset as usize].level)
        );
    }

    #[test]
    fn a_substantial_touch_can_be_cranial_to_the_curve_lower_end() {
        let mut f = Fixture::new();
        clear_contacts(&mut f);
        let mut curve = f.reading.curves[f.reading.major.unwrap()].clone();
        curve.lower_index = 12;
        f.reading.curves = vec![curve];
        f.reading.major = Some(0);
        contact(&mut f, 10, 0.0);
        contact(&mut f, 11, 0.0);
        contact(&mut f, 12, 0.0);
        // LEV=L1, LTV=T12 is substantial, but the STV definition says
        // MOST PROXIMAL without an LEV bound: the earlier T11 wins.
        assert_both(&f, Ok(Level::T11));
    }

    #[test]
    fn the_non_substantial_ltv_branch_cannot_be_overridden_by_an_stv() {
        let mut f = Fixture::new();
        clear_contacts(&mut f);
        contact(&mut f, 11, 13.0);
        contact(&mut f, 13, 0.0);
        let (offset, _) = threshold("substantially-touched", "liv-below-nstv").unwrap();
        let expected = f.input.fit.vertebrae[11 + offset as usize].level;
        // T12's body touches but its outline does not; L1 is clear;
        // L2 is substantial. Qin's nSTV+1 is L1, not the later L2.
        assert_both(&f, Ok(expected));
        // The LTV conditional is explicit. A separate cranial STV also
        // cannot turn this non-substantial LTV into a substantial LTV.
        contact(&mut f, 10, 0.0);
        assert_both(&f, Ok(expected));
    }

    #[test]
    fn no_ltv_never_invents_a_non_substantial_branch_outside_its_region() {
        let mut f = Fixture::new();
        clear_contacts(&mut f);
        contact(&mut f, 10, 13.0);
        // T11 is outside the TL/L LTV domain. Its non-substantial body
        // contact cannot manufacture T12 as an nSTV+1 rule output.
        assert_both(&f, Err(CannotTellCode::NoLevelMeetsTheRule));
        // The independent STV definition still admits a substantial
        // T11 when the completely observed TL/L region has no LTV.
        contact(&mut f, 10, 0.0);
        assert_both(&f, Ok(Level::T11));
    }

    #[test]
    fn the_shared_decision_keeps_missing_marks_and_anatomical_steps_unread() {
        let mut f = Fixture::new();
        clear_contacts(&mut f);
        contact(&mut f, 11, 0.0);
        f.reading.csvl_y_mm = Err(CannotTellCode::NoSacrum);
        assert_both(&f, Err(CannotTellCode::NoSacrum));
        f.reading.csvl_y_mm = Ok(0.0);
        f.outlines.remove(11);
        assert_both(&f, Err(CannotTellCode::NoPedicleTable));
        let mut f = Fixture::new();
        clear_contacts(&mut f);
        contact(&mut f, 11, 0.0);
        f.input.fit.vertebrae.remove(8);
        f.reading.vertebrae.remove(8);
        f.outlines.remove(8);
        assert_both(&f, Err(CannotTellCode::NoLevelMeetsTheRule));
    }
}
