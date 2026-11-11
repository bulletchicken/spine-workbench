//! Films in these tests are written in memory, with independently chosen codes.

use super::*;
use crate::{read_film, test_support::DicomBuilder};

#[test]
fn endian_and_deflated_films_keep_exact_unsigned_codes() {
    for syntax in [
        "1.2.840.10008.1.2",
        "1.2.840.10008.1.2.1",
        "1.2.840.10008.1.2.2",
        "1.2.840.10008.1.2.1.99",
    ] {
        let bytes = DicomBuilder::new(4, 1, 12, &[0, 1, 2048, 4095])
            .syntax(syntax)
            .build();
        let film = read_film(&bytes, FilmKind::Front).expect("generated film opens");
        assert_eq!(film.image.pixels(), &[0, 1, 2048, 4095]);
        assert_eq!(film.facts.bits_stored, 12);
    }
}

#[test]
fn signed_codes_rescale_and_polarity_are_exact_and_reversible() {
    for (photometric, slope, expected) in [
        ("MONOCHROME2", "0.5", [0, 2047, 2048, 4095]),
        ("MONOCHROME1", "0.5", [4095, 2048, 2047, 0]),
        ("MONOCHROME2", "-1", [4095, 2048, 2047, 0]),
        ("MONOCHROME1", "-1", [0, 2047, 2048, 4095]),
    ] {
        let bytes = DicomBuilder::new(4, 1, 12, &[2048, 4095, 0, 2047])
            .signed(true)
            .photometric(photometric)
            .text(0x28, 0x1053, *b"DS", slope)
            .text(0x28, 0x1052, *b"DS", "10.25")
            .build();
        let film = read_film(&bytes, FilmKind::Side).expect("generated film opens");
        assert_eq!(film.image.pixels(), &expected);
        assert_eq!(film.facts.values.offset, -2048);
        for (&p, s) in film.image.pixels().iter().zip([-2048, -1, 0, 2047]) {
            let recovered = -2048
                + if film.facts.values.inverted {
                    4095 - i32::from(p)
                } else {
                    i32::from(p)
                };
            assert_eq!(recovered, s);
        }
    }
}

#[test]
fn non_kept_tags_never_appear_in_any_returned_value_or_error() {
    // Synthetic markers are never interpolated into assertion messages.
    let markers = [
        "SYNTHETIC^PRIVATE",
        "PRIVATE-ID-417",
        "19010203",
        "PRIVATE-HOSPITAL",
        "PRIVATE-STATION",
        "1.2.826.999.417",
        "PRIVATE-ELEMENT",
        "PRIVATE-SERIES",
        "PRIVATE-CALIBRATION",
    ];
    let build = |syntax: &str| {
        let mut builder = DicomBuilder::new(4, 1, 12, &[0, 1, 2048, 4095])
            .syntax(syntax)
            .text(0x28, 0x30, *b"DS", "0.15\\0.16");
        for ((group, element, vr), marker) in [
            (0x10, 0x10, *b"PN"),
            (0x10, 0x20, *b"LO"),
            (0x10, 0x30, *b"DA"),
            (8, 0x80, *b"LO"),
            (8, 0x1010, *b"SH"),
            (8, 0x18, *b"UI"),
            (9, 0x10, *b"LO"),
            (8, 0x103e, *b"LO"),
            (0x28, 0x0a04, *b"LO"),
        ]
        .into_iter()
        .zip(markers)
        {
            builder = builder.text(group, element, vr, marker);
        }
        builder.build()
    };
    let clean =
        read_film(&build("1.2.840.10008.1.2.1"), FilmKind::Front).expect("generated film opens");
    let tags = clean.facts.kept_tags.as_ref().expect("kept tags");
    assert_eq!(tags.pixel_spacing, Some([0.15, 0.16]));
    assert_eq!(tags.photometric, Some(Photometric::Monochrome2));
    assert!(tags.sop_class.is_some());
    let thumbnail = crate::thumbnail::thumbnail(&clean, 32).expect("preview");
    let display = crate::display_pixels::display_pixels(&clean).expect("display");
    let mut returned = format!("{clean:?}{display:?}").into_bytes();
    returned.extend(thumbnail);
    for syntax in [
        "1.2.840.10008.1.2.4.80",
        "1.2.840.10008.1.2.4.81",
        "1.2.840.999.1",
    ] {
        let result = read_film(&build(syntax), FilmKind::Front);
        assert!(
            matches!(&result, Err(error) if error.code == ErrorCode::FilmUnsupportedCompression)
        );
        returned.extend(format!("{result:?}").into_bytes());
    }
    let bytes = build("1.2.840.10008.1.2.1");
    for end in 0..bytes.len() {
        returned.extend(format!("{:?}", read_film(&bytes[..end], FilmKind::Front)).into_bytes());
    }
    assert!(
        markers.iter().all(|marker| !returned
            .windows(marker.len())
            .any(|window| window == marker.as_bytes())),
        "unlisted metadata escaped the parser"
    );
}

#[test]
fn spacing_keeps_row_then_column_and_rejects_invalid_values() {
    let bytes = DicomBuilder::new(2, 1, 8, &[0, 255])
        .text(0x28, 0x30, *b"DS", "0.15\\0.16")
        .text(0x18, 0x1164, *b"DS", "0\\0")
        .text(0x18, 0x2010, *b"DS", "0.2\\0.2")
        .text(0x28, 0x0a02, *b"CS", "FIDUCIAL")
        .build();
    let film = read_film(&bytes, FilmKind::Front).expect("generated film opens");
    let tags = film.facts.kept_tags.expect("DICOM tags");
    assert_eq!(tags.pixel_spacing, Some([0.15, 0.16]));
    assert_eq!(tags.imager_pixel_spacing, None);
    assert_eq!(tags.nominal_scanned_pixel_spacing, Some([0.2, 0.2]));
    assert_eq!(
        tags.pixel_spacing_calibration,
        Some(SpacingCalibration::Fiducial)
    );
    assert!(film.facts.flags.contains(&IntakeFlag::AnisotropicPixels));
    assert!(film.facts.flags.contains(&IntakeFlag::InvalidSpacing));
}

fn lookup_sequence(first: i16, table: &[u16]) -> Vec<u8> {
    let mut item = vec![0x28, 0, 2, 0x30, b'S', b'S', 6, 0];
    item.extend((table.len() as u16).to_le_bytes());
    item.extend(first.to_le_bytes());
    item.extend(8u16.to_le_bytes());
    item.extend([0x28, 0, 6, 0x30, b'O', b'W', 0, 0]);
    item.extend((table.len() as u32 * 2).to_le_bytes());
    item.extend(table.iter().flat_map(|value| value.to_le_bytes()));
    let mut sequence = vec![0xfe, 0xff, 0, 0xe0];
    sequence.extend((item.len() as u32).to_le_bytes());
    sequence.extend(item);
    sequence
}

#[test]
fn lookups_preserve_signed_first_mapped_and_do_not_quantise_codes() {
    let bytes = DicomBuilder::new(3, 1, 8, &[0, 1, 2])
        .text(0x28, 0x1053, *b"DS", "-1")
        .text(0x28, 0x1052, *b"DS", "1")
        .element(0x28, 0x3010, *b"SQ", lookup_sequence(-1, &[0, 128, 255]))
        .build();
    let film = read_film(&bytes, FilmKind::Front).expect("generated film opens");
    assert_eq!(film.image.pixels(), &[255, 254, 253]);
    assert_eq!(
        film.facts.default_window,
        DisplayWindow::Lookup {
            first: -1,
            bits: 8,
            table: vec![0, 128, 255],
            from: WindowSource::FileLookup
        }
    );
    let bytes = DicomBuilder::new(3, 1, 8, &[0, 1, 2])
        .element(0x28, 0x3000, *b"SQ", lookup_sequence(0, &[200, 100, 0]))
        .build();
    let film = read_film(&bytes, FilmKind::Front).expect("generated film opens");
    assert_eq!(film.image.pixels(), &[255, 254, 253]);
    assert_eq!(
        film.facts.values.modality,
        ModalityMap::Lookup {
            first_mapped: 0,
            bits: 8,
            table: vec![200, 100, 0]
        }
    );
}

#[test]
fn padding_and_shutters_affect_statistics_but_never_stored_pixels() {
    let pixels: Vec<u16> = (0..100).collect();
    let bytes = DicomBuilder::new(100, 1, 8, &pixels)
        .us(0x28, 0x120, 0)
        .us(0x28, 0x121, 49)
        .text(0x18, 0x1600, *b"CS", "RECTANGULAR")
        .text(0x18, 0x1602, *b"IS", "1")
        .text(0x18, 0x1604, *b"IS", "70")
        .text(0x18, 0x1606, *b"IS", "1")
        .text(0x18, 0x1608, *b"IS", "1")
        .build();
    let film = read_film(&bytes, FilmKind::Front).expect("generated film opens");
    assert_eq!(film.image.pixels(), &pixels);
    // Central columns20..79, minus padding0..49, within shutter1..70:
    // stored codes50..69. Exact nearest-rank ends50 and69 -> centre59.5,width19.
    assert_eq!(
        film.facts.default_window,
        DisplayWindow::CentreWidth {
            centre: 59.5,
            width: 19.0,
            function: WindowFunction::LinearExact,
            from: WindowSource::Percentile
        }
    );
}

#[test]
fn native_high_bit_is_masked_and_first_frame_is_selected() {
    let bytes = DicomBuilder::new(3, 1, 12, &[0xf000, 0x0100, 0xfff0, 0x0050, 0x0060, 0x0070])
        .us(0x28, 0x102, 15)
        .text(0x28, 8, *b"IS", "2")
        .build();
    let film = read_film(&bytes, FilmKind::Front).expect("generated film opens");
    assert_eq!(film.image.pixels(), &[3840, 16, 4095]);
    assert!(film.facts.flags.contains(&IntakeFlag::MultiFrame));
}

#[test]
fn raw_datasets_and_no_preamble_meta_are_read_by_bytes() {
    for syntax in ["1.2.840.10008.1.2", "1.2.840.10008.1.2.1"] {
        let builder = DicomBuilder::new(2, 1, 8, &[1, 255]).syntax(syntax);
        let film = read_film(&builder.dataset(), FilmKind::Front).expect("dataset opens");
        assert_eq!(film.image.pixels(), &[1, 255]);
        assert!(film.facts.flags.contains(&IntakeFlag::NoFileMeta));
        let bytes = builder.build();
        let film =
            read_film(&bytes[132..], FilmKind::Front).expect("file meta without preamble opens");
        assert_eq!(film.image.pixels(), &[1, 255]);
    }
}

#[test]
fn zlib_wrapped_deflate_is_flagged() {
    let builder = DicomBuilder::new(2, 1, 8, &[1, 255]).syntax("1.2.840.10008.1.2.1.99");
    let mut file = builder.build();
    let meta_len = u32::from_le_bytes(file[140..144].try_into().unwrap()) as usize;
    file.truncate(144 + meta_len);
    file.extend(miniz_oxide::deflate::compress_to_vec_zlib(
        &builder.dataset(),
        6,
    ));
    let film = read_film(&file, FilmKind::Front).expect("wrapped dataset opens");
    assert_eq!(film.image.pixels(), &[1, 255]);
    assert!(film.facts.flags.contains(&IntakeFlag::NonstandardDeflate));
}

#[test]
fn malformed_headers_and_extreme_rescale_fail_without_panicking() {
    let bytes = DicomBuilder::new(2, 1, 16, &[0, 65535])
        .text(0x28, 0x1053, *b"DS", "1e308")
        .build();
    assert!(read_film(&bytes, FilmKind::Front).is_err());
    let large_intercept = DicomBuilder::new(2, 1, 16, &[0, 65535])
        .text(0x28, 0x1052, *b"DS", "1e308")
        .build();
    let finite = read_film(&large_intercept, FilmKind::Front).unwrap();
    assert!(
        matches!(finite.facts.default_window, DisplayWindow::CentreWidth { centre, width, .. } if centre.is_finite() && width.is_finite())
    );
    for (group, element, value) in [
        (0x28, 0x100, 32),
        (0x28, 0x101, 0),
        (0x28, 0x102, 32),
        (0x28, 0x103, 2),
    ] {
        let bytes = DicomBuilder::new(2, 1, 16, &[0, 65535])
            .us(group, element, value)
            .build();
        assert!(read_film(&bytes, FilmKind::Front).is_err());
    }
    let bytes = DicomBuilder::new(2, 1, 16, &[0, 65535]).build();
    for index in 0..bytes.len() {
        let mut corrupted = bytes.clone();
        corrupted[index] ^= 0xff;
        assert!(
            std::panic::catch_unwind(|| read_film(&corrupted, FilmKind::Front)).is_ok(),
            "corrupt input must not panic"
        );
    }
}

#[test]
fn offset_tables_bound_frame_zero_and_reject_invalid_boundaries() {
    use crate::test_support::rle_frame;
    let first = rle_frame(&[3, 4], 8, 1);
    let second = rle_frame(&[6, 7], 8, 1);
    let second_offset = 8 + first.len().next_multiple_of(2) as u32;
    for offset in [
        second_offset,
        second_offset + 1,
        second_offset + second.len() as u32 + 100,
    ] {
        let builder = DicomBuilder::new(2, 1, 8, &[])
            .syntax("1.2.840.10008.1.2.5")
            .text(0x28, 8, *b"IS", "2")
            .encapsulated(vec![first.clone(), second.clone()], vec![0, offset]);
        let result = read_film(&builder.build(), FilmKind::Front);
        if offset == second_offset {
            assert_eq!(result.unwrap().image.pixels(), &[3, 4]);
        } else {
            assert!(result.is_err());
        }
    }
    let extended: Vec<u8> = [1u64, u64::from(second_offset)]
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    let bytes = DicomBuilder::new(2, 1, 8, &[])
        .syntax("1.2.840.10008.1.2.5")
        .text(0x28, 8, *b"IS", "2")
        .element(0x7fe0, 1, *b"OV", extended)
        .encapsulated(vec![first, second], vec![])
        .build();
    assert!(read_film(&bytes, FilmKind::Front).is_err());
}

#[test]
fn saved_png_restores_facts_without_repeating_inversion() {
    let bytes = DicomBuilder::new(3, 1, 12, &[0, 1, 4095])
        .photometric("MONOCHROME1")
        .text(0x28, 0x30, *b"DS", "0.15\\0.16")
        .build();
    let original = read_film(&bytes, FilmKind::Front).unwrap();
    let saved = Film {
        kind: FilmKind::Front,
        image: FilmImage {
            path: FilmKind::Front.png_path().into(),
            width: 3,
            height: 1,
            bits_stored: 12,
        },
        source: original.facts.source,
        kept_tags: original.facts.kept_tags.clone(),
        values: original.facts.values.clone(),
        flags: original.facts.flags.clone(),
        blanked: vec![],
        geometry: shapes::film_geometry::FilmGeometry::UNKNOWN,
    };
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, 3, 1);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Sixteen);
        let mut writer = encoder.write_header().unwrap();
        writer.write_chunk(png::chunk::sBIT, &[12]).unwrap();
        let codes: Vec<u8> = original
            .image
            .pixels()
            .iter()
            .flat_map(|&pixel| ((pixel << 4) | (pixel >> 8)).to_be_bytes())
            .collect();
        writer.write_image_data(&codes).unwrap();
        writer.finish().unwrap();
    }
    let reopened = crate::read_saved_film(&png, &saved).unwrap();
    assert_eq!(reopened.image, original.image);
    assert_eq!(reopened.facts.source, SourceFormat::Dicom);
    assert_eq!(reopened.facts.values, original.facts.values);
    assert_eq!(reopened.facts.kept_tags, original.facts.kept_tags);
    let mut wrong = saved;
    wrong.image.bits_stored = 16;
    assert_eq!(
        crate::read_saved_film(&png, &wrong).unwrap_err().code,
        ErrorCode::CaseFilmSizeMismatch
    );
}

#[test]
fn oversized_dimensions_fail_before_pixel_decoding() {
    for (width, height) in [(16385, 1), (10000, 5000), (16384, 16384)] {
        let bytes = DicomBuilder::new(width, height, 8, &[]).build();
        assert_eq!(
            read_film(&bytes, FilmKind::Front).unwrap_err().code,
            ErrorCode::FilmTooLarge
        );
    }
}

#[test]
fn palette_and_ybr_luma_are_stored_without_double_conversion() {
    let mut builder = DicomBuilder::new(2, 1, 8, &[0, 1]).photometric("PALETTE COLOR");
    for (channel, values) in [(1, [255u16, 0]), (2, [0u16, 255]), (3, [0u16, 0])] {
        builder = builder
            .element(0x28, 0x1100 + channel, *b"US", vec![2, 0, 0, 0, 8, 0])
            .element(
                0x28,
                0x1200 + channel,
                *b"OW",
                values
                    .iter()
                    .flat_map(|value| value.to_le_bytes())
                    .collect(),
            );
    }
    let film = read_film(&builder.build(), FilmKind::Front).unwrap();
    assert_eq!(film.image.pixels(), &[76, 150]); // rounded BT.601 red and green
    let bytes = DicomBuilder::new(2, 1, 8, &[])
        .photometric("YBR_FULL_422")
        .us(0x28, 2, 3)
        .us(0x28, 6, 0)
        .raw_pixels(vec![10, 200, 0, 255])
        .build();
    let film = read_film(&bytes, FilmKind::Front).unwrap();
    assert_eq!(film.image.pixels(), &[10, 200]); // Y is already luma
}
