//! The only constructor of public DICOM tags. Values outside this list
//! are skipped by the parser; text enumerations are validated here.

use super::parse::Dataset;
use shapes::film::*;

pub(super) fn read(data: &Dataset<'_>, syntax: TransferSyntax) -> KeptTags {
    let photometric = photometric(data.text((0x28, 4)));
    KeptTags {
        sop_class: data
            .text((8, 0x16))
            .filter(|value| known_sop(value))
            .map(str::to_owned),
        modality: data
            .text((8, 0x60))
            .filter(|value| {
                matches!(
                    *value,
                    "CR" | "DX"
                        | "CT"
                        | "MR"
                        | "OT"
                        | "XA"
                        | "RF"
                        | "MG"
                        | "PX"
                        | "IO"
                        | "US"
                        | "SC"
                )
            })
            .map(str::to_owned),
        presentation_intent: match data.text((8, 0x68)) {
            Some("FOR PRESENTATION") => Some(PresentationIntent::ForPresentation),
            Some("FOR PROCESSING") => Some(PresentationIntent::ForProcessing),
            _ => None,
        },
        manufacturer: short_text(data, (8, 0x70)),
        model_name: short_text(data, (8, 0x1090)),
        photometric,
        bits_stored: data
            .word((0x28, 0x101))
            .and_then(|value| u8::try_from(value).ok()),
        pixel_spacing: pair(data, (0x28, 0x30)),
        imager_pixel_spacing: pair(data, (0x18, 0x1164)),
        nominal_scanned_pixel_spacing: pair(data, (0x18, 0x2010)),
        pixel_spacing_calibration: match data.text((0x28, 0x0a02)) {
            Some("GEOMETRY") => Some(SpacingCalibration::Geometry),
            Some("FIDUCIAL") => Some(SpacingCalibration::Fiducial),
            _ => None,
        },
        estimated_magnification: data.number((0x18, 0x1114)),
        distance_source_to_detector: data.number((0x18, 0x1110)),
        distance_source_to_patient: data.number((0x18, 0x1111)),
        pixel_aspect_ratio: integer_pair(data, (0x28, 0x34)),
        patient_orientation: orientation(data),
        view_position: match data.text((0x18, 0x5101)) {
            Some("AP") => Some(ViewPosition::Ap),
            Some("PA") => Some(ViewPosition::Pa),
            Some("LL") => Some(ViewPosition::Ll),
            Some("RL") => Some(ViewPosition::Rl),
            Some("RLD") => Some(ViewPosition::Rld),
            Some("LLD") => Some(ViewPosition::Lld),
            Some("RLO") => Some(ViewPosition::Rlo),
            Some("LLO") => Some(ViewPosition::Llo),
            Some("LATERAL") => Some(ViewPosition::Lateral),
            _ => None,
        },
        image_laterality: match data.text((0x20, 0x62)) {
            Some("L") => Some(Laterality::Left),
            Some("R") => Some(Laterality::Right),
            Some("B") => Some(Laterality::Both),
            Some("U") => Some(Laterality::Unpaired),
            _ => None,
        },
        rescale_slope: data.number((0x28, 0x1053)),
        rescale_intercept: data.number((0x28, 0x1052)),
        window_centre: data.number((0x28, 0x1050)),
        window_width: data.number((0x28, 0x1051)),
        window_function: match data.text((0x28, 0x1056)) {
            Some("LINEAR") => Some(WindowFunction::Linear),
            Some("LINEAR_EXACT") => Some(WindowFunction::LinearExact),
            Some("SIGMOID") => Some(WindowFunction::Sigmoid),
            _ => None,
        },
        shutters: shutters(data),
        lossy_compression: match data.text((0x28, 0x2110)) {
            Some("00") => Some(false),
            Some("01") => Some(true),
            _ => None,
        },
        lossy_ratio: data.number((0x28, 0x2112)),
        burned_in_annotation: match data.text((0x28, 0x301)) {
            Some("YES") => Some(true),
            Some("NO") => Some(false),
            _ => None,
        },
        transfer_syntax: Some(syntax),
        number_of_frames: data
            .text((0x28, 8))
            .and_then(|value| value.parse().ok())
            .filter(|value| *value > 0),
    }
}

pub(super) fn photometric(value: Option<&str>) -> Option<Photometric> {
    match value? {
        "MONOCHROME1" => Some(Photometric::Monochrome1),
        "MONOCHROME2" => Some(Photometric::Monochrome2),
        "RGB" => Some(Photometric::Rgb),
        "YBR_FULL" => Some(Photometric::YbrFull),
        "YBR_FULL_422" => Some(Photometric::YbrFull422),
        "YBR_RCT" => Some(Photometric::YbrRct),
        "YBR_ICT" => Some(Photometric::YbrIct),
        "PALETTE COLOR" => Some(Photometric::PaletteColour),
        _ => None,
    }
}

fn known_sop(value: &str) -> bool {
    matches!(
        value,
        "1.2.840.10008.5.1.4.1.1.1"
            | "1.2.840.10008.5.1.4.1.1.1.1"
            | "1.2.840.10008.5.1.4.1.1.1.1.1"
            | "1.2.840.10008.5.1.4.1.1.7"
            | "1.2.840.10008.5.1.4.1.1.7.2"
            | "1.2.840.10008.5.1.4.1.1.7.3"
            | "1.2.840.10008.5.1.4.1.1.2"
            | "1.2.840.10008.5.1.4.1.1.4"
    )
}

fn short_text(data: &Dataset<'_>, tag: (u16, u16)) -> Option<String> {
    data.text(tag).map(|value| {
        value
            .chars()
            .filter(|ch| !ch.is_control())
            .take(64)
            .collect()
    })
}

pub(super) fn pair(data: &Dataset<'_>, tag: (u16, u16)) -> Option<[f64; 2]> {
    let values = data.numbers(tag)?;
    (values.len() == 2).then(|| [values[0], values[1]])
}

fn integer_pair(data: &Dataset<'_>, tag: (u16, u16)) -> Option<[u32; 2]> {
    let values = pair(data, tag)?;
    values
        .iter()
        .all(|value| *value > 0.0 && *value <= u32::MAX as f64 && value.fract() == 0.0)
        .then(|| [values[0] as u32, values[1] as u32])
}

fn orientation(data: &Dataset<'_>) -> Option<[String; 2]> {
    let mut parts = data.text((0x20, 0x20))?.split('\\');
    let a = parts.next()?;
    let b = parts.next()?;
    let valid = |part: &str| {
        !part.is_empty() && part.len() <= 3 && part.bytes().all(|ch| b"APLRHF".contains(&ch))
    };
    (parts.next().is_none() && valid(a) && valid(b)).then(|| [a.to_owned(), b.to_owned()])
}

fn shutters(data: &Dataset<'_>) -> Option<Vec<Shutter>> {
    let mut shutters = Vec::new();
    for shape in data.text((0x18, 0x1600))?.split('\\') {
        let shutter = match shape {
            "RECTANGULAR" => {
                let read = |tag| {
                    data.text((0x18, tag))?
                        .parse::<u32>()
                        .ok()
                        .filter(|value| *value > 0)
                };
                let (left, right, upper, lower) =
                    (read(0x1602)?, read(0x1604)?, read(0x1606)?, read(0x1608)?);
                if left > right || upper > lower {
                    return None;
                }
                Shutter::Rectangle {
                    left,
                    right,
                    upper,
                    lower,
                }
            }
            "CIRCULAR" => {
                let [centre_row, centre_column] = pair(data, (0x18, 0x1610))?;
                let radius = data.number((0x18, 0x1612))?;
                if radius <= 0.0 {
                    return None;
                }
                Shutter::Circle {
                    centre_row,
                    centre_column,
                    radius,
                }
            }
            "POLYGONAL" => {
                let values = data.numbers((0x18, 0x1620))?;
                if values.len() < 6 || values.len() % 2 != 0 {
                    return None;
                }
                Shutter::Polygon {
                    vertices: values
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .map(|pair| [pair[0], pair[1]])
                        .collect(),
                }
            }
            _ => continue,
        };
        shutters.push(shutter);
    }
    (!shutters.is_empty()).then_some(shutters)
}
