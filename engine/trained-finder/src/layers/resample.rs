//! Maps made twice as big, or half as big.

use crate::grid::Grid;
use std::ops::Range;

/// Where one output row or column reads from when a map is made twice as
/// big: the two source indices and their weights.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Source {
    first: usize,
    second: usize,
    first_weight: f32,
    second_weight: f32,
}

/// PyTorch's rule for bilinear resizing with `align_corners=False`, for an
/// output exactly twice `len`: the output index's centre mapped back onto
/// the input, clamped at the first index, the second index clamped at the
/// last. The weights come out as exactly 1, 0.75 or 0.25.
fn source(index: usize, len: usize) -> Source {
    let place = ((index as f32 + 0.5) * 0.5 - 0.5).max(0.0);
    let first = place as usize;
    let second = if first + 1 < len { first + 1 } else { first };
    let second_weight = place - first as f32;
    Source {
        first,
        second,
        first_weight: 1.0 - second_weight,
        second_weight,
    }
}

/// Writes channels `channels` of `source` made twice as tall and twice as
/// wide into the same channels of `out` (border or none), bilinearly, as
/// PyTorch's `interpolate(mode="bilinear", align_corners=False)` does:
/// each output is `wy0 * (wx0 * a + wx1 * b) + wy1 * (wx0 * c + wx1 * d)`.
pub fn upsample_twice(source_maps: &Grid, channels: Range<usize>, out: &mut Grid) {
    let (rows, columns) = (source_maps.rows(), source_maps.columns());
    debug_assert_eq!((out.rows(), out.columns()), (2 * rows, 2 * columns));
    let across: Vec<Source> = (0..2 * columns).map(|x| source(x, columns)).collect();
    for channel in channels {
        for y in 0..2 * rows {
            let down = source(y, rows);
            let upper = source_maps.row(channel, down.first);
            let lower = source_maps.row(channel, down.second);
            let out_row = out.row_mut(channel, y);
            for (dest, along) in out_row.iter_mut().zip(&across) {
                let top = along.first_weight * upper[along.first]
                    + along.second_weight * upper[along.second];
                let bottom = along.first_weight * lower[along.first]
                    + along.second_weight * lower[along.second];
                *dest = down.first_weight * top + down.second_weight * bottom;
            }
        }
    }
}

/// Writes every channel of `source` averaged over 2x2 squares into `out`
/// (border or none): the four values added row by row, then divided by
/// four, as PyTorch's `avg_pool2d(2)` does.
pub fn average_pool_2x2(source_maps: &Grid, out: &mut Grid) {
    debug_assert_eq!(
        (out.channels(), 2 * out.rows(), 2 * out.columns()),
        (
            source_maps.channels(),
            source_maps.rows(),
            source_maps.columns()
        )
    );
    for channel in 0..out.channels() {
        for y in 0..out.rows() {
            let upper = source_maps.row(channel, 2 * y);
            let lower = source_maps.row(channel, 2 * y + 1);
            for (x, dest) in out.row_mut(channel, y).iter_mut().enumerate() {
                *dest = (upper[2 * x] + upper[2 * x + 1] + lower[2 * x] + lower[2 * x + 1]) / 4.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_weights_are_a_quarter_and_three_quarters_with_the_edges_copied() {
        let at = |index| source(index, 3);
        assert_eq!(
            at(0),
            Source {
                first: 0,
                second: 1,
                first_weight: 1.0,
                second_weight: 0.0
            }
        );
        assert_eq!(
            at(1),
            Source {
                first: 0,
                second: 1,
                first_weight: 0.75,
                second_weight: 0.25
            }
        );
        assert_eq!(
            at(2),
            Source {
                first: 0,
                second: 1,
                first_weight: 0.25,
                second_weight: 0.75
            }
        );
        assert_eq!(
            at(5),
            Source {
                first: 2,
                second: 2,
                first_weight: 0.75,
                second_weight: 0.25
            }
        );
    }

    #[test]
    fn a_two_by_two_map_upsamples_as_pytorch_does() {
        // torch.nn.functional.interpolate(torch.tensor([[[[1., 2.], [3., 4.]]]]),
        // size=(4, 4), mode="bilinear", align_corners=False)
        let source_maps =
            Grid::from_values(1, 2, 2, vec![1.0, 2.0, 3.0, 4.0]).expect("four values");
        let mut out = Grid::bordered(1, 4, 4).expect("a small grid");
        upsample_twice(&source_maps, 0..1, &mut out);
        assert_eq!(out.row(0, 0), &[1.0, 1.25, 1.75, 2.0]);
        assert_eq!(out.row(0, 1), &[1.5, 1.75, 2.25, 2.5]);
        assert_eq!(out.row(0, 2), &[2.5, 2.75, 3.25, 3.5]);
        assert_eq!(out.row(0, 3), &[3.0, 3.25, 3.75, 4.0]);
        assert_eq!(out.padded_row(0, 0), &[0.0; 6], "the border stays zero");
    }

    #[test]
    fn only_the_channels_asked_for_are_written() {
        let source_maps = Grid::from_values(2, 1, 1, vec![5.0, 7.0]).expect("two values");
        let mut out = Grid::zeros(2, 2, 2).expect("a small grid");
        upsample_twice(&source_maps, 1..2, &mut out);
        assert_eq!(out.plane(0), &[0.0; 4]);
        assert_eq!(out.plane(1), &[7.0; 4]);
    }

    #[test]
    fn pooling_averages_each_square() {
        let source_maps = Grid::from_values(1, 2, 4, vec![1.0, 2.0, 5.0, 5.0, 3.0, 4.0, 5.0, 9.0])
            .expect("eight values");
        let mut out = Grid::bordered(1, 1, 2).expect("a small grid");
        average_pool_2x2(&source_maps, &mut out);
        assert_eq!(out.row(0, 0), &[2.5, 6.0]);
    }
}
