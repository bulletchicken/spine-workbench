//! A circle fitted to a calibration object of known size.
//!
//! The largest gradient is assumed to follow an opaque circular rim. The
//! uncertainty describes fit scatter, not image-processing or depth bias.

use darkroom::{blur, grey_image::GreyImage};
use film_reader::decoded_film::DecodedFilm;
use shapes::errors::{EngineError, ErrorCode};
use shapes::film_geometry::{ScalePlane, ScaleResult, ScaleSource};
use shapes::pixels::Px;

use super::{circle_fit, from_ruler::on_film, from_tags::aspect};

/// Fit the rim near a centre-to-edge drag. The diameter is required; no
/// physical size is inferred from anatomy. The scale holds at the ball's depth.
pub fn from_known_object(
    film: &DecodedFilm,
    near: Px,
    edge: Px,
    diameter_mm: f64,
) -> Result<ScaleResult, EngineError> {
    if !(5.0..=100.0).contains(&diameter_mm) {
        return Err(EngineError::new(
            ErrorCode::ScaleBadLength,
            "the object diameter must be between 5 and 100 mm",
        ));
    }
    if !on_film(film, near) || !on_film(film, edge) {
        return Err(EngineError::new(
            ErrorCode::ToolClickOutsideFilm,
            "the object centre and edge must lie on the film",
        ));
    }
    // Work in vertical-pixel units so a physical sphere remains circular
    // when the file declares unequal row and column spacings.
    let ratio = aspect(film.facts.kept_tags.as_ref());
    let r0 = libm::hypot(ratio * (edge.x() - near.x()), edge.y() - near.y());
    if r0 < 3.0 || r0 > (f64::from(film.image.width()) * ratio).min(f64::from(film.image.height()))
    {
        return Err(failed(0, None));
    }
    let left = (near.x() - 1.6 * r0 / ratio - 5.0).floor().max(0.0) as u32;
    let top = (near.y() - 1.6 * r0 - 5.0).floor().max(0.0) as u32;
    let right = (near.x() + 1.6 * r0 / ratio + 5.0)
        .ceil()
        .min(f64::from(film.image.width())) as u32;
    let bottom = (near.y() + 1.6 * r0 + 5.0)
        .ceil()
        .min(f64::from(film.image.height())) as u32;
    let crop = film
        .image
        .crop(left, top, right - left, bottom - top)
        .ok_or_else(|| failed(0, None))?
        .map(|value| f32::from(*value));
    let mut valid =
        crate::rough_trace::exclusions::shutter_mask(&crop, film.facts.kept_tags.as_ref());
    if valid.iter().any(|keep| !keep) {
        // Gaussian sigma 1.5 has a five-pixel kernel radius. Only sample
        // places whose whole blur footprint lies inside every shutter.
        let mask = GreyImage::new(
            crop.width(),
            crop.height(),
            valid
                .iter()
                .map(|keep| if *keep { 1.0 } else { 0.0 })
                .collect(),
            crop.placement(),
        )
        .ok_or_else(|| failed(0, None))?;
        valid = blur::box_blur(&mask, 5)
            .pixels()
            .iter()
            .map(|value| *value == 1.0)
            .collect();
    }
    let blurred = blur::gaussian(&crop, 1.5).ok_or_else(|| failed(0, None))?;
    let range = blurred
        .pixels()
        .iter()
        .zip(&valid)
        .filter_map(|(&value, &keep)| keep.then_some(value))
        .fold((f32::MAX, f32::MIN), |(low, high), value| {
            (low.min(value), high.max(value))
        });
    let contrast = f64::from(range.1 - range.0);
    if contrast <= 0.0 {
        return Err(failed(0, None));
    }
    let points = rim_points(&blurred, &valid, near, r0, ratio, contrast * 0.02);
    let Some((circle, inliers, rms)) = circle_fit::fit(&points) else {
        return Err(failed(0, None));
    };
    if inliers < 90 || rms > 2.0 || (circle.radius - r0).abs() > 0.4 * r0 {
        return Err(failed(inliers, Some(rms)));
    }
    let radius_px = circle.radius / ratio;
    let mm_per_px = diameter_mm / (2.0 * radius_px);
    Ok(ScaleResult {
        mm_per_px,
        mm_per_px_vertical: (ratio != 1.0).then_some(mm_per_px / ratio),
        source: ScaleSource::KnownObject {
            centre: Px::new(circle.x / ratio, circle.y),
            radius_px,
            diameter_mm,
        },
        plane: ScalePlane::ObjectDepth,
        relative_uncertainty: Some(rms / (inliers as f64).sqrt() / circle.radius),
        weak: inliers < 162,
    })
}

fn failed(inliers: usize, rms: Option<f64>) -> EngineError {
    EngineError::new(
        ErrorCode::ScaleFitFailed,
        "the calibration object's rim could not be fitted",
    )
    .with_detail(match rms {
        Some(rms) => format!("inliers={inliers}; rms={rms:.3}"),
        None => format!("inliers={inliers}"),
    })
}

fn rim_points(
    image: &GreyImage<f32>,
    valid: &[bool],
    near: Px,
    r0: f64,
    ratio: f64,
    threshold: f64,
) -> Vec<[f64; 2]> {
    let mut points = Vec::with_capacity(180);
    let samples = ((0.8 * r0 / 0.25).floor() as usize).max(3);
    let origin = image.placement().origin;
    for ray in 0..180 {
        let angle = f64::from(ray) * std::f64::consts::TAU / 180.0;
        let (sin, cos) = libm::sincos(angle);
        let at = |radius: f64| {
            bilinear(
                image,
                valid,
                near.x() - origin.x() + radius * cos / ratio,
                near.y() - origin.y() + radius * sin,
            )
        };
        let mut gradients = Vec::with_capacity(samples);
        for sample in 0..samples {
            let radius = 0.6 * r0 + 0.25 * sample as f64;
            gradients.push(match (at(radius - 0.25), at(radius + 0.25)) {
                (Some(a), Some(b)) => (b - a).abs() / 0.5,
                _ => 0.0,
            });
        }
        let Some((peak, &value)) = gradients
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
        else {
            continue;
        };
        if peak == 0 || peak + 1 == samples || value < threshold {
            continue;
        }
        let (before, after) = (gradients[peak - 1], gradients[peak + 1]);
        let denominator = before - 2.0 * value + after;
        let offset = if denominator < 0.0 {
            (0.5 * (before - after) / denominator).clamp(-0.5, 0.5)
        } else {
            0.0
        };
        let radius = 0.6 * r0 + 0.25 * (peak as f64 + offset);
        points.push([ratio * near.x() + radius * cos, near.y() + radius * sin]);
    }
    points
}

fn bilinear(image: &GreyImage<f32>, valid: &[bool], x: f64, y: f64) -> Option<f64> {
    if x < 0.0 || y < 0.0 || x >= f64::from(image.width() - 1) || y >= f64::from(image.height() - 1)
    {
        return None;
    }
    let (ix, iy) = (x.floor() as u32, y.floor() as u32);
    let base = (iy * image.width() + ix) as usize;
    let row = image.width() as usize;
    if [base, base + 1, base + row, base + row + 1]
        .iter()
        .any(|i| !valid[*i])
    {
        return None;
    }
    let (dx, dy) = (x - f64::from(ix), y - f64::from(iy));
    let (a, b, c, d) = (
        f64::from(*image.get(ix, iy)?),
        f64::from(*image.get(ix + 1, iy)?),
        f64::from(*image.get(ix, iy + 1)?),
        f64::from(*image.get(ix + 1, iy + 1)?),
    );
    Some((1.0 - dy) * ((1.0 - dx) * a + dx * b) + dy * ((1.0 - dx) * c + dx * d))
}
