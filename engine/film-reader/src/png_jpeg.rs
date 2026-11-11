//! PNG and JPEG films: 8- and 16-bit greyscale, colour turned into luminance,
//! and the EXIF orientation of a JPEG kept as a proposal (pixels are never
//! turned).

use crate::decoded_film::DecodedFilm;
use crate::{allocate, check_dimensions, invalid_film, too_large};
use darkroom::grey_image::{GreyImage, Placement};
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::{
    FilmFacts, FilmKind, IntakeFlag, Photometric, PixelValues, SourceFormat, TransferSyntax,
};
use shapes::film_geometry::QuarterTurns;
use std::io::Cursor;

/// Decode a recognised raster format. No metadata other than significant
/// bits and a JPEG orientation proposal is copied into the returned film.
pub fn read(
    bytes: &[u8],
    kind: FilmKind,
    format: SourceFormat,
) -> Result<DecodedFilm, EngineError> {
    match format {
        SourceFormat::Png => read_png(bytes, kind),
        SourceFormat::Jpeg => read_jpeg(bytes, kind),
        SourceFormat::Dicom => Err(invalid_film()),
    }
}

fn read_png(bytes: &[u8], kind: FilmKind) -> Result<DecodedFilm, EngineError> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_ignore_text_chunk(true);
    decoder.set_ignore_iccp_chunk(true);
    decoder.set_limits(png::Limits { bytes: 512 << 20 });
    decoder.set_transformations(png::Transformations::EXPAND);
    let header = decoder.read_header_info().map_err(png_error)?;
    check_dimensions(header.width, header.height, bytes.len(), 12)?;
    let mut reader = decoder.read_info().map_err(png_error)?;
    let info = reader.info();
    let original_grey = matches!(
        info.color_type,
        png::ColorType::Grayscale | png::ColorType::GrayscaleAlpha
    );
    let significant = if original_grey {
        info.sbit
            .as_ref()
            .and_then(|bits| bits.first())
            .copied()
            .unwrap_or(info.bit_depth as u8)
    } else {
        info.bit_depth as u8
    };
    let animated = info
        .animation_control
        .is_some_and(|control| control.num_frames > 1);
    let size = reader.output_buffer_size().ok_or_else(too_large)?;
    let mut decoded = allocate(size)?;
    decoded.resize(size, 0);
    let output = reader.next_frame(&mut decoded).map_err(png_error)?;
    let depth = output.bit_depth as u8;
    if depth != 8 && depth != 16 {
        return Err(unsupported_bits());
    }
    let colour = matches!(
        output.color_type,
        png::ColorType::Rgb | png::ColorType::Rgba
    );
    let alpha = matches!(
        output.color_type,
        png::ColorType::GrayscaleAlpha | png::ColorType::Rgba
    );
    let samples = output.color_type.samples();
    let bytes_per_sample = usize::from(depth / 8);
    let bits = if colour { depth } else { significant };
    let max_alpha = (1u32 << depth) - 1;
    let mut ignored_alpha = false;
    let mut pixels = allocate(output.width as usize * output.height as usize)?;
    for pixel in decoded[..output.buffer_size()].chunks_exact(samples * bytes_per_sample) {
        let sample = |index: usize| -> u16 {
            if depth == 16 {
                u16::from_be_bytes([pixel[index * 2], pixel[index * 2 + 1]])
            } else {
                u16::from(pixel[index])
            }
        };
        if alpha && u32::from(sample(samples - 1)) != max_alpha {
            ignored_alpha = true;
        }
        pixels.push(if colour {
            luma(sample(0), sample(1), sample(2))
        } else {
            sample(0) >> (depth - bits)
        });
    }
    let mut flags = Vec::new();
    if colour {
        flags.push(IntakeFlag::ColourConverted);
    }
    if ignored_alpha {
        flags.push(IntakeFlag::AlphaIgnored);
    }
    if animated {
        flags.push(IntakeFlag::MultiFrame);
    }
    make_film(
        kind,
        SourceFormat::Png,
        output.width,
        output.height,
        bits,
        pixels,
        flags,
        None,
    )
}

fn read_jpeg(bytes: &[u8], kind: FilmKind) -> Result<DecodedFilm, EngineError> {
    let header = jpeg_header(bytes)?;
    let mut flags = Vec::new();
    if !header.lossless {
        flags.push(IntakeFlag::Lossy);
    }
    if header.components == 3 {
        flags.push(IntakeFlag::ColourConverted);
    }
    let pixels = if header.lossless {
        lossless_pixels(bytes, &header)?
    } else {
        lossy_pixels(bytes, &header)?
    };
    let turns = match header.orientation {
        Some(3) => QuarterTurns::new(2),
        Some(6) => QuarterTurns::new(1),
        Some(8) => QuarterTurns::new(3),
        Some(2 | 4 | 5 | 7) => {
            flags.push(IntakeFlag::ExifMirrored);
            None
        }
        _ => None,
    };
    make_film(
        kind,
        SourceFormat::Jpeg,
        header.width,
        header.height,
        header.precision,
        pixels,
        flags,
        turns,
    )
}

fn lossless_pixels(bytes: &[u8], header: &JpegHeader) -> Result<Vec<u16>, EngineError> {
    use crate::dicom::pixel_data::{self, PixelHeader};
    let decoded = pixel_data::decode(
        bytes,
        TransferSyntax::JpegLossless,
        PixelHeader {
            width: header.width,
            height: header.height,
            bits_allocated: if header.precision <= 8 { 8 } else { 16 },
            samples_per_pixel: header.components,
            planar_configuration: 0,
            photometric: if header.components == 3 {
                Photometric::Rgb
            } else {
                Photometric::Monochrome2
            },
        },
    )?;
    if header.components == 1 {
        return Ok(decoded.samples);
    }
    let mut pixels = allocate(header.width as usize * header.height as usize)?;
    pixels.extend(
        decoded
            .samples
            .as_chunks::<3>()
            .0
            .iter()
            .map(|rgb| luma(rgb[0], rgb[1], rgb[2])),
    );
    Ok(pixels)
}

fn lossy_pixels(bytes: &[u8], header: &JpegHeader) -> Result<Vec<u16>, EngineError> {
    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(bytes));
    decoder.set_max_decoding_buffer_size(160_000_000);
    let decoded = decoder.decode().map_err(jpeg_error)?;
    let info = decoder.info().ok_or_else(invalid_film)?;
    if u32::from(info.width) != header.width || u32::from(info.height) != header.height {
        return Err(EngineError::new(
            ErrorCode::FilmSizeMismatch,
            "The JPEG dimensions disagree.",
        ));
    }
    let count = header.width as usize * header.height as usize;
    if header.components == 3 && info.pixel_format != jpeg_decoder::PixelFormat::RGB24 {
        return Err(unsupported_colour());
    }
    if decoded.len() != count * info.pixel_format.pixel_bytes() {
        return Err(invalid_film());
    }
    let mut pixels = allocate(count)?;
    match info.pixel_format {
        jpeg_decoder::PixelFormat::L8 => {
            pixels.extend(decoded.iter().map(|&value| u16::from(value)))
        }
        jpeg_decoder::PixelFormat::L16 => return Err(unsupported_bits()),
        jpeg_decoder::PixelFormat::RGB24 => {
            pixels.extend(
                decoded
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|rgb| luma(u16::from(rgb[0]), u16::from(rgb[1]), u16::from(rgb[2]))),
            );
        }
        jpeg_decoder::PixelFormat::CMYK32 => return Err(unsupported_colour()),
    }
    Ok(pixels)
}

#[allow(clippy::too_many_arguments)]
fn make_film(
    kind: FilmKind,
    source: SourceFormat,
    width: u32,
    height: u32,
    bits: u8,
    pixels: Vec<u16>,
    flags: Vec<IntakeFlag>,
    exif_quarter_turns: Option<QuarterTurns>,
) -> Result<DecodedFilm, EngineError> {
    let image = GreyImage::new(width, height, pixels, Placement::FILM).ok_or_else(invalid_film)?;
    let values = PixelValues::AS_STORED;
    let default_window = crate::thumbnail::default_window(&image, bits, &values);
    Ok(DecodedFilm {
        image,
        facts: FilmFacts {
            kind,
            width,
            height,
            bits_stored: bits,
            source,
            kept_tags: None,
            values,
            flags,
            default_window,
            exif_quarter_turns,
        },
    })
}

/// BT.601 luma with exact decimal coefficients, rounded to a stored code.
pub(crate) fn luma(red: u16, green: u16, blue: u16) -> u16 {
    ((299 * u32::from(red) + 587 * u32::from(green) + 114 * u32::from(blue) + 500) / 1000) as u16
}

/// The bounded SOF information needed before a JPEG decoder allocates.
pub(crate) struct JpegHeader {
    pub width: u32,
    pub height: u32,
    pub precision: u8,
    pub components: u8,
    pub lossless: bool,
    orientation: Option<u16>,
}

/// Check the SOF before entering jpeg-decoder: malformed component counts
/// can otherwise reach an assertion in that dependency. No metadata escapes.
pub(crate) fn jpeg_header(bytes: &[u8]) -> Result<JpegHeader, EngineError> {
    if !bytes.starts_with(b"\xff\xd8") {
        return Err(invalid_film());
    }
    let mut position = 2;
    let mut header: Option<JpegHeader> = None;
    let mut orientation = None;
    while position < bytes.len() {
        if bytes[position] != 0xff {
            return Err(invalid_film());
        }
        while bytes.get(position) == Some(&0xff) {
            position += 1;
        }
        let marker = *bytes.get(position).ok_or_else(truncated)?;
        position += 1;
        if marker == 0xda {
            if let Some(header) = &mut header {
                header.orientation = orientation;
            }
            return header.ok_or_else(invalid_film);
        }
        if marker == 0xd9 {
            break;
        }
        if matches!(marker, 0x01 | 0xd0..=0xd7) {
            continue;
        }
        let length_bytes = bytes.get(position..position + 2).ok_or_else(truncated)?;
        let length = usize::from(u16::from_be_bytes([length_bytes[0], length_bytes[1]]));
        if length < 2 {
            return Err(invalid_film());
        }
        let segment = bytes
            .get(position + 2..position + length)
            .ok_or_else(truncated)?;
        if marker == 0xe1 && segment.starts_with(b"Exif\0\0") {
            orientation = orientation.or_else(|| exif_orientation(&segment[6..]));
        }
        if marker == 0xf7 {
            return Err(EngineError::new(
                ErrorCode::FilmUnsupportedCompression,
                "JPEG-LS is not supported.",
            ));
        }
        if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) {
            if header.is_some() || segment.len() < 6 {
                return Err(invalid_film());
            }
            let precision = segment[0];
            let height = u32::from(u16::from_be_bytes([segment[1], segment[2]]));
            let width = u32::from(u16::from_be_bytes([segment[3], segment[4]]));
            let components = segment[5];
            if !(1..=4).contains(&components) || segment.len() != 6 + usize::from(components) * 3 {
                return Err(invalid_film());
            }
            if !matches!(marker, 0xc0..=0xc3) {
                return Err(EngineError::new(
                    ErrorCode::FilmUnsupportedCompression,
                    "This JPEG coding process is not supported.",
                ));
            }
            let lossless = marker == 0xc3;
            if !(2..=16).contains(&precision) || (!lossless && precision != 8) {
                return Err(unsupported_bits());
            }
            if components == 2 || components == 4 {
                return Err(unsupported_colour());
            }
            check_dimensions(width, height, bytes.len(), if lossless { 18 } else { 10 })?;
            header = Some(JpegHeader {
                width,
                height,
                precision,
                components,
                lossless,
                orientation: None,
            });
        }
        position += length;
    }
    Err(truncated())
}

fn exif_orientation(tiff: &[u8]) -> Option<u16> {
    let little = match tiff.get(..2)? {
        b"II" => true,
        b"MM" => false,
        _ => return None,
    };
    let word = |at: usize| -> Option<u16> {
        let pair: [u8; 2] = tiff.get(at..at.checked_add(2)?)?.try_into().ok()?;
        Some(if little {
            u16::from_le_bytes(pair)
        } else {
            u16::from_be_bytes(pair)
        })
    };
    let long = |at: usize| -> Option<u32> {
        let bytes: [u8; 4] = tiff.get(at..at.checked_add(4)?)?.try_into().ok()?;
        Some(if little {
            u32::from_le_bytes(bytes)
        } else {
            u32::from_be_bytes(bytes)
        })
    };
    if word(2)? != 42 {
        return None;
    }
    let ifd = usize::try_from(long(4)?).ok()?;
    let count = usize::from(word(ifd)?);
    let first = ifd.checked_add(2)?;
    for index in 0..count {
        let entry = first.checked_add(index.checked_mul(12)?)?;
        tiff.get(entry..entry.checked_add(12)?)?;
        if word(entry)? == 0x0112 && word(entry + 2)? == 3 && long(entry + 4)? == 1 {
            return word(entry + 8).filter(|value| (1..=8).contains(value));
        }
    }
    None
}

fn unsupported_bits() -> EngineError {
    EngineError::new(
        ErrorCode::FilmUnsupportedBits,
        "The image bit depth is not supported.",
    )
}
fn unsupported_colour() -> EngineError {
    EngineError::new(
        ErrorCode::FilmUnsupportedColour,
        "The image colour layout is not supported.",
    )
}
fn truncated() -> EngineError {
    EngineError::new(
        ErrorCode::FilmTruncated,
        "The image ends before its pixels are complete.",
    )
}

fn png_error(error: png::DecodingError) -> EngineError {
    match error {
        png::DecodingError::IoError(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
            truncated()
        }
        png::DecodingError::LimitsExceeded => too_large(),
        _ => invalid_film(),
    }
}

fn jpeg_error(error: jpeg_decoder::Error) -> EngineError {
    match error {
        jpeg_decoder::Error::Io(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
            truncated()
        }
        jpeg_decoder::Error::Unsupported(_) => EngineError::new(
            ErrorCode::FilmUnsupportedCompression,
            "The JPEG coding process is not supported.",
        ),
        _ => invalid_film(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    fn png_bytes(info: png::Info<'_>, samples: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        let significant = info.sbit.clone();
        let mut writer = png::Encoder::with_info(&mut bytes, info)
            .unwrap()
            .write_header()
            .unwrap();
        // The encoder does not write Info::sbit automatically.
        if let Some(bits) = significant {
            writer.write_chunk(png::chunk::sBIT, &bits).unwrap();
        }
        writer.write_image_data(samples).unwrap();
        writer.finish().unwrap();
        bytes
    }

    #[test]
    fn greyscale_pngs_keep_significant_codes_exactly() {
        for bits in [1, 8, 12, 16] {
            let depth = if bits <= 8 { 8 } else { 16 };
            let maximum = (1u32 << bits) - 1;
            let storage_max = (1u32 << depth) - 1;
            let expected = [0, 1, maximum / 3, maximum];
            let samples: Vec<u8> = expected
                .iter()
                .flat_map(|&value| {
                    // Left-bit replication: the upper significant bits remain
                    // the original code, including at one-bit precision.
                    let scaled = (value * storage_max / maximum) as u16;
                    if depth == 16 {
                        scaled.to_be_bytes().to_vec()
                    } else {
                        vec![scaled as u8]
                    }
                })
                .collect();
            let mut info = png::Info::with_size(4, 1);
            info.color_type = png::ColorType::Grayscale;
            info.bit_depth = if depth == 16 {
                png::BitDepth::Sixteen
            } else {
                png::BitDepth::Eight
            };
            info.sbit = Some(Cow::Owned(vec![bits]));
            let bytes = png_bytes(info, &samples);
            let film = read(&bytes, FilmKind::Front, SourceFormat::Png).unwrap();
            assert_eq!(film.facts.bits_stored, bits);
            assert_eq!(film.image.pixels(), expected.map(|code| code as u16));
            assert_eq!(film.image.placement(), Placement::FILM);
            assert!(film.facts.kept_tags.is_none());
        }
    }

    #[test]
    fn packed_grey_and_palette_pngs_expand_without_moving_pixels() {
        let mut info = png::Info::with_size(4, 1);
        info.color_type = png::ColorType::Grayscale;
        info.bit_depth = png::BitDepth::Two;
        let film = read_png(&png_bytes(info, &[0b00_01_10_11]), FilmKind::Side).unwrap();
        assert_eq!(film.image.pixels(), &[0, 1, 2, 3]);
        assert_eq!(film.facts.bits_stored, 2);

        let mut info = png::Info::with_size(4, 1);
        info.color_type = png::ColorType::Indexed;
        info.bit_depth = png::BitDepth::Two;
        info.palette = Some(Cow::Borrowed(&[
            255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255,
        ]));
        let film = read_png(&png_bytes(info, &[0b00_01_10_11]), FilmKind::Side).unwrap();
        // 255 times BT.601's weights, rounded: 76, 150, 29; white = 255.
        assert_eq!(film.image.pixels(), &[76, 150, 29, 255]);
        assert!(film.facts.flags.contains(&IntakeFlag::ColourConverted));
    }

    #[test]
    fn rgb_and_alpha_pngs_use_luma_and_ignore_alpha() {
        let mut info = png::Info::with_size(2, 1);
        info.color_type = png::ColorType::Rgba;
        info.bit_depth = png::BitDepth::Sixteen;
        let data: Vec<u8> = [65535u16, 0, 0, 0, 0, 65535, 0, 65535]
            .into_iter()
            .flat_map(u16::to_be_bytes)
            .collect();
        let film = read_png(&png_bytes(info, &data), FilmKind::Front).unwrap();
        // round(65535*.299) = 19595, round(65535*.587) = 38469.
        assert_eq!(film.image.pixels(), &[19595, 38469]);
        assert!(film.facts.flags.contains(&IntakeFlag::ColourConverted));
        assert!(film.facts.flags.contains(&IntakeFlag::AlphaIgnored));
    }

    fn tiff_orientation(orientation: u16) -> Vec<u8> {
        let mut bytes = b"II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0".to_vec();
        bytes.extend_from_slice(&orientation.to_le_bytes());
        bytes.extend_from_slice(&[0; 6]);
        bytes
    }

    fn jpeg_bytes(orientation: u16) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut encoder = jpeg_encoder::Encoder::new(&mut bytes, 100);
        encoder
            .add_exif_metadata(&tiff_orientation(orientation))
            .unwrap();
        encoder
            .encode(&[128; 16 * 8], 16, 8, jpeg_encoder::ColorType::Luma)
            .unwrap();
        bytes
    }

    #[test]
    fn jpeg_exif_only_proposes_turns_and_does_not_turn_pixels() {
        for (orientation, turns) in [(1, None), (3, Some(2)), (6, Some(1)), (8, Some(3))] {
            let film = read_jpeg(&jpeg_bytes(orientation), FilmKind::Side).unwrap();
            assert_eq!((film.facts.width, film.facts.height), (16, 8));
            assert_eq!(film.facts.exif_quarter_turns.map(QuarterTurns::get), turns);
            // A constant 128 block has only a zero DC term after level
            // shifting, so even a lossy JPEG reproduces it exactly.
            assert!(film.image.pixels().iter().all(|&p| p == 128));
            assert!(film.facts.flags.contains(&IntakeFlag::Lossy));
        }
        for orientation in [2, 4, 5, 7] {
            let film = read_jpeg(&jpeg_bytes(orientation), FilmKind::Side).unwrap();
            assert_eq!(film.facts.exif_quarter_turns, None);
            assert!(film.facts.flags.contains(&IntakeFlag::ExifMirrored));
        }
    }

    #[test]
    fn standalone_lossless_jpeg_shares_exact_restart_and_point_transform_decoding() {
        let samples = [
            0, 1, 3, 7, 127, 129, 1023, 4095, 4094, 2049, 2048, 1025, 512, 257, 255, 2,
        ];
        let mut bytes = crate::test_support::lossless_jpeg(&samples, 8, 2, 12, 6, 2, 8);
        let mut exif = b"Exif\0\0".to_vec();
        exif.extend(tiff_orientation(6));
        let mut app1 = vec![0xff, 0xe1];
        app1.extend(((exif.len() + 2) as u16).to_be_bytes());
        app1.extend(exif);
        // APP1 after SOF is still before the scan and must be read without
        // asking a pixel decoder to preserve the rest of the metadata.
        bytes.splice(15..15, app1);
        let film = read_jpeg(&bytes, FilmKind::Front).unwrap();
        // Point transform two discards exactly the lowest two bits; these
        // answers are the input codes rounded down to multiples of four.
        assert_eq!(
            film.image.pixels(),
            &[
                0, 0, 0, 4, 124, 128, 1020, 4092, 4092, 2048, 2048, 1024, 512, 256, 252, 0
            ]
        );
        assert_eq!(film.facts.bits_stored, 12);
        assert_eq!(
            film.facts.exif_quarter_turns.map(QuarterTurns::get),
            Some(1)
        );
        assert!(!film.facts.flags.contains(&IntakeFlag::Lossy));
    }

    #[test]
    fn rgb_jpeg_luma_is_computed_once() {
        let samples: Vec<u8> = (0..64).flat_map(|_| [255, 0, 0]).collect();
        let mut bytes = Vec::new();
        let mut encoder = jpeg_encoder::Encoder::new(&mut bytes, 100);
        encoder.set_sampling_factor(jpeg_encoder::SamplingFactor::F_1_1);
        encoder
            .encode(&samples, 8, 8, jpeg_encoder::ColorType::Rgb)
            .unwrap();
        let film = read_jpeg(&bytes, FilmKind::Front).unwrap();
        // Red's BT.601 luma is round(.299*255)=76. One code of tolerance
        // allows the codec's chroma and integer IDCT rounding.
        assert!(
            film.image
                .pixels()
                .iter()
                .all(|&pixel| pixel.abs_diff(76) <= 1)
        );
        assert!(film.facts.flags.contains(&IntakeFlag::ColourConverted));
    }

    #[test]
    fn malformed_sof_is_refused_before_the_codec() {
        for components in [0, 2, 4, 9, 255] {
            let mut bytes =
                b"\xff\xd8\xff\xc0\0\x0b\x08\0\x08\0\x10\x01\x01\x11\0\xff\xda".to_vec();
            bytes[11] = components;
            assert!(jpeg_header(&bytes).is_err());
        }
        let bytes = jpeg_bytes(1);
        for end in 0..bytes.len() {
            let outcome = std::panic::catch_unwind(|| read_jpeg(&bytes[..end], FilmKind::Front));
            assert!(outcome.is_ok(), "truncated JPEG must not panic");
        }
    }

    #[test]
    fn png_metadata_does_not_reach_any_returned_value_or_preview() {
        let sentinel = b"metadata-canary-482";
        let mut info = png::Info::with_size(2, 1);
        info.color_type = png::ColorType::Grayscale;
        info.bit_depth = png::BitDepth::Eight;
        info.exif_metadata = Some(Cow::Borrowed(sentinel));
        info.uncompressed_latin1_text
            .push(png::text_metadata::TEXtChunk::new(
                "Comment",
                String::from_utf8(sentinel.to_vec()).unwrap(),
            ));
        let bytes = png_bytes(info, &[0, 255]);
        let film = read_png(&bytes, FilmKind::Front).unwrap();
        let debug = format!("{film:?}");
        assert!(
            !debug
                .as_bytes()
                .windows(sentinel.len())
                .any(|window| window == sentinel)
        );
        let preview = crate::thumbnail::thumbnail(&film, 256).unwrap();
        assert!(
            !preview
                .windows(sentinel.len())
                .any(|window| window == sentinel)
        );
        let mut reader = png::Decoder::new(Cursor::new(&preview))
            .read_info()
            .unwrap();
        assert!(reader.info().uncompressed_latin1_text.is_empty());
        assert!(reader.info().exif_metadata.is_none());
        let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
        reader.next_frame(&mut pixels).unwrap();
        // The tiny central box holds only code 0, so its width-one window
        // puts that code at middle grey; code 255 clips to white.
        assert_eq!(pixels, &[128, 255]);
    }
}
