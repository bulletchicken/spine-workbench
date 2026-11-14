//! Candidate level pairs, population stop-short figures and the first
//! placement's ends. Unreadable rules never add a vote.

use crate::LayoutError;
use crate::evidence::{PublishedValue, Table};
use crate::level_rules::support::{
    cell, distance, fit_template, named_distance, shift, shift_on_fit, template,
};
use crate::level_rules::{
    self, FusionRegion, LevelReadings, RuleContext, RuleReading, end_plus_one, last_touched,
    liv_index, stable_minus_one, substantially_touched, suk_neutral,
};
use shapes::implant_layout::{
    CannotTell, CannotTellCode, LayoutInput, LevelPair, LevelRule, PairCheck, Readable, RuleEnd,
    RuleVote, StopShortOdds,
};
use shapes::levels::Level;
use shapes::made_up::CurveRegion;
use shapes::naming::Template;
use shapes::spine_fit::SpineFit;
use shapes::spine_readings::LevelCount;
use std::collections::{BTreeMap, BTreeSet};

/// This project's candidate neighbourhood on either side of a rule mode.
const CANDIDATE_RADIUS: usize = 1;
/// This project's maximum distance from every evaluable rule's mode.
const MAX_MODE_DISTANCE: usize = 2;
/// This project's exact one-level stop-short diagnostic. The attached odds
/// are published population figures, without a personal probability.
const STOP_SHORT_DISTANCE: i32 = 1;
/// This project's threshold for unsettled ends, out of the full ensemble.
pub const ENDS_HOLDING_SHARE: f64 = 0.6;

/// Pairs retain stable top-to-bottom order; `first` locates the first
/// placement's ends without renaming them as a clinical preference.
#[derive(Clone, Debug, PartialEq)]
pub struct PairReadings {
    pub pairs: Vec<LevelPair>,
    /// The compatible fusion region supplying each pair's rule counts
    /// and checks, in exactly the same order as `pairs`.
    pub regions: Vec<FusionRegion>,
    /// Marginal end counts from each version's one modal pair, most
    /// frequent first. Ties use cranial upper and caudal lower levels.
    pub upper_ends: Vec<LevelCount>,
    pub lower_ends: Vec<LevelCount>,
    pub first: Option<usize>,
    pub cannot_tell: Vec<CannotTell>,
}

fn mode(vote: &RuleVote) -> Option<Level> {
    vote.counts
        .levels
        .iter()
        .filter(|count| count.versions > 0)
        .max_by(|a, b| {
            a.versions.cmp(&b.versions).then_with(|| match vote.end {
                RuleEnd::Upper => b.level.cmp(&a.level),
                RuleEnd::Lower => a.level.cmp(&b.level),
            })
        })
        .map(|count| count.level)
}

fn modes(votes: &[RuleVote], end: RuleEnd) -> BTreeMap<LevelRule, Level> {
    votes
        .iter()
        .filter(|vote| vote.end == end)
        .filter_map(|vote| mode(vote).map(|level| (vote.rule, level)))
        .collect()
}

fn index(fit: &SpineFit, level: Level) -> Option<usize> {
    fit.vertebrae
        .iter()
        .position(|vertebra| vertebra.level == level)
}

/// Union of each evaluable rule's modal level and its anatomical
/// neighbours, retaining only fitted targets. An absent ordinary body
/// does not pull its next neighbour into the one-level candidate set.
/// The fit-only API infers extra thoracic count only from a fitted T13.
pub fn candidates(fit: &SpineFit, votes: &[RuleVote], end: RuleEnd) -> Vec<Level> {
    candidates_with_template(fit, votes, end, fit_template(fit))
}

fn candidates_with_template(
    fit: &SpineFit,
    votes: &[RuleVote],
    end: RuleEnd,
    template: Template,
) -> Vec<Level> {
    let modes: Vec<_> = modes(votes, end)
        .values()
        .copied()
        .filter(|&level| index(fit, level).is_some())
        .collect();
    let mut candidates = BTreeSet::new();
    for &mode in &modes {
        for delta in -(CANDIDATE_RADIUS as i32)..=CANDIDATE_RADIUS as i32 {
            if let Ok(level) = shift_on_fit(fit, template, mode, delta) {
                candidates.insert(level);
            }
        }
    }
    candidates
        .into_iter()
        .filter(|&candidate| {
            modes.iter().any(|&mode| {
                named_distance(template, mode, candidate)
                    .is_ok_and(|distance| distance.unsigned_abs() as usize <= MAX_MODE_DISTANCE)
            })
        })
        .collect()
}

/// Count anatomical discs down to the sacrum, including the last lumbar
/// body's disc. Missing fitted distal bodies never make this count lower.
fn free_segments_below(template: Template, lower: Level) -> Option<u32> {
    let names = template.names();
    let lower = names.iter().position(|&level| level == lower)?;
    let last = template.levels().last().copied()?;
    let last = names.iter().position(|&level| level == last)?;
    last.checked_sub(lower).map(|distance| distance as u32 + 1)
}

fn modal_end(rules: &[RuleReading], end: RuleEnd) -> Option<Level> {
    let mut seen = BTreeSet::new();
    let mut counts = BTreeMap::<Level, u32>::new();
    for rule in rules.iter().filter(|rule| rule.end == end) {
        if let Ok(level) = rule.level
            && seen.insert(rule.rule)
        {
            *counts.entry(level).or_default() += 1;
        }
    }
    counts
        .into_iter()
        .max_by(|(a, ac), (b, bc)| {
            ac.cmp(bc).then_with(|| match end {
                RuleEnd::Upper => b.cmp(a),
                RuleEnd::Lower => a.cmp(b),
            })
        })
        .map(|(level, _)| level)
}

/// The exact end modes in one version. Tied lower modes take the caudal
/// level; tied upper modes take the cranial level. Each rule counts once.
pub fn modal_pair(rules: &[RuleReading]) -> Option<(Level, Level)> {
    let upper = modal_end(rules, RuleEnd::Upper)?;
    let lower = modal_end(rules, RuleEnd::Lower)?;
    (upper < lower).then_some((upper, lower))
}

/// Pure pair construction, before attaching published odds and checks.
/// Only evaluable modes enter the candidate set and the three rule lists.
pub fn from_rules(
    fit: &SpineFit,
    votes: &[RuleVote],
    versions: &[Vec<RuleReading>],
) -> Vec<LevelPair> {
    from_rules_with_template(fit, votes, versions, fit_template(fit))
}

fn from_rules_with_template(
    fit: &SpineFit,
    votes: &[RuleVote],
    versions: &[Vec<RuleReading>],
    template: Template,
) -> Vec<LevelPair> {
    let upper_modes = modes(votes, RuleEnd::Upper);
    let lower_modes = modes(votes, RuleEnd::Lower);
    let upper = candidates_with_template(fit, votes, RuleEnd::Upper, template);
    let lower = candidates_with_template(fit, votes, RuleEnd::Lower, template);
    let mut holding = BTreeMap::<(Level, Level), u32>::new();
    for version in versions {
        if let Some(pair) = modal_pair(version) {
            *holding.entry(pair).or_default() += 1;
        }
    }
    let mut pairs = Vec::new();
    for &upper in &upper {
        for &lower in &lower {
            if upper >= lower {
                continue;
            }
            let Some(free_segments_below) = free_segments_below(template, lower) else {
                continue;
            };
            let mut pair = LevelPair {
                upper,
                lower,
                versions_holding: holding.get(&(upper, lower)).copied().unwrap_or(0),
                rules_at: Vec::new(),
                rules_near: Vec::new(),
                rules_against: Vec::new(),
                stop_short: Vec::new(),
                checks: Vec::new(),
                free_segments_below,
            };
            for (end, modes) in [(upper, &upper_modes), (lower, &lower_modes)] {
                for (&rule, &level) in modes {
                    if index(fit, level).is_none() {
                        continue;
                    }
                    let Ok(distance) = named_distance(template, level, end) else {
                        continue;
                    };
                    match distance.unsigned_abs() {
                        0 => pair.rules_at.push(rule),
                        1 => pair.rules_near.push(rule),
                        _ => pair.rules_against.push(rule),
                    }
                }
            }
            for list in [
                &mut pair.rules_at,
                &mut pair.rules_near,
                &mut pair.rules_against,
            ] {
                list.sort();
                list.dedup();
            }
            pairs.push(pair);
        }
    }
    pairs
}

fn first_order(a: &LevelPair, b: &LevelPair) -> std::cmp::Ordering {
    a.rules_at
        .len()
        .cmp(&b.rules_at.len())
        .then_with(|| a.versions_holding.cmp(&b.versions_holding))
        .then_with(|| a.lower.cmp(&b.lower))
        .then_with(|| b.upper.cmp(&a.upper))
}

/// Most exact modal-rule agreements, then versions holding, then caudal
/// lower end. A remaining tie takes the cranial upper end, this project's
/// deterministic final tie-break.
pub fn first_pair(pairs: &[LevelPair]) -> Option<&LevelPair> {
    pairs.iter().max_by(|a, b| first_order(a, b))
}

fn end_counts(holding: &BTreeMap<(Level, Level), u32>, end: RuleEnd) -> Vec<LevelCount> {
    let mut counts = BTreeMap::<Level, u32>::new();
    for (&(upper, lower), &versions) in holding {
        let level = if end == RuleEnd::Upper { upper } else { lower };
        *counts.entry(level).or_default() += versions;
    }
    let mut counts: Vec<_> = counts
        .into_iter()
        .map(|(level, versions)| LevelCount { level, versions })
        .collect();
    counts.sort_by(|a, b| {
        b.versions.cmp(&a.versions).then_with(|| match end {
            RuleEnd::Upper => a.level.cmp(&b.level),
            RuleEnd::Lower => b.level.cmp(&a.level),
        })
    });
    counts
}

/// Retain the pair's unsettled test, naming only the chosen ends held
/// in too few versions. If each chosen end is steady but their pair is
/// unsettled, both chosen ends are named.
pub fn unsettled_ends(
    pair: &LevelPair,
    versions: u32,
    upper_ends: &[LevelCount],
    lower_ends: &[LevelCount],
) -> Option<CannotTell> {
    let threshold = ENDS_HOLDING_SHARE * f64::from(versions);
    if f64::from(pair.versions_holding) >= threshold {
        return None;
    }
    let mut levels = Vec::new();
    for (end, counts) in [(pair.upper, upper_ends), (pair.lower, lower_ends)] {
        let held = counts
            .iter()
            .find(|count| count.level == end)
            .map_or(0, |count| count.versions);
        if f64::from(held) < threshold {
            levels.push(end);
        }
    }
    if levels.is_empty() {
        levels.extend([pair.upper, pair.lower]);
    }
    Some(CannotTell {
        code: CannotTellCode::EndsUnsettled,
        levels,
    })
}

fn context<'a>(
    input: &'a LayoutInput,
    readings: &'a LevelReadings,
    version: Option<usize>,
    region: FusionRegion,
) -> RuleContext<'a> {
    match version {
        Some(i) => RuleContext {
            input,
            fit: &readings.versions.versions[i].fit,
            reading: &readings.measured[i],
            outlines: &readings.outlines[i],
            region,
        },
        None => RuleContext {
            input,
            fit: &input.fit,
            reading: &readings.nominal,
            outlines: &readings.nominal_outlines,
            region,
        },
    }
}

fn offered_regions(input: &LayoutInput, readings: &LevelReadings) -> Vec<FusionRegion> {
    let mut selective = Vec::new();
    let mut structural = true;
    for (i, region) in [FusionRegion::Thoracic, FusionRegion::Lumbar]
        .into_iter()
        .enumerate()
    {
        let Some(reading) = readings.selective.get(i) else {
            continue;
        };
        let Readable::Share { versions: met } = reading.all_hold else {
            continue;
        };
        let share = f64::from(met) / f64::from(input.versions);
        if share >= level_rules::selective::BOTH_REGIONS_SHARE[0] {
            selective.push(region);
            if share > level_rules::selective::BOTH_REGIONS_SHARE[1] {
                structural = false;
            }
        }
    }
    if structural {
        selective.insert(0, FusionRegion::Structural);
    }
    selective
}

fn version_region(
    readings: &LevelReadings,
    index: usize,
    offered: &[FusionRegion],
) -> FusionRegion {
    let reading = &readings.measured[index];
    let major = reading
        .major
        .and_then(|i| reading.curves.get(i))
        .and_then(|curve| curve.region);
    let selective = match major {
        Some(CurveRegion::MainThoracic) => Some((0, FusionRegion::Thoracic)),
        Some(CurveRegion::ThoracolumbarLumbar) => Some((1, FusionRegion::Lumbar)),
        _ => None,
    };
    if let Some((criterion, region)) = selective
        && offered.contains(&region)
        && readings
            .selective_versions
            .get(index)
            .is_some_and(|row| row[criterion].all_hold == Ok(true))
    {
        return region;
    }
    FusionRegion::Structural
}

fn stop_short(ctx: &RuleContext<'_>, lower: Level) -> Result<Vec<StopShortOdds>, LayoutError> {
    let stable_cell = cell(Table::LevelRuleThresholds, "lenke-ab", "liv-above-stable")?;
    let PublishedValue::Range { lower: offset, .. } = stable_cell.value else {
        return Err(LayoutError::Refused(
            "non-range-evidence:lenke-ab:liv-above-stable".into(),
        ));
    };
    let stable = stable_minus_one::stable(ctx)
        .and_then(|i| shift(ctx, ctx.reading.vertebrae[i].level, -(offset as i32)));
    let end = end_plus_one::evaluate(ctx)?.level;
    let neutral = suk_neutral::neutral(ctx).map(|i| ctx.reading.vertebrae[i].level);
    let touched = last_touched::landmark(ctx).map(|i| ctx.reading.vertebrae[i].level);
    let substantial = substantially_touched::landmark(ctx).map(|i| ctx.reading.vertebrae[i].level);
    let landmarks = [
        (LevelRule::StableMinusOne, "sv-minus-one", stable),
        (LevelRule::EndPlusOne, "ev-plus-one", end),
        (LevelRule::SukNeutral, "nv", neutral),
        (LevelRule::LastTouched, "ltv", touched),
        (
            LevelRule::SubstantiallyTouched,
            "stv-or-nstv-plus-one",
            substantial,
        ),
    ];
    let mut odds = Vec::new();
    for (rule, group, landmark) in landmarks {
        let Ok(landmark) = landmark else {
            continue;
        };
        if distance(ctx, lower, landmark) != Ok(STOP_SHORT_DISTANCE) {
            continue;
        }
        let cell = cell(Table::StopShortOdds, group, "proximal-odds")?;
        let PublishedValue::Scalar { value } = cell.value else {
            return Err(LayoutError::Refused(format!(
                "non-scalar-evidence:{group}:proximal-odds"
            )));
        };
        odds.push(StopShortOdds {
            rule,
            landmark,
            odds_ratio: value,
            source: cell.source.clone(),
        });
    }
    Ok(odds)
}

fn pair_check(
    input: &LayoutInput,
    readings: &LevelReadings,
    lower: Level,
    region: FusionRegion,
) -> Result<PairCheck, LayoutError> {
    let mut flags = 0;
    let mut readable = false;
    let mut reason = CannotTellCode::NoCurve;
    let mut source = None;
    for i in 0..readings.versions.versions.len() {
        let check = liv_index::check(&context(input, readings, Some(i), region), lower)?;
        source = Some(check.source);
        match check.flagged {
            Readable::Share { versions } => {
                readable = true;
                flags += versions;
            }
            Readable::NotReadable { reason: code } => reason = code,
        }
    }
    let source = source.unwrap_or(
        cell(Table::LevelRuleThresholds, "liv-index", "liv-index-max")?
            .source
            .clone(),
    );
    Ok(PairCheck {
        rule: LevelRule::LivIndex,
        flagged: if readable {
            Readable::Share { versions: flags }
        } else {
            Readable::NotReadable { reason }
        },
        source,
    })
}

/// Build compatible pairs in each offered fusion region, then merge equal
/// pairs. The region with more exact rule modes supplies a merged pair's
/// rule lists and checks; a remaining tie follows stable region order.
/// A version contributes to exactly one modal pair, never to each offered
/// region separately. Stop-short odds remain published population figures.
pub fn build(input: &LayoutInput, readings: &LevelReadings) -> Result<PairReadings, LayoutError> {
    let offered = offered_regions(input, readings);
    let mut merged = BTreeMap::<(Level, Level), (LevelPair, FusionRegion)>::new();
    for &region in &offered {
        let Some(regional) = readings.regions.iter().find(|row| row.region == region) else {
            continue;
        };
        let declared = template(&context(input, readings, None, region));
        for pair in from_rules_with_template(&input.fit, &regional.votes, &[], declared) {
            let key = (pair.upper, pair.lower);
            let replace = merged
                .get(&key)
                .is_none_or(|(current, _)| pair.rules_at.len() > current.rules_at.len());
            if replace {
                merged.insert(key, (pair, region));
            }
        }
    }
    let mut holding = BTreeMap::<(Level, Level), u32>::new();
    for i in 0..readings.versions.versions.len() {
        let region = version_region(readings, i, &offered);
        if let Some(regional) = readings.regions.iter().find(|row| row.region == region)
            && let Some(pair) = regional
                .version_rules
                .get(i)
                .and_then(|rules| modal_pair(rules))
        {
            *holding.entry(pair).or_default() += 1;
        }
    }
    let upper_ends = end_counts(&holding, RuleEnd::Upper);
    let lower_ends = end_counts(&holding, RuleEnd::Lower);
    let mut pairs = Vec::new();
    let mut regions = Vec::new();
    for (key, (mut pair, region)) in merged {
        pair.versions_holding = holding.get(&key).copied().unwrap_or(0);
        pair.stop_short = stop_short(&context(input, readings, None, region), pair.lower)?;
        pair.checks = vec![pair_check(input, readings, pair.lower, region)?];
        pairs.push(pair);
        regions.push(region);
    }
    let first = first_pair(&pairs).map(|pair| {
        pairs
            .iter()
            .position(|candidate| candidate.upper == pair.upper && candidate.lower == pair.lower)
            .expect("first pair belongs to list")
    });
    let cannot_tell = match first {
        Some(i) => unsettled_ends(&pairs[i], input.versions, &upper_ends, &lower_ends)
            .into_iter()
            .collect(),
        None => vec![CannotTell {
            code: CannotTellCode::NoLevelMeetsTheRule,
            levels: Vec::new(),
        }],
    };
    Ok(PairReadings {
        pairs,
        regions,
        upper_ends,
        lower_ends,
        first,
        cannot_tell,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level_rules::test_spine::Fixture;
    use shapes::implant_layout::{LumbarModifier, SelectiveReading};

    #[test]
    fn separate_end_counts_keep_the_joint_count_and_name_only_unsettled_ends() {
        let pair = |versions_holding| LevelPair {
            upper: Level::T4,
            lower: Level::L3,
            versions_holding,
            rules_at: Vec::new(),
            rules_near: Vec::new(),
            rules_against: Vec::new(),
            stop_short: Vec::new(),
            checks: Vec::new(),
            free_segments_below: 3,
        };
        // Ten hand-counted versions: first both ends occur in six;
        // then one chosen end has five, then both have five. Both ends
        // can have seven while their exact pair has only four. A mode
        // elsewhere does not settle the chosen end.
        for (holding, expected) in [
            (
                vec![((Level::T4, Level::L3), 6), ((Level::T5, Level::L4), 4)],
                None,
            ),
            (
                vec![((Level::T4, Level::L3), 5), ((Level::T4, Level::L4), 5)],
                Some(vec![Level::L3]),
            ),
            (
                vec![((Level::T4, Level::L3), 5), ((Level::T5, Level::L3), 5)],
                Some(vec![Level::T4]),
            ),
            (
                vec![((Level::T4, Level::L3), 5), ((Level::T5, Level::L4), 5)],
                Some(vec![Level::T4, Level::L3]),
            ),
            (
                vec![
                    ((Level::T4, Level::L3), 4),
                    ((Level::T4, Level::L4), 3),
                    ((Level::T5, Level::L3), 3),
                ],
                Some(vec![Level::T4, Level::L3]),
            ),
            (
                vec![
                    ((Level::T4, Level::L3), 1),
                    ((Level::T5, Level::L3), 3),
                    ((Level::T5, Level::L4), 3),
                    ((Level::T5, Level::L5), 3),
                ],
                Some(vec![Level::T4, Level::L3]),
            ),
        ] {
            let holding: BTreeMap<_, _> = holding.into_iter().collect();
            let upper = end_counts(&holding, RuleEnd::Upper);
            let lower = end_counts(&holding, RuleEnd::Lower);
            let result =
                unsettled_ends(&pair(holding[&(Level::T4, Level::L3)]), 10, &upper, &lower);
            assert_eq!(result.map(|value| value.levels), expected);
            assert_eq!(upper.iter().map(|count| count.versions).sum::<u32>(), 10);
            assert_eq!(lower.iter().map(|count| count.versions).sum::<u32>(), 10);
            for counts in [&upper, &lower] {
                assert!(counts.windows(2).all(|w| w[0].versions >= w[1].versions));
            }
        }
        let holding = BTreeMap::from([
            ((Level::T4, Level::L3), 3),
            ((Level::T5, Level::L3), 2),
            ((Level::T5, Level::L4), 4),
        ]);
        assert_eq!(
            end_counts(&holding, RuleEnd::Upper),
            vec![
                LevelCount {
                    level: Level::T5,
                    versions: 6
                },
                LevelCount {
                    level: Level::T4,
                    versions: 3
                },
            ]
        );
        assert_eq!(
            end_counts(&holding, RuleEnd::Lower),
            vec![
                LevelCount {
                    level: Level::L3,
                    versions: 5
                },
                LevelCount {
                    level: Level::L4,
                    versions: 4
                },
            ]
        );
    }

    fn fixture() -> Fixture {
        let mut f = Fixture::new();
        let mut curve = f.reading.curves[f.reading.major.unwrap()].clone();
        curve.apex_index = 5;
        curve.lower_index = 10;
        f.reading.curves = vec![curve];
        f.reading.major = Some(0);
        f.reading.classification.lumbar_modifier = Ok(LumbarModifier::BetweenPedicles);
        for (i, vertebra) in f.reading.vertebrae.iter_mut().enumerate() {
            vertebra.twist_deg = Ok(if i == 14 { 0.0 } else { 5.0 });
            let y = match i {
                10 => 30.0,
                11 => 13.0,
                12 => 0.0,
                _ => 100.0,
            };
            vertebra.centre_mm[1] = y;
            vertebra.centre_line_offset_mm = Ok(y);
            f.input.fit.vertebrae[i].pose.centre[1] = y;
            f.input.fit.vertebrae[i].pose.rotation = [0.0; 3];
            f.input.fit.vertebrae[i].sd.centre_mm = [0.0; 3];
            f.input.fit.vertebrae[i].sd.angles_rad = [0.0; 3];
            f.outlines[i].left_y_mm = [y + 8.0, y + 12.0];
            f.outlines[i].right_y_mm = [y - 12.0, y - 8.0];
        }
        f
    }

    fn readings(f: &Fixture) -> LevelReadings {
        let versions = crate::readings::versions(
            &f.input.fit,
            u64::from(f.input.seed),
            f.input.versions,
            0..f.input.versions,
        )
        .unwrap();
        let count = versions.versions.len();
        LevelReadings {
            nominal: f.reading.clone(),
            measured: vec![f.reading.clone(); count],
            versions,
            nominal_outlines: f.outlines.clone(),
            outlines: vec![f.outlines.clone(); count],
            nominal_rules: Vec::new(),
            version_rules: Vec::new(),
            votes: Vec::new(),
            selective: Vec::new(),
            selective_versions: Vec::new(),
            regions: Vec::new(),
            cannot_tell: Vec::new(),
        }
    }

    #[test]
    fn stop_short_figures_use_the_landmark_itself_and_keep_the_published_source() {
        // Made-up landmark arithmetic, independent of the pair builder:
        // EV=T11 -> EV+1=T12; SV=L1 -> SV-1=T12; first TL/L body
        // touched is T12 (offset13<half-width15). Its pedicles end at12,
        // so STV is the next body L1. Measured NV is L3. Thus LIV T11
        // is one short of the first three references; T12 is one short
        // of STV; L2 is one short of raw NV, not of Suk's NV-1 output.
        let f = fixture();
        let ctx = f.context();
        let odds = stop_short(&ctx, Level::T11).unwrap();
        assert_eq!(
            odds.iter().map(|odds| odds.rule).collect::<Vec<_>>(),
            vec![
                LevelRule::StableMinusOne,
                LevelRule::EndPlusOne,
                LevelRule::LastTouched
            ]
        );
        assert!(odds.iter().all(|odds| odds.landmark == Level::T12));
        for (lower, rule, group, landmark) in [
            (
                Level::T12,
                LevelRule::SubstantiallyTouched,
                "stv-or-nstv-plus-one",
                Level::L1,
            ),
            (Level::L2, LevelRule::SukNeutral, "nv", Level::L3),
        ] {
            let odds = stop_short(&ctx, lower).unwrap();
            assert_eq!(odds.len(), 1);
            assert_eq!(odds[0].rule, rule);
            assert_eq!(odds[0].landmark, landmark);
            let source = cell(Table::StopShortOdds, group, "proximal-odds").unwrap();
            let PublishedValue::Scalar { value } = source.value else {
                panic!("scalar odds");
            };
            assert_eq!(odds[0].odds_ratio, value);
            assert_eq!(odds[0].source, source.source);
        }
        assert!(
            !stop_short(&ctx, Level::T12)
                .unwrap()
                .iter()
                .any(|odds| odds.rule == LevelRule::EndPlusOne)
        );
        assert!(
            !stop_short(&ctx, Level::T10)
                .unwrap()
                .iter()
                .any(|odds| odds.rule == LevelRule::EndPlusOne)
        );
    }

    #[test]
    fn a_pair_check_counts_flags_and_leaves_unread_versions_out() {
        let f = fixture();
        let mut rows = readings(&f);
        // At LIV L1, NV L3 and SV L1 give index -2 in rows0,1.
        // Row2 moves NV to T11: +2. Row3 has unknown caudal twist.
        // Two of the four versions flag; the unread row never flags.
        for vertebra in &mut rows.measured[2].vertebrae {
            vertebra.twist_deg = Ok(5.0);
        }
        rows.measured[2].vertebrae[10].twist_deg = Ok(0.0);
        rows.measured[3].vertebrae[16].twist_deg = Err(CannotTellCode::TwistNotMeasured);
        assert_eq!(
            pair_check(&f.input, &rows, Level::L1, FusionRegion::Structural)
                .unwrap()
                .flagged,
            Readable::Share { versions: 2 }
        );
        for reading in &mut rows.measured {
            reading.csvl_y_mm = Err(CannotTellCode::NoSacrum);
        }
        assert_eq!(
            pair_check(&f.input, &rows, Level::L1, FusionRegion::Structural)
                .unwrap()
                .flagged,
            Readable::NotReadable {
                reason: CannotTellCode::NoSacrum
            }
        );
    }

    #[test]
    fn the_two_region_offer_includes_both_share_boundaries() {
        let mut f = fixture();
        let mut rows = readings(&f);
        f.input.versions = 10;
        for (holds, expected) in [
            (1, vec![FusionRegion::Structural]),
            (2, vec![FusionRegion::Structural, FusionRegion::Thoracic]),
            (8, vec![FusionRegion::Structural, FusionRegion::Thoracic]),
            (9, vec![FusionRegion::Thoracic]),
        ] {
            rows.selective = vec![SelectiveReading {
                rule: LevelRule::SelectiveThoracic,
                criteria: Vec::new(),
                all_hold: Readable::Share { versions: holds },
                both_regions_offered: (2..=8).contains(&holds),
                sources: Vec::new(),
            }];
            assert_eq!(offered_regions(&f.input, &rows), expected);
        }
    }

    #[test]
    fn two_offered_regions_still_count_each_version_only_once() {
        use crate::level_rules::selective::SelectiveVersion;
        use shapes::implant_layout::RuleCounts;
        let f = fixture();
        let mut rows = readings(&f);
        // No angle SD remains in this hand fixture, so preparing the
        // empty regional record performs no sensitivity search.
        let regional = level_rules::measure(&f.input).unwrap().regions[0].clone();
        let rule = |rule, end, level| RuleReading {
            rule,
            end,
            level: Ok(level),
            inputs: Vec::new(),
            sources: Vec::new(),
        };
        let vote = |rule, end, level| RuleVote {
            rule,
            end,
            counts: RuleCounts {
                levels: vec![LevelCount { level, versions: 4 }],
                unread: Vec::new(),
            },
            inputs: Vec::new(),
            flip: None,
            sources: Vec::new(),
        };
        let mut structural = regional.clone();
        structural.region = FusionRegion::Structural;
        structural.nominal_rules = vec![
            rule(LevelRule::UpperEndVertebra, RuleEnd::Upper, Level::T4),
            rule(LevelRule::EndPlusOne, RuleEnd::Lower, Level::L3),
        ];
        structural.version_rules = vec![structural.nominal_rules.clone(); 4];
        structural.votes = vec![
            vote(LevelRule::UpperEndVertebra, RuleEnd::Upper, Level::T4),
            vote(LevelRule::EndPlusOne, RuleEnd::Lower, Level::L3),
        ];
        let mut thoracic = regional;
        thoracic.region = FusionRegion::Thoracic;
        thoracic.nominal_rules = vec![
            rule(LevelRule::UpperEndVertebra, RuleEnd::Upper, Level::T4),
            rule(LevelRule::EndPlusOne, RuleEnd::Lower, Level::T12),
        ];
        thoracic.version_rules = vec![thoracic.nominal_rules.clone(); 4];
        thoracic.votes = vec![
            vote(LevelRule::UpperEndVertebra, RuleEnd::Upper, Level::T4),
            vote(LevelRule::EndPlusOne, RuleEnd::Lower, Level::T12),
        ];
        rows.regions = vec![structural, thoracic];
        rows.selective = vec![SelectiveReading {
            rule: LevelRule::SelectiveThoracic,
            criteria: Vec::new(),
            all_hold: Readable::Share { versions: 2 },
            both_regions_offered: true,
            sources: Vec::new(),
        }];
        rows.selective_versions = (0..4)
            .map(|i| {
                [
                    SelectiveVersion {
                        rule: LevelRule::SelectiveThoracic,
                        criteria: Vec::new(),
                        all_hold: Ok(i < 2),
                    },
                    SelectiveVersion {
                        rule: LevelRule::SelectiveLumbar,
                        criteria: Vec::new(),
                        all_hold: Ok(false),
                    },
                ]
            })
            .collect();
        let result = build(&f.input, &rows).unwrap();
        assert_eq!(
            result.upper_ends,
            vec![LevelCount {
                level: Level::T4,
                versions: 4
            }]
        );
        assert_eq!(
            result.lower_ends,
            vec![
                LevelCount {
                    level: Level::L3,
                    versions: 2
                },
                LevelCount {
                    level: Level::T12,
                    versions: 2
                },
            ]
        );
        // Rows0,1 satisfy the readable thoracic selective criteria and
        // rows2,3 use the structural region: two counts at each exact
        // pair, four total, even though both regional sets were offered.
        assert_eq!(
            result
                .pairs
                .iter()
                .map(|pair| pair.versions_holding)
                .sum::<u32>(),
            4
        );
        for lower in [Level::T12, Level::L3] {
            assert_eq!(
                result
                    .pairs
                    .iter()
                    .find(|pair| pair.upper == Level::T4 && pair.lower == lower)
                    .unwrap()
                    .versions_holding,
                2
            );
        }
        assert_eq!(result.pairs[result.first.unwrap()].lower, Level::L3);
        assert_eq!(result.regions.len(), result.pairs.len());
        assert_eq!(
            result.regions[result.first.unwrap()],
            FusionRegion::Structural
        );
    }

    #[test]
    fn missing_bodies_do_not_shorten_candidates_or_rule_distances() {
        use shapes::implant_layout::RuleCounts;
        let mut f = fixture();
        f.input.fit.vertebrae.retain(|v| v.level != Level::T9);
        let vote = |rule, end, level| RuleVote {
            rule,
            end,
            counts: RuleCounts {
                levels: vec![LevelCount { level, versions: 4 }],
                unread: Vec::new(),
            },
            inputs: Vec::new(),
            flip: None,
            sources: Vec::new(),
        };
        let mut votes = vec![
            vote(LevelRule::UpperEndVertebra, RuleEnd::Upper, Level::T4),
            vote(LevelRule::EndPlusOne, RuleEnd::Lower, Level::T8),
        ];
        // With T9 unfitted, T10 is still two below T8. It is neither an
        // adjacent candidate nor near the T8 rule when another mode at
        // T10 brings that body into the candidate list.
        assert_eq!(
            candidates(&f.input.fit, &votes, RuleEnd::Lower),
            vec![Level::T7, Level::T8]
        );
        votes.push(vote(LevelRule::LastTouched, RuleEnd::Lower, Level::T10));
        let pairs = from_rules(&f.input.fit, &votes, &[]);
        let pair = pairs
            .iter()
            .find(|pair| pair.upper == Level::T4 && pair.lower == Level::T10)
            .unwrap();
        assert!(pair.rules_against.contains(&LevelRule::EndPlusOne));
        assert!(!pair.rules_near.contains(&LevelRule::EndPlusOne));
        // A truncated fit does not erase the known L3/4, L4/5 or L5/S1
        // discs. From anatomical L1 the normal count remains five.
        f.input.fit.vertebrae.retain(|v| v.level < Level::L4);
        votes[1] = vote(LevelRule::EndPlusOne, RuleEnd::Lower, Level::L1);
        let pairs = from_rules(&f.input.fit, &votes, &[]);
        assert_eq!(
            pairs
                .iter()
                .find(|pair| pair.lower == Level::L1)
                .unwrap()
                .free_segments_below,
            5
        );
    }

    #[test]
    fn pair_building_respects_an_explicit_missing_thoracic_count() {
        use shapes::implant_layout::RuleCounts;
        let mut f = fixture();
        f.input.fit.vertebrae.retain(|v| v.level != Level::T12);
        let vote = RuleVote {
            rule: LevelRule::EndPlusOne,
            end: RuleEnd::Lower,
            counts: RuleCounts {
                levels: vec![LevelCount {
                    level: Level::T11,
                    versions: 4,
                }],
                unread: Vec::new(),
            },
            inputs: Vec::new(),
            flip: None,
            sources: Vec::new(),
        };
        assert!(
            !candidates(&f.input.fit, std::slice::from_ref(&vote), RuleEnd::Lower)
                .contains(&Level::L1)
        );
        assert!(
            candidates_with_template(
                &f.input.fit,
                &[vote],
                RuleEnd::Lower,
                Template::MissingThoracic
            )
            .contains(&Level::L1)
        );
        assert_eq!(
            free_segments_below(Template::ExtraLumbar, Level::L1),
            Some(6)
        );
        assert_eq!(
            free_segments_below(Template::MissingLumbar, Level::L1),
            Some(4)
        );
    }
}
