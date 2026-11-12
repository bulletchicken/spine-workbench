//! The seventeen picked in order, a one-vertebra gap filled.
//!
//! The peaks are sorted from the top of the picture down, and a dynamic
//! programme picks at most one peak for each level from T1 to L5, so that
//! each peak sits where that level usually sits, looks the size it usually
//! is, and follows the one above at a usual gap. A level may be skipped
//! between two picks at a fixed cost, and is then placed halfway between
//! them; missing levels above the first pick or below the last cost the
//! same and are never placed. The priors in `weights/chain-priors.json`
//! were measured on the films the network was taught on.
//!
//! The order of every sum, and which option wins a tie, follow the
//! reference exactly, so the same peaks give the same chain. The reference
//! prices every pair of peaks; here only the steps that can be taken are
//! priced (`costs`), one lower peak at a time, and the table is filled
//! peak by peak rather than level by level (`best_path`), so no table of
//! every pair is ever made. Both give the same chain.

mod costs;

use crate::outline::{ChainLink, Peak};
use serde::Deserialize;
use serde::de::IgnoredAny;
use shapes::errors::EngineError;
use std::sync::OnceLock;

/// T1 to L5.
const LEVELS: usize = 17;

/// The chain priors, compiled in.
const PRIORS: &str = crate::weights::CHAIN_PRIORS;

/// The priors, as the file holds them.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Priors {
    /// Per level: the centre's height down the picture, the log width and
    /// the log height (see `costs`).
    mean: [[f64; 3]; LEVELS],
    std: [[f64; 3]; LEVELS],
    /// The log of the gap to the next level down, in outline heights.
    loggap_mean: f64,
    loggap_std: f64,
    /// The log change in width, and in height, from one level to the next.
    dw_mean: f64,
    dw_std: f64,
    dh_mean: f64,
    dh_std: f64,
    params: Weights,
    // Where the priors were measured: recorded in the file, not used here.
    #[serde(rename = "fit_part")]
    _fit_part: IgnoredAny,
    #[serde(rename = "fit_films")]
    _fit_films: IgnoredAny,
    #[serde(rename = "split_sha256")]
    _split_sha256: IgnoredAny,
}

/// How much each cost counts.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Weights {
    confidence: f64,
    position: f64,
    size: f64,
    gap: f64,
    continuity: f64,
    lateral: f64,
    /// The cost of each level left out.
    missing: f64,
    /// Whether one level may be skipped between two picks.
    fill: bool,
    /// The extra cost of skipping one.
    fill_cost: f64,
}

fn priors() -> &'static Priors {
    static PARSED: OnceLock<Priors> = OnceLock::new();
    PARSED.get_or_init(|| {
        serde_json::from_str(PRIORS).expect("the compiled-in chain priors are well formed")
    })
}

/// The chain through `peaks` (fractions of the picture read), from the
/// highest level found down; at most seventeen links, possibly none.
///
/// Peaks with no score are left out, and so are any with a value that is
/// not a number, which a reading never gives. Fails only with
/// `EngineOutOfMemory`.
pub fn select(peaks: &[Peak]) -> Result<Vec<ChainLink>, EngineError> {
    let priors = priors();
    let mut usable: Vec<(usize, &Peak)> = peaks
        .iter()
        .enumerate()
        .filter(|(_, peak)| {
            peak.score.is_finite()
                && peak.corners.iter().flatten().all(|v| v.is_finite())
                && peak.score > 0.0
        })
        .collect();
    let mean_down = |peak: &Peak| {
        let q = &peak.corners;
        (((q[0][1] + q[1][1]) + q[2][1]) + q[3][1]) / 4.0
    };
    usable.sort_by(|(i, a), (j, b)| mean_down(a).total_cmp(&mean_down(b)).then_with(|| i.cmp(j)));
    if usable.is_empty() {
        return Ok(Vec::new());
    }
    let ordered: Vec<&Peak> = usable.iter().map(|&(_, peak)| peak).collect();
    let costs = costs::costs(&ordered, priors)?;
    let path = best_path(&costs, priors)?;

    let mut links: Vec<ChainLink> = Vec::with_capacity(path.len() + LEVELS);
    for (step, &(level, node)) in path.iter().enumerate() {
        let (index, peak) = usable[node];
        if step > 0 && level - path[step - 1].0 == 2 {
            let above = links.last().expect("a link precedes every fill");
            let mut corners = above.corners;
            for (corner, below) in corners.iter_mut().zip(&peak.corners) {
                *corner = [(corner[0] + below[0]) / 2.0, (corner[1] + below[1]) / 2.0];
            }
            let a = costs::corner_mean(&above.corners);
            let b = costs::corner_mean(&peak.corners);
            links.push(ChainLink {
                level_index: level - 1,
                filled_in: true,
                corners,
                centre: [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0],
                score: 0.0,
                peak_index: None,
            });
        }
        links.push(ChainLink {
            level_index: level,
            filled_in: false,
            corners: peak.corners,
            centre: peak.centre,
            score: peak.score,
            peak_index: Some(index),
        });
    }
    Ok(links)
}

/// The cheapest chain as `(level, peak)` pairs from the top down; empty
/// when even the best costs as much as finding nothing.
///
/// The reference fills its table level by level, each level over every
/// peak. A step only ever goes down the sorted list, so a peak's best cost
/// at a level needs only the peaks before it, and its own costs at the
/// levels above. Here the table is filled peak by peak, every level for
/// each: every entry is the same sum, taken in the same order, as the
/// reference's, and each peak's steps are priced once and let go.
fn best_path(costs: &costs::Costs, priors: &Priors) -> Result<Vec<(usize, usize)>, EngineError> {
    let weights = &priors.params;
    let n = costs.count;
    // `[level * n + peak]`, like the costs.
    let mut best = costs::reserved(LEVELS * n)?;
    best.resize(LEVELS * n, f64::INFINITY);
    let mut previous: Vec<Option<(usize, usize)>> = costs::reserved(LEVELS * n)?;
    previous.resize(LEVELS * n, None);
    // `[jump - 1]`: the steps down to the peak in hand.
    let mut steps = [costs::reserved(n)?, costs::reserved(n)?];
    let jumps: &[usize] = if weights.fill { &[1, 2] } else { &[1] };
    for to in 0..n {
        for &jump in jumps {
            costs.steps_down_to(to, jump, priors, &mut steps[jump - 1]);
        }
        for level in 0..LEVELS {
            let unary = costs.unary[level * n + to];
            // Starting here leaves every level above missing.
            let mut here = unary + level as f64 * weights.missing;
            let mut came_from = None;
            for &jump in jumps {
                if level < jump {
                    continue;
                }
                let extra = (jump - 1) as f64 * weights.fill_cost;
                let above = &best[(level - jump) * n..(level - jump + 1) * n];
                // The reference tries every peak in turn as where the step
                // starts: the first sets the price to beat, and a later one
                // wins only when strictly cheaper, so the first cheapest
                // wins a tie. Through a step left out the total is
                // infinite (or, from absurd corners, not a number), which
                // is never strictly cheaper than anything; so trying the
                // first peak, listed or not, and then the listed ones in
                // order, picks the same peak at the same price.
                let listed = steps[jump - 1].as_slice();
                let (first_step, rest) = match listed.split_first() {
                    Some((&(0, step), rest)) => (step, rest),
                    _ => (f64::INFINITY, listed),
                };
                let mut source = 0;
                let mut cheapest = above[0] + first_step + extra;
                for &(from, step) in rest {
                    let value = above[from] + step + extra;
                    if value < cheapest {
                        cheapest = value;
                        source = from;
                    }
                }
                let cost = cheapest + unary;
                // Strictly cheaper only: starting here beats one level on,
                // which beats a skip.
                if cost < here {
                    here = cost;
                    came_from = Some((level - jump, source));
                }
            }
            best[level * n + to] = here;
            previous[level * n + to] = came_from;
        }
    }

    // Every level below the last pick is missing too. The first cheapest
    // in level order, then peak order, wins.
    let mut end = (0, 0);
    let mut lowest = f64::NAN;
    for (level, row) in best.chunks_exact(n).enumerate() {
        for (node, &cost) in row.iter().enumerate() {
            let total = cost + (LEVELS - 1 - level) as f64 * weights.missing;
            if (level, node) == (0, 0) || total < lowest {
                lowest = total;
                end = (level, node);
            }
        }
    }
    if lowest >= LEVELS as f64 * weights.missing {
        return Ok(Vec::new());
    }
    let mut path = vec![end];
    while let Some(before) = previous[path[path.len() - 1].0 * n + path[path.len() - 1].1] {
        path.push(before);
    }
    path.reverse();
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_compiled_in_priors_read() {
        let priors = priors();
        assert_eq!(priors.params.missing, 8.0);
        assert!(priors.params.fill);
        assert_eq!(priors.mean[16][0], 0.8745885068917741);
    }

    #[test]
    fn nothing_to_choose_from_gives_no_chain() {
        assert!(select(&[]).unwrap().is_empty());
    }

    #[test]
    fn one_lone_weak_peak_is_not_a_chain() {
        // A single node is kept only if it costs less than leaving every
        // level missing; a peak scoring 0.01 costs 4 * -ln(0.01) = 18.4 in
        // confidence alone, more than the 8 its own level would save.
        let weak = Peak {
            score: 0.01,
            centre: [0.5, 0.05],
            corners: [[0.45, 0.04], [0.55, 0.04], [0.55, 0.06], [0.45, 0.06]],
            cell: [48, 10],
        };
        assert!(select(&[weak]).unwrap().is_empty());
    }
}
