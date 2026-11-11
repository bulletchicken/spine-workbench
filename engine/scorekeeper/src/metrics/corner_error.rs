//! Corner error in original pixels and relative to true vertebral height.

use super::{Quad, quad::distance};
use serde::{Deserialize, Serialize};
use shapes::levels::Level;

/// Corner samples for one level. Missing predictions provide no corner
/// samples and are counted by the separate found measure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CornerLevelScore {
    pub level: Level,
    pub pixels: Vec<f64>,
    pub normalised: Vec<f64>,
}

/// Distances in image-canonical TL, TR, BL, BR order.
pub fn distances(truth: &Quad, prediction: &Quad) -> [f64; 4] {
    let a = truth.x_canonical().points();
    let b = prediction.x_canonical().points();
    std::array::from_fn(|i| distance(a[i], b[i]))
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Distribution {
    pub count: usize,
    pub mean: f64,
    pub median: f64,
    pub p90: f64,
}

/// Median averages the middle pair; P90 is the nearest-rank percentile.
/// An empty sample has no summary, rather than a perfect score.
pub fn summarise(values: &[f64]) -> Option<Distribution> {
    if values.is_empty() || values.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    let median = if n.is_multiple_of(2) {
        (sorted[n / 2 - 1] + sorted[n / 2]) * 0.5
    } else {
        sorted[n / 2]
    };
    Some(Distribution {
        count: n,
        mean: values.iter().sum::<f64>() / n as f64,
        median,
        p90: sorted[(9 * n).div_ceil(10) - 1],
    })
}

/// Fraction strictly below a threshold, as specified for 0.10 and 0.25 h.
pub fn fraction_under(values: &[f64], threshold: f64) -> Option<f64> {
    (!values.is_empty())
        .then(|| values.iter().filter(|&&v| v < threshold).count() as f64 / values.len() as f64)
}
