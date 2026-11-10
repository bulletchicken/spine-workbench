//! `GreyImage<T>`: a grid of pixel values and where it sits on the original
//! film.
//!
//! An image is either a film as stored or a copy made from one: a crop, a
//! smaller working copy, a copy turned upright. Each carries its
//! [`Placement`], so a position found on any of them can be reported in
//! original film pixels (the pixel-space rule in `shapes::pixels`).
//!
//! The struct and the placement rule are a frozen contract.

use shapes::pixels::Px;

/// Where an image's pixels sit on the original film: the centre of image
/// pixel (x, y) is at the film point `origin + x · stepAcross + y ·
/// stepDown`.
///
/// The two steps are vectors in film pixels, so a crop (unit steps), a
/// shrunk copy (longer steps) and a turned copy (rotated steps) are all
/// placed exactly.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub origin: Px,
    pub step_across: Px,
    pub step_down: Px,
}

impl Placement {
    /// The film itself: image pixel (x, y) is film pixel (x, y).
    pub const FILM: Placement = Placement {
        origin: Px::new(0.0, 0.0),
        step_across: Px::new(1.0, 0.0),
        step_down: Px::new(0.0, 1.0),
    };
}

/// A rectangular grid of values, row-major, with its placement on the film.
/// Pixel (x, y) is at index `y · width + x`.
#[derive(Clone, Debug, PartialEq)]
pub struct GreyImage<T> {
    width: u32,
    height: u32,
    pixels: Vec<T>,
    placement: Placement,
}

impl<T> GreyImage<T> {
    /// `None` unless `pixels` holds exactly `width × height` values.
    pub fn new(width: u32, height: u32, pixels: Vec<T>, placement: Placement) -> Option<Self> {
        let expected = usize::try_from(u64::from(width) * u64::from(height)).ok()?;
        (pixels.len() == expected).then_some(GreyImage {
            width,
            height,
            pixels,
            placement,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Every value, row-major.
    pub fn pixels(&self) -> &[T] {
        &self.pixels
    }

    pub fn placement(&self) -> Placement {
        self.placement
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pixel_count_must_match_the_size() {
        assert!(GreyImage::new(3, 2, vec![0u16; 6], Placement::FILM).is_some());
        assert!(GreyImage::new(3, 2, vec![0u16; 5], Placement::FILM).is_none());
        assert!(GreyImage::new(0, 7, Vec::<f32>::new(), Placement::FILM).is_some());
    }

    #[test]
    fn an_image_keeps_its_placement() {
        let half_size = Placement {
            origin: Px::new(0.5, 0.5),
            step_across: Px::new(2.0, 0.0),
            step_down: Px::new(0.0, 2.0),
        };
        let image = GreyImage::new(2, 1, vec![1.0f32, 2.0], half_size).expect("sizes match");
        assert_eq!(image.placement(), half_size);
        assert_eq!(image.pixels(), &[1.0, 2.0]);
        assert_eq!((image.width(), image.height()), (2, 1));
    }
}
