//! Version-specific outlines, rule counts and selective regions.

use super::selective::{self, SelectiveVersion};
use super::{FusionRegion, RuleContext, RuleReading, evaluate};
use crate::LayoutError;
use crate::curves::{self, SpineReading, lumbar::PedicleOutlines};
use crate::pedicles;
use crate::readings::{self, NamedSampler, Version, VersionSet};
use forward_model::pedicles::pedicle_centres;
use forward_model::rotation::matrix_from_rotation_vector;
use forward_model::vertebra_params::VertebraParams;
use shapes::implant_layout::{
    CannotTell, CannotTellCode, LayoutInput, LevelRule, RuleCounts, RuleVote, SelectiveReading,
    UnreadCount,
};
use shapes::spine_readings::{BodySide, LevelCount};
use std::collections::{BTreeMap, BTreeSet};

/// Every rule and film-dependent input remains available per version.
/// `votes` counts the structural region. Additional regions have separate
/// counts, so offering two regions never doubles a rule's N versions.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelReadings {
    pub nominal: SpineReading,
    pub measured: Vec<SpineReading>,
    pub versions: VersionSet,
    pub nominal_outlines: Vec<PedicleOutlines>,
    pub outlines: Vec<Vec<PedicleOutlines>>,
    pub nominal_rules: Vec<RuleReading>,
    pub version_rules: Vec<Vec<RuleReading>>,
    pub votes: Vec<RuleVote>,
    pub selective: Vec<SelectiveReading>,
    /// Thoracic then lumbar, in each version's order.
    pub selective_versions: Vec<[SelectiveVersion; 2]>,
    pub regions: Vec<RegionReadings>,
    pub cannot_tell: Vec<CannotTell>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RegionReadings {
    pub region: FusionRegion,
    pub nominal_rules: Vec<RuleReading>,
    pub version_rules: Vec<Vec<RuleReading>>,
    pub votes: Vec<RuleVote>,
}

/// Unperturbed geometry with central model outlines and no flip search.
#[derive(Clone, Debug, PartialEq)]
pub struct NominalReadings {
    pub reading: SpineReading,
    pub outlines: Vec<PedicleOutlines>,
    pub rules: Vec<RuleReading>,
}

pub fn nominal(input: &LayoutInput) -> Result<NominalReadings, LayoutError> {
    let versions = readings::versions(
        &input.fit,
        u64::from(input.seed),
        input.versions,
        0..input.versions,
    )?;
    let sampler = NamedSampler::new(u64::from(input.seed), input.versions)?;
    let (reading, outlines, sources) = nominal_snapshot(input, &versions.versions, &sampler)?;
    let context = RuleContext {
        input,
        fit: &input.fit,
        reading: &reading,
        outlines: &outlines,
        region: FusionRegion::Structural,
    };
    let mut rules = evaluate(&context)?;
    add_model_sources(&mut rules, &sources);
    Ok(NominalReadings {
        reading,
        outlines,
        rules,
    })
}

/// Read the complete ensemble. Geometry and pedicle draws have exactly the
/// identities exported by `readings`; this function does not sample again.
pub fn measure(input: &LayoutInput) -> Result<LevelReadings, LayoutError> {
    let versions = readings::versions(
        &input.fit,
        u64::from(input.seed),
        input.versions,
        0..input.versions,
    )?;
    let sampler = NamedSampler::new(u64::from(input.seed), input.versions)?;
    let (nominal, nominal_outlines, nominal_sources) =
        nominal_snapshot(input, &versions.versions, &sampler)?;
    let mut measured = Vec::new();
    let mut outlines = Vec::new();
    let mut model_sources = Vec::new();
    for version in &versions.versions {
        let raw = curves::measure(input, &version.fit)?;
        let (outline, sources) = model_outlines(input, version, &raw, &sampler)?;
        let mut reading = measure_lumbar_against_table_widths(input, &version.fit, &outline)?;
        reading.sources.extend(sources.iter().cloned());
        reading.sources.sort();
        reading.sources.dedup();
        measured.push(reading);
        outlines.push(outline);
        model_sources.push(sources);
    }
    let context = |region| RuleContext {
        input,
        fit: &input.fit,
        reading: &nominal,
        outlines: &nominal_outlines,
        region,
    };
    let mut nominal_rules = evaluate(&context(FusionRegion::Structural))?;
    add_model_sources(&mut nominal_rules, &nominal_sources);
    let mut version_rules = Vec::new();
    let mut selective_versions = Vec::new();
    for (i, version) in versions.versions.iter().enumerate() {
        let ctx = RuleContext {
            input,
            fit: &version.fit,
            reading: &measured[i],
            outlines: &outlines[i],
            region: FusionRegion::Structural,
        };
        let mut rules = evaluate(&ctx)?;
        add_model_sources(&mut rules, &model_sources[i]);
        version_rules.push(rules);
        selective_versions.push([
            super::selective_thoracic::evaluate(&ctx)?,
            super::selective_lumbar::evaluate(&ctx)?,
        ]);
    }
    let selective: Vec<_> = (0..2)
        .map(|index| {
            selective::summarise(
                &selective_versions
                    .iter()
                    .map(|s| s[index].clone())
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    let votes = counts(&nominal_rules, &version_rules);
    let mut regions = Vec::new();
    for region in [
        FusionRegion::Structural,
        FusionRegion::Thoracic,
        FusionRegion::Lumbar,
    ] {
        if region == FusionRegion::Structural {
            regions.push(RegionReadings {
                region,
                nominal_rules: nominal_rules.clone(),
                version_rules: version_rules.clone(),
                votes: votes.clone(),
            });
            continue;
        }
        let index = usize::from(region == FusionRegion::Lumbar);
        let offered = match selective[index].all_hold {
            shapes::implant_layout::Readable::Share { versions: holds } => {
                (f64::from(holds) / f64::from(input.versions)) >= selective::BOTH_REGIONS_SHARE[0]
            }
            _ => false,
        };
        if !offered {
            continue;
        }
        let mut nominal_region = evaluate(&context(region))?;
        add_model_sources(&mut nominal_region, &nominal_sources);
        let mut per_version = Vec::new();
        for (i, version) in versions.versions.iter().enumerate() {
            let ctx = RuleContext {
                input,
                fit: &version.fit,
                reading: &measured[i],
                outlines: &outlines[i],
                region,
            };
            let mut rules = evaluate(&ctx)?;
            add_model_sources(&mut rules, &model_sources[i]);
            per_version.push(rules);
        }
        let votes = counts(&nominal_region, &per_version);
        regions.push(RegionReadings {
            region,
            nominal_rules: nominal_region,
            version_rules: per_version,
            votes,
        });
    }
    let mut result = LevelReadings {
        nominal,
        measured,
        versions,
        nominal_outlines,
        outlines,
        nominal_rules,
        version_rules,
        votes,
        selective,
        selective_versions,
        regions,
        cannot_tell: Vec::new(),
    };
    // Flips use one nominal reading, with the model's central silhouettes.
    // The final candidate is checked again through the complete reader.
    for i in 0..result.votes.len() {
        result.votes[i].flip = super::flips::find(
            input,
            &result,
            &result.nominal_rules[i],
            FusionRegion::Structural,
        )?;
    }
    result.regions[0].votes = result.votes.clone();
    for region_index in 1..result.regions.len() {
        for rule_index in 0..result.regions[region_index].votes.len() {
            let rule = result.regions[region_index].votes[rule_index].rule;
            if matches!(
                rule,
                LevelRule::LastTouched
                    | LevelRule::SubstantiallyTouched
                    | LevelRule::DeviatedVertebra
                    | LevelRule::LenkeShoulder
            ) {
                // These rules read the same whole-spine geometry and no
                // fusion-region boundary. Their nominal inputs and every
                // perturbed answer are identical across offered regions.
                result.regions[region_index].votes[rule_index].flip = result.votes[rule_index].flip;
                continue;
            }
            let flip = super::flips::find(
                input,
                &result,
                &result.regions[region_index].nominal_rules[rule_index],
                result.regions[region_index].region,
            )?;
            result.regions[region_index].votes[rule_index].flip = flip;
        }
    }
    result.cannot_tell = cannot_tell(&result);
    Ok(result)
}

pub(crate) fn counts(nominal: &[RuleReading], versions: &[Vec<RuleReading>]) -> Vec<RuleVote> {
    nominal
        .iter()
        .map(|rule| {
            let mut levels = BTreeMap::new();
            let mut unread = BTreeMap::new();
            let mut sources = rule.sources.clone();
            for version in versions {
                if let Some(reading) = version.iter().find(|r| r.rule == rule.rule) {
                    match reading.level {
                        Ok(level) => *levels.entry(level).or_insert(0) += 1,
                        Err(reason) => *unread.entry(reason).or_insert(0) += 1,
                    };
                    sources.extend(reading.sources.iter().cloned());
                }
            }
            sources.sort();
            sources.dedup();
            RuleVote {
                rule: rule.rule,
                end: rule.end,
                counts: RuleCounts {
                    levels: levels
                        .into_iter()
                        .map(|(level, versions)| LevelCount { level, versions })
                        .collect(),
                    unread: unread
                        .into_iter()
                        .map(|(reason, versions)| UnreadCount { reason, versions })
                        .collect(),
                },
                inputs: rule.inputs.clone(),
                flip: None,
                sources,
            }
        })
        .collect()
}

fn add_model_sources(rules: &mut [RuleReading], sources: &[String]) {
    for rule in rules {
        if matches!(
            rule.rule,
            LevelRule::SubstantiallyTouched
                | LevelRule::StableMinusOne
                | LevelRule::Uev5C
                | LevelRule::LevPlusOne5C
        ) {
            rule.sources.extend(sources.iter().cloned());
            rule.sources.sort();
            rule.sources.dedup();
        }
    }
}

/// This is the single scale-sensitive boundary for the lumbar modifier:
/// its comparison incorporates widths in millimetres from a table.
pub(crate) fn measure_lumbar_against_table_widths(
    input: &LayoutInput,
    fit: &shapes::spine_fit::SpineFit,
    outlines: &[PedicleOutlines],
) -> Result<SpineReading, LayoutError> {
    let mut reading = curves::measure(input, fit)?;
    classify_lumbar_against_table_widths(&mut reading, fit, outlines)?;
    Ok(reading)
}

pub(crate) fn classify_lumbar_against_table_widths(
    reading: &mut SpineReading,
    fit: &shapes::spine_fit::SpineFit,
    outlines: &[PedicleOutlines],
) -> Result<(), LayoutError> {
    curves::classification::classify_with_pedicle_outlines(reading, fit, outlines)?;
    if reading.csvl_y_mm.is_ok()
        && let Ok(apex) = curves::lumbar::apex(reading, fit)
        && !outlines
            .iter()
            .any(|o| o.level == fit.vertebrae[apex].level)
    {
        reading.classification.lumbar_modifier = Err(CannotTellCode::NoPedicleTable);
    }
    Ok(())
}

/// The nominal geometry uses the median of the model's width draws at
/// that geometry. This project's central-outline convention preserves
/// all per-version widths and introduces no duplicate pedicle model.
pub(crate) fn nominal_snapshot(
    input: &LayoutInput,
    versions: &[Version],
    sampler: &NamedSampler,
) -> Result<(SpineReading, Vec<PedicleOutlines>, Vec<String>), LayoutError> {
    let raw = curves::measure(input, &input.fit)?;
    let mut by_level: BTreeMap<_, (Vec<f64>, Vec<f64>)> = BTreeMap::new();
    let mut sources = BTreeSet::new();
    for version in versions {
        let mut nominal_version = version.clone();
        nominal_version.fit = input.fit.clone();
        let (outlines, keys) = model_outlines(input, &nominal_version, &raw, sampler)?;
        sources.extend(keys);
        for outline in outlines {
            let widths = by_level.entry(outline.level).or_default();
            widths.0.push(outline.left_y_mm[1] - outline.left_y_mm[0]);
            widths.1.push(outline.right_y_mm[1] - outline.right_y_mm[0]);
        }
    }
    let mut outlines = Vec::new();
    for v in &input.fit.vertebrae {
        if let Some((left, right)) = by_level
            .get(&v.level)
            .filter(|(a, b)| a.len() == versions.len() && b.len() == versions.len())
        {
            let centres = pedicle_centres(&VertebraParams::from(&v.pose));
            let half_left = curves::summary::band(left)
                .expect("nonempty finite widths")
                .p50
                * 0.5;
            let half_right = curves::summary::band(right)
                .expect("nonempty finite widths")
                .p50
                * 0.5;
            outlines.push(PedicleOutlines {
                level: v.level,
                left_y_mm: [centres[0].y - half_left, centres[0].y + half_left],
                right_y_mm: [centres[1].y - half_right, centres[1].y + half_right],
            });
        }
    }
    let sources: Vec<_> = sources.into_iter().collect();
    let mut reading = measure_lumbar_against_table_widths(input, &input.fit, &outlines)?;
    reading.sources.extend(sources.iter().cloned());
    reading.sources.sort();
    reading.sources.dedup();
    Ok((reading, outlines, sources))
}

pub(super) fn model_outlines(
    input: &LayoutInput,
    version: &Version,
    reading: &SpineReading,
    sampler: &NamedSampler,
) -> Result<(Vec<PedicleOutlines>, Vec<String>), LayoutError> {
    let mut outlines = Vec::new();
    let mut sources = BTreeSet::new();
    for v in &version.fit.vertebrae {
        let mut widths = Vec::new();
        for side in [BodySide::Left, BodySide::Right] {
            let position = pedicles::position_in_curve(reading, v.level, side);
            let draw = pedicles::draw_width(
                v.level,
                side,
                input.child.age_years,
                position,
                version,
                sampler,
                input.pedicle_child_share,
            )?;
            if let Some(draw) = draw {
                sources.extend(draw.sources);
                widths.push(draw.outer_mm);
            }
        }
        if widths.len() != 2 {
            continue;
        }
        let params = VertebraParams::from(&v.pose);
        let centres = pedicle_centres(&params);
        // A table supplies transverse width only. This project's outline
        // is that transverse segment projected into patient y, rather
        // than an invented pedicle height or a body-size multiplier.
        let projection = matrix_from_rotation_vector(&params.rotation)[(1, 1)].abs();
        if projection <= f64::EPSILON {
            continue;
        }
        let halves = [widths[0] * 0.5 * projection, widths[1] * 0.5 * projection];
        outlines.push(PedicleOutlines {
            level: v.level,
            left_y_mm: [centres[0].y - halves[0], centres[0].y + halves[0]],
            right_y_mm: [centres[1].y - halves[1], centres[1].y + halves[1]],
        });
    }
    Ok((outlines, sources.into_iter().collect()))
}

fn cannot_tell(result: &LevelReadings) -> Vec<CannotTell> {
    let mut missing: BTreeMap<CannotTellCode, BTreeSet<shapes::levels::Level>> = BTreeMap::new();
    for rules in &result.version_rules {
        for rule in rules {
            if let Err(reason) = rule.level {
                missing
                    .entry(reason)
                    .or_default()
                    .extend(rule.inputs.iter().filter_map(|i| i.level));
            }
        }
    }
    for version in &result.selective_versions {
        for selective in version {
            for criterion in &selective.criteria {
                if let Err(reason) = criterion.value {
                    missing.entry(reason).or_default();
                }
            }
        }
    }
    for reading in &result.measured {
        if let Err(reason) = reading.classification.lumbar_modifier {
            missing.entry(reason).or_default();
        }
    }
    missing
        .into_iter()
        .map(|(code, levels)| CannotTell {
            code,
            levels: levels.into_iter().collect(),
        })
        .collect()
}
