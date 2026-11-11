//! The log-Gabor filter bank, built in the frequency domain, and the one
//! spectrum it and every other frequency-domain filter read.
//!
//! A [`Spectrum`] is the transform of an image padded to a fast size by
//! reflection, with the periodic-plus-smooth decomposition taken out
//! (Moisan 2011, as Kovesi's `perfft2` computes it): only the periodic
//! part is kept, so the wrap from one side of the padded grid to the other
//! makes no false edge. A filter is a transfer function on the
//! [`FrequencyGrid`], and [`Spectrum::filtered`] returns the inverse
//! transform of the spectrum times that function, cropped back to the
//! image and on its placement. When the transfer function is `A + iB` and
//! `A` and `B` each have the Hermitian symmetry of a real filter
//! (`A(−f) = conj A(f)`), the real part of the result is the image through
//! `A` and the imaginary part the image through `B`: two real filters for
//! one inverse transform ([`Spectrum::filtered_pair`]).
//!
//! The bank's scales are radial log-Gabor filters (Field 1987), each times
//! one shared Butterworth low-pass, with the constants of evidence spec
//! 10.2: `LG(f) = exp(−ln²(f / f₀) / (2 ln² σ_f))`, zero at DC, and
//! `LP(f) = 1 / (1 + (f / f_c)^(2n))`. The Riesz pair [`riesz`],
//! `(i fx − fy) / |f|`, turns a scale into the odd, oriented half of a
//! monogenic filter: its real output is the band-passed derivative across x
//! and its imaginary output the one across y, so their angle points from
//! dark to bright in image axes.
//!
//! Frequencies are in cycles per pixel, laid out as [`crate::fft::frequency`]
//! lays them out. A filter that is odd in `fx` alone or in `fy` alone (the
//! Riesz pair, a first derivative, the cross term `fx·fy`) must be zero on
//! the Nyquist row and column of an even-sized grid, where a frequency has no
//! partner of the opposite sign; otherwise its output is not real and
//! packing leaks one result into the other (evidence spec 6.3).
//! [`Frequency::on_nyquist_line`] says where that is.

use crate::fft::{Fft2, frequency, is_nyquist, pad_reflect, padded_size};
use crate::grey_image::{GreyImage, Placement};
use core::f64::consts::PI;
use rustfft::num_complex::Complex;

/// The frequencies of a padded grid, in cycles per pixel.
#[derive(Clone, Debug, PartialEq)]
pub struct FrequencyGrid {
    across: Vec<Frequency1>,
    down: Vec<Frequency1>,
}

/// One axis' frequency and whether it is that axis' Nyquist index.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Frequency1 {
    value: f64,
    nyquist: bool,
}

/// The frequency of one point of a [`FrequencyGrid`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frequency {
    /// Cycles per pixel along x (across a row).
    pub across: f64,
    /// Cycles per pixel along y (down a column).
    pub down: f64,
    /// Whether the point is on the Nyquist row or column of an even size,
    /// where a filter odd in one axis must be zero.
    pub on_nyquist_line: bool,
}

impl Frequency {
    /// `√(fx² + fy²)`.
    pub fn radius(&self) -> f64 {
        (self.across * self.across + self.down * self.down).sqrt()
    }
}

impl FrequencyGrid {
    /// The grid of a `width` by `height` transform.
    pub fn new(width: usize, height: usize) -> FrequencyGrid {
        let axis = |n: usize| {
            (0..n)
                .map(|index| Frequency1 {
                    value: frequency(index, n),
                    nyquist: is_nyquist(index, n),
                })
                .collect()
        };
        FrequencyGrid {
            across: axis(width),
            down: axis(height),
        }
    }

    pub fn width(&self) -> usize {
        self.across.len()
    }

    pub fn height(&self) -> usize {
        self.down.len()
    }

    /// The frequency at row-major index `y · width + x`.
    pub fn at(&self, index: usize) -> Frequency {
        let width = self.width().max(1);
        self.point(&self.across[index % width], &self.down[index / width])
    }

    /// Every frequency, row-major.
    pub fn points(&self) -> impl Iterator<Item = Frequency> + '_ {
        self.down.iter().flat_map(move |down| {
            self.across
                .iter()
                .map(move |across| self.point(across, down))
        })
    }

    fn point(&self, across: &Frequency1, down: &Frequency1) -> Frequency {
        Frequency {
            across: across.value,
            down: down.value,
            on_nyquist_line: across.nyquist || down.nyquist,
        }
    }
}

/// The real and imaginary parts of a filtered image, each on the image's
/// size and placement.
#[derive(Clone, Debug, PartialEq)]
pub struct Filtered {
    pub real: GreyImage<f32>,
    pub imaginary: GreyImage<f32>,
}

/// The periodic part of an image's transform, on a padded grid, with the
/// plans to invert it.
pub struct Spectrum {
    width: u32,
    height: u32,
    placement: Placement,
    grid: FrequencyGrid,
    values: Vec<Complex<f32>>,
    fft: Fft2,
}

impl Spectrum {
    /// The spectrum of `image`: padded at the bottom and right by
    /// reflection to the smallest sizes whose only prime factors are 2, 3
    /// and 5, then the transform of the periodic part (evidence spec 6.2).
    /// Two forward transforms. `None` when the image is empty or holds a
    /// value that is not finite: unusable pixels are filled before a film
    /// is filtered.
    pub fn of(image: &GreyImage<f32>) -> Option<Spectrum> {
        if image.pixels().is_empty() || image.pixels().iter().any(|v| !v.is_finite()) {
            return None;
        }
        let width = padded_size(image.width() as usize);
        let height = padded_size(image.height() as usize);
        let padded = pad_reflect(
            image,
            u32::try_from(width).ok()?,
            u32::try_from(height).ok()?,
        )?;
        let fft = Fft2::new(width, height);
        let pixels = padded.pixels();
        let as_complex = |values: &[f32]| -> Vec<Complex<f32>> {
            values.iter().map(|&v| Complex::new(v, 0.0)).collect()
        };
        let mut values = fft.forward(&as_complex(pixels))?;
        let smooth = fft.forward(&as_complex(&boundary_image(pixels, width, height)))?;
        let cosines = |n: usize| -> Vec<f64> {
            (0..n)
                .map(|k| libm::cos(2.0 * PI * k as f64 / n as f64))
                .collect()
        };
        let (cos_across, cos_down) = (cosines(width), cosines(height));
        for (row, cos_y) in cos_down.iter().enumerate() {
            for (column, cos_x) in cos_across.iter().enumerate() {
                if row == 0 && column == 0 {
                    continue;
                }
                let index = row * width + column;
                let denominator = (2.0 * (2.0 - cos_x - cos_y)) as f32;
                values[index] -= smooth[index] / denominator;
            }
        }
        Some(Spectrum {
            width: image.width(),
            height: image.height(),
            placement: image.placement(),
            grid: FrequencyGrid::new(width, height),
            values,
            fft,
        })
    }

    /// The padded grid the spectrum lives on.
    pub fn grid(&self) -> &FrequencyGrid {
        &self.grid
    }

    /// The width and height of the image the spectrum came from.
    pub fn image_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Where the image the spectrum came from sits on the film.
    pub fn placement(&self) -> Placement {
        self.placement
    }

    /// The inverse transform of the spectrum times `transfer`, cropped to
    /// the image. `transfer` is given each point's row-major index on the
    /// padded grid and its frequency, so a filter can read gains it worked
    /// out once (see [`FilterBank::gains`]) or compute them on the spot.
    /// One inverse transform.
    pub fn filtered(&self, transfer: impl Fn(usize, Frequency) -> Complex<f32>) -> Filtered {
        let padded_width = self.grid.width();
        let mut product = Vec::with_capacity(self.values.len());
        for (row, down) in self.grid.down.iter().enumerate() {
            let start = row * padded_width;
            let values = &self.values[start..start + padded_width];
            for (column, (across, value)) in self.grid.across.iter().zip(values).enumerate() {
                let point = self.grid.point(across, down);
                product.push(value * transfer(start + column, point));
            }
        }
        let back = self
            .fft
            .inverse(&product)
            .expect("the product has one value per grid point");
        let (width, height) = (self.width as usize, self.height as usize);
        let mut real = Vec::with_capacity(width * height);
        let mut imaginary = Vec::with_capacity(width * height);
        for row in back.chunks(padded_width).take(height) {
            for value in &row[..width] {
                real.push(value.re);
                imaginary.push(value.im);
            }
        }
        let image = |pixels| {
            GreyImage::new(self.width, self.height, pixels, self.placement)
                .expect("the crop is the image's size")
        };
        Filtered {
            real: image(real),
            imaginary: image(imaginary),
        }
    }

    /// Two real filters for one inverse transform: the image through
    /// `first` comes back as the real part and through `second` as the
    /// imaginary part. Each must have the Hermitian symmetry of a real
    /// filter, and be zero on the Nyquist lines if it is odd in one axis.
    pub fn filtered_pair(
        &self,
        first: impl Fn(usize, Frequency) -> Complex<f32>,
        second: impl Fn(usize, Frequency) -> Complex<f32>,
    ) -> Filtered {
        let i = Complex::new(0.0f32, 1.0);
        self.filtered(|index, point| first(index, point) + i * second(index, point))
    }
}

/// Kovesi's boundary image: zero except on the grid's outer rows and
/// columns, where it holds the jump across the wrap.
fn boundary_image(pixels: &[f32], width: usize, height: usize) -> Vec<f32> {
    let mut boundary = vec![0.0f32; pixels.len()];
    let last_row = (height - 1) * width;
    for x in 0..width {
        let jump = pixels[x] - pixels[last_row + x];
        boundary[x] += jump;
        boundary[last_row + x] -= jump;
    }
    for y in 0..height {
        let (first, last) = (y * width, y * width + width - 1);
        let jump = pixels[first] - pixels[last];
        boundary[first] += jump;
        boundary[last] -= jump;
    }
    boundary
}

/// The Butterworth low-pass every scale shares: `1 / (1 + (f / cutoff)^(2·order))`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LowPass {
    /// Where the gain is 1/2, in cycles per pixel.
    pub cutoff: f64,
    pub order: u32,
}

impl LowPass {
    /// The cutoff and order of evidence spec 10.2: 0.45 and 15.
    pub const EVIDENCE: LowPass = LowPass {
        cutoff: 0.45,
        order: 15,
    };

    /// The gain at radius `radius` cycles per pixel.
    pub fn gain(&self, radius: f64) -> f64 {
        let ratio_sq = (radius / self.cutoff) * (radius / self.cutoff);
        let mut power = 1.0;
        for _ in 0..self.order {
            power *= ratio_sq;
        }
        1.0 / (1.0 + power)
    }
}

/// One radial log-Gabor filter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogGabor {
    /// The wavelength of the centre frequency, in pixels: `f₀ = 1 / wavelength`.
    pub wavelength: f64,
    /// The ratio of the bandwidth's sigma to the centre frequency, `σ_f`
    /// (0.55 gives about two octaves).
    pub sigma_on_f: f64,
}

impl LogGabor {
    /// `exp(−ln²(f / f₀) / (2 ln² σ_f))`, and 0 at DC.
    pub fn gain(&self, radius: f64) -> f64 {
        if radius <= 0.0 {
            return 0.0;
        }
        let spread = libm::log(self.sigma_on_f);
        let distance = libm::log(radius * self.wavelength);
        libm::exp(-(distance * distance) / (2.0 * spread * spread))
    }
}

/// The Riesz pair `(i fx − fy) / |f|`: zero at DC and on the Nyquist lines.
/// Through it the spectrum of a real image comes back as the x part in the
/// real output and the y part in the imaginary output.
pub fn riesz(point: Frequency) -> Complex<f32> {
    let radius = point.radius();
    if radius == 0.0 || point.on_nyquist_line {
        return Complex::new(0.0, 0.0);
    }
    Complex::new(
        (-point.down / radius) as f32,
        (point.across / radius) as f32,
    )
}

/// The scales of the bank, each a [`LogGabor`] times the shared [`LowPass`],
/// worked out on one grid.
#[derive(Clone, Debug, PartialEq)]
pub struct FilterBank {
    wavelengths: Vec<f64>,
    sigma_on_f: f64,
    log_radius: Vec<f32>,
    low_pass: Vec<f32>,
}

impl FilterBank {
    /// The bank for `grid`, with one scale per wavelength in pixels. `None`
    /// when a wavelength is not a finite number of at least 2 px (the
    /// Nyquist limit), when `sigma_on_f` is not strictly between 0 and 1, or
    /// when the low-pass cutoff is not positive.
    pub fn new(
        grid: &FrequencyGrid,
        wavelengths: &[f64],
        sigma_on_f: f64,
        low_pass: LowPass,
    ) -> Option<FilterBank> {
        let wavelengths_ok = wavelengths.iter().all(|w| w.is_finite() && *w >= 2.0);
        let spread_ok = sigma_on_f > 0.0 && sigma_on_f < 1.0;
        let cutoff_ok = low_pass.cutoff.is_finite() && low_pass.cutoff > 0.0;
        if !(wavelengths_ok && spread_ok && cutoff_ok) {
            return None;
        }
        let mut log_radius = Vec::with_capacity(grid.width() * grid.height());
        let mut low = Vec::with_capacity(grid.width() * grid.height());
        for point in grid.points() {
            let radius = point.radius();
            log_radius.push(if radius > 0.0 {
                libm::log(radius) as f32
            } else {
                f32::NEG_INFINITY
            });
            low.push(low_pass.gain(radius) as f32);
        }
        Some(FilterBank {
            wavelengths: wavelengths.to_vec(),
            sigma_on_f,
            log_radius,
            low_pass: low,
        })
    }

    /// `count` wavelengths from `shortest`, each `multiple` times the one
    /// before, as Kovesi's banks space them.
    pub fn geometric(shortest: f64, multiple: f64, count: usize) -> Vec<f64> {
        let mut wavelength = shortest;
        (0..count)
            .map(|_| {
                let this = wavelength;
                wavelength *= multiple;
                this
            })
            .collect()
    }

    pub fn scales(&self) -> usize {
        self.wavelengths.len()
    }

    /// The wavelength of scale `scale`, in pixels.
    pub fn wavelength(&self, scale: usize) -> Option<f64> {
        self.wavelengths.get(scale).copied()
    }

    /// The gain of scale `scale` at every grid point, row-major: the
    /// log-Gabor times the low-pass. `None` past the last scale.
    pub fn gains(&self, scale: usize) -> Option<Vec<f32>> {
        let log_wavelength = libm::log(self.wavelength(scale)?);
        let spread = libm::log(self.sigma_on_f);
        let denominator = 2.0 * spread * spread;
        Some(
            self.log_radius
                .iter()
                .zip(&self.low_pass)
                .map(|(&log_radius, &low)| {
                    if log_radius == f32::NEG_INFINITY {
                        return 0.0;
                    }
                    let distance = f64::from(log_radius) + log_wavelength;
                    (libm::exp(-(distance * distance) / denominator) * f64::from(low)) as f32
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blur::filter_separable;

    fn image(width: u32, height: u32, value: impl Fn(f64, f64) -> f64) -> GreyImage<f32> {
        let pixels = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| value(f64::from(x), f64::from(y)) as f32)
            .collect();
        GreyImage::new(width, height, pixels, Placement::FILM).expect("sizes match")
    }

    /// A fixed, irregular pattern from a multiplicative hash of the pixel
    /// position, uniform in [−1, 1).
    fn scrambled(x: f64, y: f64) -> f64 {
        let n = (x as u64).wrapping_mul(73_856_093) ^ (y as u64).wrapping_mul(19_349_663);
        let mixed = n.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 40;
        mixed as f64 / (1u64 << 23) as f64 - 1.0
    }

    #[test]
    fn a_log_gabor_peaks_at_its_centre_and_falls_by_its_spread() {
        // LG(f₀) = 1; one sigma either way on the log axis, f₀·σ_f and
        // f₀/σ_f, gives exp(−1/2); DC gives 0. These are the definition's
        // own values, so only f64 rounding separates them: 1e-12.
        let filter = LogGabor {
            wavelength: 6.0,
            sigma_on_f: 0.55,
        };
        let centre = 1.0 / 6.0;
        assert!((filter.gain(centre) - 1.0).abs() < 1e-12);
        let half_spread = libm::exp(-0.5);
        assert!((filter.gain(centre * 0.55) - half_spread).abs() < 1e-12);
        assert!((filter.gain(centre / 0.55) - half_spread).abs() < 1e-12);
        assert_eq!(filter.gain(0.0), 0.0);
    }

    #[test]
    fn the_low_pass_halves_at_its_cutoff() {
        // 1 / (1 + 1) at the cutoff, 1 at DC, and at twice the cutoff
        // 1 / (1 + 2^30): Butterworth by definition.
        let low = LowPass::EVIDENCE;
        assert_eq!(low.gain(0.0), 1.0);
        assert!((low.gain(0.45) - 0.5).abs() < 1e-12);
        let expected = 1.0 / (1.0 + libm::pow(2.0, 30.0));
        assert!((low.gain(0.9) - expected).abs() < 1e-18);
    }

    #[test]
    fn the_frequency_grid_marks_its_nyquist_lines() {
        // On a 4 × 3 grid, column 2 is the Nyquist column (fx = −1/2); 3 rows
        // have no Nyquist row. Row 1 reads fy = 1/3, row 2 reads −1/3.
        let grid = FrequencyGrid::new(4, 3);
        let at = |x: usize, y: usize| grid.at(y * 4 + x);
        assert_eq!(at(2, 0).across, -0.5);
        assert!(at(2, 1).on_nyquist_line);
        assert!(!at(1, 1).on_nyquist_line);
        assert!((at(0, 1).down - 1.0 / 3.0).abs() < 1e-15);
        assert!((at(0, 2).down + 1.0 / 3.0).abs() < 1e-15);
        assert_eq!(grid.points().count(), 12);
        assert_eq!(grid.points().nth(7), Some(at(3, 1)));
    }

    #[test]
    fn the_wavelengths_grow_geometrically() {
        let wavelengths = FilterBank::geometric(3.0, 2.1, 4);
        let expected = [3.0, 6.3, 13.23, 27.783];
        for (got, want) in wavelengths.iter().zip(expected) {
            assert!((got - want).abs() < 1e-12, "{got} against {want}");
        }
    }

    #[test]
    fn the_bank_gains_are_the_log_gabor_times_the_low_pass() {
        // Checked against LogGabor::gain and LowPass::gain at every point of
        // a small grid. The bank stores ln f and the gains in f32, so they
        // agree to f32 rounding of numbers at most 1: 1e-6.
        let grid = FrequencyGrid::new(12, 10);
        let bank = FilterBank::new(&grid, &[3.0, 6.3], 0.55, LowPass::EVIDENCE).expect("valid");
        for scale in 0..2 {
            let gains = bank.gains(scale).expect("scale exists");
            let filter = LogGabor {
                wavelength: bank.wavelength(scale).expect("scale exists"),
                sigma_on_f: 0.55,
            };
            for (point, gain) in grid.points().zip(&gains) {
                let radius = point.radius();
                let want = filter.gain(radius) * LowPass::EVIDENCE.gain(radius);
                assert!((f64::from(*gain) - want).abs() < 1e-6, "{point:?}");
            }
        }
        assert!(bank.gains(2).is_none());
        assert!(FilterBank::new(&grid, &[1.5], 0.55, LowPass::EVIDENCE).is_none());
        assert!(FilterBank::new(&grid, &[3.0], 1.0, LowPass::EVIDENCE).is_none());
    }

    /// The periodic part on the padded grid, read back from the spectrum.
    fn periodic_part(spectrum: &Spectrum) -> Vec<f32> {
        let back = spectrum
            .fft
            .inverse(&spectrum.values)
            .expect("one per point");
        back.iter().map(|v| v.re).collect()
    }

    #[test]
    fn the_periodic_part_has_the_images_own_laplacian_inside() {
        // Moisan's periodic component p is defined by two facts: its
        // Laplacian with wrap-around neighbours equals the image's
        // Laplacian with only the neighbours that exist (a pixel on the
        // edge has three, a corner two), and its mean is the image's mean.
        // Both are checked here by plain sums over the padded image, which
        // is a different route from the Fourier division under test. An
        // odd size forces padding (13 × 11 pads to 15 × 12). The image
        // values are within 1 and the FFT round trip rounds in f32 by about
        // 1e-6 relative to the largest partial sums (under 200), so 1e-4.
        let picture = image(13, 11, |x, y| 0.6 * scrambled(x, y) + 0.05 * x - 0.03 * y);
        let spectrum = Spectrum::of(&picture).expect("finite image");
        let (width, height) = (spectrum.grid.width(), spectrum.grid.height());
        assert_eq!((width, height), (15, 12));
        let padded = pad_reflect(&picture, 15, 12).expect("larger grid");
        let original = padded.pixels();
        let periodic = periodic_part(&spectrum);
        let at = |grid: &[f32], x: usize, y: usize| f64::from(grid[y * width + x]);
        for y in 0..height {
            for x in 0..width {
                let wrapped = [
                    ((x + width - 1) % width, y),
                    ((x + 1) % width, y),
                    (x, (y + height - 1) % height),
                    (x, (y + 1) % height),
                ];
                let periodic_laplacian: f64 = wrapped
                    .iter()
                    .map(|&(u, v)| at(&periodic, u, v) - at(&periodic, x, y))
                    .sum();
                let inside: Vec<(usize, usize)> = [
                    (x.checked_sub(1), Some(y)),
                    ((x + 1 < width).then_some(x + 1), Some(y)),
                    (Some(x), y.checked_sub(1)),
                    (Some(x), (y + 1 < height).then_some(y + 1)),
                ]
                .iter()
                .filter_map(|&(u, v)| Some((u?, v?)))
                .collect();
                let free_laplacian: f64 = inside
                    .iter()
                    .map(|&(u, v)| at(original, u, v) - at(original, x, y))
                    .sum();
                assert!(
                    (periodic_laplacian - free_laplacian).abs() < 1e-4,
                    "({x}, {y}): {periodic_laplacian} against {free_laplacian}"
                );
            }
        }
        let mean =
            |grid: &[f32]| grid.iter().map(|&v| f64::from(v)).sum::<f64>() / grid.len() as f64;
        assert!((mean(&periodic) - mean(original)).abs() < 1e-5);
    }

    #[test]
    fn a_ramp_loses_its_wrap_jump() {
        // A left-to-right ramp jumps by 1 across the wrap. Its periodic part
        // is the ramp less a smooth bowl that takes up the jump, so the
        // largest step between wrap-around neighbours of the periodic part
        // must be far below 1. The plain ramp's interior step is 1/31 and
        // its wrap step 1; the periodic part's wrap step is measured at
        // about 0.03, in line with the interior; 0.1 is the bound.
        let ramp = image(32, 24, |x, _| x / 31.0);
        let spectrum = Spectrum::of(&ramp).expect("finite image");
        let periodic = periodic_part(&spectrum);
        let largest_wrap_step = (0..24)
            .map(|y| (periodic[y * 32] - periodic[y * 32 + 31]).abs())
            .fold(0.0f32, f32::max);
        assert!(largest_wrap_step < 0.1, "{largest_wrap_step}");
    }

    /// A pattern that is zero within `margin` px of every edge, so its
    /// boundary image is zero and its periodic part is the image itself.
    fn framed(width: u32, height: u32, margin: f64) -> GreyImage<f32> {
        image(width, height, |x, y| {
            let inside = x >= margin
                && y >= margin
                && x < f64::from(width) - margin
                && y < f64::from(height) - margin;
            if inside { scrambled(x, y) } else { 0.0 }
        })
    }

    /// The Gaussian derivative along one axis, by its transfer function.
    fn derivative_transfer(sigma: f64, point: Frequency, along_x: bool) -> Complex<f32> {
        if point.on_nyquist_line {
            return Complex::new(0.0, 0.0);
        }
        let radius_sq = point.radius() * point.radius();
        let gaussian = libm::exp(-2.0 * PI * PI * sigma * sigma * radius_sq);
        let axis = if along_x { point.across } else { point.down };
        Complex::new(0.0, (2.0 * PI * axis * gaussian) as f32)
    }

    /// The continuous Gaussian and its derivative sampled out to 6σ, as
    /// correlation kernels: `g(j)` and `j · g(j) / σ²`.
    fn wide_kernels(sigma: f64) -> (Vec<f64>, Vec<f64>) {
        let radius = (6.0 * sigma).ceil() as i64;
        let norm = 1.0 / (sigma * (2.0 * PI).sqrt());
        let smooth: Vec<f64> = (-radius..=radius)
            .map(|j| norm * libm::exp(-((j * j) as f64) / (2.0 * sigma * sigma)))
            .collect();
        let first = (-radius..=radius)
            .zip(&smooth)
            .map(|(j, g)| j as f64 * g / (sigma * sigma))
            .collect();
        (smooth, first)
    }

    #[test]
    fn fourier_derivatives_match_direct_convolution_on_both_axes() {
        // Evidence spec T11. A framed pattern has no wrap jump, so the
        // spectrum is its plain transform, and the Gaussian derivative
        // along x and along y, packed into one inverse transform, must
        // match direct correlation with the continuous kernels sampled to
        // 6σ. A swapped fx and fy, a sign slip or a packing leak would
        // differ by the size of the answer. What remains is the kernel cut
        // at 6σ (a tail of e^−18), aliasing of the sampled kernel at σ = 2
        // (its spectrum at the Nyquist frequency is e^−2π²σ²/4, about
        // e^−20) and f32 rounding, about 1e-7 of the largest response
        // (measured 2e-7). The spec allows 2e-3; 1e-5 is held here. Run on
        // an even size and on an odd one that needs padding.
        for (width, height) in [(64u32, 48u32), (61, 45)] {
            let sigma = 2.0;
            let picture = framed(width, height, 14.0);
            let spectrum = Spectrum::of(&picture).expect("finite image");
            let fourier = spectrum.filtered_pair(
                |_, point| derivative_transfer(sigma, point, true),
                |_, point| derivative_transfer(sigma, point, false),
            );
            let (smooth, first) = wide_kernels(sigma);
            let along_x = filter_separable(&picture, &first, &smooth).expect("odd kernels");
            let along_y = filter_separable(&picture, &smooth, &first).expect("odd kernels");
            for (got, want) in [(&fourier.real, &along_x), (&fourier.imaginary, &along_y)] {
                let largest = want.pixels().iter().fold(0.0f32, |m, v| m.max(v.abs()));
                assert!(largest > 0.05, "the pattern has edges");
                for (a, b) in got.pixels().iter().zip(want.pixels()) {
                    assert!((a - b).abs() <= 1e-5 * largest, "{a} against {b}");
                }
            }
        }
    }

    #[test]
    fn a_packed_pair_matches_two_separate_inverses() {
        // Packing is exact in arithmetic; separately the two filters each
        // give an output whose imaginary part is rounding only. f32
        // transforms of values within 1 on 2880 points round at about 1e-6
        // of the largest values: 1e-5.
        let picture = image(60, 48, scrambled);
        let spectrum = Spectrum::of(&picture).expect("finite image");
        let first = |_: usize, point: Frequency| derivative_transfer(1.5, point, true);
        let second = |_: usize, point: Frequency| {
            let r = point.radius();
            Complex::new(libm::exp(-2.0 * PI * PI * 2.25 * r * r) as f32, 0.0)
        };
        let packed = spectrum.filtered_pair(first, second);
        let alone_first = spectrum.filtered(first);
        let alone_second = spectrum.filtered(second);
        for (a, b) in packed.real.pixels().iter().zip(alone_first.real.pixels()) {
            assert!((a - b).abs() < 1e-5);
        }
        for (a, b) in packed
            .imaginary
            .pixels()
            .iter()
            .zip(alone_second.real.pixels())
        {
            assert!((a - b).abs() < 1e-5);
        }
        for v in alone_first.imaginary.pixels() {
            assert!(v.abs() < 1e-5, "a real filter gives a real image: {v}");
        }
    }

    #[test]
    fn the_riesz_pair_turns_a_wave_into_its_quarter_turned_twin() {
        // For cos(2π(3x + 4y)/64), whose direction is (0.6, 0.8), the Riesz
        // pair gives −0.6·sin and −0.8·sin of the same phase: the
        // band-limited derivative, pointing along the wave, a quarter
        // period ahead. The wave is periodic on the grid, but the periodic
        // part of a sampled wave differs from it by a smooth part driven by
        // the one-sample wrap step, which fades with distance from the
        // edge; in the centre 32 × 32 it is measured below 5e-3, so 1e-2.
        let wave = |x: f64, y: f64| 2.0 * PI * (3.0 * x + 4.0 * y) / 64.0;
        let picture = image(64, 64, |x, y| libm::cos(wave(x, y)));
        let spectrum = Spectrum::of(&picture).expect("finite image");
        let odd = spectrum.filtered(|_, point| riesz(point));
        for y in 16..48u32 {
            for x in 16..48u32 {
                let phase = wave(f64::from(x), f64::from(y));
                let want_x = -0.6 * libm::sin(phase);
                let want_y = -0.8 * libm::sin(phase);
                let got_x = f64::from(*odd.real.get(x, y).expect("inside"));
                let got_y = f64::from(*odd.imaginary.get(x, y).expect("inside"));
                assert!(
                    (got_x - want_x).abs() < 1e-2,
                    "({x}, {y}) x: {got_x} against {want_x}"
                );
                assert!(
                    (got_y - want_y).abs() < 1e-2,
                    "({x}, {y}) y: {got_y} against {want_y}"
                );
            }
        }
    }

    #[test]
    fn a_filter_without_dc_ignores_brightness() {
        // Adding 0.7 everywhere changes only the DC term of the periodic
        // part (the boundary image holds differences, so it is unchanged).
        // A log-Gabor scale is zero at DC, so the outputs agree to f32
        // rounding of the transforms, under 1e-5 for values within 2.
        let picture = image(40, 30, scrambled);
        let brighter = picture.map(|v| v + 0.7);
        let grid = FrequencyGrid::new(40, 30);
        let bank = FilterBank::new(&grid, &[4.0], 0.55, LowPass::EVIDENCE).expect("valid");
        let gains = bank.gains(0).expect("one scale");
        let filter = |index: usize, _: Frequency| Complex::new(gains[index], 0.0);
        let plain = Spectrum::of(&picture).expect("finite").filtered(filter);
        let lifted = Spectrum::of(&brighter).expect("finite").filtered(filter);
        for (a, b) in plain.real.pixels().iter().zip(lifted.real.pixels()) {
            assert!((a - b).abs() < 1e-5, "{a} against {b}");
        }
    }

    #[test]
    fn the_output_keeps_the_images_size_and_placement() {
        let placement = Placement {
            origin: shapes::pixels::Px::new(10.0, 20.0),
            step_across: shapes::pixels::Px::new(2.0, 0.0),
            step_down: shapes::pixels::Px::new(0.0, 2.0),
        };
        let picture = GreyImage::new(7, 5, vec![0.5f32; 35], placement).expect("sizes match");
        let spectrum = Spectrum::of(&picture).expect("finite image");
        assert_eq!(spectrum.image_size(), (7, 5));
        assert_eq!(spectrum.placement(), placement);
        assert_eq!((spectrum.grid().width(), spectrum.grid().height()), (8, 5));
        let out = spectrum.filtered(|_, _| Complex::new(1.0, 0.0));
        assert_eq!((out.real.width(), out.real.height()), (7, 5));
        assert_eq!(out.real.placement(), placement);
        for v in out.real.pixels() {
            assert!(
                (v - 0.5).abs() < 1e-6,
                "an all-pass filter returns the image"
            );
        }
    }

    #[test]
    fn empty_or_unfinished_images_have_no_spectrum() {
        let empty = GreyImage::new(0, 4, Vec::<f32>::new(), Placement::FILM).expect("sizes match");
        assert!(Spectrum::of(&empty).is_none());
        let mut pixels = vec![0.0f32; 12];
        pixels[5] = f32::NAN;
        let holed = GreyImage::new(4, 3, pixels, Placement::FILM).expect("sizes match");
        assert!(Spectrum::of(&holed).is_none());
    }
}
