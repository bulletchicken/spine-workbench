//! The normalised half-float copy of a film for the GPU, bone bright.
//!
//! Every stored code `p` is divided by `2^bits_stored - 1`, so the
//! normalised range is exactly **[0, 1]** before rounding to IEEE binary16.
//! The returned `u16`s are half-float bit patterns, not integer grey levels.
//! No modality, percentile stretch, window or level is applied here: the
//! viewer uploads an R16F texture and applies window and level in its shader.

use crate::decoded_film::DecodedFilm;
use shapes::errors::{EngineError, ErrorCode};

/// Make a separately owned, normalised half-float texture in reading order.
pub fn display_pixels(film: &DecodedFilm) -> Result<Vec<u16>, EngineError> {
    let bits = film.facts.bits_stored;
    if !(1..=16).contains(&bits) {
        return Err(EngineError::new(
            ErrorCode::FilmUnsupportedBits,
            "The stored bit depth is invalid.",
        ));
    }
    let maximum = (1u32 << bits) - 1;
    if film
        .image
        .pixels()
        .iter()
        .any(|&pixel| u32::from(pixel) > maximum)
    {
        return Err(crate::invalid_film());
    }
    let mut pixels = crate::allocate(film.image.pixels().len())?;
    pixels.extend(
        film.image
            .pixels()
            .iter()
            .map(|&pixel| half::f16::from_f64(f64::from(pixel) / f64::from(maximum)).to_bits()),
    );
    Ok(pixels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thumbnail::default_window;
    use darkroom::grey_image::{GreyImage, Placement};
    use shapes::film::{FilmFacts, FilmKind, PixelValues, SourceFormat};

    #[test]
    fn texture_codes_span_zero_to_one_without_a_window() {
        for bits in [1, 8, 12, 16] {
            let maximum = ((1u32 << bits) - 1) as u16;
            let image =
                GreyImage::new(3, 1, vec![0, maximum / 2, maximum], Placement::FILM).unwrap();
            let facts = FilmFacts {
                kind: FilmKind::Front,
                width: 3,
                height: 1,
                bits_stored: bits,
                source: SourceFormat::Png,
                kept_tags: None,
                values: PixelValues::AS_STORED,
                flags: Vec::new(),
                default_window: default_window(&image, bits, &PixelValues::AS_STORED),
                exif_quarter_turns: None,
            };
            let texture = display_pixels(&DecodedFilm { image, facts }).unwrap();
            // Binary16 encodes exactly 0 as 0x0000 and 1 as 0x3c00.
            assert_eq!(texture[0], 0x0000);
            assert_eq!(texture[2], 0x3c00);
            let middle = f64::from(half::f16::from_bits(texture[1]).to_f32());
            // At most half a binary16 ulp on [0,1]: 2^-12 = 1/4096.
            assert!((middle - f64::from(maximum / 2) / f64::from(maximum)).abs() <= 1.0 / 4096.0);
        }
    }
}
