//! DICOM intake with a bounded, allow-list parser and pure Rust codecs.
//! Unknown tags are skipped without decoding their values. Parser and
//! codec errors are replaced with fixed messages: no input text can
//! escape through a diagnostic. File names are never an input.

pub mod grey_levels;
pub mod keep_list;
mod parse;
pub mod pixel_data;
pub mod scale_tags;

use shapes::errors::{EngineError, ErrorCode};
use shapes::film::*;

use crate::{check_dimensions, decoded_film::DecodedFilm, invalid_film};
use pixel_data::PixelHeader;

pub(crate) fn read(bytes: &[u8], kind: FilmKind) -> Result<DecodedFilm, EngineError> {
    let input = parse::input(bytes)?;
    let data = parse::dataset(&input.bytes, input.syntax)?;
    let mut tags = keep_list::read(&data, input.syntax);
    let mut flags = input.flags;
    if data.text((8, 0x16)) == Some("1.2.840.10008.1.3.10") {
        return Err(EngineError::new(
            ErrorCode::FilmIsADirectory,
            "a DICOM directory is not a film",
        ));
    }
    if data.get((0x7fe0, 0x10)).is_none() {
        return Err(EngineError::new(
            ErrorCode::FilmNoPixels,
            "the DICOM contains no pixel data",
        ));
    }
    let native = matches!(
        input.syntax,
        TransferSyntax::ImplicitLittleEndian
            | TransferSyntax::ExplicitLittleEndian
            | TransferSyntax::ExplicitBigEndian
            | TransferSyntax::DeflatedLittleEndian
    );
    if data
        .get((0x7fe0, 0x10))
        .is_some_and(|pixels| pixels.encapsulated == native)
    {
        return Err(invalid_film());
    }
    let width = u32::from(data.required_word((0x28, 0x11))?);
    let height = u32::from(data.required_word((0x28, 0x10))?);
    let bits_allocated = data.required_word((0x28, 0x100))?;
    if !matches!(bits_allocated, 8 | 16) {
        return Err(EngineError::new(
            ErrorCode::FilmUnsupportedBits,
            "only eight- and sixteen-bit containers are supported",
        ));
    }
    let samples_per_pixel = data.required_word((0x28, 2))?;
    if !matches!(samples_per_pixel, 1 | 3) {
        return Err(EngineError::new(
            ErrorCode::FilmUnsupportedColour,
            "the component layout is unsupported",
        ));
    }
    let photometric = tags.photometric.ok_or_else(|| {
        EngineError::new(
            ErrorCode::FilmUnsupportedColour,
            "the photometric interpretation is unsupported",
        )
    })?;
    if (samples_per_pixel == 1)
        != matches!(
            photometric,
            Photometric::Monochrome1 | Photometric::Monochrome2 | Photometric::PaletteColour
        )
    {
        return Err(invalid_film());
    }
    let planar = if samples_per_pixel == 3 {
        data.required_word((0x28, 6))?
    } else {
        0
    };
    if planar > 1 {
        return Err(invalid_film());
    }
    let bytes_per_pixel = match input.syntax {
        TransferSyntax::Jpeg2000Lossless | TransferSyntax::Jpeg2000 => 22,
        TransferSyntax::JpegLossless | TransferSyntax::JpegLosslessFirstOrder => 18,
        TransferSyntax::JpegBaseline | TransferSyntax::JpegExtended => 10,
        _ => 8,
    };
    check_dimensions(
        width,
        height,
        bytes.len(),
        bytes_per_pixel * usize::from(samples_per_pixel),
    )?;
    let stored = data.required_word((0x28, 0x101))?;
    let high = data.required_word((0x28, 0x102))?;
    let representation = data.required_word((0x28, 0x103))?;
    if !(1..=16).contains(&stored)
        || high >= bits_allocated
        || high + 1 < stored
        || representation > 1
    {
        return Err(EngineError::new(
            ErrorCode::FilmUnsupportedBits,
            "the stored bit layout is unsupported",
        ));
    }
    let frames = if data.get((0x28, 8)).is_some() {
        tags.number_of_frames.ok_or_else(invalid_film)?
    } else {
        1
    };
    let frame = parse::frame(&data, input.syntax, frames)?;
    let header = PixelHeader {
        width,
        height,
        bits_allocated: bits_allocated as u8,
        samples_per_pixel: samples_per_pixel as u8,
        planar_configuration: planar as u8,
        photometric,
    };
    let mut raw = pixel_data::decode(&frame, input.syntax, header)?;
    flags.append(&mut raw.flags);
    let (image, bits_stored, values) = grey_levels::store(&data, header, raw, &tags, &mut flags)?;
    let default_window = grey_levels::window(&data, &image, bits_stored, &values, &tags)?;
    scale_tags::validate(&data, &mut tags, &mut flags);
    if frames > 1 {
        flags.push(IntakeFlag::MultiFrame);
    }
    if tags.presentation_intent == Some(PresentationIntent::ForProcessing)
        || tags.sop_class.as_deref() == Some("1.2.840.10008.5.1.4.1.1.1.1.1")
    {
        flags.push(IntakeFlag::ForProcessing);
    }
    if !matches!(
        tags.sop_class.as_deref(),
        Some(
            "1.2.840.10008.5.1.4.1.1.1"
                | "1.2.840.10008.5.1.4.1.1.1.1"
                | "1.2.840.10008.5.1.4.1.1.1.1.1"
                | "1.2.840.10008.5.1.4.1.1.7"
                | "1.2.840.10008.5.1.4.1.1.7.2"
                | "1.2.840.10008.5.1.4.1.1.7.3"
        )
    ) {
        flags.push(IntakeFlag::UnusualSopClass);
    }
    if tags.lossy_compression == Some(true) {
        flags.push(IntakeFlag::Lossy);
    }
    if tags.burned_in_annotation == Some(true) {
        flags.push(IntakeFlag::BurnedInAnnotation);
    }
    flags.sort_unstable();
    flags.dedup();
    Ok(DecodedFilm {
        image,
        facts: FilmFacts {
            kind,
            width,
            height,
            bits_stored,
            source: SourceFormat::Dicom,
            kept_tags: Some(tags),
            values,
            flags,
            default_window,
            exif_quarter_turns: None,
        },
    })
}

#[cfg(test)]
mod tests;
