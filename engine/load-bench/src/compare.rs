//! Ranges, tags, shares, ghosts and paired counts on shared versions.
//!
//! Unsettled runs are counted and excluded from summaries. Missing measures
//! have no estimate or paired count. Percentiles interpolate at p × (n−1).

mod parts;
mod ranks;

use crate::BenchError;
use shapes::implant_layout::ImplantLayout;
use shapes::load_bench::*;
use spine_readings::bands::band;
use std::collections::{BTreeMap, BTreeSet};

/// This project's angular indifference margin, in degrees.
pub const ANGLE_MARGIN_DEG: f64 = 2.0;
/// This project's load indifference margin, as a share of the first value.
pub const LOAD_MARGIN_SHARE: f64 = 0.10;
/// This project's threshold for calling a paired count clear (24 of 32).
pub const CLEAR_SHARE: f64 = 0.75;
/// This project's clear-count threshold for the default 32 versions.
pub const CLEAR_COUNT_32: u32 = 24;
/// This project's sign agreement for comparison-only (29 of 32).
pub const COMPARISON_SIGN_SHARE: f64 = 0.90;
/// This project's upper limit on the springback-to-rigid-correction ratio.
pub const STIFF_ROD_LIMIT: f64 = 0.30;

const STOPS: [BenchStop; 5] = [
    BenchStop::Before,
    BenchStop::RodsIn,
    BenchStop::Standing,
    BenchStop::BendingForward,
    BenchStop::YearOfWalking,
];
const MEASURES: [Measure; 10] = [
    Measure::MainCobb,
    Measure::Kyphosis,
    Measure::ApicalTwist,
    Measure::HeaviestAnchorForce,
    Measure::HeaviestAnchorDemand,
    Measure::HeaviestAnchorOverMedian,
    Measure::HeaviestRodUse,
    Measure::JunctionAbove,
    Measure::JunctionBelow,
    Measure::HeaviestRodSwing,
];

type Runs<'a> = BTreeMap<u32, &'a BenchRun>;

/// Summarise all batches, pairing each placement with the first by version
/// number. Run order and batch boundaries do not affect the answer.
pub fn compare(input: &BenchCompareInput) -> Result<BenchResult, BenchError> {
    let runs = collect(input)?;
    let first = &runs[0];
    let mut layouts = Vec::new();
    for (index, layout) in input.layouts.iter().enumerate() {
        let own = &runs[index];
        let mut result = LayoutResult {
            layout_id: layout.id.clone(),
            versions_unsettled: own.values().filter(|run| !run.settled).count() as u32,
            stops: Vec::new(),
            counts: Vec::new(),
        };
        for stop in STOPS {
            let outside = own
                .values()
                .filter(|run| run.settled && run.stiff_rod_share > STIFF_ROD_LIMIT)
                .count() as u32;
            let mut estimates = Vec::new();
            for measure in MEASURES {
                let samples = samples(own, layout, stop, measure);
                let values: Vec<_> = samples.iter().map(|(_, value)| *value).collect();
                let Some(range) = band(&values) else {
                    continue;
                };
                let paired = paired(own, first, layout, &input.layouts[0], stop, measure);
                let sign_keeps = paired
                    .iter()
                    .filter(|row| row.1 < row.2)
                    .count()
                    .max(paired.iter().filter(|row| row.1 > row.2).count());
                // Missing pairs and unsettled versions cannot add certainty.
                let sign_consistent =
                    index > 0 && sign_keeps as f64 >= COMPARISON_SIGN_SHARE * own.len() as f64;
                estimates.push(Estimate {
                    measure,
                    band: range,
                    tag: if samples.len() == own.len() {
                        tag(measure, range, sign_consistent, outside > 0)
                    } else {
                        EstimateTag::Assumed
                    },
                    mostly_from: ranks::driver(own, &samples).0,
                });
                if index > 0 && !paired.is_empty() {
                    result.counts.push(count(own, stop, measure, &paired));
                }
            }
            let cycles: Vec<_> = own
                .values()
                .filter(|run| run.settled)
                .filter_map(|run| stop_values(run, stop)?.cycles_per_year)
                .collect();
            result.stops.push(StopResult {
                stop,
                estimates,
                parts: parts::summarise(layout, own, &runs, stop),
                ghosts: ghosts(own, stop),
                cycles_per_year: band(&cycles),
                outside_stiff_rod: if stop == BenchStop::Before {
                    0
                } else {
                    outside
                },
            });
        }
        layouts.push(result);
    }
    let mut scales: BTreeMap<Measure, (f64, f64)> = BTreeMap::new();
    for estimate in layouts
        .iter()
        .flat_map(|layout| &layout.stops)
        .flat_map(|stop| &stop.estimates)
    {
        scales
            .entry(estimate.measure)
            .and_modify(|scale| {
                scale.0 = scale.0.min(estimate.band.p10);
                scale.1 = scale.1.max(estimate.band.p90);
            })
            .or_insert((estimate.band.p10, estimate.band.p90));
    }
    Ok(BenchResult {
        versions: first.len() as u32,
        first_id: input.layouts[0].id.clone(),
        layouts,
        scales: scales
            .into_iter()
            .map(|(measure, (lowest, highest))| MeasureScale {
                measure,
                lowest,
                highest,
            })
            .collect(),
    })
}

fn refused(reason: &str) -> BenchError {
    BenchError::Refused(reason.to_string())
}

fn collect(input: &BenchCompareInput) -> Result<Vec<Runs<'_>>, BenchError> {
    if input.layouts.is_empty() {
        return Err(refused("no-layouts"));
    }
    let mut ids = BTreeSet::new();
    if input
        .layouts
        .iter()
        .any(|layout| layout.id.is_empty() || !ids.insert(&layout.id))
    {
        return Err(refused("duplicate-or-empty-layout-id"));
    }
    let mut result: Vec<Runs<'_>> = vec![BTreeMap::new(); input.layouts.len()];
    for batch in &input.batches {
        let end = batch
            .version_from
            .checked_add(batch.version_count)
            .ok_or_else(|| refused("version-range"))?;
        for run in &batch.runs {
            let index = input
                .layouts
                .iter()
                .position(|layout| layout.id == run.layout_id)
                .ok_or_else(|| refused("unknown-layout"))?;
            if run.version < batch.version_from || run.version >= end {
                return Err(refused("version-outside-batch"));
            }
            if result[index].insert(run.version, run).is_some() {
                return Err(refused("duplicate-layout-version"));
            }
            validate_draws(&run.draws)?;
            if run.settled {
                validate_run(&input.layouts[index], run)?;
            }
        }
    }
    if result[0].is_empty() {
        return Err(refused("no-versions"));
    }
    let versions: Vec<_> = result[0].keys().copied().collect();
    for (index, other) in result.iter().enumerate().skip(1) {
        if other.keys().copied().collect::<Vec<_>>() != versions {
            return Err(refused("versions-not-shared"));
        }
        for (version, run) in other {
            let baseline = result[0][version];
            if !shared_draws(&baseline.draws, &run.draws) {
                return Err(refused("draws-not-shared"));
            }
            let first_material = input.layouts[0].rods.first().map(|rod| rod.material);
            let own_material = input.layouts[index].rods.first().map(|rod| rod.material);
            if first_material.is_some()
                && first_material == own_material
                && baseline.draws.rod_yield_mpa != run.draws.rod_yield_mpa
            {
                return Err(refused("rod-strength-draws-not-shared"));
            }
            if let (Some(first), Some(own)) = (
                stop_values(baseline, BenchStop::Before).and_then(|stop| stop.poses.as_ref()),
                stop_values(run, BenchStop::Before).and_then(|stop| stop.poses.as_ref()),
            ) && first != own
            {
                return Err(refused("geometry-not-shared"));
            }
        }
    }
    Ok(result)
}

fn validate_run(layout: &ImplantLayout, run: &BenchRun) -> Result<(), BenchError> {
    if !run.stiff_rod_share.is_finite()
        || run.stiff_rod_share < 0.0
        || run.stops.iter().map(|stop| stop.stop).collect::<Vec<_>>() != STOPS
    {
        return Err(refused("invalid-run"));
    }
    let spans = parts::spans(layout).len();
    for stop in &run.stops {
        if [stop.main_cobb_deg, stop.kyphosis_deg, stop.apical_twist_deg]
            .iter()
            .any(|value| !value.is_finite())
        {
            return Err(refused("invalid-angle"));
        }
        if stop.anchor_force_n.len() != layout.anchors.len()
            || stop.anchor_demand.len() != layout.anchors.len()
            || stop.rod_span_use.len() != spans
        {
            return Err(refused("part-values-do-not-match-layout"));
        }
        if stop
            .anchor_force_n
            .iter()
            .chain(&stop.anchor_demand)
            .chain(&stop.rod_span_use)
            .any(|value| !value.is_finite() || *value < 0.0)
            || [
                stop.junction_above,
                stop.junction_below,
                stop.cycles_per_year,
            ]
            .into_iter()
            .flatten()
            .any(|value| !value.is_finite() || value < 0.0)
            || stop.rod_swing.as_ref().is_some_and(|rods| {
                rods.iter()
                    .filter_map(|rod| rod.ratio)
                    .any(|value| !value.is_finite() || value < 0.0)
            })
        {
            return Err(refused("invalid-stop-value"));
        }
    }
    Ok(())
}

fn validate_draws(draws: &VersionDraws) -> Result<(), BenchError> {
    if [
        draws.stiffness_scale,
        draws.rib_factors[0],
        draws.rib_factors[1],
        draws.rib_factors[2],
        draws.bone_factor,
        draws.weight_kg,
        draws.steps_per_day,
        draws.walking_multiple,
        draws.rod_yield_mpa,
        draws.preload_factor,
        draws.slice_offset_mm,
        draws.contour_knock_down,
    ]
    .iter()
    .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(refused("invalid-draws"));
    }
    Ok(())
}

fn shared_draws(a: &VersionDraws, b: &VersionDraws) -> bool {
    // Rod yield is material-specific; the shared quantile is applied to each material.
    a.stiffness_scale == b.stiffness_scale
        && a.rib_factors == b.rib_factors
        && a.bone_factor == b.bone_factor
        && a.weight_kg == b.weight_kg
        && a.steps_per_day == b.steps_per_day
        && a.walking_multiple == b.walking_multiple
        && a.preload_factor == b.preload_factor
        && a.slice_offset_mm == b.slice_offset_mm
        && a.contour_knock_down == b.contour_knock_down
}

fn stop_values(run: &BenchRun, stop: BenchStop) -> Option<&StopValues> {
    run.stops.iter().find(|value| value.stop == stop)
}

fn largest(values: &[f64]) -> Option<f64> {
    values.iter().copied().reduce(f64::max)
}

fn measure(value: &StopValues, layout: &ImplantLayout, which: Measure) -> Option<f64> {
    let value = match which {
        Measure::MainCobb => Some(value.main_cobb_deg),
        Measure::Kyphosis => Some(value.kyphosis_deg),
        Measure::ApicalTwist => Some(value.apical_twist_deg),
        Measure::HeaviestAnchorForce => largest(&value.anchor_force_n),
        Measure::HeaviestAnchorDemand => largest(&value.anchor_demand),
        Measure::HeaviestAnchorOverMedian => {
            let occupied: Vec<_> = layout
                .anchors
                .iter()
                .zip(&value.anchor_force_n)
                .filter(|(anchor, _)| anchor.kind != shapes::implant_layout::AnchorKind::None)
                .map(|(_, force)| *force)
                .collect();
            band(&occupied)
                .filter(|range| range.p50 > 0.0)
                .map(|range| largest(&occupied).unwrap_or(0.0) / range.p50)
        }
        Measure::HeaviestRodUse => largest(&value.rod_span_use),
        Measure::JunctionAbove => value.junction_above,
        Measure::JunctionBelow => value.junction_below,
        Measure::HeaviestRodSwing => value.rod_swing.as_ref().and_then(|rods| {
            rods.iter()
                .map(|rod| rod.ratio)
                .collect::<Option<Vec<_>>>()
                .and_then(|ratios| largest(&ratios))
        }),
    };
    value.filter(|number| number.is_finite())
}

fn samples(
    runs: &Runs<'_>,
    layout: &ImplantLayout,
    stop: BenchStop,
    which: Measure,
) -> Vec<(u32, f64)> {
    runs.iter()
        .filter(|(_, run)| run.settled)
        .filter_map(|(version, run)| {
            Some((*version, measure(stop_values(run, stop)?, layout, which)?))
        })
        .collect()
}

fn paired(
    runs: &Runs<'_>,
    first: &Runs<'_>,
    layout: &ImplantLayout,
    first_layout: &ImplantLayout,
    stop: BenchStop,
    which: Measure,
) -> Vec<(u32, f64, f64)> {
    runs.iter()
        .filter_map(|(version, run)| {
            let baseline = first[version];
            if !run.settled || !baseline.settled {
                return None;
            }
            Some((
                *version,
                measure(stop_values(run, stop)?, layout, which)?,
                measure(stop_values(baseline, stop)?, first_layout, which)?,
            ))
        })
        .collect()
}

fn angular(measure: Measure) -> bool {
    matches!(
        measure,
        Measure::MainCobb | Measure::Kyphosis | Measure::ApicalTwist
    )
}

// Sagittal shape and twist stay assumed: the batch contract does not
// retain whether those fitted values came from a film or a typical spine.
fn tag(
    measure: Measure,
    range: shapes::spine_readings::Band,
    sign_consistent: bool,
    outside: bool,
) -> EstimateTag {
    if matches!(
        measure,
        Measure::HeaviestAnchorForce | Measure::Kyphosis | Measure::ApicalTwist
    ) || (outside && matches!(measure, Measure::JunctionAbove | Measure::JunctionBelow))
    {
        EstimateTag::Assumed
    } else if range.p90 - range.p10
        <= if angular(measure) {
            ANGLE_MARGIN_DEG
        } else {
            LOAD_MARGIN_SHARE * range.p50.abs()
        }
    {
        EstimateTag::SetByTheFilms
    } else if sign_consistent {
        EstimateTag::ComparisonOnly
    } else {
        EstimateTag::Assumed
    }
}

fn count(
    runs: &Runs<'_>,
    stop: BenchStop,
    measure: Measure,
    paired: &[(u32, f64, f64)],
) -> PairedCount {
    let differences: Vec<_> = paired
        .iter()
        .map(|(version, own, first)| (*version, own - first))
        .collect();
    let (sorted_by, driver) = ranks::driver(runs, &differences);
    let mut order: Vec<_> = paired.iter().map(|row| row.0).collect();
    order.sort_by(|a, b| driver[a].total_cmp(&driver[b]).then(a.cmp(b)));
    let mut result = PairedCount {
        measure,
        stop,
        lower: 0,
        higher: 0,
        ties: 0,
        order,
        sorted_by,
    };
    for (_, own, first) in paired {
        let margin = if angular(measure) {
            ANGLE_MARGIN_DEG
        } else {
            LOAD_MARGIN_SHARE * first.abs()
        };
        if own - first < -margin {
            result.lower += 1;
        } else if own - first > margin {
            result.higher += 1;
        } else {
            result.ties += 1;
        }
    }
    result
}

fn ghosts(runs: &Runs<'_>, stop: BenchStop) -> Option<Vec<Vec<shapes::made_up::VertebraPose>>> {
    let mut ordered: Vec<_> = runs
        .iter()
        .filter(|(_, run)| run.settled)
        .filter_map(|(version, run)| {
            let value = stop_values(run, stop)?;
            Some((*version, value.main_cobb_deg, value.poses.as_ref()?))
        })
        .collect();
    if ordered.is_empty() {
        return None;
    }
    ordered.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    // A ghost is an actual version, so take the nearest rank, not interpolated poses.
    let at = |p: f64| libm::round(p * (ordered.len() - 1) as f64) as usize;
    Some(vec![ordered[at(0.1)].2.clone(), ordered[at(0.9)].2.clone()])
}

#[cfg(test)]
mod tests;
