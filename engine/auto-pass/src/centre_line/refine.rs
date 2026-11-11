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
                if width < 0.6 * prior[row]
                    || width > 1.5 * prior[row]
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
        measured[row] =
            confidence[row] >= 0.25 && (edge_width[row] - prior[row]).abs() <= 0.35 * prior[row];
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
