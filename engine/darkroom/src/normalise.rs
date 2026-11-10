//! Percentile windows and the step from stored 16-bit values to floats.
//!
//! Percentiles come from a 4096-bin histogram over the range of the values
//! (evidence spec 2.5), counted in a fixed order, so the same film gives the
//! same window natively and in the browser. A percentile is read by linear
//! interpolation inside the bin where it falls, so it is accurate to one bin
//! width (range / 4096), not exact: a 16-bit film using its whole code range
//! gets bins 16 codes wide. The exact 2^bits histogram of stored codes that
//! film intake needs is film intake's own. Every threshold downstream is
//! written against this window, so it does not depend on the stored bit
//! depth or on how much of the code range a film uses.

/// Bins in the percentile histogram.
pub const HISTOGRAM_BINS: usize = 4096;

/// Two values that a grey level maps between: `low` goes to 0 and `high` to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Window {
    pub low: f64,
    pub high: f64,
}

impl Window {
    /// `(value − low) / (high − low)`, clamped to `[0, ceiling]`. A window
    /// with no width (or a width that is not a number) maps every value to
    /// 0, and so does a ceiling that is negative or not a number. A value
    /// that is not a number stays one.
    pub fn scale(&self, value: f64, ceiling: f64) -> f64 {
        let width = self.high - self.low;
        if width.is_nan() || width <= 0.0 || ceiling.is_nan() || ceiling < 0.0 {
            return 0.0;
        }
        ((value - self.low) / width).clamp(0.0, ceiling)
    }
}

/// The values at two fractions of the finite values in `values` (0 is the
/// least, 1 the most), read from the 4096-bin histogram. `None` when no
/// value is finite.
pub fn percentile_window(values: &[f32], low_fraction: f64, high_fraction: f64) -> Option<Window> {
    let finite = values
        .iter()
        .map(|&v| f64::from(v))
        .filter(|v| v.is_finite());
    let (min, max) = finite.clone().fold(None, |range, v| match range {
        None => Some((v, v)),
        Some((lo, hi)) => Some((lo.min(v), hi.max(v))),
    })?;
    let width = (max - min) / HISTOGRAM_BINS as f64;

    let mut counts = vec![0u64; HISTOGRAM_BINS];
    let mut total = 0u64;
    for v in finite {
        let bin = if width > 0.0 {
            (((v - min) / width) as usize).min(HISTOGRAM_BINS - 1)
        } else {
            0
        };
        counts[bin] += 1;
        total += 1;
    }

    Some(Window {
        low: percentile_value(&counts, total, min, width, low_fraction),
        high: percentile_value(&counts, total, min, width, high_fraction),
    })
}

/// Reads one fraction from the histogram: the position inside the bin where
/// the running count reaches `fraction · total`, interpolated linearly.
fn percentile_value(counts: &[u64], total: u64, min: f64, width: f64, fraction: f64) -> f64 {
    let target = fraction.clamp(0.0, 1.0) * total as f64;
    let mut before = 0.0;
    for (bin, &count) in counts.iter().enumerate() {
        let after = before + count as f64;
        if count > 0 && after >= target {
            let inside = (target - before) / count as f64;
            return min + (bin as f64 + inside) * width;
        }
        before = after;
    }
    min + HISTOGRAM_BINS as f64 * width
}

/// Stored grey levels to floats: each code goes through the window and is
/// clamped to `[0, ceiling]`. The ceiling is above 1 where the spec wants
/// bright metal kept from squashing bone into a narrow band.
pub fn stretch(codes: &[u16], window: Window, ceiling: f64) -> Vec<f32> {
    codes
        .iter()
        .map(|&code| window.scale(f64::from(code), ceiling) as f32)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One bin of a window over `[0, 1]`: the most a percentile may be off.
    const ONE_BIN: f64 = 1.0 / HISTOGRAM_BINS as f64;

    #[test]
    fn percentiles_of_an_even_ramp_are_the_fractions_themselves() {
        // 100001 values evenly spaced over [0, 1]: the p-th percentile is p.
        let ramp: Vec<f32> = (0..=100_000).map(|i| i as f32 / 100_000.0).collect();
        let window = percentile_window(&ramp, 0.01, 0.99).expect("values are finite");
        assert!((window.low - 0.01).abs() <= ONE_BIN, "low {}", window.low);
        assert!(
            (window.high - 0.99).abs() <= ONE_BIN,
            "high {}",
            window.high
        );
        let extremes = percentile_window(&ramp, 0.0, 1.0).expect("values are finite");
        // Both ends are exact. The range is [0, 1], so a bin is 2^−12 wide,
        // held exactly; the 100th percentile is the far edge of the last
        // bin, where the running count reaches the whole total and the
        // share inside is count / count = 1, so it is 4096 · 2^−12 = 1.
        assert_eq!(extremes.low, 0.0);
        assert_eq!(extremes.high, 1.0);
    }

    #[test]
    fn a_percentile_is_interpolated_inside_its_bin() {
        // Values 0 and 4096 set the range, so each bin is exactly 1 wide, and
        // eight values of 100.25 fill bin 100. The 30th percentile of ten
        // values is where the running count reaches 3; one value lies below
        // bin 100, so it is (3 − 1) / 8 = 1/4 of the way into the bin,
        // 100.25. The bin's start
        // (100) and its middle (100.5) are both wrong. Every number here is
        // exact in binary, so the answer is too.
        let mut values = vec![0.0f32, 4096.0];
        values.extend([100.25f32; 8]);
        let window = percentile_window(&values, 0.3, 0.5).expect("finite");
        assert_eq!(window.low, 100.25);
        // The 50th percentile is where the count reaches 5: (5 − 1) / 8 in.
        assert_eq!(window.high, 100.5);
    }

    #[test]
    fn the_window_does_not_depend_on_the_bit_depth() {
        // Codes from a 12-bit film, and the same codes written in 16 bits
        // (times 16). Both give identical stretched values: multiplying by
        // 16 is exact in binary, so the range, the bin width and both window
        // ends come out exactly 16 times larger, and every quotient in the
        // stretch is the same to the last bit.
        let codes_12: Vec<u16> = (0..4096u32).map(|i| ((i * 37) % 4096) as u16).collect();
        let codes_16: Vec<u16> = codes_12.iter().map(|&c| c * 16).collect();
        let as_float_12: Vec<f32> = codes_12.iter().map(|&c| f32::from(c)).collect();
        let as_float_16: Vec<f32> = codes_16.iter().map(|&c| f32::from(c)).collect();
        let window_12 = percentile_window(&as_float_12, 0.01, 0.95).expect("finite");
        let window_16 = percentile_window(&as_float_16, 0.01, 0.95).expect("finite");
        let stretched_12 = stretch(&codes_12, window_12, 1.5);
        let stretched_16 = stretch(&codes_16, window_16, 1.5);
        assert_eq!(stretched_12, stretched_16);
    }

    #[test]
    fn a_stored_code_maps_linearly_and_is_capped() {
        let unit = Window {
            low: 0.0,
            high: 4096.0,
        };
        assert_eq!(stretch(&[0, 2048, 4096], unit, 1.5), vec![0.0, 0.5, 1.0]);

        let narrow = Window {
            low: 0.0,
            high: 1000.0,
        };
        // 4095 / 1000 = 4.095 is capped at the ceiling of 1.5.
        assert_eq!(stretch(&[4095], narrow, 1.5), vec![1.5]);
    }

    #[test]
    fn empty_or_flat_input_gives_no_window_and_no_contrast() {
        assert_eq!(percentile_window(&[], 0.1, 0.9), None);
        assert_eq!(percentile_window(&[f32::NAN], 0.1, 0.9), None);
        let flat = percentile_window(&[5.0, 5.0, 5.0], 0.1, 0.9).expect("finite");
        assert_eq!((flat.low, flat.high), (5.0, 5.0));
        assert_eq!(flat.scale(5.0, 1.0), 0.0);
    }

    #[test]
    fn a_ceiling_that_is_negative_or_not_a_number_maps_to_zero() {
        let unit = Window {
            low: 0.0,
            high: 1.0,
        };
        assert_eq!(unit.scale(0.5, -1.0), 0.0);
        assert_eq!(unit.scale(0.5, f64::NAN), 0.0);
        assert!(unit.scale(f64::NAN, 1.0).is_nan());
    }
}
