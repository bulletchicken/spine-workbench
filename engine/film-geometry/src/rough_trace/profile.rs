//! Coarse body profiles and a continuous ridge of bright bone.

use super::{median, percentile};
use darkroom::grey_image::{GreyImage, Placement};
use shapes::film_geometry::{PolarityGuess, QuarterTurns};
use shapes::pixels::Px;

/// Measurements in one row of a proposed upright working copy.
#[derive(Clone, Copy, Debug)]
pub struct TraceRow {
    pub centre: Option<f64>,
    pub width: f64,
    pub mass: f64,
    pub spine_x: Option<f64>,
}

/// A profile in working pixels. Use `placement` before reporting a point.
#[derive(Clone, Debug)]
pub struct Profile {
    pub width: usize,
    pub height: usize,
    pub rows: Vec<TraceRow>,
    pub placement: Placement,
    body_fraction: f64,
}

impl Profile {
    /// The body's median centroid and width over its central 60%, not the
    /// spine's curve. A tightly filled field has no reliable side reference.
    pub fn midline(&self) -> Option<(f64, f64)> {
        if self.body_fraction > 0.95 {
            return None;
        }
        let middle = &self.rows[self.height / 5..4 * self.height / 5];
        let centres = middle.iter().filter_map(|r| r.centre).collect();
        let widths = middle
            .iter()
            .filter(|r| r.centre.is_some())
            .map(|r| r.width)
            .collect();
        let width = median(widths)?;
        (width > 0.0).then_some((median(centres)?, width))
    }

    /// The coarse bone trace, in original film pixels.
    pub fn points(&self) -> Vec<Px> {
        self.rows
            .iter()
            .enumerate()
            .filter_map(|(y, row)| row.spine_x.map(|x| self.placement.film_point(x, y as f64)))
            .collect()
    }
}

pub(super) fn build(image: &GreyImage<f32>, mask: &[bool], turns: QuarterTurns) -> Profile {
    let (image, valid) = turned(image, mask, turns.get());
    let (w, h) = (image.width() as usize, image.height() as usize);
    let threshold = otsu(image.pixels(), &valid);
    let mut body_count = 0;
    let mut rows = Vec::with_capacity(h);
    for y in 0..h {
        let mut sum = 0.0;
        let mut count = 0;
        let mut first = w;
        let mut last = 0;
        let mut mass = 0.0;
        for x in 0..w {
            let i = y * w + x;
            if valid[i] {
                mass += f64::from(image.pixels()[i]);
                if image.pixels()[i] > threshold {
                    sum += x as f64;
                    count += 1;
                    first = first.min(x);
                    last = last.max(x);
                }
            }
        }
        body_count += count;
        rows.push(TraceRow {
            centre: (count > 0).then(|| sum / count as f64),
            width: if count > 0 {
                (last - first + 1) as f64
            } else {
                0.0
            },
            mass: mass / w.max(1) as f64,
            spine_x: None,
        });
    }
    trace_bone(&image, &valid, &mut rows);
    Profile {
        width: w,
        height: h,
        rows,
        placement: image.placement(),
        body_fraction: body_count as f64 / valid.iter().filter(|&&v| v).count().max(1) as f64,
    }
}

/// Between-class variance on 256 bins. Ties keep the lower threshold.
fn otsu(pixels: &[f32], valid: &[bool]) -> f32 {
    let mut histogram = [0.0f64; 256];
    for (&v, &keep) in pixels.iter().zip(valid) {
        if keep {
            histogram[(v.clamp(0.0, 1.0) * 255.0) as usize] += 1.0;
        }
    }
    let total: f64 = histogram.iter().sum();
    let sum: f64 = histogram
        .iter()
        .enumerate()
        .map(|(i, n)| i as f64 * n)
        .sum();
    let (mut count, mut partial, mut best, mut threshold) = (0.0, 0.0, 0.0, 0);
    for (i, &n) in histogram.iter().enumerate() {
        count += n;
        partial += i as f64 * n;
        if count == 0.0 || count == total {
            continue;
        }
        let difference = partial / count - (sum - partial) / (total - count);
        let score = count * (total - count) * difference * difference;
        if score > best {
            best = score;
            threshold = i;
        }
    }
    // The background includes the entire winning bin, not just its centre.
    (threshold as f32 + 1.0) / 255.0
}

fn trace_bone(image: &GreyImage<f32>, valid: &[bool], rows: &mut [TraceRow]) {
    let (w, h) = (image.width() as usize, image.height() as usize);
    if w < 8 || h < 8 {
        return;
    }
    // A body-width window suppresses soft tissue; the narrow mean keeps
    // endplates and bodies, instead of chasing individual bright pixels.
    // These fractions and the continuity cost are experimental coarse cues.
    let narrow = (w / 30).max(1);
    let wide = (w / 7).max(narrow + 1);
    let mut reward = vec![0.0; w * h];
    for y in 0..h {
        let mut sums = vec![0.0; w + 1];
        let mut counts = vec![0usize; w + 1];
        for x in 0..w {
            counts[x + 1] = counts[x] + usize::from(valid[y * w + x]);
            sums[x + 1] = sums[x]
                + if valid[y * w + x] {
                    f64::from(image.pixels()[y * w + x])
                } else {
                    0.0
                };
        }
        let mean = |x: usize, r: usize| {
            let (a, b) = (x.saturating_sub(r), (x + r + 1).min(w));
            (sums[b] - sums[a]) / (counts[b] - counts[a]).max(1) as f64
        };
        for x in w / 8..7 * w / 8 {
            if valid[y * w + x] {
                reward[y * w + x] = (mean(x, narrow) - mean(x, wide)).max(0.0);
            }
        }
    }
    let mut previous = vec![0.0; w];
    let mut parent = vec![0usize; w * h];
    for y in 0..h {
        let mut current = vec![f64::NEG_INFINITY; w];
        for x in w / 8..7 * w / 8 {
            let (mut best, mut from) = (f64::NEG_INFINITY, x);
            for (old, &previous_score) in previous
                .iter()
                .enumerate()
                .take((x + 3).min(7 * w / 8 - 1) + 1)
                .skip(x.saturating_sub(3).max(w / 8))
            {
                let distance = x.abs_diff(old) as f64;
                let value = previous_score - 0.025 * distance * distance;
                if value > best {
                    best = value;
                    from = old;
                }
            }
            current[x] = best + reward[y * w + x];
            parent[y * w + x] = from;
        }
        previous = current;
    }
    let mut x = (w / 8..7 * w / 8)
        .max_by(|&a, &b| previous[a].total_cmp(&previous[b]))
        .unwrap_or(w / 2);
    for y in (0..h).rev() {
        if reward[y * w + x] > 0.015 && rows[y].mass > 0.01 {
            rows[y].spine_x = Some(x as f64);
        }
        x = parent[y * w + x];
    }
}

pub(super) fn polarity(image: &GreyImage<f32>, valid: &[bool]) -> PolarityGuess {
    let q = u8::from(image.width() > image.height());
    let (image, valid) = turned(image, valid, q);
    let (w, h) = (image.width() as usize, image.height() as usize);
    let mut spine = Vec::new();
    let mut air = [Vec::new(), Vec::new()];
    for y in h / 5..4 * h / 5 {
        for x in 0..w {
            if !valid[y * w + x] {
                continue;
            }
            let value = image.pixels()[y * w + x];
            if x >= 2 * w / 5 && x < 3 * w / 5 {
                spine.push(value);
            }
            if x < (6 * w / 100).max(1) {
                air[0].push(value);
            }
            if x >= w.saturating_sub((6 * w / 100).max(1)) {
                air[1].push(value);
            }
        }
    }
    if spine.is_empty() || air.iter().any(Vec::is_empty) {
        return PolarityGuess::Unknown;
    }
    spine.sort_by(f32::total_cmp);
    let (lo, hi) = (percentile(&spine, 0.10), percentile(&spine, 0.90));
    let normal = air
        .iter()
        .all(|a| a.iter().filter(|&&v| v < lo).count() as f64 >= 0.15 * a.len() as f64);
    let inverted = air
        .iter()
        .all(|a| a.iter().filter(|&&v| v > hi).count() as f64 >= 0.15 * a.len() as f64);
    match (normal, inverted) {
        (true, false) => PolarityGuess::AsStored,
        (false, true) => PolarityGuess::LooksInverted,
        _ => PolarityGuess::Unknown,
    }
}

fn turned(image: &GreyImage<f32>, valid: &[bool], q: u8) -> (GreyImage<f32>, Vec<bool>) {
    let (w, h) = (image.width(), image.height());
    if w == 0 || h == 0 {
        return (image.clone(), valid.to_vec());
    }
    let (ow, oh) = if q.is_multiple_of(2) { (w, h) } else { (h, w) };
    let source = |x: u32, y: u32| match q {
        1 => (y, h - 1 - x),
        2 => (w - 1 - x, h - 1 - y),
        3 => (w - 1 - y, x),
        _ => (x, y),
    };
    let p = image.placement();
    let negate = |v: Px| Px::new(-v.x(), -v.y());
    let (origin, across, down) = match q {
        1 => (
            image.film_point(0.0, f64::from(h - 1)),
            negate(p.step_down),
            p.step_across,
        ),
        2 => (
            image.film_point(f64::from(w - 1), f64::from(h - 1)),
            negate(p.step_across),
            negate(p.step_down),
        ),
        3 => (
            image.film_point(f64::from(w - 1), 0.0),
            p.step_down,
            negate(p.step_across),
        ),
        _ => (p.origin, p.step_across, p.step_down),
    };
    let mut pixels = Vec::with_capacity(image.pixels().len());
    let mut mask = Vec::with_capacity(valid.len());
    for y in 0..oh {
        for x in 0..ow {
            let (sx, sy) = source(x, y);
            let i = (sy * w + sx) as usize;
            pixels.push(image.pixels()[i]);
            mask.push(valid[i]);
        }
    }
    (
        GreyImage::new(
            ow,
            oh,
            pixels,
            Placement {
                origin,
                step_across: across,
                step_down: down,
            },
        )
        .expect("a permutation preserves the pixel count"),
        mask,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_turned_working_point_maps_back_to_its_original_pixel() {
        let image =
            GreyImage::new(8, 12, (0..96).map(|x| x as f32).collect(), Placement::FILM).unwrap();
        for q in 0..4 {
            let (copy, _) = turned(&image, &[true; 96], q);
            for y in 0..copy.height() {
                for x in 0..copy.width() {
                    let p = copy.film_point(f64::from(x), f64::from(y));
                    assert_eq!(copy.get(x, y), image.get(p.x() as u32, p.y() as u32));
                }
            }
        }
    }

    #[test]
    fn a_flat_field_has_no_trace_or_polarity_proposal() {
        let image = GreyImage::new(40, 80, vec![0.0; 3200], Placement::FILM).unwrap();
        let valid = vec![true; 3200];
        let profile = build(&image, &valid, QuarterTurns::NONE);
        assert!(profile.points().is_empty());
        assert!(profile.midline().is_none());
        assert_eq!(polarity(&image, &valid), PolarityGuess::Unknown);
    }

    #[test]
    fn an_excluded_border_does_not_create_a_bone_ridge() {
        let image = GreyImage::new(80, 120, vec![1.0; 9600], Placement::FILM).unwrap();
        let valid: Vec<_> = (0..9600).map(|i| (10..70).contains(&(i % 80))).collect();
        let profile = build(&image, &valid, QuarterTurns::NONE);
        assert!(profile.points().is_empty());
    }
}
