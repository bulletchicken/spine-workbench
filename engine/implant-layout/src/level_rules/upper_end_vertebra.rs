//! The highest curve's upper end in the supplied fusion region. A missing
//! structural test remains unreadable rather than excluding that curve.

use super::support::{input, output, threshold, upper_curve};
use super::{FusionRegion, RuleContext, RuleReading};
use crate::LayoutError;
use shapes::implant_layout::{LevelRule, Measurement, RuleEnd};
use shapes::made_up::CurveRegion;

pub fn evaluate(ctx: &RuleContext<'_>) -> Result<RuleReading, LayoutError> {
    let curve = upper_curve(ctx);
    let level = curve.map(|c| ctx.reading.vertebrae[c.upper_index].level);
    let mut inputs = Vec::new();
    let (bend_min, bend_source) = threshold("structural-minor", "bending-cobb-min")?;
    let (proximal_min, proximal_source) = threshold("structural-minor", "kyphosis-t2-t5-min")?;
    let (lower_min, lower_source) = threshold("structural-minor", "kyphosis-t10-l2-min")?;
    for (index, c) in ctx.reading.curves.iter().enumerate() {
        let in_region = match ctx.region {
            FusionRegion::Structural => true,
            FusionRegion::Thoracic => matches!(
                c.region,
                Some(CurveRegion::ProximalThoracic | CurveRegion::MainThoracic)
            ),
            FusionRegion::Lumbar => c.region == Some(CurveRegion::ThoracolumbarLumbar),
        };
        if !in_region {
            continue;
        }
        inputs.push(input(
            ctx,
            Measurement::CoronalTilt,
            Some(ctx.reading.vertebrae[c.upper_index].level),
            c.region,
            Ok(ctx.reading.vertebrae[c.upper_index].upper_tilt_deg),
            None,
        ));
        if ctx.reading.major != Some(index) && ctx.region != FusionRegion::Lumbar {
            inputs.push(input(
                ctx,
                Measurement::BendingCobb,
                None,
                c.region,
                c.bending_cobb_deg,
                Some(bend_min),
            ));
            let (measurement, value, threshold) = if c.region == Some(CurveRegion::ProximalThoracic)
            {
                (
                    Measurement::KyphosisT2T5,
                    ctx.reading.kyphosis_t2_t5_deg,
                    proximal_min,
                )
            } else {
                (
                    Measurement::KyphosisT10L2,
                    ctx.reading.kyphosis_t10_l2_deg,
                    lower_min,
                )
            };
            inputs.push(input(
                ctx,
                measurement,
                None,
                c.region,
                value,
                Some(threshold),
            ));
        }
    }
    Ok(output(
        LevelRule::UpperEndVertebra,
        RuleEnd::Upper,
        level,
        inputs,
        vec![
            "ao-lenke".into(),
            "lenke-2001".into(),
            bend_source,
            proximal_source,
            lower_source,
        ],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level_rules::test_spine::Fixture;
    use shapes::levels::Level;

    #[test]
    fn the_highest_structural_minor_extends_the_upper_end_at_the_threshold() {
        // The major starts at T6. A proximal minor starts at T2. The
        // rule's words include that minor at the bending boundary, so
        // the upper end moves T6 -> T2. Below the boundary it stays T6.
        let (limit, _) = threshold("structural-minor", "bending-cobb-min").unwrap();
        let mut f = Fixture::new();
        let mut minor = f.reading.curves[0].clone();
        minor.upper_index = 1;
        minor.lower_index = 5;
        minor.apex_index = 3;
        minor.region = Some(CurveRegion::ProximalThoracic);
        let mut major = minor.clone();
        major.upper_index = 5;
        major.lower_index = 10;
        major.apex_index = 7;
        major.region = Some(CurveRegion::MainThoracic);
        major.structural = Ok(true);
        f.reading.curves = vec![minor, major];
        f.reading.major = Some(1);
        f.reading.kyphosis_t2_t5_deg = Ok(0.0);
        for (step, expected) in [(-1.0, Level::T6), (0.0, Level::T2), (1.0, Level::T2)] {
            f.reading.curves[0].bending_cobb_deg = Ok(limit + step);
            f.reading.curves[0].structural = Ok(step >= 0.0);
            assert_eq!(evaluate(&f.context()).unwrap().level, Ok(expected));
        }
    }
}
