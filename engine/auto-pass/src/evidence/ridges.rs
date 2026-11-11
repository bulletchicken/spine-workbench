//! The ridge map and the disc-gap valley map, over several scales.

use super::edges::Gaussian;
use darkroom::grey_image::GreyImage;
use darkroom::hessian::line_measure;
use darkroom::log_gabor::Spectrum;
use std::f64::consts::PI;

pub(super) struct Ridges {
    pub ridge: GreyImage<f32>,
    pub ridge_orientation: GreyImage<f32>,
    pub valley: GreyImage<f32>,
    pub valley_orientation: GreyImage<f32>,
    pub valley_scale: GreyImage<u8>,
}

/// Keep maxima in place: the response is the largest Frangi measure, but
/// the orientation and scale follow the largest sign-gated eigenvalue.
struct Best {
    strength: Vec<f32>,
    eigenvalue: Vec<f32>,
    normal: Vec<f32>,
    scale: Vec<u8>,
}

impl Best {
    fn new(len: usize, keep_scale: bool) -> Self {
        Self {
            strength: vec![0.0; len],
            eigenvalue: vec![0.0; len],
            normal: vec![0.0; len],
            scale: vec![0; if keep_scale { len } else { 0 }],
        }
    }

    fn include(&mut self, i: usize, strength: f64, eigenvalue: f64, normal: f64, scale: usize) {
        self.strength[i] = self.strength[i].max(strength as f32);
        if eigenvalue.abs() > f64::from(self.eigenvalue[i]) {
            self.eigenvalue[i] = eigenvalue.abs() as f32;
            self.normal[i] = normal as f32;
            if let Some(value) = self.scale.get_mut(i) {
                *value = scale as u8;
            }
        }
    }
}

pub(super) fn compute(
    spectrum: &Spectrum,
    contrast: &GreyImage<f32>,
    valid: &GreyImage<u8>,
    spacing: f64,
    scales: &[f64],
    mut checkpoint: impl FnMut(usize, &GreyImage<f32>, &GreyImage<f32>),
) -> Ridges {
    let mut ridge = Best::new(contrast.pixels().len(), false);
    let mut valley = Best::new(contrast.pixels().len(), true);
    // Lxy at two scales shares an inverse transform. Lxx and Lyy share
    // another at each scale: five scales take eight inverses.
    for pair in (0..scales.len()).step_by(2) {
        let sigma_a = (scales[pair] / spacing).max(1.0);
        let sigma_b = scales.get(pair + 1).map(|mm| (mm / spacing).max(1.0));
        let ga = Gaussian::new(spectrum, sigma_a);
        let gb = sigma_b.map(|sigma| Gaussian::new(spectrum, sigma));
        let cross = spectrum.filtered_pair(
            |i, f| {
                let gain = if f.on_nyquist_line {
                    0.0
                } else {
                    -4.0 * PI * PI * f.across * f.down
                };
                ((gain * sigma_a * sigma_a) as f32 * ga.at(i)).into()
            },
            |i, f| {
                let gain = if f.on_nyquist_line {
                    0.0
                } else {
                    -4.0 * PI * PI * f.across * f.down
                };
                let value = gb
                    .as_ref()
                    .zip(sigma_b)
                    .map_or(0.0, |(g, s)| (gain * s * s) as f32 * g.at(i));
                value.into()
            },
        );
        for (index, &scale_mm) in scales.iter().enumerate().skip(pair).take(2) {
            let (gaussian, sigma, xy) = if index == pair {
                (&ga, sigma_a, &cross.real)
            } else {
                (
                    gb.as_ref().expect("second scale"),
                    sigma_b.expect("second scale"),
                    &cross.imaginary,
                )
            };
            let diagonal = spectrum.filtered_pair(
                |i, f| {
                    ((-4.0 * PI * PI * f.across * f.across * sigma * sigma) as f32 * gaussian.at(i))
                        .into()
                },
                |i, f| {
                    ((-4.0 * PI * PI * f.down * f.down * sigma * sigma) as f32 * gaussian.at(i))
                        .into()
                },
            );
            let mut bright = vec![0.0; contrast.pixels().len()];
            let mut dark = vec![0.0; bright.len()];
            for i in 0..bright.len() {
                if valid.pixels()[i] == 0 {
                    continue;
                }
                let c = f64::from(contrast.pixels()[i]).max(1e-6);
                let measure = line_measure(
                    f64::from(diagonal.real.pixels()[i]) / c,
                    f64::from(xy.pixels()[i]) / c,
                    f64::from(diagonal.imaginary.pixels()[i]) / c,
                );
                if measure.lambda2 < 0.0 {
                    bright[i] = measure.strength as f32;
                    if scale_mm <= 2.0 {
                        ridge.include(i, measure.strength, measure.lambda2, measure.normal, index);
                    }
                } else {
                    dark[i] = measure.strength as f32;
                    valley.include(i, measure.strength, measure.lambda2, measure.normal, index);
                }
            }
            checkpoint(
                index,
                &super::like(contrast, bright),
                &super::like(contrast, dark),
            );
        }
    }
    Ridges {
        ridge: super::like(contrast, ridge.strength),
        ridge_orientation: super::like(contrast, ridge.normal),
        valley: super::like(contrast, valley.strength),
        valley_orientation: super::like(contrast, valley.normal),
        valley_scale: super::like(contrast, valley.scale),
    }
}
