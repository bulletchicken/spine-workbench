//! DICOM RLE byte planes (PS3.5 Annex G), rebuilt into interleaved words.

use super::{PixelHeader, RawPixels, truncated};
use shapes::errors::EngineError;
use shapes::film::Photometric;

pub(super) fn decode(frame: &[u8], header: PixelHeader) -> Result<RawPixels, EngineError> {
    if frame.len() < 64 {
        return Err(truncated());
    }
    let word = |at: usize| u32::from_le_bytes(frame[at..at + 4].try_into().unwrap());
    let segments = word(0) as usize;
    let components = usize::from(header.samples_per_pixel);
    let bytes_per_sample = usize::from(header.bits_allocated / 8);
    if segments != components * bytes_per_sample || !(1..=15).contains(&segments) {
        return Err(crate::invalid_film());
    }
    let count = header.width as usize * header.height as usize;
    let mut samples = crate::allocate(count * components)?;
    samples.resize(count * components, 0);
    for segment in 0..segments {
        let start = word(4 + segment * 4) as usize;
        let end = if segment + 1 < segments {
            word(8 + segment * 4) as usize
        } else {
            frame.len()
        };
        if start < 64 || start >= end || end > frame.len() {
            return Err(crate::invalid_film());
        }
        let plane = unpack(&frame[start..end], count)?;
        let component = segment / bytes_per_sample;
        let shift = (bytes_per_sample - 1 - segment % bytes_per_sample) * 8;
        for (i, &value) in plane.iter().enumerate() {
            samples[i * components + component] |= u16::from(value) << shift;
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

fn unpack(bytes: &[u8], count: usize) -> Result<Vec<u8>, EngineError> {
    let mut out = crate::allocate(count)?;
    let mut at = 0;
    while out.len() < count {
        let control = *bytes.get(at).ok_or_else(truncated)? as i8;
        at += 1;
        match control {
            0..=127 => {
                let n = control as usize + 1;
                if n > count - out.len() {
                    return Err(crate::invalid_film());
                }
                out.extend_from_slice(bytes.get(at..at + n).ok_or_else(truncated)?);
                at += n;
            }
            -127..=-1 => {
                let n = (1i16 - i16::from(control)) as usize;
                if n > count - out.len() {
                    return Err(crate::invalid_film());
                }
                let value = *bytes.get(at).ok_or_else(truncated)?;
                at += 1;
                out.resize(out.len() + n, value);
            }
            -128 => {}
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annex_g_repeat_literal_and_no_op_have_exact_values() {
        assert_eq!(unpack(&[0x80, 0xfe, 10, 0, 7], 4).unwrap(), [10, 10, 10, 7]);
        assert!(unpack(&[0xfe], 4).is_err());
        assert!(unpack(&[4, 1, 2, 3, 4, 5], 4).is_err());
    }
}
