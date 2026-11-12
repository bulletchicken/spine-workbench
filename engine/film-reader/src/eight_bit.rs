//! A film as 8-bit grey, the way it opens on screen.
//!
//! A film stored in 8 bits is handed over as stored. Any other film,
//! deeper or stored in fewer bits, goes through its modality map and
//! opening window, exactly as the slot preview reads it, and each level is
//! scaled to 0..255 and rounded.
//!
//! Such a film reopened from a saved case holds the user's brightness flip
//! in its stored values, and the modality map takes the flip out again to
//! find the file's own values; it is put back after the window, as the
//! slot preview does. A flip the user makes after the film was read is not
//! applied here: whoever reads the film through its case record applies
//! that, as every other pass does.

use crate::decoded_film::DecodedFilm;
use shapes::errors::EngineError;

/// The film as 8-bit grey, bone bright: one value per pixel, row-major,
/// at the film's own size and placement. A plain list, so a caller can go
/// on to change it in place rather than copy a whole film.
pub fn as_opened(film: &DecodedFilm) -> Result<Vec<u8>, EngineError> {
    let mut pixels = crate::allocate(film.image.pixels().len())?;
    if film.facts.bits_stored == 8 {
        pixels.extend(film.image.pixels().iter().map(|&value| value as u8));
    } else {
        // Every stored value is looked up once, so a large deep film costs
        // one table of at most 65,536 levels rather than a window per pixel.
        let view = crate::thumbnail::opening_view(film);
        let flipped_in_store = film.facts.values.inverted_by_user;
        let highest = (1usize << film.facts.bits_stored.clamp(1, 16)) - 1;
        let mut levels = crate::allocate(highest + 1)?;
        // The flip goes on after the rounding, as it does when a fresh
        // film is flipped through its record, so that a level halfway
        // between two greys lands on the same one both times.
        levels.extend((0..=highest).map(|value| {
            let grey = (view(value as f64) * 255.0).round() as u8;
            if flipped_in_store { 255 - grey } else { grey }
        }));
        pixels.extend(
            film.image
                .pixels()
                .iter()
                .map(|&value| levels[usize::from(value).min(highest)]),
        );
    }
    Ok(pixels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read_film;
    use crate::test_support::DicomBuilder;
    use darkroom::grey_image::{GreyImage, Placement};
    use shapes::film::{
        DisplayWindow, Film, FilmFacts, FilmImage, FilmKind, PixelValues, SourceFormat,
        WindowFunction, WindowSource,
    };
    use shapes::film_geometry::FilmGeometry;

    fn stored(width: u32, height: u32, bits: u8, pixels: Vec<u16>) -> DecodedFilm {
        let image = GreyImage::new(width, height, pixels, Placement::FILM).unwrap();
        DecodedFilm {
            facts: FilmFacts {
                kind: FilmKind::Front,
                width,
                height,
                bits_stored: bits,
                source: SourceFormat::Png,
                kept_tags: None,
                values: PixelValues::AS_STORED,
                flags: Vec::new(),
                default_window: crate::thumbnail::default_window(
                    &image,
                    bits,
                    &PixelValues::AS_STORED,
                ),
                exif_quarter_turns: None,
            },
            image,
        }
    }

    #[test]
    fn an_eight_bit_film_comes_back_as_stored() {
        // A narrow opening window would stretch these if it were applied.
        let film = stored(4, 1, 8, vec![0, 40, 41, 255]);
        assert_eq!(as_opened(&film).unwrap(), [0, 40, 41, 255]);
    }

    #[test]
    fn a_deep_film_opens_through_its_window() {
        // A file window of centre 1000.5 and width 201 runs, in the DICOM
        // linear form, from 900 (black) to 1100 (white); 1000 sits just
        // below the middle: (1000 - 1000) / 200 + 0.5 = 0.5, so 127.5,
        // which rounds away from zero to 128.
        let bytes = DicomBuilder::new(5, 1, 16, &[0, 900, 1000, 1100, 60_000])
            .text(0x28, 0x1050, *b"DS", "1000.5")
            .text(0x28, 0x1051, *b"DS", "201")
            .build();
        let film = read_film(&bytes, FilmKind::Front).unwrap();
        assert!(matches!(
            film.facts.default_window,
            DisplayWindow::CentreWidth {
                function: WindowFunction::Linear,
                from: WindowSource::File,
                ..
            }
        ));
        assert_eq!(as_opened(&film).unwrap(), [0, 0, 128, 255, 255]);
    }

    #[test]
    fn a_monochrome_one_film_stays_bone_bright() {
        // In the file, 0 is white and 4095 black; once opened, the brightest
        // stored value must still be the brightest grey.
        let bytes = DicomBuilder::new(3, 1, 12, &[0, 2048, 4095])
            .photometric("MONOCHROME1")
            .text(0x28, 0x1050, *b"DS", "2048")
            .text(0x28, 0x1051, *b"DS", "4096")
            .build();
        let film = read_film(&bytes, FilmKind::Front).unwrap();
        assert_eq!(film.image.pixels(), &[4095, 2047, 0]);
        let grey = as_opened(&film).unwrap();
        assert_eq!(grey[0], 255);
        assert_eq!(grey[2], 0);
    }

    /// `pixels` as a saved case's PNG holds them: grey, in 8 or 16 bits,
    /// with the significant bits recorded.
    fn saved_png(width: u32, height: u32, bits: u8, pixels: &[u16]) -> Vec<u8> {
        let mut png = Vec::new();
        let mut encoder = png::Encoder::new(&mut png, width, height);
        encoder.set_color(png::ColorType::Grayscale);
        let codes: Vec<u8> = if bits <= 8 {
            encoder.set_depth(png::BitDepth::Eight);
            pixels.iter().map(|&value| value as u8).collect()
        } else {
            encoder.set_depth(png::BitDepth::Sixteen);
            pixels
                .iter()
                .flat_map(|&value| (value << (16 - bits)).to_be_bytes())
                .collect()
        };
        let mut writer = encoder.write_header().unwrap();
        writer.write_chunk(png::chunk::sBIT, &[bits]).unwrap();
        writer.write_image_data(&codes).unwrap();
        writer.finish().unwrap();
        png
    }

    #[test]
    fn a_film_the_user_flipped_reads_the_same_after_its_case_is_reopened() {
        // A ramp, brightest on the left, that the user flipped. Fresh, the
        // record's flip differs from the decoder's, so whoever reads the
        // film through its record flips this picture. The saved case holds
        // the values flipped and a record that says so; reopened, the two
        // agree, so the flip must already be in this picture. A deep film's
        // modality map takes it out to reach the file's values, and it is
        // put back after the window. An 8-bit film is the control: its
        // stored values are handed over as they are.
        for bits in [12u8, 8] {
            let highest = ((1u32 << bits) - 1) as u16;
            let (width, height) = (60u32, 8u32);
            let ramp: Vec<u16> = (0..width * height)
                .map(|i| (u32::from(highest) * (width - 1 - i % width) / (width - 1)) as u16)
                .collect();
            let fresh = stored(width, height, bits, ramp.clone());
            let record = Film {
                kind: FilmKind::Front,
                image: FilmImage {
                    path: FilmKind::Front.png_path().into(),
                    width,
                    height,
                    bits_stored: bits,
                },
                source: SourceFormat::Png,
                kept_tags: None,
                values: PixelValues {
                    inverted: true,
                    inverted_by_user: true,
                    ..PixelValues::AS_STORED
                },
                flags: Vec::new(),
                blanked: Vec::new(),
                geometry: FilmGeometry::UNKNOWN,
            };
            let flipped: Vec<u16> = ramp.iter().map(|value| highest - value).collect();
            let png = saved_png(width, height, bits, &flipped);
            let reopened = crate::read_saved_film(&png, &record).unwrap();
            let seen_fresh: Vec<u8> = as_opened(&fresh)
                .unwrap()
                .iter()
                .map(|value| 255 - value)
                .collect();
            assert!(
                seen_fresh[0] < seen_fresh[width as usize - 1],
                "{bits} bits"
            );
            assert_eq!(as_opened(&reopened).unwrap(), seen_fresh, "{bits} bits");
        }
    }

    #[test]
    fn a_level_halfway_between_two_greys_reads_the_same_after_reopening() {
        // Five levels whose opening window runs from 0 to 4000, so 2000 is
        // exactly halfway: 127.5 of 255. Fresh, it rounds to 128 and the
        // record's flip makes it 127. Reopened, the flip is put back here,
        // and it must land on 127 too, not on 128.
        let (width, height, bits) = (64u32, 128u32, 12u8);
        let levels = [0u16, 1000, 2000, 3000, 4000];
        let film: Vec<u16> = (0..width * height)
            .map(|i| levels[(i % 5) as usize])
            .collect();
        let fresh = stored(width, height, bits, film.clone());
        let record = Film {
            kind: FilmKind::Front,
            image: FilmImage {
                path: FilmKind::Front.png_path().into(),
                width,
                height,
                bits_stored: bits,
            },
            source: SourceFormat::Png,
            kept_tags: None,
            values: PixelValues {
                inverted: true,
                inverted_by_user: true,
                ..PixelValues::AS_STORED
            },
            flags: Vec::new(),
            blanked: Vec::new(),
            geometry: FilmGeometry::UNKNOWN,
        };
        let flipped: Vec<u16> = film.iter().map(|value| 4095 - value).collect();
        let reopened =
            crate::read_saved_film(&saved_png(width, height, bits, &flipped), &record).unwrap();
        let seen_fresh: Vec<u8> = as_opened(&fresh)
            .unwrap()
            .iter()
            .map(|value| 255 - value)
            .collect();
        assert_eq!(seen_fresh[..5], [255, 191, 127, 64, 0]);
        assert_eq!(as_opened(&reopened).unwrap(), seen_fresh);
    }
}
