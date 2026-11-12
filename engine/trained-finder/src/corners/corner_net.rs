//! The small network that says where each corner of one outline looks to be.
//!
//! It reads a 128 by 128 square of the full-size film cut out around one
//! outline, turned so the outline's upper edge runs across (see
//! [`super::patch`]), together with where the outline's four corners sit
//! in that square. It halves the square to 64 by 64, evens out its
//! brightness, and adds four maps of a soft spot around each corner's
//! starting place. A small U-Net (three encoder blocks of 16, 32 and 48
//! channels, two decoder blocks back up, each block two 3x3 convolutions
//! with group norm and SiLU) then scores every cell of the 64 by 64 square
//! for each corner. The soft spot's logarithm is added to those scores, so
//! the network only says how far a corner should move, and each corner's
//! new place is the mean of the cells weighted by the softmax of its
//! scores. What comes back is how far each corner moves, in patch units:
//! the square's side is 1, its centre 0, across first.
//!
//! One patch is about a hundred million multiply-adds: a few milliseconds
//! in WebAssembly.

use super::patch::PATCH_SIDE;
use crate::grid::Grid;
use crate::layers::activation::silu_maps;
use crate::layers::conv::Conv3x3;
use crate::layers::group_norm::GroupNorm;
use crate::layers::pointwise::Conv1x1;
use crate::layers::resample::{average_pool_2x2, upsample_twice};
use crate::layers::softmax_mean::softmax_mean;
use crate::weights::Tensors;
use shapes::errors::{EngineError, ErrorCode};

/// The side of the square the network scores, half the patch's.
const SIDE: usize = PATCH_SIDE / 2;

/// The group norms split the channels into this many groups.
const GROUPS: usize = 4;

/// How wide the soft spot around each starting corner is, in patch units.
const SPOT_WIDTH: f64 = 0.08;

/// The patch's brightness spread is never taken as less than this, so a
/// flat patch is not blown up into noise.
const LEAST_SPREAD: f32 = 0.04;

/// Each block's name and its input and output channels. A decoder block's
/// input is the doubled map below it joined with the encoder map of the
/// same size, in that order.
const BLOCKS: [(&str, usize, usize); 5] = [
    ("enc1", 1 + 4, 16),
    ("enc2", 16, 32),
    ("enc3", 32, 48),
    ("dec2", 48 + 32, 32),
    ("dec1", 32 + 16, 16),
];

/// The corner network, its weights packed.
pub struct CornerNet {
    /// In the order of [`BLOCKS`].
    blocks: [Block; 5],
    head: Conv1x1,
    /// The centre of every cell of the 64 by 64 square in patch units,
    /// row by row, as (across, down).
    cells: Vec<[f32; 2]>,
}

/// Two 3x3 convolutions, each followed by group norm and SiLU.
struct Block {
    first: Conv3x3,
    first_norm: GroupNorm,
    second: Conv3x3,
    second_norm: GroupNorm,
}

impl Block {
    fn from_tensors(
        tensors: &mut Tensors,
        (name, inputs, outputs): (&str, usize, usize),
    ) -> Result<Block, EngineError> {
        let mut conv = |index: &str, conv_inputs: usize| -> Result<Conv3x3, EngineError> {
            let weights = tensors.take(
                &format!("corner-net/{name}.{index}.weight"),
                &[outputs, conv_inputs, 3, 3],
            )?;
            let bias = tensors.take(&format!("corner-net/{name}.{index}.bias"), &[outputs])?;
            Conv3x3::new(&weights, bias, conv_inputs, outputs, 1, false)
        };
        let first = conv("0", inputs)?;
        let second = conv("3", outputs)?;
        let mut norm = |index: &str| -> Result<GroupNorm, EngineError> {
            let scale = tensors.take(&format!("corner-net/{name}.{index}.weight"), &[outputs])?;
            let shift = tensors.take(&format!("corner-net/{name}.{index}.bias"), &[outputs])?;
            GroupNorm::new(GROUPS, scale, shift).ok_or_else(|| {
                EngineError::new(
                    ErrorCode::EngineLoadFailed,
                    "a group norm's channels do not split into its groups",
                )
            })
        };
        Ok(Block {
            first,
            first_norm: norm("1")?,
            second,
            second_norm: norm("4")?,
        })
    }

    fn run(&self, inputs: &[&Grid], rows: usize) -> Grid {
        let mut middle = Grid::small(self.first.outputs(), rows, rows, 1);
        self.first.run_all(inputs, &mut middle);
        self.first_norm.apply(&mut middle);
        silu_maps(&mut middle);
        let mut out = Grid::small(self.second.outputs(), rows, rows, 1);
        self.second.run_all(&[&middle], &mut out);
        self.second_norm.apply(&mut out);
        silu_maps(&mut out);
        out
    }
}

impl CornerNet {
    pub(crate) fn from_tensors(tensors: &mut Tensors) -> Result<CornerNet, EngineError> {
        let blocks = [
            Block::from_tensors(tensors, BLOCKS[0])?,
            Block::from_tensors(tensors, BLOCKS[1])?,
            Block::from_tensors(tensors, BLOCKS[2])?,
            Block::from_tensors(tensors, BLOCKS[3])?,
            Block::from_tensors(tensors, BLOCKS[4])?,
        ];
        let weights = tensors.take("corner-net/head.weight", &[4, 16, 1, 1])?;
        let bias = tensors.take("corner-net/head.bias", &[4])?;
        let head = Conv1x1::new(weights, bias, 16)?;
        let place = |index: usize| (index as f32 + 0.5) / SIDE as f32 - 0.5;
        let cells = (0..SIDE * SIDE)
            .map(|cell| [place(cell % SIDE), place(cell / SIDE)])
            .collect();
        Ok(CornerNet {
            blocks,
            head,
            cells,
        })
    }

    /// How far each corner of `patch`'s outline moves, in patch units.
    pub fn deltas(&self, patch: &super::patch::Patch) -> [[f32; 2]; 4] {
        self.deltas_for(&patch.pixels, &patch.initial)
    }

    /// How far each corner moves, in patch units, for a patch of
    /// [`PATCH_SIDE`] by [`PATCH_SIDE`] grey values (row by row) whose
    /// outline starts at `initial` (patch units; upper left, upper right,
    /// lower right, lower left).
    ///
    /// A patch is always the full size; were one short, the missing
    /// pixels would read as black rather than stop the engine.
    pub fn deltas_for(&self, pixels: &[u8], initial: &[[f32; 2]; 4]) -> [[f32; 2]; 4] {
        debug_assert_eq!(pixels.len(), PATCH_SIDE * PATCH_SIDE);
        let mut patch = Grid::small(1, PATCH_SIDE, PATCH_SIDE, 0);
        for row in 0..PATCH_SIDE {
            let bytes = pixels
                .get(row * PATCH_SIDE..(row + 1) * PATCH_SIDE)
                .unwrap_or(&[]);
            for (value, &byte) in patch.row_mut(0, row).iter_mut().zip(bytes) {
                *value = f32::from(byte) / 255.0;
            }
        }
        let mut halved = Grid::small(1, SIDE, SIDE, 0);
        average_pool_2x2(&patch, &mut halved);
        let (mean, spread) = mean_and_spread(halved.plane(0));

        // The input: the evened-out picture, then a soft spot around each
        // corner's starting place.
        let mut input = Grid::small(5, SIDE, SIDE, 1);
        for row in 0..SIDE {
            for (value, &pixel) in input.row_mut(0, row).iter_mut().zip(halved.row(0, row)) {
                *value = ((pixel - mean) / spread).clamp(-4.0, 4.0);
            }
        }
        let priors = self.priors(initial);
        for (corner, prior) in priors.iter().enumerate() {
            for row in 0..SIDE {
                for (value, &logit) in input
                    .row_mut(corner + 1, row)
                    .iter_mut()
                    .zip(prior.row(0, row))
                {
                    *value = libm::expf(logit);
                }
            }
        }

        let [enc1, enc2, enc3, dec2, dec1] = &self.blocks;
        let a = enc1.run(&[&input], SIDE);
        let mut pooled = Grid::small(a.channels(), SIDE / 2, SIDE / 2, 1);
        average_pool_2x2(&a, &mut pooled);
        let b = enc2.run(&[&pooled], SIDE / 2);
        let mut pooled = Grid::small(b.channels(), SIDE / 4, SIDE / 4, 1);
        average_pool_2x2(&b, &mut pooled);
        let c = enc3.run(&[&pooled], SIDE / 4);
        let mut doubled = Grid::small(c.channels(), SIDE / 2, SIDE / 2, 1);
        upsample_twice(&c, 0..c.channels(), &mut doubled);
        let d = dec2.run(&[&doubled, &b], SIDE / 2);
        let mut doubled = Grid::small(d.channels(), SIDE, SIDE, 1);
        upsample_twice(&d, 0..d.channels(), &mut doubled);
        let e = dec1.run(&[&doubled, &a], SIDE);

        let mut scores = Grid::small(4, SIDE, SIDE, 0);
        self.head.run(&e, 0..SIDE, &mut scores, 0);
        let mut deltas = [[0.0f32; 2]; 4];
        for (corner, delta) in deltas.iter_mut().enumerate() {
            let logits: Vec<f32> = scores
                .plane(corner)
                .iter()
                .zip(priors[corner].plane(0))
                .map(|(&score, &prior)| score + prior)
                .collect();
            let place = softmax_mean(&logits, &self.cells);
            *delta = [place[0] - initial[corner][0], place[1] - initial[corner][1]];
        }
        deltas
    }

    /// For each corner, the logarithm of the soft spot around its starting
    /// place: minus the squared distance from each cell, over twice the
    /// spot's width squared.
    fn priors(&self, initial: &[[f32; 2]; 4]) -> [Grid; 4] {
        // The reference divides by `2 * 0.08 ** 2` worked out in double
        // precision and rounded once.
        let spread = (2.0 * (SPOT_WIDTH * SPOT_WIDTH)) as f32;
        initial.map(|start| {
            let mut prior = Grid::small(1, SIDE, SIDE, 0);
            for row in 0..SIDE {
                for (value, cell) in prior
                    .row_mut(0, row)
                    .iter_mut()
                    .zip(&self.cells[row * SIDE..])
                {
                    let across = cell[0] - start[0];
                    let down = cell[1] - start[1];
                    *value = -(across * across + down * down) / spread;
                }
            }
            prior
        })
    }
}

/// The mean of `values` and their spread (the standard deviation dividing
/// by one less than the count), worked out in double precision and rounded
/// once; the spread is never less than [`LEAST_SPREAD`].
fn mean_and_spread(values: &[f32]) -> (f32, f32) {
    let count = values.len() as f64;
    let mean = values.iter().map(|&v| f64::from(v)).sum::<f64>() / count;
    let squares = values
        .iter()
        .map(|&v| (f64::from(v) - mean) * (f64::from(v) - mean))
        .sum::<f64>();
    let spread = (squares / (count - 1.0)).sqrt() as f32;
    (mean as f32, spread.max(LEAST_SPREAD))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weights::Weights;

    #[test]
    fn a_flat_patch_reads_with_the_least_spread() {
        assert_eq!(mean_and_spread(&[0.5; 16]), (0.5, LEAST_SPREAD));
        let (mean, spread) = mean_and_spread(&[0.0, 1.0]);
        assert_eq!((mean, spread), (0.5, std::f32::consts::FRAC_1_SQRT_2));
    }

    #[test]
    fn the_cells_run_across_then_down_and_are_centred() {
        let net = Weights::baked()
            .expect("the compiled-in weights read")
            .corner_net();
        assert_eq!(net.cells.len(), SIDE * SIDE);
        assert_eq!(net.cells[0], [-0.5 + 0.5 / 64.0, -0.5 + 0.5 / 64.0]);
        assert_eq!(net.cells[1], [-0.5 + 1.5 / 64.0, -0.5 + 0.5 / 64.0]);
        assert_eq!(net.cells[SIDE], [-0.5 + 0.5 / 64.0, -0.5 + 1.5 / 64.0]);
    }

    #[test]
    fn the_soft_spot_peaks_at_the_starting_corner() {
        let net = Weights::baked()
            .expect("the compiled-in weights read")
            .corner_net();
        let start = net.cells[10 * SIDE + 20];
        let priors = net.priors(&[start; 4]);
        assert_eq!(priors[0].at(0, 10, 20), 0.0);
        assert!(priors[0].at(0, 10, 21) < 0.0);
        assert!(priors[3].at(0, 11, 20) < 0.0);
    }

    #[test]
    fn a_patch_gives_small_finite_moves_the_same_every_time() {
        // A bright square on a dark film, its outline started a little
        // inside the square's corners.
        let mut pixels = vec![20u8; PATCH_SIDE * PATCH_SIDE];
        for row in 40..88 {
            pixels[row * PATCH_SIDE + 30..row * PATCH_SIDE + 98].fill(200);
        }
        let initial = [[-0.24, -0.16], [0.24, -0.16], [0.24, 0.16], [-0.24, 0.16]];
        let net = Weights::baked()
            .expect("the compiled-in weights read")
            .corner_net();
        let deltas = net.deltas_for(&pixels, &initial);
        for delta in deltas {
            assert!(
                delta.iter().all(|v| v.is_finite() && v.abs() < 0.5),
                "{deltas:?}"
            );
        }
        assert_eq!(deltas, net.deltas_for(&pixels, &initial));
    }
}
