//! A film as a lossless greyscale PNG at its own bit depth, with no metadata
//! beyond the significant-bits chunk.

use crate::strict;
use png::{BitDepth, ColorType};
use shapes::case_file_v1::CaseFileV1;
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::{Film, FilmFacts, FilmImage, MAX_FILM_SIDE};
use std::io::Cursor;

/// Encode the desk's pixels through the current record. An opened case's
/// facts already describe its saved pixels, so saving it again does not
/// apply the brightness flip twice. Turns and facing never move pixels.
pub fn encode_png(pixels: &[u16], facts: &FilmFacts, film: &Film) -> Result<Vec<u8>, EngineError> {
    let mut case = CaseFileV1::new("0.1.0");
    case.films.push(film.clone());
    shapes::reading::check_case_file(&case)?;
    if facts.kind != film.kind
        || facts.width != film.image.width
        || facts.height != film.image.height
        || facts.bits_stored != film.image.bits_stored
    {
        return Err(size_mismatch(film.kind.png_path()));
    }
    if facts.values.offset != film.values.offset || facts.values.modality != film.values.modality {
        return Err(strict::invalid("film.values"));
    }
    encode_pixels(&film.image, pixels, |value, x, y| {
        if film.blanked.iter().any(|rect| rect.covers_pixel(x, y)) {
            0
        } else if facts.values.inverted != film.values.inverted {
            maximum(film.image.bits_stored) - value
        } else {
            value
        }
    })
}

fn maximum(bits: u8) -> u16 {
    ((1_u32 << bits) - 1) as u16
}

fn dimensions(image: &FilmImage) -> Result<(usize, u8), EngineError> {
    if !(1..=MAX_FILM_SIDE).contains(&image.width)
        || !(1..=MAX_FILM_SIDE).contains(&image.height)
        || !(1..=16).contains(&image.bits_stored)
    {
        return Err(strict::invalid("film.image"));
    }
    let count = u64::from(image.width) * u64::from(image.height);
    // Both the u16 pixels and the PNG buffer are bounded independently.
    if count * 2 > strict::MAX_DECODED_BYTES as u64 {
        return Err(strict::too_large("film.image"));
    }
    Ok((count as usize, if image.bits_stored <= 8 { 8 } else { 16 }))
}

fn encode_pixels(
    image: &FilmImage,
    pixels: &[u16],
    transform: impl Fn(u16, u32, u32) -> u16,
) -> Result<Vec<u8>, EngineError> {
    let (count, depth) = dimensions(image)?;
    if pixels.len() != count
        || pixels
            .iter()
            .any(|value| *value > maximum(image.bits_stored))
    {
        return Err(strict::invalid("film.pixels"));
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(count * usize::from(depth / 8))
        .map_err(|_| strict::allocation())?;
    for (index, &pixel) in pixels.iter().enumerate() {
        let value = transform(
            pixel,
            index as u32 % image.width,
            index as u32 / image.width,
        );
        // Repeat the significant high bits to fill the container. Reading
        // by shifting right recovers exactly the original 1–16 bit value.
        let mut expanded = u32::from(value);
        let mut filled = image.bits_stored;
        while filled < depth {
            let next = image.bits_stored.min(depth - filled);
            expanded = (expanded << next) | u32::from(value >> (image.bits_stored - next));
            filled += next;
        }
        if depth == 8 {
            bytes.push(expanded as u8);
        } else {
            bytes.extend_from_slice(&(expanded as u16).to_be_bytes());
        }
    }
    let mut info = png::Info::with_size(image.width, image.height);
    info.color_type = ColorType::Grayscale;
    info.bit_depth = if depth == 8 {
        BitDepth::Eight
    } else {
        BitDepth::Sixteen
    };
    let mut output = Vec::new();
    let mut encoder =
        png::Encoder::with_info(&mut output, info).map_err(|_| strict::invalid("film.pixels"))?;
    encoder.set_compression(png::Compression::Fast);
    encoder.set_filter(png::Filter::Adaptive);
    let mut writer = encoder
        .write_header()
        .map_err(|_| strict::invalid("film.pixels"))?;
    writer
        .write_chunk(png::chunk::sBIT, &[image.bits_stored])
        .map_err(|_| strict::invalid("film.pixels"))?;
    writer
        .write_image_data(&bytes)
        .map_err(|_| strict::invalid("film.pixels"))?;
    writer
        .finish()
        .map_err(|_| strict::invalid("film.pixels"))?;
    Ok(output)
}

fn size_mismatch(path: &str) -> EngineError {
    EngineError::new(
        ErrorCode::CaseFilmSizeMismatch,
        "PNG dimensions or significant bits differ from the record",
    )
    .with_detail(path)
}

/// Decode without scaling or colour conversion; case PNGs must already
/// be 8/16-bit greyscale with the recorded significant bits.
fn decode(bytes: &[u8], film: &Film, require_blanked: bool) -> Result<Vec<u16>, EngineError> {
    let path = film.kind.png_path();
    if bytes.len() as u64 > strict::MAX_FILM_PNG_BYTES {
        return Err(strict::too_large(path));
    }
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_limits(png::Limits {
        bytes: strict::MAX_DECODED_BYTES,
    });
    decoder.set_ignore_text_chunk(true);
    decoder.set_ignore_iccp_chunk(true);
    let mut reader = decoder.read_info().map_err(|_| strict::invalid(path))?;
    let info = reader.info();
    let expected_depth = if film.image.bits_stored <= 8 {
        BitDepth::Eight
    } else {
        BitDepth::Sixteen
    };
    if info.width != film.image.width
        || info.height != film.image.height
        || info.bit_depth != expected_depth
        || info.sbit.as_deref() != Some(&[film.image.bits_stored])
    {
        return Err(size_mismatch(path));
    }
    if info.color_type != ColorType::Grayscale
        || info.animation_control.is_some()
        || info.trns.is_some()
    {
        return Err(strict::invalid(path));
    }
    let (count, depth) = dimensions(&film.image).map_err(|mut error| {
        error.detail = Some(path.to_owned());
        error
    })?;
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| strict::too_large(path))?;
    if size != count * usize::from(depth / 8) {
        return Err(size_mismatch(path));
    }
    let mut buffer = Vec::new();
    buffer
        .try_reserve_exact(size)
        .map_err(|_| strict::allocation())?;
    buffer.resize(size, 0);
    reader
        .next_frame(&mut buffer)
        .map_err(|_| strict::invalid(path))?;
    reader.finish().map_err(|_| strict::invalid(path))?;
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(count)
        .map_err(|_| strict::allocation())?;
    if depth == 8 {
        pixels.extend(buffer.iter().map(|value| u16::from(*value)));
    } else {
        pixels.extend(
            buffer
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u16::from_be_bytes(*pair)),
        );
    }
    let shift = depth - film.image.bits_stored;
    for (index, value) in pixels.iter_mut().enumerate() {
        // Check the container before dropping low bits: a blanked PNG pixel
        // must be literally zero, including its insignificant bits.
        if require_blanked
            && *value != 0
            && film.blanked.iter().any(|rect| {
                rect.covers_pixel(
                    index as u32 % film.image.width,
                    index as u32 / film.image.width,
                )
            })
        {
            return Err(strict::invalid(path));
        }
        *value >>= shift;
    }
    Ok(pixels)
}

/// Re-encoding from samples strips every supplied metadata chunk. The
/// record already describes these pixels, so only blanking is applied.
pub(crate) fn clean(bytes: &[u8], film: &Film) -> Result<Vec<u8>, EngineError> {
    let pixels = decode(bytes, film, false)?;
    encode_pixels(&film.image, &pixels, |value, x, y| {
        if film.blanked.iter().any(|rect| rect.covers_pixel(x, y)) {
            0
        } else {
            value
        }
    })
}

/// An opened case cannot contain unblanked pixels behind a saved mask.
pub(crate) fn validate(bytes: &[u8], film: &Film) -> Result<(), EngineError> {
    decode(bytes, film, true)?;
    Ok(())
}
