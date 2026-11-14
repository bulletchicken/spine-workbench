//! The shoulders are outside this spine model. T1 tilt can be displayed
//! beside this unreadable rule, with its own measured status; it never
//! supplies a shoulder-height vote.

use super::support::{input, output};
use super::{RuleContext, RuleReading};
use crate::LayoutError;
use shapes::implant_layout::{CannotTellCode, LevelRule, Measurement, RuleEnd};
use shapes::levels::Level;

pub fn evaluate(ctx: &RuleContext<'_>) -> Result<RuleReading, LayoutError> {
    let inputs = ctx
        .reading
        .vertebrae
        .iter()
        .find(|v| v.level == Level::T1)
        .map(|v| {
            input(
                ctx,
                Measurement::CoronalTilt,
                Some(v.level),
                None,
                Ok(v.upper_tilt_deg),
                None,
            )
        })
        .into_iter()
        .collect();
    Ok(output(
        LevelRule::LenkeShoulder,
        RuleEnd::Upper,
        Err(CannotTellCode::ShouldersOutsideTheModel),
        inputs,
        vec![
            "bmc-level-review".into(),
            "brooks-2018".into(),
            "uiv-tilt-40020691".into(),
        ],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level_rules::test_spine::Fixture;

    #[test]
    fn t1_tilt_on_either_side_of_level_never_becomes_a_shoulder_vote() {
        let mut f = Fixture::new();
        for tilt in [-1.0, 0.0, 1.0] {
            f.reading.vertebrae[0].upper_tilt_deg = tilt;
            let vote = evaluate(&f.context()).unwrap();
            assert_eq!(vote.level, Err(CannotTellCode::ShouldersOutsideTheModel));
            assert_eq!(vote.inputs[0].value, Some(tilt));
        }
    }
}
