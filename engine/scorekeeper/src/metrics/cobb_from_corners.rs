//! The dataset's own corner-based Cobb angle, reproduced faithfully, for
//! scoring only. Raw dataset left/right order must be preserved here.

use super::{Prediction, Quad};
use serde::{Deserialize, Serialize};
use shapes::levels::Level;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CobbBranch {
    NotS,
    UpperS,
    LowerS,
}

fn argmax(values: &[f64]) -> usize {
    let mut best = 0;
    for i in 1..values.len() {
        if values[i] > values[best] {
            best = i;
        }
    }
    best
}

/// Faithful AASCE three-angle calculation. Inputs are finite original
/// pixels, TL/TR/BL/BR per level, with positive image height. Ties choose
/// the first index. Zero-length mid-lines contribute a zero angle row.
pub fn aasce_cobb(points: &[[f64; 2]; 68], image_height: f64) -> ([f64; 3], CobbBranch) {
    let midpoints: [[f64; 2]; 34] = std::array::from_fn(|i| {
        [
            (points[2 * i][0] + points[2 * i + 1][0]) * 0.5,
            (points[2 * i][1] + points[2 * i + 1][1]) * 0.5,
        ]
    });
    let vectors: [[f64; 2]; 17] = std::array::from_fn(|i| {
        [
            (points[4 * i + 1][0] + points[4 * i + 3][0] - points[4 * i][0] - points[4 * i + 2][0])
                * 0.5,
            (points[4 * i + 1][1] + points[4 * i + 3][1] - points[4 * i][1] - points[4 * i + 2][1])
                * 0.5,
        ]
    });
    let lengths: [f64; 17] = vectors.map(|v| libm::hypot(v[0], v[1]));
    let mut angles = [[0.0; 17]; 17];
    for j in 0..17 {
        for k in j..17 {
            let denominator = lengths[j] * lengths[k];
            let angle = if denominator == 0.0 {
                0.0
            } else {
                libm::acos(
                    ((vectors[j][0] * vectors[k][0] + vectors[j][1] * vectors[k][1]) / denominator)
                        .clamp(-1.0, 1.0),
                )
            };
            angles[j][k] = angle;
            angles[k][j] = angle;
        }
    }
    let positions: [usize; 17] = std::array::from_fn(|j| argmax(&angles[j]));
    let maxima: [f64; 17] = std::array::from_fn(|j| angles[j][positions[j]]);
    let pos2 = argmax(&maxima);
    let pos1 = positions[pos2];
    let mut sum = 0.0;
    let mut sum_abs = 0.0;
    for m in &midpoints[..32] {
        let ll = (m[1] - midpoints[33][1]) / (midpoints[0][1] - midpoints[33][1])
            - (m[0] - midpoints[33][0]) / (midpoints[0][0] - midpoints[33][0]);
        sum += ll;
        sum_abs += ll.abs();
    }
    // NaN must take an S branch; changing this to a >= comparison changes
    // straight and symmetric inputs where the x denominator is zero.
    let not_s = (sum * sum - sum_abs * sum_abs).abs() < 1e-4;
    let (c2, c3, branch) = if not_s {
        (angles[0][pos2], angles[16][pos1], CobbBranch::NotS)
    } else if midpoints[2 * pos2 + 1][1] + midpoints[2 * pos1 + 1][1] < image_height {
        (
            angles[pos2][argmax(&angles[pos2][..=pos2])],
            angles[pos1][pos1 + argmax(&angles[pos1][pos1..])],
            CobbBranch::UpperS,
        )
    } else {
        let p11 = argmax(&angles[pos2][..=pos2]);
        (
            angles[pos2][p11],
            angles[p11][argmax(&angles[p11][..=p11])],
            CobbBranch::LowerS,
        )
    };
    (
        [maxima[pos2], c2, c3].map(|r| r * 180.0 / std::f64::consts::PI),
        branch,
    )
}

/// Per-film symmetric absolute percentage error (0..100). The denominator
/// is the sum across the three angles; a zero/zero film contributes zero.
pub fn smape_term(prediction: [f64; 3], truth: [f64; 3]) -> f64 {
    let denominator: f64 = prediction.iter().chain(&truth).sum();
    if denominator == 0.0 {
        0.0
    } else {
        100.0
            * prediction
                .into_iter()
                .zip(truth)
                .map(|(a, b)| (a - b).abs())
                .sum::<f64>()
            / denominator
    }
}

pub fn smape(prediction: &[[f64; 3]], truth: &[[f64; 3]]) -> Option<f64> {
    if prediction.len() != truth.len() || prediction.is_empty() {
        return None;
    }
    Some(
        prediction
            .iter()
            .zip(truth)
            .map(|(&p, &t)| smape_term(p, t))
            .sum::<f64>()
            / truth.len() as f64,
    )
}

/// Fill the usual seventeen levels for Cobb only. Interior coordinates
/// interpolate linearly; at either end the nearest quad is translated by
/// the centre step of the nearest two present levels. Corner metrics never
/// use these inferred points.
pub fn fill_missing(prediction: &Prediction) -> Option<([Quad; 17], bool)> {
    let present: Vec<(usize, Quad)> = Level::MEASURED
        .iter()
        .enumerate()
        .filter_map(|(i, level)| {
            prediction
                .vertebrae
                .iter()
                .find(|v| v.level == *level)
                .map(|v| (i, v.quad))
        })
        .collect();
    if present.len() < 2 {
        return None;
    }
    let quads = std::array::from_fn(|i| {
        if let Some((_, q)) = present.iter().find(|(j, _)| *j == i) {
            return *q;
        }
        let after = present.partition_point(|(j, _)| *j < i);
        let (a, b) = if after == 0 {
            (present[0], present[1])
        } else if after == present.len() {
            (present[present.len() - 2], present[present.len() - 1])
        } else {
            (present[after - 1], present[after])
        };
        let t = (i as f64 - a.0 as f64) / (b.0 - a.0) as f64;
        let points = if after == 0 || after == present.len() {
            let nearest = if after == 0 { a } else { b };
            let ac = a.1.centre();
            let bc = b.1.centre();
            let steps = (i as f64 - nearest.0 as f64) / (b.0 - a.0) as f64;
            nearest.1.points().map(|p| {
                [
                    p[0] + steps * (bc[0] - ac[0]),
                    p[1] + steps * (bc[1] - ac[1]),
                ]
            })
        } else {
            let ap = a.1.points();
            let bp = b.1.points();
            std::array::from_fn(|j| {
                [
                    ap[j][0] + t * (bp[j][0] - ap[j][0]),
                    ap[j][1] + t * (bp[j][1] - ap[j][1]),
                ]
            })
        };
        Quad {
            tl: points[0],
            tr: points[1],
            bl: points[2],
            br: points[3],
        }
    });
    Some((quads, present.len() != 17))
}

/// Endplate Cobb is informational and uses canonical endplate directions.
pub fn endplate_cobb(chain: &[Quad]) -> f64 {
    let chain: Vec<_> = chain.iter().map(Quad::x_canonical).collect();
    let mut result: f64 = 0.0;
    for (i, top) in chain.iter().enumerate() {
        let a = libm::atan2(top.tr[1] - top.tl[1], top.tr[0] - top.tl[0]);
        for bottom in &chain[i..] {
            let b = libm::atan2(bottom.br[1] - bottom.bl[1], bottom.br[0] - bottom.bl[0]);
            result = result.max((a - b).abs());
        }
    }
    result * 180.0 / std::f64::consts::PI
}
