//! Exact stored codes, signed offsets and bone-bright polarity. Rescale
//! and lookup tables describe the codes; neither quantises the film.

use darkroom::grey_image::{GreyImage, Placement};
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::*;

use super::parse::Dataset;
use super::pixel_data::{PixelHeader, RawPixels};
use crate::{allocate, invalid_film};

pub(super) fn store(
    data: &Dataset<'_>,
    header: PixelHeader,
    raw: RawPixels,
    tags: &KeptTags,
    flags: &mut Vec<IntakeFlag>,
) -> Result<(GreyImage<u16>, u8, PixelValues), EngineError> {
    let declared = data.required_word((0x28, 0x101))?;
    let high = data.required_word((0x28, 0x102))?;
    let representation = data.required_word((0x28, 0x103))?;
    if !(1..=16).contains(&declared)
        || high >= u16::from(header.bits_allocated)
        || high + 1 < declared
        || representation > 1
    {
        return Err(EngineError::new(
            ErrorCode::FilmUnsupportedBits,
            "the stored bit layout is unsupported",
        ));
    }
    let declared = declared as u8;
    let signed = raw.signed.unwrap_or(representation == 1);
    let mut bits = if raw.signed.is_some() {
        raw.precision
    } else {
        declared
    };
    if raw.precision != declared && !raw.native {
        flags.push(IntakeFlag::BitsStoredMismatch);
    }
    let mask = (1u32 << declared) - 1;
    let shift = high as u8 + 1 - declared;
    let decode = |word: u16, flags: &mut Vec<IntakeFlag>| -> i32 {
        if raw.native {
            let pattern = (u32::from(word) >> shift) & mask;
            sample(pattern, declared, signed)
        } else if raw.signed.is_some() {
            sample(u32::from(word), raw.precision, signed)
        } else {
            let value = sample(u32::from(word), raw.precision, signed);
            if signed && (value < -(1i32 << (declared - 1)) || value >= (1i32 << (declared - 1))) {
                if !flags.contains(&IntakeFlag::BitsStoredMismatch) {
                    flags.push(IntakeFlag::BitsStoredMismatch);
                }
                sample(u32::from(word) & mask, declared, true)
            } else {
                value
            }
        }
    };
    let count = header.width as usize * header.height as usize;
    let mut samples = allocate::<i32>(count)?;
    if header.photometric == Photometric::PaletteColour {
        let tables = [
            palette(data, 1, signed)?,
            palette(data, 2, signed)?,
            palette(data, 3, signed)?,
        ];
        bits = tables
            .iter()
            .map(|table| table.1)
            .max()
            .ok_or_else(invalid_film)?;
        if signed {
            return Err(EngineError::new(
                ErrorCode::FilmUnsupportedColour,
                "signed palette indices are unsupported",
            ));
        }
        for &word in &raw.samples {
            let index = decode(word, flags);
            let colour: Vec<u16> = tables
                .iter()
                .map(|(first, _, table)| {
                    table[(index - first).clamp(0, table.len() as i32 - 1) as usize]
                })
                .collect();
            samples.push(i32::from(crate::png_jpeg::luma(
                colour[0], colour[1], colour[2],
            )));
        }
        flags.push(IntakeFlag::ColourConverted);
    } else if header.samples_per_pixel == 1 {
        samples.extend(raw.samples.iter().map(|&word| decode(word, flags)));
    } else {
        if signed {
            return Err(EngineError::new(
                ErrorCode::FilmUnsupportedColour,
                "signed colour components are unsupported",
            ));
        }
        for pixel in raw.samples.as_chunks::<3>().0 {
            let r = decode(pixel[0], flags) as u16;
            let g = decode(pixel[1], flags) as u16;
            let b = decode(pixel[2], flags) as u16;
            let value = if !raw.colour_rgb
                && matches!(
                    header.photometric,
                    Photometric::YbrFull | Photometric::YbrFull422
                ) {
                r
            } else {
                crate::png_jpeg::luma(r, g, b)
            };
            samples.push(i32::from(value));
        }
        flags.push(IntakeFlag::ColourConverted);
    }
    if samples.len() != count {
        return Err(invalid_film());
    }
    let smallest = samples.iter().copied().min().ok_or_else(invalid_film)?;
    let largest = samples.iter().copied().max().ok_or_else(invalid_film)?;
    while bits <= 16 {
        let low = if signed { -(1i32 << (bits - 1)) } else { 0 };
        let high = low + (1i32 << bits) - 1;
        if smallest >= low && largest <= high {
            break;
        }
        bits += 1;
    }
    if bits > 16 {
        return Err(EngineError::new(
            ErrorCode::FilmUnsupportedBits,
            "the samples need more than sixteen bits",
        ));
    }
    if bits != declared {
        flags.push(IntakeFlag::BitsStoredMismatch);
    }
    let offset = if signed { -(1i32 << (bits - 1)) } else { 0 };
    let modality = modality(data, tags, signed)?;
    if let ModalityMap::Linear { slope, intercept } = modality {
        if !(slope * f64::from((1i32 << bits) - 1)).is_finite() {
            return Err(invalid_film());
        }
        for code in [offset, offset + (1i32 << bits) - 1] {
            if !(slope * f64::from(code) + intercept).is_finite() {
                return Err(invalid_film());
            }
        }
    }
    let direction = match &modality {
        ModalityMap::Linear { slope, .. } => *slope > 0.0,
        ModalityMap::Lookup { table, .. } => {
            let tail = table.len().div_ceil(20);
            let start: u64 = table[..tail].iter().map(|&value| u64::from(value)).sum();
            let end: u64 = table[table.len() - tail..]
                .iter()
                .map(|&value| u64::from(value))
                .sum();
            let up = end >= start;
            let against = table
                .windows(2)
                .filter(|pair| {
                    if up {
                        pair[1] < pair[0]
                    } else {
                        pair[1] > pair[0]
                    }
                })
                .count();
            if against * 50 > table.len().saturating_sub(1) {
                flags.push(IntakeFlag::NonMonotoneModalityLut);
            }
            up
        }
    };
    let monochrome1 = header.photometric == Photometric::Monochrome1;
    let inverted = monochrome1 == direction;
    let maximum = (1i32 << bits) - 1;
    let mut pixels = allocate(count)?;
    pixels.extend(samples.into_iter().map(|value| {
        let code = value - offset;
        (if inverted { maximum - code } else { code }) as u16
    }));
    let image = GreyImage::new(header.width, header.height, pixels, Placement::FILM)
        .ok_or_else(invalid_film)?;
    let values = PixelValues {
        offset,
        inverted,
        inverted_by_user: false,
        modality,
    };
    if !monochrome1 && data.text((0x2050, 0x20)) == Some("INVERSE") {
        flags.push(IntakeFlag::PolarityConflict);
    }
    if tags.sop_class.as_deref().is_some_and(|value| {
        value == "1.2.840.10008.5.1.4.1.1.1.1" || value == "1.2.840.10008.5.1.4.1.1.1.1.1"
    }) && !matches!(
        values.modality,
        ModalityMap::Linear {
            slope: 1.0,
            intercept: 0.0
        }
    ) {
        flags.push(IntakeFlag::DxRescaleNotIdentity);
    }
    Ok((image, bits, values))
}

fn sample(pattern: u32, bits: u8, signed: bool) -> i32 {
    let pattern = pattern & ((1u32 << bits) - 1);
    if signed && pattern >= (1 << (bits - 1)) {
        pattern as i32 - (1 << bits)
    } else {
        pattern as i32
    }
}

fn modality(data: &Dataset<'_>, tags: &KeptTags, signed: bool) -> Result<ModalityMap, EngineError> {
    if let Some(sequence) = data.get((0x28, 0x3000)) {
        let item = sequence.items.first().ok_or_else(invalid_film)?;
        let (first_mapped, bits, table) = lookup(item, (0x28, 0x3002), (0x28, 0x3006), signed)?;
        Ok(ModalityMap::Lookup {
            first_mapped,
            bits,
            table,
        })
    } else {
        if (data.get((0x28, 0x1053)).is_some() && tags.rescale_slope.is_none())
            || (data.get((0x28, 0x1052)).is_some() && tags.rescale_intercept.is_none())
        {
            return Err(invalid_film());
        }
        let slope = tags.rescale_slope.unwrap_or(1.0);
        let intercept = tags.rescale_intercept.unwrap_or(0.0);
        if slope == 0.0 {
            return Err(invalid_film());
        }
        Ok(ModalityMap::Linear { slope, intercept })
    }
}

fn palette(
    data: &Dataset<'_>,
    channel: u16,
    signed: bool,
) -> Result<(i32, u8, Vec<u16>), EngineError> {
    lookup(
        data,
        (0x28, 0x1100 + channel),
        (0x28, 0x1200 + channel),
        signed,
    )
}

fn lookup(
    data: &Dataset<'_>,
    descriptor: (u16, u16),
    values: (u16, u16),
    signed: bool,
) -> Result<(i32, u8, Vec<u16>), EngineError> {
    let descriptor = data.get(descriptor).ok_or_else(invalid_film)?;
    let signed = match descriptor.vr {
        Some(vr) if vr == *b"SS" => true,
        Some(vr) if vr == *b"US" => false,
        _ => signed,
    };
    let descriptor = descriptor.bytes;
    if descriptor.len() != 6 {
        return Err(invalid_film());
    }
    let count = data.encoding.word(&descriptor[..2]);
    let count = if count == 0 {
        65_536
    } else {
        usize::from(count)
    };
    let first = data.encoding.word(&descriptor[2..4]);
    let first = if signed {
        i32::from(first as i16)
    } else {
        i32::from(first)
    };
    let bits = data.encoding.word(&descriptor[4..6]);
    if !(1..=16).contains(&bits) {
        return Err(invalid_film());
    }
    let raw = data.get(values).ok_or_else(invalid_film)?.bytes;
    let mut table = allocate(count)?;
    if raw.len() == count * 2 {
        table.extend(
            raw.as_chunks::<2>()
                .0
                .iter()
                .map(|word| data.encoding.word(word)),
        );
    } else if bits <= 8 && raw.len() == count + count % 2 {
        table.extend(raw[..count].iter().map(|&value| u16::from(value)));
    } else {
        return Err(invalid_film());
    }
    if table.iter().any(|&value| u32::from(value) >= 1u32 << bits) {
        return Err(invalid_film());
    }
    Ok((first, bits as u8, table))
}

/// File windows stay in modality units. The stored codes themselves are
/// never windowed; GPU pixels are independently normalised to [0, 1].
pub(crate) fn default_window(
    image: &GreyImage<u16>,
    bits: u8,
    values: &PixelValues,
    tags: Option<&KeptTags>,
) -> DisplayWindow {
    if let Some(tags) = tags
        && let (Some(centre), Some(width)) = (tags.window_centre, tags.window_width)
    {
        let function = tags.window_function.unwrap_or(WindowFunction::Linear);
        let valid = if function == WindowFunction::Linear {
            width >= 1.0
        } else {
            width > 0.0
        };
        if valid && centre.is_finite() && width.is_finite() {
            return DisplayWindow::CentreWidth {
                centre,
                width,
                function,
                from: WindowSource::File,
            };
        }
    }
    crate::thumbnail::default_window(image, bits, values)
}

pub(super) fn window(
    data: &Dataset<'_>,
    image: &GreyImage<u16>,
    bits: u8,
    values: &PixelValues,
    tags: &KeptTags,
) -> Result<DisplayWindow, EngineError> {
    let default = default_window(image, bits, values, Some(tags));
    if matches!(
        default,
        DisplayWindow::CentreWidth {
            from: WindowSource::File,
            ..
        }
    ) {
        return Ok(default);
    }
    if let Some(sequence) = data.get((0x28, 0x3010)) {
        let item = sequence.items.first().ok_or_else(invalid_film)?;
        let signed = values.offset < 0
            || matches!(values.modality, ModalityMap::Linear { slope, intercept }
            if intercept + slope * f64::from(values.offset) < 0.0
                || intercept + slope * f64::from(values.offset + (1i32 << bits) - 1) < 0.0);
        let (first, bits, table) = lookup(item, (0x28, 0x3002), (0x28, 0x3006), signed)?;
        return Ok(DisplayWindow::Lookup {
            first,
            bits,
            table,
            from: WindowSource::FileLookup,
        });
    }
    let padding = data.word((0x28, 0x120)).map(|word| {
        if values.offset < 0 {
            i32::from(word as i16)
        } else {
            i32::from(word)
        }
    });
    let limit = data
        .word((0x28, 0x121))
        .map(|word| {
            if values.offset < 0 {
                i32::from(word as i16)
            } else {
                i32::from(word)
            }
        })
        .or(padding);
    Ok(crate::thumbnail::default_window_where(
        image,
        bits,
        values,
        |x, y, pixel| {
            let code = if values.inverted {
                (1i32 << bits) - 1 - i32::from(pixel)
            } else {
                i32::from(pixel)
            };
            let stored = values.offset + code;
            let padded = padding
                .zip(limit)
                .is_some_and(|(a, b)| (a.min(b)..=a.max(b)).contains(&stored));
            !padded && inside_shutters(x, y, tags.shutters.as_deref().unwrap_or_default())
        },
    ))
}

fn inside_shutters(x: u32, y: u32, shutters: &[Shutter]) -> bool {
    // DICOM shutters use one-based pixel centres; stored image centres are zero-based.
    let column = f64::from(x) + 1.0;
    let row = f64::from(y) + 1.0;
    shutters.iter().all(|shutter| match shutter {
        Shutter::Rectangle {
            left,
            right,
            upper,
            lower,
        } => x + 1 >= *left && x < *right && y + 1 >= *upper && y < *lower,
        Shutter::Circle {
            centre_row,
            centre_column,
            radius,
        } => {
            let dx = column - centre_column;
            let dy = row - centre_row;
            dx * dx + dy * dy <= radius * radius
        }
        Shutter::Polygon { vertices } => {
            if vertices.len() < 3 {
                return false;
            }
            let mut inside = false;
            for (a, b) in vertices
                .iter()
                .zip(vertices.iter().cycle().skip(1))
                .take(vertices.len())
            {
                let [ay, ax] = *a;
                let [by, bx] = *b;
                let cross = (column - ax) * (by - ay) - (row - ay) * (bx - ax);
                if cross == 0.0
                    && (ax.min(bx)..=ax.max(bx)).contains(&column)
                    && (ay.min(by)..=ay.max(by)).contains(&row)
                {
                    return true;
                }
                if (ay > row) != (by > row) && column < (bx - ax) * (row - ay) / (by - ay) + ax {
                    inside = !inside;
                }
            }
            inside
        }
    })
}
