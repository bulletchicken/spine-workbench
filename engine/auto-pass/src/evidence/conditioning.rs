//! Robust intensities, normalised-convolution filling and local noise.

use crate::handoffs::Polarity;
use darkroom::blur::{box_blur, gaussian};
use darkroom::grey_image::GreyImage;

pub(super) fn median(values: &mut [f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let middle = values.len() / 2;
    *values.select_nth_unstable_by(middle, f32::total_cmp).1
}

/// A fixed 4096-bin histogram avoids sorting a film-sized copy.
pub(super) fn window(values: &[f32], valid: Option<&[u8]>, low: f64, high: f64) -> (f32, f32) {
    let usable = |i: usize| valid.is_none_or(|v| v[i] != 0);
    let (mut min, mut max) = (f32::INFINITY, f32::NEG_INFINITY);
    let mut count = 0usize;
    for (i, &value) in values.iter().enumerate() {
        if usable(i) {
            min = min.min(value);
            max = max.max(value);
            count += 1;
        }
    }
    if count == 0 {
        return (0.0, 0.0);
    }
    if min == max {
        return (min, max);
    }
    let mut bins = [0usize; 4096];
    let scale = 4095.0 / (max - min);
    for (i, &value) in values.iter().enumerate() {
        if usable(i) {
            bins[((value - min) * scale) as usize] += 1
        }
    }
    let quantile = |fraction: f64| {
        let target = fraction * (count - 1) as f64;
        let mut before = 0usize;
        for (i, &n) in bins.iter().enumerate() {
            if n != 0 && (before + n) as f64 > target {
                // Centre interpolation is bounded by the populated bin's
                // two edges, with the extrema restored exactly.
                let position = i as f64 + (target - before as f64) / n as f64 - 0.5;
                return (min + position as f32 / scale).clamp(min, max);
            }
            before += n;
        }
        max
    };
    (quantile(low), quantile(high))
}

pub(super) fn local_sd(image: &GreyImage<f32>, radius: u32) -> GreyImage<f32> {
    let mean = box_blur(image, radius);
    let square = box_blur(&image.map(|&v| v * v), radius);
    like(
        image,
        mean.pixels()
            .iter()
            .zip(square.pixels())
            .map(|(&m, &s)| (s - m * m).max(0.0).sqrt())
            .collect(),
    )
}

pub(super) fn stretch_and_fill(
    preliminary: &GreyImage<f32>,
    valid: &GreyImage<u8>,
    spacing: f64,
    polarity: Polarity,
) -> GreyImage<f32> {
    let oriented = preliminary.map(|&v| {
        if polarity == Polarity::BoneDark {
            1.0 - v
        } else {
            v
        }
    });
    let (low, high) = window(oriented.pixels(), Some(valid.pixels()), 0.005, 0.995);
    let image = oriented.map(|&v| ((v - low) / (high - low).max(1e-6)).clamp(0.0, 1.0));
    if valid.pixels().iter().all(|&v| v != 0) {
        return image;
    }
    let weights = valid.map(|&v| f32::from(v != 0));
    let weighted = like(
        &image,
        image
            .pixels()
            .iter()
            .zip(valid.pixels())
            .map(|(&v, &on)| if on != 0 { v } else { 0.0 })
            .collect(),
    );
    let count = valid.pixels().iter().filter(|&&v| v != 0).count().max(1);
    let mean = weighted.pixels().iter().map(|&v| f64::from(v)).sum::<f64>() / count as f64;
    let near_values = gaussian(&weighted, 4.0 / spacing).expect("bounded positive sigma");
    let near_weights = gaussian(&weights, 4.0 / spacing).expect("bounded positive sigma");
    let far_values = gaussian(&weighted, 32.0 / spacing).expect("bounded positive sigma");
    let far_weights = gaussian(&weights, 32.0 / spacing).expect("bounded positive sigma");
    let mut filled = image.pixels().to_vec();
    for (i, value) in filled.iter_mut().enumerate() {
        if valid.pixels()[i] == 0 {
            *value = ((f64::from(near_values.pixels()[i])
                + 1e-3 * f64::from(far_values.pixels()[i])
                + 1e-6 * mean)
                / (f64::from(near_weights.pixels()[i])
                    + 1e-3 * f64::from(far_weights.pixels()[i])
                    + 1e-6)) as f32;
        }
    }
    like(&image, filled)
}

pub(super) fn contrast(
    image: &GreyImage<f32>,
    valid: &GreyImage<u8>,
    spacing: f64,
) -> GreyImage<f32> {
    let mean = gaussian(image, 8.0 / spacing).expect("bounded positive sigma");
    let mean_square =
        gaussian(&image.map(|&v| v * v), 8.0 / spacing).expect("bounded positive sigma");
    let (w, h) = (image.width() as usize, image.height() as usize);
    let side = (32.0 / spacing).round().max(1.0) as usize;
    let (nx, ny) = (w.div_ceil(side), h.div_ceil(side));
    let mut triplets = vec![0u8; w * h];
    for y in 1..h - 1 {
        let above = &valid.pixels()[(y - 1) * w..y * w];
        let row = &valid.pixels()[y * w..(y + 1) * w];
        let below = &valid.pixels()[(y + 1) * w..(y + 2) * w];
        for (((out, &a), &b), &c) in triplets[y * w..(y + 1) * w]
            .iter_mut()
            .zip(above)
            .zip(row)
            .zip(below)
        {
            *out = a & b & c;
        }
    }
    let mut response = vec![0.0f32; w * h];
    let mut whole = vec![0u8; w * h];
    for y in 1..h - 1 {
        let above = &image.pixels()[(y - 1) * w..y * w];
        let row = &image.pixels()[y * w..(y + 1) * w];
        let below = &image.pixels()[(y + 1) * w..(y + 2) * w];
        let use_row = &triplets[y * w..(y + 1) * w];
        let out = &mut response[y * w..(y + 1) * w];
        let usable = &mut whole[y * w..(y + 1) * w];
        for x in 1..w - 1 {
            out[x] = (above[x - 1] - 2.0 * above[x] + above[x + 1] - 2.0 * row[x - 1]
                + 4.0 * row[x]
                - 2.0 * row[x + 1]
                + below[x - 1]
                - 2.0 * below[x]
                + below[x + 1])
                .abs();
            usable[x] = use_row[x - 1] & use_row[x] & use_row[x + 1];
        }
    }
    drop(triplets);
    let mut tiles = vec![None; nx * ny];
    let mut values = Vec::with_capacity(side * side);
    for ty in 0..ny {
        for tx in 0..nx {
            values.clear();
            for y in ty * side..((ty + 1) * side).min(h) {
                let start = y * w + tx * side;
                let end = y * w + ((tx + 1) * side).min(w);
                values.extend(
                    response[start..end]
                        .iter()
                        .zip(&whole[start..end])
                        .filter_map(|(&v, &on)| (on != 0).then_some(v)),
                );
            }
            let area = side.min(w - tx * side) * side.min(h - ty * side);
            if values.len() * 4 >= area {
                tiles[ty * nx + tx] = Some((median(&mut values) / (0.6745 * 6.0)).max(1e-4));
            }
        }
    }
    let global = if tiles.iter().any(Option::is_none) {
        values.clear();
        values.extend(
            response
                .iter()
                .zip(&whole)
                .filter_map(|(&v, &on)| (on != 0).then_some(v)),
        );
        (median(&mut values) / (0.6745 * 6.0)).max(1e-4)
    } else {
        1e-4
    };
    drop(response);
    drop(whole);
    let mut noise: Vec<f32> = tiles.iter().map(|&v| v.unwrap_or(global)).collect();
    for y in 0..ny {
        for x in 0..nx {
            if tiles[y * nx + x].is_none() {
                let mut neighbours = Vec::new();
                for yy in y.saturating_sub(1)..=(y + 1).min(ny - 1) {
                    for xx in x.saturating_sub(1)..=(x + 1).min(nx - 1) {
                        if let Some(value) = tiles[yy * nx + xx] {
                            neighbours.push(value)
                        }
                    }
                }
                if !neighbours.is_empty() {
                    noise[y * nx + x] = median(&mut neighbours)
                }
            }
        }
    }
    let columns: Vec<(usize, usize, f32)> = (0..w)
        .map(|x| {
            let fx = ((x as f64 + 0.5) / side as f64 - 0.5).clamp(0.0, (nx - 1) as f64);
            let ix = fx as usize;
            (ix, (ix + 1).min(nx - 1), (fx - ix as f64) as f32)
        })
        .collect();
    let mut result = vec![0.0; w * h];
    for y in 0..h {
        let fy = ((y as f64 + 0.5) / side as f64 - 0.5).clamp(0.0, (ny - 1) as f64);
        let iy = fy as usize;
        let dy = (fy - iy as f64) as f32;
        let top_row = &noise[iy * nx..(iy + 1) * nx];
        let below = (iy + 1).min(ny - 1);
        let bottom_row = &noise[below * nx..(below + 1) * nx];
        let mean_row = &mean.pixels()[y * w..(y + 1) * w];
        let square_row = &mean_square.pixels()[y * w..(y + 1) * w];
        for (((out, &(ix, right, dx)), &m), &s) in result[y * w..(y + 1) * w]
            .iter_mut()
            .zip(&columns)
            .zip(mean_row)
            .zip(square_row)
        {
            let top = top_row[ix] * (1.0 - dx) + top_row[right] * dx;
            let bottom = bottom_row[ix] * (1.0 - dx) + bottom_row[right] * dx;
            let sigma = top * (1.0 - dy) + bottom * dy;
            *out = ((s - m * m).max(0.0) + 9.0 * sigma * sigma).sqrt();
        }
    }
    like(image, result)
}

pub(super) fn like<T, U>(image: &GreyImage<T>, values: Vec<U>) -> GreyImage<U> {
    GreyImage::new(image.width(), image.height(), values, image.placement())
        .expect("same grid size")
}
