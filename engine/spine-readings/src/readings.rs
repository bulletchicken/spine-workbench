//! Repeatable versions from the fit's marginal standard deviations.
//!
//! Independent dimensions are named, so every version retains its identity
//! alone or in a batch. Typical side values and inferred twist are held fixed.

mod sampler;
mod versions;
pub use sampler::{NamedSampler, inverse_normal};
pub use versions::{PedicleDraw, Version, VersionSet, version, versions};
