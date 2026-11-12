//! The square of full-size film cut out around one outline.
//!
//! The square is turned to lie along the outline's width, centred on its
//! corners' mean, and 1.7 widths across; it is sampled to 128 by 128 by
//! the Python imaging library's affine bilinear transform, which the
//! corner network was taught on. That transform is reproduced to the bit:
//! its coordinate is one fused multiply-add (the library's C is compiled
//! that way on the machines it was taught on), the two blends are plain,
//! the value is cut down to a byte rather than rounded, and anything off
//! the picture is black. There is no smoothing first, so a wide outline's
//! patch is aliased, and the network expects that too.
//!
//! Positions here are in pixels of the picture measured from its outer
//! top-left corner, as the network's fractions times the picture's size
//! give them.

use crate::outline::{Point, Quad};
use darkroom::grey_image::GreyImage;

/// The patch's side in pixels.
pub const PATCH_SIDE: usize = 128;

/// The patch's side over the outline's width.
const EXTENT: f64 = 1.7;

/// One outline's patch and where it sits.
#[derive(Clone, Debug, PartialEq)]
pub struct Patch {
    /// 128 by 128, row-major.
    pub pixels: Vec<u8>,
    /// The outline's corners in patch units: across and down the patch
    /// from its centre, as shares of its side, so about -0.5 to 0.5.
    pub initial: [[f32; 2]; 4],
    /// The patch's centre on the picture.
    pub centre: Point,
    /// The turn, as rows: a patch step of (across, down) moves the picture
    /// point by `(axes[0][0] * across + axes[0][1] * down, axes[1][0] *
    /// across + axes[1][1] * down)`. Its first column is the unit vector
    /// along the outline's width.
    pub axes: [[f64; 2]; 2],
    /// The patch's side in picture pixels.
    pub extent: f64,
}

/// The patch around `quad`, cut from `picture`.
pub fn cut(picture: &GreyImage<u8>, quad_in_film_pixels: &Quad) -> Patch {
    let q = quad_in_film_pixels;
    let direction = [0, 1].map(|k| (((q[1][k] - q[0][k]) + q[2][k]) - q[3][k]) / 2.0);
    let length = (direction[0] * direction[0] + direction[1] * direction[1]).sqrt();
    let width = length.max(4.0);
    let mut along = direction.map(|v| v / length.max(1e-8));
    if (along[0] * along[0] + along[1] * along[1]).sqrt() < 0.5 {
        along = [1.0, 0.0];
    }
    let axes = [[along[0], -along[1]], [along[1], along[0]]];
    let centre = [0, 1].map(|k| (((q[0][k] + q[1][k]) + q[2][k]) + q[3][k]) / 4.0);
    let extent = width * EXTENT;

    let half = extent / 2.0;
    let origin = [0, 1].map(|row| centre[row] - (axes[row][0] * half + axes[row][1] * half));
    let scale = extent / PATCH_SIDE as f64;
    let affine = [
        axes[0][0] * scale,
        axes[0][1] * scale,
        origin[0],
        axes[1][0] * scale,
        axes[1][1] * scale,
        origin[1],
    ];
    let pixels = sample(picture, affine);

    let initial = q.map(|corner| {
        let d = [corner[0] - centre[0], corner[1] - centre[1]];
        [0, 1].map(|column| ((d[0] * axes[0][column] + d[1] * axes[1][column]) / extent) as f32)
    });
    Patch {
        pixels,
        initial,
        centre,
        axes,
        extent,
    }
}

/// The library's affine transform with bilinear sampling, to 128 by 128.
fn sample(picture: &GreyImage<u8>, affine: [f64; 6]) -> Vec<u8> {
    let [a0, a1, a2, a3, a4, a5] = affine;
    let (width, height) = (picture.width() as i64, picture.height() as i64);
    let (w, h) = (width as f64, height as f64);
    let read = |x: i64, y: i64| -> f64 {
        let x = x.clamp(0, width - 1);
        let y = y.clamp(0, height - 1);
        f64::from(picture.pixels()[(y * width + x) as usize])
    };
    let mut pixels = Vec::with_capacity(PATCH_SIDE * PATCH_SIDE);
    for row in 0..PATCH_SIDE {
        for column in 0..PATCH_SIDE {
            let across = column as f64 + 0.5;
            let down = row as f64 + 0.5;
            let x = libm::fma(a0, across, a1 * down) + a2;
            let y = libm::fma(a3, across, a4 * down) + a5;
            if !(x >= 0.0 && x < w && y >= 0.0 && y < h) {
                pixels.push(0);
                continue;
            }
            let (x, y) = (x - 0.5, y - 0.5);
            let (x0, y0) = (x.floor(), y.floor());
            let (dx, dy) = (x - x0, y - y0);
            let (x0, y0) = (x0 as i64, y0 as i64);
            let upper = read(x0, y0) + (read(x0 + 1, y0) - read(x0, y0)) * dx;
            let lower = read(x0, y0 + 1) + (read(x0 + 1, y0 + 1) - read(x0, y0 + 1)) * dx;
            let value = upper + (lower - upper) * dy;
            // Cut down, not rounded.
            pixels.push(value as u8);
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkroom::grey_image::Placement;

    #[test]
    fn a_level_outline_cuts_a_level_patch_around_its_centre() {
        // A square outline 20 pixels wide, centred at (50, 40), on a picture
        // whose value is its column: the patch runs 34 pixels across, so
        // its middle columns read about 50 and it rises left to right.
        let pixels = (0..100 * 80).map(|i| (i % 100) as u8).collect();
        let picture = GreyImage::new(100, 80, pixels, Placement::FILM).unwrap();
        let quad = [[40.0, 30.0], [60.0, 30.0], [60.0, 50.0], [40.0, 50.0]];
        let patch = cut(&picture, &quad);
        assert_eq!(patch.centre, [50.0, 40.0]);
        assert_eq!(patch.axes, [[1.0, -0.0], [0.0, 1.0]]);
        assert!((patch.extent - 34.0).abs() < 1e-12);
        let middle = patch.pixels[64 * PATCH_SIDE + 64];
        assert!((49..=50).contains(&middle));
        assert!(patch.pixels[64 * PATCH_SIDE] < patch.pixels[64 * PATCH_SIDE + 127]);
        let expected = 10.0 / 34.0;
        assert!((f64::from(patch.initial[0][0]) + expected).abs() < 1e-6);
        assert!((f64::from(patch.initial[2][1]) - expected).abs() < 1e-6);
    }

    #[test]
    fn outside_the_picture_is_black() {
        let picture = GreyImage::new(10, 10, vec![200; 100], Placement::FILM).unwrap();
        let quad = [[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0]];
        let patch = cut(&picture, &quad);
        assert_eq!(patch.pixels[0], 0);
        assert_eq!(patch.pixels[64 * PATCH_SIDE + 64], 200);
    }
}
