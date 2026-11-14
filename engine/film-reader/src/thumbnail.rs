//! A small 8-bit preview for the film slots.
//!
//! Preview pixels are area means of the stored film followed by its modality
//! map and display window. The output PNG contains only image chunks. The
//! texture returned by `display_pixels` is separate and has no window applied.
//!
//! SIGMOID file windows remain in `FilmFacts`, but the thumbnail uses the
//! percentile LINEAR_EXACT window for those films. The deterministic
//! exponential dependency is not available to this crate.

use crate::decoded_film::DecodedFilm;
use darkroom::grey_image::GreyImage;
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::{
    DisplayWindow, ModalityMap, Photometric, PixelValues, WindowFunction, WindowSource,
};
use std::borrow::Cow;

/// An exact nearest-rank P1/P99 window over the central 60% of columns and
/// 80% of rows. Tiny images retain at least one row and column. A flat image
/// has width one; the returned centre and width are in modality units.
pub fn default_window(image: &GreyImage<u16>, bits: u8, values: &PixelValues) -> DisplayWindow {
    default_window_where(image, bits, values, |_, _, _| true)
}

/// The intake window with pixels excluded by padding or shutter geometry.
/// When the central area contains no usable pixels, it uses all usable
/// pixels; when none remain, it uses the stored range.
pub(crate) fn default_window_where(
    image: &GreyImage<u16>,
    bits: u8,
    values: &PixelValues,
    mut include: impl FnMut(u32, u32, u16) -> bool,
) -> DisplayWindow {
    let maximum = (1u32 << bits.clamp(1, 16)) - 1;
    // A bounded stack histogram avoids a fallible allocation in this small
    // value-only operation; each bin counts at most MAX_PIXELS samples.
    let mut counts = [0u32; 65_536];
    let left = image.width() / 5;
    let right = (image.width() * 4 / 5).max(left + 1).min(image.width());
    let top = image.height() / 10;
    let bottom = (image.height() * 9 / 10).max(top + 1).min(image.height());
    let mut count = 0u32;
    for y in top..bottom {
        for x in left..right {
            let pixel = image.pixels()[y as usize * image.width() as usize + x as usize];
            if include(x, y, pixel) {
                counts[usize::from(pixel)] += 1;
                count += 1;
            }
        }
    }
    if count == 0 {
        for (index, &pixel) in image.pixels().iter().enumerate() {
            if include(
                index as u32 % image.width(),
                index as u32 / image.width(),
                pixel,
            ) {
                counts[usize::from(pixel)] += 1;
                count += 1;
            }
        }
    }
    let (low, high) = if count == 0 {
        (0, maximum as u16)
    } else {
        let at_rank = |rank: u32| {
            let mut through = 0;
            for (value, &n) in counts.iter().enumerate() {
                through += n;
                if through >= rank {
                    return value as u16;
                }
            }
            maximum as u16
        };
        (
            at_rank(count.div_ceil(100)),
            at_rank((count * 99).div_ceil(100)),
        )
    };
    let first = modality_value(f64::from(low), bits, values);
    let last = modality_value(f64::from(high), bits, values);
    DisplayWindow::CentreWidth {
        centre: first / 2.0 + last / 2.0,
        width: (last - first).abs().max(1.0),
        function: WindowFunction::LinearExact,
        from: WindowSource::Percentile,
    }
}

/// Render an 8-bit greyscale PNG no larger than `max_side` on either axis.
/// The original film and its placement are never changed.
pub fn thumbnail(film: &DecodedFilm, max_side: u32) -> Result<Vec<u8>, EngineError> {
    if max_side == 0 || film.image.width() == 0 || film.image.height() == 0 {
        return Err(EngineError::new(
            ErrorCode::FilmSizeMismatch,
            "The thumbnail needs non-zero dimensions.",
        ));
    }
    let max_side = max_side.min(shapes::film::MAX_FILM_SIDE);
    let factor =
        (f64::from(film.image.width().max(film.image.height())) / f64::from(max_side)).max(1.0);
    let width = (f64::from(film.image.width()) / factor).floor().max(1.0) as u32;
    let height = (f64::from(film.image.height()) / factor).floor().max(1.0) as u32;
    let reduced = darkroom::pyramid::shrink(
        &film.image,
        f64::from(film.image.width()) / f64::from(width),
        f64::from(film.image.height()) / f64::from(height),
    )
    .ok_or_else(crate::invalid_film)?;
    let view = opening_view(film);
    let mut pixels = crate::allocate(reduced.pixels().len())?;
    for &pixel in reduced.pixels() {
        let mut value = view(f64::from(pixel));
        if film.facts.values.inverted_by_user {
            value = 1.0 - value;
        }
        pixels.push((value * 255.0).round() as u8);
    }
    let mut png = crate::allocate(pixels.len() * 2 + 1024)?;
    {
        let mut encoder = png::Encoder::new(&mut png, reduced.width(), reduced.height());
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|_| crate::invalid_film())?;
        writer
            .write_image_data(&pixels)
            .map_err(|_| crate::invalid_film())?;
        writer.finish().map_err(|_| crate::invalid_film())?;
    }
    Ok(png)
}

/// How the film opens on screen, from 0 (black) to 1 (white), for a stored
/// value: the modality map, then the opening window, then a MONOCHROME1
/// film turned back so bone stays bright. The user's own flip is left to
/// the caller. The thumbnail and [`crate::eight_bit`] both read a film
/// through this, so the slot preview and the finder see the same levels.
pub fn opening_view(film: &DecodedFilm) -> impl Fn(f64) -> f64 + '_ {
    let window = if matches!(
        &film.facts.default_window,
        DisplayWindow::CentreWidth {
            function: WindowFunction::Sigmoid,
            ..
        }
    ) {
        Cow::Owned(default_window(
            &film.image,
            film.facts.bits_stored,
            &film.facts.values,
        ))
    } else {
        Cow::Borrowed(&film.facts.default_window)
    };
    let monochrome_one = film
        .facts
        .kept_tags
        .as_ref()
        .is_some_and(|tags| tags.photometric == Some(Photometric::Monochrome1));
    move |pixel| {
        let modality = modality_value(pixel, film.facts.bits_stored, &film.facts.values);
        let value = window_value(modality, &window);
        if monochrome_one { 1.0 - value } else { value }
    }
}

fn modality_value(pixel: f64, bits: u8, values: &PixelValues) -> f64 {
    let maximum = f64::from((1u32 << bits.clamp(1, 16)) - 1);
    let stored = f64::from(values.offset)
        + if values.inverted {
            maximum - pixel
        } else {
            pixel
        };
    match &values.modality {
        ModalityMap::Linear { slope, intercept } => slope * stored + intercept,
        ModalityMap::Lookup {
            first_mapped,
            table,
            ..
        } => lookup(stored, *first_mapped, table),
    }
}

fn lookup(value: f64, first: i32, table: &[u16]) -> f64 {
    if table.is_empty() {
        return 0.0;
    }
    let index = ((value - f64::from(first)).max(0.0) as usize).min(table.len() - 1);
    f64::from(table[index])
}

fn window_value(value: f64, window: &DisplayWindow) -> f64 {
    match window {
        DisplayWindow::CentreWidth {
            centre,
            width,
            function,
            ..
        } => match function {
            WindowFunction::Linear if *width <= 1.0 => f64::from(value > centre - 0.5),
            WindowFunction::Linear => {
                ((value - (centre - 0.5)) / (width - 1.0) + 0.5).clamp(0.0, 1.0)
            }
            WindowFunction::LinearExact => ((value - centre) / width + 0.5).clamp(0.0, 1.0),
            // The thumbnail entry replaces this with its percentile window.
            WindowFunction::Sigmoid => unreachable!("thumbnail selects a linear window"),
        },
        DisplayWindow::Lookup {
            first, bits, table, ..
        } => (lookup(value, *first, table) / f64::from((1u32 << bits.clamp(&1, &16)) - 1))
            .clamp(0.0, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(centre: f64, width: f64, function: WindowFunction) -> DisplayWindow {
        DisplayWindow::CentreWidth {
            centre,
            width,
            function,
            from: WindowSource::File,
        }
    }

    #[test]
    fn linear_windows_match_the_dicom_equations() {
        let linear = window(2048.0, 4096.0, WindowFunction::Linear);
        // Independently worked PS3.3 C.11.2 values, within 1e-9 display units.
        for (value, expected) in [
            (0.0, 0.0),
            (1024.0, 63.765567765567766),
            (2047.5, 127.5),
            (3000.0, 186.81318681318683),
            (4095.0, 255.0),
        ] {
            assert!((255.0 * window_value(value, &linear) - expected).abs() < 1e-9);
        }
        let threshold = window(10.0, 1.0, WindowFunction::Linear);
        assert_eq!(window_value(9.5, &threshold), 0.0);
        assert_eq!(window_value(9.6, &threshold), 1.0);
        let exact = window(100.0, 50.0, WindowFunction::LinearExact);
        assert_eq!(window_value(75.0, &exact), 0.0);
        assert_eq!(window_value(100.0, &exact), 0.5);
    }

    #[test]
    fn the_intake_histogram_uses_exact_codes_and_excludes_padding() {
        let image = GreyImage::new(
            100,
            1,
            (0..100).collect(),
            darkroom::grey_image::Placement::FILM,
        )
        .unwrap();
        let all = default_window(&image, 8, &PixelValues::AS_STORED);
        // Central columns 20..80 contain 60 values: nearest-rank P1 = 20,
        // P99 = 79, giving centre 49.5 and width 59 exactly.
        assert_eq!(
            all,
            DisplayWindow::CentreWidth {
                centre: 49.5,
                width: 59.0,
                function: WindowFunction::LinearExact,
                from: WindowSource::Percentile
            }
        );
        let excluded = default_window_where(&image, 8, &PixelValues::AS_STORED, |_, _, p| p >= 40);
        assert_eq!(
            excluded,
            DisplayWindow::CentreWidth {
                centre: 59.5,
                width: 39.0,
                function: WindowFunction::LinearExact,
                from: WindowSource::Percentile
            }
        );
    }

    #[test]
    fn preview_area_means_preserve_the_film_and_fit_thin_images() {
        use darkroom::grey_image::Placement;
        use shapes::film::{FilmFacts, FilmKind, SourceFormat};
        let image = GreyImage::new(
            4,
            2,
            vec![0, 64, 128, 192, 0, 64, 128, 192],
            Placement::FILM,
        )
        .unwrap();
        let facts = FilmFacts {
            kind: FilmKind::Front,
            width: 4,
            height: 2,
            bits_stored: 8,
            source: SourceFormat::Png,
            kept_tags: None,
            values: PixelValues::AS_STORED,
            flags: Vec::new(),
            default_window: window(127.5, 255.0, WindowFunction::LinearExact),
            exif_quarter_turns: None,
        };
        let mut film = DecodedFilm { image, facts };
        let unchanged = film.clone();
        let preview = thumbnail(&film, 2).unwrap();
        let decoded = crate::png_jpeg::read(&preview, FilmKind::Front, SourceFormat::Png).unwrap();
        assert_eq!((decoded.facts.width, decoded.facts.height), (2, 1));
        // Means of each 2x2 block: (0+64)/2=32, (128+192)/2=160.
        assert_eq!(decoded.image.pixels(), &[32, 160]);
        assert_eq!(film, unchanged);

        film.image = GreyImage::new(1, 100, vec![128; 100], Placement::FILM).unwrap();
        film.facts.width = 1;
        film.facts.height = 100;
        let preview = thumbnail(&film, 10).unwrap();
        let decoded = crate::png_jpeg::read(&preview, FilmKind::Front, SourceFormat::Png).unwrap();
        assert_eq!((decoded.facts.width, decoded.facts.height), (1, 10));
        assert!(decoded.image.pixels().iter().all(|&value| value == 128));
    }

    #[test]
    fn lookups_clamp_and_modality_values_are_recovered_before_windowing() {
        let values = PixelValues {
            offset: -2048,
            inverted: true,
            inverted_by_user: false,
            modality: ModalityMap::Linear {
                slope: 0.5,
                intercept: 10.25,
            },
        };
        // p=0 means s=2047, m=1033.75; p=4095 means s=-2048, m=-1013.75.
        assert_eq!(modality_value(0.0, 12, &values), 1033.75);
        assert_eq!(modality_value(4095.0, 12, &values), -1013.75);
        let table = DisplayWindow::Lookup {
            first: -2,
            bits: 8,
            table: vec![0, 85, 170, 255],
            from: WindowSource::FileLookup,
        };
        assert_eq!(window_value(-50.0, &table), 0.0);
        assert_eq!(window_value(-1.0, &table), 1.0 / 3.0);
        assert_eq!(window_value(50.0, &table), 1.0);
    }
}
