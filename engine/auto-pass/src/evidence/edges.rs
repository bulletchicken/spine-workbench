//! The phase-based edge map and the contrast-normalised gradient fallback.

use darkroom::fft::frequency;
use darkroom::grey_image::GreyImage;
use darkroom::log_gabor::{Spectrum, riesz};
use darkroom::radial_symmetry::Voters;
use std::f64::consts::PI;

/// A separable Gaussian transfer function, with its exponentials evaluated
/// once per axis rather than once per frequency and derivative.
pub(super) struct Gaussian {
    across: Vec<f32>,
    down: Vec<f32>,
}

impl Gaussian {
    pub(super) fn new(spectrum: &Spectrum, sigma: f64) -> Self {
        let axis = |n: usize| {
            (0..n)
                .map(|i| {
                    let f = frequency(i, n);
                    libm::exp(-2.0 * PI * PI * sigma * sigma * f * f) as f32
                })
                .collect()
        };
        Self {
            across: axis(spectrum.grid().width()),
            down: axis(spectrum.grid().height()),
        }
    }

    pub(super) fn at(&self, i: usize) -> f32 {
        self.across[i % self.across.len()] * self.down[i / self.across.len()]
    }
}

/// Packed Gaussian derivatives: the Riesz pair times `2π|f|G` is
/// `i2πfxG − 2πfyG`, whose inverse holds gx in real and gy in imaginary.
/// The ideal step normalisation is `2 / (σ sqrt(2π))`.
pub(super) fn voters(
    spectrum: &Spectrum,
    contrast: &GreyImage<f32>,
    valid: &GreyImage<u8>,
    sigma: f64,
) -> Voters {
    let gaussian = Gaussian::new(spectrum, sigma);
    let gradient = spectrum
        .filtered(|i, point| riesz(point) * (2.0 * PI * point.radius()) as f32 * gaussian.at(i));
    let ideal = 2.0 / (sigma * (2.0 * PI).sqrt());
    let strength = gradient
        .real
        .pixels()
        .iter()
        .zip(gradient.imaginary.pixels())
        .zip(contrast.pixels())
        .zip(valid.pixels())
        .map(|(((&x, &y), &c), &v)| {
            if v == 0 || c <= 0.0 {
                0.0
            } else {
                let (x, y) = (f64::from(x), f64::from(y));
                ((x * x + y * y).sqrt() / (f64::from(c) * ideal)) as f32
            }
        })
        .collect();
    Voters {
        x: gradient.real,
        y: gradient.imaginary,
        strength: super::like(contrast, strength),
    }
}

pub(super) fn gradient_maps(
    voters: Voters,
    valid: &GreyImage<u8>,
) -> (GreyImage<f32>, GreyImage<f32>) {
    let edge = voters.strength.map(|v| v.clamp(0.0, 1.0));
    let direction = voters
        .x
        .pixels()
        .iter()
        .zip(voters.y.pixels())
        .zip(valid.pixels())
        .map(|((&x, &y), &v)| {
            if v == 0 {
                0.0
            } else {
                let angle = libm::atan2(f64::from(y), f64::from(x));
                (if angle <= -PI { PI } else { angle }) as f32
            }
        })
        .collect();
    let direction = super::like(&edge, direction);
    (edge, direction)
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkroom::grey_image::Placement;

    #[test]
    fn an_isolated_step_has_unit_strength_and_points_into_the_bar() {
        // A 32 px wide unit bar has well-separated edges at sigma 2.
        // Local contrast 1/2 makes either ideal step score one. Half-pixel
        // sampling of the Gaussian peak costs under 4%, hence 0.95..1.01.
        let image = GreyImage::new(
            96,
            64,
            (0..96 * 64)
                .map(|i| {
                    if (32..64).contains(&(i % 96)) {
                        1.0
                    } else {
                        0.0
                    }
                })
                .collect(),
            Placement::FILM,
        )
        .unwrap();
        let valid = image.map(|_| 1);
        let contrast = image.map(|_| 0.5);
        let spectrum = Spectrum::of(&image).unwrap();
        let (edge, direction) = gradient_maps(voters(&spectrum, &contrast, &valid, 2.0), &valid);
        assert!((0.95..=1.01).contains(edge.get(32, 32).unwrap()));
        assert!((0.95..=1.01).contains(edge.get(63, 32).unwrap()));
        assert!(direction.get(32, 32).unwrap().abs() < 1e-4);
        assert!((direction.get(63, 32).unwrap().abs() - std::f32::consts::PI).abs() < 1e-4);
    }
}
