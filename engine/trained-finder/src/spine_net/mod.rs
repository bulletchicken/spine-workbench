//! The main network, run in small steps.
//!
//! It reads a film shrunk to 384 by 768 and gives three maps a quarter that
//! size, one cell for each 4 by 4 square of the picture: how likely a
//! vertebra's centre is to lie in the cell (as a logit, before the
//! sigmoid), where that vertebra's four corners are from its centre, and
//! where in the cell the centre lies.
//!
//! The network is a small U-Net. Five encoder blocks each halve the map
//! (24, 48, 96, 160 and 224 channels); three decoder blocks each double it
//! again, joining the doubled map with the encoder's map of the same size
//! (160, 96 and 64 channels); three heads read the last. Every block is two
//! 3x3 convolutions, each followed by a ReLU, with the batch norms already
//! folded into their weights.
//!
//! About nine billion multiply-adds is far too long for one call in the
//! browser, so a reading is cut into small units of work (see
//! [`SpineReading`]): one unit is one block of eight output channels over a
//! band of rows of one convolution, a few channels of one upsampling, or a
//! band of rows of one head. The units and their order depend on nothing
//! but the network, so the maps never depend on how the work was sliced.

mod stepped;

pub use stepped::SpineReading;

use crate::grid::Grid;
use crate::layers::conv::Conv3x3;
use crate::layers::pointwise::Conv1x1;
use crate::weights::Tensors;
use shapes::errors::EngineError;
use stepped::Planned;

/// The picture the network reads is this wide…
pub const PICTURE_WIDTH: usize = 384;
/// …and this tall, one byte per pixel, row by row.
pub const PICTURE_HEIGHT: usize = 768;
/// Its maps are a quarter the picture's width…
pub const MAP_WIDTH: usize = PICTURE_WIDTH / 4;
/// …and a quarter its height.
pub const MAP_HEIGHT: usize = PICTURE_HEIGHT / 4;

/// Each block's name, its input and output channels, and the stride of
/// its first convolution. A decoder block's input is the doubled map below
/// it joined with the encoder map of the same size, in that order.
const BLOCKS: [(&str, usize, usize, usize); 8] = [
    ("e1", 1, 24, 2),
    ("e2", 24, 48, 2),
    ("e3", 48, 96, 2),
    ("e4", 96, 160, 2),
    ("e5", 160, 224, 2),
    ("d4", 224 + 160, 160, 1),
    ("d3", 160 + 96, 96, 1),
    ("d2", 96 + 48, 64, 1),
];

/// Each head's name and how many maps it gives.
const HEADS: [(&str, usize); 3] = [("heat", 1), ("corners", 8), ("sub", 2)];

/// Channels the last decoder block gives and each head's first convolution
/// keeps.
const HEAD_CHANNELS: usize = 64;

/// The main network, its weights packed.
pub struct SpineNet {
    /// Two per block, in the order of [`BLOCKS`].
    convs: Vec<Conv3x3>,
    /// In the order of [`HEADS`].
    heads: Vec<Head>,
    plan: Vec<Planned>,
}

/// One head: a 3x3 convolution with ReLU, then a 1x1 convolution.
struct Head {
    conv: Conv3x3,
    out: Conv1x1,
}

/// The three maps a reading gives, each [`MAP_HEIGHT`] by [`MAP_WIDTH`]
/// with no border.
#[derive(Clone, Debug, PartialEq)]
pub struct HeadMaps {
    /// One map: the logit of a vertebra's centre lying in each cell.
    pub heat: Grid,
    /// Eight maps: the four corners of the vertebra centred in each cell,
    /// in cells from its centre, as (across, down) for the upper left,
    /// upper right, lower left and lower right corners in that order (the
    /// order the network was taught in).
    pub corners: Grid,
    /// Two maps: where in its cell the centre lies, across and down, about
    /// 0 to 1.
    pub centre_in_cell: Grid,
}

impl SpineNet {
    pub(crate) fn from_tensors(tensors: &mut Tensors) -> Result<SpineNet, EngineError> {
        let mut convs = Vec::with_capacity(2 * BLOCKS.len());
        for (name, inputs, outputs, stride) in BLOCKS {
            for (conv, conv_inputs, conv_stride) in [("0", inputs, stride), ("3", outputs, 1)] {
                let weights = tensors.take(
                    &format!("spine-net/{name}.{conv}.weight"),
                    &[outputs, conv_inputs, 3, 3],
                )?;
                let bias = tensors.take(&format!("spine-net/{name}.{conv}.bias"), &[outputs])?;
                convs.push(Conv3x3::new(
                    &weights,
                    bias,
                    conv_inputs,
                    outputs,
                    conv_stride,
                    true,
                )?);
            }
        }
        let mut heads = Vec::with_capacity(HEADS.len());
        for (name, outputs) in HEADS {
            let weights = tensors.take(
                &format!("spine-net/{name}.0.weight"),
                &[HEAD_CHANNELS, HEAD_CHANNELS, 3, 3],
            )?;
            let bias = tensors.take(&format!("spine-net/{name}.0.bias"), &[HEAD_CHANNELS])?;
            let conv = Conv3x3::new(&weights, bias, HEAD_CHANNELS, HEAD_CHANNELS, 1, true)?;
            let weights = tensors.take(
                &format!("spine-net/{name}.2.weight"),
                &[outputs, HEAD_CHANNELS, 1, 1],
            )?;
            let bias = tensors.take(&format!("spine-net/{name}.2.bias"), &[outputs])?;
            heads.push(Head {
                conv,
                out: Conv1x1::new(weights, bias, HEAD_CHANNELS)?,
            });
        }
        let plan = stepped::plan(&convs, &heads);
        Ok(SpineNet { convs, heads, plan })
    }

    /// Starts a reading of `picture`: [`PICTURE_WIDTH`] by
    /// [`PICTURE_HEIGHT`] grey values, row by row. Nothing is worked out
    /// yet; the first unit of work turns the bytes into the network's
    /// input.
    pub fn start(&self, picture: &[u8]) -> Result<SpineReading<'_>, EngineError> {
        SpineReading::new(self, picture)
    }

    /// How many units of work one reading takes. The same for every
    /// picture.
    pub fn units(&self) -> u32 {
        stepped::total_units(&self.plan)
    }
}
