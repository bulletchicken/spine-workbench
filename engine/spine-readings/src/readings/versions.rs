//! Geometry and shared-child/per-pedicle standard-normal draws.

use super::NamedSampler;
use super::sampler::invalid;
use crate::ReadingsError;
use crate::plates::{self, PlateSpread};
use shapes::levels::Level;
use shapes::spine_fit::SpineFit;
use shapes::spine_readings::BodySide;
use shapes::spine_readings::ReadingsInput;
use std::ops::Range;

/// The independent normal attached to one pedicle. Shared-child noise is
/// carried once on [`Version`], so all pedicles use exactly the same draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PedicleDraw {
    pub level: Level,
    pub side: BodySide,
    pub z_pedicle: f64,
}

/// One perturbed copy of the fit. Centres and independently drawn upper
/// and lower plate tilts move; wedges and body angles encode those plates.
/// Sizes, reported SDs and inferred flags remain the fit's.
#[derive(Clone, Debug, PartialEq)]
pub struct Version {
    /// Zero-based index in the complete ensemble, independent of batching.
    pub index: u32,
    pub fit: SpineFit,
    pub z_child: f64,
    /// Top to bottom, left before right.
    pub pedicles: Vec<PedicleDraw>,
}

impl Version {
    pub fn pedicle(&self, level: Level, side: BodySide) -> Option<&PedicleDraw> {
        self.pedicles
            .iter()
            .find(|draw| draw.level == level && draw.side == side)
    }
}

/// The nominal fit is retained alongside its perturbed versions.
#[derive(Clone, Debug, PartialEq)]
pub struct VersionSet {
    pub nominal: SpineFit,
    /// Whole ensemble count, which can exceed this batch's length.
    pub total: u32,
    pub versions: Vec<Version>,
}

/// Draw a single version. Geometry dimension names use top-to-bottom
/// positions, not level labels, so relabelling every level changes no draw.
/// These names are `versions-geometry/{i}/centre/{axis}` and
/// `versions-geometry/{i}/plate/{front|side}/{upper|lower}` and
/// `versions-geometry/{i}/angle/2` for twist, with zero-based positions. Pedicles use `versions-pedicles/child` and
/// `versions-pedicles/{i}/left` or `/right`.
pub fn version(fit: &SpineFit, seed: u64, n: u32, k: u32) -> Result<Version, ReadingsError> {
    validate(fit)?;
    let sampler = NamedSampler::new(seed, n)?;
    draw(fit, &plates::from_fit(fit), &sampler, k)
}

/// Draw `range` of the complete N-version ensemble. Empty batches are
/// permitted; reversed ranges and endpoints past N are refused.
pub fn versions(
    fit: &SpineFit,
    seed: u64,
    n: u32,
    range: Range<u32>,
) -> Result<VersionSet, ReadingsError> {
    validate(fit)?;
    let sampler = NamedSampler::new(seed, n)?;
    if range.start > range.end || range.end > n {
        return Err(invalid("version-range", "outside-ensemble"));
    }
    let spreads = plates::from_fit(fit);
    make_set(fit, &spreads, sampler, n, range)
}

/// Draw versions with plate spreads from the supplied films and their stated
/// mark spread. A level absent from those films uses the fit-only fallback.
pub fn versions_for_readings(
    input: &ReadingsInput,
    range: Range<u32>,
) -> Result<VersionSet, ReadingsError> {
    validate(&input.fit)?;
    let sampler = NamedSampler::new(u64::from(input.seed), input.versions)?;
    if range.start > range.end || range.end > input.versions {
        return Err(invalid("version-range", "outside-ensemble"));
    }
    let spreads = plates::from_input(input)?;
    make_set(&input.fit, &spreads, sampler, input.versions, range)
}

fn make_set(
    fit: &SpineFit,
    spreads: &[PlateSpread],
    sampler: NamedSampler,
    total: u32,
    range: Range<u32>,
) -> Result<VersionSet, ReadingsError> {
    let versions = range
        .map(|k| draw(fit, spreads, &sampler, k))
        .collect::<Result<_, _>>()?;
    Ok(VersionSet {
        nominal: fit.clone(),
        total,
        versions,
    })
}

fn draw(
    fit: &SpineFit,
    spreads: &[PlateSpread],
    sampler: &NamedSampler,
    k: u32,
) -> Result<Version, ReadingsError> {
    // This also checks k for an empty chain (although validate refuses one).
    let z_child = sampler.normal("versions-pedicles/child", k)?;
    let has_side = fit.side_film.is_some();
    let mut perturbed = fit.clone();
    let mut pedicles = Vec::with_capacity(2 * fit.vertebrae.len());
    for (i, vertebra) in perturbed.vertebrae.iter_mut().enumerate() {
        for axis in 0..3 {
            let z = sampler.normal(&format!("versions-geometry/{i}/centre/{axis}"), k)?;
            if axis != 0 || (has_side && !vertebra.inferred.depth) {
                vertebra.pose.centre[axis] += vertebra.sd.centre_mm[axis] * z;
            }
        }
        super::plates::perturb(
            vertebra,
            &fit.vertebrae[i],
            spreads[i],
            sampler,
            i,
            k,
            has_side,
        )?;
        for (side, name) in [(BodySide::Left, "left"), (BodySide::Right, "right")] {
            pedicles.push(PedicleDraw {
                level: vertebra.level,
                side,
                z_pedicle: sampler.normal(&format!("versions-pedicles/{i}/{name}"), k)?,
            });
        }
    }
    if perturbed.vertebrae.iter().any(|vertebra| {
        vertebra
            .pose
            .centre
            .iter()
            .chain([
                &vertebra.angles.side_tilt_rad,
                &vertebra.angles.forward_lean_rad,
                &vertebra.angles.twist_rad,
            ])
            .any(|number| !number.is_finite())
    }) {
        return Err(invalid("fit", "version-overflow"));
    }
    Ok(Version {
        index: k,
        fit: perturbed,
        z_child,
        pedicles,
    })
}

fn validate(fit: &SpineFit) -> Result<(), ReadingsError> {
    if fit.vertebrae.is_empty() {
        return Err(ReadingsError::NoSpine);
    }
    for (i, vertebra) in fit.vertebrae.iter().enumerate() {
        let pose = &vertebra.pose;
        let sizes = [
            pose.height,
            pose.top_half_width,
            pose.top_half_depth,
            pose.bottom_half_width,
            pose.bottom_half_depth,
        ];
        let angles = [
            vertebra.angles.side_tilt_rad,
            vertebra.angles.forward_lean_rad,
            vertebra.angles.twist_rad,
        ];
        if pose.level != vertebra.level || (i > 0 && fit.vertebrae[i - 1].level >= vertebra.level) {
            return Err(invalid("fit.vertebrae", "levels-must-agree-and-increase"));
        }
        if pose
            .centre
            .iter()
            .chain(&pose.rotation)
            .chain(&angles)
            .any(|number| !number.is_finite())
            || vertebra
                .corner_tilt_rad
                .is_some_and(|tilt| !tilt.is_finite())
            || sizes.iter().any(|size| !size.is_finite() || *size <= 0.0)
            || [pose.wedge_front, pose.wedge_side]
                .iter()
                .any(|wedge| !wedge.is_finite() || wedge.abs() >= core::f64::consts::PI)
        {
            return Err(invalid("fit.vertebrae", "invalid-pose"));
        }
        if vertebra
            .sd
            .centre_mm
            .iter()
            .chain(&vertebra.sd.angles_rad)
            .any(|sd| !sd.is_finite() || *sd < 0.0)
        {
            return Err(invalid(
                "fit.vertebrae.sd",
                "must-be-finite-and-nonnegative",
            ));
        }
    }
    Ok(())
}
