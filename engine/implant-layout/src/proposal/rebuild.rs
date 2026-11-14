//! Rebuild a whole placement using the original table draws and hand choices.

use super::{apply_pattern, candidate_anchors, measure, registered_input, released_levels};
use crate::patterns::Pattern;
use crate::screws::sizing::{diameter_grid, length_grid};
use crate::{LayoutError, rods};
use shapes::implant_layout::{
    AnchorChoice, AnchorKind, AnchorProposal, ImplantLayout, LayoutChange, LookCode, RebuildInput,
    ValueStatus,
};
use shapes::spine_readings::{Band, BodySide};
use std::collections::BTreeMap;

pub(crate) fn rebuild(input: &RebuildInput) -> Result<ImplantLayout, LayoutError> {
    let working = registered_input(&input.layout);
    let fit = &working.fit;
    let start = fit.vertebrae.iter().position(|v| v.level == input.upper);
    let end = fit.vertebrae.iter().position(|v| v.level == input.lower);
    let (Some(start), Some(end)) = (start, end) else {
        return Err(invalid("ends", "level-not-in-fit"));
    };
    if start >= end {
        return Err(invalid("ends", "at-least-two-levels-in-fit-order-required"));
    }
    let levels = crate::alternatives::levels(&working, input.upper, input.lower)
        .ok_or_else(|| invalid("ends", "instrumented-span-not-fitted"))?;
    let diameters = diameter_grid();
    let lengths = length_grid();
    for (i, choice) in input.anchors.iter().enumerate() {
        if !levels.contains(&choice.level) {
            return Err(invalid("anchors", "level-outside-instrumented-span"));
        }
        if input.anchors[..i]
            .iter()
            .any(|a| a.level == choice.level && a.side == choice.side)
        {
            return Err(invalid("anchors", "duplicate-pedicle"));
        }
        for (value, grid, field) in [
            (choice.diameter_mm, &diameters, "anchors.diameterMm"),
            (choice.length_mm, &lengths, "anchors.lengthMm"),
        ] {
            if value.is_some_and(|mm| !grid.iter().any(|g| g.mm == mm)) {
                return Err(invalid(field, "size-off-grid"));
            }
        }
        if choice.kind != AnchorKind::Screw
            && (choice.diameter_mm.is_some() || choice.length_mm.is_some())
        {
            return Err(invalid("anchors", "size-on-non-screw"));
        }
    }
    let default = rods::default_choice()?;
    let rod =
        if default.material == input.rod_material && default.diameter_mm == input.rod_diameter_mm {
            default
        } else {
            rods::choices()
                .into_iter()
                .find(|rod| {
                    rod.material == input.rod_material && rod.diameter_mm == input.rod_diameter_mm
                })
                .ok_or_else(|| invalid("rod", "rod-not-in-choices"))?
        };
    let readings = measure(&working)?;
    let mut anchors = candidate_anchors(&working, &readings, &mut BTreeMap::new(), &levels)?;
    let raw_anchors = anchors.clone();
    apply_pattern(&mut anchors, &levels, &readings, Pattern::AllPedicles)?;
    for choice in &input.anchors {
        let anchor = anchors
            .iter_mut()
            .find(|a| a.level == choice.level && a.side == choice.side)
            .expect("each instrumented pedicle has a candidate");
        let raw = raw_anchors
            .iter()
            .find(|a| a.level == choice.level && a.side == choice.side)
            .expect("the candidate retains each pedicle");
        apply_choice(anchor, raw, choice)?;
        if choice.kind == AnchorKind::Screw && anchor.screw_to_pedicle.is_none() {
            let ratio = crate::screws::ratio_for_diameter(
                &working,
                &readings.versions.versions,
                &readings.measured,
                choice.level,
                choice.side,
                anchor.diameter_mm.expect("a screw has a diameter"),
            )?;
            anchor.screw_to_pedicle = ratio;
        }
        if choice.kind == AnchorKind::Screw
            && anchor.wider_than_pedicle.iter().any(|c| {
                Some(c.diameter_mm) == anchor.diameter_mm
                    && f64::from(c.versions)
                        > crate::screws::sizing::DEFAULT_ALPHA * f64::from(working.versions)
            })
        {
            anchor.looks.push(LookCode::MayBeTooNarrow);
            anchor.looks.sort();
            anchor.looks.dedup();
        }
        crate::screws::refresh_look_sources(anchor);
    }
    for side in [BodySide::Left, BodySide::Right] {
        if anchors
            .iter()
            .filter(|a| a.side == side && a.kind != AnchorKind::None)
            .count()
            < 2
        {
            return Err(invalid("anchors", "side-needs-two-occupied-heads"));
        }
    }
    let rods = rods::propose(fit, &levels, &readings.nominal, &anchors, &rod)?;
    let released_levels = released_levels(&anchors, &levels, input.lower);
    Ok(ImplantLayout {
        id: "changed-by-hand".into(),
        change: LayoutChange::ChangedByHand {},
        upper: input.upper,
        lower: input.lower,
        anchors,
        rods,
        released_levels,
    })
}

fn apply_choice(
    anchor: &mut AnchorProposal,
    raw: &AnchorProposal,
    choice: &AnchorChoice,
) -> Result<(), LayoutError> {
    anchor.kind = choice.kind;
    if choice.kind == AnchorKind::Hook && anchor.direction_status == ValueStatus::Missing {
        return Err(invalid("anchors", "hook-direction-missing"));
    }
    if choice.kind != AnchorKind::Screw {
        anchor.diameter_mm = None;
        anchor.length_mm = None;
        anchor.diameter_also_seen.clear();
        anchor.length_also_seen.clear();
        anchor.medial_angle = None;
        anchor.screw_to_pedicle = None;
        anchor.diameter_status = ValueStatus::Missing;
        anchor.length_status = ValueStatus::Missing;
        return Ok(());
    }
    let candidate = raw.screw_here.as_ref();
    let diameter = choice
        .diameter_mm
        .or(candidate.map(|s| s.diameter_mm))
        .ok_or_else(|| invalid("anchors.diameterMm", "no-table-size"))?;
    let length = choice
        .length_mm
        .or(candidate.map(|s| s.length_mm))
        .ok_or_else(|| invalid("anchors.lengthMm", "no-table-size"))?;
    if anchor.direction_status == ValueStatus::Missing {
        return Err(invalid("anchors", "screw-direction-missing"));
    }
    let changed_diameter = candidate.is_none_or(|s| s.diameter_mm != diameter);
    let changed_length = candidate.is_none_or(|s| s.length_mm != length);
    let ratio = candidate.and_then(|s| {
        s.screw_to_pedicle
            .map(|b| scale_band(b, diameter / s.diameter_mm))
    });
    let medial_angle = raw.medial_angle.clone();
    anchor.diameter_mm = Some(diameter);
    anchor.length_mm = Some(length);
    anchor.diameter_also_seen = candidate
        .filter(|_| !changed_diameter)
        .map_or_else(Vec::new, |s| s.diameter_also_seen.clone());
    anchor.length_also_seen = candidate
        .filter(|_| !changed_length)
        .map_or_else(Vec::new, |s| s.length_also_seen.clone());
    anchor.screw_to_pedicle = ratio;
    anchor.medial_angle = medial_angle;
    anchor.diameter_status = if changed_diameter {
        ValueStatus::Entered
    } else {
        ValueStatus::FromTables
    };
    anchor.length_status = if changed_length {
        ValueStatus::Entered
    } else {
        ValueStatus::FromTables
    };
    if changed_diameter {
        anchor.diameter_sources.clear();
    }
    if changed_length {
        anchor.length_sources.clear();
    }
    Ok(())
}

fn scale_band(band: Band, scale: f64) -> Band {
    Band {
        p10: band.p10 * scale,
        p50: band.p50 * scale,
        p90: band.p90 * scale,
    }
}

fn invalid(field: &str, reason: &str) -> LayoutError {
    LayoutError::InvalidValue {
        field: field.into(),
        reason: reason.into(),
    }
}
