//! The network's maps read as peaks and outlines.
//!
//! The heat map's logits become scores through the sigmoid, and a cell is
//! a peak when its score equals the largest in the 3x3 square around it
//! (cells off the map do not count). The test is exact equality, so two
//! neighbouring cells with the same score are both peaks. Each peak is read
//! as an outline from the corner and centre maps at its cell, in single
//! precision in the reference's order, and its corners are put in going
//! round order: upper left, upper right, lower right, lower left.
//!
//! Two lists come out (see [`Reading`]): every peak scoring at least 0.05,
//! and the outlines, the best thirty peaks at or above the threshold, each
//! kept only if it overlaps no better outline by half or more.

use crate::layers::activation::sigmoid;
use crate::outline::{Peak, Reading};
use crate::overlap::overlap;
use crate::spine_net::{HeadMaps, MAP_HEIGHT, MAP_WIDTH};
use std::cmp::Ordering;

/// The lowest score a peak needs to be listed.
pub const LOWEST_PEAK_SCORE: f64 = 0.05;

/// The outlines are drawn from at most this many of the best peaks…
const MOST_OUTLINES: usize = 30;

/// …and one is dropped when it overlaps a better one by this much or more.
const MOST_OVERLAP: f64 = 0.5;

/// Reads the three maps of one reading as peaks and outlines; `threshold`
/// is the score an outline needs (the weights' 0.5).
pub fn read(maps: &HeadMaps, threshold: f64) -> Reading {
    let scores: Vec<f32> = (0..MAP_HEIGHT)
        .flat_map(|row| maps.heat.row(0, row))
        .map(|&logit| sigmoid(logit))
        .collect();
    let peak_scores = local_maxima(&scores);

    let mut listed: Vec<usize> = (0..peak_scores.len())
        .filter(|&cell| f64::from(peak_scores[cell]) >= LOWEST_PEAK_SCORE)
        .collect();
    listed.sort_by(|&a, &b| best_first(&peak_scores, a, b));
    let peaks: Vec<Peak> = listed
        .iter()
        .map(|&cell| candidate(maps, cell, peak_scores[cell]))
        .collect();

    // The best thirty cells by peak score, stopping at the first below the
    // threshold, is the same as the best thirty at or above it.
    let mut best: Vec<usize> = (0..peak_scores.len())
        .filter(|&cell| f64::from(peak_scores[cell]) >= threshold)
        .collect();
    best.sort_by(|&a, &b| best_first(&peak_scores, a, b));
    best.truncate(MOST_OUTLINES);
    let mut outlines: Vec<Peak> = Vec::new();
    for cell in best {
        let outline = candidate(maps, cell, peak_scores[cell]);
        if outlines
            .iter()
            .all(|kept| overlap(&outline.corners, &kept.corners) < MOST_OVERLAP)
        {
            outlines.push(outline);
        }
    }
    Reading { peaks, outlines }
}

/// Each cell's score where it is a 3x3 local maximum, and zero elsewhere.
fn local_maxima(scores: &[f32]) -> Vec<f32> {
    let mut peaks = vec![0.0f32; scores.len()];
    for row in 0..MAP_HEIGHT {
        let rows = row.saturating_sub(1)..(row + 2).min(MAP_HEIGHT);
        for column in 0..MAP_WIDTH {
            let columns = column.saturating_sub(1)..(column + 2).min(MAP_WIDTH);
            let mut largest = f32::NEG_INFINITY;
            for r in rows.clone() {
                for &score in &scores[r * MAP_WIDTH + columns.start..r * MAP_WIDTH + columns.end] {
                    if score > largest {
                        largest = score;
                    }
                }
            }
            let score = scores[row * MAP_WIDTH + column];
            if score == largest {
                peaks[row * MAP_WIDTH + column] = score;
            }
        }
    }
    peaks
}

/// Higher score first, ties by cell in row order.
fn best_first(scores: &[f32], a: usize, b: usize) -> Ordering {
    scores[b].total_cmp(&scores[a]).then(a.cmp(&b))
}

/// The outline of the vertebra centred in `cell`, in fractions of the
/// picture: `((corner + cell) + offset) / size` in single precision, in
/// that order, as the reference adds it up.
fn candidate(maps: &HeadMaps, cell: usize, score: f32) -> Peak {
    let (row, column) = (cell / MAP_WIDTH, cell % MAP_WIDTH);
    let cell_place = [column as f32, row as f32];
    let offset = [
        maps.centre_in_cell.at(0, row, column),
        maps.centre_in_cell.at(1, row, column),
    ];
    let size = [MAP_WIDTH as f32, MAP_HEIGHT as f32];
    let centre = [0, 1].map(|axis| f64::from((cell_place[axis] + offset[axis]) / size[axis]));
    // The network's corner order is upper left, upper right, lower left,
    // lower right.
    let taught = [0, 1, 2, 3].map(|corner| {
        [0, 1].map(|axis| {
            let reach = maps.corners.at(2 * corner + axis, row, column);
            f64::from((reach + cell_place[axis] + offset[axis]) / size[axis])
        })
    });
    Peak {
        score: f64::from(score),
        centre,
        corners: [taught[0], taught[1], taught[3], taught[2]],
        cell: [column as u32, row as u32],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::Grid;

    /// Maps whose heat logits are `low` everywhere except where `set` says.
    fn maps_with(low: f32, set: &[(usize, usize, f32)]) -> HeadMaps {
        let mut heat = Grid::zeros(1, MAP_HEIGHT, MAP_WIDTH).expect("a small grid");
        for row in 0..MAP_HEIGHT {
            heat.row_mut(0, row).fill(low);
        }
        for &(row, column, logit) in set {
            heat.row_mut(0, row)[column] = logit;
        }
        let mut corners = Grid::zeros(8, MAP_HEIGHT, MAP_WIDTH).expect("a small grid");
        // Every cell's vertebra reaches 2 cells across and 1 down from its
        // centre, corners in the network's order: upper left, upper right,
        // lower left, lower right.
        for (channel, reach) in [-2.0, -1.0, 2.0, -1.0, -2.0, 1.0, 2.0, 1.0]
            .into_iter()
            .enumerate()
        {
            for row in 0..MAP_HEIGHT {
                corners.row_mut(channel, row).fill(reach);
            }
        }
        let mut centre_in_cell = Grid::zeros(2, MAP_HEIGHT, MAP_WIDTH).expect("a small grid");
        for channel in 0..2 {
            for row in 0..MAP_HEIGHT {
                centre_in_cell.row_mut(channel, row).fill(0.5);
            }
        }
        HeadMaps {
            heat,
            corners,
            centre_in_cell,
        }
    }

    #[test]
    fn a_lone_peak_is_read_with_its_corners_going_round() {
        let reading = read(&maps_with(-9.0, &[(10, 20, 2.0)]), 0.5);
        assert_eq!(reading.peaks.len(), 1);
        let peak = &reading.peaks[0];
        assert_eq!(peak.cell, [20, 10]);
        assert_eq!(peak.score, f64::from(sigmoid(2.0)));
        assert_eq!(
            peak.centre,
            [f64::from(20.5f32 / 96.0), f64::from(10.5f32 / 192.0)]
        );
        let corner = |x: f32, y: f32| {
            [
                f64::from((x + 20.0 + 0.5) / 96.0),
                f64::from((y + 10.0 + 0.5) / 192.0),
            ]
        };
        assert_eq!(
            peak.corners,
            [
                corner(-2.0, -1.0),
                corner(2.0, -1.0),
                corner(2.0, 1.0),
                corner(-2.0, 1.0)
            ]
        );
        assert_eq!(reading.outlines, reading.peaks);
    }

    #[test]
    fn peaks_are_listed_best_first_and_weak_ones_left_out() {
        let set = [(5, 5, 0.0), (50, 50, 1.0), (100, 10, -2.0), (150, 80, -4.0)];
        let reading = read(&maps_with(-9.0, &set), 0.5);
        let cells: Vec<[u32; 2]> = reading.peaks.iter().map(|peak| peak.cell).collect();
        // sigmoid(-4) is under 0.05, so it is not listed.
        assert_eq!(cells, vec![[50, 50], [5, 5], [10, 100]]);
        // Only scores at or above the threshold become outlines; sigmoid(0)
        // is exactly a half.
        let cells: Vec<[u32; 2]> = reading.outlines.iter().map(|peak| peak.cell).collect();
        assert_eq!(cells, vec![[50, 50], [5, 5]]);
    }

    #[test]
    fn equal_neighbours_are_both_peaks_and_ties_go_in_row_order() {
        let reading = read(&maps_with(-9.0, &[(40, 40, 1.0), (40, 41, 1.0)]), 0.5);
        let cells: Vec<[u32; 2]> = reading.peaks.iter().map(|peak| peak.cell).collect();
        assert_eq!(cells, vec![[40, 40], [41, 40]]);
        // The two outlines overlap almost wholly, so only the first is kept.
        assert_eq!(reading.outlines.len(), 1);
        assert_eq!(reading.outlines[0].cell, [40, 40]);
    }

    #[test]
    fn a_cell_beside_a_higher_one_is_not_a_peak() {
        let reading = read(&maps_with(-9.0, &[(0, 0, 1.0), (1, 1, 1.5)]), 0.5);
        let cells: Vec<[u32; 2]> = reading.peaks.iter().map(|peak| peak.cell).collect();
        assert_eq!(cells, vec![[1, 1]]);
    }

    #[test]
    fn a_flat_map_is_all_peaks_but_none_listed_when_too_weak() {
        let reading = read(&maps_with(-9.0, &[]), 0.5);
        assert!(reading.peaks.is_empty());
        assert!(reading.outlines.is_empty());
    }

    #[test]
    fn at_most_thirty_outlines_are_considered() {
        // Forty strong peaks far enough apart not to overlap.
        let set: Vec<(usize, usize, f32)> = (0..40)
            .map(|i| (4 * (i / 8) + 2, 12 * (i % 8) + 2, 3.0 - i as f32 * 0.01))
            .collect();
        let reading = read(&maps_with(-9.0, &set), 0.5);
        assert_eq!(reading.peaks.len(), 40);
        assert_eq!(reading.outlines.len(), 30);
        assert_eq!(reading.outlines[..], reading.peaks[..30]);
    }
}
