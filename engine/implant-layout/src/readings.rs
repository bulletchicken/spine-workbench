//! Repeatable versions of a fitted spine, shared with the load bench.
//!
//! Patient coordinates are millimetres: x anterior, y to the patient's left,
//! z cranial; origin at the underside of L5. Named angles are radians in
//! `rotX(side_tilt) * rotY(forward_lean) * rotZ(twist)` order. A right-convex
//! curve deviates towards negative y; its signed Cobb is lower frontal tilt
//! minus upper frontal tilt. Positive regional kyphosis is upper sagittal
//! tilt minus lower sagittal tilt.
//!
//! [`NamedSampler`] accepts arbitrary dimension names; no dimension registry
//! needs extending for new bench unknowns. Every draw depends only on its
//! seed, dimension name, total version count and zero-based version index.
//! Fit noise uses marginal SDs independently across vertebrae and axes:
//! neighbouring correlations are unavailable, so joint noise is overstated.

pub use spine_readings::readings::{
    NamedSampler, PedicleDraw, Version, VersionSet, inverse_normal,
};

use shapes::spine_fit::SpineFit;
/// The shared version with a placement-facing error.
pub fn version(fit: &SpineFit, seed: u64, n: u32, k: u32) -> Result<Version, LayoutError> {
    spine_readings::readings::version(fit, seed, n, k).map_err(Into::into)
}
/// Draw shared versions without changing their identities.
pub fn versions(
    fit: &SpineFit,
    seed: u64,
    n: u32,
    range: Range<u32>,
) -> Result<VersionSet, LayoutError> {
    spine_readings::readings::versions(fit, seed, n, range).map_err(Into::into)
}

use std::ops::Range;

use crate::LayoutError;
use crate::curves::{SpineReading, measure};
use shapes::implant_layout::LayoutInput;

/// The unperturbed reading beside each generated version and its reading.
#[derive(Clone, Debug, PartialEq)]
pub struct Readings {
    pub nominal: SpineReading,
    pub versions: VersionSet,
    /// Same order as `versions.versions`, including when a batch starts
    /// after zero. The version's own index identifies its global row.
    pub measured: Vec<SpineReading>,
}

/// Measure a batch without changing any version's identity. Use `0..N` for
/// the complete ensemble; counts from partial batches must be combined
/// before they are presented as counts out of N.
pub fn readings(input: &LayoutInput, range: Range<u32>) -> Result<Readings, LayoutError> {
    let versions = versions(&input.fit, u64::from(input.seed), input.versions, range)?;
    let nominal = measure(input, &versions.nominal)?;
    let measured = versions
        .versions
        .iter()
        .map(|version| measure(input, &version.fit))
        .collect::<Result<_, _>>()?;
    Ok(Readings {
        nominal,
        versions,
        measured,
    })
}
