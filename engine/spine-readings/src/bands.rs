//! Percentiles over a complete set of versions.

use shapes::spine_readings::Band;

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
