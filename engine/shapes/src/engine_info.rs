//! What the engine reports about itself when a worker starts.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Reported by every worker at boot. If `shapesFingerprint` differs from
/// the generated TypeScript's, the wasm build is stale and the app stops
/// with `engine-out-of-sync` rather than corrupting data.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EngineInfo {
    pub engine_version: String,
    pub shapes_fingerprint: String,
    pub case_format: String,
    /// The case file format's version this engine reads and writes
    /// (`CASE_VERSION`), not the case store's version that edits and
    /// engine answers carry.
    pub case_version: u32,
}

impl EngineInfo {
    /// This build's info, given the engine's own version.
    pub fn current(engine_version: impl Into<String>) -> Self {
        EngineInfo {
            engine_version: engine_version.into(),
            shapes_fingerprint: crate::SHAPES_FINGERPRINT.to_owned(),
            case_format: crate::CASE_FORMAT.to_owned(),
            case_version: crate::CASE_VERSION,
        }
    }
}
