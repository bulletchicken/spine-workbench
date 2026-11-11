//! Column-consensus contrast of a disc against its two flanks.

use super::{Statistics, median};
use crate::handoffs::{Polarity, Strip};

#[allow(clippy::too_many_arguments)]
pub(super) fn gap_table(
    strip: &Strip,
    usable: &[bool],
    polarity: Polarity,
    observed: &[bool],
    half_width: &[f64],
    step: f64,
    padding: usize,
    min_gap: usize,
    max_gap: usize,
) -> Vec<f64> {
    let rows = observed.len();
    let cols = strip.cols as usize;
    let gap_count = max_gap - min_gap + 1;
    // The narrowest centre band guarantees every selected column belongs
    // to the consensus band over the whole query, including both flanks.
    let radius = (0.3 * half_width.iter().copied().fold(f64::INFINITY, f64::min) / strip.col_step)
        .floor()
        .min(cols as f64) as usize;
    let left = (cols / 2).saturating_sub(radius);
    let right = (cols / 2 + radius + 1).min(cols);
    let mut prefix = vec![vec![0.0; rows + 1]; right - left];
    let mut valid_prefix = vec![vec![0usize; rows + 1]; right - left];
    for i in 0..rows {
        let native = (i as f64 - padding as f64) * step / strip.row_step;
        let start = native.max(0.0).floor() as usize;
        let end = ((native + step / strip.row_step).max(0.0).ceil() as usize)
            .max(start.saturating_add(1))
            .min(strip.rows as usize);
        for c in left..right {
            let mut sum = 0.0;
            let mut n = 0;
            if observed[i] && start < end && start < strip.rows as usize {
                for r in start..end {
                    if usable[r * cols + c] {
                        sum += f64::from(strip.values[r * cols + c]);
                        n += 1;
                    }
                }
            }
            prefix[c - left][i + 1] =
                prefix[c - left][i] + if n > 0 { sum / n as f64 } else { 0.0 };
            valid_prefix[c - left][i + 1] = valid_prefix[c - left][i] + usize::from(n > 0);
        }
    }
    let flank = if strip.scale_assumed {
        3
    } else {
        (3.0 / step).round().max(1.0) as usize
    };
    let mut raw = vec![None; rows * gap_count];
    let sign = if polarity == Polarity::BoneBright {
        1.0
    } else {
        -1.0
    };
    let mut contrasts = Vec::with_capacity(right - left);
    for bottom in flank..rows {
        for gap in min_gap..=max_gap {
            let end = bottom + gap + flank;
            if end > rows || observed[bottom - flank..end].iter().any(|v| !v) {
                continue;
            }
            contrasts.clear();
            for c in 0..prefix.len() {
                if valid_prefix[c][end] - valid_prefix[c][bottom - flank] != gap + 2 * flank {
                    continue;
                }
                let before = prefix[c][bottom] - prefix[c][bottom - flank];
                let after = prefix[c][end] - prefix[c][bottom + gap];
                let middle = prefix[c][bottom + gap] - prefix[c][bottom];
                contrasts
                    .push(sign * ((before + after) / (2 * flank) as f64 - middle / gap as f64));
            }
            if !contrasts.is_empty() && contrasts.len() * 5 >= prefix.len() * 2 {
                raw[bottom * gap_count + gap - min_gap] = Some(median(&mut contrasts));
            }
        }
    }
    let stats = Statistics::new(raw.iter().flatten().copied());
    raw.into_iter()
        .map(|v| v.map_or(0.0, |v| -stats.z(v).clamp(-2.0, 6.0)))
        .collect()
}
