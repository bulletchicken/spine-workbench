//! JPEG 2000 decoding with SIZ precision and signedness checked before
//! allocation. The pinned decoder ignores SIZ signedness and adds a sign
//! shift to every component; signed samples have that shift removed here.

use super::{PixelHeader, RawPixels, size_mismatch, truncated};
use hayro_jpeg2000::{DecodeSettings, DecoderContext, Image};
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::{IntakeFlag, Photometric, TransferSyntax};

pub(super) fn decode(
    bytes: &[u8],
    syntax: TransferSyntax,
    header: PixelHeader,
) -> Result<RawPixels, EngineError> {
    let (codestream, wrapped) = codestream(bytes)?;
    let size = siz(codestream)?;
    if size.width != header.width || size.height != header.height {
        return Err(size_mismatch());
    }
    crate::check_dimensions(size.width, size.height, bytes.len(), 22)?;
    let monochrome = matches!(
        header.photometric,
        Photometric::Monochrome1 | Photometric::Monochrome2
    );
    let components = if monochrome { 1 } else { size.components };
    if components != header.samples_per_pixel || !matches!(components, 1 | 3) {
        return Err(EngineError::new(
            ErrorCode::FilmUnsupportedColour,
            "The JPEG 2000 component layout is not supported.",
        ));
    }
    let image =
        Image::new(codestream, &DecodeSettings::default()).map_err(|_| crate::invalid_film())?;
    let mut context = DecoderContext::default();
    let decoded = image
        .decode(&mut context)
        .map_err(|_| crate::invalid_film())?;
    let planes = decoded.components();
    let count = size.width as usize * size.height as usize;
    if planes.len() != usize::from(size.components)
        || planes
            .iter()
            .take(usize::from(components))
            .any(|plane| plane.samples().len() != count || plane.bit_depth() != size.precision)
    {
        return Err(size_mismatch());
    }
    let mut flags = Vec::new();
    if wrapped {
        flags.push(IntakeFlag::Jp2Wrapper);
    }
    if syntax == TransferSyntax::Jpeg2000 {
        flags.push(IntakeFlag::Lossy);
    }
    let shift = if size.signed {
        1i32 << (size.precision - 1)
    } else {
        0
    };
    let low = -shift;
    let high = (1i32 << size.precision) - 1 - shift;
    let mask = (1u32 << size.precision) - 1;
    let mut samples = crate::allocate(count * usize::from(components))?;
    let mut exact = true;
    for i in 0..count {
        for plane in planes.iter().take(usize::from(components)) {
            let sample = plane.samples()[i] - shift as f32;
            if !sample.is_finite() {
                return Err(crate::invalid_film());
            }
            let rounded = sample.round();
            exact &= (sample - rounded).abs() < 1e-3;
            let value = rounded.clamp(low as f32, high as f32) as i32;
            samples.push((value as u32 & mask) as u16);
        }
    }
    if !exact && syntax == TransferSyntax::Jpeg2000Lossless {
        flags.extend([IntakeFlag::J2kNotExact, IntakeFlag::Lossy]);
    }
    Ok(RawPixels {
        samples,
        precision: size.precision,
        signed: Some(size.signed),
        native: false,
        colour_rgb: components == 3,
        flags,
    })
}

struct Size {
    width: u32,
    height: u32,
    components: u8,
    precision: u8,
    signed: bool,
}

fn siz(bytes: &[u8]) -> Result<Size, EngineError> {
    if !bytes.starts_with(&[0xff, 0x4f, 0xff, 0x51]) || bytes.len() < 42 {
        return Err(crate::invalid_film());
    }
    let u16_at = |at: usize| u16::from_be_bytes([bytes[at], bytes[at + 1]]);
    let u32_at = |at: usize| u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap());
    let length = usize::from(u16_at(4));
    let components = usize::from(u16_at(40));
    if !matches!(components, 1 | 3) || length != 38 + 3 * components {
        return Err(EngineError::new(
            ErrorCode::FilmUnsupportedColour,
            "The JPEG 2000 SIZ layout is not supported.",
        ));
    }
    let component_info = bytes.get(42..4 + length).ok_or_else(truncated)?;
    let precision = (component_info[0] & 0x7f) + 1;
    if precision > 16 {
        return Err(EngineError::new(
            ErrorCode::FilmUnsupportedBits,
            "JPEG 2000 precision exceeds 16 bits.",
        ));
    }
    // Subsampled components would need interpolation, which changes the
    // stored grid. Film components must share its full resolution.
    if component_info
        .as_chunks::<3>()
        .0
        .iter()
        .any(|part| *part != [component_info[0], 1, 1])
    {
        return Err(EngineError::new(
            ErrorCode::FilmUnsupportedColour,
            "JPEG 2000 components must have matching precision and resolution.",
        ));
    }
    let xs = u32_at(8);
    let ys = u32_at(12);
    let xo = u32_at(16);
    let yo = u32_at(20);
    let xt = u32_at(24);
    let yt = u32_at(28);
    let xto = u32_at(32);
    let yto = u32_at(36);
    if xo >= xs
        || yo >= ys
        || xt == 0
        || yt == 0
        || xto > xo
        || yto > yo
        || u64::from(xto) + u64::from(xt) <= u64::from(xo)
        || u64::from(yto) + u64::from(yt) <= u64::from(yo)
    {
        return Err(crate::invalid_film());
    }
    crate::check_dimensions(xs, ys, bytes.len(), 22)?;
    Ok(Size {
        width: xs - xo,
        height: ys - yo,
        components: components as u8,
        precision,
        signed: component_info[0] & 0x80 != 0,
    })
}

fn codestream(bytes: &[u8]) -> Result<(&[u8], bool), EngineError> {
    if bytes.starts_with(&[0xff, 0x4f, 0xff, 0x51]) {
        return Ok((bytes, false));
    }
    if !bytes.starts_with(b"\0\0\0\x0cjP  \r\n\x87\n") {
        return Err(crate::invalid_film());
    }
    let mut at = 0;
    while at < bytes.len() {
        let box_header = bytes.get(at..at + 8).ok_or_else(truncated)?;
        let mut length = u64::from(u32::from_be_bytes(box_header[..4].try_into().unwrap()));
        let mut header_size = 8;
        if length == 1 {
            length = u64::from_be_bytes(
                bytes
                    .get(at + 8..at + 16)
                    .ok_or_else(truncated)?
                    .try_into()
                    .unwrap(),
            );
            header_size = 16;
        } else if length == 0 {
            length = (bytes.len() - at) as u64;
        }
        let length = usize::try_from(length).map_err(|_| crate::invalid_film())?;
        if length < header_size || length > bytes.len() - at {
            return Err(truncated());
        }
        if &box_header[4..8] == b"jp2c" {
            return Ok((&bytes[at + header_size..at + length], true));
        }
        at += length;
    }
    Err(crate::invalid_film())
}
