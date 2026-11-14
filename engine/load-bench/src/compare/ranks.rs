//! Tied-rank correlations with the individual components of each draw.

use super::{BenchRun, BenchStop, Runs, SampledUnknown, stop_values};
use std::collections::BTreeMap;

type Column = (SampledUnknown, Vec<(u32, f64)>);
type Scalar = (SampledUnknown, fn(&BenchRun) -> f64);

/// Pick the scalar component with greatest absolute Spearman correlation.
/// Geometry and rib factors keep their components separate: summing them
/// would hide equal and opposite changes. Stable component order breaks ties.
pub(super) fn driver(
    runs: &Runs<'_>,
    samples: &[(u32, f64)],
) -> (Option<SampledUnknown>, BTreeMap<u32, f64>) {
    let mut columns: Vec<Column> = Vec::new();
    let geometry_count = runs
        .values()
        .find_map(|run| {
            stop_values(run, BenchStop::Before)?
                .poses
                .as_ref()
                .map(|poses| poses.len() * 6)
        })
        .unwrap_or(0);
    for component in 0..geometry_count {
        columns.push((
            SampledUnknown::Geometry,
            samples
                .iter()
                .filter_map(|(version, _)| {
                    let pose = stop_values(runs[version], BenchStop::Before)?
                        .poses
                        .as_ref()?
                        .get(component / 6)?;
                    let value = if component % 6 < 3 {
                        pose.centre[component % 6]
                    } else {
                        pose.rotation[component % 6 - 3]
                    };
                    Some((*version, value))
                })
                .collect(),
        ));
    }
    // Before Cobb is still available when a hand-supplied batch omits poses.
    columns.push((
        SampledUnknown::Geometry,
        samples
            .iter()
            .filter_map(|(version, _)| {
                Some((
                    *version,
                    stop_values(runs[version], BenchStop::Before)?.main_cobb_deg,
                ))
            })
            .collect(),
    ));
    let scalars: [Scalar; 11] = [
        (SampledUnknown::StiffnessScale, |run| {
            run.draws.stiffness_scale
        }),
        (SampledUnknown::RibCage, |run| run.draws.rib_factors[0]),
        (SampledUnknown::RibCage, |run| run.draws.rib_factors[1]),
        (SampledUnknown::RibCage, |run| run.draws.rib_factors[2]),
        (SampledUnknown::Bone, |run| run.draws.bone_factor),
        (SampledUnknown::Weight, |run| run.draws.weight_kg),
        (SampledUnknown::Steps, |run| run.draws.steps_per_day),
        (SampledUnknown::Walking, |run| run.draws.walking_multiple),
        (SampledUnknown::RodYield, |run| run.draws.rod_yield_mpa),
        (SampledUnknown::Preload, |run| run.draws.preload_factor),
        (SampledUnknown::ContourKnockDown, |run| {
            run.draws.contour_knock_down
        }),
    ];
    for (unknown, read) in scalars {
        columns.push((
            unknown,
            samples
                .iter()
                .map(|(version, _)| (*version, read(runs[version])))
                .collect(),
        ));
    }
    let values: Vec<_> = samples.iter().map(|row| row.1).collect();
    let mut selected = (None, samples.iter().map(|row| (row.0, 0.0)).collect());
    let mut largest = 0.0;
    for (unknown, column) in columns {
        if column.len() != samples.len() || column.iter().any(|row| !row.1.is_finite()) {
            continue;
        }
        let correlation =
            spearman(&values, &column.iter().map(|row| row.1).collect::<Vec<_>>()).abs();
        if correlation > largest {
            largest = correlation;
            selected = (Some(unknown), column.into_iter().collect());
        }
    }
    selected
}

fn ranks(values: &[f64]) -> Vec<f64> {
    let mut order: Vec<_> = (0..values.len()).collect();
    order.sort_by(|a, b| values[*a].total_cmp(&values[*b]).then(a.cmp(b)));
    let mut result = vec![0.0; values.len()];
    let mut low = 0;
    while low < order.len() {
        let mut high = low + 1;
        while high < order.len() && values[order[high]] == values[order[low]] {
            high += 1;
        }
        let rank = (low + high - 1) as f64 / 2.0;
        for index in &order[low..high] {
            result[*index] = rank;
        }
        low = high;
    }
    result
}

fn spearman(a: &[f64], b: &[f64]) -> f64 {
    if a.len() < 2 {
        return 0.0;
    }
    let a = ranks(a);
    let b = ranks(b);
    let mean = (a.len() - 1) as f64 / 2.0;
    let mut cross = 0.0;
    let mut aa = 0.0;
    let mut bb = 0.0;
    for (a, b) in a.iter().zip(&b) {
        cross += (a - mean) * (b - mean);
        aa += (a - mean) * (a - mean);
        bb += (b - mean) * (b - mean);
    }
    if aa == 0.0 || bb == 0.0 {
        0.0
    } else {
        cross / libm::sqrt(aa * bb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tied_ranks_keep_the_hand_worked_correlations() {
        assert_eq!(ranks(&[7.0, 2.0, 2.0, 9.0]), vec![2.0, 0.5, 0.5, 3.0]);
        assert_eq!(spearman(&[1.0, 2.0, 2.0], &[6.0, 4.0, 4.0]), -1.0);
        assert_eq!(spearman(&[1.0, 1.0], &[2.0, 3.0]), 0.0);
    }
}
