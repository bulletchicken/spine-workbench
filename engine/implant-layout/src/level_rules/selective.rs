//! The selective-fusion criteria kept separately, including unread inputs.
//! A conjunction excludes unread criteria; it is unread itself if none of
//! its criteria can be read. Counts always use the whole ensemble.

use super::RuleContext;
use super::support::threshold;
use crate::LayoutError;
use crate::curves::summary::band;
use crate::curves::{Curve, Reading};
use shapes::implant_layout::{
    CannotTellCode, LevelRule, Measurement, Readable, SelectiveCriterion, SelectiveReading,
};
use shapes::levels::Level;
use shapes::made_up::CurveRegion;

/// This project's inclusive interval for offering both fusion regions.
pub const BOTH_REGIONS_SHARE: [f64; 2] = [0.2, 0.8];

/// One criterion on one version. Inferences remain explicit beside their
/// published threshold; a missing reading is never a met criterion.
#[derive(Clone, Debug, PartialEq)]
pub struct CriterionReading {
    pub measurement: Measurement,
    pub region: Option<CurveRegion>,
    pub threshold: f64,
    pub holds_below: bool,
    pub value: Reading<f64>,
    pub source: String,
    /// True for a degree-based AVR ratio, or the corrected thoracic bend
    /// used by the selective-lumbar criterion. Neither is printed as such
    /// in the source table.
    pub project_inference: bool,
}

impl CriterionReading {
    pub fn holds(&self) -> Reading<bool> {
        self.value.map(|value| {
            if self.holds_below {
                value < self.threshold
            } else {
                value > self.threshold
            }
        })
    }
}

/// Each criterion and the conjunction of the readable ones, per version.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectiveVersion {
    pub rule: LevelRule,
    pub criteria: Vec<CriterionReading>,
    pub all_hold: Reading<bool>,
}

pub(super) fn finish(rule: LevelRule, criteria: Vec<CriterionReading>) -> SelectiveVersion {
    let mut any = false;
    let mut all = true;
    let mut missing = CannotTellCode::NoCurve;
    for criterion in &criteria {
        match criterion.holds() {
            Ok(holds) => {
                any = true;
                all &= holds;
            }
            Err(reason) => missing = reason,
        }
    }
    SelectiveVersion {
        rule,
        criteria,
        all_hold: if any { Ok(all) } else { Err(missing) },
    }
}

pub(super) fn criterion(
    group: &str,
    key: &str,
    measurement: Measurement,
    region: Option<CurveRegion>,
    value: Reading<f64>,
    holds_below: bool,
    project_inference: bool,
) -> Result<CriterionReading, LayoutError> {
    let (threshold, source) = threshold(group, key)?;
    Ok(CriterionReading {
        measurement,
        region,
        threshold,
        holds_below,
        value,
        source,
        project_inference,
    })
}

/// Largest standing curve in this region. Ties take the first in the
/// measured top-to-bottom order, as the nominal major-curve reading does.
pub fn curve<'a>(ctx: &'a RuleContext<'_>, region: CurveRegion) -> Reading<&'a Curve> {
    ctx.reading
        .curves
        .iter()
        .filter(|curve| curve.region == Some(region))
        .fold(None, |result: Option<&Curve>, curve| {
            if result.is_none_or(|previous| curve.cobb_deg > previous.cobb_deg) {
                Some(curve)
            } else {
                result
            }
        })
        .ok_or(CannotTellCode::NoCurve)
}

/// Ratios of absolute magnitudes have no value for a zero denominator.
/// Infinity is not used as a substitute for an unreadable ratio.
pub fn ratio(numerator: Reading<f64>, denominator: Reading<f64>) -> Reading<f64> {
    let numerator = numerator?;
    let denominator = denominator?;
    let value = numerator.abs() / denominator.abs();
    if value.is_finite() {
        Ok(value)
    } else {
        Err(CannotTellCode::NoLevelMeetsTheRule)
    }
}

/// TL/L translation uses the marked CSVL; thoracic translation requires
/// a fitted C7 centre to define the C7 plumb line. The curve's end-centre
/// chord is an apex locator, not a substitute plumb line.
pub fn translation(ctx: &RuleContext<'_>, curve: Reading<&Curve>) -> Reading<f64> {
    let curve = curve?;
    let apex = ctx
        .reading
        .vertebrae
        .get(curve.apex_index)
        .ok_or(CannotTellCode::NoCurve)?;
    let line = if curve.region == Some(CurveRegion::ThoracolumbarLumbar) {
        ctx.reading.csvl_y_mm?
    } else {
        ctx.reading
            .vertebrae
            .iter()
            .find(|vertebra| vertebra.level == Level::C7)
            .map(|vertebra| vertebra.centre_mm[1])
            .ok_or(CannotTellCode::NoLevelMeetsTheRule)?
    };
    Ok((apex.centre_mm[1] - line).abs())
}

pub fn rotation(ctx: &RuleContext<'_>, curve: Reading<&Curve>) -> Reading<f64> {
    ctx.reading
        .vertebrae
        .get(curve?.apex_index)
        .ok_or(CannotTellCode::NoCurve)?
        .twist_deg
        .map(f64::abs)
}

/// Summarise one test over a complete ensemble. When only some versions
/// are readable, their met count is still out of the whole ensemble. The
/// raw versions retain each missing reason for `cannotTell` assembly.
pub fn summarise(versions: &[SelectiveVersion]) -> SelectiveReading {
    let rule = versions
        .first()
        .map_or(LevelRule::SelectiveThoracic, |version| version.rule);
    let mut criteria = Vec::new();
    let mut sources = Vec::new();
    if let Some(first) = versions.first() {
        for (index, criterion) in first.criteria.iter().enumerate() {
            let values: Vec<_> = versions
                .iter()
                .filter_map(|version| version.criteria.get(index)?.value.ok())
                .collect();
            criteria.push(SelectiveCriterion {
                measurement: criterion.measurement,
                region: criterion.region,
                threshold: criterion.threshold,
                holds_below: criterion.holds_below,
                value: band(&values),
                holds: count(versions.iter().filter_map(|version| {
                    version.criteria.get(index).map(CriterionReading::holds)
                })),
            });
            sources.push(criterion.source.clone());
        }
    }
    sources.sort();
    sources.dedup();
    let all_hold = count(versions.iter().map(|version| version.all_hold));
    let both_regions_offered = match all_hold {
        Readable::Share { versions: met } if !versions.is_empty() => (BOTH_REGIONS_SHARE[0]
            ..=BOTH_REGIONS_SHARE[1])
            .contains(&(f64::from(met) / versions.len() as f64)),
        _ => false,
    };
    SelectiveReading {
        rule,
        criteria,
        all_hold,
        both_regions_offered,
        sources,
    }
}

fn count(values: impl Iterator<Item = Reading<bool>>) -> Readable {
    let mut read = false;
    let mut met = 0;
    let mut reason = CannotTellCode::NoCurve;
    for value in values {
        match value {
            Ok(holds) => {
                read = true;
                met += u32::from(holds);
            }
            Err(code) => reason = code,
        }
    }
    if read {
        Readable::Share { versions: met }
    } else {
        Readable::NotReadable { reason }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level_rules::{selective_lumbar, selective_thoracic, test_spine::Fixture};

    #[test]
    fn every_published_selective_threshold_keeps_its_strict_boundary() {
        // The note's words are "greater than" for ratios and "less than"
        // for residual Cobb, standing Cobb and regional kyphosis. Equality
        // therefore never meets any of these criteria.
        for (group, keys) in [
            (
                "selective-thoracic",
                vec![
                    ("cobb-ratio-min", false),
                    ("translation-ratio-min", false),
                    ("rotation-ratio-min", false),
                    ("bending-cobb-max", true),
                    ("kyphosis-t10-l2-max", true),
                    ("cobb-max", true),
                ],
            ),
            (
                "selective-lumbar",
                vec![
                    ("cobb-ratio-min", false),
                    ("translation-ratio-min", false),
                    ("rotation-ratio-min", false),
                    ("tll-bending-cobb-max", true),
                    ("thoracic-cobb-max", true),
                ],
            ),
        ] {
            for (key, below) in keys {
                let mut c =
                    criterion(group, key, Measurement::Cobb, None, Ok(0.0), below, false).unwrap();
                for step in [-1.0, 0.0, 1.0] {
                    c.value = Ok(c.threshold + step);
                    assert_eq!(c.holds(), Ok(if below { step < 0.0 } else { step > 0.0 }));
                }
                c.value = Err(CannotTellCode::NoBendingFilms);
                assert_eq!(c.holds(), Err(CannotTellCode::NoBendingFilms));
            }
        }
    }

    #[test]
    fn unread_is_excluded_from_the_conjunction_and_never_counted_as_met() {
        let (threshold, _) = threshold("selective-thoracic", "cobb-ratio-min").unwrap();
        let readable = criterion(
            "selective-thoracic",
            "cobb-ratio-min",
            Measurement::CobbRatio,
            None,
            Ok(threshold + 1.0),
            false,
            false,
        )
        .unwrap();
        let mut unread = readable.clone();
        unread.value = Err(CannotTellCode::TwistNotMeasured);
        let version = finish(LevelRule::SelectiveThoracic, vec![readable, unread]);
        assert_eq!(version.all_hold, Ok(true));
        let summary = summarise(&[version]);
        assert_eq!(summary.criteria[0].holds, Readable::Share { versions: 1 });
        assert_eq!(
            summary.criteria[1].holds,
            Readable::NotReadable {
                reason: CannotTellCode::TwistNotMeasured
            }
        );
    }

    #[test]
    fn both_regions_include_the_two_share_boundaries() {
        for met in [1, 2, 8, 9] {
            let versions: Vec<_> = (0..10)
                .map(|i| SelectiveVersion {
                    rule: LevelRule::SelectiveLumbar,
                    criteria: Vec::new(),
                    all_hold: Ok(i < met),
                })
                .collect();
            assert_eq!(
                summarise(&versions).both_regions_offered,
                met == 2 || met == 8
            );
        }
    }

    #[test]
    fn regional_kyphosis_alone_does_not_supply_an_absent_pair_of_curves() {
        let mut f = Fixture::new();
        f.reading.curves.clear();
        f.reading.major = None;
        f.reading.kyphosis_t10_l2_deg = Ok(0.0);
        assert_eq!(
            selective_thoracic::evaluate(&f.context()).unwrap().all_hold,
            Err(CannotTellCode::NoCurve)
        );
        assert_eq!(
            selective_lumbar::evaluate(&f.context()).unwrap().all_hold,
            Err(CannotTellCode::NoCurve)
        );
    }

    #[test]
    fn two_made_up_curves_keep_their_independent_ratio_and_film_inputs() {
        // Hand readings: C7 lies at y=0, MT apex at y=-18, lumbar apex
        // at y=6 and CSVL=0. Hence MT:TL/L AVT=18/6=3. Twist magnitudes
        // 20 and 10 give AVR=2. The end-centre chord is deliberately a
        // different number, proving it is not used as the plumb line.
        let mut f = Fixture::new();
        f.reading.vertebrae[0].level = Level::C7;
        let mut mt = f.reading.curves[0].clone();
        mt.region = Some(CurveRegion::MainThoracic);
        mt.upper_index = 0;
        mt.lower_index = 10;
        mt.apex_index = 7;
        let mut tl = mt.clone();
        tl.region = Some(CurveRegion::ThoracolumbarLumbar);
        tl.upper_index = 11;
        tl.lower_index = 15;
        tl.apex_index = 13;
        let (ratio_limit, _) = threshold("selective-thoracic", "cobb-ratio-min").unwrap();
        let (cobb_max, _) = threshold("selective-thoracic", "cobb-max").unwrap();
        let (bend_max, _) = threshold("selective-thoracic", "bending-cobb-max").unwrap();
        let (kyphosis_max, _) = threshold("selective-thoracic", "kyphosis-t10-l2-max").unwrap();
        tl.cobb_deg = cobb_max - 1.0;
        mt.cobb_deg = (ratio_limit + 1.0) * tl.cobb_deg;
        tl.bending_cobb_deg = Ok(bend_max - 1.0);
        mt.bending_cobb_deg = Ok(bend_max + 1.0);
        mt.apical_deviation_mm = -77.0;
        f.reading.curves = vec![mt, tl];
        f.reading.csvl_y_mm = Ok(0.0);
        f.reading.kyphosis_t10_l2_deg = Ok(kyphosis_max - 1.0);
        f.reading.vertebrae[7].centre_mm[1] = -18.0;
        f.reading.vertebrae[13].centre_mm[1] = 6.0;
        f.reading.vertebrae[7].twist_deg = Ok(-20.0);
        f.reading.vertebrae[13].twist_deg = Ok(10.0);
        let thoracic = selective_thoracic::evaluate(&f.context()).unwrap();
        assert!((thoracic.criteria[0].value.unwrap() - (ratio_limit + 1.0)).abs() < 1e-12);
        assert_eq!(thoracic.criteria[1].value, Ok(3.0));
        assert_eq!(thoracic.criteria[2].value, Ok(2.0));
        assert_eq!(thoracic.criteria[3].value, Ok(bend_max - 1.0));
        assert!(thoracic.criteria[2].project_inference);
        f.reading.vertebrae[0].level = Level::T1;
        let missing = selective_thoracic::evaluate(&f.context()).unwrap();
        assert_eq!(
            missing.criteria[1].value,
            Err(CannotTellCode::NoLevelMeetsTheRule)
        );
        assert_eq!(missing.criteria[0].value, thoracic.criteria[0].value);
        f.reading.vertebrae[13].twist_deg = Err(CannotTellCode::TwistNotMeasured);
        let missing = selective_thoracic::evaluate(&f.context()).unwrap();
        assert_eq!(
            missing.criteria[2].value,
            Err(CannotTellCode::TwistNotMeasured)
        );

        // DECISIONS corrects the review's printed TL/L-bend cell to read
        // the thoracic bend for selective lumbar fusion. The two curves
        // deliberately straddle the boundary in opposite directions.
        let (limit, _) = threshold("selective-lumbar", "tll-bending-cobb-max").unwrap();
        for step in [-1.0, 0.0, 1.0] {
            f.reading.curves[0].bending_cobb_deg = Ok(limit + step);
            f.reading.curves[1].bending_cobb_deg = Ok(limit - step);
            let lumbar = selective_lumbar::evaluate(&f.context()).unwrap();
            assert_eq!(lumbar.criteria[3].value, Ok(limit + step));
            assert_eq!(lumbar.criteria[3].holds(), Ok(step < 0.0));
            assert!(lumbar.criteria[3].project_inference);
        }
    }
}
