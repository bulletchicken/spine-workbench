//! Native validity and signed, smoothed line evidence.

use super::median;
use crate::handoffs::{Polarity, Strip};

/// Dilate the frozen unusable mask by a 2-length-unit rectangle. Grey
/// values themselves cannot identify saturation after strip normalisation.
pub(super) fn dilated_validity(strip: &Strip) -> Vec<bool> {
    let rows = strip.rows as usize;
    let cols = strip.cols as usize;
    let mut prefix = vec![0usize; (rows + 1) * (cols + 1)];
    for r in 0..rows {
        for c in 0..cols {
            let bad = usize::from(
                strip.valid[r * cols + c] == 0 || !strip.values[r * cols + c].is_finite(),
            );
            prefix[(r + 1) * (cols + 1) + c + 1] =
                bad + prefix[r * (cols + 1) + c + 1] + prefix[(r + 1) * (cols + 1) + c]
                    - prefix[r * (cols + 1) + c];
        }
    }
    let rr = (2.0 / strip.row_step).ceil().min(rows as f64) as usize;
    let cr = (2.0 / strip.col_step).ceil().min(cols as f64) as usize;
    let mut usable = vec![false; rows * cols];
    for r in 0..rows {
        let top = r.saturating_sub(rr);
        let bottom = (r + rr + 1).min(rows);
        for c in 0..cols {
            let left = c.saturating_sub(cr);
            let right = (c + cr + 1).min(cols);
            let bad = prefix[bottom * (cols + 1) + right] + prefix[top * (cols + 1) + left]
                - prefix[top * (cols + 1) + right]
                - prefix[bottom * (cols + 1) + left];
            usable[r * cols + c] = bad == 0;
        }
    }
    usable
}

pub(super) fn signed_edges(strip: &Strip, usable: &[bool], polarity: Polarity) -> Vec<f64> {
    let rows = strip.rows as usize;
    let cols = strip.cols as usize;
    let mut numerator: Vec<f64> = strip
        .values
        .iter()
        .zip(usable)
        .map(|(&v, &ok)| if ok { f64::from(v) } else { 0.0 })
        .collect();
    let mut denominator: Vec<f64> = usable.iter().map(|&ok| f64::from(ok)).collect();
    for (across, sigma) in [(true, 0.5 / strip.col_step), (false, 0.75 / strip.row_step)] {
        let kernel = darkroom::blur::sampled_gaussian(sigma).expect("sample spacing was validated");
        numerator = convolve(&numerator, rows, cols, &kernel, across);
        denominator = convolve(&denominator, rows, cols, &kernel, across);
    }
    let smooth: Vec<f64> = numerator
        .iter()
        .zip(&denominator)
        .map(|(&v, &w)| if w > 0.0 { v / w } else { 0.0 })
        .collect();
    let sign = if polarity == Polarity::BoneBright {
        1.0
    } else {
        -1.0
    };
    let mut edges = vec![0.0; rows * cols];
    for r in 1..rows {
        for c in 0..cols {
            if usable[r * cols + c] && usable[(r - 1) * cols + c] {
                edges[r * cols + c] =
                    sign * (smooth[r * cols + c] - smooth[(r - 1) * cols + c]) / strip.row_step;
            }
        }
    }
    let mut magnitudes: Vec<_> = edges
        .iter()
        .zip(usable)
        .filter_map(|(&v, &ok)| ok.then_some(v.abs()))
        .collect();
    let median_value = median(&mut magnitudes);
    let mut deviations: Vec<_> = magnitudes
        .iter()
        .map(|v| (v - median_value).abs())
        .collect();
    let cap = median_value + 6.0 * 1.4826 * median(&mut deviations);
    // A clean barcode has more than half its pixels on exactly flat bone.
    let cap = if cap <= 1e-12 {
        magnitudes.iter().copied().fold(0.0, f64::max)
    } else {
        cap
    };
    for edge in &mut edges {
        *edge = edge.clamp(-cap, cap);
    }
    edges
}

fn convolve(values: &[f64], rows: usize, cols: usize, kernel: &[f64], across: bool) -> Vec<f64> {
    let mut result = vec![0.0; values.len()];
    let radius = kernel.len() as isize / 2;
    for r in 0..rows {
        for c in 0..cols {
            for (k, &weight) in kernel.iter().enumerate() {
                let offset = k as isize - radius;
                let rr = if across {
                    r
                } else {
                    (r as isize + offset).clamp(0, rows as isize - 1) as usize
                };
                let cc = if across {
                    (c as isize + offset).clamp(0, cols as isize - 1) as usize
                } else {
                    c
                };
                result[r * cols + c] += weight * values[rr * cols + cc];
            }
        }
    }
    result
}

pub(super) fn line_evidence(
    strip: &Strip,
    usable: &[bool],
    edges: &[f64],
    row: f64,
    tilt: f64,
    inner: f64,
    outer: f64,
) -> (f64, f64) {
    let cols = strip.cols as usize;
    let centre = cols / 2;
    let mut positive = 0.0;
    let mut negative = 0.0;
    let mut total = 0;
    let mut count = 0;
    let slope = libm::tan(tilt) / strip.row_step;
    for c in 0..cols {
        let offset = (c as f64 - centre as f64) * strip.col_step;
        if offset.abs() < inner || offset.abs() > outer {
            continue;
        }
        total += 1;
        let sample = row + offset * slope;
        if sample < 1.0 || sample > f64::from(strip.rows - 1) {
            continue;
        }
        let low = sample.floor() as usize;
        let high = (low + 1).min(strip.rows as usize - 1);
        if !usable[low * cols + c] || !usable[high * cols + c] {
            continue;
        }
        let fraction = sample - low as f64;
        let value = edges[low * cols + c] * (1.0 - fraction) + edges[high * cols + c] * fraction;
        positive += value.max(0.0);
        negative += (-value).max(0.0);
        count += 1;
    }
    if count == 0 || count * 5 < total * 2 {
        (0.0, 0.0)
    } else {
        (positive / count as f64, negative / count as f64)
    }
}
