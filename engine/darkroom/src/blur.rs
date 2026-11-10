//! Separable Gaussian and box blurs, with every sigma in pixels of the image
//! being blurred.
//!
//! Below sigma 2 px a sampled Gaussian kernel of radius ceil(3 sigma) is
//! applied along each axis, normalised to sum 1. From sigma 2 px up, three
//! cascaded box filters stand in for the Gaussian (Wells 1986, box widths
//! from Kovesi 2010): the widths are odd, the first `m` of the three are the
//! smaller width and the rest the larger, and `m` is rounded to a whole
//! pass. Each box pass is a running mean, so the cost of a line is its
//! length plus one box width. Every blur reads past the edge with the
//! mirror rule of [`crate::grey_image::reflect_index`]. Sums run in f64.
//!
//! The cascade's width differs from the Gaussian it stands in for by the
//! rounding of its pass count, and at whole sigmas it is narrower: see
//! [`box_widths`] for by how much.
//!
//! A value that is not finite, NaN or either infinity, makes every output
//! whose window holds it NaN and no other, on both paths and in every
//! [`filter_separable`] pass: an infinity is read as NaN rather than carried
//! through the arithmetic, where it would come out as an infinity of either
//! sign, or NaN where two meet. Invalid pixels belong in a mask and are
//! filled before blurring.

use crate::grey_image::{GreyImage, reflect_index};

/// A Gaussian of standard deviation `sigma` px, applied along x and then y.
/// `None` when sigma is negative or not finite, so small that its square
/// is zero (see [`sampled_gaussian`]), or so large that the box widths do
/// not fit in 32 bits (see [`box_widths`]).
pub fn gaussian(image: &GreyImage<f32>, sigma: f64) -> Option<GreyImage<f32>> {
    if !sigma.is_finite() || sigma < 0.0 {
        return None;
    }
    if sigma < 2.0 {
        let kernel = sampled_gaussian(sigma)?;
        filter_separable(image, &kernel, &kernel)
    } else {
        let widths = box_widths(sigma)?;
        Some(map_lines(
            image,
            |line| cascade(line, &widths),
            |line| cascade(line, &widths),
        ))
    }
}

/// One box blur of radius `radius` px (width `2·radius + 1`) along each axis.
pub fn box_blur(image: &GreyImage<f32>, radius: u32) -> GreyImage<f32> {
    let radius = radius as usize;
    map_lines(
        image,
        |line| box_mean(line, radius),
        |line| box_mean(line, radius),
    )
}

/// Correlates the image with `kernel_x` along each row, then with `kernel_y`
/// down each column. Output `i` of a line is `Σ_k kernel[k] · line[i + k − r]`
/// with `r = (len − 1) / 2`, and NaN when any value it reads is not finite.
/// `None` unless both kernels have odd, non-zero lengths.
pub fn filter_separable(
    image: &GreyImage<f32>,
    kernel_x: &[f64],
    kernel_y: &[f64],
) -> Option<GreyImage<f32>> {
    if !is_odd_length(kernel_x) || !is_odd_length(kernel_y) {
        return None;
    }
    Some(map_lines(
        image,
        |line| correlate(line, kernel_x),
        |line| correlate(line, kernel_y),
    ))
}

/// The longest radius a sampled kernel may have, in px: the largest film
/// side. No grid in the engine has a line longer than that, so a wider
/// kernel would only read the same mirrored values again, and the cap keeps
/// a kernel to at most 32769 weights (256 KiB).
pub const MAX_KERNEL_RADIUS: u32 = shapes::film::MAX_FILM_SIDE;

/// The sampled Gaussian `exp(−j² / 2σ²)` for `j` from −r to r, with
/// `r = ceil(3σ)` and the weights summing to 1. Sigma 0 gives the identity
/// kernel `[1]`. `None` when sigma is negative or not finite, positive but
/// so small (under about 1.1e-162) that `2σ²` is zero, where every weight
/// would divide by zero, or so large (over about 5461 px) that `r` would
/// pass [`MAX_KERNEL_RADIUS`].
pub fn sampled_gaussian(sigma: f64) -> Option<Vec<f64>> {
    if !sigma.is_finite() || sigma < 0.0 {
        return None;
    }
    if sigma == 0.0 {
        return Some(vec![1.0]);
    }
    let two_sigma_sq = 2.0 * sigma * sigma;
    if two_sigma_sq == 0.0 {
        return None;
    }
    let reach = (3.0 * sigma).ceil();
    if reach > f64::from(MAX_KERNEL_RADIUS) {
        return None;
    }
    let radius = reach as i64;
    let weights: Vec<f64> = (-radius..=radius)
        .map(|j| libm::exp(-((j * j) as f64) / two_sigma_sq))
        .collect();
    let total: f64 = weights.iter().sum();
    Some(weights.into_iter().map(|w| w / total).collect())
}

/// The odd widths of the three box passes that stand in for a Gaussian of
/// `sigma` px, by evidence spec 4.3. The cascade's variance is the sum of
/// the three boxes' `(w² − 1) / 12`. With the pass count `m` unrounded it
/// would be exactly `sigma²`; rounding `m` by `δ` changes it by
/// `−δ (wl + 1) / 3`, where `wl` is the smaller width (about `2σ − 1`), so
/// the error is at most `(wl + 1) / 6`, about `σ / 3` px².
///
/// At every whole sigma the unrounded `m` is exactly 1.5 and rounds up to 2,
/// so the cascade has variance `σ² − σ/3`, the most the rule allows, and is
/// narrower than the Gaussian: its standard deviation is 8.7% short at
/// σ = 2, 5.7% at 3, 3.4% at 5, 1.7% at 10 and 0.8% at 20. Below σ = 2 the
/// sampled kernel is used instead, so the error steps from under 1% at
/// σ = 1.99 to 8.7% at σ = 2.
///
/// `None` when sigma is negative or not finite, or when the larger width
/// would not fit in 32 bits.
pub fn box_widths(sigma: f64) -> Option<[usize; 3]> {
    let ideal = (4.0 * sigma * sigma + 1.0).sqrt();
    if !(sigma >= 0.0 && ideal < f64::from(u32::MAX) - 2.0) {
        return None;
    }
    let mut low = ideal.floor() as i64;
    if low % 2 == 0 {
        low -= 1;
    }
    let high = low + 2;
    let (low_f, sigma_sq) = (low as f64, sigma * sigma);
    let passes = ((12.0 * sigma_sq - 3.0 * low_f * low_f - 12.0 * low_f - 9.0)
        / (-4.0 * low_f - 4.0))
        .round()
        .clamp(0.0, 3.0) as usize;
    let mut widths = [high as usize; 3];
    for width in widths.iter_mut().take(passes) {
        *width = low as usize;
    }
    Some(widths)
}

fn is_odd_length(kernel: &[f64]) -> bool {
    !kernel.is_empty() && kernel.len() % 2 == 1
}

/// Runs the three box passes of `widths` one after another on a line.
fn cascade(line: &[f64], widths: &[usize; 3]) -> Vec<f64> {
    widths.iter().fold(line.to_vec(), |current, &width| {
        box_mean(&current, (width - 1) / 2)
    })
}

/// The mean of each value and the `radius` values either side of it, read
/// past the edges by the mirror rule. Running sum, so the cost is the length
/// of the line plus one box width. The sum holds only finite values, and a
/// count of the others in the window makes those means NaN, so a bad value
/// spoils only the windows that hold it.
fn box_mean(line: &[f64], radius: usize) -> Vec<f64> {
    let len = line.len();
    if len == 0 {
        return Vec::new();
    }
    let radius = radius as i64;
    let width = (2 * radius + 1) as f64;
    let at = |index: i64| line[reflect_index(index, len as u32).unwrap_or(0)];
    let mut sum = 0.0f64;
    let mut not_finite = 0i64;
    for value in (-radius..=radius).map(at) {
        if value.is_finite() {
            sum += value;
        } else {
            not_finite += 1;
        }
    }
    let mut means = Vec::with_capacity(len);
    for i in 0..len as i64 {
        means.push(if not_finite > 0 {
            f64::NAN
        } else {
            sum / width
        });
        let (entering, leaving) = (at(i + 1 + radius), at(i - radius));
        if entering.is_finite() {
            sum += entering;
        } else {
            not_finite += 1;
        }
        if leaving.is_finite() {
            sum -= leaving;
        } else {
            not_finite -= 1;
        }
    }
    means
}

/// Correlates one line with a kernel of odd length, reading past the edges
/// by the mirror rule. A value that is not finite is read as NaN, so every
/// output whose window holds it is NaN, whatever the kernel's signs.
fn correlate(line: &[f64], kernel: &[f64]) -> Vec<f64> {
    let len = line.len();
    if len == 0 {
        return Vec::new();
    }
    let radius = (kernel.len() as i64 - 1) / 2;
    let at = |index: i64| {
        let value = line[reflect_index(index, len as u32).unwrap_or(0)];
        if value.is_finite() { value } else { f64::NAN }
    };
    (0..len as i64)
        .map(|i| {
            kernel
                .iter()
                .enumerate()
                .map(|(k, &weight)| weight * at(i + k as i64 - radius))
                .sum::<f64>()
        })
        .collect()
}

/// Applies `along_x` to every row and then `along_y` to every column, in f64,
/// and returns the result on the same placement. An empty image stays empty.
fn map_lines(
    image: &GreyImage<f32>,
    along_x: impl Fn(&[f64]) -> Vec<f64>,
    along_y: impl Fn(&[f64]) -> Vec<f64>,
) -> GreyImage<f32> {
    let (width, height) = (image.width() as usize, image.height() as usize);
    let mut plane: Vec<f64> = image.pixels().iter().map(|&v| f64::from(v)).collect();
    if width > 0 && height > 0 {
        for row in plane.chunks_mut(width) {
            let filtered = along_x(row);
            row.copy_from_slice(&filtered);
        }
        let mut column = vec![0.0; height];
        for x in 0..width {
            for (y, value) in column.iter_mut().enumerate() {
                *value = plane[y * width + x];
            }
            let filtered = along_y(&column);
            for (y, value) in filtered.into_iter().enumerate() {
                plane[y * width + x] = value;
            }
        }
    }
    let pixels: Vec<f32> = plane.into_iter().map(|v| v as f32).collect();
    GreyImage::new(image.width(), image.height(), pixels, image.placement())
        .expect("the pixel count is unchanged")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grey_image::Placement;

    fn impulse(width: u32, height: u32, x: u32, y: u32) -> GreyImage<f32> {
        let mut pixels = vec![0.0f32; (width * height) as usize];
        pixels[(y * width + x) as usize] = 1.0;
        GreyImage::new(width, height, pixels, Placement::FILM).expect("sizes match")
    }

    fn row(image: &GreyImage<f32>, y: u32) -> Vec<f64> {
        (0..image.width())
            .map(|x| f64::from(*image.get(x, y).expect("inside")))
            .collect()
    }

    #[test]
    fn a_constant_image_stays_constant_under_every_blur() {
        // The sampled weights sum to 1 within f64 rounding and the box sums
        // of 4 are exact, so the only error is storing the result as f32:
        // half an ulp of 4 is 2.4e-7, so 1e-6.
        let flat = GreyImage::new(9, 7, vec![4.0f32; 63], Placement::FILM).expect("sizes");
        for sigma in [0.0, 1.0, 1.5, 3.0, 4.0] {
            let blurred = gaussian(&flat, sigma).expect("sigma is valid");
            for value in blurred.pixels() {
                assert!((value - 4.0).abs() < 1e-6, "sigma {sigma} gave {value}");
            }
        }
        for value in box_blur(&flat, 2).pixels() {
            assert!((value - 4.0).abs() < 1e-6);
        }
    }

    #[test]
    fn a_radius_one_box_spreads_an_impulse_over_its_nine_pixels() {
        // Each of the nine pixels around the impulse gets 1/9 of it, and nothing
        // else does. 1/9 stored in f32 is within 4e-9 of 1/9, so 1e-7.
        let spot = impulse(5, 5, 2, 2);
        let blurred = box_blur(&spot, 1);
        for y in 0..5u32 {
            for x in 0..5u32 {
                let expected = if (1..=3).contains(&x) && (1..=3).contains(&y) {
                    1.0 / 9.0
                } else {
                    0.0
                };
                let value = f64::from(*blurred.get(x, y).expect("inside"));
                assert!((value - expected).abs() < 1e-7, "({x}, {y}) is {value}");
            }
        }
    }

    #[test]
    fn a_blur_keeps_the_mass_of_an_impulse() {
        // Each output is stored in f32 with a relative error under 6e-8, so
        // the stored values together are within 6e-8 of the true mass of 1;
        // 1e-6 leaves room for the f64 running sums.
        let spot = impulse(41, 41, 20, 20);
        let blurred = gaussian(&spot, 3.0).expect("sigma is valid");
        let mass: f64 = blurred.pixels().iter().map(|&v| f64::from(v)).sum();
        assert!((mass - 1.0).abs() < 1e-6, "mass {mass}");
    }

    #[test]
    fn the_box_cascade_for_sigma_three_has_the_variance_of_its_three_boxes() {
        // Widths 5, 5, 7 (see box_widths_follow_the_wells_rule): a box of width w
        // has variance (w² − 1) / 12, so the cascade has 2 + 2 + 4 = 8 px². The
        // Gaussian it stands in for has 9 px²: the rounding of m from 1.5 to 2
        // costs (wl + 1) / 6 = 1 px². The profile is stored in f32 (relative
        // error 6e-8), and offsets reach 7 px, so the variance is good to
        // about 1e-6; 1e-5 is the bound.
        let spot = impulse(41, 41, 20, 20);
        let blurred = gaussian(&spot, 3.0).expect("sigma is valid");
        let variance = spread(&row(&blurred, 20), 20.0);
        assert!((variance - 8.0).abs() < 1e-5, "variance {variance}");
    }

    #[test]
    fn at_sigma_two_the_cascade_falls_short_by_a_third_of_sigma() {
        // sigma 2: ideal width √17 = 4.12, widths 3 and 5, and
        // m = (48 − 27 − 36 − 9) / −16 = 1.5, rounded to 2: boxes 3, 3, 5,
        // with variances 8/12 + 8/12 + 24/12 = 10/3 px² = σ² − σ/3. Its
        // standard deviation, √(10/3) = 1.826, is 8.7% short of 2. Tolerance
        // as for sigma 3.
        assert_eq!(box_widths(2.0), Some([3, 3, 5]));
        let spot = impulse(41, 41, 20, 20);
        let blurred = gaussian(&spot, 2.0).expect("sigma is valid");
        let variance = spread(&row(&blurred, 20), 20.0);
        assert!((variance - 10.0 / 3.0).abs() < 1e-5, "variance {variance}");
    }

    /// The variance of a profile whose mass sits at `centre` by symmetry.
    fn spread(profile: &[f64], centre: f64) -> f64 {
        let mass: f64 = profile.iter().sum();
        profile
            .iter()
            .enumerate()
            .map(|(x, v)| {
                let d = x as f64 - centre;
                d * d * v
            })
            .sum::<f64>()
            / mass
    }

    #[test]
    fn box_widths_follow_the_wells_rule() {
        // sigma 3: ideal width sqrt(37) = 6.08, so the odd widths are 5 and 7;
        // m = round((108 − 75 − 60 − 9) / −24) = round(1.5) = 2 passes of 5.
        assert_eq!(box_widths(3.0), Some([5, 5, 7]));
        // sigma 4: sqrt(65) = 8.06, widths 7 and 9; m = round(1.5) = 2.
        assert_eq!(box_widths(4.0), Some([7, 7, 9]));
        // sigma 0 is the identity: three widths of 1.
        assert_eq!(box_widths(0.0), Some([1, 1, 1]));
        // Sigmas with no widths that fit.
        assert_eq!(box_widths(f64::INFINITY), None);
        assert_eq!(box_widths(1e300), None);
        assert_eq!(box_widths(f64::NAN), None);
        assert_eq!(box_widths(-1.0), None);
    }

    #[test]
    fn a_sampled_gaussian_has_the_exp_ratio_and_unit_sum() {
        let kernel = sampled_gaussian(1.0).expect("sigma is valid");
        assert_eq!(kernel.len(), 7);
        // Seven weights divided by their own total: the sum is 1 to a few
        // f64 roundings (each about 1e-16), so 1e-12 has a wide margin and
        // still catches a kernel left unnormalised.
        let total: f64 = kernel.iter().sum();
        assert!((total - 1.0).abs() < 1e-12);
        // The neighbour of the centre is exp(−1/2) times the centre. Both
        // were divided by the same total, so the ratio carries only the
        // rounding of the two divisions and the exp, a few parts in 1e16;
        // 1e-12 sits far above that and far below the gap to any other
        // exponent.
        let centre = kernel[3];
        assert!((kernel[4] / centre - libm::exp(-0.5)).abs() < 1e-12);
        assert!((kernel[5] / centre - libm::exp(-2.0)).abs() < 1e-12);
    }

    #[test]
    fn two_cascades_add_their_variances() {
        // sigma 2.5 gives m = round(2.875) = 3, so three boxes of width 5: each
        // box has variance (25 − 1) / 12 = 2 px². Two such blurs make six boxes,
        // so the variance is 12 px². The Gaussian of sigma 2.5 twice would give
        // 2 · 6.25 = 12.5; the gap is the rounding of m in each blur. The first
        // blur is stored in f32 before the second, which adds a relative 6e-8
        // to the f32 profile's; 1e-5 covers both, as for sigma 3.
        assert_eq!(box_widths(2.5), Some([5, 5, 5]));
        let spot = impulse(81, 81, 40, 40);
        let once = gaussian(&spot, 2.5).expect("sigma is valid");
        let twice = gaussian(&once, 2.5).expect("sigma is valid");
        let variance = spread(&row(&twice, 40), 40.0);
        assert!((variance - 12.0).abs() < 1e-5, "variance {variance}");
    }

    #[test]
    fn the_edge_mirrors_without_repeating_the_edge_value() {
        // Box of radius 1 on [1, 2, 3]: at the left edge the window reads
        // (2, 1, 2); at the right edge it reads (2, 3, 2). The running sums
        // are whole numbers, held exactly, and the one division by 3 rounds
        // by at most a part in 1e16; 1e-12 sits far above that and far below
        // the 1/3 that repeating the edge value, (1, 1, 2), would cost.
        let means = box_mean(&[1.0, 2.0, 3.0], 1);
        assert!((means[0] - 5.0 / 3.0).abs() < 1e-12);
        assert!((means[1] - 2.0).abs() < 1e-12);
        assert!((means[2] - 7.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn a_kernel_must_have_an_odd_length() {
        let flat = impulse(3, 3, 1, 1);
        assert!(filter_separable(&flat, &[1.0, 1.0], &[1.0]).is_none());
        assert!(filter_separable(&flat, &[1.0], &[]).is_none());
        assert!(gaussian(&flat, -1.0).is_none());
        assert!(gaussian(&flat, f64::NAN).is_none());
        assert!(gaussian(&flat, 1e300).is_none());
    }

    #[test]
    fn a_value_that_is_not_finite_spoils_only_the_windows_that_hold_it() {
        // Radius 1 over [1, NaN, 1, 1, 1, 1]: the windows of outputs 0, 1
        // and 2 hold index 1 (output 0 by the mirror, reading index −1 as 1).
        // Outputs 3 to 5 hold only ones, so their means are exactly 1.
        let means = box_mean(&[1.0, f64::NAN, 1.0, 1.0, 1.0, 1.0], 1);
        assert!(means[..3].iter().all(|m| m.is_nan()), "{means:?}");
        assert_eq!(&means[3..], &[1.0, 1.0, 1.0]);
        let infinite = box_mean(&[1.0, 1.0, 1.0, 1.0, f64::INFINITY], 1);
        assert_eq!(&infinite[..3], &[1.0, 1.0, 1.0]);
        assert!(infinite[3].is_nan() && infinite[4].is_nan());
    }

    #[test]
    fn an_infinity_on_the_sampled_path_spoils_its_window_with_nan() {
        // Sigma 1 takes the sampled kernel of radius 3, so an infinity at
        // (10, 10) of a 21 × 21 image lies in the 7 × 7 window of the outputs
        // from (7, 7) to (13, 13), and in no other. Those are NaN, not
        // infinite, for either sign; every other output reads only zeros
        // and is exactly 0.
        for bad in [f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
            let mut pixels = vec![0.0f32; 21 * 21];
            pixels[10 * 21 + 10] = bad;
            let spot = GreyImage::new(21, 21, pixels, Placement::FILM).expect("sizes match");
            let blurred = gaussian(&spot, 1.0).expect("sigma is valid");
            for y in 0..21u32 {
                for x in 0..21u32 {
                    let value = *blurred.get(x, y).expect("inside");
                    if (7..=13).contains(&x) && (7..=13).contains(&y) {
                        assert!(value.is_nan(), "{bad} at ({x}, {y}) gave {value}");
                    } else {
                        assert_eq!(value, 0.0, "{bad} at ({x}, {y})");
                    }
                }
            }
        }
    }

    #[test]
    fn a_sigma_too_small_to_square_has_no_kernel() {
        // At 1e-170 the square is 1e-340, below the smallest f64, so it is
        // zero and every weight would be 0/0: refused like a negative sigma.
        assert_eq!(sampled_gaussian(1e-170), None);
        assert!(gaussian(&impulse(3, 3, 1, 1), 1e-170).is_none());
        // At 1e-161 the square is 1e-322, still above zero. The side weights
        // e^(−1/2σ²) underflow to exactly 0, leaving the identity.
        assert_eq!(sampled_gaussian(1e-161), Some(vec![0.0, 1.0, 0.0]));
        // The product is (2σ)·σ, which reaches the smallest subnormal,
        // 2^−1074, once 2σ² passes half of it: σ above √(2^−1076), about
        // 1.11e-162. Either side of that, 1.0e-162 is refused and 1.2e-162
        // is the identity.
        assert_eq!(sampled_gaussian(1.0e-162), None);
        assert_eq!(sampled_gaussian(1.2e-162), Some(vec![0.0, 1.0, 0.0]));
    }

    #[test]
    fn a_sigma_too_large_for_the_longest_kernel_has_none() {
        // MAX_KERNEL_RADIUS is 16384. At sigma 5461, 3σ = 16383 exactly, so
        // the kernel has 2 · 16383 + 1 = 32767 weights. At 5461.5, 3σ =
        // 16384.5 rounds up to 16385, one past the cap: refused, and so are
        // sigmas whose kernels would not fit in memory at all.
        assert_eq!(MAX_KERNEL_RADIUS, 16_384);
        assert_eq!(sampled_gaussian(5461.0).map(|k| k.len()), Some(32_767));
        assert_eq!(sampled_gaussian(5461.5), None);
        assert_eq!(sampled_gaussian(1e19), None);
        assert_eq!(sampled_gaussian(1e300), None);
    }

    #[test]
    fn a_shifting_kernel_moves_the_image_by_its_offset() {
        // Kernel [0, 0, 1]: output i reads input i + 1. The last output reads
        // one past the edge, which mirrors to input 2.
        let ramp = GreyImage::new(4, 1, vec![0.0f32, 1.0, 2.0, 3.0], Placement::FILM)
            .expect("sizes match");
        let moved = filter_separable(&ramp, &[0.0, 0.0, 1.0], &[1.0]).expect("odd kernels");
        assert_eq!(row(&moved, 0), vec![1.0, 2.0, 3.0, 2.0]);
    }
}
