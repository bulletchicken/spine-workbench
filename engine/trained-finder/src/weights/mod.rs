//! The trained numbers, compiled in and read strictly.
//!
//! `weights/finder-weights.bin` holds every tensor of both networks as
//! little-endian 32-bit floats, end to end; `finder-weights.json` names
//! each one with its shape and where it starts. Both are compiled into the
//! engine, so the app still loads exactly one file. They are read once, the
//! first time a network is asked for, and checked strictly: a table that
//! does not fit the file, or a tensor missing, misshapen or unknown, is an
//! error rather than a crash. The convolution weights are packed for the
//! kernel at the same moment, once.
//!
//! The chain's priors and the framing's parameters are compiled in beside
//! them as text, for the stages that read them.

mod table;

use crate::corners::corner_net::CornerNet;
use crate::spine_net::SpineNet;
use shapes::errors::EngineError;
use std::sync::OnceLock;

pub(crate) use table::Tensors;

static WEIGHT_FILE: &[u8] = include_bytes!("../../weights/finder-weights.bin");

/// The weight file's table (`finder-weights.json`), as compiled in. Its
/// `sha256` names the weight file it describes, so a score can say which
/// weights it was made with; a test below checks that it does.
pub static WEIGHT_TABLE: &str = include_str!("../../weights/finder-weights.json");

/// The chain's priors (`chain-priors.json`): where each level sits and how
/// big it is, and how much each kind of misfit costs.
pub const CHAIN_PRIORS: &str = include_str!("../../weights/chain-priors.json");

/// The framing's parameters (`framing.json`): when a film is cropped to its
/// spine for a second reading, and how.
pub const FRAMING: &str = include_str!("../../weights/framing.json");

/// Both networks, ready to run.
pub struct Weights {
    spine_net: SpineNet,
    corner_net: CornerNet,
    threshold: f64,
    corner_blend: f64,
}

impl Weights {
    /// The compiled-in weights, read and checked the first time they are
    /// asked for and kept from then on. A failure is not kept, so running
    /// out of memory once does not stop a later try.
    pub fn baked() -> Result<&'static Weights, EngineError> {
        static BAKED: OnceLock<Weights> = OnceLock::new();
        if let Some(weights) = BAKED.get() {
            return Ok(weights);
        }
        let weights = Weights::read(WEIGHT_TABLE, WEIGHT_FILE)?;
        Ok(BAKED.get_or_init(|| weights))
    }

    /// Weights from a table and the file it describes, checked as
    /// [`Weights::baked`] checks the compiled-in pair.
    pub fn read(table: &str, file: &[u8]) -> Result<Weights, EngineError> {
        let mut tensors = Tensors::read(table, file)?;
        let spine_net = SpineNet::from_tensors(&mut tensors)?;
        let corner_net = CornerNet::from_tensors(&mut tensors)?;
        let (threshold, corner_blend) = (tensors.threshold, tensors.corner_blend);
        tensors.finish()?;
        Ok(Weights {
            spine_net,
            corner_net,
            threshold,
            corner_blend,
        })
    }

    /// The main network, which finds the vertebrae on the shrunk film.
    pub fn spine_net(&self) -> &SpineNet {
        &self.spine_net
    }

    /// The small network that settles each corner on the full-size film.
    pub fn corner_net(&self) -> &CornerNet {
        &self.corner_net
    }

    /// The score an outline needs to count in the framing's first look
    /// (0.5).
    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    /// How far each corner moves towards where the corner network points
    /// (0.375).
    pub fn corner_blend(&self) -> f64 {
        self.corner_blend
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shapes::errors::ErrorCode;

    fn table_with(change: impl FnOnce(&mut serde_json::Value)) -> String {
        let mut table: serde_json::Value =
            serde_json::from_str(WEIGHT_TABLE).expect("the table is JSON");
        change(&mut table);
        table.to_string()
    }

    fn refusal(table: &str, file: &[u8]) -> EngineError {
        match Weights::read(table, file) {
            Ok(_) => panic!("a damaged table was accepted"),
            Err(error) => error,
        }
    }

    #[test]
    fn the_compiled_in_weights_are_read_and_kept() {
        let weights = Weights::baked().expect("the compiled-in weights read");
        assert_eq!(weights.threshold(), 0.5);
        assert_eq!(weights.corner_blend(), 0.375);
        assert!(std::ptr::eq(weights, Weights::baked().expect("read once")));
    }

    #[test]
    fn the_file_and_the_text_files_are_the_ones_the_table_names() {
        use sha2::{Digest, Sha256};
        let table: serde_json::Value =
            serde_json::from_str(WEIGHT_TABLE).expect("the table is JSON");
        let hex = |bytes: &[u8]| {
            Sha256::digest(bytes)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        };
        assert_eq!(table["sha256"]["finder-weights.bin"], hex(WEIGHT_FILE));
        assert_eq!(
            table["sha256"]["chain-priors.json"],
            hex(CHAIN_PRIORS.as_bytes())
        );
        assert_eq!(table["sha256"]["framing.json"], hex(FRAMING.as_bytes()));
    }

    #[test]
    fn a_wrong_format_or_version_is_refused() {
        let table = table_with(|t| t["version"] = 2.into());
        assert_eq!(
            refusal(&table, WEIGHT_FILE).code,
            ErrorCode::EngineLoadFailed
        );
        let table = table_with(|t| t["format"] = "something-else".into());
        assert_eq!(
            refusal(&table, WEIGHT_FILE).code,
            ErrorCode::EngineLoadFailed
        );
        let table = table_with(|t| t["batchNormFolded"] = false.into());
        assert_eq!(
            refusal(&table, WEIGHT_FILE).code,
            ErrorCode::EngineLoadFailed
        );
        let table = table_with(|t| t["surprise"] = true.into());
        assert_eq!(
            refusal(&table, WEIGHT_FILE).code,
            ErrorCode::EngineLoadFailed
        );
        assert_eq!(
            refusal("not json", WEIGHT_FILE).code,
            ErrorCode::EngineLoadFailed
        );
    }

    #[test]
    fn a_file_of_the_wrong_length_is_refused() {
        let short = &WEIGHT_FILE[..WEIGHT_FILE.len() - 4];
        let error = refusal(WEIGHT_TABLE, short);
        assert_eq!(error.code, ErrorCode::EngineLoadFailed);
        assert!(error.message.contains("length"), "{error}");
    }

    #[test]
    fn a_tensor_missing_misshapen_moved_or_unknown_is_refused() {
        let missing = table_with(|t| {
            t["tensors"]
                .as_array_mut()
                .expect("a list")
                .retain(|e| e["name"] != "spine-net/d2.3.bias")
        });
        assert!(
            refusal(&missing, WEIGHT_FILE).message.contains("gap"),
            "a missing tensor leaves a gap"
        );

        let renamed = table_with(|t| t["tensors"][3]["name"] = "spine-net/e1.3.extra".into());
        let error = refusal(&renamed, WEIGHT_FILE);
        assert!(error.message.contains("missing"), "{error}");

        let misshapen = table_with(|t| {
            t["tensors"][2]["shape"] = serde_json::json!([24, 24, 9, 1]);
        });
        let error = refusal(&misshapen, WEIGHT_FILE);
        assert!(error.message.contains("wrong shape"), "{error}");

        let miscounted = table_with(|t| t["tensors"][0]["count"] = 217.into());
        let error = refusal(&miscounted, WEIGHT_FILE);
        assert!(error.message.contains("does not fit"), "{error}");

        let overlapping = table_with(|t| t["tensors"][1]["offset"] = 860.into());
        let error = refusal(&overlapping, WEIGHT_FILE);
        assert_eq!(error.code, ErrorCode::EngineLoadFailed);

        let past_the_end = table_with(|t| {
            let last = t["tensors"]
                .as_array_mut()
                .expect("a list")
                .last_mut()
                .expect("a tensor");
            last["offset"] = (WEIGHT_FILE.len() as u64).into();
        });
        let error = refusal(&past_the_end, WEIGHT_FILE);
        assert!(error.message.contains("does not fit"), "{error}");
    }

    #[test]
    fn a_value_that_is_not_a_number_is_refused() {
        let mut file = WEIGHT_FILE.to_vec();
        file[..4].copy_from_slice(&f32::NAN.to_le_bytes());
        let error = refusal(WEIGHT_TABLE, &file);
        assert!(error.message.contains("not a number"), "{error}");
    }
}
