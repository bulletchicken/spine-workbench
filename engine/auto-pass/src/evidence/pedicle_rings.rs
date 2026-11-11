//! The pedicle ring map on front and bending films.

use super::{edges, prepare};
use crate::handoffs::RingMaps;
use darkroom::grey_image::GreyImage;
use darkroom::log_gabor::Spectrum;
use darkroom::radial_symmetry::{RingSettings, RingShape, ideal_ring, shape_response};
use shapes::errors::{EngineError, ErrorCode};

pub(super) fn sizes(assumed: bool) -> Vec<f64> {
    if assumed {
        vec![2.5 / 1.6, 2.5, 3.5, 5.0, 7.0, 7.0 * 1.6]
    } else {
        vec![2.5, 3.5, 5.0, 7.0]
    }
}

pub(super) fn compute(
    spectrum: &Spectrum,
    contrast: &GreyImage<f32>,
    valid: &GreyImage<u8>,
    spacing: f64,
    semi_axes_mm: Vec<f64>,
    quarter_turns: u8,
    mut checkpoint: impl FnMut(usize, &GreyImage<f32>),
) -> Result<RingMaps, EngineError> {
    let settings = RingSettings::evidence();
    let aspects = vec![0.8, 1.25, 2.0];
    let voters = edges::voters(spectrum, contrast, valid, (0.6 / spacing).max(1.0));
    // Compare the calibrated responses before clipping. Two strong shapes
    // can both exceed one; the larger must still win the shape index.
    let mut best = vec![0.0f32; valid.pixels().len()];
    let mut winning_shape = vec![0u8; best.len()];
    for (size, &a_mm) in semi_axes_mm.iter().enumerate() {
        for (aspect, &ratio) in aspects.iter().enumerate() {
            let a = a_mm / spacing;
            let b = a * ratio;
            let rim = (0.3 * a_mm).clamp(1.0, 2.0) / spacing;
            if a - rim < 1.5 || b - rim < 1.5 {
                continue;
            }
            let (a, b) = if quarter_turns.is_multiple_of(2) {
                (a, b)
            } else {
                (b, a)
            };
            let shape = RingShape::new(a, b, rim).ok_or_else(failed)?;
            let ideal = ideal_ring(shape, 0.5 / spacing).ok_or_else(failed)?;
            let ideal_valid = ideal.map(|_| 1u8);
            let ideal_contrast = prepare::local_contrast(&ideal, &ideal_valid, spacing);
            let ideal_spectrum = Spectrum::of(&ideal).ok_or_else(failed)?;
            let ideal_voters = edges::voters(
                &ideal_spectrum,
                &ideal_contrast,
                &ideal_valid,
                (0.6 / spacing).max(1.0),
            );
            let reference = shape_response(&ideal_voters, shape, &settings).ok_or_else(failed)?;
            let peak = reference.pixels().iter().copied().fold(0.0f32, f32::max);
            let response = shape_response(&voters, shape, &settings).ok_or_else(failed)?;
            if !peak.is_finite() || peak <= 0.0 {
                return Err(failed());
            }
            for (i, &value) in response.pixels().iter().enumerate() {
                let calibrated = value / peak;
                if valid.pixels()[i] != 0 && calibrated > best[i] {
                    best[i] = calibrated;
                    winning_shape[i] = (size * aspects.len() + aspect) as u8;
                }
            }
        }
        checkpoint(
            size,
            &super::like(contrast, best.iter().map(|v| v.min(1.0)).collect()),
        );
    }
    let shape = super::like(contrast, winning_shape);
    Ok(RingMaps {
        response: super::like(contrast, best.into_iter().map(|v| v.min(1.0)).collect()),
        shape,
        semi_axes_mm,
        aspects,
    })
}

fn failed() -> EngineError {
    EngineError::new(
        ErrorCode::PassNoSpineFound,
        "The pedicle ring filters could not be prepared",
    )
}
