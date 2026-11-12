//! 3x3 convolution with zero padding 1, stride 1 or 2, bias, and an
//! optional ReLU.
//!
//! Every output value is summed in one fixed order: the bias, then each
//! input channel in turn, within it each tap row from the top, within that
//! each tap column from the left. The fast kernels below keep exactly that
//! order, so they give the same bits as the plainest loop, natively and in
//! WebAssembly alike: a faster layout only changes which values are worked
//! on together, never the order any one sum is added up in.
//!
//! The speed comes from three things. The input maps carry a ring of zeros
//! (see [`Grid::bordered`]), so the taps an output row needs are runs of
//! neighbouring values. The weights are repacked once, so the eight output
//! channels worked on together find their weights for one tap side by side.
//! And a tile of eight output channels by eight pixels is added up in
//! registers across every input channel and tap, then written once.

use crate::grid::Grid;
use shapes::errors::{EngineError, ErrorCode};
use std::ops::Range;

/// Output channels are worked out this many at a time; every convolution
/// in both networks has a multiple of it.
pub const BLOCK: usize = 8;

/// Weights per input channel within one block of output channels: nine
/// taps, each with one weight per output channel of the block.
const PER_INPUT: usize = 9 * BLOCK;

/// One 3x3 convolution, its weights packed for the kernel.
#[derive(Clone, Debug)]
pub struct Conv3x3 {
    inputs: usize,
    outputs: usize,
    stride: usize,
    relu: bool,
    /// For each block of eight output channels, for each input channel, for
    /// each tap (row by row), the eight output channels' weights.
    packed: Vec<f32>,
    bias: Vec<f32>,
}

impl Conv3x3 {
    /// Packs weights laid out as PyTorch keeps them (output channel, input
    /// channel, tap row, tap column).
    pub fn new(
        weights: &[f32],
        bias: Vec<f32>,
        inputs: usize,
        outputs: usize,
        stride: usize,
        relu: bool,
    ) -> Result<Conv3x3, EngineError> {
        if !outputs.is_multiple_of(BLOCK)
            || !(stride == 1 || stride == 2)
            || bias.len() != outputs
            || Some(weights.len()) != outputs.checked_mul(inputs).and_then(|n| n.checked_mul(9))
        {
            return Err(EngineError::new(
                ErrorCode::EngineLoadFailed,
                "a convolution's weights do not fit its shape",
            ));
        }
        let mut packed = Vec::new();
        packed.try_reserve_exact(weights.len()).map_err(|_| {
            EngineError::new(
                ErrorCode::EngineOutOfMemory,
                "not enough memory for the trained finder's weights",
            )
        })?;
        for block in 0..outputs / BLOCK {
            for input in 0..inputs {
                for tap in 0..9 {
                    for lane in 0..BLOCK {
                        let output = block * BLOCK + lane;
                        packed.push(weights[(output * inputs + input) * 9 + tap]);
                    }
                }
            }
        }
        Ok(Conv3x3 {
            inputs,
            outputs,
            stride,
            relu,
            packed,
            bias,
        })
    }

    pub fn inputs(&self) -> usize {
        self.inputs
    }

    pub fn outputs(&self) -> usize {
        self.outputs
    }

    pub fn stride(&self) -> usize {
        self.stride
    }

    /// How many blocks of eight output channels it has.
    pub fn blocks(&self) -> usize {
        self.outputs / BLOCK
    }

    /// Multiply-adds to make one output row of one block, `columns` wide.
    pub fn row_cost(&self, columns: usize) -> usize {
        BLOCK * self.inputs * 9 * columns
    }

    /// Writes every block and every row of the convolution of `inputs`
    /// into `out`.
    pub fn run_all(&self, inputs: &[&Grid], out: &mut Grid) {
        let rows = out.rows();
        for block in 0..self.blocks() {
            self.run(inputs, block, 0..rows, out, 0);
        }
    }

    /// Writes output channels `block * 8` to `block * 8 + 7`, output rows
    /// `rows`, of the convolution of `inputs` into `out`, output row `y`
    /// going to row `y - rows.start + out_first_row` of `out`.
    ///
    /// `inputs` are read one after another as if stacked into one (that is
    /// how the decoder's joined inputs are read without a copy); each must
    /// be bordered, and `stride` times the output size.
    pub fn run(
        &self,
        inputs: &[&Grid],
        block: usize,
        rows: Range<usize>,
        out: &mut Grid,
        out_first_row: usize,
    ) {
        debug_assert!(inputs.iter().all(|grid| grid.border() == 1));
        debug_assert_eq!(
            inputs.iter().map(|grid| grid.channels()).sum::<usize>(),
            self.inputs
        );
        let job = Job {
            inputs,
            weights: &self.packed[block * self.inputs * PER_INPUT..][..self.inputs * PER_INPUT],
            bias: &self.bias[block * BLOCK..][..BLOCK],
            first_channel: block * BLOCK,
            relu: self.relu,
            columns: inputs
                .first()
                .map_or(0, |grid| grid.columns() / self.stride),
        };
        if self.stride == 1 {
            job.run::<1>(rows, out, out_first_row);
        } else {
            job.run::<2>(rows, out, out_first_row);
        }
    }
}

/// One block of one convolution over some rows.
struct Job<'a> {
    inputs: &'a [&'a Grid],
    /// The block's packed weights: for each input channel, nine taps of
    /// eight.
    weights: &'a [f32],
    bias: &'a [f32],
    first_channel: usize,
    relu: bool,
    columns: usize,
}

impl Job<'_> {
    fn run<const S: usize>(&self, rows: Range<usize>, out: &mut Grid, out_first_row: usize) {
        for y in rows.clone() {
            let out_row = y - rows.start + out_first_row;
            let mut x = 0;
            while x + 8 <= self.columns {
                self.wide_tile::<S>(y, x, out, out_row);
                x += 8;
            }
            while x + 4 <= self.columns {
                self.narrow_tile::<S>(y, x, out, out_row);
                x += 4;
            }
            while x < self.columns {
                let sums = self.tile::<BLOCK, 1, S>(0, y, x);
                self.write(&sums, 0, x, out, out_row);
                x += 1;
            }
        }
    }

    /// `OB` output channels, from `lane` within the block, by `PX` pixels,
    /// as plain arrays. Natively the compiler turns the inner loop into
    /// vector instructions; four channels at a time keeps every sum in a
    /// register there.
    #[inline(always)]
    fn tile<const OB: usize, const PX: usize, const S: usize>(
        &self,
        lane: usize,
        y: usize,
        x: usize,
    ) -> [[f32; PX]; OB] {
        let mut sums = [[0.0f32; PX]; OB];
        for (sum, &bias) in sums.iter_mut().zip(&self.bias[lane..lane + OB]) {
            *sum = [bias; PX];
        }
        let mut input = 0;
        for grid in self.inputs {
            for channel in 0..grid.channels() {
                let weights = &self.weights[input * PER_INPUT..][..PER_INPUT];
                for ky in 0..3 {
                    let row = grid.padded_row(channel, y * S + ky);
                    for kx in 0..3 {
                        let taps = &row[x * S + kx..][..PX * S];
                        let tap_weights = &weights[(ky * 3 + kx) * BLOCK + lane..][..OB];
                        for (sum, &weight) in sums.iter_mut().zip(tap_weights) {
                            for (q, value) in sum.iter_mut().enumerate() {
                                *value += weight * taps[q * S];
                            }
                        }
                    }
                }
                input += 1;
            }
        }
        sums
    }

    fn write<const OB: usize, const PX: usize>(
        &self,
        sums: &[[f32; PX]; OB],
        lane: usize,
        x: usize,
        out: &mut Grid,
        out_row: usize,
    ) {
        for (k, sum) in sums.iter().enumerate() {
            let row = &mut out.row_mut(self.first_channel + lane + k, out_row)[x..x + PX];
            for (dest, &value) in row.iter_mut().zip(sum) {
                *dest = if self.relu { relu(value) } else { value };
            }
        }
    }

    #[cfg(not(all(target_arch = "wasm32", target_feature = "simd128")))]
    fn wide_tile<const S: usize>(&self, y: usize, x: usize, out: &mut Grid, out_row: usize) {
        for lane in [0, 4] {
            let sums = self.tile::<4, 8, S>(lane, y, x);
            self.write(&sums, lane, x, out, out_row);
        }
    }

    #[cfg(not(all(target_arch = "wasm32", target_feature = "simd128")))]
    fn narrow_tile<const S: usize>(&self, y: usize, x: usize, out: &mut Grid, out_row: usize) {
        for lane in [0, 4] {
            let sums = self.tile::<4, 4, S>(lane, y, x);
            self.write(&sums, lane, x, out, out_row);
        }
    }

    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    fn wide_tile<const S: usize>(&self, y: usize, x: usize, out: &mut Grid, out_row: usize) {
        let sums = self.vector_tile::<2, S>(y, x);
        self.write_vectors(&sums, x, out, out_row);
    }

    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    fn narrow_tile<const S: usize>(&self, y: usize, x: usize, out: &mut Grid, out_row: usize) {
        let sums = self.vector_tile::<1, S>(y, x);
        self.write_vectors(&sums, x, out, out_row);
    }

    /// Eight output channels by `4 * NV` pixels in WebAssembly's 128-bit
    /// vectors. The same multiplies and adds as [`Job::tile`] in the same
    /// order, four lanes at a time; WebAssembly has no fused multiply-add
    /// here, so each product is rounded before it is added.
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    #[inline(always)]
    fn vector_tile<const NV: usize, const S: usize>(
        &self,
        y: usize,
        x: usize,
    ) -> [[core::arch::wasm32::v128; NV]; BLOCK] {
        use core::arch::wasm32::{f32x4, f32x4_add, f32x4_mul, f32x4_splat, v128};
        let mut sums: [[v128; NV]; BLOCK] =
            core::array::from_fn(|k| [f32x4_splat(self.bias[k]); NV]);
        let mut input = 0;
        for grid in self.inputs {
            for channel in 0..grid.channels() {
                let weights = &self.weights[input * PER_INPUT..][..PER_INPUT];
                for ky in 0..3 {
                    let row = grid.padded_row(channel, y * S + ky);
                    for kx in 0..3 {
                        let taps = &row[x * S + kx..][..4 * NV * S];
                        let values: [v128; NV] = core::array::from_fn(|j| {
                            f32x4(
                                taps[4 * j * S],
                                taps[(4 * j + 1) * S],
                                taps[(4 * j + 2) * S],
                                taps[(4 * j + 3) * S],
                            )
                        });
                        let tap_weights = &weights[(ky * 3 + kx) * BLOCK..][..BLOCK];
                        for (sum, &weight) in sums.iter_mut().zip(tap_weights) {
                            let weight = f32x4_splat(weight);
                            for (lanes, &value) in sum.iter_mut().zip(&values) {
                                *lanes = f32x4_add(*lanes, f32x4_mul(weight, value));
                            }
                        }
                    }
                }
                input += 1;
            }
        }
        sums
    }

    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    fn write_vectors<const NV: usize>(
        &self,
        sums: &[[core::arch::wasm32::v128; NV]; BLOCK],
        x: usize,
        out: &mut Grid,
        out_row: usize,
    ) {
        use core::arch::wasm32::{f32x4_extract_lane, f32x4_pmax, f32x4_splat};
        let zero = f32x4_splat(0.0);
        for (k, sum) in sums.iter().enumerate() {
            let row = &mut out.row_mut(self.first_channel + k, out_row)[x..x + 4 * NV];
            for (dest, &lanes) in row.as_chunks_mut::<4>().0.iter_mut().zip(sum) {
                // `pmax(0, v)` is `if 0 < v { v } else { 0 }`, exactly as
                // `relu` below.
                let lanes = if self.relu {
                    f32x4_pmax(zero, lanes)
                } else {
                    lanes
                };
                dest[0] = f32x4_extract_lane::<0>(lanes);
                dest[1] = f32x4_extract_lane::<1>(lanes);
                dest[2] = f32x4_extract_lane::<2>(lanes);
                dest[3] = f32x4_extract_lane::<3>(lanes);
            }
        }
    }
}

/// Negative values, and minus zero, become zero, so both targets give the
/// same bits whatever their maximum does with signed zeros.
#[inline(always)]
pub fn relu(value: f32) -> f32 {
    if value > 0.0 { value } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layers::seeded::Numbers;

    /// The plainest convolution: unpadded maps, every tap tested against the
    /// edge, weights as PyTorch keeps them, summed in the stated order.
    fn plain_conv(
        weights: &[f32],
        bias: &[f32],
        inputs: &[&Grid],
        stride: usize,
        relu_after: bool,
    ) -> Grid {
        let planes: Vec<(&Grid, usize)> = inputs
            .iter()
            .flat_map(|grid| (0..grid.channels()).map(move |channel| (*grid, channel)))
            .collect();
        let (rows, columns) = (inputs[0].rows(), inputs[0].columns());
        let (out_rows, out_columns) = (rows / stride, columns / stride);
        let mut out = Grid::zeros(bias.len(), out_rows, out_columns).expect("a small grid");
        for (o, &b) in bias.iter().enumerate() {
            for y in 0..out_rows {
                for x in 0..out_columns {
                    let mut sum = b;
                    for (i, &(grid, channel)) in planes.iter().enumerate() {
                        for ky in 0..3 {
                            for kx in 0..3 {
                                let (iy, ix) = (y * stride + ky, x * stride + kx);
                                if iy >= 1 && ix >= 1 && iy - 1 < rows && ix - 1 < columns {
                                    sum += weights[((o * planes.len() + i) * 3 + ky) * 3 + kx]
                                        * grid.at(channel, iy - 1, ix - 1);
                                }
                            }
                        }
                    }
                    out.row_mut(o, y)[x] = if relu_after { relu(sum) } else { sum };
                }
            }
        }
        out
    }

    fn random_maps(numbers: &mut Numbers, channels: usize, rows: usize, columns: usize) -> Grid {
        let mut grid = Grid::bordered(channels, rows, columns).expect("a small grid");
        for channel in 0..channels {
            for row in 0..rows {
                for value in grid.row_mut(channel, row) {
                    *value = numbers.between(-1.0, 1.0);
                }
            }
        }
        grid
    }

    fn assert_same_bits(fast: &Grid, plain: &Grid) {
        assert_eq!(
            (fast.channels(), fast.rows(), fast.columns()),
            (plain.channels(), plain.rows(), plain.columns())
        );
        for channel in 0..plain.channels() {
            for row in 0..plain.rows() {
                let fast_bits: Vec<u32> =
                    fast.row(channel, row).iter().map(|v| v.to_bits()).collect();
                let plain_bits: Vec<u32> = plain
                    .row(channel, row)
                    .iter()
                    .map(|v| v.to_bits())
                    .collect();
                assert_eq!(fast_bits, plain_bits, "channel {channel}, row {row}");
            }
        }
    }

    /// Runs one random case through the kernel, split into the given row
    /// bands, and through the plain loop.
    fn check(
        seed: u64,
        sources: &[usize],
        outputs: usize,
        size: [usize; 2],
        stride: usize,
        relu_after: bool,
    ) {
        let mut numbers = Numbers::new(seed);
        let [rows, columns] = size;
        let inputs: usize = sources.iter().sum();
        let grids: Vec<Grid> = sources
            .iter()
            .map(|&channels| random_maps(&mut numbers, channels, rows, columns))
            .collect();
        let refs: Vec<&Grid> = grids.iter().collect();
        let weights: Vec<f32> = (0..outputs * inputs * 9)
            .map(|_| numbers.between(-0.5, 0.5))
            .collect();
        let bias: Vec<f32> = (0..outputs).map(|_| numbers.between(-0.2, 0.2)).collect();
        let conv = Conv3x3::new(&weights, bias.clone(), inputs, outputs, stride, relu_after)
            .expect("a fitting shape");
        let plain = plain_conv(&weights, &bias, &refs, stride, relu_after);

        let mut whole =
            Grid::zeros(outputs, rows / stride, columns / stride).expect("a small grid");
        conv.run_all(&refs, &mut whole);
        assert_same_bits(&whole, &plain);

        // The same answer written band by band into a bordered grid, the
        // way the stepped reading does it.
        let mut banded =
            Grid::bordered(outputs, rows / stride, columns / stride).expect("a small grid");
        let out_rows = rows / stride;
        for block in 0..conv.blocks() {
            let mut start = 0;
            while start < out_rows {
                let end = (start + 3).min(out_rows);
                conv.run(&refs, block, start..end, &mut banded, start);
                start = end;
            }
        }
        assert_same_bits(&banded, &plain);
        assert_eq!(
            banded.padded_row(0, 0).iter().sum::<f32>(),
            0.0,
            "the border stays zero"
        );
    }

    #[test]
    fn a_single_tap_moves_the_picture_by_one_pixel() {
        // Only the top-left tap is set, so each output is its upper-left
        // neighbour (zero off the edge) plus the bias.
        let mut weights = vec![0.0; 8 * 9];
        for output in 0..8 {
            weights[output * 9] = 1.0;
        }
        let conv = Conv3x3::new(&weights, vec![0.5; 8], 1, 8, 1, false).expect("a fitting shape");
        let mut picture = Grid::bordered(1, 2, 3).expect("a small grid");
        picture.row_mut(0, 0).copy_from_slice(&[1.0, 2.0, 3.0]);
        picture.row_mut(0, 1).copy_from_slice(&[4.0, 5.0, 6.0]);
        let mut out = Grid::zeros(8, 2, 3).expect("a small grid");
        conv.run_all(&[&picture], &mut out);
        assert_eq!(out.row(7, 0), &[0.5, 0.5, 0.5]);
        assert_eq!(out.row(7, 1), &[0.5, 1.5, 2.5]);
    }

    #[test]
    fn stride_two_with_relu_keeps_every_other_pixel_and_drops_negatives() {
        // The centre tap only: stride 2 keeps pixels (0, 0), (0, 2), (2, 0)...
        let mut weights = vec![0.0; 8 * 9];
        for output in 0..8 {
            weights[output * 9 + 4] = if output == 0 { 1.0 } else { -1.0 };
        }
        let conv = Conv3x3::new(&weights, vec![0.0; 8], 1, 8, 2, true).expect("a fitting shape");
        let mut picture = Grid::bordered(1, 2, 4).expect("a small grid");
        picture.row_mut(0, 0).copy_from_slice(&[1.0, 9.0, 3.0, 9.0]);
        let mut out = Grid::zeros(8, 1, 2).expect("a small grid");
        conv.run_all(&[&picture], &mut out);
        assert_eq!(out.row(0, 0), &[1.0, 3.0]);
        assert_eq!(out.row(1, 0), &[0.0, 0.0]);
        assert_eq!(out.row(1, 0)[0].to_bits(), 0, "plus zero, not minus zero");
    }

    #[test]
    fn the_kernel_gives_the_same_bits_as_the_plain_loop_at_stride_one() {
        check(1, &[3], 8, [5, 13], 1, true);
        check(2, &[5], 16, [9, 16], 1, false);
        check(3, &[1], 8, [4, 7], 1, true);
    }

    #[test]
    fn the_kernel_gives_the_same_bits_as_the_plain_loop_at_stride_two() {
        check(4, &[4], 8, [6, 26], 2, true);
        check(5, &[2], 16, [8, 16], 2, false);
        check(6, &[3], 8, [2, 2], 2, true);
    }

    #[test]
    fn joined_inputs_read_as_one_stack() {
        check(7, &[3, 2], 8, [6, 12], 1, true);
        check(8, &[1, 4, 2], 16, [4, 20], 1, false);
    }

    #[test]
    fn weights_that_do_not_fit_the_shape_are_refused() {
        assert!(Conv3x3::new(&[0.0; 8 * 9], vec![0.0; 8], 1, 8, 1, true).is_ok());
        assert!(Conv3x3::new(&[0.0; 8 * 9 - 1], vec![0.0; 8], 1, 8, 1, true).is_err());
        assert!(Conv3x3::new(&[0.0; 4 * 9], vec![0.0; 4], 1, 4, 1, true).is_err());
        assert!(Conv3x3::new(&[0.0; 8 * 9], vec![0.0; 8], 1, 8, 3, true).is_err());
    }
}
