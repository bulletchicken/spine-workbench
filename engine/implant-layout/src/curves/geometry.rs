//! Placement-facing adapter to the shared spine measurements.

use super::lumbar::PedicleOutlines;
use super::{Classification, Curve, Reading, SpineReading, VertebraReading};
use crate::LayoutError;
use shapes::implant_layout::{CannotTellCode, LayoutFilm, LayoutInput};
use shapes::levels::Level;
use shapes::pixels::Px;
use shapes::spine_fit::{FilmToFit, MarkSpread, SpineFit};
use shapes::spine_readings::ReadingsInput;
use spine_readings::curves::{self as shared, ReadReason};

pub use shared::geometry::{frontal_tilt_deg, sagittal_tilt_deg};

/// Adapt the layout's films without inventing observations or film noise.
/// These geometric readers use only marks, geometry and size.
pub(crate) fn measurement_input(input: &LayoutInput) -> ReadingsInput {
    let front = input.front.as_ref().map(|value| {
        let mut front = film(value);
        // The fit's provisional spacing registers marks to the same
        // typical drawing. Dimensional rules still check FitLengths.
        if front.geometry.mm_per_px.is_none()
            && input.fit.lengths == shapes::spine_fit::FitLengths::TypicalSize
        {
            front.geometry.mm_per_px = Some(input.fit.front_film.mm_per_px[0]);
            front.geometry.mm_per_px_vertical = Some(input.fit.front_film.mm_per_px[1]);
        }
        front
    });
    ReadingsInput {
        fit: input.fit.clone(),
        front,
        side: None,
        bend_left: input.bend_left.as_ref().map(film),
        bend_right: input.bend_right.as_ref().map(film),
        versions: input.versions,
        seed: input.seed,
    }
}
fn film(value: &LayoutFilm) -> FilmToFit {
    FilmToFit {
        marks: value.marks.clone(),
        geometry: value.geometry.clone(),
        size: value.size,
        mark_spread: MarkSpread::Millimetres {
            sd_mm: 0.0,
            shared_sd_mm: 0.0,
        },
        facing_assumed: false,
    }
}
fn reason(value: ReadReason) -> CannotTellCode {
    match value {
        ReadReason::NoSacrum => CannotTellCode::NoSacrum,
        ReadReason::NoSideFilm => CannotTellCode::NoSideFilm,
        ReadReason::NoBendingFilms => CannotTellCode::NoBendingFilms,
        ReadReason::TwistNotMeasured => CannotTellCode::TwistNotMeasured,
        ReadReason::NotFitted => CannotTellCode::NoLevelMeetsTheRule,
    }
}
fn reading<T>(value: shared::Reading<T>) -> Reading<T> {
    value.map_err(reason)
}

/// Measure geometry once through `spine-readings`, then apply placement classification.
pub fn measure(input: &LayoutInput, fit: &SpineFit) -> Result<SpineReading, LayoutError> {
    measure_with_pedicle_outlines(input, fit, &[])
}
/// Classification may additionally consume explicit pedicle silhouettes.
pub fn measure_with_pedicle_outlines(
    input: &LayoutInput,
    fit: &SpineFit,
    outlines: &[PedicleOutlines],
) -> Result<SpineReading, LayoutError> {
    let value = shared::measure(&measurement_input(input), fit)?;
    let mut result = SpineReading {
        vertebrae: value
            .vertebrae
            .into_iter()
            .map(|v| VertebraReading {
                level: v.level,
                centre_mm: v.centre_mm,
                upper_tilt_deg: v.upper_tilt_deg,
                lower_tilt_deg: v.lower_tilt_deg,
                twist_deg: reading(v.twist_deg),
                disc_below_deg: reading(v.disc_below_deg),
                centre_line_offset_mm: reading(v.centre_line_offset_mm),
            })
            .collect(),
        curves: value
            .curves
            .into_iter()
            .map(|c| Curve {
                upper_index: c.upper_index,
                lower_index: c.lower_index,
                apex_index: c.apex_index,
                region: c.region,
                convex: c.convex,
                cobb_deg: c.cobb_deg,
                signed_cobb_deg: c.signed_cobb_deg,
                apical_deviation_mm: c.apical_deviation_mm,
                bending_cobb_deg: reading(c.bending_cobb_deg),
                straightening_bend: reading(c.straightening_bend),
                bending_disc_below_deg: reading(c.bending_disc_below_deg),
                structural: Err(CannotTellCode::NoCurve),
            })
            .collect(),
        major: value.major,
        csvl_y_mm: reading(value.csvl_y_mm),
        kyphosis_t2_t5_deg: reading(value.kyphosis_t2_t5_deg),
        kyphosis_t5_t12_deg: reading(value.kyphosis_t5_t12_deg),
        kyphosis_t10_l2_deg: reading(value.kyphosis_t10_l2_deg),
        l4_tilt_deg: reading(value.l4_tilt_deg),
        classification: Classification {
            curve_type: Err(CannotTellCode::NoCurve),
            lumbar_modifier: Err(CannotTellCode::NoCurve),
            sagittal_modifier: Err(CannotTellCode::NoSideFilm),
        },
        minor_curve_minimum_deg: value.minor_curve_minimum_deg,
        sources: Vec::new(),
    };
    super::classification::classify_with_pedicle_outlines(&mut result, fit, outlines)?;
    Ok(result)
}

pub fn upright(value: &LayoutFilm, point: Px) -> [f64; 2] {
    shared::geometry::upright(&film(value), point)
}
pub fn csvl(input: &LayoutInput, fit: &SpineFit) -> Reading<f64> {
    reading(shared::geometry::csvl(&measurement_input(input), fit))
}
pub fn regional_kyphosis(fit: &SpineFit, upper: Level, lower: Level) -> Reading<f64> {
    reading(shared::geometry::regional_kyphosis(fit, upper, lower))
}
pub fn bending_angle(value: &LayoutFilm, upper: Level, lower: Level) -> Reading<f64> {
    reading(shared::geometry::bending_angle(&film(value), upper, lower))
}
pub fn bending_cobb(
    input: &LayoutInput,
    upper: Level,
    lower: Level,
) -> Reading<(f64, shapes::film::FilmKind)> {
    reading(shared::geometry::bending_cobb(
        &measurement_input(input),
        upper,
        lower,
    ))
}
pub fn bending_disc(value: &LayoutFilm, upper: Level, lower: Level) -> Reading<f64> {
    reading(shared::geometry::bending_disc(&film(value), upper, lower))
}
