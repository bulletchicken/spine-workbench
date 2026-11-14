use darkroom::grey_image::{GreyImage, Placement};
use film_reader::{decoded_film::DecodedFilm, eight_bit};
use serde_json::{Value, json};
use shapes::film::{
    DisplayWindow, Film, FilmFacts, FilmImage, FilmKind, KeptTags, PixelValues, SourceFormat,
    WindowFunction, WindowSource,
};
use shapes::film_geometry::{FilmGeometry, ImageRight, QuarterTurns};
use shapes::pixels::PxRect;
use std::rc::Rc;
use trained_finder::side::{
    intake::{Mapping, SideIntake},
    resize::{Letterbox, Resize},
};

fn stored(w: u32, h: u32, bits: u8, pixels: Vec<u16>) -> DecodedFilm {
    let image = GreyImage::new(w, h, pixels, Placement::FILM).unwrap();
    let window = film_reader::thumbnail::default_window(&image, bits, &PixelValues::AS_STORED);
    DecodedFilm {
        image,
        facts: FilmFacts {
            kind: FilmKind::Side,
            width: w,
            height: h,
            bits_stored: bits,
            source: SourceFormat::Png,
            kept_tags: None,
            values: PixelValues::AS_STORED,
            flags: vec![],
            default_window: window,
            exif_quarter_turns: None,
        },
    }
}

fn record(decoded: &DecodedFilm) -> Film {
    Film {
        kind: FilmKind::Side,
        image: FilmImage {
            path: FilmKind::Side.png_path().into(),
            width: decoded.image.width(),
            height: decoded.image.height(),
            bits_stored: decoded.facts.bits_stored,
        },
        source: decoded.facts.source,
        kept_tags: decoded.facts.kept_tags.clone(),
        values: decoded.facts.values.clone(),
        flags: vec![],
        blanked: vec![],
        geometry: FilmGeometry {
            image_right: Some(ImageRight::Posterior),
            ..FilmGeometry::UNKNOWN
        },
    }
}

fn opened(decoded: DecodedFilm, film: Film) -> (Vec<u8>, Mapping) {
    let decoded = Rc::new(decoded);
    let original = decoded.image.clone();
    let mut work = SideIntake::new(Rc::clone(&decoded), film).unwrap();
    assert_eq!(work.last_work(), 0);
    while !work.advance().unwrap() {
        assert!(work.last_work() <= 65_536);
    }
    assert!(work.last_work() <= 65_536);
    assert!(work.advance().unwrap());
    assert_eq!(decoded.image, original);
    let out = work.finish().unwrap();
    assert_eq!(Rc::strong_count(&decoded), 1);
    out
}

#[test]
fn opening_bytes_match_for_depths_windows_inversions_and_blanked_edges() {
    for bits in [1, 8, 12, 16] {
        for mono in [false, true] {
            for stored_flip in [false, true] {
                for user_flip in [false, true] {
                    for sigmoid in [false, true] {
                        let highest = (1u32 << bits) - 1;
                        let pixels = (0..89 * 47)
                            .map(|i| ((i * 71) % (highest + 1)) as u16)
                            .collect();
                        let mut decoded = stored(89, 47, bits, pixels);
                        if mono {
                            decoded.facts.kept_tags = Some(
                                serde_json::from_value::<KeptTags>(
                                    json!({"photometric":"monochrome1"}),
                                )
                                .unwrap(),
                            );
                        }
                        decoded.facts.values.inverted = mono ^ stored_flip;
                        decoded.facts.values.inverted_by_user = stored_flip;
                        if sigmoid {
                            decoded.facts.default_window = DisplayWindow::CentreWidth {
                                centre: 10.0,
                                width: 50.0,
                                function: WindowFunction::Sigmoid,
                                from: WindowSource::File,
                            };
                        }
                        let mut film = record(&decoded);
                        film.values.inverted ^= user_flip;
                        film.blanked = vec![
                            PxRect {
                                left: 0.5,
                                top: 1.5,
                                right: 3.5,
                                bottom: 8.25,
                            },
                            PxRect {
                                left: 60.2,
                                top: 20.3,
                                right: 88.5,
                                bottom: 46.5,
                            },
                        ];
                        let expected: Vec<_> = eight_bit::as_opened(&decoded)
                            .unwrap()
                            .iter()
                            .enumerate()
                            .map(|(i, &value)| {
                                if film
                                    .blanked
                                    .iter()
                                    .any(|b| b.covers_pixel(i as u32 % 89, i as u32 / 89))
                                {
                                    0
                                } else if user_flip {
                                    255 - value
                                } else {
                                    value
                                }
                            })
                            .collect();
                        assert_eq!(
                            opened(decoded, film).0,
                            expected,
                            "{bits} bits mono {mono} stored flip {stored_flip} user flip {user_flip} sigmoid {sigmoid}"
                        );
                    }
                }
            }
        }
    }
    let mut decoded = stored(4, 1, 12, vec![0, 1, 2, 3]);
    decoded.facts.default_window = DisplayWindow::Lookup {
        first: 0,
        bits: 8,
        table: vec![0, 77, 123, 255],
        from: WindowSource::FileLookup,
    };
    let film = record(&decoded);
    assert_eq!(opened(decoded, film).0, [0, 77, 123, 255]);
}

#[test]
fn clockwise_turns_then_right_facing_flips_have_independent_pixel_answers() {
    let answers = [
        vec![0, 1, 2, 3, 4, 5],
        vec![3, 0, 4, 1, 5, 2],
        vec![5, 4, 3, 2, 1, 0],
        vec![2, 5, 1, 4, 0, 3],
    ];
    for turns in 0..4 {
        for facing in [ImageRight::Posterior, ImageRight::Anterior] {
            let decoded = stored(3, 2, 8, vec![0, 1, 2, 3, 4, 5]);
            let mut film = record(&decoded);
            film.geometry.quarter_turns = QuarterTurns::new(turns).unwrap();
            film.geometry.image_right = Some(facing);
            let mut expected = answers[usize::from(turns)].clone();
            let width = if turns % 2 == 0 { 3 } else { 2 };
            if facing == ImageRight::Anterior {
                for row in expected.chunks_mut(width) {
                    row.reverse();
                }
            }
            assert_eq!(opened(decoded, film).0, expected);
        }
    }
}

#[test]
fn actual_centre_scales_turns_and_flips_roundtrip_without_a_half_pixel_shift() {
    let mut largest = 0.0f64;
    for (w, h) in [(3, 2), (17, 811), (1001, 2003), (16384, 131)] {
        for turns in 0..4 {
            for facing in [ImageRight::Posterior, ImageRight::Anterior] {
                let m = Mapping::new(w, h, turns, facing).unwrap();
                for p in [
                    [0.0, 0.0],
                    [w as f64 - 1.0, h as f64 - 1.0],
                    [21.125, -7.75],
                    [w as f64 / 3.0, h as f64 / 7.0],
                ] {
                    let [x, y] = p;
                    let upright = match turns {
                        0 => [x, y],
                        1 => [h as f64 - 1.0 - y, x],
                        2 => [w as f64 - 1.0 - x, h as f64 - 1.0 - y],
                        _ => [y, w as f64 - 1.0 - x],
                    };
                    let b = m.letterbox;
                    let cx = if facing == ImageRight::Anterior {
                        b.width as f64 - 1.0 - upright[0]
                    } else {
                        upright[0]
                    };
                    let expected = [
                        (cx + 0.5) * b.resized_width as f64 / b.width as f64 - 0.5 + b.left as f64,
                        (upright[1] + 0.5) * b.resized_height as f64 / b.height as f64 - 0.5
                            + b.top as f64,
                    ];
                    let actual = m.to_model(p);
                    for axis in 0..2 {
                        assert!((actual[axis] - expected[axis]).abs() < 1e-9);
                    }
                    let back = m.to_film(actual);
                    for axis in 0..2 {
                        largest = largest.max((back[axis] - p[axis]).abs());
                    }
                }
            }
        }
    }
    assert!(largest <= 1e-9);
    eprintln!("maximum original/model centre roundtrip {largest:e} px");
}

#[test]
fn letterboxing_matches_every_pinned_byte_and_python_rounding_case() {
    let answer: Value =
        serde_json::from_str(include_str!("side-known-answers/synthetic.json")).unwrap();
    for case in answer["resize"]["cases"].as_array().unwrap() {
        let size = |name: &str| {
            [
                case[name][0].as_u64().unwrap() as usize,
                case[name][1].as_u64().unwrap() as usize,
            ]
        };
        let [w, h] = size("inputSize");
        let [tw, th] = size("targetSize");
        let b = Letterbox::new(w, h, tw, th).unwrap();
        assert_eq!([b.resized_width, b.resized_height], size("resizedSize"));
        let pixels = case["inputBytes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_u64().unwrap() as u8)
            .collect();
        let mut work = Resize::new(pixels, b).unwrap();
        while !work.advance().unwrap() {
            let (pixels, taps) = work.last_work();
            assert!(pixels <= 65_536 && taps <= 1 << 20);
        }
        let expected: Vec<_> = case["outputBytes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_u64().unwrap() as u8)
            .collect();
        assert_eq!(work.finish().unwrap(), expected, "{}", case["name"]);
        for axis in 0..2 {
            let point = b.to_model([0.0, 0.0]);
            assert!((point[axis] - case["affine"][axis][2].as_f64().unwrap()).abs() < 1e-12);
        }
    }
}

#[test]
fn wide_shrink_rows_and_large_intake_are_cut_into_fixed_bands() {
    let (w, h) = (16384, 65);
    let mut work = Resize::new(vec![77; w * h], Letterbox::new(w, h, 384, 512).unwrap()).unwrap();
    let mut calls = 0;
    let mut largest = (0, 0);
    while !work.is_done() {
        work.advance().unwrap();
        let (p, t) = work.last_work();
        largest.0 = largest.0.max(p);
        largest.1 = largest.1.max(t);
        assert!(p <= 65_536 && t <= 1 << 20);
        calls += 1;
    }
    let out = work.finish().unwrap();
    assert!(calls > 8);
    assert!(out.iter().all(|&v| v == 0 || v == 77));
    let decoded = stored(
        1301,
        129,
        16,
        (0..1301 * 129).map(|i| (i % 65536) as u16).collect(),
    );
    let expected = eight_bit::as_opened(&decoded).unwrap();
    let film = record(&decoded);
    assert_eq!(opened(decoded, film).0, expected);
    eprintln!(
        "wide resize {calls} units; maximum {} pixels, {} filter taps",
        largest.0, largest.1
    );
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend((data.len() as u32).to_be_bytes());
    out.extend(kind);
    out.extend(data);
    let mut crc = !0u32;
    for &byte in kind.iter().chain(data) {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xedb88320
            } else {
                crc >> 1
            };
        }
    }
    out.extend((!crc).to_be_bytes());
}

fn saved_png(w: u32, h: u32, bits: u8, pixels: &[u16]) -> Vec<u8> {
    let mut raw = vec![];
    for row in pixels.chunks(w as usize) {
        raw.push(0);
        for &value in row {
            if bits <= 8 {
                raw.push(value as u8);
            } else {
                raw.extend((value << (16 - bits)).to_be_bytes());
            }
        }
    }
    assert!(raw.len() < 65536);
    let length = raw.len() as u16;
    let mut zlib = vec![0x78, 0x01, 1];
    zlib.extend(length.to_le_bytes());
    zlib.extend((!length).to_le_bytes());
    zlib.extend(&raw);
    let (mut a, mut b) = (1u32, 0u32);
    for byte in raw {
        a = (a + u32::from(byte)) % 65521;
        b = (b + a) % 65521;
    }
    zlib.extend(((b << 16) | a).to_be_bytes());
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = vec![];
    header.extend(w.to_be_bytes());
    header.extend(h.to_be_bytes());
    header.extend([if bits <= 8 { 8 } else { 16 }, 0, 0, 0, 0]);
    chunk(&mut png, b"IHDR", &header);
    chunk(&mut png, b"sBIT", &[bits]);
    chunk(&mut png, b"IDAT", &zlib);
    chunk(&mut png, b"IEND", &[]);
    png
}

#[test]
fn saved_reopen_keeps_user_inversion_and_halfway_byte_rounding() {
    for bits in [8, 12, 16] {
        let highest = ((1u32 << bits) - 1) as u16;
        let pixels: Vec<_> = (0..64 * 16)
            .map(|i| (u32::from(highest) * (i % 64) / 63) as u16)
            .collect();
        let fresh = stored(64, 16, bits, pixels.clone());
        let mut film = record(&fresh);
        film.values.inverted = true;
        film.values.inverted_by_user = true;
        let flipped: Vec<_> = pixels.iter().map(|&v| highest - v).collect();
        let png = saved_png(64, 16, bits, &flipped);
        let reopened = film_reader::read_saved_film(&png, &film).unwrap();
        let expected = eight_bit::as_opened(&reopened).unwrap();
        assert_eq!(opened(reopened, film.clone()).0, expected);
        assert_eq!(opened(fresh, film).0, expected, "{bits} bits");
    }
}
