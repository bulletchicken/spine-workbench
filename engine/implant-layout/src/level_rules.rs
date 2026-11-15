//! Published level rules, counted over the same versions as the load bench.
//! A missing reading remains a refusal, never a vote. Distances are patient
//! millimetres and angles in rule inputs are degrees.

pub mod deviated_vertebra;
pub mod end_plus_one;
pub mod last_touched;
pub mod lenke_l3_l4;
pub mod lenke_shoulder;
pub mod lev_plus_one_5c;
pub mod liv_index;
pub mod selective;
pub mod selective_lumbar;
pub mod selective_thoracic;
pub mod stable_minus_one;
pub mod substantially_touched;
pub mod suk_neutral;
pub mod uev_5c;
pub mod upper_end_vertebra;

mod contact_events;
mod contact_roots;
mod ensemble;
pub mod flips;
#[cfg(test)]
mod outline_tests;
pub mod support;
#[cfg(test)]
pub(crate) mod test_spine;

use crate::LayoutError;
use crate::curves::{Reading, SpineReading, lumbar::PedicleOutlines};
use shapes::implant_layout::{LayoutInput, LevelRule, RuleEnd, RuleInput};
use shapes::levels::Level;
use shapes::spine_fit::SpineFit;

pub(crate) use ensemble::measure_curves;
pub use ensemble::{LevelReadings, NominalReadings, RegionReadings, measure, nominal};
pub use flips::apply_flip;

/// Which curves supply the ends. A selective region still includes its
/// structural proximal thoracic curve when one is present.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FusionRegion {
    Structural,
    Thoracic,
    Lumbar,
}

/// Geometry, films and finite outlines of exactly one version. A caller
/// supplying explicit outlines owns their provenance; [`measure`] supplies
/// outlines from the published pedicle model, in every version separately.
#[derive(Clone, Copy)]
pub struct RuleContext<'a> {
    pub input: &'a LayoutInput,
    pub fit: &'a SpineFit,
    pub reading: &'a SpineReading,
    pub outlines: &'a [PedicleOutlines],
    pub region: FusionRegion,
}

/// A rule's complete result in one version, before counts are collected.
#[derive(Clone, Debug, PartialEq)]
pub struct RuleReading {
    pub rule: LevelRule,
    pub end: RuleEnd,
    pub level: Reading<Level>,
    pub inputs: Vec<RuleInput>,
    pub sources: Vec<String>,
}

/// Evaluate every rule which proposes an end. LIV-index is a check on a
/// candidate pair instead; selective criteria have their own readings.
pub fn evaluate(context: &RuleContext<'_>) -> Result<Vec<RuleReading>, LayoutError> {
    let mut rules = vec![
        end_plus_one::evaluate(context)?,
        suk_neutral::evaluate(context)?,
        last_touched::evaluate(context)?,
        stable_minus_one::evaluate(context)?,
        substantially_touched::evaluate(context)?,
        lenke_l3_l4::evaluate(context)?,
        lev_plus_one_5c::evaluate(context)?,
        deviated_vertebra::evaluate(context)?,
        upper_end_vertebra::evaluate(context)?,
        uev_5c::evaluate(context)?,
        lenke_shoulder::evaluate(context)?,
    ];
    for rule in &mut rules {
        if matches!(
            rule.rule,
            LevelRule::LastTouched | LevelRule::DeviatedVertebra | LevelRule::LenkeShoulder
        ) {
            // LastTouched supplies its body-contact angles directly;
            // it does not read the curve-end classification inputs below.
            continue;
        }
        // End detection reads competing extrema as well as the selected
        // end. Retain all those inputs so the nearest flip can be at a
        // different vertebra, with that vertebra's own marginal SD.
        for v in &context.reading.vertebrae {
            let value = support::input(
                context,
                shapes::implant_layout::Measurement::CoronalTilt,
                Some(v.level),
                None,
                Ok(v.upper_tilt_deg),
                None,
            );
            if !rule
                .inputs
                .iter()
                .any(|i| i.measurement == value.measurement && i.level == value.level)
            {
                rule.inputs.push(value);
            }
        }
        for curve in &context.reading.curves {
            let value = support::input(
                context,
                shapes::implant_layout::Measurement::Cobb,
                Some(context.reading.vertebrae[curve.apex_index].level),
                curve.region,
                Ok(curve.cobb_deg),
                None,
            );
            if !rule
                .inputs
                .iter()
                .any(|i| i.measurement == value.measurement && i.region == value.region)
            {
                rule.inputs.push(value);
            }
        }
        // Regional sagittal tests decide which minor curves supply the
        // ends. They are rule inputs even when the current minor is not
        // structural; an absent side film retains a missing value.
        for (measurement, value, group, key) in [
            (
                shapes::implant_layout::Measurement::KyphosisT2T5,
                context.reading.kyphosis_t2_t5_deg,
                "structural-minor",
                "kyphosis-t2-t5-min",
            ),
            (
                shapes::implant_layout::Measurement::KyphosisT10L2,
                context.reading.kyphosis_t10_l2_deg,
                "structural-minor",
                "kyphosis-t10-l2-min",
            ),
        ] {
            if !rule.inputs.iter().any(|i| i.measurement == measurement) {
                let (threshold, source) = support::threshold(group, key)?;
                rule.inputs.push(support::input(
                    context,
                    measurement,
                    None,
                    None,
                    value,
                    Some(threshold),
                ));
                rule.sources.push(source);
            }
        }
        rule.sources.sort();
        rule.sources.dedup();
    }
    Ok(rules)
}
