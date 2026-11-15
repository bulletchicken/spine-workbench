//! Join measured versions, rule counts, table sizes and contours. No
//! sampling stream is advanced while alternatives are constructed.

mod anchors;
mod catalogue;
mod naming;
pub(crate) mod rebuild;

use crate::LayoutError;
use crate::alternatives::{self, Alternative};
use crate::curves::{SpineReading, summary};
use crate::level_rules::{self, LevelReadings};
use crate::patterns::Pattern;
use crate::readings::Version;
use crate::{pairs, rods, screws};
use shapes::implant_layout::{
    AnchorProposal, CannotTell, CannotTellCode, ImplantLayout, LayoutChange, LayoutInput,
    LayoutProposals,
};
use shapes::levels::Level;
use shapes::spine_readings::FitTier;
use std::collections::{BTreeMap, BTreeSet};

/// When the first pair that can be laid out holds in fewer than one version
/// in ten, the sketch from the main curve replaces it if that sketch can be
/// laid out; otherwise the pair stays. One in ten is this project's own
/// threshold, the same tenth a range leaves out at each end.
const SKETCH_HOLDING_DENOMINATOR: u64 = 10;

fn needs_sketch(holding: u32, versions: u32) -> bool {
    u64::from(holding) * SKETCH_HOLDING_DENOMINATOR < u64::from(versions)
}

/// Compute the complete ensemble once; construct all placements from those
/// same draws. Missing rule inputs never become substitute votes.
pub fn propose(input: &LayoutInput) -> Result<LayoutProposals, LayoutError> {
    validate(input)?;
    let working = registered_input(input);
    let readings = measure(&working)?;
    assemble(&working, &readings)
}

/// Assemble complete level readings. Global version order is preserved
/// regardless of how the underlying versions were computed in batches.
pub fn assemble(
    input: &LayoutInput,
    readings: &LevelReadings,
) -> Result<LayoutProposals, LayoutError> {
    validate(input)?;
    let working = registered_input(input);
    let input = &working;
    if readings.versions.total != input.versions
        || readings.versions.nominal != input.fit
        || readings.versions.versions.len() != input.versions as usize
        || readings.measured.len() != input.versions as usize
        || readings
            .versions
            .versions
            .iter()
            .enumerate()
            .any(|(i, v)| v.index != i as u32)
    {
        return Err(LayoutError::InvalidValue {
            field: "readings".into(),
            reason: "complete-ordered-ensemble-required".into(),
        });
    }
    let summary = summary::summarise(&readings.measured);
    let pair_readings = pairs::build(input, readings)?;
    let mut pairs: Vec<_> = pair_readings
        .pairs
        .into_iter()
        .filter(|p| alternatives::levels(input, p.upper, p.lower).is_some())
        .collect();
    let mut cannot_tell = summary.cannot_tell;
    cannot_tell.extend(readings.cannot_tell.iter().cloned());
    let mut fallback = pairs.is_empty();
    if fallback {
        pairs = fallback_pairs(input, readings);
    }
    cannot_tell.extend(screws::context_looks(input, None));
    let mut layouts = Vec::new();
    if !pairs.is_empty() {
        let rod = rods::default_choice()?;
        let mut anchors = BTreeMap::new();
        'choose: while let Some(pair) = if fallback {
            pairs.first()
        } else {
            pairs::first_pair(&pairs)
        } {
            let first = Alternative {
                id: "first".into(),
                change: LayoutChange::First {},
                upper: pair.upper,
                lower: pair.lower,
                pattern: Pattern::AllPedicles,
                rod: rod.clone(),
            };
            let first = match layout(input, readings, &mut anchors, &first) {
                Ok(layout) => layout,
                Err(LayoutError::Refused(_)) => {
                    let (upper, lower) = (pair.upper, pair.lower);
                    pairs.retain(|p| p.upper != upper || p.lower != lower);
                    continue;
                }
                Err(error) => return Err(error),
            };
            if !fallback && needs_sketch(pair.versions_holding, input.versions) {
                let mut sketches = fallback_pairs(input, readings);
                while let Some(sketch) = sketches.first() {
                    let alternative = Alternative {
                        id: "first".into(),
                        change: LayoutChange::First {},
                        upper: sketch.upper,
                        lower: sketch.lower,
                        pattern: Pattern::AllPedicles,
                        rod: rod.clone(),
                    };
                    match layout(input, readings, &mut anchors, &alternative) {
                        Ok(_) => {
                            pairs = sketches;
                            fallback = true;
                            continue 'choose;
                        }
                        Err(LayoutError::Refused(_)) => {
                            sketches.remove(0);
                        }
                        Err(_) => break,
                    }
                }
            }
            if !fallback
                && let Some(unsettled) = pairs::unsettled_ends(
                    pair,
                    input.versions,
                    &pair_readings.upper_ends,
                    &pair_readings.lower_ends,
                )
            {
                cannot_tell.push(unsettled);
            }
            let alternatives = alternatives::one_step(input, &first, &rod);
            layouts.push(first);
            for alternative in alternatives {
                match layout(input, readings, &mut anchors, &alternative) {
                    Ok(layout) => layouts.push(layout),
                    Err(LayoutError::Refused(_)) => {}
                    Err(error) => return Err(error),
                }
            }
            break;
        }
    }
    if layouts.is_empty() {
        cannot_tell.push(CannotTell {
            code: CannotTellCode::NoLevelMeetsTheRule,
            levels: Vec::new(),
        });
    }
    if input.fit.side_film.is_none() {
        cannot_tell.push(CannotTell {
            code: CannotTellCode::NoSideFilm,
            levels: Vec::new(),
        });
    }
    let naming_scenarios = match layouts.first() {
        Some(first) => naming::scenarios(input, readings, first)?,
        None => Vec::new(),
    };
    Ok(LayoutProposals {
        first_is_sketch: fallback && !layouts.is_empty(),
        upper_ends: if fallback || layouts.is_empty() {
            Vec::new()
        } else {
            pair_readings.upper_ends
        },
        lower_ends: if fallback || layouts.is_empty() {
            Vec::new()
        } else {
            pair_readings.lower_ends
        },
        tier: if input.fit.side_film.is_some() {
            FitTier::TwoFilms
        } else {
            FitTier::FrontFilmOnly
        },
        versions: input.versions,
        curves: summary.curves,
        classification: summary.classification,
        selective: readings.selective.clone(),
        rules: readings.votes.clone(),
        pairs,
        layouts,
        choices: catalogue::choices(),
        cannot_tell: merge(cannot_tell),
        naming_scenarios,
    })
}

fn layout(
    input: &LayoutInput,
    readings: &LevelReadings,
    candidates: &mut BTreeMap<Level, Vec<AnchorProposal>>,
    alternative: &Alternative,
) -> Result<ImplantLayout, LayoutError> {
    let levels = alternatives::levels(input, alternative.upper, alternative.lower)
        .ok_or_else(|| LayoutError::Refused("instrumented-span-not-fitted".into()))?;
    let mut anchors = candidate_anchors(
        input,
        &readings.versions.versions,
        &readings.measured,
        candidates,
        &levels,
    )?;
    apply_pattern(
        &mut anchors,
        &levels,
        &readings.nominal,
        alternative.pattern,
    )?;
    let released_levels = released_levels(&anchors, &levels, alternative.lower);
    let rods = rods::propose(
        &input.fit,
        &levels,
        &readings.nominal,
        &anchors,
        &alternative.rod,
    )?;
    Ok(ImplantLayout {
        id: alternative.id.clone(),
        change: alternative.change,
        upper: alternative.upper,
        lower: alternative.lower,
        anchors,
        rods,
        released_levels,
    })
}

fn candidate_anchors(
    input: &LayoutInput,
    versions: &[Version],
    measured: &[SpineReading],
    candidates: &mut BTreeMap<Level, Vec<AnchorProposal>>,
    levels: &[Level],
) -> Result<Vec<AnchorProposal>, LayoutError> {
    let mut anchors = Vec::new();
    for &level in levels {
        if let std::collections::btree_map::Entry::Vacant(entry) = candidates.entry(level) {
            let pair = screws::anchors(input, versions, measured, &[level])?;
            entry.insert(pair);
        }
        anchors.extend(candidates[&level].iter().cloned());
    }
    Ok(anchors)
}

fn apply_pattern(
    anchors: &mut [AnchorProposal],
    levels: &[Level],
    nominal: &SpineReading,
    pattern: Pattern,
) -> Result<(), LayoutError> {
    let apex = nominal
        .major
        .map(|i| nominal.vertebrae[nominal.curves[i].apex_index].level);
    anchors::apply(
        anchors,
        levels,
        apex,
        nominal
            .major
            .and_then(|i| nominal.curves.get(i))
            .map(|c| c.convex),
        pattern,
    )
}

fn released_levels(anchors: &[AnchorProposal], levels: &[Level], lower: Level) -> Vec<Level> {
    levels
        .iter()
        .copied()
        .filter(|level| {
            *level != lower
                && anchors.iter().any(|a| {
                    a.level == *level && a.kind == shapes::implant_layout::AnchorKind::Screw
                })
        })
        .collect()
}

fn merge(values: Vec<CannotTell>) -> Vec<CannotTell> {
    let mut codes: BTreeMap<CannotTellCode, BTreeSet<Level>> = BTreeMap::new();
    for value in values {
        codes.entry(value.code).or_default().extend(value.levels);
    }
    codes
        .into_iter()
        .map(|(code, levels)| CannotTell {
            code,
            levels: levels.into_iter().collect(),
        })
        .collect()
}

fn validate(input: &LayoutInput) -> Result<(), LayoutError> {
    for (field, value) in [
        ("child.ageYears", input.child.age_years),
        ("child.weightKg", input.child.weight_kg),
        ("pedicleChildShare", input.pedicle_child_share),
    ] {
        if value.is_some_and(|v| {
            !v.is_finite()
                || v < 0.0
                || (field == "pedicleChildShare" && v > 1.0)
                || (field == "child.weightKg" && v == 0.0)
        }) {
            return Err(LayoutError::InvalidValue {
                field: field.into(),
                reason: "outside-range".into(),
            });
        }
    }
    Ok(())
}

// The shared geometry needs spacing to map pixel offsets into its drawn
// patient frame. On a typical-size fit this is a drawing registration,
// never a physical scale; dimensional rule guards still read FitLengths.
fn registered_input(input: &LayoutInput) -> LayoutInput {
    let mut working = input.clone();
    if let Some(front) = &mut working.front
        && front.geometry.mm_per_px.is_none()
        && input.fit.lengths == shapes::spine_fit::FitLengths::TypicalSize
    {
        front.geometry.mm_per_px = Some(input.fit.front_film.mm_per_px[0]);
        front.geometry.mm_per_px_vertical = Some(input.fit.front_film.mm_per_px[1]);
    }
    working
}

// When the rules give no pair on fitted levels, or the first pair that can
// be laid out holds in fewer than one version in ten, try this project's
// sketch from the nominal major curve plus one lower body where present.
// It replaces the thin pair only if it can be laid out. No unreadable rule
// receives a vote.
fn fallback_pairs(
    input: &LayoutInput,
    readings: &LevelReadings,
) -> Vec<shapes::implant_layout::LevelPair> {
    let Some(curve) = readings
        .nominal
        .major
        .and_then(|i| readings.nominal.curves.get(i))
    else {
        return Vec::new();
    };
    let upper = readings.nominal.vertebrae[curve.upper_index].level;
    let lower = readings.nominal.vertebrae[curve.lower_index].level;
    let names = input
        .front
        .as_ref()
        .map_or_else(
            || crate::level_rules::support::fit_template(&input.fit),
            |f| f.marks.naming.template,
        )
        .names();
    let Some(i) = names.iter().position(|&l| l == lower) else {
        return Vec::new();
    };
    let lower = names
        .get(i + 1)
        .copied()
        .filter(|l| *l <= Level::L5 && input.fit.vertebrae.iter().any(|v| v.level == *l))
        .unwrap_or(lower);
    if alternatives::levels(input, upper, lower).is_none() {
        return Vec::new();
    }
    let start = names.iter().position(|&l| l == lower).unwrap_or(i);
    names[start..]
        .iter()
        .copied()
        .take_while(|l| *l <= Level::L5)
        .filter(|&lower| alternatives::levels(input, upper, lower).is_some())
        .map(|lower| shapes::implant_layout::LevelPair {
            upper,
            lower,
            versions_holding: 0,
            rules_at: Vec::new(),
            rules_near: Vec::new(),
            rules_against: Vec::new(),
            stop_short: Vec::new(),
            checks: Vec::new(),
            free_segments_below: names
                .iter()
                .position(|&l| l == Level::L5)
                .unwrap_or(i)
                .saturating_sub(names.iter().position(|&l| l == lower).unwrap_or(i))
                as u32
                + 1,
        })
        .collect()
}

/// Read the complete ensemble using the same drawn-frame registration as
/// the entry point. Reuse this result with assemble after batching draws.
pub fn measure(input: &LayoutInput) -> Result<LevelReadings, LayoutError> {
    validate(input)?;
    level_rules::measure(&registered_input(input))
}

#[cfg(test)]
mod tests {
    use super::needs_sketch;

    #[test]
    fn fewer_than_one_in_ten_uses_integer_edges() {
        for (versions, holding, expected) in [
            (32, 3, true),
            (32, 4, false),
            (10, 0, true),
            (10, 1, false),
            (4, 0, true),
            (4, 1, false),
            (1, 0, true),
            (1, 1, false),
            (u32::MAX, u32::MAX, false),
        ] {
            assert_eq!(needs_sketch(holding, versions), expected);
        }
    }
}
