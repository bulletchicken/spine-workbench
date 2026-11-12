//! Group norm: the channels split into groups, each group brought to mean
//! zero and variance one over all its values, then each channel scaled and
//! shifted by its own trained pair.

use crate::grid::Grid;

/// The small number added to the variance before its square root.
pub const EPSILON: f64 = 1e-5;

/// One group norm's trained scale and shift per channel.
#[derive(Clone, Debug)]
pub struct GroupNorm {
    groups: usize,
    scale: Vec<f32>,
    shift: Vec<f32>,
}

impl GroupNorm {
    /// `None` unless the scale and shift are one per channel and the
    /// channels split evenly into `groups`.
    pub fn new(groups: usize, scale: Vec<f32>, shift: Vec<f32>) -> Option<GroupNorm> {
        (groups > 0 && scale.len() == shift.len() && scale.len().is_multiple_of(groups)).then_some(
            GroupNorm {
                groups,
                scale,
                shift,
            },
        )
    }

    /// Normalises every map of `grid` in place, leaving any border at zero.
    ///
    /// Each group's mean and variance (divided by the count, not one less)
    /// are worked out in double precision in a fixed order, and each value
    /// is scaled and shifted in double precision and rounded once, so the
    /// answer is as close to the exact one as a single rounding allows.
    pub fn apply(&self, grid: &mut Grid) {
        debug_assert_eq!(grid.channels(), self.scale.len());
        let per_group = self.scale.len() / self.groups;
        let count = (per_group * grid.rows() * grid.columns()) as f64;
        for group in 0..self.groups {
            let channels = group * per_group..(group + 1) * per_group;
            let mut total = 0.0f64;
            for channel in channels.clone() {
                for row in 0..grid.rows() {
                    total += grid
                        .row(channel, row)
                        .iter()
                        .map(|&v| f64::from(v))
                        .sum::<f64>();
                }
            }
            let mean = total / count;
            let mut squares = 0.0f64;
            for channel in channels.clone() {
                for row in 0..grid.rows() {
                    squares += grid
                        .row(channel, row)
                        .iter()
                        .map(|&v| (f64::from(v) - mean) * (f64::from(v) - mean))
                        .sum::<f64>();
                }
            }
            let inverse_spread = 1.0 / (squares / count + EPSILON).sqrt();
            for channel in channels {
                let scale = f64::from(self.scale[channel]) * inverse_spread;
                let shift = f64::from(self.shift[channel]) - mean * scale;
                for row in 0..grid.rows() {
                    for value in grid.row_mut(channel, row) {
                        *value = (f64::from(*value) * scale + shift) as f32;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_group_comes_out_with_mean_zero_and_variance_one_before_its_scale() {
        // Two groups of one channel each: [1, 3] and [10, 10].
        let norm = GroupNorm::new(2, vec![1.0, 2.0], vec![0.0, 5.0]).expect("two per group");
        let mut grid = Grid::bordered(2, 1, 2).expect("a small grid");
        grid.row_mut(0, 0).copy_from_slice(&[1.0, 3.0]);
        grid.row_mut(1, 0).copy_from_slice(&[10.0, 10.0]);
        norm.apply(&mut grid);
        let spread = (1.0f64 + EPSILON).sqrt();
        assert_eq!(
            grid.row(0, 0),
            &[(-1.0 / spread) as f32, (1.0 / spread) as f32]
        );
        // A flat group becomes its shift.
        assert_eq!(grid.row(1, 0), &[5.0, 5.0]);
        assert_eq!(grid.padded_row(0, 0), &[0.0; 4], "the border stays zero");
    }

    #[test]
    fn channels_that_do_not_split_evenly_are_refused() {
        assert!(GroupNorm::new(4, vec![1.0; 6], vec![0.0; 6]).is_none());
        assert!(GroupNorm::new(4, vec![1.0; 8], vec![0.0; 7]).is_none());
        assert!(GroupNorm::new(4, vec![1.0; 8], vec![0.0; 8]).is_some());
    }
}
