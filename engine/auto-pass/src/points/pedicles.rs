//! Conservative pedicle visibility from calibrated, masked ring evidence.

use crate::handoffs::{FoundBody, Strip};
use shapes::marks::{MarkSource, PedicleMark};

use super::back_to_film::{sample, strip_to_film};
use super::walls::BodyCorners;

const NOT_VISIBLE: PedicleMark = PedicleMark::NotVisible {
    source: MarkSource::Auto,
};

/// The absolute floor is experimental and deliberately conservative. A
/// robust z score alone promotes arbitrarily weak noise on a ring-free
/// film; 0.2 also requires a fifth of the ideal-ring calibrated response.
const MIN_RING_RESPONSE: f64 = 0.2;

pub(super) struct RingEvidence<'a> {
    ring: &'a [f32],
    median: f64,
    noise: f64,
}

/// Compute the robust background once for the whole film.
pub(super) fn evidence(strip: &Strip) -> Option<RingEvidence<'_>> {
    let ring = strip.evidence.ring.as_deref()?;
    let mut background: Vec<f64> = ring
        .iter()
        .zip(&strip.valid)
        .filter_map(|(&value, &valid)| {
            (valid != 0 && value.is_finite()).then_some(f64::from(value))
        })
        .collect();
    if background.is_empty() {
        return None;
    }
    let median = middle(&mut background);
    for value in &mut background {
        *value = (*value - median).abs();
    }
    let noise = 1.4826 * middle(&mut background) + 1e-6;
    Some(RingEvidence {
        ring,
        median,
        noise,
    })
}

pub(super) fn find(
    strip: &Strip,
    body: &FoundBody,
    corners: &BodyCorners,
    evidence: Option<&RingEvidence<'_>>,
) -> [PedicleMark; 2] {
    let Some(evidence) = evidence else {
        return [NOT_VISIBLE; 2];
    };
    if body.inferred || corners.half_width <= 0.0 {
        return [NOT_VISIBLE; 2];
    }
    [-1.0, 1.0].map(|side| {
        one(
            strip,
            evidence.ring,
            corners,
            side,
            evidence.median,
            evidence.noise,
        )
    })
}

fn middle(values: &mut [f64]) -> f64 {
    let n = values.len();
    values.sort_unstable_by(f64::total_cmp);
    (values[(n - 1) / 2] + values[n / 2]) * 0.5
}

fn one(
    strip: &Strip,
    ring: &[f32],
    corners: &BodyCorners,
    side: f64,
    median: f64,
    noise: f64,
) -> PedicleMark {
    let centre = f64::from(strip.cols - 1) * 0.5;
    let low = (0.25 * corners.half_width / strip.col_step).ceil() as usize;
    let high = (0.85 * corners.half_width / strip.col_step).floor() as usize;
    let mut candidates = Vec::new();
    let mut best = None;
    let mut maximum = 0.0;
    for distance in low..=high.min(strip.cols as usize / 2) {
        let col = (centre + side * distance as f64) as usize;
        let offset = side * distance as f64 * strip.col_step;
        let top = corners.top.row + offset * libm::tan(corners.top.tilt) / strip.row_step;
        let bottom = corners.bottom.row + offset * libm::tan(corners.bottom.tilt) / strip.row_step;
        let height = bottom - top;
        if height <= 0.0 {
            continue;
        }
        let first = (top + 0.20 * height).ceil().max(0.0) as usize;
        let last = (top + 0.44 * height).floor().min(f64::from(strip.rows - 1));
        if last < first as f64 {
            continue;
        }
        for row in first..=last as usize {
            let Some(value) = sample(strip, ring, row as f64, offset) else {
                continue;
            };
            let index = row * strip.cols as usize + col;
            candidates.push((index, value));
            if value > maximum {
                maximum = value;
                best = Some(index);
            }
        }
    }
    if maximum < MIN_RING_RESPONSE || (maximum - median) / noise < 2.0 {
        return NOT_VISIBLE;
    }
    let Some(start) = best else {
        return NOT_VISIBLE;
    };
    // Keep only the connected half-height lobe containing the maximum.
    // Averaging a separate ring would put the mark between two structures.
    let mut remaining: std::collections::BTreeMap<usize, f64> = candidates
        .into_iter()
        .filter(|(_, value)| *value >= 0.5 * maximum)
        .collect();
    let mut pending = vec![start];
    let (mut weight, mut sum_row, mut sum_col) = (0.0, 0.0, 0.0);
    while let Some(index) = pending.pop() {
        let Some(value) = remaining.remove(&index) else {
            continue;
        };
        let (row, col) = (index / strip.cols as usize, index % strip.cols as usize);
        weight += value;
        sum_row += value * row as f64;
        sum_col += value * col as f64;
        for (dr, dc) in [(-1_isize, 0_isize), (1, 0), (0, -1), (0, 1)] {
            if let (Some(r), Some(c)) = (row.checked_add_signed(dr), col.checked_add_signed(dc))
                && r < strip.rows as usize
                && c < strip.cols as usize
            {
                let next = r * strip.cols as usize + c;
                if remaining.contains_key(&next) {
                    pending.push(next);
                }
            }
        }
    }
    if weight <= 0.0 {
        return NOT_VISIBLE;
    }
    let row = sum_row / weight;
    let col = sum_col / weight;
    if sample(strip, ring, row, (col - centre) * strip.col_step).is_none() {
        return NOT_VISIBLE;
    }
    let Some(at) = strip_to_film(strip, row, col) else {
        return NOT_VISIBLE;
    };
    PedicleMark::Visible {
        at,
        source: MarkSource::Auto,
        pinned: false,
        inferred: false,
        ring_score: Some(maximum.clamp(0.0, 1.0)),
    }
}
