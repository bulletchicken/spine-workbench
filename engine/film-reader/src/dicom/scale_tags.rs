//! Validate spacing facts in row-then-column order. Choosing a scale and
//! its depth plane belongs to film-geometry, never to the reader.

use super::parse::Dataset;
use shapes::film::{IntakeFlag, KeptTags};

pub(super) fn validate(data: &Dataset<'_>, tags: &mut KeptTags, flags: &mut Vec<IntakeFlag>) {
    for (tag, spacing) in [
        ((0x28, 0x30), &mut tags.pixel_spacing),
        ((0x18, 0x1164), &mut tags.imager_pixel_spacing),
        ((0x18, 0x2010), &mut tags.nominal_scanned_pixel_spacing),
    ] {
        if data.get(tag).is_some()
            && !spacing
                .is_some_and(|values| values.iter().all(|value| (0.01..=2.0).contains(value)))
        {
            *spacing = None;
            flags.push(IntakeFlag::InvalidSpacing);
        }
        if spacing.is_some_and(|[row, column]| (row / column - 1.0).abs() > 0.005) {
            flags.push(IntakeFlag::AnisotropicPixels);
        }
    }
    if tags
        .pixel_aspect_ratio
        .is_some_and(|[row, column]| row != column)
    {
        flags.push(IntakeFlag::AnisotropicPixels);
    }
    if tags
        .patient_orientation
        .as_ref()
        .is_some_and(|directions| directions.iter().any(|direction| direction.len() > 1))
    {
        flags.push(IntakeFlag::ObliqueOrientation);
    }
    if [&tags.manufacturer, &tags.model_name]
        .iter()
        .filter_map(|value| value.as_ref())
        .any(|value| {
            value
                .split(|ch: char| !ch.is_alphanumeric())
                .any(|word| word.eq_ignore_ascii_case("EOS"))
        })
    {
        flags.push(IntakeFlag::SlotGeometry);
    }
}
