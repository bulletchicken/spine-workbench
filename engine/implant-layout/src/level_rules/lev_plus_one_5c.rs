//! Type 5C's LEV/LEV+1 rule. Both bending films are required. A disc can
//! reach neutral when their signed angles bracket the published zero:
//! this project's continuous-bend reading avoids rejecting an overshoot.

use super::selective::curve;
use super::support::{input, output, shift, threshold};
use super::uev_5c::applicable;
use super::{RuleContext, RuleReading};
use crate::LayoutError;
use crate::curves::{Reading, geometry::bending_disc};
use shapes::implant_layout::{CannotTellCode, LevelRule, Measurement, RuleEnd};
use shapes::levels::Level;
use shapes::made_up::CurveRegion;

pub fn evaluate(ctx: &RuleContext<'_>) -> Result<RuleReading, LayoutError> {
    let (cobb_min, source) = threshold("lev-plus-one-5c", "tll-cobb-min")?;
    let (neutral, disc_source) = threshold("lev-plus-one-5c", "bending-disc-angle")?;
    let (offset, offset_source) = threshold("lev-plus-one-5c", "liv-below-end")?;
    let selected = applicable(ctx).and_then(|()| curve(ctx, CurveRegion::ThoracolumbarLumbar));
    let mut inputs = Vec::new();
    let result = selected.and_then(|c| {
        let end = ctx.reading.vertebrae[c.lower_index].level;
        inputs.push(input(
            ctx,
            Measurement::Cobb,
            None,
            c.region,
            Ok(c.cobb_deg),
            Some(cobb_min),
        ));
        let next = shift(ctx, end, offset as i32)?;
        let angles = disc_bends(ctx, end, next);
        let residual = angles.map(|(a, b)| {
            if reaches_neutral(a, b, neutral) {
                neutral
            } else if a.abs() <= b.abs() {
                a
            } else {
                b
            }
        });
        inputs.push(input(
            ctx,
            Measurement::BendingDiscAngle,
            Some(end),
            c.region,
            residual,
            Some(neutral),
        ));
        let (left, right) = angles?;
        Ok(
            if c.cobb_deg > cobb_min || !reaches_neutral(left, right, neutral) {
                next
            } else {
                end
            },
        )
    });
    Ok(output(
        LevelRule::LevPlusOne5C,
        RuleEnd::Lower,
        result,
        inputs,
        vec![source, disc_source, offset_source],
    ))
}

fn disc_bends(ctx: &RuleContext<'_>, upper: Level, lower: Level) -> Reading<(f64, f64)> {
    let left = ctx
        .input
        .bend_left
        .as_ref()
        .ok_or(CannotTellCode::NoBendingFilms)?;
    let right = ctx
        .input
        .bend_right
        .as_ref()
        .ok_or(CannotTellCode::NoBendingFilms)?;
    if left.marks.film != shapes::film::FilmKind::BendLeft
        || right.marks.film != shapes::film::FilmKind::BendRight
    {
        return Err(CannotTellCode::NoBendingFilms);
    }
    Ok((
        bending_disc(left, upper, lower)?,
        bending_disc(right, upper, lower)?,
    ))
}

fn reaches_neutral(left: f64, right: f64, neutral: f64) -> bool {
    (left.min(right)..=left.max(right)).contains(&neutral)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level_rules::test_spine::Fixture;
    use shapes::implant_layout::{CurveType, LumbarModifier};
    use shapes::marks::Mark;
    use shapes::pixels::Px;
    use shapes::vertebra::VertebraMarks;

    fn lumbar_fixture() -> Fixture {
        let mut f = Fixture::new();
        let mut c = f.reading.curves[0].clone();
        c.upper_index = 11;
        c.lower_index = 15;
        c.apex_index = 13;
        c.region = Some(CurveRegion::ThoracolumbarLumbar);
        c.structural = Ok(true);
        f.reading.curves = vec![c];
        f.reading.major = Some(0);
        f.reading.classification.curve_type = Ok(CurveType::ThoracolumbarLumbar);
        f.reading.classification.lumbar_modifier = Ok(LumbarModifier::ClearOfApex);
        f
    }

    fn set_disc(fixture: &mut Fixture, left: f64, right: f64) {
        for (film, angle) in [
            (&mut fixture.input.bend_left, left),
            (&mut fixture.input.bend_right, right),
        ] {
            let film = film.as_mut().unwrap();
            let vertebra = film
                .marks
                .vertebrae
                .iter_mut()
                .find(|v| v.level == Level::L4)
                .unwrap();
            let VertebraMarks::Bending { lines } = &mut vertebra.marks else {
                unreachable!()
            };
            lines.inf = [
                Mark::auto(Px::new(100.0, 100.0)),
                Mark::auto(Px::new(120.0, 100.0 - 20.0 * libm::tan(angle.to_radians()))),
            ];
        }
    }

    #[test]
    fn disc_neutral_is_reachable_at_either_boundary_and_between_the_bends() {
        // A continuous bend from zero-1 to zero+1 crosses zero. Two angles
        // on the same side do not. No angular tolerance is supplied.
        let (zero, _) = threshold("lev-plus-one-5c", "bending-disc-angle").unwrap();
        assert!(reaches_neutral(zero - 1.0, zero + 1.0, zero));
        assert!(reaches_neutral(zero, zero + 1.0, zero));
        assert!(reaches_neutral(zero - 1.0, zero, zero));
        assert!(!reaches_neutral(zero + 1.0, zero + 2.0, zero));
        assert!(!reaches_neutral(zero - 2.0, zero - 1.0, zero));
    }

    #[test]
    fn the_published_cobb_boundary_and_disc_crossing_pick_the_hand_worked_lower_end() {
        // LEV is L4 and the next fitted body is L5. The words choose L4
        // at/below the Cobb threshold if the disc reaches neutral; L5
        // above it or when both bends leave the disc on the same side.
        let (limit, _) = threshold("lev-plus-one-5c", "tll-cobb-min").unwrap();
        let mut f = lumbar_fixture();
        for (step, expected) in [(-1.0, Level::L4), (0.0, Level::L4), (1.0, Level::L5)] {
            f.reading.curves[0].cobb_deg = limit + step;
            assert_eq!(evaluate(&f.context()).unwrap().level, Ok(expected));
        }
        f.reading.curves[0].cobb_deg = limit - 1.0;
        set_disc(&mut f, -1.0, 1.0);
        assert_eq!(evaluate(&f.context()).unwrap().level, Ok(Level::L4));
        set_disc(&mut f, 1.0, 2.0);
        assert_eq!(evaluate(&f.context()).unwrap().level, Ok(Level::L5));
        set_disc(&mut f, -2.0, -1.0);
        assert_eq!(evaluate(&f.context()).unwrap().level, Ok(Level::L5));
        f.input.bend_left = None;
        f.reading.curves[0].cobb_deg = limit + 1.0;
        assert_eq!(
            evaluate(&f.context()).unwrap().level,
            Err(CannotTellCode::NoBendingFilms)
        );
        let mut f = lumbar_fixture();
        f.input.bend_right.as_mut().unwrap().marks.vertebrae.clear();
        assert_eq!(
            evaluate(&f.context()).unwrap().level,
            Err(CannotTellCode::NoBendingFilms)
        );
    }
}
