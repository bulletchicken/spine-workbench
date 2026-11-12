//! What each peak costs as each level, and what each step between two
//! peaks costs, from the priors.
//!
//! All in f64, in the reference's order of operations, so the chain picks
//! the same peaks whenever two paths cost nearly the same.
//!
//! The reference prices every pair of peaks, in two tables of every pair,
//! and a film with no spine on it can give well over a thousand weak
//! peaks. Here a step is priced only where it could be taken, one lower
//! peak at a time ([`Costs::steps_down_to`]); every step left out would
//! have cost infinitely much, so the chain comes out the same.

use super::{LEVELS, Priors};
use crate::outline::{Peak, Quad};
use shapes::errors::{EngineError, ErrorCode};

/// The costs for the peaks the chain may use.
pub(super) struct Costs {
    /// How many peaks.
    pub count: usize,
    /// `[level * count + peak]`: the cost of that peak standing as that
    /// level.
    pub unary: Vec<f64>,
    /// Each peak's shape, in chain order.
    shapes: Vec<Shape>,
    /// The tallest outline's height.
    tallest: f64,
}

/// One peak's shape in the network's 384 by 768 frame, where across is
/// counted in units of the picture height.
struct Shape {
    centre: [f64; 2],
    width: f64,
    height: f64,
    /// The unit normal to the outline's width, pointing down.
    normal: [f64; 2],
}

fn norm(v: [f64; 2]) -> f64 {
    (v[0] * v[0] + v[1] * v[1]).sqrt()
}

/// The mean of four corners, summed in order.
pub(super) fn corner_mean(q: &Quad) -> [f64; 2] {
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

/// The costs for `peaks`, already in chain order: sorted by the mean
/// height of their corners from the top down, the same sum `shape` takes
/// for a centre.
pub(super) fn costs(peaks: &[&Peak], priors: &Priors) -> Result<Costs, EngineError> {
    let count = peaks.len();
    let shapes: Vec<Shape> = peaks.iter().map(|p| shape(&p.corners)).collect();
    debug_assert!(
        shapes
            .windows(2)
            .all(|pair| pair[0].centre[1].total_cmp(&pair[1].centre[1]).is_le()),
        "the peaks come sorted from the top down"
    );
    let weights = &priors.params;

    let confidence: Vec<f64> = peaks
        .iter()
        .map(|p| -libm::log(p.score.clamp(0.001, 1.0)))
        .collect();
    let mut unary = reserved(LEVELS * count)?;
    for level in 0..LEVELS {
        let mean = priors.mean[level];
        let std = priors.std[level];
        unary.extend(shapes.iter().zip(&confidence).map(|(shape, &confidence)| {
            let feature = [
                shape.centre[1],
                libm::log(shape.width),
                libm::log(shape.height),
            ];
            let z = [0, 1, 2].map(|k| (feature[k] - mean[k]) / std[k]);
            let position = libm::log1p(z[0] * z[0]);
            let size = libm::log1p(z[1] * z[1]) + libm::log1p(z[2] * z[2]);
            weights.confidence * confidence + weights.position * position + weights.size * size
        }));
    }

    let tallest = shapes.iter().map(|shape| shape.height).fold(0.0, f64::max);
    Ok(Costs {
        count,
        unary,
        shapes,
        tallest,
    })
}

impl Costs {
    /// Writes over `steps` every step `jump` levels on, down to peak `to`,
    /// that could be taken: the upper peak and the step's cost, upper peaks
    /// in order. A step not listed costs infinitely much. `steps` must have
    /// room for `count` steps, so this never asks for memory.
    ///
    /// Why the others need no pricing. A step must go down: a peak no
    /// higher than `to` never steps to it, and the peaks are sorted, so
    /// every peak after `to` in the list is out. A step's gap (the distance
    /// between the two centres, over the two outlines' mean height) must be
    /// under 3.8 jumps; the distance is at least the drop, and the mean
    /// height is at most (the tallest outline's height plus `to`'s) / 2, so
    /// a peak whose drop to `to` is more than 3.8 jumps of that can never
    /// step to it. Those are the peaks at the front of the list; they are
    /// left out with 4 jumps in place of 3.8, so rounding cannot matter.
    /// The rest are priced exactly as the reference prices them, and a step
    /// that comes out infinite is left out too.
    pub fn steps_down_to(
        &self,
        to: usize,
        jump: usize,
        priors: &Priors,
        steps: &mut Vec<(usize, f64)>,
    ) {
        steps.clear();
        let jump = jump as f64;
        let root = jump.sqrt();
        let lower = &self.shapes[to];
        let reach = 4.0 * jump * ((self.tallest + lower.height) / 2.0);
        // The drop is worked out exactly as `step` works it out.
        let first = self.shapes[..to]
            .partition_point(|upper| (lower.centre[1] - upper.centre[1]) * 1.0 > reach);
        for (from, upper) in self.shapes.iter().enumerate().take(to).skip(first) {
            let cost = step(upper, lower, jump, root, priors);
            if cost != f64::INFINITY {
                steps.push((from, cost));
            }
        }
    }
}

/// The cost of stepping from `a` down to `b`, `jump` levels on; infinite
/// where the step is implausible.
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

    let valid =
        delta[1] > 0.005 * jump && gap > 0.65 * jump && gap < 3.8 * jump && lateral < 1.0 * jump;
    if !valid {
        return f64::INFINITY;
    }
    // Worked out only for a step that can be taken; the values are the
    // same wherever they are worked out.
    let width_change = libm::log(b.width / a.width);
    let height_change = libm::log(b.height / a.height);
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

/// An empty list with room for `len` values, or `EngineOutOfMemory`.
pub(super) fn reserved<T>(len: usize) -> Result<Vec<T>, EngineError> {
    let mut values = Vec::new();
    values.try_reserve_exact(len).map_err(|_| {
        EngineError::new(
            ErrorCode::EngineOutOfMemory,
            "not enough memory for the trained finder's chain",
        )
    })?;
    Ok(values)
}
