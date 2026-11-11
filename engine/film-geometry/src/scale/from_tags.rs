//! The spacing tags and magnification, read by the rules in order of
//! preference.

use shapes::film::{KeptTags, SpacingCalibration};
use shapes::film_geometry::{FileScaleRule, ScalePlane, ScaleResult, ScaleSource};

/// Every supported interpretation of the spacing tags, best first.
/// Spacing is DICOM row then column; the returned primary spacing is horizontal.
/// The relative uncertainties are experimental starting values, not depth error.
pub fn from_tags(tags: Option<&KeptTags>) -> Vec<ScaleResult> {
    let Some(tags) = tags else { return Vec::new() };
    let ps = valid_spacing(tags.pixel_spacing);
    let ips = valid_spacing(tags.imager_pixel_spacing);
    let nsps = valid_spacing(tags.nominal_scanned_pixel_spacing);
    let mut scales = Vec::new();
    let mut add = |spacing: [f64; 2], rule, plane, uncertainty, weak| {
        if spacing
            .iter()
            .all(|value| value.is_finite() && *value > 0.0)
        {
            scales.push(result(spacing, rule, plane, uncertainty, weak));
        }
    };
    if let Some(spacing) = ps {
        match tags.pixel_spacing_calibration {
            Some(SpacingCalibration::Fiducial) => add(
                spacing,
                FileScaleRule::Fiducial,
                ScalePlane::FiducialDepth,
                Some(0.01),
                false,
            ),
            Some(SpacingCalibration::Geometry) => add(
                spacing,
                FileScaleRule::Geometry,
                ScalePlane::AssumedDepth,
                Some(0.03),
                false,
            ),
            None => (),
        }
        if [ips, nsps]
            .into_iter()
            .flatten()
            .any(|other| differs(spacing, other))
        {
            add(
                spacing,
                FileScaleRule::CorrectedUnknownMethod,
                ScalePlane::AssumedDepth,
                Some(0.05),
                true,
            );
        }
    }
    if let Some(spacing) = ips {
        if let Some(magnification) = tags
            .estimated_magnification
            .filter(|value| value.is_finite() && *value > 1.0 && *value <= 2.0)
        {
            add(
                spacing.map(|value| value / magnification),
                FileScaleRule::DetectorOverMagnification,
                ScalePlane::TableOrBucky,
                Some(0.10),
                true,
            );
        } else if let (Some(sid), Some(sod)) = (
            tags.distance_source_to_detector,
            tags.distance_source_to_patient,
        ) && sid.is_finite()
            && sod.is_finite()
            && sod > 0.0
            && sod < sid
        {
            add(
                spacing.map(|value| value * (sod / sid)),
                FileScaleRule::DetectorFromDistances,
                ScalePlane::TableOrBucky,
                Some(0.10),
                true,
            );
        }
        add(
            spacing,
            FileScaleRule::DetectorUncorrected,
            ScalePlane::DetectorFace,
            None,
            true,
        );
    }
    if let Some(spacing) = ps
        && tags.imager_pixel_spacing.is_none()
        && tags.nominal_scanned_pixel_spacing.is_none()
        && tags.pixel_spacing_calibration.is_none()
    {
        add(
            spacing,
            FileScaleRule::UnknownCalibration,
            ScalePlane::Unknown,
            Some(0.10),
            true,
        );
    }
    scales
}

pub(super) fn valid_spacing(spacing: Option<[f64; 2]>) -> Option<[f64; 2]> {
    spacing.filter(|pair| pair.iter().all(|value| (0.01..=2.0).contains(value)))
}

/// Pixel shape comes from spacing, even if its absolute depth is unknown.
pub(super) fn aspect(tags: Option<&KeptTags>) -> f64 {
    tags.and_then(|tags| {
        valid_spacing(tags.pixel_spacing)
            .or_else(|| valid_spacing(tags.imager_pixel_spacing))
            .or_else(|| valid_spacing(tags.nominal_scanned_pixel_spacing))
    })
    .map_or(1.0, |[row, col]| col / row)
}

fn differs(a: [f64; 2], b: [f64; 2]) -> bool {
    a.into_iter()
        .zip(b)
        .any(|(a, b)| (a - b).abs() / a.max(b) > 0.001)
}

fn result(
    [row, col]: [f64; 2],
    rule: FileScaleRule,
    plane: ScalePlane,
    relative_uncertainty: Option<f64>,
    weak: bool,
) -> ScaleResult {
    ScaleResult {
        mm_per_px: col,
        mm_per_px_vertical: (row != col).then_some(row),
        source: ScaleSource::File { rule },
        plane,
        relative_uncertainty,
        weak,
    }
}
