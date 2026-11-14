//! Strict, bounded metadata for the optional packet. Floats are read later.

use serde::Deserialize;
use shapes::errors::{EngineError, ErrorCode};

pub const CHECKPOINT: &str = "a11a131c0f17eb6b7dc56c5c8d7e51b3a57db54aa576d74671d863a6dbfe0763";
pub const PAYLOAD_BYTES: usize = 10_203_888;
pub const BLOCKS: [(&str, usize, usize, usize); 8] = [
    ("e1", 1, 24, 2),
    ("e2", 24, 48, 2),
    ("e3", 48, 96, 2),
    ("e4", 96, 160, 2),
    ("e5", 160, 224, 2),
    ("d4", 384, 160, 1),
    ("d3", 256, 96, 1),
    ("d2", 144, 64, 1),
];

/// One convolution's exact tensor names and dimensions.
#[derive(Clone, Debug)]
pub struct Layer {
    pub name: String,
    pub inputs: usize,
    pub outputs: usize,
    pub stride: usize,
    pub kernel: usize,
}

/// The 18 convolutions, in execution and packet order.
pub fn layers() -> Vec<Layer> {
    let mut layers = Vec::with_capacity(18);
    for (name, inputs, outputs, stride) in BLOCKS {
        for (suffix, inputs, stride) in [(0, inputs, stride), (3, outputs, 1)] {
            layers.push(Layer {
                name: format!("{name}.{suffix}"),
                inputs,
                outputs,
                stride,
                kernel: 3,
            });
        }
    }
    for (suffix, outputs, kernel) in [(0, 64, 3), (2, 20, 1)] {
        layers.push(Layer {
            name: format!("landmarks.{suffix}"),
            inputs: 64,
            outputs,
            stride: 1,
            kernel,
        });
    }
    layers
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Header {
    format: String,
    version: u32,
    model: String,
    checkpoint_sha256: String,
    batch_norm_eps: f64,
    batch_norm_folded: bool,
    input_shape: [usize; 3],
    output_shape: [usize; 3],
    bytes: usize,
    payload_sha256: String,
    tensors: Vec<Tensor>,
}

/// A checked byte span in the float payload.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tensor {
    pub name: String,
    pub shape: Vec<usize>,
    pub offset: usize,
    pub count: usize,
}

/// A validated header borrowing the payload. No float allocation or scan.
pub struct Packet<'a> {
    pub payload: &'a [u8],
    pub payload_sha256: String,
    tensors: Vec<Tensor>,
}

impl<'a> Packet<'a> {
    /// Checks the whole schema and the exact set and coverage of tensor spans.
    pub fn read(bytes: &'a [u8]) -> Result<Self, EngineError> {
        if bytes.get(..8) != Some(b"SWSIDE01") {
            return Err(refused("the side packet header is invalid"));
        }
        let raw: [u8; 4] = bytes
            .get(8..12)
            .ok_or_else(|| refused("the side packet is truncated"))?
            .try_into()
            .map_err(|_| refused("the side packet is truncated"))?;
        let length = u32::from_le_bytes(raw) as usize;
        // The fixed table needs under 4 KiB. This also bounds JSON allocation.
        if length > 16_384 {
            return Err(refused("the side packet table is too large"));
        }
        let end = 12usize
            .checked_add(length)
            .ok_or_else(|| refused("the side packet length overflows"))?;
        let header: Header = serde_json::from_slice(
            bytes
                .get(12..end)
                .ok_or_else(|| refused("the side packet table is truncated"))?,
        )
        .map_err(|error| {
            refused("the side packet table is invalid").with_detail(error.to_string())
        })?;
        let payload = bytes
            .get(end..)
            .ok_or_else(|| refused("the side packet is truncated"))?;
        if header.format != "side-finder-weights"
            || header.version != 1
            || header.model != "side_lumbar_heatmap_v1"
            || header.checkpoint_sha256 != CHECKPOINT
            || header.batch_norm_eps != 0.00001
            || !header.batch_norm_folded
            || header.input_shape != [1, 512, 384]
            || header.output_shape != [20, 128, 96]
            || header.bytes != PAYLOAD_BYTES
            || payload.len() != header.bytes
            || header.payload_sha256.len() != 64
            || !header
                .payload_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || header.tensors.len() != 36
        {
            return Err(refused(
                "the side packet schema does not match this network",
            ));
        }
        let mut tensors = header.tensors;
        tensors.sort_by_key(|tensor| tensor.offset);
        let mut through = 0usize;
        for tensor in &tensors {
            let count = tensor
                .shape
                .iter()
                .try_fold(1usize, |n, &side| n.checked_mul(side));
            let end = tensor
                .count
                .checked_mul(4)
                .and_then(|n| tensor.offset.checked_add(n));
            if count != Some(tensor.count)
                || tensor.count == 0
                || tensor.offset != through
                || end.is_none_or(|n| n > payload.len())
            {
                return Err(refused(
                    "a side tensor leaves a gap, overlap or invalid span",
                ));
            }
            through = end.ok_or_else(|| refused("a side tensor length overflows"))?;
        }
        if through != payload.len() {
            return Err(refused("the side tensors do not fill their payload"));
        }
        for layer in layers() {
            for (suffix, shape) in [
                (
                    "weight",
                    vec![layer.outputs, layer.inputs, layer.kernel, layer.kernel],
                ),
                ("bias", vec![layer.outputs]),
            ] {
                let name = format!("{}.{}", layer.name, suffix);
                let mut matching = tensors.iter().filter(|tensor| tensor.name == name);
                let tensor = matching.next().ok_or_else(|| {
                    refused("a required side tensor is missing").with_detail(name.clone())
                })?;
                if matching.next().is_some() || tensor.shape != shape {
                    return Err(refused("a side tensor is repeated or has the wrong shape"));
                }
            }
        }
        Ok(Self {
            payload,
            payload_sha256: header.payload_sha256,
            tensors,
        })
    }

    pub fn tensor(&self, name: &str) -> Result<&Tensor, EngineError> {
        self.tensors
            .iter()
            .find(|tensor| tensor.name == name)
            .ok_or_else(|| refused("a required side tensor is missing"))
    }
}

pub(crate) fn refused(message: &str) -> EngineError {
    EngineError::new(ErrorCode::EngineLoadFailed, message)
}
