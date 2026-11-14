//! Shares use the same per-version denominator across all placements and stops.

use super::*;
use shapes::implant_layout::AnchorKind;
use shapes::spine_readings::BodySide;

pub(super) fn spans(layout: &ImplantLayout) -> Vec<PartRef> {
    let mut result = Vec::new();
    for side in [BodySide::Left, BodySide::Right] {
        if !layout.rods.iter().any(|rod| rod.side == side) {
            continue;
        }
        let mut levels: Vec<_> = layout
            .anchors
            .iter()
            .filter(|anchor| anchor.side == side && anchor.kind != AnchorKind::None)
            .map(|anchor| anchor.level)
            .collect();
        levels.sort();
        levels.dedup();
        for pair in levels.windows(2) {
            result.push(PartRef::RodSpan {
                side,
                from: pair[0],
                to: pair[1],
            });
        }
    }
    result
}

fn kind(part: PartRef) -> usize {
    match part {
        PartRef::Anchor { .. } => 0,
        PartRef::RodSpan { .. } => 1,
        PartRef::Junction { .. } => 2,
        PartRef::Rod { .. } => 3,
    }
}

fn values(layout: &ImplantLayout, stop: &StopValues) -> Vec<(PartRef, f64, bool)> {
    let mut result: Vec<_> = layout
        .anchors
        .iter()
        .zip(&stop.anchor_force_n)
        .zip(&stop.anchor_demand)
        .map(|((anchor, force), demand)| {
            (
                PartRef::Anchor {
                    level: anchor.level,
                    side: anchor.side,
                },
                *force,
                *demand > 1.0,
            )
        })
        .collect();
    result.extend(
        spans(layout)
            .into_iter()
            .zip(&stop.rod_span_use)
            .map(|(part, value)| (part, *value, *value > 1.0)),
    );
    for (which, value) in [
        (JunctionSide::Above, stop.junction_above),
        (JunctionSide::Below, stop.junction_below),
    ] {
        if let Some(value) = value {
            result.push((PartRef::Junction { which }, value, false));
        }
    }
    if let Some(rods) = &stop.rod_swing {
        result.extend(rods.iter().filter_map(|rod| {
            rod.ratio
                .map(|value| (PartRef::Rod { side: rod.side }, value, value > 1.0))
        }));
    }
    result
}

pub(super) fn summarise(
    layout: &ImplantLayout,
    own: &Runs<'_>,
    all: &[Runs<'_>],
    stop: BenchStop,
) -> Vec<PartResult> {
    let mut denominators: BTreeMap<u32, [f64; 4]> = BTreeMap::new();
    let mut missing_rod_comparisons = BTreeSet::new();
    for runs in all {
        for (version, run) in runs.iter().filter(|(_, run)| run.settled) {
            let maximum = denominators.entry(*version).or_insert([0.0; 4]);
            for value in &run.stops {
                if value
                    .rod_swing
                    .as_ref()
                    .is_some_and(|rods| rods.iter().any(|rod| rod.ratio.is_none()))
                {
                    missing_rod_comparisons.insert(*version);
                }
                maximum[0] = maximum[0].max(largest(&value.anchor_force_n).unwrap_or(0.0));
                maximum[1] = maximum[1].max(largest(&value.rod_span_use).unwrap_or(0.0));
                maximum[2] = maximum[2]
                    .max(value.junction_above.unwrap_or(0.0))
                    .max(value.junction_below.unwrap_or(0.0));
                maximum[3] = maximum[3].max(
                    value
                        .rod_swing
                        .as_ref()
                        .and_then(|rods| rods.iter().filter_map(|rod| rod.ratio).reduce(f64::max))
                        .unwrap_or(0.0),
                );
            }
        }
    }
    let mut parts: Vec<(PartRef, Vec<f64>, u32)> = Vec::new();
    for (version, run) in own.iter().filter(|(_, run)| run.settled) {
        let Some(value) = stop_values(run, stop) else {
            continue;
        };
        for (part, load, past) in values(layout, value) {
            // An unknown rod ratio can be the global maximum. It cannot
            // silently become a finite denominator for the known rods.
            if matches!(part, PartRef::Rod { .. }) && missing_rod_comparisons.contains(version) {
                continue;
            }
            let denominator = denominators[version][kind(part)];
            let share = if denominator > 0.0 {
                load / denominator
            } else {
                0.0
            };
            if let Some(row) = parts.iter_mut().find(|row| row.0 == part) {
                row.1.push(share);
                row.2 += u32::from(past);
            } else {
                parts.push((part, vec![share], u32::from(past)));
            }
        }
    }
    parts
        .into_iter()
        .filter_map(|(part, values, versions_past_strength)| {
            let share = band(&values)?;
            Some(PartResult {
                part,
                share,
                steady: values.len() == own.len()
                    && share.p90 - share.p10 <= LOAD_MARGIN_SHARE * share.p50.abs(),
                versions_past_strength,
            })
        })
        .collect()
}
