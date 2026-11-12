//! The chain as the reference works it, pricing every pair of peaks: two
//! full tables of steps, and a search that tries every peak as the source
//! of every step. It is kept here, word for word as the crate had it, only
//! to hold the shipped chain to it; the shipped chain prices only the
//! steps that can be taken.

#![allow(dead_code)]

use serde::Deserialize;
use serde::de::IgnoredAny;
use std::sync::OnceLock;
use trained_finder::outline::{ChainLink, Peak, Quad};

const LEVELS: usize = 17;

const PRIORS: &str = include_str!("../../weights/chain-priors.json");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Priors {
    mean: [[f64; 3]; LEVELS],
    std: [[f64; 3]; LEVELS],
    loggap_mean: f64,
    loggap_std: f64,
    dw_mean: f64,
    dw_std: f64,
    dh_mean: f64,
    dh_std: f64,
    params: Weights,
    #[serde(rename = "fit_part")]
    _fit_part: IgnoredAny,
    #[serde(rename = "fit_films")]
    _fit_films: IgnoredAny,
    #[serde(rename = "split_sha256")]
    _split_sha256: IgnoredAny,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Weights {
    confidence: f64,
    position: f64,
    size: f64,
    gap: f64,
    continuity: f64,
    lateral: f64,
    missing: f64,
    fill: bool,
    fill_cost: f64,
}

fn priors() -> &'static Priors {
    static PARSED: OnceLock<Priors> = OnceLock::new();
    PARSED.get_or_init(|| serde_json::from_str(PRIORS).expect("the chain priors are well formed"))
}

/// The chain through `peaks`, from the highest level found down.
pub fn select(peaks: &[Peak]) -> Vec<ChainLink> {
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
        return Vec::new();
    }
    let ordered: Vec<&Peak> = usable.iter().map(|&(_, peak)| peak).collect();
    let costs = costs(&ordered, priors);
    let path = best_path(&costs, &priors.params);

    let mut links: Vec<ChainLink> = Vec::with_capacity(path.len() + LEVELS);
    for (step, &(level, node)) in path.iter().enumerate() {
        let (index, peak) = usable[node];
        if step > 0 && level - path[step - 1].0 == 2 {
            let above = links.last().expect("a link precedes every fill");
            let mut corners = above.corners;
            for (corner, below) in corners.iter_mut().zip(&peak.corners) {
                *corner = [(corner[0] + below[0]) / 2.0, (corner[1] + below[1]) / 2.0];
            }
            let a = corner_mean(&above.corners);
            let b = corner_mean(&peak.corners);
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
    links
}

fn best_path(costs: &Costs, weights: &Weights) -> Vec<(usize, usize)> {
    let n = costs.count;
    let mut best = vec![vec![f64::INFINITY; n]; LEVELS];
    let mut previous: Vec<Vec<Option<(usize, usize)>>> = vec![vec![None; n]; LEVELS];
    for level in 0..LEVELS {
        for (start, &unary) in best[level].iter_mut().zip(&costs.unary[level]) {
            *start = unary + level as f64 * weights.missing;
        }
        for jump in [1, 2] {
            if level < jump || (jump == 2 && !weights.fill) {
                continue;
            }
            let extra = (jump - 1) as f64 * weights.fill_cost;
            let steps = &costs.steps[jump - 1];
            for to in 0..n {
                let mut source = 0;
                let mut cheapest = f64::NAN;
                for from in 0..n {
                    let value = best[level - jump][from] + steps[from][to] + extra;
                    if from == 0 || value < cheapest {
                        cheapest = value;
                        source = from;
                    }
                }
                let cost = cheapest + costs.unary[level][to];
                if cost < best[level][to] {
                    best[level][to] = cost;
                    previous[level][to] = Some((level - jump, source));
                }
            }
        }
    }

    let mut end = (0, 0);
    let mut lowest = f64::NAN;
    for (level, row) in best.iter().enumerate() {
        for (node, &cost) in row.iter().enumerate() {
            let total = cost + (LEVELS - 1 - level) as f64 * weights.missing;
            if (level, node) == (0, 0) || total < lowest {
                lowest = total;
                end = (level, node);
            }
        }
    }
    if lowest >= LEVELS as f64 * weights.missing {
        return Vec::new();
    }
    let mut path = vec![end];
    while let Some(before) = previous[path[path.len() - 1].0][path[path.len() - 1].1] {
        path.push(before);
    }
    path.reverse();
    path
}

struct Costs {
    count: usize,
    unary: Vec<Vec<f64>>,
    steps: [Vec<Vec<f64>>; 2],
}

struct Shape {
    centre: [f64; 2],
    width: f64,
    height: f64,
    normal: [f64; 2],
}

fn norm(v: [f64; 2]) -> f64 {
    (v[0] * v[0] + v[1] * v[1]).sqrt()
}

fn corner_mean(q: &Quad) -> [f64; 2] {
    let mut mean = [0.0; 2];
    for (k, value) in mean.iter_mut().enumerate() {
        *value = (((q[0][k] + q[1][k]) + q[2][k]) + q[3][k]) / 4.0;
    }
    mean
}

fn shape(q: &Quad) -> Shape {
    let s = q.map(|[x, y]| [x * 0.5, y]);
    let centre = corner_mean(q);
    let edge = [0, 1].map(|k| (((s[1][k] + s[2][k]) - s[0][k]) - s[3][k]) / 2.0);
    let width = norm(edge).max(0.001);
    let left = norm([s[3][0] - s[0][0], s[3][1] - s[0][1]]);
    let right = norm([s[2][0] - s[1][0], s[2][1] - s[1][1]]);
    let height = ((left + right) / 2.0).max(0.001);
    let mut normal = [-edge[1] / width, edge[0] / width];
    let sign = if normal[1] >= 0.0 { 1.0 } else { -1.0 };
    normal = normal.map(|v| v * sign);
    Shape {
        centre,
        width,
        height,
        normal,
    }
}

fn costs(peaks: &[&Peak], priors: &Priors) -> Costs {
    let count = peaks.len();
    let shapes: Vec<Shape> = peaks.iter().map(|p| shape(&p.corners)).collect();
    let weights = &priors.params;

    let confidence: Vec<f64> = peaks
        .iter()
        .map(|p| -libm::log(p.score.clamp(0.001, 1.0)))
        .collect();
    let unary = (0..LEVELS)
        .map(|level| {
            let mean = priors.mean[level];
            let std = priors.std[level];
            shapes
                .iter()
                .zip(&confidence)
                .map(|(shape, &confidence)| {
                    let feature = [
                        shape.centre[1],
                        libm::log(shape.width),
                        libm::log(shape.height),
                    ];
                    let z = [0, 1, 2].map(|k| (feature[k] - mean[k]) / std[k]);
                    let position = libm::log1p(z[0] * z[0]);
                    let size = libm::log1p(z[1] * z[1]) + libm::log1p(z[2] * z[2]);
                    weights.confidence * confidence
                        + weights.position * position
                        + weights.size * size
                })
                .collect()
        })
        .collect();

    let steps = [1usize, 2].map(|jump| {
        let jump = jump as f64;
        let root = jump.sqrt();
        (0..count)
            .map(|i| {
                (0..count)
                    .map(|j| step(&shapes[i], &shapes[j], jump, root, priors))
                    .collect()
            })
            .collect()
    });

    Costs {
        count,
        unary,
        steps,
    }
}

fn step(a: &Shape, b: &Shape, jump: f64, root: f64, priors: &Priors) -> f64 {
    let delta = [
        (b.centre[0] - a.centre[0]) * 0.5,
        (b.centre[1] - a.centre[1]) * 1.0,
    ];
    let gap = norm(delta) / ((a.height + b.height) / 2.0);
    let mut tangent = [a.normal[0] + b.normal[0], a.normal[1] + b.normal[1]];
    let length = norm(tangent).max(0.001);
    tangent = tangent.map(|v| v / length);
    let lateral =
        (delta[0] * tangent[1] - delta[1] * tangent[0]).abs() / ((a.width + b.width) / 2.0);
    let width_change = libm::log(b.width / a.width);
    let height_change = libm::log(b.height / a.height);

    let valid =
        delta[1] > 0.005 * jump && gap > 0.65 * jump && gap < 3.8 * jump && lateral < 1.0 * jump;
    if !valid {
        return f64::INFINITY;
    }
    let squared = |v: f64| v * v;
    let gap_cost = libm::log1p(squared(
        (libm::log((gap / jump).max(0.01)) - priors.loggap_mean) / priors.loggap_std,
    ));
    let continuity = libm::log1p(squared(
        (width_change - jump * priors.dw_mean) / (priors.dw_std * root),
    )) + libm::log1p(squared(
        (height_change - jump * priors.dh_mean) / (priors.dh_std * root),
    ));
    let weights = &priors.params;
    weights.gap * gap_cost + weights.continuity * continuity + weights.lateral * squared(lateral)
}
