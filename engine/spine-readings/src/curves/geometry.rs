//! Endplate geometry and film registration. Film pixels are turned upright
//! before physical spacings are applied; film facing never changes an angle.

mod films;

pub use films::{bending_angle, bending_cobb, bending_disc, csvl, upright};

use super::ReadReason;
use super::{Curve, Reading, SpineReading, VertebraReading};
use crate::ReadingsError;
use forward_model::endplates::Endplates;
use forward_model::vertebra_params::VertebraParams;
use shapes::levels::Level;
use shapes::made_up::CurveRegion;
use shapes::spine_fit::SpineFit;
use shapes::spine_readings::{BodySide, ReadingsInput};

/// This project's arithmetic zero, in degrees, only to suppress round-off
/// in otherwise identical normals. It is not a minimum minor-curve size.
const ANGLE_ROUND_OFF_DEG: f64 = 1e-10;

/// Read one fitted or perturbed spine using the input's films. The input's
/// nominal fit fixes film registration, so a version cannot move the film.
/// No missing film is replaced by typical geometry in a classification.
/// The evidence floor applies only to the contract’s `curves`; this Rust
/// API retains every nonzero curve for level rules, regardless of its size.
pub fn measure(input: &ReadingsInput, fit: &SpineFit) -> Result<SpineReading, ReadingsError> {
    validate_fit(fit)?;
    let plates: Vec<_> = fit
        .vertebrae
        .iter()
        .map(|v| Endplates::of(&VertebraParams::from(&v.pose)))
        .collect();
    let csvl_y_mm = csvl(input, fit);
    let vertebrae: Vec<_> = fit
        .vertebrae
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let disc_below_deg = plates
                .get(i + 1)
                .map(|_| lower_coronal_tilt_deg(v) - upper_coronal_tilt_deg(&fit.vertebrae[i + 1]))
                .ok_or(ReadReason::NotFitted);
            VertebraReading {
                level: v.level,
                centre_mm: v.pose.centre,
                upper_tilt_deg: upper_coronal_tilt_deg(v),
                lower_tilt_deg: lower_coronal_tilt_deg(v),
                twist_deg: if v.inferred.twist {
                    Err(ReadReason::TwistNotMeasured)
                } else {
                    Ok(v.angles.twist_rad.to_degrees())
                },
                disc_below_deg,
                centre_line_offset_mm: csvl_y_mm.map(|line| v.pose.centre[1] - line),
            }
        })
        .collect();
    let turns = turning_points(
        &vertebrae
            .iter()
            .map(|v| v.upper_tilt_deg)
            .collect::<Vec<_>>(),
    );
    let mut curves = Vec::new();
    for (position, ends) in turns.windows(2).enumerate() {
        let rising = vertebrae[ends[1]].upper_tilt_deg > vertebrae[ends[0]].upper_tilt_deg;
        // A turning vertebra's upper and lower plates need not attain their
        // extremes at the same level. Search each plate in its own turning
        // neighbourhood, bounded halfway to the neighbouring turns.
        let upper = plate_extreme(&vertebrae, &turns, position, true, !rising);
        let lower = plate_extreme(&vertebrae, &turns, position + 1, false, rising);
        let signed = vertebrae[lower].lower_tilt_deg - vertebrae[upper].upper_tilt_deg;
        if signed.abs() <= ANGLE_ROUND_OFF_DEG {
            continue;
        }
        let (apex, deviation) = apex(fit, upper, lower);
        let bends = bending_cobb(
            input,
            fit.vertebrae[upper].level,
            fit.vertebrae[lower].level,
        );
        let bending_cobb_deg = bends.map(|b| b.0);
        let straightening_bend = bends.map(|b| b.1);
        let bending_disc_below_deg = straightening_bend.and_then(|kind| {
            let next = fit.vertebrae.get(lower + 1).ok_or(ReadReason::NotFitted)?;
            let film = if kind == shapes::film::FilmKind::BendLeft {
                input.bend_left.as_ref()
            } else {
                input.bend_right.as_ref()
            }
            .ok_or(ReadReason::NoBendingFilms)?;
            bending_disc(film, fit.vertebrae[lower].level, next.level)
        });
        curves.push(Curve {
            upper_index: upper,
            lower_index: lower,
            apex_index: apex,
            region: region(fit.vertebrae[apex].level),
            convex: if signed > 0.0 {
                BodySide::Right
            } else {
                BodySide::Left
            },
            cobb_deg: signed.abs(),
            signed_cobb_deg: signed,
            apical_deviation_mm: deviation,
            bending_cobb_deg,
            straightening_bend,
            bending_disc_below_deg,
        });
    }
    let major = curves
        .iter()
        .enumerate()
        .fold(None, |most: Option<usize>, (i, c)| {
            if most.is_none_or(|j| c.cobb_deg > curves[j].cobb_deg) {
                Some(i)
            } else {
                most
            }
        });
    let reading = SpineReading {
        vertebrae,
        curves,
        major,
        csvl_y_mm,
        kyphosis_t2_t5_deg: regional_kyphosis(fit, Level::T2, Level::T5),
        kyphosis_t5_t12_deg: regional_kyphosis(fit, Level::T5, Level::T12),
        kyphosis_t10_l2_deg: regional_kyphosis(fit, Level::T10, Level::L2),
        l4_tilt_deg: fit
            .vertebrae
            .iter()
            .position(|v| v.level == Level::L4)
            .map(|i| upper_coronal_tilt_deg(&fit.vertebrae[i]))
            .ok_or(ReadReason::NotFitted),
        minor_curve_minimum_deg: None,
    };
    Ok(reading)
}

/// The front film fixes the centre tilt and front wedge independently of
/// typical lean, side wedge and twist. Splitting that observed wedge between
/// the two plates prevents typical side geometry from becoming a front reading.
pub fn upper_coronal_tilt_deg(v: &shapes::spine_fit::FittedVertebra) -> f64 {
    v.corner_tilt_rad.map_or_else(
        || frontal_tilt_deg(&Endplates::of(&VertebraParams::from(&v.pose)).top.normal),
        |tilt| (tilt + 0.5 * v.pose.wedge_front).to_degrees(),
    )
}

/// Lower endplate paired with [`upper_coronal_tilt_deg`].
pub fn lower_coronal_tilt_deg(v: &shapes::spine_fit::FittedVertebra) -> f64 {
    v.corner_tilt_rad.map_or_else(
        || frontal_tilt_deg(&Endplates::of(&VertebraParams::from(&v.pose)).bottom.normal),
        |tilt| (tilt - 0.5 * v.pose.wedge_front).to_degrees(),
    )
}

/// Coronal normal angle in degrees: positive lowers the patient's right.
pub fn frontal_tilt_deg(normal: &nalgebra::Vector3<f64>) -> f64 {
    libm::atan2(-normal.y, normal.z).to_degrees()
}

/// Sagittal normal angle in degrees: positive lowers the anterior edge.
pub fn sagittal_tilt_deg(normal: &nalgebra::Vector3<f64>) -> f64 {
    libm::atan2(normal.x, normal.z).to_degrees()
}

/// Signed upper-top minus lower-bottom sagittal angle, positive kyphosis.
/// Both endpoint vertebrae must have measured side-film geometry.
pub fn regional_kyphosis(fit: &SpineFit, upper: Level, lower: Level) -> Reading<f64> {
    if fit.side_film.is_none()
        || fit
            .vertebrae
            .iter()
            .all(|v| v.inferred.depth || v.rms_mm.side.is_none())
    {
        return Err(ReadReason::NoSideFilm);
    }
    let a = fit
        .vertebrae
        .iter()
        .find(|v| v.level == upper)
        .ok_or(ReadReason::NotFitted)?;
    let b = fit
        .vertebrae
        .iter()
        .find(|v| v.level == lower)
        .ok_or(ReadReason::NotFitted)?;
    if [a, b]
        .iter()
        .any(|v| v.inferred.depth || v.rms_mm.side.is_none())
    {
        return Err(ReadReason::NoSideFilm);
    }
    let a = Endplates::of(&VertebraParams::from(&a.pose)).top;
    let b = Endplates::of(&VertebraParams::from(&b.pose)).bottom;
    Ok(sagittal_tilt_deg(&a.normal) - sagittal_tilt_deg(&b.normal))
}

fn region(level: Level) -> Option<CurveRegion> {
    match level {
        l if (Level::T3..=Level::T5).contains(&l) => Some(CurveRegion::ProximalThoracic),
        l if (Level::T6..=Level::T11).contains(&l) => Some(CurveRegion::MainThoracic),
        l if (Level::T12..=Level::L4).contains(&l) => Some(CurveRegion::ThoracolumbarLumbar),
        _ => None,
    }
}

// Collapse equal-tilt plateaux onto their first level. The endpoints and
// each strict change of slope are alternating extremes of the fitted tilts.
fn turning_points(tilts: &[f64]) -> Vec<usize> {
    let mut distinct = Vec::new();
    for (i, &tilt) in tilts.iter().enumerate() {
        if distinct
            .last()
            .is_none_or(|&j: &usize| (tilt - tilts[j]).abs() > ANGLE_ROUND_OFF_DEG)
        {
            distinct.push(i);
        }
    }
    if distinct.len() < 2 {
        return Vec::new();
    }
    let mut result = vec![distinct[0]];
    for trio in distinct.windows(3) {
        let a = tilts[trio[1]] - tilts[trio[0]];
        let b = tilts[trio[2]] - tilts[trio[1]];
        if a * b < 0.0 {
            result.push(trio[1]);
        }
    }
    result.push(*distinct.last().expect("two distinct tilts"));
    result
}

// Midpoints between successive upper-tilt turns partition the chain.
// Within that neighbourhood the plate used for the Cobb determines the end.
fn plate_extreme(
    vertebrae: &[VertebraReading],
    turns: &[usize],
    position: usize,
    upper_plate: bool,
    maximum: bool,
) -> usize {
    let start = if position == 0 {
        0
    } else {
        (turns[position - 1] + turns[position]).div_ceil(2)
    };
    let stop = turns
        .get(position + 1)
        .map_or(vertebrae.len() - 1, |next| (turns[position] + next) / 2);
    let tilt = |i: usize| {
        if upper_plate {
            vertebrae[i].upper_tilt_deg
        } else {
            vertebrae[i].lower_tilt_deg
        }
    };
    let mut result = start;
    for i in start + 1..=stop {
        if (maximum && tilt(i) > tilt(result)) || (!maximum && tilt(i) < tilt(result)) {
            result = i;
        }
    }
    result
}

fn apex(fit: &SpineFit, upper: usize, lower: usize) -> (usize, f64) {
    let a = fit.vertebrae[upper].pose.centre;
    let b = fit.vertebrae[lower].pose.centre;
    let mut index = upper;
    let mut deviation = 0.0_f64;
    for i in upper + 1..lower {
        let c = fit.vertebrae[i].pose.centre;
        let fraction = if a[2] != b[2] {
            (c[2] - a[2]) / (b[2] - a[2])
        } else {
            (i - upper) as f64 / (lower - upper) as f64
        };
        let offset = c[1] - (a[1] + fraction * (b[1] - a[1]));
        if offset.abs() > deviation.abs() {
            index = i;
            deviation = offset;
        }
    }
    (index, deviation)
}

fn validate_fit(fit: &SpineFit) -> Result<(), ReadingsError> {
    if fit.vertebrae.is_empty() {
        return Err(ReadingsError::NoSpine);
    }
    let mut previous = None;
    for v in &fit.vertebrae {
        let p = VertebraParams::from(&v.pose);
        let invalid = p.to_numbers().iter().any(|n| !n.is_finite())
            || p.height <= 0.0
            || [
                p.top_half_width,
                p.top_half_depth,
                p.bottom_half_width,
                p.bottom_half_depth,
            ]
            .iter()
            .any(|n| *n <= 0.0)
            || p.wedge_front.abs() >= core::f64::consts::PI
            || p.wedge_side.abs() >= core::f64::consts::PI
            || !v.angles.twist_rad.is_finite()
            || previous.is_some_and(|level| level >= v.level)
            || v.level != v.pose.level;
        if invalid {
            return Err(ReadingsError::InvalidValue {
                field: "fit.vertebrae".into(),
                reason: "invalid-pose-or-order".into(),
            });
        }
        previous = Some(v.level);
    }
    Ok(())
}
