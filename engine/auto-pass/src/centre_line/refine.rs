//! Exact cut-line refinement and coupled searches for the two body walls.

use darkroom::blur::{filter_separable, sampled_gaussian};
use darkroom::grey_image::{GreyImage, Placement};
use darkroom::hessian::DerivativeKernels;
use darkroom::sampling::bilinear;

use super::cost_map::{CostMap, percentile};
use super::smooth::{Point, Sample};

const PROFILE_HALF: i32 = 100;
const PROFILE_COLS: usize = 201;

fn profiles(map: &CostMap, curve: &[Sample], sigma_along: f64) -> GreyImage<f32> {
    let mut values = Vec::with_capacity(curve.len() * PROFILE_COLS);
    for sample in curve {
        for t in -PROFILE_HALF..=PROFILE_HALF {
            let point = Point {
                x: sample.point.x + f64::from(t) * sample.tangent.y,
                y: sample.point.y - f64::from(t) * sample.tangent.x,
            };
            let film = map.film_point(point);
            let value = map
                .fine
                .image_point(film)
                .and_then(|(x, y)| bilinear(&map.fine, x, y))
                .unwrap_or(0.0);
            values.push(value);
        }
    }
    let grid = GreyImage::new(
        PROFILE_COLS as u32,
        curve.len() as u32,
        values,
        Placement::FILM,
    )
    .expect("profile grid");
    filter_separable(
        &grid,
        &[1.0],
        &sampled_gaussian(sigma_along).expect("bounded sigma"),
    )
    .expect("odd kernels")
}

pub(crate) fn refine(map: &CostMap, guide: &[Sample], front: bool) -> (Vec<Point>, Vec<f64>) {
    let profiles = profiles(map, guide, 3.0);
    let rows = guide.len();
    let mut data = vec![0.0f32; rows * 51];
    for (i, sample) in guide.iter().enumerate() {
        let sigma = map.sample(&map.sigma, sample.point).clamp(4.0, 30.0);
        let kernels = DerivativeKernels::at_scale(sigma).expect("bounded scale");
        let radius = (kernels.second.len() / 2) as i32;
        let row = &profiles.pixels()[i * PROFILE_COLS..(i + 1) * PROFILE_COLS];
        for k in 0..51 {
            let at = k as i32 - 25 + PROFILE_HALF;
            let mut second = 0.0;
            for (j, &weight) in kernels.second.iter().enumerate() {
                let x = (at + j as i32 - radius).clamp(0, PROFILE_COLS as i32 - 1) as usize;
                second += weight * f64::from(row[x]);
            }
            data[i * 51 + k] = (-second * sigma * sigma).max(0.0) as f32;
        }
    }
    let scale = percentile(&data, 0.99).max(1e-6);
    for v in &mut data {
        *v = (*v / scale).min(1.0);
    }
    if !front {
        for (i, sample) in guide.iter().enumerate() {
            for k in 0..51 {
                let offset = k as f64 - 25.0;
                let point = Point {
                    x: sample.point.x + offset * sample.tangent.y,
                    y: sample.point.y - offset * sample.tangent.x,
                };
                data[i * 51 + k] = map.sample(&map.ridge, point) as f32;
            }
        }
    }
    let ridge = data.clone();
    if front {
        for (i, sample) in guide.iter().enumerate() {
            let half = (0.8 * map.sample(&map.sigma, sample.point))
                .round()
                .clamp(4.0, 24.0) as usize;
            let row = &profiles.pixels()[i * PROFILE_COLS..(i + 1) * PROFILE_COLS];
            for k in 0..51 {
                let at = k + 75;
                let mean = row[at - half..=at + half].iter().sum::<f32>() / (2 * half + 1) as f32;
                let mut difference = 0.0f32;
                let mut total = 0.0f32;
                for delta in 1..=half {
                    difference += (row[at + delta] - row[at - delta]).abs();
                    total +=
                        (row[at + delta] - mean).abs() + (row[at - delta] - mean).abs() + 0.001;
                }
                data[i * 51 + k] += 0.5 * (1.0 - difference / total).clamp(0.0, 1.0);
            }
        }
    }
    let mut previous = vec![0.0f32; 51];
    let mut next = vec![0.0f32; 51];
    let mut parents = vec![0u8; rows * 51];
    for i in 0..rows {
        for k in 0usize..51 {
            let mut best = f32::NEG_INFINITY;
            let mut parent = k;
            for (j, &previous_score) in previous
                .iter()
                .enumerate()
                .take((k + 3).min(51))
                .skip(k.saturating_sub(2))
            {
                let score = previous_score - 0.2 * ((k as f32 - j as f32) * (k as f32 - j as f32));
                if score > best {
                    best = score;
                    parent = j;
                }
            }
            next[k] = best + data[i * 51 + k];
            parents[i * 51 + k] = parent as u8;
        }
        std::mem::swap(&mut previous, &mut next);
    }
    let mut k = (0..51)
        .max_by(|&a, &b| previous[a].total_cmp(&previous[b]))
        .unwrap_or(25);
    let mut points = vec![Point::default(); rows];
    let mut weights = vec![0.0; rows];
    for i in (0..rows).rev() {
        let offset = k as f64 - 25.0;
        points[i] = Point {
            x: guide[i].point.x + offset * guide[i].tangent.y,
            y: guide[i].point.y - offset * guide[i].tangent.x,
        };
        weights[i] = 0.2 + f64::from(ridge[i * 51 + k]);
        k = parents[i * 51 + k] as usize;
    }
    (points, weights)
}

/// Widths at one-unit arc spacing, with a measured/fallback flag and confidence.
pub(crate) struct Widths {
    pub values: Vec<f64>,
    pub measured: Vec<bool>,
    pub confidence: Vec<f64>,
}

fn smooth_profile(values: &[f64], sigma: f64) -> Vec<f64> {
    let mut medians = Vec::with_capacity(values.len());
    for i in 0..values.len() {
        let window: Vec<_> = values[i.saturating_sub(7)..(i + 8).min(values.len())]
            .iter()
            .map(|&v| v as f32)
            .collect();
        medians.push(percentile(&window, 0.5));
    }
    let grid = GreyImage::new(1, values.len() as u32, medians, Placement::FILM).expect("column");
    darkroom::blur::gaussian(&grid, sigma)
        .expect("finite sigma")
        .pixels()
        .iter()
        .map(|&v| f64::from(v))
        .collect()
}

// The broad Hessian ridge can include the torso. A supported repetition
// period supplies a loose body aspect bound, while signed wall gradients
// still choose the actual width. No periodic signal means the existing
// Hessian width domain remains in use.
fn front_periods(profile: &GreyImage<f32>, size: f64) -> Vec<Option<f64>> {
    let rows = profile.height() as usize;
    let radius = (8.0 * size).round().max(3.0) as usize;
    let signal: Vec<f32> = (0..rows)
        .map(|row| {
            let row_values = &profile.pixels()[row * PROFILE_COLS..(row + 1) * PROFILE_COLS];
            row_values[100 - radius..=100 + radius].iter().sum::<f32>() / (2 * radius + 1) as f32
        })
        .collect();
    let grid =
        GreyImage::new(1, rows as u32, signal.clone(), Placement::FILM).expect("profile column");
    let baseline = darkroom::blur::gaussian(&grid, 12.0 * size).expect("bounded size");
    let signal: Vec<f64> = signal
        .iter()
        .zip(baseline.pixels())
        .map(|(a, b)| f64::from(a - b))
        .collect();
    // A 120-unit window contains several bodies. The first supported
    // autocorrelation maximum avoids choosing a multiple of the period.
    let half = (60.0 * size).round() as usize;
    let first = (8.0 * size).round().max(3.0) as usize;
    let last = (45.0 * size).round() as usize;
    let mut periods = vec![None; rows];
    for (row, period) in periods.iter_mut().enumerate() {
        let start = row.saturating_sub(half);
        let end = (row + half + 1).min(rows);
        let mut scores = Vec::new();
        for lag in first..=last {
            if end <= start + 2 * lag {
                scores.push(0.0);
                continue;
            }
            let (mut xy, mut xx, mut yy) = (0.0, 0.0, 0.0);
            for i in start..end - lag {
                xy += signal[i] * signal[i + lag];
                xx += signal[i] * signal[i];
                yy += signal[i + lag] * signal[i + lag];
            }
            scores.push(if xx * yy > 1e-20 {
                xy / libm::sqrt(xx * yy)
            } else {
                0.0
            });
        }
        for i in 1..scores.len() - 1 {
            if scores[i] >= 0.2 && scores[i] > scores[i - 1] && scores[i] >= scores[i + 1] {
                *period = Some((i + first) as f64);
                break;
            }
        }
    }
    // Isolated weak windows borrow the median of supported neighbours;
    // without at least 20 size-units of support they retain no estimate.
    let samples = periods.clone();
    for (row, period) in periods.iter_mut().enumerate() {
        let mut values: Vec<f32> = samples[row.saturating_sub(half)..(row + half + 1).min(rows)]
            .iter()
            .flatten()
            .map(|&v| v as f32)
            .collect();
        if values.len() >= half / 3 {
            let n = values.len() / 2;
            *period = Some(f64::from(
                *values.select_nth_unstable_by(n, f32::total_cmp).1,
            ));
        }
    }
    periods
}

pub(crate) fn widths(map: &CostMap, curve: &[Sample], front: bool) -> Widths {
    const SIDE: usize = 36;
    const STATES: usize = SIDE * SIDE;
    let profile = profiles(map, curve, 2.0);
    let derivative = DerivativeKernels::at_scale(1.5).expect("finite sigma");
    let gradient = filter_separable(&profile, &derivative.first, &[1.0]).expect("odd kernels");
    let normaliser = percentile(
        &gradient
            .pixels()
            .iter()
            .map(|v| v.abs())
            .collect::<Vec<_>>(),
        0.95,
    )
    .max(1e-6);
    let prior = smooth_profile(
        &curve
            .iter()
            .map(|s| 2.0 * map.sample(&map.sigma, s.point))
            .collect::<Vec<_>>(),
        10.0,
    );
    let rows = curve.len();
    let periods = if front {
        front_periods(&profile, map.size_factor)
    } else {
        vec![None; rows]
    };
    let mut previous = vec![0.0f32; STATES];
    let mut middle = vec![f32::NEG_INFINITY; STATES];
    let mut next = vec![f32::NEG_INFINITY; STATES];
    let mut middle_parent = vec![0u8; STATES];
    let mut parent = vec![0u16; rows * STATES];
    for row in 0..rows {
        for l in 0..SIDE {
            for r in 0..SIDE {
                let index = l * SIDE + r;
                let mut best = f32::NEG_INFINITY;
                let mut best_l = l;
                for previous_l in l.saturating_sub(2)..=(l + 2).min(SIDE - 1) {
                    let score = previous[previous_l * SIDE + r]
                        - 0.1 * ((l as f32 - previous_l as f32) * (l as f32 - previous_l as f32));
                    if score > best {
                        best = score;
                        best_l = previous_l;
                    }
                }
                middle[index] = best;
                middle_parent[index] = best_l as u8;
            }
        }
        for l in 0..SIDE {
            for r in 0..SIDE {
                let index = l * SIDE + r;
                let left = l as f64 - 40.0;
                let right = r as f64 + 5.0;
                let width = right - left;
                // A full body width may range from 0.7 to 2 repetition
                // periods. These broad bounds exclude narrow posterior
                // elements and distant torso walls; they do not prescribe
                // a width or replace the measured edge locations.
                let (minimum, maximum) = periods[row]
                    .map_or((0.6 * prior[row], 1.5 * prior[row]), |p| (0.7 * p, 2.0 * p));
                if width < minimum
                    || width > maximum
                    || (0.5 * (left + right)).abs() > 4.0f64.max(0.15 * prior[row])
                {
                    next[index] = f32::NEG_INFINITY;
                    continue;
                }
                let mut best = f32::NEG_INFINITY;
                let mut best_r = r;
                for previous_r in r.saturating_sub(2)..=(r + 2).min(SIDE - 1) {
                    let score = middle[l * SIDE + previous_r]
                        - 0.1 * ((r as f32 - previous_r as f32) * (r as f32 - previous_r as f32));
                    if score > best {
                        best = score;
                        best_r = previous_r;
                    }
                }
                let pixels = &gradient.pixels()[row * PROFILE_COLS..(row + 1) * PROFILE_COLS];
                let left_score = (pixels[(left + 100.0) as usize] / normaliser).clamp(0.0, 1.0);
                let right_score = (-pixels[(right + 100.0) as usize] / normaliser).clamp(0.0, 1.0);
                next[index] = best + left_score + right_score;
                parent[row * STATES + index] =
                    u16::from(middle_parent[l * SIDE + best_r]) * SIDE as u16 + best_r as u16;
            }
        }
        std::mem::swap(&mut previous, &mut next);
    }
    let mut state = (0..STATES)
        .max_by(|&a, &b| previous[a].total_cmp(&previous[b]))
        .unwrap_or(0);
    let mut measured = vec![false; rows];
    let mut confidence = vec![0.0; rows];
    let mut edge_width = prior.clone();
    for row in (0..rows).rev() {
        let (left, right) = ((state / SIDE) as i32 - 40, (state % SIDE) as i32 + 5);
        edge_width[row] = f64::from(right - left);
        let pixels = &gradient.pixels()[row * PROFILE_COLS..(row + 1) * PROFILE_COLS];
        confidence[row] = 0.5
            * f64::from(
                (pixels[(left + 100) as usize] / normaliser).clamp(0.0, 1.0)
                    + (-pixels[(right + 100) as usize] / normaliser).clamp(0.0, 1.0),
            );
        state = parent[row * STATES + state] as usize;
    }
    let mut values = prior.clone();
    let raw_confidence = confidence.clone();
    for row in 0..rows {
        let window = &raw_confidence[row.saturating_sub(10)..(row + 11).min(rows)];
        confidence[row] = window.iter().sum::<f64>() / window.len() as f64;
        // Only supported repetition can overrule an over-broad Hessian
        // prior. Without it, keep the original agreement gate as well.
        measured[row] = confidence[row] >= 0.25
            && (periods[row].is_some()
                || (edge_width[row] - prior[row]).abs() <= 0.35 * prior[row]);
        if measured[row] {
            values[row] = edge_width[row];
        }
    }
    let values = smooth_profile(&values, 10.0)
        .into_iter()
        .map(|v| {
            if front {
                v.clamp(10.0, 65.0)
            } else {
                v.clamp(9.0, 55.0)
            }
        })
        .collect();
    Widths {
        values,
        measured,
        confidence,
    }
}

#[cfg(test)]
mod width_tests {
    use super::*;

    fn fixture(half: f64, prior: f64, repeating: bool) -> (CostMap, Vec<Sample>) {
        let (w, h) = (201, 241);
        let mut pixels = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let offset = (f64::from(x) - 100.0).abs();
                let outer = 0.8 * (30.5 - offset).clamp(0.0, 1.0);
                let body = if !repeating || y % 20 < 16 {
                    0.3 * (half + 0.5 - offset).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                pixels.push((outer + body) as f32);
            }
        }
        let image = GreyImage::new(w, h, pixels, Placement::FILM).unwrap();
        let count = (w * h) as usize;
        let map = CostMap {
            image: image.clone(),
            fine: image,
            spacing: 1.0,
            ridge: vec![1.0; count],
            sigma: vec![(prior / 2.0) as f32; count],
            along: vec![Point { x: 0.0, y: 1.0 }; count],
            cost: vec![0.0; count],
            top: vec![],
            bottom: vec![],
            top_cropped: true,
            bottom_cropped: true,
            neck_y: None,
            crest_y: None,
            size_factor: 1.0,
            warnings: vec![],
        };
        let curve = (0..=160)
            .map(|i| Sample {
                arc: f64::from(i),
                point: Point {
                    x: 100.0,
                    y: 40.0 + f64::from(i),
                },
                tangent: Point { x: 0.0, y: 1.0 },
                curvature: 0.0,
            })
            .collect();
        (map, curve)
    }

    #[test]
    fn repeated_body_walls_beat_stronger_continuous_outer_edges() {
        // Actual body width 26, repetition 20, torso width 60. The broad
        // ridge prior 50 includes the torso and excludes the body under
        // the previous 0.6-prior bound. Truth never enters the estimator.
        let (map, curve) = fixture(13.0, 50.0, true);
        let result = widths(&map, &curve, true);
        for row in 30..130 {
            assert!(
                (result.values[row] - 26.0).abs() < 2.0,
                "row{row}: {}",
                result.values[row]
            );
            assert!(result.measured[row]);
        }
    }

    #[test]
    fn a_nonrepeating_body_retains_the_existing_width_domain() {
        let (map, curve) = fixture(15.0, 30.0, false);
        let profile = profiles(&map, &curve, 2.0);
        assert!(front_periods(&profile, 1.0).iter().all(Option::is_none));
        let result = widths(&map, &curve, true);
        for row in 30..130 {
            assert!((result.values[row] - 30.0).abs() < 2.0);
            assert!(result.measured[row]);
        }
    }

    #[test]
    fn unsupported_repetition_cannot_promote_a_rejected_wall_measurement() {
        // A 36 mm body is visible within the 26 mm prior's search domain,
        // but outside its original 35% agreement limit (35.1 mm). With no
        // repeated bodies, that estimate must remain explicitly unmeasured.
        let (map, curve) = fixture(18.0, 26.0, false);
        let result = widths(&map, &curve, true);
        assert!(result.measured.iter().all(|measured| !measured));
        assert!(
            result
                .values
                .iter()
                .all(|width| (*width - 26.0).abs() < 1e-5)
        );
    }

    #[test]
    fn a_flat_image_does_not_invent_measured_walls_or_a_period() {
        let (mut map, curve) = fixture(15.0, 30.0, false);
        map.fine = map.fine.map(|_| 0.5);
        let result = widths(&map, &curve, true);
        assert!(result.measured.iter().all(|v| !*v));
        assert!(result.values.iter().all(|v| v.is_finite()));
    }
}
