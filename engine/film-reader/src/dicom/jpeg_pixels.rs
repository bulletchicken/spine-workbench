//! JPEG frame preflight and decoding. Size and component checks happen
//! before the third-party decoder can allocate a pixel buffer.

use super::{PixelHeader, RawPixels, size_mismatch, truncated, unsupported};
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::{IntakeFlag, Photometric, TransferSyntax};
use std::io::Cursor;

#[path = "lossless_jpeg.rs"]
mod lossless;

pub(super) fn decode(
    bytes: &[u8],
    syntax: TransferSyntax,
    header: PixelHeader,
) -> Result<RawPixels, EngineError> {
    let frame = frame_header(bytes)?;
    if frame.width != header.width || frame.height != header.height {
        return Err(size_mismatch());
    }
    if frame.components != header.samples_per_pixel {
        return Err(crate::invalid_film());
    }
    let lossless = matches!(
        syntax,
        TransferSyntax::JpegLossless | TransferSyntax::JpegLosslessFirstOrder
    );
    if lossless != (frame.marker == 0xc3) {
        return Err(unsupported(
            "The JPEG process does not match the transfer syntax.",
        ));
    }
    if !lossless && frame.precision != 8 {
        return Err(unsupported("12-bit lossy JPEG is not supported."));
    }
    if !matches!(frame.marker, 0xc0 | 0xc1 | 0xc3) {
        return Err(unsupported("The JPEG process is not supported."));
    }
    if lossless {
        let samples = lossless::decode(
            bytes,
            header,
            frame.precision,
            syntax == TransferSyntax::JpegLosslessFirstOrder,
        )?;
        return Ok(RawPixels {
            samples,
            precision: frame.precision,
            signed: None,
            native: false,
            colour_rgb: header.photometric == Photometric::Rgb,
            flags: Vec::new(),
        });
    }
    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(bytes));
    decoder.set_max_decoding_buffer_size(900 * 1024 * 1024);
    let decoded = decoder.decode().map_err(|_| crate::invalid_film())?;
    let count = header.width as usize * header.height as usize * usize::from(frame.components);
    let bytes_per_sample = if frame.precision <= 8 { 1 } else { 2 };
    if decoded.len() != count * bytes_per_sample {
        return Err(size_mismatch());
    }
    let mut samples = crate::allocate(count)?;
    if bytes_per_sample == 1 {
        samples.extend(decoded.iter().map(|&value| u16::from(value)));
    } else {
        samples.extend(
            decoded
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&pair| u16::from_ne_bytes(pair)),
        );
    }
    Ok(RawPixels {
        samples,
        precision: frame.precision,
        signed: None,
        native: false,
        colour_rgb: !lossless || header.photometric == Photometric::Rgb,
        flags: if lossless {
            Vec::new()
        } else {
            vec![IntakeFlag::Lossy]
        },
    })
}

struct FrameHeader {
    width: u32,
    height: u32,
    components: u8,
    precision: u8,
    marker: u8,
}

fn frame_header(bytes: &[u8]) -> Result<FrameHeader, EngineError> {
    if !bytes.starts_with(&[0xff, 0xd8]) {
        return Err(crate::invalid_film());
    }
    let mut at = 2;
    loop {
        if bytes.get(at) != Some(&0xff) {
            return Err(crate::invalid_film());
        }
        while bytes.get(at) == Some(&0xff) {
            at += 1;
        }
        let marker = *bytes.get(at).ok_or_else(truncated)?;
        at += 1;
        if matches!(marker, 0xd9 | 0xda | 0x00) {
            return Err(crate::invalid_film());
        }
        if marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
            continue;
        }
        let size = bytes.get(at..at + 2).ok_or_else(truncated)?;
        let size = usize::from(u16::from_be_bytes([size[0], size[1]]));
        if size < 2 {
            return Err(crate::invalid_film());
        }
        let segment = bytes.get(at + 2..at + size).ok_or_else(truncated)?;
        if (0xc0..=0xcf).contains(&marker) && !matches!(marker, 0xc4 | 0xc8 | 0xcc) {
            if segment.len() < 6 {
                return Err(truncated());
            }
            let precision = segment[0];
            let height = u32::from(u16::from_be_bytes([segment[1], segment[2]]));
            let width = u32::from(u16::from_be_bytes([segment[3], segment[4]]));
            let components = segment[5];
            if !matches!(components, 1 | 3) || segment.len() != 6 + 3 * usize::from(components) {
                return Err(EngineError::new(
                    ErrorCode::FilmUnsupportedColour,
                    "The JPEG component layout is not supported.",
                ));
            }
            if !(2..=16).contains(&precision) {
                return Err(EngineError::new(
                    ErrorCode::FilmUnsupportedBits,
                    "The JPEG precision is not supported.",
                ));
            }
            return Ok(FrameHeader {
                width,
                height,
                components,
                precision,
                marker,
            });
        }
        at += size;
    }
}
