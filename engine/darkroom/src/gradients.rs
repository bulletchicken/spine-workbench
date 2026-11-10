//! Scharr gradients: magnitude and direction, the direction measured from +x
//! toward +y in (−π, π].
//!
//! The Scharr kernels are separable. The x gradient differences across a row
//! with `[−1, 0, 1]` and smooths down the column with `[3, 10, 3] / 32`; the y
//! gradient does the reverse. A plane `a·x + b·y` has gradient exactly
//! `(a, b)` away from the edge, and an edge is read with the mirror rule of
//! [`crate::grey_image::reflect_index`].

use crate::blur::filter_separable;
use crate::grey_image::GreyImage;
use core::f64::consts::PI;

const DIFFERENCE: [f64; 3] = [-1.0, 0.0, 1.0];
const SMOOTH: [f64; 3] = [3.0 / 32.0, 10.0 / 32.0, 3.0 / 32.0];

/// The x and y gradients of an image, on the image's placement. From
/// [`scharr`] they are Scharr's, in change per pixel; from
/// [`crate::hessian::normalised_gradient`] they are Gaussian derivatives
/// multiplied by sigma, in change per sigma.
#[derive(Clone, Debug, PartialEq)]
pub struct Gradients {
    /// Change toward +x.
    pub x: GreyImage<f32>,
    /// Change toward +y.
    pub y: GreyImage<f32>,
}

/// The Scharr gradients of `image`.
pub fn scharr(image: &GreyImage<f32>) -> Gradients {
    Gradients {
        x: filter_separable(image, &DIFFERENCE, &SMOOTH).expect("the Scharr kernels are odd"),
        y: filter_separable(image, &SMOOTH, &DIFFERENCE).expect("the Scharr kernels are odd"),
    }
}

impl Gradients {
    /// `√(x² + y²)` at every pixel.
    pub fn magnitude(&self) -> GreyImage<f32> {
        combine(&self.x, &self.y, |gx, gy| (gx * gx + gy * gy).sqrt())
    }

    /// The angle of the gradient from +x toward +y, in (−π, π]. A flat pixel
    /// has the angle 0.
    pub fn direction(&self) -> GreyImage<f32> {
        combine(&self.x, &self.y, |gx, gy| {
            let angle = libm::atan2(gy, gx);
            if angle <= -PI {
                angle + 2.0 * PI
            } else {
                angle
            }
        })
    }
}

/// Combines two grids of the same size value by value, on the first's
/// placement.
fn combine(
    a: &GreyImage<f32>,
    b: &GreyImage<f32>,
    join: impl Fn(f64, f64) -> f64,
) -> GreyImage<f32> {
    let pixels: Vec<f32> = a
        .pixels()
        .iter()
        .zip(b.pixels())
        .map(|(&p, &q)| join(f64::from(p), f64::from(q)) as f32)
        .collect();
    GreyImage::new(a.width(), a.height(), pixels, a.placement()).expect("the sizes match")
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

    /// The pixels that are at least one step from the edge, where the Scharr
    /// kernels read no mirrored value.
    fn interior(image: &GreyImage<f32>) -> Vec<f32> {
        let mut values = Vec::new();
        for y in 1..image.height() - 1 {
            for x in 1..image.width() - 1 {
                values.push(*image.get(x, y).expect("inside"));
            }
        }
        values
    }

    #[test]
    fn the_gradient_of_a_plane_is_its_slope_everywhere_inside() {
        // I = 2x − 3y has gradient (2, −3) at every interior pixel. The
        // difference spans two pixels and the smoothing weights sum to 16/32,
        // so each component is exactly the slope. The values are whole
        // numbers and the weights are sixteenths, all exact in binary, so
        // the result is exact.
        let image = plane(9, 9, |x, y| 2.0 * x - 3.0 * y);
        let gradients = scharr(&image);
        for value in interior(&gradients.x) {
            assert_eq!(value, 2.0, "x gradient");
        }
        for value in interior(&gradients.y) {
            assert_eq!(value, -3.0, "y gradient");
        }
    }

    #[test]
    fn an_impulse_shows_the_three_ten_three_weights() {
        // A unit impulse at (2, 2). Output (x, y) of the x gradient is
        // I(x + 1) − I(x − 1) along the row, smoothed down the column by
        // [3, 10, 3] / 32: so +10/32 just left of the impulse, −10/32 just
        // right, ±3/32 on the diagonals, and 0 elsewhere, the centre
        // included. The y gradient is the same turned a quarter turn. Every
        // weight is a multiple of 1/32, exact in binary, so the values are
        // exact; Sobel's [1, 2, 1] / 8 would give 8/32 and 2/32.
        let mut pixels = vec![0.0f32; 25];
        pixels[2 * 5 + 2] = 1.0;
        let spot = GreyImage::new(5, 5, pixels, Placement::FILM).expect("sizes match");
        let gradients = scharr(&spot);
        let mut expected_x = [[0.0f32; 5]; 5];
        for (dy, weight) in [(1usize, 3.0f32), (2, 10.0), (3, 3.0)] {
            expected_x[dy][1] = weight / 32.0;
            expected_x[dy][3] = -weight / 32.0;
        }
        for y in 0..5u32 {
            for x in 0..5u32 {
                let (row, column) = (y as usize, x as usize);
                assert_eq!(
                    *gradients.x.get(x, y).expect("inside"),
                    expected_x[row][column],
                    "x gradient at ({x}, {y})"
                );
                assert_eq!(
                    *gradients.y.get(x, y).expect("inside"),
                    expected_x[column][row],
                    "y gradient at ({x}, {y})"
                );
            }
        }
    }

    #[test]
    fn magnitude_and_direction_of_a_slope_are_hand_computed() {
        // Gradient (2, −3): magnitude √13 and direction atan2(−3, 2). Both
        // are worked in f64 and stored in f32, which rounds them by at most
        // half an ulp of 4 (2.4e-7), so 1e-6.
        let image = plane(9, 9, |x, y| 2.0 * x - 3.0 * y);
        let gradients = scharr(&image);
        let magnitude = interior(&gradients.magnitude());
        let direction = interior(&gradients.direction());
        let expected_magnitude = 13.0f64.sqrt();
        let expected_direction = libm::atan2(-3.0, 2.0);
        for (m, d) in magnitude.iter().zip(&direction) {
            assert!(
                (f64::from(*m) - expected_magnitude).abs() < 1e-6,
                "magnitude {m}"
            );
            assert!(
                (f64::from(*d) - expected_direction).abs() < 1e-6,
                "direction {d}"
            );
        }
    }

    #[test]
    fn a_slope_facing_minus_x_points_at_pi_not_minus_pi() {
        // I = −x has gradient (−1, 0). The angle is π, the top of the range
        // (−π, π]. π stored in f32 is within 1e-7 of π, so 1e-6.
        let image = plane(9, 9, |x, _| -x);
        let direction = interior(&scharr(&image).direction());
        for value in direction {
            assert!((f64::from(value) - PI).abs() < 1e-6, "direction {value}");
        }
    }

    #[test]
    fn a_negative_zero_y_component_still_reads_pi() {
        // atan2(−0, −1) is −π, outside (−π, π]; it must be folded to π. A
        // Scharr pass never makes −0 from a plane, so the pair is built here.
        let single = |value: f32| GreyImage::new(1, 1, vec![value], Placement::FILM).expect("one");
        let gradients = Gradients {
            x: single(-1.0),
            y: single(-0.0),
        };
        let direction = f64::from(gradients.direction().pixels()[0]);
        assert!((direction - PI).abs() < 1e-6, "direction {direction}");
    }

    #[test]
    fn a_flat_image_has_no_gradient_even_at_its_edges() {
        let flat = plane(5, 4, |_, _| 7.0);
        let gradients = scharr(&flat);
        assert!(gradients.magnitude().pixels().iter().all(|&m| m == 0.0));
        assert!(gradients.direction().pixels().iter().all(|&d| d == 0.0));
    }
}
