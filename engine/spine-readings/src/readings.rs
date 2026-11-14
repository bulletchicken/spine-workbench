//! Repeatable versions with independently moved upper and lower plates.
//!
//! Independent dimensions are named, so every version retains its identity
//! alone or in a batch. Typical side values and inferred twist are held fixed.

mod plates;
mod sampler;
mod versions;
pub use sampler::{NamedSampler, inverse_normal};
pub use versions::{PedicleDraw, Version, VersionSet, version, versions, versions_for_readings};
