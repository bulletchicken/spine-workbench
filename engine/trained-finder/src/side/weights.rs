//! Optional compiled bytes and incremental float decoding and packing.

use super::network::SideNetwork;
use super::table::{self, Packet};
use super::{memory, reserve};
use crate::layers::conv::{Conv3x3, ConvPacking};
use crate::layers::pointwise::Conv1x1;
use shapes::errors::{EngineError, ErrorCode};

#[cfg(side_finder_weights)]
static PACKET: &[u8] = include_bytes!("../../weights/on-this-machine/side-finder.bin");

/// The sole optional include. The absent branch has no file expression.
pub fn packet() -> Option<&'static [u8]> {
    #[cfg(side_finder_weights)]
    {
        Some(PACKET)
    }
    #[cfg(not(side_finder_weights))]
    {
        None
    }
}

/// Cold setup. Construction borrows bytes and does no parsing or packing.
pub struct WeightSetup<'a> {
    bytes: &'a [u8],
    table: Option<Packet<'a>>,
    layer: usize,
    bias_stage: bool,
    values: Vec<f32>,
    weights: Vec<f32>,
    packing: Option<ConvPacking>,
    convs: Vec<Conv3x3>,
    head: Option<Conv1x1>,
    done: u32,
}

impl<'a> WeightSetup<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            table: None,
            layer: 0,
            bias_stage: false,
            values: Vec::new(),
            weights: Vec::new(),
            packing: None,
            convs: Vec::new(),
            head: None,
            done: 0,
        }
    }

    pub fn compiled() -> Result<WeightSetup<'static>, EngineError> {
        packet().map(WeightSetup::new).ok_or_else(|| {
            EngineError::new(
                ErrorCode::SideFinderUnavailable,
                "the trained side finder is not in this build",
            )
        })
    }

    /// Fixed for the checked architecture, including table reading and packing.
    pub fn units() -> u32 {
        1 + table::layers()
            .iter()
            .map(|layer| {
                let weights = layer.outputs * layer.inputs * layer.kernel * layer.kernel;
                weights.div_ceil(65_536) as u32
                    + 1
                    + if layer.kernel == 3 {
                        weights.div_ceil(65_536) as u32
                    } else {
                        0
                    }
            })
            .sum::<u32>()
    }

    pub fn units_done(&self) -> u32 {
        self.done
    }
    pub fn is_done(&self) -> bool {
        self.layer == 18
    }

    /// Decodes or packs at most 65,536 float values, or checks the small table.
    pub fn advance(&mut self) -> Result<bool, EngineError> {
        if self.is_done() {
            return Ok(true);
        }
        if self.table.is_none() {
            self.table = Some(Packet::read(self.bytes)?);
            self.convs.try_reserve_exact(17).map_err(|_| memory())?;
        } else if let Some(packing) = self.packing.as_mut() {
            if packing.advance()? {
                self.convs
                    .push(self.packing.take().ok_or_else(super::lost)?.finish()?);
                self.layer += 1;
            }
        } else {
            let layers = table::layers();
            let layer = &layers[self.layer];
            let name = format!(
                "{}.{}",
                layer.name,
                if self.bias_stage { "bias" } else { "weight" }
            );
            let packet = self.table.as_ref().ok_or_else(super::lost)?;
            let tensor = packet.tensor(&name)?;
            if self.values.is_empty() {
                self.values = reserve(tensor.count)?;
            }
            let end = tensor.count.min(self.values.len().saturating_add(65_536));
            for index in self.values.len()..end {
                let start = tensor.offset + 4 * index;
                let raw: [u8; 4] = packet.payload[start..start + 4]
                    .try_into()
                    .map_err(|_| super::lost())?;
                let value = f32::from_le_bytes(raw);
                if !value.is_finite() {
                    return Err(
                        table::refused("a side tensor holds a nonfinite value").with_detail(name)
                    );
                }
                self.values.push(value);
            }
            if end == tensor.count {
                if !self.bias_stage {
                    self.weights = std::mem::take(&mut self.values);
                    self.bias_stage = true;
                } else {
                    let weights = std::mem::take(&mut self.weights);
                    let bias = std::mem::take(&mut self.values);
                    if layer.kernel == 3 {
                        self.packing = Some(ConvPacking::new(
                            weights,
                            bias,
                            layer.inputs,
                            layer.outputs,
                            layer.stride,
                            true,
                        )?);
                    } else {
                        self.head = Some(Conv1x1::new(weights, bias, layer.inputs)?);
                        self.layer += 1;
                    }
                    self.bias_stage = false;
                }
            }
        }
        self.done += 1;
        Ok(self.is_done())
    }

    pub fn finish(self) -> Result<SideNetwork, EngineError> {
        if !self.is_done() {
            return Err(super::lost());
        }
        Ok(SideNetwork {
            convs: self.convs,
            head: self.head.ok_or_else(super::lost)?,
        })
    }
}
