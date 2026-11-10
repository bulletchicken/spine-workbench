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

    /// The value at (x, y), or `None` outside the image.
    pub fn get(&self, x: u32, y: u32) -> Option<&T> {
        if x < self.width && y < self.height {
            self.pixels
                .get(y as usize * self.width as usize + x as usize)
        } else {
            None
        }
    }

    /// The value at (x, y), read with the edge rule of [`reflect_index`].
    /// `None` only when the image is empty.
    pub fn reflected(&self, x: i64, y: i64) -> Option<&T> {
        let column = reflect_index(x, self.width)?;
        let row = reflect_index(y, self.height)?;
        self.pixels.get(row * self.width as usize + column)
    }

    /// The film point of the image position (x, y); x and y may be
    /// fractional.
    pub fn film_point(&self, x: f64, y: f64) -> Px {
        self.placement.film_point(x, y)
    }

    /// The image position of a film point, when the placement can be
    /// inverted.
    pub fn image_point(&self, point: Px) -> Option<(f64, f64)> {
        self.placement.image_point(point)
    }

    /// The part of the image with top-left (left, top) and the given size,
    /// placed exactly where it sits on the film. `None` when it does not
    /// lie inside the image.
    pub fn crop(&self, left: u32, top: u32, width: u32, height: u32) -> Option<GreyImage<T>>
    where
        T: Clone,
    {
        let right = left.checked_add(width)?;
        let bottom = top.checked_add(height)?;
        if right > self.width || bottom > self.height {
            return None;
        }
        let row_length = self.width as usize;
        let mut pixels = Vec::with_capacity(width as usize * height as usize);
        for y in top..bottom {
            let start = y as usize * row_length + left as usize;
            pixels.extend_from_slice(&self.pixels[start..start + width as usize]);
        }
        let placement = Placement {
            origin: self.placement.film_point(f64::from(left), f64::from(top)),
            ..self.placement
        };
        GreyImage::new(width, height, pixels, placement)
    }

    /// The same grid with every value passed through `convert`, on the same
    /// placement.
    pub fn map<U>(&self, convert: impl Fn(&T) -> U) -> GreyImage<U> {
        GreyImage {
            width: self.width,
            height: self.height,
            pixels: self.pixels.iter().map(convert).collect(),
            placement: self.placement,
        }
    }
}

impl Placement {
    /// The film point of the image position (u, v), where u and v may be
    /// fractional: `origin + u · stepAcross + v · stepDown`.
    pub fn film_point(&self, u: f64, v: f64) -> Px {
        Px::new(
            self.origin.x() + u * self.step_across.x() + v * self.step_down.x(),
            self.origin.y() + u * self.step_across.y() + v * self.step_down.y(),
        )
    }

    /// The image position (u, v) of a film point: the inverse of
    /// [`Placement::film_point`]. `None` when the two steps do not span the
    /// plane.
    pub fn image_point(&self, point: Px) -> Option<(f64, f64)> {
        let (ax, ay) = (self.step_across.x(), self.step_across.y());
        let (bx, by) = (self.step_down.x(), self.step_down.y());
        let determinant = ax * by - bx * ay;
        if !determinant.is_finite() || determinant == 0.0 {
            return None;
        }
        let dx = point.x() - self.origin.x();
        let dy = point.y() - self.origin.y();
        Some((
            (by * dx - bx * dy) / determinant,
            (ax * dy - ay * dx) / determinant,
        ))
    }
}

/// The index an out-of-range position reads, on a row or column of `len`
/// values: the edge value is mirrored about its own pixel and not repeated,
/// so −1 reads 1 and `len` reads `len − 2`. This is the edge rule for every
/// filter and every padding in the darkroom. `None` when `len` is 0.
pub fn reflect_index(index: i64, len: u32) -> Option<usize> {
    match len {
        0 => None,
        1 => Some(0),
        n => {
            let period = 2 * i64::from(n - 1);
            let m = index.rem_euclid(period);
            let mirrored = if m < i64::from(n) { m } else { period - m };
            usize::try_from(mirrored).ok()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn numbered(width: u32, height: u32) -> GreyImage<f32> {
        let pixels = (0..width * height).map(|i| i as f32).collect();
        GreyImage::new(width, height, pixels, Placement::FILM).expect("sizes match")
    }

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

    #[test]
    fn a_crop_keeps_its_pixels_and_lands_on_the_same_film_point() {
        let image = numbered(4, 4);
        let crop = image.crop(1, 2, 2, 2).expect("the crop lies inside");
        // Crop pixel (0, 0) is image pixel (1, 2), which holds 2 * 4 + 1.
        assert_eq!(crop.pixels(), &[9.0, 10.0, 13.0, 14.0]);
        assert_eq!(crop.film_point(0.0, 0.0), Px::new(1.0, 2.0));
        assert_eq!(crop.film_point(1.0, 1.0), image.film_point(2.0, 3.0));
        assert!(image.crop(3, 0, 2, 1).is_none());
        assert!(image.crop(0, 0, 4, 5).is_none());
    }

    #[test]
    fn a_position_maps_to_the_film_and_back_exactly() {
        // Half-size copy: image pixel (u, v) sits at film (0.5 + 2u, 0.5 + 2v).
        let half = Placement {
            origin: Px::new(0.5, 0.5),
            step_across: Px::new(2.0, 0.0),
            step_down: Px::new(0.0, 2.0),
        };
        assert_eq!(half.film_point(3.0, 1.5), Px::new(6.5, 3.5));
        assert_eq!(half.image_point(Px::new(6.5, 3.5)), Some((3.0, 1.5)));

        // Turned copy: step across points down the film, step down points left.
        let turned = Placement {
            origin: Px::new(10.0, 0.0),
            step_across: Px::new(0.0, 1.0),
            step_down: Px::new(-1.0, 0.0),
        };
        assert_eq!(turned.film_point(2.0, 3.0), Px::new(7.0, 2.0));
        // Every step and offset is a small whole number and the determinant
        // is 1, so each product, difference and the division are exact in
        // f64: the inverse is checked for equality, with no tolerance.
        assert_eq!(turned.image_point(Px::new(7.0, 2.0)), Some((2.0, 3.0)));

        let flat = Placement {
            origin: Px::new(0.0, 0.0),
            step_across: Px::new(1.0, 0.0),
            step_down: Px::new(2.0, 0.0),
        };
        assert_eq!(flat.image_point(Px::new(1.0, 1.0)), None);
    }

    #[test]
    fn reading_past_the_edge_mirrors_without_repeating_the_edge() {
        // Three values 0, 1, 2 mirror at both ends: -1 reads 1, -2 reads 2,
        // 3 reads 1, 4 reads 0 and 5 reads 1.
        let expected = [(-1, 1), (-2, 2), (3, 1), (4, 0), (5, 1)];
        for (index, want) in expected {
            assert_eq!(reflect_index(index, 3), Some(want), "index {index}");
        }
        assert_eq!(reflect_index(7, 1), Some(0));
        assert_eq!(reflect_index(0, 0), None);

        let image = numbered(3, 1);
        assert_eq!(image.reflected(-1, 0), Some(&1.0));
        assert_eq!(image.reflected(3, 0), Some(&1.0));
        assert_eq!(image.get(3, 0), None);
    }

    #[test]
    fn map_keeps_the_size_and_the_placement() {
        let image = numbered(2, 2);
        let mask = image.map(|v| *v > 1.5);
        assert_eq!(mask.pixels(), &[false, false, true, true]);
        assert_eq!(mask.placement(), image.placement());
    }
}
