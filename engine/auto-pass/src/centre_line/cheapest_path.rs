//! Deterministic multi-source Dijkstra on a sixteen-neighbour physical grid.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use super::cost_map::CostMap;
use super::smooth::Point;

const MOVES: [(i32, i32); 16] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
    (-2, -1),
    (-2, 1),
    (2, -1),
    (2, 1),
    (-1, -2),
    (1, -2),
    (-1, 2),
    (1, 2),
];

/// The coarse path and up to sixteen real search-front snapshots.
pub(crate) struct Path {
    pub points: Vec<Point>,
    pub wavefronts: Vec<Vec<f32>>,
}

/// A knight step includes the two cells it passes between, so it cannot
/// jump across a one-cell barrier without paying its cost.
fn between(x: i32, y: i32, dx: i32, dy: i32) -> [(i32, i32); 2] {
    if dx.abs() == 2 {
        [(x + dx / 2, y), (x + dx / 2, y + dy)]
    } else {
        [(x, y + dy / 2), (x + dx, y + dy / 2)]
    }
}

pub(crate) fn search(map: &CostMap, direction_penalty: bool, checkpoints: bool) -> Option<Path> {
    let (w, h) = (map.image.width() as usize, map.image.height() as usize);
    let n = w * h;
    if map.top.is_empty() || map.bottom.is_empty() {
        return None;
    }
    let mut distance = vec![f32::INFINITY; n];
    let mut parent = vec![u8::MAX; n];
    let mut is_end = vec![false; n];
    for &i in &map.bottom {
        is_end[i] = true;
    }
    let mut heap = BinaryHeap::new();
    for &i in &map.top {
        distance[i] = 0.0;
        heap.push(Reverse((0u32, i)));
    }
    let mut wavefronts = Vec::new();
    let mut settled = 0usize;
    let interval = n.div_ceil(16);
    let mut end = None;
    while let Some(Reverse((bits, p))) = heap.pop() {
        let current = f32::from_bits(bits);
        if current != distance[p] {
            continue;
        }
        settled += 1;
        if checkpoints && settled.is_multiple_of(interval) && wavefronts.len() < 16 {
            let max = distance
                .iter()
                .copied()
                .filter(|d| d.is_finite())
                .fold(0.0f32, f32::max)
                .max(1e-6);
            wavefronts.push(
                distance
                    .iter()
                    .map(|&v| {
                        if v.is_finite() {
                            (v / max).min(254.0 / 255.0)
                        } else {
                            1.0
                        }
                    })
                    .collect(),
            );
        }
        if is_end[p] {
            end = Some(p);
            break;
        }
        let (x, y) = ((p % w) as i32, (p / w) as i32);
        for (direction, &(dx, dy)) in MOVES.iter().enumerate() {
            let (qx, qy) = (x + dx, y + dy);
            if qx < 0 || qy < 0 || qx >= w as i32 || qy >= h as i32 {
                continue;
            }
            let q = qy as usize * w + qx as usize;
            let length = libm::hypot(f64::from(dx), f64::from(dy));
            let mut average = 0.5 * (map.cost[p] + map.cost[q]);
            if dx.abs() == 2 || dy.abs() == 2 {
                let mid = between(x, y, dx, dy);
                average = 0.25
                    * (map.cost[p]
                        + map.cost[q]
                        + map.cost[mid[0].1 as usize * w + mid[0].0 as usize]
                        + map.cost[mid[1].1 as usize * w + mid[1].0 as usize]);
            }
            let middle = ((y + dy / 2) as usize) * w + (x + dx / 2) as usize;
            let along = map.along[middle];
            let projection = (f64::from(dx) * along.x + f64::from(dy) * along.y).abs() / length;
            let anisotropy = 1.0 + f64::from(map.ridge[middle]) * (1.0 - projection);
            let theta = libm::atan2(f64::from(dx.abs()), f64::from(dy.abs())).to_degrees();
            let direction_cost = if direction_penalty {
                1.0 + 2.0 * (theta - 60.0).max(0.0) / 30.0
            } else {
                1.0
            };
            let candidate = current
                + (length * map.spacing * f64::from(average) * anisotropy * direction_cost) as f32;
            if candidate < distance[q] {
                distance[q] = candidate;
                parent[q] = direction as u8;
                heap.push(Reverse((candidate.to_bits(), q)));
            }
        }
    }
    let mut p = end?;
    let mut points = Vec::new();
    loop {
        let (x, y) = (p % w, p / w);
        points.push(Point {
            x: x as f64 * map.spacing,
            y: y as f64 * map.spacing,
        });
        if parent[p] == u8::MAX {
            break;
        }
        let (dx, dy) = MOVES[parent[p] as usize];
        p = (y as i32 - dy) as usize * w + (x as i32 - dx) as usize;
    }
    points.reverse();
    (points.len() >= 2).then_some(Path { points, wavefronts })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knight_steps_pay_the_cells_between_them() {
        assert_eq!(between(4, 4, 2, 1), [(5, 4), (5, 5)]);
        assert_eq!(between(4, 4, -2, -1), [(3, 4), (3, 3)]);
        assert_eq!(between(4, 4, 1, -2), [(4, 3), (5, 3)]);
    }
}
