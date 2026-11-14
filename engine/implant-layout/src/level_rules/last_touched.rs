//! The first body touched by the centre line in the TL/L region.

use super::support::{input, output, template};
use super::{RuleContext, RuleReading};
use crate::LayoutError;
use crate::curves::Reading;
use crate::evidence::{Coordinate, Table, tables};
use forward_model::endplates::Endplates;
use forward_model::vertebra_params::VertebraParams;
use shapes::implant_layout::{CannotTellCode, LevelRule, Measurement, RuleEnd};
use shapes::levels::Level;

/// The body's lateral silhouette, relative to its centre, in patient mm.
/// An ellipse's support is sqrt((w e_w.y)^2 + (d e_d.y)^2).
/// Both endplate supports bound the projected body.
pub(crate) fn body_interval(ctx: &RuleContext<'_>, index: usize) -> [f64; 2] {
    let params = VertebraParams::from(&ctx.fit.vertebrae[index].pose);
    let plates = Endplates::of(&params);
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;
    for plate in [plates.top, plates.bottom] {
        let width = plate.half_width * plate.width_axis.y;
        let depth = plate.half_depth * plate.depth_axis.y;
        let extent = libm::sqrt(width * width + depth * depth);
        low = low.min(plate.centre.y - params.centre.y - extent);
        high = high.max(plate.centre.y - params.centre.y + extent);
    }
    [low, high]
}

/// Compare two distances on this fitted body; this needs no table width.
pub(crate) fn body_touched(ctx: &RuleContext<'_>, index: usize) -> Reading<bool> {
    let [low, high] = body_interval(ctx, index);
    ctx.reading.vertebrae[index]
        .centre_line_offset_mm
        .map(|offset| (low..=high).contains(&-offset))
}

pub fn landmark(ctx: &RuleContext<'_>) -> Reading<usize> {
    ctx.reading.csvl_y_mm?;
    // A missing ordinary candidate cannot be assumed not to touch. The
    // declared missing-thoracic count omits T12 anatomically, rather than
    // treating its missing marks as a negative body-contact measurement.
    for level in template(ctx)
        .names()
        .into_iter()
        .filter(|level| (Level::T12..=Level::L5).contains(level))
    {
        let index = ctx
            .reading
            .vertebrae
            .iter()
            .position(|v| v.level == level)
            .ok_or(CannotTellCode::NoLevelMeetsTheRule)?;
        if body_touched(ctx, index)? {
            return Ok(index);
        }
    }
    Err(CannotTellCode::NoLevelMeetsTheRule)
}

pub fn evaluate(ctx: &RuleContext<'_>) -> Result<RuleReading, LayoutError> {
    let mut inputs = Vec::new();
    let level = landmark(ctx).map(|found| {
        for (index, vertebra) in ctx.reading.vertebrae.iter().enumerate().take(found + 1) {
            if !(Level::T12..=Level::L5).contains(&vertebra.level) {
                continue;
            }
            let interval = body_interval(ctx, index);
            let offset = vertebra.centre_line_offset_mm;
            inputs.push(input(
                ctx,
                Measurement::CentreLineOffset,
                Some(vertebra.level),
                None,
                offset,
                offset.ok().map(|offset| {
                    if offset < 0.0 {
                        interval[1]
                    } else {
                        -interval[0]
                    }
                }),
            ));
            // The silhouette includes endplate-centre displacement and
            // rotated elliptical supports, so angle uncertainty can move
            // contact even when centre uncertainty is zero.
            inputs.push(input(
                ctx,
                Measurement::CoronalTilt,
                Some(vertebra.level),
                None,
                Ok(vertebra.upper_tilt_deg),
                None,
            ));
            inputs.push(input(
                ctx,
                Measurement::Twist,
                Some(vertebra.level),
                None,
                vertebra.twist_deg,
                None,
            ));
        }
        ctx.reading.vertebrae[found].level
    });
    // The stop-short table cites the same LTV landmark definition.
    let cell = tables()
        .lookup(
            Table::StopShortOdds,
            &Coordinate {
                key: "proximal-odds".into(),
                group: Some("ltv".into()),
                ..Coordinate::default()
            },
        )
        .ok_or_else(|| LayoutError::Refused("missing-evidence:ltv:proximal-odds".into()))?;
    Ok(output(
        LevelRule::LastTouched,
        RuleEnd::Lower,
        level,
        inputs,
        vec![cell.source.clone()],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level_rules::test_spine::Fixture;

    #[test]
    fn body_contact_counts_its_edge_and_one_step_to_either_side() {
        let mut fixture = Fixture::new();
        for index in 11..fixture.input.fit.vertebrae.len() {
            fixture.input.fit.vertebrae[index].pose.rotation = [0.0; 3];
            fixture.reading.vertebrae[index].centre_line_offset_mm = Ok(0.0);
        }
        let half_width = fixture.input.fit.vertebrae[11].pose.top_half_width;
        // Upright equal-width endplates span centre +/- half_width.
        // T12 is the first TL/L body: its edge counts as a touch; just
        // outside, the next body L1 is the first touch.
        for (offset, expected) in [
            (half_width - 0.1, Level::T12),
            (half_width, Level::T12),
            (half_width + 0.1, Level::L1),
        ] {
            fixture.reading.vertebrae[11].centre_line_offset_mm = Ok(offset);
            fixture.reading.vertebrae[11].centre_mm[1] = offset;
            fixture.input.fit.vertebrae[11].pose.centre[1] = offset;
            assert_eq!(evaluate(&fixture.context()).unwrap().level, Ok(expected));
        }
    }
}
