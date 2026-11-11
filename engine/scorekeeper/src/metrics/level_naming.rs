//! Naming on geometric matches, independent of the labels used for corners.

use serde::{Deserialize, Serialize};
use shapes::levels::Level;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NamingScore {
    pub correct: usize,
    pub matched: usize,
    pub fraction: Option<f64>,
    pub chain_shift: Option<f64>,
}

/// Anatomical indices use the union of the usual T1–L5 order and any
/// transitional levels present in this comparison, so absent T13 does not
/// introduce a fictitious extra step at the thoracolumbar junction.
pub fn score(truth: &[Level], prediction: &[Level], matches: &[(usize, usize)]) -> NamingScore {
    let mut order = Level::MEASURED.to_vec();
    order.extend_from_slice(truth);
    order.extend_from_slice(prediction);
    order.sort_unstable();
    order.dedup();
    let mut shifts = Vec::with_capacity(matches.len());
    let mut correct = 0;
    for &(a, b) in matches {
        correct += usize::from(truth[a] == prediction[b]);
        let true_index = order
            .binary_search(&truth[a])
            .expect("truth level in order");
        let predicted_index = order
            .binary_search(&prediction[b])
            .expect("predicted level in order");
        shifts.push(predicted_index as f64 - true_index as f64);
    }
    shifts.sort_by(f64::total_cmp);
    let n = shifts.len();
    let chain_shift = if n == 0 {
        None
    } else if n.is_multiple_of(2) {
        Some((shifts[n / 2 - 1] + shifts[n / 2]) * 0.5)
    } else {
        Some(shifts[n / 2])
    };
    NamingScore {
        correct,
        matched: n,
        fraction: (n > 0).then(|| correct as f64 / n as f64),
        chain_shift,
    }
}
