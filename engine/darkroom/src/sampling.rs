//! Bilinear sampling at a point, along a curve and along the curve's normals.
//!
//! Positions are pixel coordinates of the grid being sampled: the centre of
//! pixel (0, 0) is (0, 0), x runs right and y runs down. A position between
//! the centres of the outermost pixels, those centres included, is read from
//! the four pixels around it; a position outside is not read at all and
//! gives `None`, so a profile that runs off the grid says so instead of
//! inventing values. That includes the outer half pixel of the grid, between
//! the edge pixels' centres and the grid's own edge: a tap there is dropped,
//! not extrapolated or renormalised.
//!
//! Any grid whose values widen to f64 can be sampled, so a decoded 16-bit
//! film is read as it is.

use crate::grey_image::GreyImage;

/// The value at (x, y) by bilinear interpolation of the four pixels around
/// it. `None` when (x, y) lies outside the grid.
pub fn bilinear<T: Copy + Into<f64>>(image: &GreyImage<T>, x: f64, y: f64) -> Option<f32> {
    let (width, height) = (image.width(), image.height());
    if width == 0 || height == 0 {
        return None;
    }
    let inside = x >= 0.0 && y >= 0.0 && x <= f64::from(width - 1) && y <= f64::from(height - 1);
    if !inside {
        return None;
    }
    let (left, top) = (x.floor(), y.floor());
    let (left_index, top_index) = (left as u32, top as u32);
    let right_index = (left_index + 1).min(width - 1);
    let bottom_index = (top_index + 1).min(height - 1);
    let (across, down) = (x - left, y - top);
    let at = |column: u32, row: u32| -> f64 {
        (*image.get(column, row).expect("inside the grid")).into()
    };
    let upper = at(left_index, top_index) * (1.0 - across) + at(right_index, top_index) * across;
    let lower =
        at(left_index, bottom_index) * (1.0 - across) + at(right_index, bottom_index) * across;
    Some((upper * (1.0 - down) + lower * down) as f32)
}

/// The sample at each point of a curve, in order.
pub fn along_curve<T: Copy + Into<f64>>(
    image: &GreyImage<T>,
    points: &[(f64, f64)],
) -> Vec<Option<f32>> {
    points.iter().map(|&(x, y)| bilinear(image, x, y)).collect()
}

/// The samples along the normal through `centre`: offset `t` is the position
/// `centre + t · normal`, so `normal` should be a unit vector for the offsets
/// to be distances in pixels.
pub fn normal_profile<T: Copy + Into<f64>>(
    image: &GreyImage<T>,
    centre: (f64, f64),
    normal: (f64, f64),
    offsets: &[f64],
) -> Vec<Option<f32>> {
    offsets
        .iter()
        .map(|&t| bilinear(image, centre.0 + t * normal.0, centre.1 + t * normal.1))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grey_image::Placement;

    /// A grid whose value at (x, y) is `value(x, y)`.
    fn grid(width: u32, height: u32, value: impl Fn(f64, f64) -> f64) -> GreyImage<f32> {
        let mut pixels = Vec::with_capacity(width as usize * height as usize);
        for y in 0..height {
            for x in 0..width {
                pixels.push(value(f64::from(x), f64::from(y)) as f32);
            }
        }
        GreyImage::new(width, height, pixels, Placement::FILM).expect("sizes match")
    }

    #[test]
    fn bilinear_is_exact_on_a_linear_ramp() {
        // I = 2x + 3y + 1 is linear, so bilinear interpolation returns it exactly:
        // at (1.25, 2.5) it is 2.5 + 7.5 + 1 = 11.
        let ramp = grid(5, 5, |x, y| 2.0 * x + 3.0 * y + 1.0);
        assert_eq!(bilinear(&ramp, 1.25, 2.5), Some(11.0));
        assert_eq!(bilinear(&ramp, 4.0, 4.0), Some(21.0));
        assert_eq!(bilinear(&ramp, 0.0, 0.0), Some(1.0));
    }

    #[test]
    fn a_16_bit_film_is_read_as_it_is() {
        // Codes 1000 and 3000 side by side: a quarter of the way across reads
        // 1500, exactly.
        let film = GreyImage::new(2, 1, vec![1000u16, 3000], Placement::FILM).expect("sizes");
        assert_eq!(bilinear(&film, 0.25, 0.0), Some(1500.0));
        // The outer half pixel is off the sampled area.
        assert_eq!(bilinear(&film, -0.25, 0.0), None);
    }

    #[test]
    fn a_pixel_centre_returns_its_own_value() {
        let image = grid(3, 2, |x, y| 10.0 * y + x);
        assert_eq!(bilinear(&image, 2.0, 1.0), Some(12.0));
    }

    #[test]
    fn positions_outside_the_grid_give_nothing() {
        let image = grid(5, 5, |x, _| x);
        assert_eq!(bilinear(&image, -0.001, 2.0), None);
        assert_eq!(bilinear(&image, 4.0001, 2.0), None);
        assert_eq!(bilinear(&image, 2.0, f64::NAN), None);
        let empty = grid(0, 3, |_, _| 0.0);
        assert_eq!(bilinear(&empty, 0.0, 0.0), None);
    }

    #[test]
    fn a_curve_is_sampled_point_by_point() {
        // On I = 2x + 3y + 1 along the row y = 2, the samples are 2x + 7.
        let ramp = grid(5, 5, |x, y| 2.0 * x + 3.0 * y + 1.0);
        let points = [(0.0, 2.0), (0.5, 2.0), (3.0, 2.0)];
        assert_eq!(
            along_curve(&ramp, &points),
            vec![Some(7.0), Some(8.0), Some(13.0)]
        );
    }

    #[test]
    fn a_normal_profile_reads_across_the_curve() {
        // I = 3y. Crossing a horizontal curve at (2, 2) along the normal (0, 1),
        // the offsets −1, −1/2, 0, 1/2, 1 give y = 1, 1.5, 2, 2.5, 3 and so
        // values 3, 4.5, 6, 7.5, 9.
        let ramp = grid(5, 5, |_, y| 3.0 * y);
        let profile = normal_profile(&ramp, (2.0, 2.0), (0.0, 1.0), &[-1.0, -0.5, 0.0, 0.5, 1.0]);
        assert_eq!(
            profile,
            vec![Some(3.0), Some(4.5), Some(6.0), Some(7.5), Some(9.0)]
        );
    }

    #[test]
    fn a_profile_that_runs_off_the_grid_says_so() {
        let ramp = grid(5, 5, |x, _| x);
        // Starting at x = 0 and going left (normal (−1, 0)), offset 1 is off the grid.
        let profile = normal_profile(&ramp, (0.0, 2.0), (-1.0, 0.0), &[0.0, 1.0]);
        assert_eq!(profile, vec![Some(0.0), None]);
    }
}
