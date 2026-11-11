//! Local autocorrelation scale from the strip barcode.

use super::median;
use crate::handoffs::Barcode;

/// Experimental local autocorrelation period. Windows span 15% of the
/// strip and overlap by 50%; accepted peaks must correlate by at least 0.2.
pub(super) fn local_period(barcode: &Barcode, row_step: f64) -> Option<f64> {
    let rows = barcode.values.len();
    let width = ((rows as f64 * 0.15).round() as usize).max(8);
    if width >= rows {
        return None;
    }
    let radius = ((rows as f64 * 0.05).round() as usize).max(1);
    let mut detrended = vec![0.0; rows];
    for (r, value) in detrended.iter_mut().enumerate() {
        if barcode.valid[r] == 0 || !barcode.values[r].is_finite() {
            continue;
        }
        let mut total = 0.0;
        let mut count = 0;
        for j in r.saturating_sub(radius)..(r + radius + 1).min(rows) {
            if barcode.valid[j] != 0 && barcode.values[j].is_finite() {
                total += f64::from(barcode.values[j]);
                count += 1;
            }
        }
        if count > 0 {
            *value = f64::from(barcode.values[r]) - total / count as f64;
        }
    }
    let min_lag = (rows / 30).max(2);
    let max_lag = (rows / 10).max(min_lag + 1).min(width - 2);
    if min_lag >= max_lag {
        return None;
    }
    let mut periods = Vec::new();
    for start in (0..=rows - width).step_by((width / 2).max(1)) {
        let correlation = |lag: usize| {
            let mut cross = 0.0;
            let mut first = 0.0;
            let mut second = 0.0;
            let mut count = 0;
            for r in start..start + width - lag {
                if barcode.valid[r] == 0
                    || barcode.valid[r + lag] == 0
                    || !barcode.values[r].is_finite()
                    || !barcode.values[r + lag].is_finite()
                {
                    continue;
                }
                cross += detrended[r] * detrended[r + lag];
                first += detrended[r] * detrended[r];
                second += detrended[r + lag] * detrended[r + lag];
                count += 1;
            }
            if count * 2 < width - lag || first <= 0.0 || second <= 0.0 {
                0.0
            } else {
                cross / libm::sqrt(first * second)
            }
        };
        let scores: Vec<_> = (min_lag..=max_lag).map(correlation).collect();
        for lag in min_lag + 1..max_lag {
            let value = scores[lag - min_lag];
            if value >= 0.2
                && value > scores[lag - min_lag - 1]
                && value >= scores[lag - min_lag + 1]
            {
                let half = lag / 2;
                let chosen = if half >= min_lag && correlation(half) >= 0.8 * value {
                    half
                } else {
                    lag
                };
                periods.push(chosen as f64 * row_step);
                break;
            }
        }
    }
    (periods.len() >= 3).then(|| median(&mut periods))
}
