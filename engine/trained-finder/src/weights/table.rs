//! The weight file's table of contents, read strictly.

use serde::Deserialize;
use serde::de::IgnoredAny;
use shapes::errors::{EngineError, ErrorCode};
use std::collections::{BTreeMap, BTreeSet};

/// What `finder-weights.json` must say about itself.
const FORMAT: &str = "spine-finder-weights";
const VERSION: u32 = 1;
const VALUES: &str = "float32, little-endian, each tensor in row-major order";

/// `finder-weights.json`, as written.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TableText {
    format: String,
    version: u32,
    values: String,
    batch_norm_folded: bool,
    batch_norm_eps: f64,
    threshold: f64,
    corner_blend: f64,
    bytes: usize,
    /// The files' fingerprints and the checkpoints they were packed from:
    /// a test checks the fingerprints; nothing at run time needs either.
    #[serde(rename = "sha256")]
    _fingerprints: IgnoredAny,
    #[serde(rename = "packedFrom")]
    _packed_from: IgnoredAny,
    tensors: Vec<EntryText>,
}

/// One tensor's line in the table: where its values sit in the file, in
/// bytes, and its shape.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryText {
    name: String,
    shape: Vec<usize>,
    offset: usize,
    count: usize,
}

/// The table checked against the file, handing out each tensor once.
pub(crate) struct Tensors<'a> {
    bytes: &'a [u8],
    entries: BTreeMap<String, EntryText>,
    taken: BTreeSet<String>,
    pub(crate) threshold: f64,
    pub(crate) corner_blend: f64,
}

impl<'a> Tensors<'a> {
    /// Reads the table and checks it against the file: the format and
    /// version, that the batch norms are folded in, every tensor's count
    /// against its shape, and that the tensors fill the file exactly, end
    /// to end, with no gap or overlap.
    pub(crate) fn read(table: &str, bytes: &'a [u8]) -> Result<Tensors<'a>, EngineError> {
        let text: TableText = serde_json::from_str(table).map_err(|error| {
            refused("the weight table cannot be read").with_detail(error.to_string())
        })?;
        if text.format != FORMAT || text.version != VERSION || text.values != VALUES {
            return Err(refused("the weight table is not one this engine reads")
                .with_detail(format!("{} version {}", text.format, text.version)));
        }
        // The packing script folds each batch norm into the convolution
        // before it with this epsilon; nothing else would match.
        if !text.batch_norm_folded || text.batch_norm_eps != 1e-5 {
            return Err(refused(
                "the weight table's batch norms are not folded as expected",
            ));
        }
        if !(0.05..=1.0).contains(&text.threshold) || !(0.0..=1.0).contains(&text.corner_blend) {
            return Err(refused(
                "the weight table's threshold or corner blend is out of range",
            ));
        }
        if text.bytes != bytes.len() {
            return Err(refused("the weight file is not the length its table says")
                .with_detail(format!("{} bytes, table says {}", bytes.len(), text.bytes)));
        }
        let mut entries = BTreeMap::new();
        let mut spans = Vec::with_capacity(text.tensors.len());
        for entry in text.tensors {
            let count = entry
                .shape
                .iter()
                .try_fold(1usize, |product, &side| product.checked_mul(side));
            let end = entry
                .count
                .checked_mul(4)
                .and_then(|length| length.checked_add(entry.offset));
            if count != Some(entry.count)
                || !entry.offset.is_multiple_of(4)
                || end.is_none_or(|end| end > bytes.len())
            {
                return Err(
                    refused("a tensor does not fit the weight file").with_detail(entry.name)
                );
            }
            spans.push((entry.offset, entry.count * 4, entry.name.clone()));
            if let Some(repeated) = entries.insert(entry.name.clone(), entry) {
                return Err(refused("a tensor is listed twice").with_detail(repeated.name));
            }
        }
        spans.sort();
        let mut reached = 0;
        for (offset, length, name) in spans {
            if offset != reached {
                return Err(
                    refused("the tensors leave a gap or overlap in the weight file")
                        .with_detail(name),
                );
            }
            reached = offset + length;
        }
        if reached != bytes.len() {
            return Err(refused(
                "the tensors do not reach the end of the weight file",
            ));
        }
        Ok(Tensors {
            bytes,
            entries,
            taken: BTreeSet::new(),
            threshold: text.threshold,
            corner_blend: text.corner_blend,
        })
    }

    /// The values of the tensor called `name`, which must have exactly
    /// `shape` and be finite.
    pub(crate) fn take(&mut self, name: &str, shape: &[usize]) -> Result<Vec<f32>, EngineError> {
        let entry = self
            .entries
            .get(name)
            .ok_or_else(|| refused("a tensor the network needs is missing").with_detail(name))?;
        if entry.shape != shape {
            return Err(refused("a tensor has the wrong shape")
                .with_detail(format!("{name}: {:?}, expected {shape:?}", entry.shape)));
        }
        if !self.taken.insert(name.to_owned()) {
            return Err(refused("a tensor was asked for twice").with_detail(name));
        }
        let raw = &self.bytes[entry.offset..entry.offset + 4 * entry.count];
        let mut values = Vec::new();
        values.try_reserve_exact(entry.count).map_err(|_| {
            EngineError::new(
                ErrorCode::EngineOutOfMemory,
                "not enough memory for the trained finder's weights",
            )
        })?;
        values.extend(
            raw.as_chunks::<4>()
                .0
                .iter()
                .map(|&four| f32::from_le_bytes(four)),
        );
        if values.iter().any(|value| !value.is_finite()) {
            return Err(refused("a tensor holds a value that is not a number").with_detail(name));
        }
        Ok(values)
    }

    /// Every tensor in the table must have been taken: one the networks do
    /// not know means the file is for some other network.
    pub(crate) fn finish(self) -> Result<(), EngineError> {
        match self.entries.keys().find(|name| !self.taken.contains(*name)) {
            Some(unknown) => Err(refused("the weight file holds a tensor no network uses")
                .with_detail(unknown.clone())),
            None => Ok(()),
        }
    }
}

/// The compiled-in weights cannot be used: the engine was built wrongly.
fn refused(message: &str) -> EngineError {
    EngineError::new(ErrorCode::EngineLoadFailed, message)
}
