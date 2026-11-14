//! Closed-form normal bands and percentiles over a set of versions.

use shapes::spine_readings::Band;

/// Exact normal percentiles for a fixed linear angle, in degrees. The
/// 90th standard-normal percentile is 1.2815515655446004; the lower tail
/// is its negative. No seed or finite ensemble enters this calculation.
pub(crate) fn normal_band(value_deg: f64, sd_deg: f64) -> Band {
    let half = 1.281_551_565_544_600_4 * sd_deg;
    Band {
        p10: value_deg - half,
        p50: value_deg,
        p90: value_deg + half,
    }
}

/// This project's percentile convention: linear interpolation at p*(n-1).
/// Empty or non-finite samples have no band.
pub fn band(values: &[f64]) -> Option<Band> {
    if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let quantile = |p: f64| {
        let index = p * (sorted.len() - 1) as f64;
        let low = libm::floor(index) as usize;
        let high = libm::ceil(index) as usize;
        sorted[low] + (sorted[high] - sorted[low]) * (index - low as f64)
    };
    Some(Band {
        p10: quantile(0.1),
        p50: quantile(0.5),
        p90: quantile(0.9),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_values_interpolate_by_hand() {
        assert_eq!(
            band(&[30.0, 10.0, 20.0]),
            Some(Band {
                p10: 12.0,
                p50: 20.0,
                p90: 28.0
            })
        );
        assert_eq!(
            band(&[7.0]),
            Some(Band {
                p10: 7.0,
                p50: 7.0,
                p90: 7.0
            })
        );
        assert_eq!(band(&[]), None);
        assert_eq!(band(&[f64::NAN]), None);
    }
}
