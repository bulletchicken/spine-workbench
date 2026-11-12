//! The picture the network is shown, read from a film through its record,
//! and the way back from that picture to the film.
//!
//! The film goes to 8-bit grey the way it opens on screen
//! (`film_reader::eight_bit`); it is flipped where the user flipped its
//! brightness; every blanked box is black; and it is turned upright by the
//! record's quarter turns, because the network only ever saw upright
//! films. The pixels of the decoded film itself are never changed.
//!
//! A film can be 40 megapixels, so the 8-bit copy is flipped and blanked
//! where it lies, and copied once more only to turn it. Each film-sized
//! list is asked for up front, and a refusal is `EngineOutOfMemory`.

use crate::outline::Point;
use darkroom::grey_image::{GreyImage, Placement};
use film_reader::decoded_film::DecodedFilm;
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::Film;
use shapes::pixels::{Px, PxRect};
use std::ops::Range;

/// The upright 8-bit picture of the film, placed on the film so that every
/// position on it maps back exactly (see [`to_film_pixels`]).
pub fn picture(decoded: &DecodedFilm, film: &Film) -> Result<GreyImage<u8>, EngineError> {
    let (width, height) = (decoded.image.width(), decoded.image.height());
    if decoded.facts.kind != film.kind || width != film.image.width || height != film.image.height {
        return Err(EngineError::new(
            ErrorCode::EngineOutOfSync,
            "The film record does not match the decoded pixels",
        ));
    }
    if film
        .blanked
        .iter()
        .any(|area| !area.is_on_film(width, height))
    {
        return Err(EngineError::new(
            ErrorCode::CaseInvalidValue,
            "a blanked region lies outside its film",
        ));
    }
    let mut pixels = film_reader::eight_bit::as_opened(decoded)?;
    // The record's `inverted` includes the user's flip and the decoder's
    // does not, so where they differ the user flipped the film, as the
    // auto pass reads it.
    let flipped = decoded.facts.values.inverted != film.values.inverted;
    if flipped {
        for value in &mut pixels {
            *value = 255 - *value;
        }
    }
    for area in &film.blanked {
        blank(&mut pixels, width, height, area);
    }
    turned(pixels, width, height, film.geometry.quarter_turns.get())
}

/// Every pixel `area` reaches into, by the case's own rule
/// ([`PxRect::covers_pixel`]), set to black. Only the pixels in and just
/// around the box are looked at.
fn blank(pixels: &mut [u8], width: u32, height: u32, area: &PxRect) {
    let columns = reach(area.left, area.right, width);
    for y in reach(area.top, area.bottom, height) {
        for x in columns.clone() {
            if area.covers_pixel(x, y) {
                pixels[y as usize * width as usize + x as usize] = 0;
            }
        }
    }
}

/// The pixels, along one side of `size`, that a box from `low` to `high`
/// can reach into. Pixel i is reached when `low < i + 0.5` and
/// `i - 0.5 < high`, so the first is `floor(low + 0.5)` and the last
/// `ceil(high - 0.5)`; one more each side leaves room for rounding, and
/// `covers_pixel` has the last word.
fn reach(low: f64, high: f64, size: u32) -> Range<u32> {
    let first = ((low + 0.5).floor() - 1.0).max(0.0);
    let last = ((high - 0.5).ceil() + 1.0).min(f64::from(size) - 1.0);
    if first <= last {
        first as u32..last as u32 + 1
    } else {
        0..0
    }
}

/// A point on the upright picture back on the original film, in film
/// pixels.
///
/// The point is measured the way the network's outlines are measured: from
/// the picture's outer top-left corner, so the centre of its top-left pixel
/// is (0.5, 0.5). That half pixel is taken off here, where such a point is
/// read, as `docs/conventions/pixel-space.md` asks; the placement then
/// carries it to the film. A crop of the picture carries its own placement,
/// so the same call serves it too.
pub fn to_film_pixels(placement: Placement, point: Point) -> Px {
    placement.film_point(point[0] - 0.5, point[1] - 0.5)
}

/// The film turned `turns` quarter turns clockwise, with its placement.
/// Upright pixel (x, y) reads stored pixel (u, v) by the same rule the auto
/// pass uses for its upright profiles (`engine/auto-pass/src/evidence/
/// prepare.rs`, `upright_index`). A film already upright keeps its list.
fn turned(
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    turns: u8,
) -> Result<GreyImage<u8>, EngineError> {
    let (w, h) = (f64::from(width), f64::from(height));
    let (out_width, out_height, placement) = match turns % 4 {
        0 => {
            return Ok(GreyImage::new(width, height, pixels, Placement::FILM)
                .expect("one value per pixel"));
        }
        // (u, v) = (y, h - 1 - x)
        1 => (
            height,
            width,
            Placement {
                origin: Px::new(0.0, h - 1.0),
                step_across: Px::new(0.0, -1.0),
                step_down: Px::new(1.0, 0.0),
            },
        ),
        // (u, v) = (w - 1 - x, h - 1 - y)
        2 => (
            width,
            height,
            Placement {
                origin: Px::new(w - 1.0, h - 1.0),
                step_across: Px::new(-1.0, 0.0),
                step_down: Px::new(0.0, -1.0),
            },
        ),
        // (u, v) = (w - 1 - y, x)
        _ => (
            height,
            width,
            Placement {
                origin: Px::new(w - 1.0, 0.0),
                step_across: Px::new(0.0, 1.0),
                step_down: Px::new(-1.0, 0.0),
            },
        ),
    };
    let (width, height) = (width as usize, height as usize);
    let mut out = Vec::new();
    out.try_reserve_exact(pixels.len()).map_err(|_| {
        EngineError::new(
            ErrorCode::EngineOutOfMemory,
            "not enough memory to turn the film upright for the trained finder",
        )
    })?;
    for y in 0..out_height as usize {
        for x in 0..out_width as usize {
            let (u, v) = match turns % 4 {
                1 => (y, height - 1 - x),
                2 => (width - 1 - x, height - 1 - y),
                _ => (width - 1 - y, x),
            };
            out.push(pixels[v * width + u]);
        }
    }
    Ok(GreyImage::new(out_width, out_height, out, placement).expect("a turn keeps every pixel"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use shapes::film::{
        DisplayWindow, FilmFacts, FilmImage, FilmKind, PixelValues, SourceFormat, WindowFunction,
        WindowSource,
    };
    use shapes::film_geometry::{FilmGeometry, QuarterTurns};

    /// A 3 by 2 film whose values name their pixel: 10 * row + column.
    fn decoded() -> DecodedFilm {
        let pixels = vec![0, 1, 2, 10, 11, 12];
        DecodedFilm {
            image: GreyImage::new(3, 2, pixels, Placement::FILM).unwrap(),
            facts: FilmFacts {
                kind: FilmKind::Front,
                width: 3,
                height: 2,
                bits_stored: 8,
                source: SourceFormat::Png,
                kept_tags: None,
                values: PixelValues::AS_STORED,
                flags: Vec::new(),
                default_window: DisplayWindow::CentreWidth {
                    centre: 127.5,
                    width: 255.0,
                    function: WindowFunction::LinearExact,
                    from: WindowSource::Percentile,
                },
                exif_quarter_turns: None,
            },
        }
    }

    fn record(turns: u8) -> Film {
        Film {
            kind: FilmKind::Front,
            image: FilmImage {
                path: FilmKind::Front.png_path().into(),
                width: 3,
                height: 2,
                bits_stored: 8,
            },
            source: SourceFormat::Png,
            kept_tags: None,
            values: PixelValues::AS_STORED,
            flags: Vec::new(),
            blanked: Vec::new(),
            geometry: FilmGeometry {
                quarter_turns: QuarterTurns::new(turns).unwrap(),
                ..FilmGeometry::UNKNOWN
            },
        }
    }

    #[test]
    fn each_turn_is_clockwise_and_maps_back_to_the_pixel_it_read() {
        // Turned once clockwise, the film's lower left pixel (10) comes to
        // the top left, worked by hand for each turn.
        let expected: [(u32, u32, [u8; 6]); 4] = [
            (3, 2, [0, 1, 2, 10, 11, 12]),
            (2, 3, [10, 0, 11, 1, 12, 2]),
            (3, 2, [12, 11, 10, 2, 1, 0]),
            (2, 3, [2, 12, 1, 11, 0, 10]),
        ];
        let film = decoded();
        for (turns, (width, height, values)) in expected.into_iter().enumerate() {
            let upright = picture(&film, &record(turns as u8)).unwrap();
            assert_eq!((upright.width(), upright.height()), (width, height));
            assert_eq!(upright.pixels(), &values);
            for y in 0..height {
                for x in 0..width {
                    // The centre of upright pixel (x, y), measured from the
                    // outer corner, lands on the centre of the film pixel
                    // whose value it holds. Every step is a whole number,
                    // so the mapping is exact.
                    let back = to_film_pixels(
                        upright.placement(),
                        [f64::from(x) + 0.5, f64::from(y) + 0.5],
                    );
                    let value = upright.get(x, y).copied().unwrap();
                    let expected = Px::new(f64::from(value % 10), f64::from(value / 10));
                    assert_eq!(back, expected, "turn {turns}, pixel ({x}, {y})");
                }
            }
        }
    }

    #[test]
    fn a_flipped_film_is_inverted_and_blanked_boxes_are_black() {
        let mut film = record(0);
        film.values.inverted = true;
        film.values.inverted_by_user = true;
        // Reaches into pixels (1, 0) and (2, 0) only.
        film.blanked = vec![PxRect {
            left: 0.6,
            top: -0.5,
            right: 2.5,
            bottom: 0.4,
        }];
        let upright = picture(&decoded(), &film).unwrap();
        assert_eq!(upright.pixels(), &[255, 0, 0, 245, 244, 243]);
    }

    #[test]
    fn blanking_looks_only_near_the_box_and_takes_exactly_the_pixels_it_reaches() {
        // Edges on pixel edges and a hair either side of them, on pixel
        // centres, between, and on the film's own outer edges, on a 7 by 5
        // film: every box with such edges is blanked exactly as a scan of
        // the whole film by the case's rule blanks it.
        let (width, height) = (7u32, 5u32);
        let edges = |size: u32| {
            let mut edges = vec![0.25, f64::from(size) / 2.0 - 0.1];
            for pixel in 0..=size {
                let edge = f64::from(pixel) - 0.5;
                edges.extend([edge.next_down(), edge, edge.next_up()]);
                edges.extend([f64::from(pixel), f64::from(pixel).next_up()]);
            }
            edges
        };
        let mut boxes = 0;
        for &left in &edges(width) {
            for &right in &edges(width) {
                for &top in &edges(height) {
                    for &bottom in &edges(height) {
                        let area = PxRect {
                            left,
                            top,
                            right,
                            bottom,
                        };
                        if !area.is_on_film(width, height) {
                            continue;
                        }
                        let mut quick = vec![9u8; (width * height) as usize];
                        blank(&mut quick, width, height, &area);
                        let mut scanned = vec![9u8; (width * height) as usize];
                        for y in 0..height {
                            for x in 0..width {
                                if area.covers_pixel(x, y) {
                                    scanned[(y * width + x) as usize] = 0;
                                }
                            }
                        }
                        assert_eq!(quick, scanned, "{area:?}");
                        boxes += 1;
                    }
                }
            }
        }
        assert!(boxes > 10_000, "{boxes} boxes tried");
    }

    #[test]
    fn a_deep_film_the_user_flipped_reads_the_same_after_the_case_is_reopened() {
        use made_up_spines::film_painter::PaintedFilm;
        // A ramp, brightest on the left, stored in 12 bits and, as the
        // control, in 8; the user flipped it. A saved case holds the
        // values flipped (as `case_file::film_png::encode_png` writes
        // them), and reopening it gives the record's values back.
        for bits in [12u8, 8] {
            let highest = (1u32 << bits) - 1;
            let (width, height) = (60u32, 8u32);
            let ramp: Vec<u16> = (0..width * height)
                .map(|i| (highest * (width - 1 - i % width) / (width - 1)) as u16)
                .collect();
            let image = GreyImage::new(width, height, ramp.clone(), Placement::FILM).unwrap();
            let facts = FilmFacts {
                kind: FilmKind::Front,
                width,
                height,
                bits_stored: bits,
                source: SourceFormat::Png,
                kept_tags: None,
                values: PixelValues::AS_STORED,
                flags: Vec::new(),
                default_window: film_reader::thumbnail::default_window(
                    &image,
                    bits,
                    &PixelValues::AS_STORED,
                ),
                exif_quarter_turns: None,
            };
            let fresh = DecodedFilm { image, facts };
            let mut film = record(0);
            film.image = FilmImage {
                path: FilmKind::Front.png_path().into(),
                width,
                height,
                bits_stored: bits,
            };
            film.values.inverted = true;
            film.values.inverted_by_user = true;
            let saved = PaintedFilm {
                kind: FilmKind::Front,
                image: GreyImage::new(
                    width,
                    height,
                    ramp.iter().map(|&value| highest as u16 - value).collect(),
                    Placement::FILM,
                )
                .unwrap(),
                bits_stored: bits,
                inverted: false,
                lead_letter: None,
                ruler: None,
                ball: None,
                field: None,
            };
            let reopened =
                film_reader::read_saved_film(&made_up_spines::film_png::encode(&saved), &film)
                    .unwrap();
            let first = picture(&fresh, &film).unwrap();
            let again = picture(&reopened, &film).unwrap();
            // Flipped: the right-hand end is now the bright one.
            assert!(
                first.pixels()[0] < first.pixels()[width as usize - 1],
                "{bits} bits"
            );
            assert_eq!(first.pixels(), again.pixels(), "{bits} bits");
        }
    }

    #[test]
    fn a_record_for_another_film_is_refused() {
        let mut film = record(0);
        film.image.width = 4;
        let error = picture(&decoded(), &film).unwrap_err();
        assert_eq!(error.code, ErrorCode::EngineOutOfSync);
    }
}
