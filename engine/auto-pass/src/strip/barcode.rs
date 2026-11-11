//! The bone and gap profile along the strip: bone reads positive, disc gaps
//! negative.

use crate::handoffs::{Barcode, Strip};
use shapes::errors::{EngineError, ErrorCode};

/// Preserve the central mean in `values`. For `high_pass`, use the lower
/// quartile across the central three quarters of the body, followed by a
/// 12-size-unit Gaussian high pass and a running 60-size-unit MAD. This
/// broader, robust projection limits bright posterior overlap without
/// taking the minimum of a noisy row. Invalid rows stay invalid.
pub fn build(strip: &Strip, size_factor: f64) -> Result<Barcode, EngineError> {
    let rows = strip.rows as usize;
    let cols = strip.cols as usize;
    if rows == 0
        || cols == 0
        || !strip.col_step.is_finite()
        || strip.col_step <= 0.0
        || !strip.row_step.is_finite()
        || strip.row_step <= 0.0
        || strip.layout.len() != rows
        || rows.checked_mul(cols) != Some(strip.values.len())
        || strip.valid.len() != strip.values.len()
        || !size_factor.is_finite()
        || !(0.45..=1.2).contains(&size_factor)
    {
        return Err(EngineError::new(
            ErrorCode::EngineOutOfSync,
            "invalid strip geometry for the barcode",
        ));
    }
    let middle = (cols - 1) as f64 / 2.0;
    let mut values = vec![f32::NAN; rows];
    let mut valid = vec![0; rows];
    let mut profile = vec![f32::NAN; rows];
    let mut samples = Vec::with_capacity(cols);
    for (row, layout) in strip.layout.iter().enumerate() {
        if !layout.half_width.is_finite() || layout.half_width < 0.0 {
            return Err(EngineError::new(
                ErrorCode::EngineOutOfSync,
                "invalid body width for the barcode",
            ));
        }
        // Width is twice half_width: 0.25 width = 0.5 half_width.
        let radius = 0.5 * layout.half_width;
        let first = (middle - radius / strip.col_step).ceil().max(0.0) as usize;
        let last = (middle + radius / strip.col_step)
            .floor()
            .min((cols - 1) as f64) as usize;
        let mut sum = 0.0;
        let mut count = 0;
        for col in first..=last {
            let index = row * cols + col;
            if strip.valid[index] != 0 && strip.values[index].is_finite() {
                sum += f64::from(strip.values[index]);
                count += 1;
            }
        }
        if count > 0 {
            values[row] = (sum / f64::from(count)) as f32;
            valid[row] = 1;
        }
        // The cortical side walls lie outside this interior band. At its
        // lower quartile, bright pedicles and central posterior overlap
        // cannot dominate unless they cover three quarters of the band.
        let radius = 0.75 * layout.half_width;
        let first = (middle - radius / strip.col_step).ceil().max(0.0) as usize;
        let last = (middle + radius / strip.col_step)
            .floor()
            .min((cols - 1) as f64) as usize;
        samples.clear();
        for col in first..=last {
            let index = row * cols + col;
            if strip.valid[index] != 0 && strip.values[index].is_finite() {
                samples.push(strip.values[index]);
            }
        }
        if !samples.is_empty() {
            let rank = (samples.len() - 1) / 4;
            profile[row] = *samples.select_nth_unstable_by(rank, f32::total_cmp).1;
        }
    }

    let sigma = 12.0 * size_factor / strip.row_step;
    let radius = (3.0 * sigma).ceil().min(rows as f64) as usize;
    let weights: Vec<f64> = (0..=radius)
        .map(|offset| libm::exp(-0.5 * (offset as f64 / sigma) * (offset as f64 / sigma)))
        .collect();
    let mut residuals = vec![f32::NAN; rows];
    for row in 0..rows {
        if valid[row] == 0 {
            continue;
        }
        let mut sum = 0.0;
        let mut weight = 0.0;
        for other in row.saturating_sub(radius)..=(row + radius).min(rows - 1) {
            if valid[other] != 0 {
                let w = weights[row.abs_diff(other)];
                sum += w * f64::from(profile[other]);
                weight += w;
            }
        }
        residuals[row] = (f64::from(profile[row]) - sum / weight) as f32;
    }
    let half_window = (30.0 * size_factor / strip.row_step)
        .floor()
        .min(rows as f64) as usize;
    let mut high_pass = vec![f32::NAN; rows];
    let mut window = Vec::with_capacity((2 * half_window + 1).min(rows));
    for row in 0..rows {
        if valid[row] == 0 {
            continue;
        }
        window.clear();
        window.extend(
            residuals[row.saturating_sub(half_window)..=(row + half_window).min(rows - 1)]
                .iter()
                .copied()
                .filter(|v| v.is_finite()),
        );
        let centre = median(&mut window);
        for value in &mut window {
            *value = (*value - centre).abs();
        }
        let mad = median(&mut window).max(1e-3);
        high_pass[row] = residuals[row] / mad;
    }
    Ok(Barcode {
        film: strip.film,
        values,
        high_pass,
        valid,
    })
}

fn median(values: &mut [f32]) -> f32 {
    let len = values.len();
    let middle = len / 2;
    let (left, value, _) = values.select_nth_unstable_by(middle, f32::total_cmp);
    if middle > 0 && left.len() * 2 == len {
        let lower = left
            .iter()
            .copied()
            .max_by(f32::total_cmp)
            .unwrap_or(*value);
        0.5 * (lower + *value)
    } else {
        *value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handoffs::{StripEvidence, StripRow};
    use shapes::film::FilmKind;
    use shapes::pixels::Px;

    fn striped() -> Strip {
        let rows = 401;
        let cols = 81;
        let mut values = Vec::new();
        for row in 0..rows {
            let s = f64::from(row) * 0.5;
            // The analytic gap centres are 20, 60, 100, 140 and 180 mm.
            let bone = 0.5 + 0.3 * libm::cos(std::f64::consts::TAU * s / 40.0);
            for col in 0..cols {
                // Pedicles outside the middle half must not enter the mean.
                values.push(if col == 10 || col == 70 {
                    1.5
                } else {
                    bone as f32
                });
            }
        }
        Strip {
            film: FilmKind::Front,
            rows,
            cols,
            row_step: 0.5,
            col_step: 0.5,
            scale_assumed: false,
            values,
            valid: vec![1; (rows * cols) as usize],
            layout: (0..rows)
                .map(|i| StripRow {
                    centre: Px::new(50.0, f64::from(i) * 0.5),
                    across: Px::new(0.5, 0.0),
                    along: Px::new(0.0, 0.5),
                    curvature: 0.0,
                    half_width: 20.0,
                    z: -f64::from(i) * 0.5,
                })
                .collect(),
            image_right: None,
            polarity_hint: None,
            evidence: StripEvidence::default(),
        }
    }

    #[test]
    fn periodic_gaps_stay_at_the_analytic_disc_centres() {
        let strip = striped();
        let barcode = build(&strip, 1.0).expect("valid strip");
        for gap in [60usize, 100, 140] {
            let row = gap * 2;
            let best = (row - 4..=row + 4)
                .min_by(|&a, &b| barcode.high_pass[a].total_cmp(&barcode.high_pass[b]))
                .expect("nine rows");
            // Interior periodic minima are exactly on a row; MAD and finite
            // Gaussian support can shift them at most one half-mm sample.
            assert!(best.abs_diff(row) <= 1, "gap {gap}, row {best}");
            assert!(barcode.high_pass[best] < 0.0);
        }
        assert!((barcode.values[200] - 0.2).abs() < 1e-6);
    }

    #[test]
    fn bright_central_overlap_does_not_move_the_disc_profile() {
        let mut strip = striped();
        for row in 0..strip.rows as usize {
            let arc = row as f64 * strip.row_step;
            // A second projected structure has its bright peaks at the
            // real gaps. It occupies the central 10 mm of a 40 mm body;
            // the broad interior still shows the true discs at 60/100/140.
            let overlap = 0.8 * (1.0 - libm::cos(std::f64::consts::TAU * arc / 40.0));
            for col in 30..=50 {
                strip.values[row * strip.cols as usize + col] += overlap as f32;
            }
        }
        let barcode = build(&strip, 1.0).unwrap();
        for gap in [60usize, 100, 140] {
            let row = gap * 2;
            let best = (row - 4..=row + 4)
                .min_by(|&a, &b| barcode.high_pass[a].total_cmp(&barcode.high_pass[b]))
                .unwrap();
            // The clean columns have an analytic minimum at the disc.
            assert!(best.abs_diff(row) <= 1);
            assert!(barcode.high_pass[best] < 0.0);
            // The frozen mean still reports the observed bright overlap.
            assert!(barcode.values[row] > 0.9);
        }
    }

    #[test]
    fn missing_rows_stay_missing_and_do_not_poison_the_neighbours() {
        let mut strip = striped();
        for index in 100 * strip.cols as usize..101 * strip.cols as usize {
            strip.values[index] = f32::NAN;
            strip.valid[index] = 0;
        }
        let barcode = build(&strip, 1.0).expect("valid strip");
        assert_eq!(barcode.valid[100], 0);
        assert!(barcode.values[100].is_nan());
        assert!(barcode.high_pass[100].is_nan());
        assert!(barcode.high_pass[99].is_finite());
        assert!(barcode.high_pass[101].is_finite());
    }

    #[test]
    fn a_constant_profile_has_no_gaps() {
        let mut strip = striped();
        strip.values.fill(0.7);
        let barcode = build(&strip, 0.6).expect("valid strip");
        assert!(barcode.high_pass.iter().all(|v| v.abs() < 1e-5));
    }
}
