//! Exact integer known answers: no floating-point tolerances are needed.

use case_file::{film_png::encode_png, pack::pack_case, unpack::unpack_case};
use shapes::case_file_v1::CaseFileV1;
use shapes::errors::ErrorCode;
use shapes::film::{
    DisplayWindow, Film, FilmFacts, FilmKind, FilmPng, WindowFunction, WindowSource,
};
use shapes::pixels::PxRect;
use std::io::{Cursor, Read};

fn film(bits: u8, width: u32, height: u32) -> (CaseFileV1, FilmFacts) {
    let mut case = shapes::reading::read_case_json(include_str!(
        "../../../proving-ground/known-answers/case-files/valid/minimal.json"
    ))
    .unwrap();
    let film = &mut case.films[0];
    film.image.width = width;
    film.image.height = height;
    film.image.bits_stored = bits;
    let facts = FilmFacts {
        kind: film.kind,
        width,
        height,
        bits_stored: bits,
        source: film.source,
        kept_tags: film.kept_tags.clone(),
        values: film.values.clone(),
        flags: film.flags.clone(),
        default_window: DisplayWindow::CentreWidth {
            centre: 127.5,
            width: 256.0,
            function: WindowFunction::Linear,
            from: WindowSource::Percentile,
        },
        exif_quarter_turns: None,
    };
    (case, facts)
}

fn samples(png: &[u8]) -> (u8, Vec<u16>) {
    let mut reader = png::Decoder::new(Cursor::new(png)).read_info().unwrap();
    let bits = reader.info().sbit.as_ref().unwrap()[0];
    let depth = reader.info().bit_depth as u8;
    let mut bytes = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut bytes).unwrap();
    reader.finish().unwrap();
    let values = if depth == 8 {
        bytes.into_iter().map(u16::from).collect()
    } else {
        bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_be_bytes(*pair))
            .collect()
    };
    (bits, values)
}

#[test]
fn significant_bits_have_exact_known_container_values() {
    // The low bits repeat the high bits: 3-bit 011 becomes 01101101.
    for (bits, pixels, expected) in [
        (1, vec![0, 1], vec![0, 255]),
        (3, vec![0, 1, 3, 7], vec![0, 36, 109, 255]),
        (8, vec![0, 1, 127, 255], vec![0, 1, 127, 255]),
        (12, vec![0, 1, 2047, 4095], vec![0, 0x0010, 0x7ff7, 0xffff]),
        (16, vec![0, 1, 32767, 65535], vec![0, 1, 32767, 65535]),
    ] {
        let (case, facts) = film(bits, pixels.len() as u32, 1);
        let png = encode_png(&pixels, &facts, &case.films[0]).unwrap();
        assert_eq!(samples(&png), (bits, expected));
    }
}

#[test]
fn every_significant_depth_and_value_round_trips_exactly() {
    for bits in 1..=16 {
        let pixels: Vec<u16> = (0..(1_u32 << bits)).map(|value| value as u16).collect();
        let width = pixels.len().min(256) as u32;
        let height = pixels.len() as u32 / width;
        let (case, facts) = film(bits, width, height);
        let png = encode_png(&pixels, &facts, &case.films[0]).unwrap();
        let (actual_bits, expanded) = samples(&png);
        assert_eq!(actual_bits, bits);
        let shift = if bits <= 8 { 8 - bits } else { 16 - bits };
        let exact: Vec<_> = expanded.into_iter().map(|value| value >> shift).collect();
        assert_eq!(exact, pixels);
        let input = vec![FilmPng {
            kind: FilmKind::Front,
            png,
        }];
        let packed = pack_case(&case, &input).unwrap();
        let opened = unpack_case(&packed).unwrap();
        assert_eq!(opened.case, case);
        assert_eq!(opened.films, input);
        assert_eq!(pack_case(&opened.case, &opened.films).unwrap(), packed);
    }
}

#[test]
fn blanking_uses_pixel_overlap_and_brightness_is_applied_once() {
    let (mut case, facts) = film(8, 4, 2);
    let record = &mut case.films[0];
    record.values.inverted = true;
    record.values.inverted_by_user = true;
    record.blanked.push(PxRect {
        left: 0.5,
        top: -0.5,
        right: 1.5,
        bottom: 0.5,
    });
    let pixels = [0, 1, 2, 3, 4, 5, 6, 255];
    let png = encode_png(&pixels, &facts, record).unwrap();
    // Only column 1 of row 0 has positive overlap; its neighbours touch edges.
    let expected = vec![255, 0, 253, 252, 251, 250, 249, 0];
    assert_eq!(samples(&png).1, expected);
    let mut saved_facts = facts;
    saved_facts.values = record.values.clone();
    assert_eq!(encode_png(&expected, &saved_facts, record).unwrap(), png);
    let packed = pack_case(
        &case,
        &[FilmPng {
            kind: FilmKind::Front,
            png,
        }],
    )
    .unwrap();
    let opened = unpack_case(&packed).unwrap();
    assert_eq!(samples(&opened.films[0].png).1, expected);
}

fn with_source_metadata(film: &Film, file_name: &str) -> Vec<u8> {
    let info = png::Info::with_size(film.image.width, film.image.height);
    let mut output = Vec::new();
    let mut encoder = png::Encoder::with_info(&mut output, info).unwrap();
    encoder
        .add_text_chunk("OriginalFile".to_owned(), file_name.to_owned())
        .unwrap();
    let mut writer = encoder.write_header().unwrap();
    writer.write_chunk(png::chunk::sBIT, &[8]).unwrap();
    writer.write_image_data(&[0, 1, 2, 3]).unwrap();
    writer.finish().unwrap();
    output
}

fn contains(bytes: &[u8], needle: &str) -> bool {
    bytes
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

#[test]
fn no_original_file_name_survives_anywhere_in_a_packed_case() {
    let original_file_name = "original-film-export.dcm";
    let (case, _) = film(8, 4, 1);
    let png = with_source_metadata(&case.films[0], original_file_name);
    assert!(contains(&png, original_file_name));
    let packed = pack_case(
        &case,
        &[FilmPng {
            kind: FilmKind::Front,
            png,
        }],
    )
    .unwrap();
    assert!(!contains(&packed, original_file_name));
    let mut archive = zip::ZipArchive::new(Cursor::new(&packed)).unwrap();
    let mut names = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).unwrap();
        names.push(entry.name().unwrap().to_string());
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        assert!(!contains(&bytes, original_file_name));
        if names.last().unwrap().ends_with(".png") {
            let mut at = 8;
            let mut chunks = Vec::new();
            while at < bytes.len() {
                let len = u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
                chunks.push(bytes[at + 4..at + 8].to_vec());
                at += len + 12;
            }
            assert_eq!(chunks, [b"IHDR", b"sBIT", b"IDAT", b"IEND"]);
        }
    }
    assert_eq!(names, ["case.json", "films/front.png", "README.txt"]);
}

#[test]
fn pack_refuses_records_and_films_that_would_not_read_back() {
    let (case, facts) = film(8, 2, 1);
    let png = FilmPng {
        kind: FilmKind::Front,
        png: encode_png(&[1, 2], &facts, &case.films[0]).unwrap(),
    };
    assert_eq!(
        pack_case(&case, &[]).unwrap_err().code,
        ErrorCode::CaseMissingFilm
    );
    assert_eq!(
        pack_case(&case, &[png.clone(), png.clone()])
            .unwrap_err()
            .code,
        ErrorCode::CaseUnknownEntry
    );
    let mut invalid = case.clone();
    invalid.version = 2;
    assert_eq!(
        pack_case(&invalid, std::slice::from_ref(&png))
            .unwrap_err()
            .code,
        ErrorCode::CaseNewerVersion
    );
    invalid = case.clone();
    invalid.films[0].image.width = 3;
    assert_eq!(
        pack_case(&invalid, &[png]).unwrap_err().code,
        ErrorCode::CaseFilmSizeMismatch
    );
    assert_eq!(
        encode_png(&[256, 0], &facts, &case.films[0])
            .unwrap_err()
            .code,
        ErrorCode::CaseInvalidValue
    );
}
