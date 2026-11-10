//! The two-dimensional FFT with padding, on rustfft (wasm SIMD in the browser
//! build), and the frequency grids the filters are built on.
//!
//! A grid is row-major: index `y · width + x`. Frequencies are in cycles per
//! pixel, with index 0 at zero and the negative frequencies wrapped after the
//! Nyquist index, as `ifftshift` lays them out. Padding for a fast transform
//! is added at the bottom and right, by reflection without repeating the edge
//! pixel (the rule of [`crate::grey_image::reflect_index`]); the padded pixels
//! must be left out of every statistic taken afterwards.

use crate::grey_image::GreyImage;
use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;
use transpose::transpose;

/// A two-dimensional forward and inverse FFT of one size, with its plans
/// made once.
pub struct Fft2 {
    width: usize,
    height: usize,
    row_forward: Arc<dyn Fft<f32>>,
    column_forward: Arc<dyn Fft<f32>>,
    row_inverse: Arc<dyn Fft<f32>>,
    column_inverse: Arc<dyn Fft<f32>>,
}

impl Fft2 {
    /// Plans for grids of `width` by `height` values.
    pub fn new(width: usize, height: usize) -> Fft2 {
        let mut planner = FftPlanner::<f32>::new();
        Fft2 {
            width,
            height,
            row_forward: planner.plan_fft_forward(width),
            column_forward: planner.plan_fft_forward(height),
            row_inverse: planner.plan_fft_inverse(width),
            column_inverse: planner.plan_fft_inverse(height),
        }
    }

    /// The unnormalised forward transform: `F(k, l) = Σ f(x, y) e^(−2πi(kx/W + ly/H))`.
    /// `None` unless there is one value per grid point.
    pub fn forward(&self, values: &[Complex<f32>]) -> Option<Vec<Complex<f32>>> {
        self.transform(values, &self.row_forward, &self.column_forward, 1.0)
    }

    /// The inverse transform, divided by `W · H` so that `inverse(forward(f)) = f`.
    /// `None` unless there is one value per grid point.
    pub fn inverse(&self, values: &[Complex<f32>]) -> Option<Vec<Complex<f32>>> {
        let scale = 1.0 / (self.width * self.height) as f32;
        self.transform(values, &self.row_inverse, &self.column_inverse, scale)
    }

    /// Transforms every row, transposes, transforms every column and transposes
    /// back, then multiplies by `scale`.
    fn transform(
        &self,
        values: &[Complex<f32>],
        row_plan: &Arc<dyn Fft<f32>>,
        column_plan: &Arc<dyn Fft<f32>>,
        scale: f32,
    ) -> Option<Vec<Complex<f32>>> {
        let (width, height) = (self.width, self.height);
        if values.len() != width.checked_mul(height)? {
            return None;
        }
        if values.is_empty() {
            return Some(Vec::new());
        }
        let mut scratch = vec![
            Complex::new(0.0, 0.0);
            row_plan
                .get_inplace_scratch_len()
                .max(column_plan.get_inplace_scratch_len())
        ];
        let mut data = values.to_vec();
        for row in data.chunks_mut(width) {
            row_plan.process_with_scratch(row, &mut scratch);
        }
        let mut turned = vec![Complex::new(0.0, 0.0); values.len()];
        transpose(&data, &mut turned, width, height);
        for column in turned.chunks_mut(height) {
            column_plan.process_with_scratch(column, &mut scratch);
        }
        transpose(&turned, &mut data, height, width);
        for value in &mut data {
            *value *= scale;
        }
        Some(data)
    }
}

/// The smallest size at least `n` whose only prime factors are 2, 3 and 5,
/// which the FFT handles fastest.
pub fn padded_size(n: usize) -> usize {
    let mut size = n.max(1);
    loop {
        let mut rest = size;
        for factor in [2, 3, 5] {
            while rest.is_multiple_of(factor) {
                rest /= factor;
            }
        }
        if rest == 1 {
            return size;
        }
        size += 1;
    }
}

/// The frequency, in cycles per pixel, of index `index` on a grid of `n`
/// samples: `k / n` with `k` the index, wrapped to negative past the middle.
/// For even `n` the Nyquist index `n / 2` reads −1/2.
pub fn frequency(index: usize, n: usize) -> f64 {
    if n == 0 {
        return 0.0;
    }
    let signed = if index <= (n - 1) / 2 {
        index as f64
    } else {
        index as f64 - n as f64
    };
    signed / n as f64
}

/// Whether `index` is the Nyquist index of a grid of `n` samples: the one
/// that has no partner of the opposite sign, and so needs the Hermitian rule
/// of the filters built on this grid.
pub fn is_nyquist(index: usize, n: usize) -> bool {
    n > 0 && n.is_multiple_of(2) && index == n / 2
}

/// The image on a grid of `width` by `height` (each at least the image's own
/// size), with the new rows and columns filled by reflection. The placement
/// does not change, because the padding is at the bottom and right. `None`
/// when the grid is smaller than the image.
pub fn pad_reflect(image: &GreyImage<f32>, width: u32, height: u32) -> Option<GreyImage<f32>> {
    if width < image.width() || height < image.height() {
        return None;
    }
    let mut pixels = Vec::with_capacity(width as usize * height as usize);
    for y in 0..height {
        for x in 0..width {
            let value = image
                .reflected(i64::from(x), i64::from(y))
                .copied()
                .unwrap_or(0.0);
            pixels.push(value);
        }
    }
    GreyImage::new(width, height, pixels, image.placement())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grey_image::Placement;

    fn complex(values: &[f32]) -> Vec<Complex<f32>> {
        values.iter().map(|&v| Complex::new(v, 0.0)).collect()
    }

    #[test]
    fn the_transform_of_a_delta_is_flat() {
        // A unit impulse at the origin has every frequency at magnitude 1.
        // Every output is the one value 1 times twiddles of unit size, so
        // the f32 error is a few ulps of 1 (1.2e-7 each); 1e-6.
        let (width, height) = (6, 4);
        let mut values = vec![0.0f32; width * height];
        values[0] = 1.0;
        let spectrum = Fft2::new(width, height)
            .forward(&complex(&values))
            .expect("one value per point");
        for value in spectrum {
            assert!(
                (value.re - 1.0).abs() < 1e-6 && value.im.abs() < 1e-6,
                "{value}"
            );
        }
    }

    #[test]
    fn a_shifted_delta_keeps_unit_magnitude_and_turns_the_phase() {
        // An impulse at (1, 2) has F(k, l) = e^(−2πi(k/6 + 2l/4)). At (1, 0) that
        // is e^(−iπ/3) = 1/2 − i√3/2. As for the delta at the origin, the
        // error is a few f32 ulps of 1; 1e-6.
        let (width, height) = (6, 4);
        let mut values = vec![0.0f32; width * height];
        values[2 * width + 1] = 1.0;
        let spectrum = Fft2::new(width, height)
            .forward(&complex(&values))
            .expect("one value per point");
        for value in &spectrum {
            assert!((value.norm() - 1.0).abs() < 1e-6, "{value}");
        }
        let at_one_zero = spectrum[1];
        assert!((at_one_zero.re - 0.5).abs() < 1e-6, "{at_one_zero}");
        assert!(
            (at_one_zero.im + 3f32.sqrt() / 2.0).abs() < 1e-6,
            "{at_one_zero}"
        );
    }

    #[test]
    fn a_round_trip_returns_the_values() {
        // The values are below 2.5 in size and there are 30 of them. Each
        // f32 pass rounds by a few ulps of its largest partial sum (under 75,
        // ulp 7.6e-6), and the inverse divides that by 30, so each value
        // comes back within about 1e-6; 1e-5 is the bound.
        let (width, height) = (6, 5);
        let values: Vec<f32> = (0..width * height)
            .map(|i| {
                let (x, y) = ((i % width) as f64, (i / width) as f64);
                (libm::sin(0.7 * x + 1.3 * y) + 0.1 * x * y) as f32
            })
            .collect();
        let fft = Fft2::new(width, height);
        let spectrum = fft.forward(&complex(&values)).expect("one value per point");
        let back = fft.inverse(&spectrum).expect("one value per point");
        for (original, returned) in values.iter().zip(&back) {
            assert!(
                (returned.re - original).abs() < 1e-5,
                "{original} came back {returned}"
            );
            assert!(returned.im.abs() < 1e-5);
        }
    }

    #[test]
    fn a_cosine_wave_has_two_peaks_of_half_the_grid_size() {
        // cos(2π · 2x / 8) on an 8 × 4 grid: F(2, 0) = F(6, 0) = 8 · 4 / 2 = 16.
        // The inputs are f32 roundings of the cosine (within 6e-8 each) and
        // the transform rounds by a few ulps of 16 (1.9e-6 each), so 1e-4.
        let (width, height) = (8, 4);
        let values: Vec<f32> = (0..width * height)
            .map(|i| {
                let x = (i % width) as f64;
                libm::cos(2.0 * core::f64::consts::PI * 2.0 * x / 8.0) as f32
            })
            .collect();
        let spectrum = Fft2::new(width, height)
            .forward(&complex(&values))
            .expect("one value per point");
        assert!((spectrum[2].re - 16.0).abs() < 1e-4 && spectrum[2].im.abs() < 1e-4);
        assert!((spectrum[6].re - 16.0).abs() < 1e-4 && spectrum[6].im.abs() < 1e-4);
        for (index, value) in spectrum.iter().enumerate() {
            if index != 2 && index != 6 {
                assert!(value.norm() < 1e-4, "index {index} is {value}");
            }
        }
    }

    #[test]
    fn a_grid_of_the_wrong_length_has_no_transform() {
        let fft = Fft2::new(3, 2);
        assert!(fft.forward(&complex(&[1.0; 5])).is_none());
        assert!(fft.inverse(&complex(&[1.0; 7])).is_none());
        assert_eq!(Fft2::new(0, 4).forward(&[]), Some(Vec::new()));
    }

    #[test]
    fn padded_sizes_use_only_the_factors_two_three_and_five() {
        assert_eq!(padded_size(1), 1);
        assert_eq!(padded_size(7), 8);
        assert_eq!(padded_size(11), 12);
        assert_eq!(padded_size(13), 15);
        assert_eq!(padded_size(17), 18);
        assert_eq!(padded_size(4096), 4096);
    }

    #[test]
    fn the_frequency_grid_wraps_after_the_middle() {
        // Eight samples: 0, 1/8, 2/8, 3/8, the Nyquist −1/2, then the negatives.
        let even: Vec<f64> = (0..8).map(|k| frequency(k, 8)).collect();
        assert_eq!(
            even,
            vec![0.0, 0.125, 0.25, 0.375, -0.5, -0.375, -0.25, -0.125]
        );
        assert!(is_nyquist(4, 8));
        assert!(!is_nyquist(3, 8));
        // Seven samples: no Nyquist index, and the wrap is at index 4.
        assert_eq!(frequency(3, 7), 3.0 / 7.0);
        assert_eq!(frequency(4, 7), -3.0 / 7.0);
        assert!(!is_nyquist(3, 7));
    }

    #[test]
    fn padding_mirrors_the_edge_without_repeating_it() {
        // A 3 × 2 image padded to 5 × 4: x = 3 reads column 1 and x = 4 reads
        // column 0; y = 2 reads row 0 and y = 3 reads row 1.
        let image = GreyImage::new(3, 2, vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0], Placement::FILM)
            .expect("sizes match");
        let padded = pad_reflect(&image, 5, 4).expect("the grid is larger");
        let expected = [
            1.0, 2.0, 3.0, 2.0, 1.0, //
            4.0, 5.0, 6.0, 5.0, 4.0, //
            1.0, 2.0, 3.0, 2.0, 1.0, //
            4.0, 5.0, 6.0, 5.0, 4.0,
        ];
        assert_eq!(padded.pixels(), &expected);
        assert_eq!(padded.placement(), image.placement());
        assert!(pad_reflect(&image, 2, 4).is_none());
    }
}
