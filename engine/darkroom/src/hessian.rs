//! Second derivatives and the measures built on them: bright ridges and dark
//! valleys, their strength and the direction across them.
//!
//! The derivatives are those of the image smoothed by a Gaussian, taken with
//! the separable sampled Gaussian-derivative kernels of the centre-line
//! spec (C1): radius `r = ceil(3σ)`, the smoothing kernel `g(j)` normalised
//! to sum 1, the first derivative `j · g(j) / σ²` and the second derivative
//! `(j² − m₂) · g(j) / σ⁴`, where `m₂ = Σ j² g(j)`. The second kernel is the
//! sampled `G''`, `(j² − σ²) g(j) / σ⁴`, with the multiple of `g` that makes
//! it sum to zero taken off, so a flat image has no curvature. The constants
//! downstream (a top-hat bar of width `d` peaks at `σ²|L''| = 2φ(1) ≈
//! 0.484` at `σ = d/2`, and the edge-suppression ratio) are derived for the
//! continuous Gaussian. These kernels follow it but stop at 3σ, which
//! leaves `m₂` about 2.7% short of σ², so their responses come out low by
//! an amount that depends on the shape and grows with its width toward the
//! limit of a continuous Gaussian cut at 3σ. Measured with these kernels, a
//! top-hat bar at its best sigma peaks 0.5% high at 5 px wide (pixel
//! sampling), then about 1% low at 9 px, 1.9% at 15, 2.7% at 31, 3.1% at
//! 61 and 3.3% at 121, toward a limit of 3.5% low (a peak of 0.467). A
//! Gaussian ridge of width `s` read at σ = s comes out 0.6% low at σ = 1.5,
//! 3.1% at 3, 4.0% at 6 and 4.5% at 12, toward 5.0%. A stage that
//! calibrates against 0.484 allows for this.
//!
//! The measures follow Frangi (1998) with the constants of the evidence-map
//! spec: a 2 × 2 eigen-analysis per pixel, `Rb = λ1 / λ2`,
//! `S² = λ1² + λ2²`, and
//! `V = exp(−Rb² / (2β²)) · (1 − exp(−S² / (2c²)))` with β = 0.5 and c = 0.5.
//! A bright ridge has `λ2 < 0`, a dark valley `λ2 > 0`. The derivatives
//! are scale-normalised with γ = 1: second derivatives are multiplied by
//! `σ²` and first derivatives by `σ`, so one scale's numbers are comparable
//! to another's.

use crate::blur::{filter_separable, sampled_gaussian};
use crate::gradients::Gradients;
use crate::grey_image::GreyImage;
use core::f64::consts::PI;

/// Frangi's β: how strongly a blob (equal eigenvalues) is penalised.
const BETA: f64 = 0.5;
/// Frangi's c, here a fixed constant because the input is already normalised.
const C: f64 = 0.5;

/// The three separable kernels of one scale, sampled at the whole offsets
/// `−r ..= r` with `r = ceil(3σ)`. Each is a correlation kernel for
/// [`filter_separable`], so a value rising toward +x has a positive first
/// derivative.
#[derive(Clone, Debug, PartialEq)]
pub struct DerivativeKernels {
    /// `g(j) = exp(−j² / 2σ²)`, normalised to sum 1.
    pub smooth: Vec<f64>,
    /// The sampled first derivative, `j · g(j) / σ²`.
    pub first: Vec<f64>,
    /// The sampled second derivative made zero-sum, `(j² − m₂) · g(j) / σ⁴`
    /// with `m₂ = Σ j² g(j)`.
    pub second: Vec<f64>,
}

impl DerivativeKernels {
    /// The kernels at sigma `sigma` px. `None` when sigma is not a positive
    /// number, is so small (under about 1.3e-81) that σ⁴ is zero, where
    /// the second kernel would divide by zero, or is so large (over about
    /// 5461 px) that the radius would pass
    /// [`crate::blur::MAX_KERNEL_RADIUS`].
    pub fn at_scale(sigma: f64) -> Option<DerivativeKernels> {
        if !sigma.is_finite() || sigma <= 0.0 {
            return None;
        }
        let sigma_sq = sigma * sigma;
        if sigma_sq * sigma_sq == 0.0 {
            return None;
        }
        let smooth = sampled_gaussian(sigma)?;
        let radius = (smooth.len() as i64 - 1) / 2;
        let offsets = (-radius..=radius).map(|j| j as f64);
        let second_moment: f64 = offsets.clone().zip(&smooth).map(|(j, g)| j * j * g).sum();
        let first = offsets
            .clone()
            .zip(&smooth)
            .map(|(j, g)| j * g / sigma_sq)
            .collect();
        let second = offsets
            .zip(&smooth)
            .map(|(j, g)| (j * j - second_moment) * g / (sigma_sq * sigma_sq))
            .collect();
        Some(DerivativeKernels {
            smooth,
            first,
            second,
        })
    }
}

/// The second derivatives of an image at one scale, each multiplied by σ².
#[derive(Clone, Debug, PartialEq)]
pub struct Hessian {
    /// σ² · ∂²I/∂x².
    pub xx: GreyImage<f32>,
    /// σ² · ∂²I/∂x∂y.
    pub xy: GreyImage<f32>,
    /// σ² · ∂²I/∂y².
    pub yy: GreyImage<f32>,
}

/// Which way a line is bent, by the sign of its weaker-curvature eigenvalue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Polarity {
    /// A bright line: `λ2 < 0`.
    Bright,
    /// A dark line: `λ2 > 0`.
    Dark,
}

/// The line measure of one pixel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineMeasure {
    /// Frangi's V, from 0 to 1.
    pub strength: f64,
    /// The eigenvalue of larger magnitude, which carries the sign.
    pub lambda2: f64,
    /// The direction across the line, in [−π/2, π/2), measured from +x.
    pub normal: f64,
}

/// The line measures of an image at one scale, on its placement.
#[derive(Clone, Debug, PartialEq)]
pub struct LineMeasures {
    pub strength: GreyImage<f32>,
    pub lambda2: GreyImage<f32>,
    pub normal: GreyImage<f32>,
}

/// The strongest response over several scales for one polarity.
#[derive(Clone, Debug, PartialEq)]
pub struct ScaleCombination {
    /// The largest strength among the scales with the right sign.
    pub strength: GreyImage<f32>,
    /// The index of the scale with the largest `|λ2|` among those scales.
    pub scale_index: GreyImage<u8>,
    /// The normal of that winning scale.
    pub normal: GreyImage<f32>,
}

impl Hessian {
    /// The second derivatives of `image` smoothed at sigma `sigma` px. `None`
    /// when sigma is not a positive number or is too small or too large to
    /// make kernels (see [`DerivativeKernels::at_scale`]).
    pub fn at_scale(image: &GreyImage<f32>, sigma: f64) -> Option<Hessian> {
        let kernels = DerivativeKernels::at_scale(sigma)?;
        let DerivativeKernels {
            smooth,
            first,
            second,
        } = &kernels;
        let scaled = |grid| scale_by(grid, sigma * sigma);
        Some(Hessian {
            xx: scaled(filter_separable(image, second, smooth)?),
            xy: scaled(filter_separable(image, first, first)?),
            yy: scaled(filter_separable(image, smooth, second)?),
        })
    }

    /// The line measure at every pixel.
    pub fn measures(&self) -> LineMeasures {
        let mut strength = Vec::with_capacity(self.xx.pixels().len());
        let mut lambda2 = Vec::with_capacity(self.xx.pixels().len());
        let mut normal = Vec::with_capacity(self.xx.pixels().len());
        for ((a, b), c) in self
            .xx
            .pixels()
            .iter()
            .zip(self.xy.pixels())
            .zip(self.yy.pixels())
        {
            let measure = line_measure(f64::from(*a), f64::from(*b), f64::from(*c));
            strength.push(measure.strength as f32);
            lambda2.push(measure.lambda2 as f32);
            normal.push(measure.normal as f32);
        }
        LineMeasures {
            strength: like(&self.xx, strength),
            lambda2: like(&self.xx, lambda2),
            normal: like(&self.xx, normal),
        }
    }
}

/// The first derivatives of `image` smoothed at sigma `sigma` px, each
/// multiplied by σ: `σ·∂L/∂x` in `x` and `σ·∂L/∂y` in `y`, from the same
/// kernels as [`Hessian::at_scale`]. The centre line's edge suppression
/// compares `|σ∇L|` with the Hessian's eigenvalues at the same scale. The
/// pair comes back in a [`Gradients`], but these are σ-scaled Gaussian
/// derivatives, not Scharr's: a plane of slope 1 reads close to σ, not 1.
/// `None` when sigma is not a positive number or is too small or too large
/// to make kernels (see [`DerivativeKernels::at_scale`]).
pub fn normalised_gradient(image: &GreyImage<f32>, sigma: f64) -> Option<Gradients> {
    let kernels = DerivativeKernels::at_scale(sigma)?;
    Some(Gradients {
        x: scale_by(
            filter_separable(image, &kernels.first, &kernels.smooth)?,
            sigma,
        ),
        y: scale_by(
            filter_separable(image, &kernels.smooth, &kernels.first)?,
            sigma,
        ),
    })
}

/// Every value of `grid` multiplied by `factor`.
fn scale_by(grid: GreyImage<f32>, factor: f64) -> GreyImage<f32> {
    grid.map(|v| (f64::from(*v) * factor) as f32)
}

/// The line measure of the symmetric matrix `[[a, b], [b, c]]`.
pub fn line_measure(a: f64, b: f64, c: f64) -> LineMeasure {
    let mean = (a + c) / 2.0;
    let half_gap = (a - c) / 2.0;
    let root = (half_gap * half_gap + b * b).sqrt();
    let plus = mean + root;
    let minus = mean - root;
    let theta_plus = 0.5 * libm::atan2(2.0 * b, a - c);
    // The stronger eigenvalue is λ2; its direction is the normal across the line.
    let (lambda1, lambda2, theta2) = if plus.abs() >= minus.abs() {
        (minus, plus, theta_plus)
    } else {
        (plus, minus, theta_plus + PI / 2.0)
    };
    let ratio = if lambda2 == 0.0 {
        0.0
    } else {
        lambda1 / lambda2
    };
    let energy = lambda1 * lambda1 + lambda2 * lambda2;
    let strength = libm::exp(-ratio * ratio / (2.0 * BETA * BETA))
        * (1.0 - libm::exp(-energy / (2.0 * C * C)));
    LineMeasure {
        strength,
        lambda2,
        normal: fold_to_half_turn(theta2),
    }
}

/// Folds an angle into [−π/2, π/2): a line has the same normal at θ and θ + π.
fn fold_to_half_turn(theta: f64) -> f64 {
    theta - PI * ((theta + PI / 2.0) / PI).floor()
}

/// Combines the line measures of several scales, for one polarity. At each
/// pixel the strength is the largest over the scales whose `λ2` has the
/// polarity's sign, the scale is the one of those with the largest `|λ2|`,
/// and the normal is that scale's. Pixels with no such scale get strength 0.
/// `None` when there are no scales, more than 256 of them, or their sizes
/// differ.
pub fn combine_scales(scales: &[LineMeasures], polarity: Polarity) -> Option<ScaleCombination> {
    let first = scales.first()?;
    if scales.len() > 256 {
        return None;
    }
    let same_size = scales.iter().all(|s| {
        s.strength.width() == first.strength.width()
            && s.strength.height() == first.strength.height()
    });
    if !same_size {
        return None;
    }
    let count = first.strength.pixels().len();
    let mut strength = vec![0.0f32; count];
    let mut index = vec![0u8; count];
    let mut normal = vec![0.0f32; count];
    let mut winning_magnitude = vec![0.0f32; count];
    for (scale_number, scale) in scales.iter().enumerate() {
        for pixel in 0..count {
            let lambda = scale.lambda2.pixels()[pixel];
            let right_sign = match polarity {
                Polarity::Bright => lambda < 0.0,
                Polarity::Dark => lambda > 0.0,
            };
            if !right_sign {
                continue;
            }
            strength[pixel] = strength[pixel].max(scale.strength.pixels()[pixel]);
            if lambda.abs() > winning_magnitude[pixel] {
                winning_magnitude[pixel] = lambda.abs();
                index[pixel] = scale_number as u8;
                normal[pixel] = scale.normal.pixels()[pixel];
            }
        }
    }
    Some(ScaleCombination {
        strength: like(&first.strength, strength),
        scale_index: like(&first.strength, index),
        normal: like(&first.strength, normal),
    })
}

/// A grid of `pixels` on the placement and size of `template`.
fn like<U>(template: &GreyImage<f32>, pixels: Vec<U>) -> GreyImage<U> {
    GreyImage::new(
        template.width(),
        template.height(),
        pixels,
        template.placement(),
    )
    .expect("one value per pixel of the template")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grey_image::Placement;

    fn plane(width: u32, height: u32, value: impl Fn(f64, f64) -> f64) -> GreyImage<f32> {
        let pixels = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| value(f64::from(x), f64::from(y)) as f32)
            .collect();
        GreyImage::new(width, height, pixels, Placement::FILM).expect("sizes match")
    }

    /// The standard normal density.
    fn phi(a: f64) -> f64 {
        libm::exp(-a * a / 2.0) / (2.0 * PI).sqrt()
    }

    #[test]
    fn the_kernels_are_the_sampled_gaussian_derivatives() {
        // Sigma 1, radius 3. By hand: the weights e^(−j²/2) sum to
        // Z = 1 + 2(e^(−1/2) + e^(−2) + e^(−9/2)) = 2.5059499, so g(0) = 1/Z =
        // 0.3990503 and g(1) = e^(−1/2)/Z = 0.2420362; m₂ = Σ j² g(j) =
        // 0.9959120. Then first(1) = g(1), first(3) = 3 e^(−9/2)/Z = 0.0132991,
        // second(0) = −m₂ g(0) = −0.3974190, second(1) = (1 − m₂) g(1) =
        // 0.0009894 and second(2) = (4 − m₂) e^(−2)/Z = 0.1622375. The hand
        // values carry seven decimals, so they are checked to 1e-7.
        let kernels = DerivativeKernels::at_scale(1.0).expect("sigma is valid");
        assert_eq!(kernels.smooth.len(), 7);
        let near = |value: f64, want: f64| (value - want).abs() < 1e-7;
        assert!(near(kernels.smooth[3], 0.399_050_3));
        assert!(near(kernels.first[4], 0.242_036_2));
        assert!(near(kernels.first[2], -0.242_036_2));
        assert!(near(kernels.first[6], 0.013_299_1));
        assert!(near(kernels.second[3], -0.397_419_0));
        assert!(near(kernels.second[4], 0.000_989_4));
        assert!(near(kernels.second[5], 0.162_237_5));
        // The second kernel sums to zero and the smoothing kernel to one, to
        // f64 rounding over seven terms.
        assert!(kernels.second.iter().sum::<f64>().abs() < 1e-15);
        assert!((kernels.smooth.iter().sum::<f64>() - 1.0).abs() < 1e-15);
        assert!(DerivativeKernels::at_scale(0.0).is_none());
    }

    #[test]
    fn a_bar_peaks_at_half_its_width_with_the_analytic_strength() {
        // A bright top-hat bar of width d and contrast 1 has, at its centre,
        // σ²|L''| = 2aφ(a) with a = d / 2σ (evidence spec 9.1). That is
        // largest at a = 1, σ = d/2, where it is 2φ(1) = 0.48394. The kernels
        // stop at 3σ, which drops 2·3φ(3) = 2.7% of the weight of G''. For
        // bars up to 15 px that leaves the peak under 2% low (wider bars come
        // out lower: see the next test), and the narrowest lands slightly
        // high from pixel sampling, so the strength is checked to 0.012,
        // 2.5% of the peak. The bar's edges fall between whole
        // pixels, which moves the best sigma by under 0.2 px; it is checked
        // to 0.25 px on a grid of 0.05 px.
        for width in [5u32, 9, 15] {
            let half = f64::from(width - 1) / 2.0;
            let bar = plane(
                81,
                1,
                |x, _| {
                    if (x - 40.0).abs() <= half { 1.0 } else { 0.0 }
                },
            );
            let (best_sigma, best) = (0..=160)
                .map(|step| 1.5 + 0.05 * f64::from(step))
                .map(|sigma| {
                    let hessian = Hessian::at_scale(&bar, sigma).expect("sigma is valid");
                    (sigma, -f64::from(*hessian.xx.get(40, 0).expect("inside")))
                })
                .fold(
                    (0.0, f64::MIN),
                    |kept, next| {
                        if next.1 > kept.1 { next } else { kept }
                    },
                );
            let half_width = f64::from(width) / 2.0;
            assert!(
                (best - 2.0 * phi(1.0)).abs() < 0.012,
                "width {width}: peak {best}"
            );
            assert!(
                (best_sigma - half_width).abs() < 0.25,
                "width {width}: best sigma {best_sigma}"
            );
        }
    }

    #[test]
    fn a_wide_bar_peaks_low_by_the_cut_of_the_kernel() {
        // In units of σ the kernels stop at 3, so the smoothing weights sum
        // to Z = 1 − 2Φ(−3) = 0.997300 and m₂ = 1 − 6φ(3)/Z = 0.973337. A
        // bar reaching a either side of the centre then reads
        // σ²|L''| = [(m₂ − 1)(2Φ(a) − 1) + 2aφ(a)] / Z, which is largest
        // where its derivative 2φ(a)(m₂ − a²) / Z is zero, at a = √m₂ =
        // 0.986578, where it is 0.467087: 3.5% under 2φ(1). A sampled kernel
        // reaches ceil(3σ) rather than 3σ and the bar's edges fall between
        // pixels, both effects of order 1/σ; at σ ≈ 60 that is about 0.5% of
        // the peak, so 0.0025.
        let bar = plane(
            801,
            1,
            |x, _| {
                if (x - 400.0).abs() <= 60.0 { 1.0 } else { 0.0 }
            },
        );
        let best = (0..=60)
            .map(|step| 58.0 + 0.1 * f64::from(step))
            .map(|sigma| {
                let hessian = Hessian::at_scale(&bar, sigma).expect("sigma is valid");
                -f64::from(*hessian.xx.get(400, 0).expect("inside"))
            })
            .fold(f64::MIN, f64::max);
        assert!((best - 0.467_087).abs() < 0.0025, "peak {best}");
        assert!(best < 2.0 * phi(1.0) * 0.98, "peak {best}");
    }

    #[test]
    fn a_ridge_reads_the_same_along_an_axis_and_a_diagonal() {
        // A Gaussian ridge of width s blurred at sigma σ is a Gaussian of
        // width √(s² + σ²) and height s / √(s² + σ²), so across its centre
        // σ²L'' = −σ² s / (s² + σ²)^(3/2). With s = σ = 3 that is
        // −27 / 18^(3/2) = −0.35355. The 3σ cut of the kernels (2.7% of the
        // weight of G'') makes it come out 3.1% low at this scale (module
        // doc), so 4%. Turned 45°, the ridge's λ2 comes from the
        // cross kernel as well as the second-derivative kernel, and the two
        // must agree to the 1% that sampling a diagonal on whole pixels
        // costs.
        let along_y = plane(41, 41, |x, _| libm::exp(-(x - 20.0) * (x - 20.0) / 18.0));
        let along_diagonal = plane(41, 41, |x, y| {
            let across = (x - 20.0) - (y - 20.0);
            libm::exp(-across * across / 36.0)
        });
        let measure_at_centre = |image: &GreyImage<f32>| {
            let measures = Hessian::at_scale(image, 3.0)
                .expect("sigma is valid")
                .measures();
            (
                f64::from(*measures.lambda2.get(20, 20).expect("inside")),
                f64::from(*measures.normal.get(20, 20).expect("inside")),
            )
        };
        let exact = -27.0 / libm::pow(18.0, 1.5);
        let (axis, axis_normal) = measure_at_centre(&along_y);
        let (diagonal, diagonal_normal) = measure_at_centre(&along_diagonal);
        assert!(((axis - exact) / exact).abs() < 0.04, "axis λ2 {axis}");
        assert!(
            ((diagonal - exact) / exact).abs() < 0.04,
            "diagonal λ2 {diagonal}"
        );
        assert!(
            ((diagonal - axis) / axis).abs() < 0.01,
            "{diagonal} against {axis}"
        );
        // The normals: along x for the first, along (1, −1) for the second.
        // Each is an f32 angle from an exact symmetry, so 1e-6 covers it.
        assert!(axis_normal.abs() < 1e-6, "normal {axis_normal}");
        assert!(
            (diagonal_normal + PI / 4.0).abs() < 1e-6,
            "normal {diagonal_normal}"
        );
    }

    #[test]
    fn the_normalised_gradient_of_a_step_and_a_ramp_are_analytic() {
        // A unit step between pixels 39 and 40: pixel 40 lies a = 0.5/σ from
        // the edge, where σ·∂L/∂x = φ(a). At σ = 3 that is φ(1/6) = 0.39344;
        // the kernel sampled on whole pixels gives 0.39326 by hand, so 1e-3
        // allows for that and for f32.
        let step = plane(81, 1, |x, _| if x >= 40.0 { 1.0 } else { 0.0 });
        let gradient = normalised_gradient(&step, 3.0).expect("sigma is valid");
        let at_edge = f64::from(*gradient.x.get(40, 0).expect("inside"));
        assert!((at_edge - phi(0.5 / 3.0)).abs() < 1e-3, "{at_edge}");
        // Down the single row every tap reads the same value, so the odd
        // kernel cancels to f64 rounding.
        assert!(gradient.y.get(40, 0).expect("inside").abs() < 1e-12);
        // A ramp 2y has σ·∂L/∂y = 2σ = 6, less the share of the second moment
        // beyond 3σ, at most 2(3φ(3) + Φ(−3)) = 2.9%; so 3%, and never more
        // than 6.
        let ramp = plane(41, 41, |_, y| 2.0 * y);
        let gradient = normalised_gradient(&ramp, 3.0).expect("sigma is valid");
        let slope = f64::from(*gradient.y.get(20, 20).expect("inside"));
        assert!(((slope - 6.0) / 6.0).abs() < 0.03, "{slope}");
        assert!(slope < 6.0, "the cut only loses weight: {slope}");
        assert!(normalised_gradient(&ramp, -1.0).is_none());
    }

    #[test]
    fn a_bright_line_is_a_ridge_across_its_normal() {
        // I depends on y only, bright at y = 15: no curvature along x, and
        // the normal across the line is vertical. On −y² the zero-sum second
        // kernel gives σ²L_yy = −σ²(m₄ − m₂²)/σ⁴, close to −2σ² = −4.5, while
        // L_xx and L_xy are zero up to f64 rounding. So Rb is about 1e-16
        // and S²/2c² about 40: V = e^(−Rb²/2β²)(1 − e^(−S²/2c²)) is 1 to
        // within 1e-17, and stored as f32 it is 1 to within half an ulp (6e-8);
        // 1e-6 covers both. The normal is the f32 of an exact π/2, so 1e-6.
        let ridge = plane(31, 31, |_, y| -(y - 15.0) * (y - 15.0));
        let measures = Hessian::at_scale(&ridge, 1.5)
            .expect("sigma is valid")
            .measures();
        let lambda2 = *measures.lambda2.get(15, 15).expect("inside");
        let strength = *measures.strength.get(15, 15).expect("inside");
        let normal = f64::from(*measures.normal.get(15, 15).expect("inside"));
        assert!(lambda2 < 0.0, "λ2 {lambda2}");
        assert!((strength - 1.0).abs() < 1e-6, "strength {strength}");
        assert!((normal.abs() - PI / 2.0).abs() < 1e-6, "normal {normal}");
    }

    #[test]
    fn a_dark_line_is_a_valley_not_a_ridge() {
        let valley = plane(31, 31, |_, y| (y - 15.0) * (y - 15.0));
        let measures = Hessian::at_scale(&valley, 1.5)
            .expect("sigma is valid")
            .measures();
        let lambda2 = *measures.lambda2.get(15, 15).expect("inside");
        assert!(lambda2 > 0.0, "λ2 {lambda2}");
        let bright =
            combine_scales(std::slice::from_ref(&measures), Polarity::Bright).expect("one scale");
        assert_eq!(*bright.strength.get(15, 15).expect("inside"), 0.0);
        // The valley y² is the bright ridge's −y² turned over: λ2 is the same
        // size with the other sign, Rb and S² are unchanged, and so V is 1 to
        // within 1e-17 before f32 storage and half an ulp (6e-8) after; 1e-6
        // covers both, as for the ridge.
        let dark =
            combine_scales(std::slice::from_ref(&measures), Polarity::Dark).expect("one scale");
        let dark_strength = *dark.strength.get(15, 15).expect("inside");
        assert!(
            (dark_strength - 1.0).abs() < 1e-6,
            "strength {dark_strength}"
        );
    }

    #[test]
    fn the_line_measure_of_a_blob_and_of_a_bar_are_hand_computed() {
        // Every matrix here has small whole-number entries, so the
        // eigenvalues come out exact and Rb and S² with them; what is left
        // is the rounding of exp and atan2, a few parts in 1e16. Each
        // strength and normal is checked to 1e-12, far above that rounding
        // and far below any wrong constant or branch.
        //
        // Equal eigenvalues (a blob): Rb = 1 and S² = 18, so
        // V = e^(−2) · (1 − e^(−36)), which is e^(−2) less 3e-17.
        let blob = line_measure(-3.0, 0.0, -3.0);
        assert!(
            (blob.strength - libm::exp(-2.0)).abs() < 1e-12,
            "{}",
            blob.strength
        );
        assert_eq!(blob.lambda2, -3.0);
        // Curvature −5 along x and none along y: a bright bar running along
        // y, so its normal is along x. Rb = 0 and V = 1 − e^(−50).
        let bar = line_measure(-5.0, 0.0, 0.0);
        assert!((bar.strength - 1.0).abs() < 1e-12);
        assert_eq!(bar.lambda2, -5.0);
        assert!(bar.normal.abs() < 1e-12, "normal {}", bar.normal);
        // Curvature only along y: a bar running along x, normal along y.
        let bar_x = line_measure(0.0, 0.0, -5.0);
        assert!(
            (bar_x.normal.abs() - PI / 2.0).abs() < 1e-12,
            "{}",
            bar_x.normal
        );
        // Dark valleys take the λ+ branch. Curvature +5 along x: a valley
        // running along y, normal 0. Along y only: normal −π/2, the folded
        // end of [−π/2, π/2).
        let valley = line_measure(5.0, 0.0, 0.0);
        assert_eq!(valley.lambda2, 5.0);
        assert!(valley.normal.abs() < 1e-12, "normal {}", valley.normal);
        let valley_x = line_measure(0.0, 0.0, 5.0);
        assert!(
            (valley_x.normal + PI / 2.0).abs() < 1e-12,
            "{}",
            valley_x.normal
        );
        // I = (x − y)² / 2 has Hessian [[1, −1], [−1, 1]]: eigenvalue 2 along
        // (1, −1), so a valley along the diagonal with normal −π/4.
        let diagonal = line_measure(1.0, -1.0, 1.0);
        assert!((diagonal.lambda2 - 2.0).abs() < 1e-12);
        assert!(
            (diagonal.normal + PI / 4.0).abs() < 1e-12,
            "{}",
            diagonal.normal
        );
        // Distinct eigenvalues −2 along x and −1 along y: λ2 = −2, normal 0.
        let ellipse = line_measure(-2.0, 0.0, -1.0);
        assert_eq!(ellipse.lambda2, -2.0);
        assert!(ellipse.normal.abs() < 1e-12);
    }

    #[test]
    fn a_weak_bar_shows_c_and_a_flat_pixel_has_no_line() {
        // λ2 = −0.5 and λ1 = 0: Rb = 0 and S² = 0.25, so with c = 0.5,
        // V = 1 − e^(−0.25 / 0.5) = 1 − e^(−0.5) = 0.3934693. A bar this weak
        // is the only place c shows; with c = 0.6 it would be 0.2934. The
        // hand value carries seven decimals, so it is checked to 1e-7.
        let weak = line_measure(-0.5, 0.0, 0.0);
        assert!(
            (weak.strength - 0.393_469_3).abs() < 1e-7,
            "{}",
            weak.strength
        );
        // A flat pixel has λ1 = λ2 = 0. Rb is taken as 0 there rather than
        // 0/0, so V = 1 · (1 − e^0) = 0 exactly, and finite.
        let flat = line_measure(0.0, 0.0, 0.0);
        assert_eq!(flat.strength, 0.0);
        assert!(flat.normal.is_finite(), "normal {}", flat.normal);
    }

    #[test]
    fn scales_combine_by_the_largest_strength_and_the_largest_curvature() {
        let single = |strength: f32, lambda: f32, normal: f32| LineMeasures {
            strength: GreyImage::new(1, 1, vec![strength], Placement::FILM).expect("one value"),
            lambda2: GreyImage::new(1, 1, vec![lambda], Placement::FILM).expect("one value"),
            normal: GreyImage::new(1, 1, vec![normal], Placement::FILM).expect("one value"),
        };
        let scales = [
            single(0.3, -2.0, 0.1),
            single(0.1, -5.0, 0.7),
            single(0.9, 4.0, -0.4),
        ];
        // Bright: scales 0 and 1 count. Strength is the larger 0.3; the winner is
        // scale 1, the larger |λ2| = 5, so its normal 0.7 is reported.
        let bright = combine_scales(&scales, Polarity::Bright).expect("scales agree");
        assert_eq!(bright.strength.pixels(), &[0.3]);
        assert_eq!(bright.scale_index.pixels(), &[1]);
        assert_eq!(bright.normal.pixels(), &[0.7]);
        // Dark: only scale 2 has λ2 > 0.
        let dark = combine_scales(&scales, Polarity::Dark).expect("scales agree");
        assert_eq!(dark.strength.pixels(), &[0.9]);
        assert_eq!(dark.scale_index.pixels(), &[2]);
        assert_eq!(dark.normal.pixels(), &[-0.4]);
    }

    #[test]
    fn a_sigma_that_is_not_positive_has_no_hessian() {
        let flat = plane(5, 5, |_, _| 1.0);
        assert!(Hessian::at_scale(&flat, 0.0).is_none());
        assert!(Hessian::at_scale(&flat, -1.0).is_none());
        assert!(combine_scales(&[], Polarity::Bright).is_none());
    }

    #[test]
    fn a_sigma_too_small_for_its_fourth_power_has_no_kernels() {
        // At 1e-160, σ² = 1e-320 is still above zero but σ⁴ = 1e-640 is
        // not, so the second kernel would be 0/0: refused, and so is
        // everything built on the kernels.
        let flat = plane(5, 5, |_, _| 1.0);
        assert!(DerivativeKernels::at_scale(1e-160).is_none());
        assert!(Hessian::at_scale(&flat, 1e-160).is_none());
        assert!(normalised_gradient(&flat, 1e-160).is_none());
        // At 1e-80, σ⁴ = 1e-320 is above zero. The side weights
        // e^(−1/2σ²) underflow to exactly 0, so the smoothing kernel is the
        // identity and both derivative kernels are exactly zero.
        let tiny = DerivativeKernels::at_scale(1e-80).expect("σ⁴ is above zero");
        assert_eq!(tiny.smooth, vec![0.0, 1.0, 0.0]);
        assert_eq!(tiny.first, vec![0.0, 0.0, 0.0]);
        assert_eq!(tiny.second, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn a_sigma_too_large_for_the_longest_kernel_has_no_kernels() {
        // The kernels share the smoothing kernel's radius ceil(3σ), capped at
        // 16384: at 5461 it is 16383, so each kernel has 32767 weights; at
        // 5461.5 it would be 16385, so the kernels are refused, and so is
        // everything built on them, up to sigmas whose kernels would not fit
        // in memory at all.
        let at_cap = DerivativeKernels::at_scale(5461.0).expect("radius 16383 is allowed");
        assert_eq!(at_cap.first.len(), 32_767);
        assert_eq!(at_cap.second.len(), 32_767);
        let flat = plane(5, 5, |_, _| 1.0);
        for sigma in [5461.5, 1e7, 1e19, 1e300] {
            assert!(DerivativeKernels::at_scale(sigma).is_none(), "{sigma}");
            assert!(Hessian::at_scale(&flat, sigma).is_none(), "{sigma}");
            assert!(normalised_gradient(&flat, sigma).is_none(), "{sigma}");
        }
    }
}
