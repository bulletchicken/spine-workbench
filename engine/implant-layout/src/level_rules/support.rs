//! Evidence access, anatomical naming distances and uncertainty of rule inputs.

use super::{FusionRegion, RuleContext, RuleReading};
use crate::LayoutError;
use crate::curves::{Curve, Reading};
use crate::evidence::{Cell, Coordinate, PublishedValue, Table, tables};
use shapes::implant_layout::{
    CannotTellCode, LevelRule, Measurement, RuleEnd, RuleInput, ValueStatus,
};
use shapes::levels::Level;
use shapes::made_up::CurveRegion;
use shapes::naming::Template;
use shapes::spine_fit::SpineFit;

pub fn cell(table: Table, group: &str, key: &str) -> Result<&'static Cell, LayoutError> {
    tables()
        .lookup(
            table,
            &Coordinate {
                key: key.into(),
                group: Some(group.into()),
                ..Coordinate::default()
            },
        )
        .ok_or_else(|| LayoutError::Refused(format!("missing-evidence:{group}:{key}")))
}

pub fn threshold(group: &str, key: &str) -> Result<(f64, String), LayoutError> {
    let cell = cell(Table::LevelRuleThresholds, group, key)?;
    let PublishedValue::Scalar { value } = cell.value else {
        return Err(LayoutError::Refused(format!(
            "non-scalar-evidence:{group}:{key}"
        )));
    };
    Ok((value, cell.source.clone()))
}

/// Curves included by the declared region. An unresolved structural minor
/// in that region prevents inventing its extent. Lumbar selective fusion
/// explicitly reads the TL/L curve alone.
pub fn included_curves<'a>(context: &RuleContext<'a>) -> Reading<Vec<&'a Curve>> {
    let mut curves = Vec::new();
    for (i, curve) in context.reading.curves.iter().enumerate() {
        let in_region = match context.region {
            FusionRegion::Structural => true,
            FusionRegion::Thoracic => matches!(
                curve.region,
                Some(CurveRegion::ProximalThoracic | CurveRegion::MainThoracic)
            ),
            FusionRegion::Lumbar => curve.region == Some(CurveRegion::ThoracolumbarLumbar),
        };
        if !in_region {
            continue;
        }
        if context.region == FusionRegion::Lumbar
            || context.reading.major == Some(i)
            || (context.region == FusionRegion::Thoracic
                && curve.region == Some(CurveRegion::MainThoracic))
            || curve.structural?
        {
            curves.push(curve);
        }
    }
    if curves.is_empty() {
        Err(CannotTellCode::NoCurve)
    } else {
        Ok(curves)
    }
}

pub fn lower_curve<'a>(context: &RuleContext<'a>) -> Reading<&'a Curve> {
    included_curves(context)?
        .into_iter()
        .max_by_key(|c| c.lower_index)
        .ok_or(CannotTellCode::NoCurve)
}

pub fn upper_curve<'a>(context: &RuleContext<'a>) -> Reading<&'a Curve> {
    included_curves(context)?
        .into_iter()
        .min_by_key(|c| c.upper_index)
        .ok_or(CannotTellCode::NoCurve)
}

/// Use the film's declared count where available. With no count metadata,
/// only an explicitly fitted T13 establishes an extra thoracic vertebra;
/// a missing ordinary level never becomes evidence of a different count.
pub fn template(context: &RuleContext<'_>) -> Template {
    context.input.front.as_ref().map_or_else(
        || fit_template(context.fit),
        |front| front.marks.naming.template,
    )
}

/// Count inference available to pure fit-only pair functions. Extra or
/// missing lumbar counts need the film's metadata and are not inferred
/// from a partial fitted chain.
pub fn fit_template(fit: &SpineFit) -> Template {
    if fit
        .vertebrae
        .iter()
        .any(|vertebra| vertebra.level == Level::T13)
    {
        Template::ExtraThoracic
    } else {
        Template::Standard
    }
}

/// Signed caudal distance on the declared anatomical name sequence.
/// This does not need geometry between the named endpoints.
pub fn named_distance(template: Template, from: Level, to: Level) -> Reading<i32> {
    let names = template.names();
    let from = names
        .iter()
        .position(|&name| name == from)
        .ok_or(CannotTellCode::NoLevelMeetsTheRule)?;
    let to = names
        .iter()
        .position(|&name| name == to)
        .ok_or(CannotTellCode::NoLevelMeetsTheRule)?;
    Ok(to as i32 - from as i32)
}

/// Distance between two fitted vertebrae in anatomical levels. Missing
/// intermediate geometry does not shorten this known naming distance.
pub fn distance(context: &RuleContext<'_>, from: Level, to: Level) -> Reading<i32> {
    if ![from, to]
        .iter()
        .all(|level| context.fit.vertebrae.iter().any(|v| v.level == *level))
    {
        return Err(CannotTellCode::NoLevelMeetsTheRule);
    }
    named_distance(template(context), from, to)
}

/// Shift along anatomical names, requiring the target and every traversed
/// vertebra to be fitted. An omitted T9 never makes T10 one below T8.
/// The declared count still gives the ordinary T12/L1 transition directly,
/// or T11/L1 when the film explicitly declares a missing thoracic body.
pub fn shift(context: &RuleContext<'_>, level: Level, delta: i32) -> Reading<Level> {
    shift_on_fit(context.fit, template(context), level, delta)
}

pub(crate) fn shift_on_fit(
    fit: &SpineFit,
    template: Template,
    level: Level,
    delta: i32,
) -> Reading<Level> {
    let names = template.names();
    let from = names
        .iter()
        .position(|&name| name == level)
        .ok_or(CannotTellCode::NoLevelMeetsTheRule)?;
    let target = usize::try_from(from as i64 + i64::from(delta))
        .ok()
        .filter(|&i| i < names.len())
        .ok_or(CannotTellCode::NoLevelMeetsTheRule)?;
    if !names[from.min(target)..=from.max(target)]
        .iter()
        .all(|name| fit.vertebrae.iter().any(|v| v.level == *name))
    {
        return Err(CannotTellCode::NoLevelMeetsTheRule);
    }
    Ok(names[target])
}

pub fn output(
    rule: LevelRule,
    end: RuleEnd,
    level: Reading<Level>,
    inputs: Vec<RuleInput>,
    mut sources: Vec<String>,
) -> RuleReading {
    sources.sort();
    sources.dedup();
    RuleReading {
        rule,
        end,
        level,
        inputs,
        sources,
    }
}

/// Marginal uncertainty propagated in quadrature, using the fit's existing
/// independent-marginal convention. Films supply no bending-mark SD.
pub fn input(
    context: &RuleContext<'_>,
    measurement: Measurement,
    level: Option<Level>,
    region: Option<CurveRegion>,
    value: Reading<f64>,
    threshold: Option<f64>,
) -> RuleInput {
    let vertebra = level.and_then(|l| context.fit.vertebrae.iter().position(|v| v.level == l));
    let angle_sd =
        |i: usize, axis: usize| context.fit.vertebrae[i].sd.angles_rad[axis].to_degrees();
    let pair_sd =
        |a: usize, b: usize, axis: usize| libm::hypot(angle_sd(a, axis), angle_sd(b, axis));
    let curve = region.and_then(|r| {
        context
            .reading
            .curves
            .iter()
            .filter(|c| {
                c.region == Some(r)
                    && level.is_none_or(|l| context.reading.vertebrae[c.apex_index].level == l)
            })
            .max_by(|a, b| a.cobb_deg.total_cmp(&b.cobb_deg))
    });
    let sd = match measurement {
        Measurement::CoronalTilt => vertebra.map(|i| angle_sd(i, 0)),
        Measurement::Twist | Measurement::ApicalRotation => vertebra.map(|i| angle_sd(i, 2)),
        Measurement::CentreLineOffset | Measurement::ApicalTranslation => {
            vertebra.map(|i| context.fit.vertebrae[i].sd.centre_mm[1])
        }
        Measurement::DiscAngle => vertebra
            .filter(|i| i + 1 < context.fit.vertebrae.len())
            .map(|i| pair_sd(i, i + 1, 0)),
        Measurement::Cobb => curve.map(|c| pair_sd(c.upper_index, c.lower_index, 0)),
        Measurement::KyphosisT2T5 | Measurement::KyphosisT5T12 | Measurement::KyphosisT10L2 => {
            let (a, b) = match measurement {
                Measurement::KyphosisT2T5 => (Level::T2, Level::T5),
                Measurement::KyphosisT5T12 => (Level::T5, Level::T12),
                _ => (Level::T10, Level::L2),
            };
            context
                .fit
                .vertebrae
                .iter()
                .position(|v| v.level == a)
                .zip(context.fit.vertebrae.iter().position(|v| v.level == b))
                .map(|(a, b)| pair_sd(a, b, 1))
        }
        _ => None,
    };
    RuleInput {
        measurement,
        level,
        region,
        value: value.ok(),
        sd: value.ok().and(sd),
        threshold,
        status: if value.is_ok() {
            ValueStatus::Measured
        } else {
            ValueStatus::Missing
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level_rules::test_spine::Fixture;

    #[test]
    fn an_unfitted_t9_does_not_make_t10_adjacent_to_t8() {
        let mut f = Fixture::new();
        f.input.fit.vertebrae.retain(|v| v.level != Level::T9);
        assert_eq!(
            shift(&f.context(), Level::T8, 1),
            Err(CannotTellCode::NoLevelMeetsTheRule)
        );
        assert_eq!(
            shift(&f.context(), Level::T8, 2),
            Err(CannotTellCode::NoLevelMeetsTheRule)
        );
        assert_eq!(
            shift(&f.context(), Level::T10, -1),
            Err(CannotTellCode::NoLevelMeetsTheRule)
        );
        // Counting named levels still gives T8 -> T9 -> T10, distance2.
        assert_eq!(distance(&f.context(), Level::T8, Level::T10), Ok(2));
    }

    #[test]
    fn the_declared_count_controls_the_thoracolumbar_transition() {
        let mut f = Fixture::new();
        assert_eq!(shift(&f.context(), Level::T12, 1), Ok(Level::L1));
        f.input.fit.vertebrae.retain(|v| v.level != Level::T12);
        assert_eq!(
            shift(&f.context(), Level::T11, 1),
            Err(CannotTellCode::NoLevelMeetsTheRule)
        );
        f.input.front.as_mut().unwrap().marks.naming.template = Template::MissingThoracic;
        assert_eq!(shift(&f.context(), Level::T11, 1), Ok(Level::L1));
        assert_eq!(distance(&f.context(), Level::T11, Level::L1), Ok(1));
        // With no film metadata a missing T12 stays a missing mark.
        f.input.front = None;
        assert_eq!(
            shift(&f.context(), Level::T11, 1),
            Err(CannotTellCode::NoLevelMeetsTheRule)
        );
        assert_eq!(distance(&f.context(), Level::T11, Level::L1), Ok(2));
    }
}
