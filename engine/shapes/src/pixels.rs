//! Points and boxes on a film, and the pixel-space rule.
//!
//! # The pixel-space rule
//!
//! Every point the engine, the app or the case file holds is in **original
//! film pixels**: x to the right (column), y down (row), and the centre of
//! the top-left pixel is (0, 0). So pixel (i, j) covers the square from
//! (i − 0.5, j − 0.5) to (i + 0.5, j + 0.5).
//!
//! Films are never rotated, flipped or resized. Turning a film upright and
//! mirroring it are view transforms in the app; any stage that works on a
//! turned or resampled copy maps its results back to original film pixels
//! before they leave it.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A point in original film pixels: `[x, y]`, x right, y down, the centre
/// of the top-left pixel at (0, 0).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct Px(pub [f64; 2]);

impl Px {
    pub const fn new(x: f64, y: f64) -> Self {
        Px([x, y])
    }

    pub const fn x(self) -> f64 {
        self.0[0]
    }

    pub const fn y(self) -> f64 {
        self.0[1]
    }

    pub fn is_finite(self) -> bool {
        self.0[0].is_finite() && self.0[1].is_finite()
    }
}

/// An axis-aligned box in original film pixels, edges included.
///
/// Where a box stands for pixels (a blanked box, a mask), it covers every
/// pixel it reaches into: pixel (i, j) when the box enters the pixel's
/// square, not when it only meets its edge. See [`PxRect::covers_pixel`].
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PxRect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl PxRect {
    /// Finite, with `left <= right` and `top <= bottom`.
    pub fn is_well_formed(&self) -> bool {
        [self.left, self.top, self.right, self.bottom]
            .iter()
            .all(|v| v.is_finite())
            && self.left <= self.right
            && self.top <= self.bottom
    }

    /// Inside the film's outer edges: from (−0.5, −0.5) to
    /// (width − 0.5, height − 0.5).
    pub fn is_on_film(&self, width: u32, height: u32) -> bool {
        self.is_well_formed()
            && self.left >= -0.5
            && self.top >= -0.5
            && self.right <= f64::from(width) - 0.5
            && self.bottom <= f64::from(height) - 0.5
    }

    /// Whether the box reaches into pixel (i, j), whose square runs from
    /// (i − 0.5, j − 0.5) to (i + 0.5, j + 0.5): `left < i + 0.5`,
    /// `i − 0.5 < right`, and the same for rows.
    pub fn covers_pixel(&self, i: u32, j: u32) -> bool {
        let (x, y) = (f64::from(i), f64::from(j));
        self.left < x + 0.5 && x - 0.5 < self.right && self.top < y + 0.5 && y - 0.5 < self.bottom
    }
}

/// How far outside its film a stored point may lie: one film width
/// sideways, one film height up or down.
///
/// Points do leave the film legitimately. A vertebra cropped off the top
/// or bottom is still placed, marked inferred, from the chain's padding
/// (up to about 100 mm), and a corner on a vertebra at the edge can sit
/// just outside. A point further out than a whole film is a damaged value.
pub fn is_near_film(point: Px, width: u32, height: u32) -> bool {
    let (w, h) = (f64::from(width), f64::from(height));
    point.is_finite()
        && point.x() >= -w
        && point.x() <= 2.0 * w
        && point.y() >= -h
        && point.y() <= 2.0 * h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_one_film_away_is_still_near() {
        assert!(is_near_film(Px::new(-100.0, 0.0), 100, 300));
        assert!(is_near_film(Px::new(200.0, 600.0), 100, 300));
        assert!(!is_near_film(Px::new(200.1, 0.0), 100, 300));
        assert!(!is_near_film(Px::new(0.0, -300.5), 100, 300));
        assert!(!is_near_film(Px::new(f64::NAN, 0.0), 100, 300));
    }

    #[test]
    fn a_box_must_not_be_inside_out() {
        let good = PxRect {
            left: 1.0,
            top: 2.0,
            right: 1.0,
            bottom: 3.0,
        };
        assert!(good.is_well_formed());
        assert!(!PxRect { right: 0.0, ..good }.is_well_formed());
    }

    #[test]
    fn a_box_covers_the_pixels_it_reaches_into() {
        let rect = PxRect {
            left: 0.5,
            top: 1.2,
            right: 2.0,
            bottom: 1.2,
        };
        // Columns: 0.5 is pixel 0's right edge, so only pixels 1 and 2.
        assert!(!rect.covers_pixel(0, 1));
        assert!(rect.covers_pixel(1, 1));
        assert!(rect.covers_pixel(2, 1));
        assert!(!rect.covers_pixel(3, 1));
        // A box with no height still covers the row it lies in.
        assert!(!rect.covers_pixel(1, 0));
        assert!(!rect.covers_pixel(1, 2));
    }

    #[test]
    fn a_box_on_the_film_stays_inside_its_outer_edges() {
        let whole = PxRect {
            left: -0.5,
            top: -0.5,
            right: 99.5,
            bottom: 299.5,
        };
        assert!(whole.is_on_film(100, 300));
        assert!(
            !PxRect {
                right: 99.6,
                ..whole
            }
            .is_on_film(100, 300)
        );
        assert!(!PxRect { top: -0.6, ..whole }.is_on_film(100, 300));
    }
}
