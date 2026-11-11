//! Joint selection of an inner body-wall pair, within a height/width envelope.

use super::endplates::peak_offset;
use super::walls::{Wall, wall_score};
use crate::handoffs::{EndplateOnStrip, Polarity, Strip};

#[derive(Clone, Copy)]
struct Candidate {
    wall: Wall,
    strength: f64,
}

pub(super) fn choose(
    strip: &Strip,
    plates: [EndplateOnStrip; 2],
    half: f64,
    polarity: Polarity,
    observed: bool,
) -> [Wall; 2] {
    let height = (plates[1].row - plates[0].row) * strip.row_step;
    let middle = (plates[0].row + plates[1].row) * strip.row_step * 0.5;
    let maximum = super::widths::maximum_ratio(strip.film) * height;
    let minimum_cosine = libm::cos(plates[0].tilt).min(libm::cos(plates[1].tilt));
    let fallback_half = half.min(0.5 * maximum * minimum_cosine);
    let fallback = [-fallback_half, fallback_half].map(|offset| Wall {
        offset,
        slope: 0.0,
        middle,
        observed: false,
    });
    if !observed {
        return fallback;
    }
    let [negative, positive] =
        [-1.0, 1.0].map(|side| candidates(strip, plates, half, maximum, polarity, side));
    if negative.is_empty() && positive.is_empty() {
        return fallback;
    }
    let strongest = |values: &[Candidate]| values.iter().map(|c| c.strength).fold(0.005, f64::max);
    let scale = [strongest(&negative), strongest(&positive)];
    let left = if negative.is_empty() {
        vec![Candidate {
            wall: fallback[0],
            strength: 0.0,
        }]
    } else {
        negative
    };
    let right = if positive.is_empty() {
        vec![Candidate {
            wall: fallback[1],
            strength: 0.0,
        }]
    } else {
        positive
    };
    let mut best = fallback;
    let mut cost = f64::INFINITY;
    for a in left {
        for b in &right {
            let width = b.wall.offset - a.wall.offset;
            // One wall cannot wander away while the other stays on the body.
            let taper = (b.wall.slope - a.wall.slope).abs() * height * 0.5;
            if width - taper < 0.65 * 2.0 * half
                || !within_envelope(strip, plates, [a.wall, b.wall], maximum)
            {
                continue;
            }
            let mismatch = (width / (2.0 * half) - 1.0).abs();
            let off_centre = (b.wall.offset + a.wall.offset) / width;
            // Once strong enough, preference goes to a compact, centred pair
            // near the column/neighbour estimate, not the strongest trunk edge.
            let strength = a.strength / scale[0] + b.strength / scale[1];
            let value = 3.0 * mismatch * mismatch
                + 0.7 * off_centre * off_centre
                + 0.5 * width / (2.0 * half)
                - 0.5 * strength;
            if value < cost {
                cost = value;
                best = [a.wall, b.wall];
            }
        }
    }
    best
}

/// Check the intersections themselves. A wall slope and endplate tilt
/// together can amplify the span even when the mid-body offsets fit.
fn within_envelope(
    strip: &Strip,
    plates: [EndplateOnStrip; 2],
    walls: [Wall; 2],
    maximum: f64,
) -> bool {
    plates.into_iter().all(|plate| {
        let (Some((ar, ax)), Some((br, bx))) = (
            walls[0].intersection(strip, plate),
            walls[1].intersection(strip, plate),
        ) else {
            return false;
        };
        let span = libm::hypot(bx - ax, (br - ar) * strip.row_step);
        bx > ax && span.is_finite() && span <= maximum
    })
}

fn candidates(
    strip: &Strip,
    plates: [EndplateOnStrip; 2],
    half: f64,
    maximum: f64,
    polarity: Polarity,
    side: f64,
) -> Vec<Candidate> {
    let height = (plates[1].row - plates[0].row) * strip.row_step;
    let middle = (plates[0].row + plates[1].row) * strip.row_step * 0.5;
    let low = 0.4 * half;
    let high = (1.6 * half)
        .min(0.8 * maximum)
        .min(f64::from(strip.cols - 1) * strip.col_step * 0.5);
    if high <= low {
        return vec![];
    }
    let steps = ((high - low) / 0.5)
        .ceil()
        .clamp(1.0, f64::from(strip.cols)) as usize;
    let spacing = (high - low) / steps as f64;
    let slope_step = (3.0 / height).min(libm::tan(10.0_f64.to_radians())) / 6.0;
    let mut profile = Vec::new();
    for i in 0..=steps {
        let mut candidate = Candidate {
            wall: Wall {
                offset: side * (low + i as f64 * spacing),
                slope: 0.0,
                middle,
                observed: true,
            },
            strength: 0.0,
        };
        for j in -6..=6 {
            let wall = Wall {
                slope: f64::from(j) * slope_step,
                ..candidate.wall
            };
            let strength = wall_score(strip, plates, wall, side, polarity);
            if strength > candidate.strength + 1e-10
                || ((strength - candidate.strength).abs() <= 1e-10
                    && wall.slope.abs() < candidate.wall.slope.abs())
            {
                candidate = Candidate { wall, strength };
            }
        }
        profile.push(candidate);
    }
    let maximum_strength = profile.iter().map(|c| c.strength).fold(0.0, f64::max);
    let threshold = (0.2 * maximum_strength).max(0.005);
    let mut peaks = Vec::new();
    for i in 0..profile.len() {
        let mut candidate = profile[i];
        if candidate.strength < threshold {
            continue;
        }
        let before = profile[i.saturating_sub(1)].strength;
        let after = profile[(i + 1).min(profile.len() - 1)].strength;
        if candidate.strength < before || candidate.strength < after {
            continue;
        }
        let shift = spacing * peak_offset(before, candidate.strength, after);
        candidate.wall.offset += side * shift;
        peaks.push(candidate);
        // Innermost strong candidates are considered first, bounding pair work.
        if peaks.len() == 12 {
            break;
        }
    }
    peaks
}
