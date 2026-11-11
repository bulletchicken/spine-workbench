//! Threshold probabilities, retain seeded components, fill holes and trace a
//! subpixel outer contour. The frozen output keeps the largest component;
//! a disconnected result is reported rather than joining separate bones.

use super::{
    geometry::area,
    local_edges::{Crop, Label},
};
use shapes::{
    errors::{EngineError, ErrorCode},
    pixels::Px,
    toolbox::ToolWarning,
};
use std::collections::{BTreeMap, VecDeque};

pub(super) fn contour(
    crop: &Crop,
    values: &[f32],
    warnings: &mut Vec<ToolWarning>,
) -> Result<Vec<Px>, EngineError> {
    let n = values.len();
    let mut seen = vec![false; n];
    let mut largest = Vec::new();
    let mut pieces = 0;
    for start in 0..n {
        if values[start] >= 0.5 && !seen[start] {
            let part = walk(crop.width, crop.height, start, true, &mut seen, |i| {
                values[i] >= 0.5
            });
            if part.iter().any(|&i| crop.labels[i] == Label::Foreground) {
                pieces += 1;
                if part.len() > largest.len() {
                    largest = part;
                }
            }
        }
    }
    if largest.is_empty() {
        return Err(EngineError::new(
            ErrorCode::ToolWouldRemoveVertebra,
            "the region has no seeded component",
        ));
    }
    if pieces > 1 {
        warnings.push(ToolWarning::Disconnected);
    }
    let mut mask = vec![false; n];
    for i in largest {
        mask[i] = true;
    }
    let mut seen = vec![false; n];
    for start in 0..n {
        if !mask[start] && !seen[start] {
            let part = walk(crop.width, crop.height, start, false, &mut seen, |i| {
                !mask[i]
            });
            if !part.iter().any(|&i| {
                i % crop.width == 0
                    || i / crop.width == 0
                    || i % crop.width + 1 == crop.width
                    || i / crop.width + 1 == crop.height
            }) {
                for i in part {
                    mask[i] = true;
                }
            }
        }
    }
    let mut limit = 0;
    let mut reached = 0;
    for y in 1..crop.height - 1 {
        for x in 1..crop.width - 1 {
            let i = y * crop.width + x;
            if crop.labels[i] == Label::Unknown
                && [i - 1, i + 1, i - crop.width, i + crop.width]
                    .iter()
                    .any(|&j| !crop.free[j])
            {
                limit += 1;
                if mask[i] {
                    reached += 1;
                }
            }
        }
    }
    if limit > 0 && reached as f64 / limit as f64 > 0.3 {
        warnings.push(ToolWarning::ReachedLimit);
    }
    // Filled holes carry probability 1; discarded components carry 0.
    let probabilities: Vec<_> = values
        .iter()
        .zip(&mask)
        .map(|(&v, &keep)| {
            if !keep {
                0.0
            } else if v < 0.5 {
                1.0
            } else {
                v
            }
        })
        .collect();
    let (outline, rings) = marching(crop, &probabilities)?;
    if rings > 1 && !warnings.contains(&ToolWarning::Disconnected) {
        warnings.push(ToolWarning::Disconnected);
    }
    Ok(outline)
}
fn walk(
    width: usize,
    height: usize,
    start: usize,
    eight: bool,
    seen: &mut [bool],
    keep: impl Fn(usize) -> bool,
) -> Vec<usize> {
    let mut queue = VecDeque::from([start]);
    seen[start] = true;
    let mut part = Vec::new();
    while let Some(i) = queue.pop_front() {
        part.push(i);
        let x = i % width;
        let y = i / width;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if (dx == 0 && dy == 0) || (!eight && dx != 0 && dy != 0) {
                    continue;
                }
                let nx = x as isize + dx;
                let ny = y as isize + dy;
                if nx < 0 || ny < 0 || nx >= width as isize || ny >= height as isize {
                    continue;
                }
                let j = ny as usize * width + nx as usize;
                if !seen[j] && keep(j) {
                    seen[j] = true;
                    queue.push_back(j);
                }
            }
        }
    }
    part
}
type Edge = (usize, usize);
fn marching(crop: &Crop, p: &[f32]) -> Result<(Vec<Px>, usize), EngineError> {
    let mut nodes: BTreeMap<Edge, (Px, Vec<Edge>)> = BTreeMap::new();
    for y in 0..crop.height - 1 {
        for x in 0..crop.width - 1 {
            let i = y * crop.width + x;
            let corners = [i, i + 1, i + crop.width + 1, i + crop.width];
            let mut crossings = Vec::new();
            for e in 0..4 {
                let (a, b) = (corners[e], corners[(e + 1) % 4]);
                if (p[a] >= 0.5) != (p[b] >= 0.5) {
                    let t = f64::from((0.5 - p[a]) / (p[b] - p[a]));
                    let ax = (a % crop.width) as f64;
                    let ay = (a / crop.width) as f64;
                    let bx = (b % crop.width) as f64;
                    let by = (b / crop.width) as f64;
                    let key = (a.min(b), a.max(b));
                    let point = crop.point(ax + t * (bx - ax), ay + t * (by - ay));
                    nodes.entry(key).or_insert((point, Vec::new()));
                    crossings.push(key);
                }
            }
            let pairs = match crossings.len() {
                2 => vec![(0, 1)],
                4 => {
                    let middle = corners.iter().map(|&c| p[c]).sum::<f32>() * 0.25 >= 0.5;
                    if (p[corners[0]] >= 0.5) == middle {
                        vec![(0, 1), (2, 3)]
                    } else {
                        vec![(0, 3), (1, 2)]
                    }
                }
                _ => Vec::new(),
            };
            for (a, b) in pairs {
                let (a, b) = (crossings[a], crossings[b]);
                nodes.get_mut(&a).expect("crossing").1.push(b);
                nodes.get_mut(&b).expect("crossing").1.push(a);
            }
        }
    }
    let mut largest = Vec::new();
    let mut largest_area = 0.;
    let mut rings = 0;
    while let Some((&start, _)) = nodes.first_key_value() {
        let mut ring = Vec::new();
        let mut at = start;
        let mut previous = None;
        while let Some((point, links)) = nodes.remove(&at) {
            ring.push(point);
            let next = links.into_iter().find(|e| Some(*e) != previous);
            let Some(next) = next else { break };
            previous = Some(at);
            at = next;
            if at == start {
                break;
            }
        }
        let size = area(&ring).abs();
        if size > 0.0 {
            rings += 1;
        }
        if size > largest_area {
            largest_area = size;
            largest = ring;
        }
    }
    if largest.len() < 4 {
        return Err(EngineError::new(
            ErrorCode::ToolRefitFailed,
            "the region has no closed outer contour",
        ));
    }
    largest.push(largest[0]);
    Ok((largest, rings))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discarded_diagonal_lobes_are_reported() {
        let mut labels = vec![Label::Background; 16];
        labels[5] = Label::Foreground;
        labels[10] = Label::Foreground;
        let crop = Crop {
            width: 4,
            height: 4,
            origin: Px::new(0., 0.),
            factor: 1.,
            old: vec![false; 16],
            free: vec![true; 16],
            labels,
            right: vec![0.; 16],
            down: vec![0.; 16],
            warnings: vec![],
        };
        // One 8-connected component, but the cell's mean is only 0.3:
        // marching squares must separate the two diagonal islands.
        let mut values = vec![0.; 16];
        values[5] = 0.6;
        values[10] = 0.6;
        let mut warnings = Vec::new();
        let ring = contour(&crop, &values, &mut warnings).unwrap();
        assert_eq!(ring.len(), 5);
        assert_eq!(warnings, [ToolWarning::Disconnected]);
    }
    #[test]
    fn an_isolated_unit_pixel_has_a_diamond_contour() {
        let crop = Crop {
            width: 3,
            height: 3,
            origin: Px::new(0., 0.),
            factor: 1.,
            old: vec![false; 9],
            free: vec![true; 9],
            labels: vec![Label::Background; 9],
            right: vec![0.; 9],
            down: vec![0.; 9],
            warnings: vec![],
        };
        let mut values = vec![0.; 9];
        values[4] = 1.;
        let (ring, rings) = marching(&crop, &values).unwrap();
        assert_eq!(rings, 1);
        assert_eq!(area(&ring).abs(), 0.5);
        for want in [
            Px::new(1., 0.5),
            Px::new(1.5, 1.),
            Px::new(1., 1.5),
            Px::new(0.5, 1.),
        ] {
            assert!(ring.contains(&want));
        }
    }
}
