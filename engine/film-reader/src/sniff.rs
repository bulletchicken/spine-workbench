//! PNG, JPEG or DICOM, judged from the first bytes only, never from a file
//! name.

use shapes::errors::{EngineError, ErrorCode};
use shapes::film::SourceFormat;

/// Recognise a film from its signature, including DICOM without a preamble
/// or file meta information. A recognised signature still needs decoding.
pub fn sniff(bytes: &[u8]) -> Result<SourceFormat, EngineError> {
    if bytes.get(128..132) == Some(b"DICM") {
        return Ok(SourceFormat::Dicom);
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Ok(SourceFormat::Png);
    }
    if bytes.starts_with(b"\xff\xd8\xff") {
        return Ok(SourceFormat::Jpeg);
    }
    if let Some(header) = bytes.get(..8) {
        let group = u16::from_le_bytes([header[0], header[1]]);
        let element = u16::from_le_bytes([header[2], header[3]]);
        let explicit = known_vr(&header[4..6]);
        let implicit = matches!(element, 0x0005 | 0x0008 | 0x0016 | 0x0018)
            && u32::from_le_bytes([header[4], header[5], header[6], header[7]]) < 1024;
        if (group == 2 && element <= 0x0102 && explicit) || (group == 8 && (explicit || implicit)) {
            return Ok(SourceFormat::Dicom);
        }
    }
    let message = if bytes.starts_with(b"%PDF") {
        "This is a PDF; export a PNG, JPEG or DICOM image."
    } else if bytes.starts_with(b"PK\x03\x04") {
        "This is a ZIP archive; choose a PNG, JPEG or DICOM image."
    } else if bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*") {
        "This is a TIFF; export a PNG, JPEG or DICOM image."
    } else if bytes.starts_with(b"\0\0\0\x0cjP  ") || bytes.starts_with(b"\xff\x4f\xff\x51") {
        "This is JPEG 2000 outside DICOM; export a PNG or DICOM image."
    } else {
        "The bytes do not identify a PNG, JPEG or DICOM image."
    };
    Err(EngineError::new(ErrorCode::FilmUnknownFormat, message))
}

fn known_vr(vr: &[u8]) -> bool {
    matches!(
        vr,
        b"AE"
            | b"AS"
            | b"AT"
            | b"CS"
            | b"DA"
            | b"DS"
            | b"DT"
            | b"FD"
            | b"FL"
            | b"IS"
            | b"LO"
            | b"LT"
            | b"OB"
            | b"OD"
            | b"OF"
            | b"OL"
            | b"OV"
            | b"OW"
            | b"PN"
            | b"SH"
            | b"SL"
            | b"SQ"
            | b"SS"
            | b"ST"
            | b"SV"
            | b"TM"
            | b"UC"
            | b"UI"
            | b"UL"
            | b"UN"
            | b"UR"
            | b"US"
            | b"UT"
            | b"UV"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signatures_and_datasets_are_recognised_without_names() {
        assert_eq!(sniff(b"\x89PNG\r\n\x1a\n").unwrap(), SourceFormat::Png);
        assert_eq!(sniff(b"\xff\xd8\xff").unwrap(), SourceFormat::Jpeg);
        let mut dicom = [0; 132];
        dicom[128..].copy_from_slice(b"DICM");
        assert_eq!(sniff(&dicom).unwrap(), SourceFormat::Dicom);
        for prefix in [
            &b"\x02\0\x10\0UI\x04\0"[..],
            &b"\x08\0\x16\0UI\x04\0"[..],
            &b"\x08\0\x16\0\x04\0\0\0"[..],
        ] {
            assert_eq!(sniff(prefix).unwrap(), SourceFormat::Dicom);
        }
    }

    #[test]
    fn near_misses_are_refused() {
        for bytes in [
            &b"DICM"[..],
            &b"\x89PNG\r\n\x1aX"[..],
            &b"\xff\xd8\0"[..],
            &b"\x02\0\x10\0ZZ\x04\0"[..],
            &b"II*\0"[..],
            &b"%PDF"[..],
            &b"PK\x03\x04"[..],
            &b"\0\0\0\x0cjP  "[..],
            &[0; 131][..],
        ] {
            assert_eq!(sniff(bytes).unwrap_err().code, ErrorCode::FilmUnknownFormat);
        }
    }
}
