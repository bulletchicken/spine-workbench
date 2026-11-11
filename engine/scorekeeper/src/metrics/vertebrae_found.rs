//! Vertebrae found, missed and invented, matched one to one by hull IoU.

use super::{
    Quad, ScoreError,
    quad::{cross, distance},
};

fn area(polygon: &[[f64; 2]]) -> f64 {
    if polygon.len() < 3 {
        return 0.0;
    }
    let origin = polygon[0];
    (1..polygon.len() - 1)
        .map(|i| cross(origin, polygon[i], polygon[i + 1]))
        .sum::<f64>()
        .abs()
        * 0.5
}

/// Intersection over union of the convex hulls, including crossed quads.
/// A zero-area or non-finite quad has IoU zero.
pub fn quad_iou(a: &Quad, b: &Quad) -> f64 {
    let a = a.hull();
    let b = b.hull();
    let (aa, ab) = (area(&a), area(&b));
    if aa <= 0.0 || ab <= 0.0 {
        return 0.0;
    }
    let mut clipped = a;
    for i in 0..b.len() {
        let (start, end) = (b[i], b[(i + 1) % b.len()]);
        let input = std::mem::take(&mut clipped);
        if input.is_empty() {
            return 0.0;
        }
        let mut previous = input[input.len() - 1];
        let mut dp = cross(start, end, previous);
        for current in input {
            let dc = cross(start, end, current);
            if (dc >= 0.0) != (dp >= 0.0) {
                let t = dp / (dp - dc);
                clipped.push([
                    previous[0] + t * (current[0] - previous[0]),
                    previous[1] + t * (current[1] - previous[1]),
                ]);
            }
            if dc >= 0.0 {
                clipped.push(current);
            }
            previous = current;
            dp = dc;
        }
    }
    let intersection = area(&clipped).min(aa.min(ab));
    (intersection / (aa + ab - intersection)).clamp(0.0, 1.0)
}

/// Minimum-cost one-to-one assignment. Rows without a column are `None`.
/// Rectangular matrices are padded with zero-cost dummy rows or columns;
/// non-finite and ragged inputs are refused.
pub fn hungarian(cost: &[Vec<f64>]) -> Result<Vec<Option<usize>>, ScoreError> {
    let rows = cost.len();
    let columns = cost.first().map_or(0, Vec::len);
    if rows > 30
        || columns > 30
        || cost
            .iter()
            .any(|row| row.len() != columns || row.iter().any(|v| !v.is_finite()))
    {
        return Err(ScoreError::InvalidAssignment);
    }
    if rows == 0 || columns == 0 {
        return Ok(vec![None; rows]);
    }
    let n = rows.max(columns);
    let mut u = vec![0.0; n + 1];
    let mut v = vec![0.0; n + 1];
    let mut p = vec![0; n + 1];
    let mut way = vec![0; n + 1];
    for i in 1..=n {
        p[0] = i;
        let mut j0 = 0;
        let mut minimum = vec![f64::INFINITY; n + 1];
        let mut used = vec![false; n + 1];
        loop {
            used[j0] = true;
            let i0 = p[j0];
            let mut delta = f64::INFINITY;
            let mut j1 = 0;
            for j in 1..=n {
                if used[j] {
                    continue;
                }
                let value = if i0 <= rows && j <= columns {
                    cost[i0 - 1][j - 1]
                } else {
                    0.0
                };
                let reduced = value - u[i0] - v[j];
                if reduced < minimum[j] {
                    minimum[j] = reduced;
                    way[j] = j0;
                }
                if minimum[j] < delta {
                    delta = minimum[j];
                    j1 = j;
                }
            }
            for j in 0..=n {
                if used[j] {
                    u[p[j]] += delta;
                    v[j] -= delta;
                } else {
                    minimum[j] -= delta;
                }
            }
            j0 = j1;
            if p[j0] == 0 {
                break;
            }
        }
        loop {
            let j1 = way[j0];
            p[j0] = p[j1];
            j0 = j1;
            if j0 == 0 {
                break;
            }
        }
    }
    let mut result = vec![None; rows];
    for j in 1..=columns {
        if p[j] > 0 && p[j] <= rows {
            result[p[j] - 1] = Some(j - 1);
        }
    }
    Ok(result)
}

/// The assignment maximises total IoU, then keeps pairs with IoU ≥ 0.5.
/// Inputs are at most thirty finite quads each. Invalid inputs match none.
pub fn match_quads(truth: &[Quad], prediction: &[Quad]) -> Vec<(usize, usize)> {
    if truth.iter().chain(prediction).any(|q| !q.is_finite()) {
        return Vec::new();
    }
    let costs: Vec<Vec<f64>> = truth
        .iter()
        .map(|a| prediction.iter().map(|b| 1.0 - quad_iou(a, b)).collect())
        .collect();
    let Ok(assignment) = hungarian(&costs) else {
        return Vec::new();
    };
    assignment
        .into_iter()
        .enumerate()
        .filter_map(|(i, j)| j.filter(|&j| costs[i][j] <= 0.5).map(|j| (i, j)))
        .collect()
}

fn distance_to_segment(point: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let length_squared = d[0] * d[0] + d[1] * d[1];
    if length_squared == 0.0 {
        return distance(point, a);
    }
    let t =
        (((point[0] - a[0]) * d[0] + (point[1] - a[1]) * d[1]) / length_squared).clamp(0.0, 1.0);
    distance(point, [a[0] + t * d[0], a[1] + t * d[1]])
}

/// The spine strip is the union of truth hulls dilated by half their mean
/// height. Only unmatched predicted centres in this strip count as false
/// positives; remote detections remain visible in the unmatched count.
pub fn in_spine_strip(point: [f64; 2], truth: &[Quad]) -> bool {
    if truth.is_empty() {
        return false;
    }
    let radius = truth.iter().map(Quad::height).sum::<f64>() / (2.0 * truth.len() as f64);
    truth.iter().any(|q| {
        let hull = q.hull();
        if hull.is_empty() {
            return false;
        }
        let inside = hull.len() >= 3
            && (0..hull.len()).all(|i| cross(hull[i], hull[(i + 1) % hull.len()], point) >= 0.0);
        inside
            || (0..hull.len())
                .any(|i| distance_to_segment(point, hull[i], hull[(i + 1) % hull.len()]) <= radius)
    })
}
