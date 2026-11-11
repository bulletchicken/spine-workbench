//! The machine-readable report, with the git revision and a hash of the
//! dataset's file listing.

use super::Report;
use sha2::{Digest, Sha256};

/// Hash the exact input bytes; the digest discloses no file names.
pub fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn render(report: &Report) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report).map(|text| text + "\n")
}
