//! Shutters, saturated small objects, seams and their physical guard bands.

use super::conditioning::{like, local_sd, median, window};
use darkroom::grey_image::GreyImage;
use shapes::film_geometry::QuarterTurns;
use shapes::pixels::PxRect;

pub(super) struct MaskedField {
    pub valid: GreyImage<u8>,
    pub markers: Vec<PxRect>,
    pub seam_rows: Vec<f64>,
    pub central_marker: bool,
}

pub(super) fn find(
    image: &GreyImage<f32>,
    spacing: f64,
    turns: QuarterTurns,
    mut usable: Vec<u8>,
) -> MaskedField {
    let sd = local_sd(image, (2.0 / spacing).round().max(1.0) as u32);
    constant_borders(image, &sd, &mut usable);
    tilted_shutters(image, &sd, &mut usable, spacing);
    let (markers, central_marker) = markers(image, &mut usable, spacing, turns);
    let seam_rows = seams(image, &mut usable, spacing);
    MaskedField {
        valid: like(image, usable),
        markers,
        seam_rows,
        central_marker,
    }
}

fn side_shape(image: &GreyImage<f32>, side: usize) -> (usize, usize) {
    if side < 2 {
        (image.width() as usize, image.height() as usize)
    } else {
        (image.height() as usize, image.width() as usize)
    }
}

fn side_index(image: &GreyImage<f32>, side: usize, d: usize, t: usize) -> usize {
    let (w, h) = (image.width() as usize, image.height() as usize);
    match side {
        0 => t * w + d,
        1 => t * w + w - 1 - d,
        2 => d * w + t,
        _ => (h - 1 - d) * w + t,
    }
}

fn constant_borders(image: &GreyImage<f32>, sd: &GreyImage<f32>, valid: &mut [u8]) {
    for side in 0..4 {
        let (depth, length) = side_shape(image, side);
        let mut edge: Vec<f32> = (0..length)
            .map(|t| image.pixels()[side_index(image, side, 0, t)])
            .collect();
        let reference = median(&mut edge);
        for d in 0..(depth * 3 / 10) {
            let accepted = (0..length)
                .filter(|&t| {
                    let i = side_index(image, side, d, t);
                    (image.pixels()[i] - reference).abs() <= 1e-3 && sd.pixels()[i] < 1e-4
                })
                .count();
            if accepted * 100 < length * 98 {
                break;
            }
            for t in 0..length {
                valid[side_index(image, side, d, t)] = 0
            }
        }
    }
}

fn tilted_shutters(image: &GreyImage<f32>, sd: &GreyImage<f32>, valid: &mut [u8], spacing: f64) {
    for side in 0..4 {
        let (depth, length) = side_shape(image, side);
        let all_usable = valid.iter().all(|&v| v != 0);
        let offsets = depth * 3 / 10;
        // In side coordinates every profile is a contiguous row. The
        // derivative and the line's fractional shift are calculated once,
        // rather than four coordinate transforms per bilinear sample.
        // Central differences commute with bilinear interpolation.
        let mut derivative = vec![0.0f32; depth * length];
        let mut usable = vec![0u8; depth * length];
        for t in 0..length {
            for d in 1..depth - 1 {
                derivative[t * depth + d] = 0.5
                    * (image.pixels()[side_index(image, side, d + 1, t)]
                        - image.pixels()[side_index(image, side, d - 1, t)]);
                usable[t * depth + d] = valid[side_index(image, side, d, t)];
            }
        }
        let mut profiles = Vec::with_capacity(25 * offsets);
        let (mut best, mut best_offset, mut best_slope) = (0.0f32, 0usize, 0.0f64);
        let mut sums = vec![0.0f32; offsets];
        let mut counts = vec![0u32; offsets];
        for angle in -12..=12 {
            let slope = libm::tan(f64::from(angle) * 0.25 * std::f64::consts::PI / 180.0);
            sums.fill(0.0);
            counts.fill(0);
            for t in (1..length - 1).step_by(2) {
                let shift = slope * (t as f64 - (length - 1) as f64 / 2.0);
                let integer = shift.floor() as isize;
                let fraction = (shift - integer as f64) as f32;
                let start = (1.0 - shift).ceil().max(1.0) as usize;
                let end = ((depth - 2) as f64 - shift).floor().max(0.0) as usize + 1;
                let end = end.min(offsets);
                if start >= end {
                    continue;
                }
                let first = ((t * depth) as isize + integer + start as isize) as usize;
                let n = end - start;
                let a = &derivative[first..first + n];
                let b = &derivative[first + 1..first + n + 1];
                let sum = &mut sums[start..end];
                let count = &mut counts[start..end];
                if all_usable {
                    for (((out, number), &a), &b) in sum.iter_mut().zip(count).zip(a).zip(b) {
                        *out += (a * (1.0 - fraction) + b * fraction).abs();
                        *number += 1;
                    }
                } else {
                    let mask = &usable[first..first + n];
                    for ((((out, number), &a), &b), &on) in
                        sum.iter_mut().zip(count).zip(a).zip(b).zip(mask)
                    {
                        if on != 0 {
                            *out += (a * (1.0 - fraction) + b * fraction).abs();
                            *number += 1;
                        }
                    }
                }
            }
            for offset in 1..offsets {
                let score = if counts[offset] as usize * 4 >= length {
                    sums[offset] / counts[offset] as f32
                } else {
                    0.0
                };
                profiles.push(score);
                if score > best {
                    (best, best_offset, best_slope) = (score, offset, slope)
                }
            }
        }
        if best <= (4.0 * median(&mut profiles)).max(1e-4) {
            continue;
        }
        // A shutter is a coherent field edge. A bright spine ending in
        // otherwise flat air can win the mean-gradient search while
        // occupying only a short part of the line; do not cut the film
        // there. At least 60% of the available line must make a meaningful
        // step in the same direction (the field-edge coverage rule).
        let (mut samples, mut positive, mut negative) = (0usize, 0usize, 0usize);
        for t in (1..length - 1).step_by(2) {
            let d = best_offset as f64 + best_slope * (t as f64 - (length - 1) as f64 / 2.0);
            if d < 1.0 || d > (depth - 2) as f64 {
                continue;
            }
            let i = t * depth + d as usize;
            if usable[i] == 0 {
                continue;
            }
            samples += 1;
            let fraction = (d - d.floor()) as f32;
            let step = 2.0 * (derivative[i] * (1.0 - fraction) + derivative[i + 1] * fraction);
            positive += usize::from(step > 0.05);
            negative += usize::from(step < -0.05);
        }
        if samples == 0 || positive.max(negative) * 5 < samples * 3 {
            continue;
        }
        let inner_width = (5.0 / spacing).ceil() as usize;
        let (mut outer_values, mut inner_values, mut outer_sd, mut inner_sd) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for t in (0..length).step_by(4) {
            let boundary = best_offset as f64 + best_slope * (t as f64 - (length - 1) as f64 / 2.0);
            if boundary <= 1.0 || boundary >= (depth - 1) as f64 {
                continue;
            }
            let boundary = boundary as usize;
            // Stay two millimetres from the line when measuring the
            // background flatness, so its edge cannot contaminate it.
            let margin = (2.0 / spacing).ceil() as usize;
            for d in (0..boundary.saturating_sub(margin)).step_by(4) {
                let i = side_index(image, side, d, t);
                outer_values.push(image.pixels()[i]);
                outer_sd.push(sd.pixels()[i]);
            }
            for d in boundary + 1..(boundary + inner_width).min(depth) {
                let i = side_index(image, side, d, t);
                inner_values.push(image.pixels()[i]);
                inner_sd.push(sd.pixels()[i]);
            }
        }
        if outer_values.is_empty() || inner_values.is_empty() {
            continue;
        }
        let outside_noise = median(&mut outer_sd);
        if outside_noise >= 0.01
            || outside_noise >= 0.5 * median(&mut inner_sd)
            || (median(&mut outer_values) - median(&mut inner_values)).abs() <= 0.05
        {
            continue;
        }
        for t in 0..length {
            let boundary = best_offset as f64 + best_slope * (t as f64 - (length - 1) as f64 / 2.0);
            let end = boundary.ceil().clamp(0.0, depth as f64) as usize;
            for d in 0..end {
                valid[side_index(image, side, d, t)] = 0
            }
        }
    }
}

fn markers(
    image: &GreyImage<f32>,
    valid: &mut [u8],
    spacing: f64,
    turns: QuarterTurns,
) -> (Vec<PxRect>, bool) {
    let (w, h) = (image.width() as usize, image.height() as usize);
    let (lo, hi) = window(image.pixels(), Some(valid), 0.001, 0.999);
    let (lo, hi) = (lo.min(0.02), hi.max(0.98));
    let mut candidate: Vec<u8> = image
        .pixels()
        .iter()
        .zip(valid.iter())
        .map(|(&v, &on)| u8::from(on != 0 && (v <= lo || v >= hi)))
        .collect();
    let mut found = Vec::new();
    let mut central = false;
    let mut removed = vec![1u8; w * h];
    let mut component = Vec::new();
    let mut intensities = Vec::new();
    for seed in 0..w * h {
        if candidate[seed] == 0 {
            continue;
        }
        component.clear();
        component.push(seed);
        candidate[seed] = 0;
        let (mut left, mut right, mut top, mut bottom) = (seed % w, seed % w, seed / w, seed / w);
        let mut head = 0;
        while head < component.len() {
            let i = component[head];
            head += 1;
            let (x, y) = (i % w, i / w);
            left = left.min(x);
            right = right.max(x);
            top = top.min(y);
            bottom = bottom.max(y);
            for yy in y.saturating_sub(1)..=(y + 1).min(h - 1) {
                for xx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                    let j = yy * w + xx;
                    if candidate[j] != 0 {
                        candidate[j] = 0;
                        component.push(j)
                    }
                }
            }
        }
        if component.len() as f64 * spacing * spacing > 1500.0
            || (right - left + 1) as f64 * spacing > 60.0
            || (bottom - top + 1) as f64 * spacing > 60.0
        {
            continue;
        }
        intensities.clear();
        intensities.extend(component.iter().map(|&i| image.pixels()[i]));
        let inside = median(&mut intensities);
        intensities.clear();
        let band = (5.0 / spacing).ceil() as usize;
        for y in top.saturating_sub(band)..=(bottom + band).min(h - 1) {
            for x in left.saturating_sub(band)..=(right + band).min(w - 1) {
                if (x < left || x > right || y < top || y > bottom) && valid[y * w + x] != 0 {
                    intensities.push(image.pixels()[y * w + x]);
                }
            }
        }
        if intensities.is_empty() || (inside - median(&mut intensities)).abs() <= 0.15 {
            continue;
        }
        for &i in &component {
            removed[i] = 0
        }
        let a = image.film_point(left as f64 - 0.5, top as f64 - 0.5);
        let b = image.film_point(right as f64 + 0.5, bottom as f64 + 0.5);
        found.push(PxRect {
            left: a.x(),
            top: a.y(),
            right: b.x(),
            bottom: b.y(),
        });
        let (centre, across) = if turns.get().is_multiple_of(2) {
            (
                component.iter().map(|&i| (i % w) as f64).sum::<f64>() / component.len() as f64,
                w,
            )
        } else {
            (
                component.iter().map(|&i| (i / w) as f64).sum::<f64>() / component.len() as f64,
                h,
            )
        };
        central |= centre >= 0.3 * across as f64 && centre <= 0.7 * across as f64;
    }
    if !found.is_empty() {
        let expanded = dilate_invalid(&like(image, removed), 1.0 / spacing, false);
        for (on, &keep) in valid.iter_mut().zip(expanded.pixels()) {
            *on &= keep
        }
    }
    (found, central)
}

fn seams(image: &GreyImage<f32>, valid: &mut [u8], spacing: f64) -> Vec<f64> {
    let (w, h) = (image.width() as usize, image.height() as usize);
    let mut jumps = vec![0.0; h - 1];
    let mut samples = Vec::with_capacity(w);
    for (y, jump) in jumps.iter_mut().enumerate() {
        samples.clear();
        for x in 0..w {
            let i = y * w + x;
            if valid[i] != 0 && valid[i + w] != 0 {
                samples.push(image.pixels()[i + w] - image.pixels()[i])
            }
        }
        if !samples.is_empty() {
            *jump = median(&mut samples)
        }
    }
    let mut absolute: Vec<f32> = jumps.iter().map(|v| v.abs()).collect();
    let threshold = (5.0 * median(&mut absolute)).max(0.02);
    let mut rows = Vec::new();
    for (y, &jump) in jumps.iter().enumerate() {
        if jump.abs() <= threshold {
            continue;
        }
        let (mut count, mut agree) = (0, 0);
        for x in 0..w {
            let i = y * w + x;
            if valid[i] != 0 && valid[i + w] != 0 {
                count += 1;
                agree += usize::from((image.pixels()[i + w] - image.pixels()[i]) * jump > 0.0);
            }
        }
        if count > 0 && agree * 5 >= count * 4 {
            rows.push(y)
        }
    }
    let radius = (2.0 / spacing).ceil() as usize;
    for &y in &rows {
        for row in y.saturating_sub(radius)..=(y + 1 + radius).min(h - 1) {
            valid[row * w..(row + 1) * w].fill(0);
        }
    }
    rows.into_iter()
        .map(|y| image.film_point(0.0, y as f64 + 0.5).y())
        .collect()
}

pub(super) fn guard(valid: &GreyImage<u8>, spacing: f64) -> GreyImage<u8> {
    dilate_invalid(valid, 3.0 / spacing, true)
}

/// Exact squared Euclidean distance by two separable lower envelopes of
/// parabolas. Unlike repeated neighbourhood dilation its cost is linear
/// in the number of pixels, independent of the physical guard width.
fn dilate_invalid(valid: &GreyImage<u8>, radius: f64, border: bool) -> GreyImage<u8> {
    let (w, h) = (valid.width() as usize, valid.height() as usize);
    if valid.pixels().iter().all(|&v| v != 0) {
        let mut result = valid.pixels().to_vec();
        if border {
            let reach = radius.floor() as usize;
            for (y, row) in result.chunks_exact_mut(w).enumerate() {
                if y < reach || h - y <= reach {
                    row.fill(0);
                } else {
                    row[..reach.min(w)].fill(0);
                    row[w.saturating_sub(reach)..].fill(0);
                }
            }
        }
        return like(valid, result);
    }
    let far = (w * w + h * h) as f64;
    let mut horizontal = vec![0.0f64; w * h];
    let mut input = vec![0.0f64; w.max(h)];
    let mut output = vec![0.0f64; input.len()];
    let mut sites = vec![0usize; input.len()];
    let mut cuts = vec![0.0f64; input.len() + 1];
    for y in 0..h {
        for (x, value) in input[..w].iter_mut().enumerate() {
            *value = if valid.pixels()[y * w + x] == 0 {
                0.0
            } else {
                far
            };
        }
        distance_line(&input[..w], &mut output[..w], &mut sites, &mut cuts);
        horizontal[y * w..(y + 1) * w].copy_from_slice(&output[..w]);
    }
    let mut result = valid.pixels().to_vec();
    for x in 0..w {
        for y in 0..h {
            input[y] = horizontal[y * w + x]
        }
        distance_line(&input[..h], &mut output[..h], &mut sites, &mut cuts);
        for y in 0..h {
            let border_distance = (x + 1).min(w - x).min(y + 1).min(h - y) as f64;
            if output[y] <= radius * radius || (border && border_distance <= radius) {
                result[y * w + x] = 0;
            }
        }
    }
    like(valid, result)
}

fn distance_line(input: &[f64], output: &mut [f64], sites: &mut [usize], cuts: &mut [f64]) {
    let mut last = 0usize;
    sites[0] = 0;
    cuts[0] = f64::NEG_INFINITY;
    cuts[1] = f64::INFINITY;
    for q in 1..input.len() {
        let intersection = |p: usize| {
            ((input[q] + (q * q) as f64) - (input[p] + (p * p) as f64)) / (2 * (q - p)) as f64
        };
        let mut cut = intersection(sites[last]);
        while cut <= cuts[last] && last > 0 {
            last -= 1;
            cut = intersection(sites[last]);
        }
        last += 1;
        sites[last] = q;
        cuts[last] = cut;
        cuts[last + 1] = f64::INFINITY;
    }
    last = 0;
    for (q, value) in output.iter_mut().enumerate() {
        while cuts[last + 1] < q as f64 {
            last += 1
        }
        let delta = q as f64 - sites[last] as f64;
        *value = delta * delta + input[sites[last]];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkroom::grey_image::Placement;

    fn grid(w: u32, h: u32, f: impl Fn(u32, u32) -> f32) -> GreyImage<f32> {
        let mut values = Vec::new();
        for y in 0..h {
            for x in 0..w {
                values.push(f(x, y))
            }
        }
        GreyImage::new(w, h, values, Placement::FILM).unwrap()
    }

    #[test]
    fn the_guard_is_a_disc_and_includes_the_film_border() {
        let mask =
            grid(25, 25, |x, y| if (x, y) == (12, 12) { 0.0 } else { 1.0 }).map(|&v| v as u8);
        let guarded = guard(&mask, 1.0);
        for y in 0..25u32 {
            for x in 0..25u32 {
                let dx = f64::from(x) - 12.0;
                let dy = f64::from(y) - 12.0;
                let near_border = !(3..22).contains(&x) || !(3..22).contains(&y);
                let expected = u8::from(!near_border && dx * dx + dy * dy > 9.0);
                assert_eq!(guarded.get(x, y), Some(&expected));
            }
        }
    }

    #[test]
    fn a_full_width_step_is_a_seam_but_a_local_edge_is_not() {
        let image = grid(100, 140, |x, y| {
            0.2 + x as f32 * 0.0001
                + if y >= 70 { 0.1 } else { 0.0 }
                + if (40..60).contains(&x) && y >= 30 {
                    0.3
                } else {
                    0.0
                }
        });
        let mut valid = vec![1; 14_000];
        assert_eq!(seams(&image, &mut valid, 1.0), vec![69.5]);
        assert_eq!(valid[30 * 100 + 50], 1);
        assert_eq!(valid[70 * 100 + 50], 0);
    }

    #[test]
    fn small_saturated_text_is_masked_but_a_long_rod_is_not() {
        let image = grid(200, 400, |x, y| {
            if (20..30).contains(&x) && (30..40).contains(&y)
                || (130..134).contains(&x) && (50..350).contains(&y)
            {
                1.0
            } else {
                0.2 + 0.0001 * (x + y) as f32
            }
        });
        let mut valid = vec![1; 80_000];
        let (boxes, central) = markers(&image, &mut valid, 1.0, QuarterTurns::NONE);
        assert_eq!(boxes.len(), 1);
        assert!(!central);
        assert_eq!(
            boxes[0],
            PxRect {
                left: 19.5,
                top: 29.5,
                right: 29.5,
                bottom: 39.5
            }
        );
        assert_eq!(valid[35 * 200 + 25], 0);
        assert_eq!(valid[200 * 200 + 131], 1);
        // One-millimetre dilation covers a neighbour at distance one,
        // but does not erase the pixel two millimetres from the letter.
        assert_eq!(valid[35 * 200 + 19], 0);
        assert_eq!(valid[35 * 200 + 18], 1);
    }

    #[test]
    fn a_tilted_low_noise_border_is_masked_to_its_edge() {
        let slope = libm::tan(2.0 * std::f64::consts::PI / 180.0);
        let image = grid(180, 240, |x, y| {
            let boundary = 25.0 + slope * (f64::from(y) - 119.5);
            if f64::from(x) < boundary {
                0.1 + ((x * 3 + y * 7) % 11) as f32 * 0.0005
            } else {
                0.5 + ((x * 7 + y * 13) % 19) as f32 * 0.002
            }
        });
        let sd = local_sd(&image, 2);
        let mut valid = vec![1; 180 * 240];
        tilted_shutters(&image, &sd, &mut valid, 1.0);
        for y in [20, 120, 220] {
            let truth = (25.0 + slope * (y as f64 - 119.5)).ceil() as usize;
            let first = valid[y * 180..(y + 1) * 180]
                .iter()
                .position(|&v| v != 0)
                .unwrap();
            // The line search has one-pixel offsets and quarter-degree
            // angle spacing; its raster edge is within two pixels.
            assert!(
                (first as isize - truth as isize).abs() <= 2,
                "row {y}: {first}, expected {truth}"
            );
        }
    }

    #[test]
    fn a_short_anatomical_edge_cannot_cut_off_the_spine() {
        let image = grid(200, 400, |x, y| {
            0.1 + ((x * 3 + y * 7) % 11) as f32 * 0.0001
                + if (75..125).contains(&x) && (70..330).contains(&y) {
                    0.5
                } else {
                    0.0
                }
        });
        let sd = local_sd(&image, 2);
        let mut valid = vec![1; 200 * 400];
        tilted_shutters(&image, &sd, &mut valid, 1.0);
        // Both horizontal body ends lie in the searched outer 30%, with
        // low-noise air beyond. They cover only one quarter of the film,
        // so they cannot be accepted as a coherent collimator edge.
        assert!(valid.iter().all(|&v| v == 1));
    }
}
