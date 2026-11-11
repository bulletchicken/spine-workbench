//! Independent encoder fixtures and small exact answers for the codec shelf.

use super::*;
use crate::test_support::{DicomBuilder, lossless_jpeg, rle_frame};
use shapes::film::FilmKind;

fn header(width: u32, height: u32, bits: u8) -> PixelHeader {
    PixelHeader {
        width,
        height,
        bits_allocated: if bits <= 8 { 8 } else { 16 },
        samples_per_pixel: 1,
        planar_configuration: 0,
        photometric: Photometric::Monochrome2,
    }
}

#[test]
fn raw_byte_order_and_colour_planes_have_hand_written_answers() {
    let h = header(2, 1, 16);
    let little = decode(
        &[0x34, 0x12, 0xff, 0],
        TransferSyntax::ExplicitLittleEndian,
        h,
    )
    .unwrap();
    let big = decode(&[0x12, 0x34, 0, 0xff], TransferSyntax::ExplicitBigEndian, h).unwrap();
    assert_eq!(little.samples, [0x1234, 0x00ff]);
    assert_eq!(big.samples, little.samples);
    let mut colour = header(2, 1, 8);
    colour.samples_per_pixel = 3;
    colour.planar_configuration = 1;
    colour.photometric = Photometric::Rgb;
    let pixels = decode(
        &[1, 2, 3, 4, 5, 6],
        TransferSyntax::ExplicitLittleEndian,
        colour,
    )
    .unwrap();
    assert_eq!(pixels.samples, [1, 3, 5, 2, 4, 6]);
    colour.photometric = Photometric::YbrFull422;
    colour.planar_configuration = 0;
    let pixels = decode(
        &[10, 20, 128, 129],
        TransferSyntax::ExplicitLittleEndian,
        colour,
    )
    .unwrap();
    assert_eq!(pixels.samples, [10, 128, 129, 20, 128, 129]);
}

#[test]
fn rle_byte_planes_reassemble_exactly() {
    let samples = [0x1234, 0xabcd, 0x00ff, 0xff00];
    let frame = rle_frame(&samples, 16, 1);
    let decoded = decode(&frame, TransferSyntax::RleLossless, header(4, 1, 16)).unwrap();
    assert_eq!(decoded.samples, samples);
    let bytes = DicomBuilder::new(4, 1, 16, &[])
        .syntax("1.2.840.10008.1.2.5")
        .encapsulated(vec![frame], vec![])
        .build();
    let film = crate::read_film(&bytes, FilmKind::Front).unwrap();
    assert_eq!(film.image.pixels(), samples);
}

#[test]
fn baseline_and_eight_bit_extended_stay_within_eight_codes() {
    // A gentle integer ramp has no discontinuity or clipping. Quality 95
    // may quantise each sample; eight 8-bit codes is the intake tolerance.
    let samples: Vec<u8> = (0..32)
        .flat_map(|y| (0..32).map(move |x| (2 * x + 3 * y + 20) as u8))
        .collect();
    let mut jpeg = Vec::new();
    jpeg_encoder::Encoder::new(&mut jpeg, 95)
        .encode(&samples, 32, 32, jpeg_encoder::ColorType::Luma)
        .unwrap();
    let decoded = decode(&jpeg, TransferSyntax::JpegBaseline, header(32, 32, 8)).unwrap();
    assert!(
        decoded
            .samples
            .iter()
            .zip(&samples)
            .all(|(&a, &b)| a.abs_diff(u16::from(b)) <= 8)
    );
    assert_eq!(decoded.flags, [IntakeFlag::Lossy]);
    let sof = jpeg
        .windows(2)
        .position(|part| part == [0xff, 0xc0])
        .unwrap();
    jpeg[sof + 1] = 0xc1;
    let extended = decode(&jpeg, TransferSyntax::JpegExtended, header(32, 32, 8)).unwrap();
    assert_eq!(extended.samples, decoded.samples);
    jpeg[sof + 4] = 12;
    assert_eq!(
        decode(&jpeg, TransferSyntax::JpegExtended, header(32, 32, 12))
            .err()
            .unwrap()
            .code,
        ErrorCode::FilmUnsupportedCompression
    );
}

#[test]
fn baseline_colour_is_returned_as_rgb_once() {
    let pixels: Vec<u8> = (0..256)
        .flat_map(|i| [(40 + i % 16) as u8, (80 + i / 16) as u8, 140])
        .collect();
    let mut bytes = Vec::new();
    jpeg_encoder::Encoder::new(&mut bytes, 95)
        .encode(&pixels, 16, 16, jpeg_encoder::ColorType::Rgb)
        .unwrap();
    let expected = jpeg_decoder::Decoder::new(std::io::Cursor::new(&bytes))
        .decode()
        .unwrap();
    let mut h = header(16, 16, 8);
    h.samples_per_pixel = 3;
    h.photometric = Photometric::YbrFull422;
    let result = decode(&bytes, TransferSyntax::JpegBaseline, h).unwrap();
    assert!(result.colour_rgb);
    assert!(
        result
            .samples
            .iter()
            .zip(&expected)
            .all(|(&a, &b)| a == u16::from(b))
    );
}

#[test]
fn lossless_predictors_point_transform_and_restarts_are_exact() {
    for precision in [8, 12, 16] {
        let mask = (1u32 << precision) - 1;
        let samples: Vec<u16> = (0..136)
            .map(|i| ((i * 977 + i * i * 31) & mask) as u16)
            .collect();
        for predictor in 1..=7 {
            for restart in [0, 17, 34] {
                for point_transform in [0, 2] {
                    let jpeg = lossless_jpeg(
                        &samples,
                        17,
                        8,
                        precision,
                        predictor,
                        point_transform,
                        restart,
                    );
                    let result = decode(
                        &jpeg,
                        TransferSyntax::JpegLossless,
                        header(17, 8, precision),
                    )
                    .unwrap();
                    let expected: Vec<u16> = samples
                        .iter()
                        .map(|&value| (value >> point_transform) << point_transform)
                        .collect();
                    assert_eq!(
                        result.samples, expected,
                        "precision={precision}, predictor={predictor}, point={point_transform}, restart={restart}"
                    );
                }
            }
        }
    }
    // At P=16 the initial predictor is 32768: this first zero is exactly
    // difference category 16. The second difference is its opposite.
    let jpeg = lossless_jpeg(&[0, 32768, 65535], 3, 1, 16, 1, 0, 0);
    let result = decode(
        &jpeg,
        TransferSyntax::JpegLosslessFirstOrder,
        header(3, 1, 16),
    )
    .unwrap();
    assert_eq!(result.samples, [0, 32768, 65535]);
}

#[test]
fn lossless_restart_starts_a_new_prediction_row() {
    // Header: 3x3, P=8, predictor4, restart interval6. Uniform five-bit
    // Huffman categories. Entropy bytes below were worked out from the
    // differences [-118,10,10,5,0,0], RST0, [-28,10,10]. Thus the last row
    // is [100,110,120], independently of [15,25,35] above it.
    let mut bytes = vec![
        0xff, 0xd8, 0xff, 0xc3, 0, 11, 8, 0, 3, 0, 3, 1, 1, 0x11, 0, 0xff, 0xc4, 0, 36, 0,
    ];
    bytes.extend([0, 0, 0, 0, 17, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    bytes.extend(0..=16);
    bytes.extend([0xff, 0xdd, 0, 4, 0, 6, 0xff, 0xda, 0, 8, 1, 1, 0, 4, 0, 0]);
    bytes.extend([
        0x38, 0x92, 0x51, 0x28, 0x74, 0x00, 0xff, 0xd0, 0x28, 0xc9, 0x44, 0xaf, 0xff, 0xd9,
    ]);
    let decoded = decode(&bytes, TransferSyntax::JpegLossless, header(3, 3, 8)).unwrap();
    assert_eq!(decoded.samples, [10, 20, 30, 15, 25, 35, 100, 110, 120]);
}

fn j2k(bits: u8, signed: bool, levels: u8) -> (Vec<u16>, Vec<u8>) {
    let mask = (1u32 << bits) - 1;
    // The test encoder's two-axis integer lifting order does not invert its
    // decoder's order. A five-level one-axis ramp avoids that encoder bug;
    // zero-level images separately exercise varying rows and every bit.
    let samples: Vec<u16> = (0..64)
        .flat_map(|y| {
            (0..64).map(move |x| {
                if levels == 0 {
                    ((x * 137 + y * 521) & mask) as u16
                } else {
                    (x * mask / 63) as u16
                }
            })
        })
        .collect();
    let raw: Vec<u8> = samples
        .iter()
        .flat_map(|&value| {
            // The independent encoder takes signed i16 values, even at P=12.
            let word = if signed && u32::from(value) >= 1 << (bits - 1) {
                (i32::from(value) - (1 << bits)) as u16
            } else {
                value
            };
            word.to_le_bytes()
        })
        .collect();
    let options = dicom_toolkit_jpeg2000::EncodeOptions {
        num_decomposition_levels: levels,
        ..Default::default()
    };
    let encoded = dicom_toolkit_jpeg2000::encode(&raw, 64, 64, 1, bits, signed, &options).unwrap();
    (samples, encoded)
}

#[test]
fn jpeg2000_reversible_samples_and_signedness_are_exact() {
    for bits in [12, 16] {
        for signed in [false, true] {
            for levels in [0, 5] {
                let (expected, bytes) = j2k(bits, signed, levels);
                let decoded = decode(
                    &bytes,
                    TransferSyntax::Jpeg2000Lossless,
                    header(64, 64, bits),
                )
                .unwrap();
                let mismatch = decoded
                    .samples
                    .iter()
                    .zip(&expected)
                    .filter(|(a, b)| a != b)
                    .count();
                let max_error = decoded
                    .samples
                    .iter()
                    .zip(&expected)
                    .map(|(a, b)| a.abs_diff(*b))
                    .max()
                    .unwrap();
                assert!(
                    decoded.samples == expected,
                    "bits={bits}, signed={signed}, levels={levels}, mismatches={mismatch}, max_error={max_error}"
                );
                assert_eq!(decoded.signed, Some(signed));
                assert!(decoded.flags.is_empty());
            }
        }
    }
}

#[test]
fn pinned_jpeg2000_decoder_adds_a_shift_even_to_signed_samples() {
    let (samples, bytes) = j2k(12, true, 5);
    let image =
        hayro_jpeg2000::Image::new(&bytes, &hayro_jpeg2000::DecodeSettings::default()).unwrap();
    let mut context = hayro_jpeg2000::DecoderContext::default();
    let decoded = image.decode(&mut context).unwrap();
    for (&raw, &pattern) in decoded.components()[0].samples().iter().zip(&samples) {
        let value = if pattern >= 2048 {
            i32::from(pattern) - 4096
        } else {
            i32::from(pattern)
        };
        assert_eq!(raw, (value + 2048) as f32);
    }
}

#[test]
fn jpeg2000_wrapper_and_fragmented_dicom_preserve_samples() {
    let (samples, bytes) = j2k(12, false, 5);
    let mut wrapped = b"\0\0\0\x0cjP  \r\n\x87\n".to_vec();
    wrapped.extend(((bytes.len() + 8) as u32).to_be_bytes());
    wrapped.extend(b"jp2c");
    wrapped.extend(&bytes);
    let decoded = decode(
        &wrapped,
        TransferSyntax::Jpeg2000Lossless,
        header(64, 64, 12),
    )
    .unwrap();
    assert!(decoded.samples == samples);
    assert_eq!(decoded.flags, [IntakeFlag::Jp2Wrapper]);
    let cut = (bytes.len() / 3) & !1;
    let dicom = DicomBuilder::new(64, 64, 12, &[])
        .syntax("1.2.840.10008.1.2.4.90")
        .encapsulated(
            vec![
                bytes[..cut].to_vec(),
                bytes[cut..2 * cut].to_vec(),
                bytes[2 * cut..].to_vec(),
            ],
            vec![],
        )
        .build();
    let film = crate::read_film(&dicom, FilmKind::Front).unwrap();
    assert!(film.image.pixels() == samples);
}

#[test]
fn jpeg2000_general_syntax_accepts_a_reversible_stream_and_marks_it_lossy() {
    // The general syntax permits both transforms. The independent test
    // encoder's irreversible quantisation is not an accuracy oracle.
    let (samples, bytes) = j2k(12, false, 5);
    let decoded = decode(&bytes, TransferSyntax::Jpeg2000, header(64, 64, 12)).unwrap();
    assert!(decoded.samples == samples);
    assert_eq!(decoded.flags, [IntakeFlag::Lossy]);
}

#[test]
fn unsupported_syntaxes_have_a_fixed_error_and_never_echo_the_uid() {
    for uid in [
        "1.2.840.10008.1.2.4.80",
        "1.2.840.10008.1.2.4.81",
        "1.2.840.10008.1.2.4.201",
        "untrusted-value",
    ] {
        let error = transfer_syntax(uid).unwrap_err();
        assert_eq!(error.code, ErrorCode::FilmUnsupportedCompression);
        assert!(!error.to_string().contains(uid));
    }
}

#[test]
fn generated_compressed_streams_tolerate_truncation_and_corruption() {
    let (_, j2k) = j2k(12, false, 5);
    let samples: Vec<u16> = (0..4096).map(|value| value as u16).collect();
    let lossless = lossless_jpeg(&samples, 64, 64, 12, 7, 2, 64);
    let rle = rle_frame(&samples, 16, 1);
    for (bytes, syntax) in [
        (j2k, TransferSyntax::Jpeg2000Lossless),
        (lossless, TransferSyntax::JpegLossless),
        (rle, TransferSyntax::RleLossless),
    ] {
        for i in 0..500 {
            let cut = (i * 104729) % bytes.len();
            assert!(
                std::panic::catch_unwind(|| decode(&bytes[..cut], syntax, header(64, 64, 12)))
                    .is_ok(),
                "truncation must be an error, not a panic"
            );
            let mut damaged = bytes.clone();
            damaged[cut] ^= 1 << (i % 8);
            assert!(
                std::panic::catch_unwind(|| decode(&damaged, syntax, header(64, 64, 12))).is_ok(),
                "corruption must be an error, not a panic"
            );
        }
    }
}
