//! First-rib, sacral, and pedicle evidence on a fixed lattice.

use super::native_edges::line_evidence;
use super::{DEG, Statistics};
use crate::handoffs::Strip;
use shapes::film::FilmKind;
use shapes::film_geometry::ImageRight;

pub(super) fn rib_onset(
    strip: &Strip,
    usable: &[bool],
    observed: &[bool],
    step: f64,
    padding: usize,
    half_width: &[f64],
) -> Vec<f64> {
    let mut onset = vec![0.0; observed.len()];
    if strip.film != FilmKind::Front {
        return onset;
    }
    let (Some(ridge), Some(angles)) = (&strip.evidence.ridge, &strip.evidence.ridge_angle) else {
        return onset;
    };
    let cols = strip.cols as usize;
    let mut response = vec![0.0; observed.len()];
    for i in 0..observed.len() {
        if !observed[i] {
            continue;
        }
        let r = (((i as f64 - padding as f64) * step / strip.row_step).round() as usize)
            .min(strip.rows as usize - 1);
        let mut sum = 0.0;
        let mut count = 0;
        for c in 0..cols {
            let offset = ((c as f64 - (cols / 2) as f64) * strip.col_step).abs();
            let index = r * cols + c;
            if offset < half_width[i] || offset > 2.5 * half_width[i] || !usable[index] {
                continue;
            }
            let angle = f64::from(angles[index]).abs();
            if angle.is_finite()
                && (20.0 * DEG..=70.0 * DEG).contains(&angle)
                && ridge[index].is_finite()
            {
                sum += f64::from(ridge[index]);
            }
            count += 1;
        }
        if count > 0 {
            response[i] = sum / count as f64;
        }
    }
    let before = (15.0 / step).round().max(1.0) as usize;
    let after = (10.0 / step).round().max(1.0) as usize;
    for i in before..observed.len().saturating_sub(after) {
        if !observed[i] {
            continue;
        }
        let upper: f64 = response[i - before..i].iter().sum();
        let lower: f64 = response[i..i + after].iter().sum();
        onset[i] = lower / after as f64 - upper / before as f64;
    }
    let stats = Statistics::new(
        onset
            .iter()
            .zip(observed)
            .filter_map(|(&v, &ok)| ok.then_some(v)),
    );
    for (value, &ok) in onset.iter_mut().zip(observed) {
        *value = if ok {
            stats.z(*value).clamp(-4.0, 6.0)
        } else {
            0.0
        };
    }
    onset
}

pub(super) struct SacralEvidence {
    /// Best positive strength and its tilt; unchanged by retaining options.
    pub best: Vec<(f64, f64)>,
    /// Costs and tilts for the final tilt chain, best first.
    pub options: Vec<Vec<(f64, f64)>>,
}

pub(super) fn sacral_table(
    strip: &Strip,
    usable: &[bool],
    edges: &[f64],
    observed: &[bool],
    step: f64,
    padding: usize,
    half_width: &[f64],
) -> SacralEvidence {
    let rows = observed.len();
    let mut result = vec![(0.0, 0.0); rows];
    let mut options = vec![Vec::new(); rows];
    let (right_x, right_y) = upright_right(strip);
    let first =
        padding + (((f64::from(strip.rows) * strip.row_step / step) * 0.6).floor() as usize);
    for i in 0..rows {
        let native = (i as f64 - padding as f64) * step / strip.row_step;
        let r = native.round().clamp(0.0, f64::from(strip.rows - 1)) as usize;
        let axis = strip.layout[r].across;
        // The layout is in original film pixels, which may have been
        // stored with a quarter turn. Measure the local normal relative to
        // upright image right, not the original pixel +x direction.
        let normal_angle = libm::atan2(
            right_x * axis.y() - right_y * axis.x(),
            right_x * axis.x() + right_y * axis.y(),
        );
        // Mac-Thiong et al., Spine 2004, PMID 15284510: mean sacral slope
        // 41.2° in 180 subjects aged 4–18. The ±25° search is experimental.
        let centres: &[f64] = match (strip.film, strip.image_right) {
            (FilmKind::Side, Some(ImageRight::Anterior)) => &[41.2],
            (FilmKind::Side, Some(ImageRight::Posterior)) => &[-41.2],
            (FilmKind::Side, _) => &[-41.2, 41.2],
            _ => &[0.0],
        };
        for &centre in centres {
            for j in 0..=20 {
                let tilt = (centre - 25.0 + j as f64 * 2.5) * DEG
                    - if strip.film == FilmKind::Side {
                        normal_angle
                    } else {
                        0.0
                    };
                let score = if i >= first && observed[i] {
                    line_evidence(strip, usable, edges, native, tilt, 0.0, 0.6 * half_width[i]).0
                } else {
                    0.0
                };
                options[i].push((score, tilt));
                if score > result[i].0 {
                    result[i] = (score, tilt);
                }
            }
        }
    }
    let stats = Statistics::new(
        result
            .iter()
            .enumerate()
            .filter_map(|(i, v)| (i >= first && observed[i]).then_some(v.0)),
    );
    for i in 0..rows {
        if i >= first && observed[i] {
            result[i].0 = stats.z(result[i].0).clamp(-4.0, 6.0);
            for option in &mut options[i] {
                option.0 = -stats.z(option.0).clamp(-4.0, 6.0);
            }
        } else {
            for option in &mut options[i] {
                option.0 = 0.0;
            }
        }
        // Keep the previous best first for deterministic ties. A row with
        // no positive evidence also retains its unconstrained zero tilt.
        let best = (-result[i].0, result[i].1);
        options[i].retain(|option| option.1 != best.1);
        options[i].insert(0, best);
    }
    SacralEvidence {
        best: result,
        options,
    }
}

/// The strip has no turn field. Its mean across vector points toward
/// upright image right; the nearest original-film cardinal direction
/// removes the stored quarter turn while preserving local curvature.
fn upright_right(strip: &Strip) -> (f64, f64) {
    let mut x = 0.0;
    let mut y = 0.0;
    for row in &strip.layout {
        let length = libm::hypot(row.across.x(), row.across.y());
        if length > 0.0 {
            x += row.across.x() / length;
            y += row.across.y() / length;
        }
    }
    // On the exact diagonal, choose the next counterclockwise cardinal.
    // This tie rule, unlike always choosing x, rotates with the film.
    let choose_x =
        x.abs() > y.abs() || (x.abs() == y.abs() && x.is_sign_negative() != y.is_sign_negative());
    if choose_x || (x == 0.0 && y == 0.0) {
        (if x < 0.0 { -1.0 } else { 1.0 }, 0.0)
    } else {
        (0.0, if y < 0.0 { -1.0 } else { 1.0 })
    }
}

pub(super) fn pedicle_tables(
    strip: &Strip,
    usable: &[bool],
    observed: &[bool],
    step: f64,
    padding: usize,
    half_width: &[f64],
) -> [RangeMaximum; 2] {
    let mut sides = [vec![0.0; observed.len()], vec![0.0; observed.len()]];
    if strip.film != FilmKind::Side
        && let Some(ring) = &strip.evidence.ring
    {
        let stats = Statistics::new(
            ring.iter()
                .zip(usable)
                .filter_map(|(&v, &ok)| (ok && v.is_finite()).then_some(f64::from(v))),
        );
        let cols = strip.cols as usize;
        for i in 0..observed.len() {
            if !observed[i] {
                continue;
            }
            let r = (((i as f64 - padding as f64) * step / strip.row_step).round() as usize)
                .min(strip.rows as usize - 1);
            for c in 0..cols {
                let offset = (c as f64 - (cols / 2) as f64) * strip.col_step;
                if offset.abs() < 0.25 * half_width[i] || offset.abs() > 0.85 * half_width[i] {
                    continue;
                }
                let index = r * cols + c;
                if usable[index] && ring[index].is_finite() {
                    let side = usize::from(offset > 0.0);
                    sides[side][i] = f64::max(sides[side][i], stats.z(f64::from(ring[index])));
                }
            }
        }
    }
    sides.map(RangeMaximum::new)
}

#[derive(Clone, Debug)]
pub(super) struct RangeMaximum {
    layers: Vec<Vec<f64>>,
}
impl RangeMaximum {
    pub(super) fn new(values: Vec<f64>) -> Self {
        let mut layers = vec![values];
        let mut span = 1;
        while span * 2 <= layers[0].len() {
            let previous = &layers[layers.len() - 1];
            let next = (0..layers[0].len() + 1 - span * 2)
                .map(|i| previous[i].max(previous[i + span]))
                .collect();
            layers.push(next);
            span *= 2;
        }
        Self { layers }
    }
    pub(super) fn maximum(&self, first: usize, end: usize) -> f64 {
        let layer = (end - first).ilog2() as usize;
        let span = 1usize << layer;
        self.layers[layer][first].max(self.layers[layer][end - span])
    }
}
