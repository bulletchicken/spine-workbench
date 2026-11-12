//! A stack of float maps: channels by rows by columns.
//!
//! Every map a network makes is held here, one channel after another, each
//! row by row. A map that a 3x3 convolution will read carries a ring of
//! zeros one value wide around it: that ring is the zero padding the
//! networks were trained with, so the convolution never has to ask whether
//! a tap falls off the edge, and every tap of an output row is one run of
//! neighbouring values.

use shapes::errors::{EngineError, ErrorCode};

/// A stack of maps, all the same size.
#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    channels: usize,
    rows: usize,
    columns: usize,
    border: usize,
    values: Vec<f32>,
}

impl Grid {
    /// A stack of zero maps with no border.
    pub fn zeros(channels: usize, rows: usize, columns: usize) -> Result<Grid, EngineError> {
        Grid::allocate(channels, rows, columns, 0)
    }

    /// A stack of zero maps, each with a ring of zeros one value wide around
    /// it, ready to be read by a 3x3 convolution.
    pub fn bordered(channels: usize, rows: usize, columns: usize) -> Result<Grid, EngineError> {
        Grid::allocate(channels, rows, columns, 1)
    }

    /// A stack of maps with no border from values already laid out channel
    /// by channel, row by row; `None` unless the count matches.
    pub fn from_values(
        channels: usize,
        rows: usize,
        columns: usize,
        values: Vec<f32>,
    ) -> Option<Grid> {
        let expected = channels.checked_mul(rows)?.checked_mul(columns)?;
        (values.len() == expected).then_some(Grid {
            channels,
            rows,
            columns,
            border: 0,
            values,
        })
    }

    /// Like [`Grid::zeros`] or [`Grid::bordered`], for the corner network's
    /// maps, which are at most a megabyte or two and are made inside a call
    /// that has no way to report a failure. Running out of memory for one
    /// of these means the engine is already beyond saving.
    pub(crate) fn small(channels: usize, rows: usize, columns: usize, border: usize) -> Grid {
        let len = channels * (rows + 2 * border) * (columns + 2 * border);
        Grid {
            channels,
            rows,
            columns,
            border,
            values: vec![0.0; len],
        }
    }

    fn allocate(
        channels: usize,
        rows: usize,
        columns: usize,
        border: usize,
    ) -> Result<Grid, EngineError> {
        let len = rows
            .checked_add(2 * border)
            .zip(columns.checked_add(2 * border))
            .and_then(|(padded_rows, padded_columns)| padded_rows.checked_mul(padded_columns))
            .and_then(|plane| plane.checked_mul(channels))
            .ok_or_else(out_of_memory)?;
        let mut values = Vec::new();
        values.try_reserve_exact(len).map_err(|_| out_of_memory())?;
        values.resize(len, 0.0);
        Ok(Grid {
            channels,
            rows,
            columns,
            border,
            values,
        })
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn columns(&self) -> usize {
        self.columns
    }

    /// How wide the ring of zeros around each map is: 0 or 1.
    pub fn border(&self) -> usize {
        self.border
    }

    /// Values in one row of a map, border included.
    pub(crate) fn padded_columns(&self) -> usize {
        self.columns + 2 * self.border
    }

    fn padded_rows(&self) -> usize {
        self.rows + 2 * self.border
    }

    /// One row of one map, without its border.
    pub fn row(&self, channel: usize, row: usize) -> &[f32] {
        let start = self.start_of(channel, row + self.border) + self.border;
        &self.values[start..start + self.columns]
    }

    /// One row of one map, without its border, to write.
    pub fn row_mut(&mut self, channel: usize, row: usize) -> &mut [f32] {
        let start = self.start_of(channel, row + self.border) + self.border;
        &mut self.values[start..start + self.columns]
    }

    /// One row of one map with its border, counting rows from the top
    /// border row: row `r` of the map is padded row `r + border`.
    pub(crate) fn padded_row(&self, channel: usize, padded_row: usize) -> &[f32] {
        let start = self.start_of(channel, padded_row);
        &self.values[start..start + self.padded_columns()]
    }

    /// The value at one place on one map.
    pub fn at(&self, channel: usize, row: usize, column: usize) -> f32 {
        self.row(channel, row)[column]
    }

    /// Every value of one map, row by row, border included.
    pub fn plane(&self, channel: usize) -> &[f32] {
        let len = self.padded_rows() * self.padded_columns();
        &self.values[channel * len..(channel + 1) * len]
    }

    fn start_of(&self, channel: usize, padded_row: usize) -> usize {
        (channel * self.padded_rows() + padded_row) * self.padded_columns()
    }
}

fn out_of_memory() -> EngineError {
    EngineError::new(
        ErrorCode::EngineOutOfMemory,
        "not enough memory for the trained finder's maps",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bordered_grid_keeps_its_ring_of_zeros_apart_from_its_rows() {
        let mut grid = Grid::bordered(2, 3, 4).expect("a small grid");
        grid.row_mut(1, 2).copy_from_slice(&[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(grid.at(1, 2, 3), 4.0);
        assert_eq!(grid.padded_row(1, 3), &[0.0, 1.0, 2.0, 3.0, 4.0, 0.0]);
        assert_eq!(grid.padded_row(1, 4), &[0.0; 6]);
        assert_eq!(grid.plane(0), &[0.0; 30]);
        assert_eq!(grid.plane(1).iter().sum::<f32>(), 10.0);
    }

    #[test]
    fn values_must_fill_the_stack_exactly() {
        assert!(Grid::from_values(2, 2, 2, vec![0.0; 8]).is_some());
        assert!(Grid::from_values(2, 2, 2, vec![0.0; 7]).is_none());
        assert!(Grid::from_values(usize::MAX, 2, 2, Vec::new()).is_none());
    }

    #[test]
    fn a_stack_too_big_for_memory_is_refused_not_a_crash() {
        let refused = Grid::zeros(usize::MAX / 8, 2, 2).expect_err("far too big");
        assert_eq!(refused.code, ErrorCode::EngineOutOfMemory);
        let refused = Grid::bordered(1 << 20, 1 << 20, 1 << 20).expect_err("far too big");
        assert_eq!(refused.code, ErrorCode::EngineOutOfMemory);
    }
}
