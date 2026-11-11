//! The single transfer-syntax decoder shelf. JPEG-LS deliberately has no
//! decoder yet; it and unknown syntaxes return `film-unsupported-compression`.
//! Codec diagnostics never escape this module: only fixed messages do.

use shapes::errors::{EngineError, ErrorCode};
use shapes::film::{IntakeFlag, Photometric, TransferSyntax};

#[path = "jpeg_pixels.rs"]
mod jpeg;
#[path = "jpeg2000_pixels.rs"]
mod jpeg2000;
#[path = "rle_pixels.rs"]
mod rle;

/// Only the numeric header fields needed to allocate and decode a frame.
#[derive(Clone, Copy)]
pub(crate) struct PixelHeader {
    pub width: u32,
    pub height: u32,
    pub bits_allocated: u8,
    pub samples_per_pixel: u8,
    pub planar_configuration: u8,
    pub photometric: Photometric,
}

/// Bit patterns in pixel order, components interleaved. Native and RLE
/// samples still need High Bit / Bits Stored masking. JPEG samples use
/// codestream precision; JPEG 2000 also supplies codestream signedness.
pub(crate) struct RawPixels {
    pub samples: Vec<u16>,
    pub precision: u8,
    pub signed: Option<bool>,
    pub native: bool,
    pub colour_rgb: bool,
    pub flags: Vec<IntakeFlag>,
}

/// A fixed vocabulary, never a copy of an unrecognised UID from the input.
pub(crate) fn transfer_syntax(uid: &str) -> Result<TransferSyntax, EngineError> {
    use TransferSyntax::*;
    match uid {
        "1.2.840.10008.1.2" => Ok(ImplicitLittleEndian),
        "1.2.840.10008.1.2.1" => Ok(ExplicitLittleEndian),
        "1.2.840.10008.1.2.2" => Ok(ExplicitBigEndian),
        "1.2.840.10008.1.2.1.99" => Ok(DeflatedLittleEndian),
        "1.2.840.10008.1.2.1.98" => Ok(EncapsulatedUncompressed),
        "1.2.840.10008.1.2.5" => Ok(RleLossless),
        "1.2.840.10008.1.2.4.50" => Ok(JpegBaseline),
        "1.2.840.10008.1.2.4.51" => Ok(JpegExtended),
        "1.2.840.10008.1.2.4.57" => Ok(JpegLossless),
        "1.2.840.10008.1.2.4.70" => Ok(JpegLosslessFirstOrder),
        "1.2.840.10008.1.2.4.90" => Ok(Jpeg2000Lossless),
        "1.2.840.10008.1.2.4.91" => Ok(Jpeg2000),
        "1.2.840.10008.1.2.4.80" | "1.2.840.10008.1.2.4.81" => {
            Err(unsupported("JPEG-LS compression is not supported."))
        }
        _ => Err(unsupported("The DICOM compression is not supported.")),
    }
}

/// This match is the one place a supported transfer syntax selects its
/// decoder. Deflation is a dataset transform, already undone by the parser.
pub(crate) fn decode(
    frame: &[u8],
    syntax: TransferSyntax,
    header: PixelHeader,
) -> Result<RawPixels, EngineError> {
    use TransferSyntax::*;
    if !matches!(header.bits_allocated, 8 | 16) {
        return Err(EngineError::new(
            ErrorCode::FilmUnsupportedBits,
            "Only 8- and 16-bit allocated samples are supported.",
        ));
    }
    if !matches!(header.samples_per_pixel, 1 | 3) || header.planar_configuration > 1 {
        return Err(EngineError::new(
            ErrorCode::FilmUnsupportedColour,
            "The DICOM sample layout is not supported.",
        ));
    }
    let cost = match syntax {
        Jpeg2000 | Jpeg2000Lossless => 22,
        JpegLossless | JpegLosslessFirstOrder => 18,
        JpegBaseline | JpegExtended => 10,
        _ => 8,
    };
    crate::check_dimensions(header.width, header.height, frame.len(), cost)?;
    match syntax {
        ImplicitLittleEndian
        | ExplicitLittleEndian
        | DeflatedLittleEndian
        | EncapsulatedUncompressed => native(frame, header, false),
        ExplicitBigEndian => native(frame, header, true),
        RleLossless => rle::decode(frame, header),
        JpegBaseline | JpegExtended | JpegLossless | JpegLosslessFirstOrder => {
            jpeg::decode(frame, syntax, header)
        }
        Jpeg2000Lossless | Jpeg2000 => jpeg2000::decode(frame, syntax, header),
        JpegLsLossless | JpegLsNearLossless => {
            Err(unsupported("JPEG-LS compression is not supported."))
        }
    }
}

fn native(frame: &[u8], header: PixelHeader, big_endian: bool) -> Result<RawPixels, EngineError> {
    let count = header.width as usize * header.height as usize;
    let components = usize::from(header.samples_per_pixel);
    let bytes_per_sample = usize::from(header.bits_allocated / 8);
    let packed_ybr = header.photometric == Photometric::YbrFull422;
    if packed_ybr && (components != 3 || !header.width.is_multiple_of(2)) {
        return Err(crate::invalid_film());
    }
    let words = count * if packed_ybr { 2 } else { components };
    if frame.len() < words * bytes_per_sample {
        return Err(truncated());
    }
    let word = |i: usize| {
        if bytes_per_sample == 1 {
            u16::from(frame[i])
        } else {
            let pair = [frame[2 * i], frame[2 * i + 1]];
            if big_endian {
                u16::from_be_bytes(pair)
            } else {
                u16::from_le_bytes(pair)
            }
        }
    };
    let mut samples = crate::allocate(count * components)?;
    for i in 0..count {
        for component in 0..components {
            let index = if packed_ybr {
                (i / 2) * 4 + if component == 0 { i % 2 } else { component + 1 }
            } else if header.planar_configuration == 1 {
                component * count + i
            } else {
                i * components + component
            };
            samples.push(word(index));
        }
    }
    Ok(RawPixels {
        samples,
        precision: header.bits_allocated,
        signed: None,
        native: true,
        colour_rgb: header.photometric == Photometric::Rgb,
        flags: Vec::new(),
    })
}

fn unsupported(message: &'static str) -> EngineError {
    EngineError::new(ErrorCode::FilmUnsupportedCompression, message)
}

fn truncated() -> EngineError {
    EngineError::new(ErrorCode::FilmTruncated, "The pixel data ends early.")
}

fn size_mismatch() -> EngineError {
    EngineError::new(
        ErrorCode::FilmSizeMismatch,
        "The codec dimensions do not match the DICOM dimensions.",
    )
}

#[cfg(test)]
#[path = "codec_tests.rs"]
mod tests;
