//! Marker answers come from the independent film painter, including its
//! block-letter outlines, not from the detector's parametric glyphs.

use film_geometry::lead_marker;
use film_reader::decoded_film::DecodedFilm;
use made_up_spines::film_painter::PaintedFilm;
use shapes::film::{
    DisplayWindow, FilmFacts, FilmKind, PixelValues, SourceFormat, WindowFunction, WindowSource,
};
use shapes::made_up::{MadeUpRecipe, RecipeName, Tier};

fn decoded(film: &PaintedFilm) -> DecodedFilm {
    DecodedFilm {
        image: film.image.clone(),
        facts: FilmFacts {
            kind: film.kind,
            width: film.image.width(),
            height: film.image.height(),
            bits_stored: film.bits_stored,
            source: SourceFormat::Png,
            kept_tags: None,
            values: PixelValues::AS_STORED,
            flags: vec![],
            default_window: DisplayWindow::CentreWidth {
                centre: 0.5,
                width: 1.0,
                function: WindowFunction::Linear,
                from: WindowSource::Percentile,
            },
            exif_quarter_turns: None,
        },
    }
}

#[test]
fn independent_block_letters_are_found_in_the_mirrored_and_turned_recipes() {
    for name in [
        RecipeName::Straight,
        RecipeName::MirroredFront,
        RecipeName::UpsideDown,
    ] {
        let recipe = MadeUpRecipe {
            name,
            seed: 42,
            tier: Tier::T2,
        };
        let patient = made_up_spines::patient::make_up_patient(name, recipe.seed).unwrap();
        let film_recipe =
            made_up_spines::recipes::film_recipes(&recipe, u64::from(recipe.seed)).remove(0);
        let spine = made_up_spines::answer_key::spine_for(&patient, FilmKind::Front);
        let layout = made_up_spines::film_layout::FilmLayout::new(&film_recipe, spine);
        let film = made_up_spines::film_painter::paint(
            &film_recipe,
            &layout,
            &patient,
            spine,
            u64::from(recipe.seed),
        );
        let expected = film.lead_letter.unwrap();
        let (letters, _) = lead_marker::find(&decoded(&film));
        let matching = letters.iter().find(|found| {
            let dx = (found.bounds.left + found.bounds.right
                - expected.bounds.left
                - expected.bounds.right)
                .abs()
                * 0.5;
            let dy = (found.bounds.top + found.bounds.bottom
                - expected.bounds.top
                - expected.bounds.bottom)
                .abs()
                * 0.5;
            // A four-pixel averaging cell may move each box edge by one cell.
            dx <= 4.0 && dy <= 4.0
        });
        assert!(
            matching.is_some(),
            "{name:?}: {letters:?}; expected {expected:?}"
        );
        let found = matching.unwrap();
        assert_eq!(found.letter, expected.letter, "{name:?}");
        assert_eq!(found.mirrored, expected.mirrored, "{name:?}");
        assert!(found.score >= 0.60 && found.margin >= 0.15);
        if name == RecipeName::MirroredFront {
            let guess = film_geometry::sense_geometry(&decoded(&film)).unwrap();
            assert!(guess.image_right.looks_mirrored);
        }
    }
}

fn phantom(letter: bool, mirrored: bool, quarter: usize, cut_out: bool) -> DecodedFilm {
    use darkroom::grey_image::{GreyImage, Placement};
    let (w, h) = (512usize, 768usize);
    let mut pixels = vec![1800u16; w * h];
    for y in 0..h {
        for x in 0..w {
            // Broad anatomy and a straight collimation band are independent
            // distractors. The letter below is a polygon union, not a template.
            if (180..330).contains(&x) && (100..700).contains(&y) {
                pixels[y * w + x] = 20000 + (y % 7) as u16 * 200;
            }
            if x < 12 {
                pixels[y * w + x] = 60000;
            }
            let (mut u, mut v) = (x as isize - 88, y as isize - 130);
            for _ in 0..quarter {
                (u, v) = (v, -u);
            }
            if mirrored {
                u = -u;
            }
            let (u, v) = (u + 26, v + 40);
            let inside = (0..52).contains(&u) && (0..80).contains(&v);
            if cut_out && (-16..68).contains(&u) && (-16..96).contains(&v) {
                pixels[y * w + x] = 57000;
            }
            if letter && inside && (u < 15 || v >= 65) {
                pixels[y * w + x] = if cut_out { 3500 } else { 57000 };
            }
        }
    }
    let image = GreyImage::new(w as u32, h as u32, pixels, Placement::FILM).unwrap();
    let painted = PaintedFilm {
        kind: FilmKind::Front,
        image,
        bits_stored: 16,
        inverted: false,
        lead_letter: None,
        ruler: None,
        ball: None,
        field: None,
    };
    decoded(&painted)
}

#[test]
fn isolated_polygon_letters_report_original_boxes_rotation_and_mirroring() {
    use shapes::film_geometry::MarkerLetter;
    for cut_out in [false, true] {
        for mirrored in [false, true] {
            for quarter in 0..4 {
                let film = phantom(true, mirrored, quarter, cut_out);
                let (found, _) = lead_marker::find(&film);
                assert_eq!(
                    found.len(),
                    1,
                    "cut {cut_out}, mirror {mirrored}, q {quarter}: {found:?}"
                );
                assert_eq!(found[0].letter, MarkerLetter::L);
                assert_eq!(
                    found[0].mirrored, mirrored,
                    "cut {cut_out}, q {quarter}: {found:?}"
                );
                assert_eq!(found[0].cut_out, cut_out);
                // Four-pixel averaging can tilt a short stroke by one
                // five-degree template bin; allow that raster uncertainty.
                let error = (found[0].rotation_deg as i32 - (quarter * 90) as i32 + 180)
                    .rem_euclid(360)
                    - 180;
                assert!(error.abs() <= 5, "{found:?}");
            }
        }
    }
}

#[test]
fn anatomy_and_collimation_without_a_letter_offer_no_marker() {
    let (found, _) = lead_marker::find(&phantom(false, false, 0, false));
    assert!(found.is_empty());
}

#[test]
fn a_uniform_maximum_letter_is_reported_as_digital() {
    use shapes::film_geometry::MarkerMaterial;
    let mut film = phantom(true, false, 0, false);
    film.image = film.image.map(|p| if *p == 57000 { 60000 } else { *p });
    let (found, _) = lead_marker::find(&film);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].material, MarkerMaterial::Digital);
}

#[test]
fn neighbouring_word_letters_are_text_hints_not_markers() {
    use darkroom::grey_image::{GreyImage, Placement};
    let mut film = phantom(false, false, 0, false);
    let mut pixels = film.image.pixels().to_vec();
    // Keep the word clear of anatomy: otherwise the morphology joins
    // its last letter to the broad body and it is no longer a text part.
    for y in 0..160 {
        for x in 180..330 {
            pixels[y * 512 + x] = 1800;
        }
    }
    for left in [40, 120, 200] {
        for v in 0..80 {
            for u in 0..52 {
                if u < 15 || v >= 65 {
                    pixels[(v + 10) * 512 + u + left] = 57000;
                }
            }
        }
    }
    film.image = GreyImage::new(512, 768, pixels, Placement::FILM).unwrap();
    let (found, hints) = lead_marker::find(&film);
    assert!(found.is_empty(), "{found:?}");
    assert!(hints.len() >= 3, "{hints:?}");
}

fn in_polygon(x: f64, y: f64, vertices: &[[f64; 2]]) -> bool {
    let mut inside = false;
    for (a, b) in vertices.iter().zip(vertices.iter().cycle().skip(1)) {
        if (a[1] > y) != (b[1] > y) && x < a[0] + (y - a[1]) * (b[0] - a[0]) / (b[1] - a[1]) {
            inside = !inside;
        }
    }
    inside
}

#[test]
fn independent_polygon_r_has_the_right_letter_and_handedness() {
    use darkroom::grey_image::{GreyImage, Placement};
    use shapes::film_geometry::MarkerLetter;
    let outside = [
        [0.0, 0.0],
        [40.0, 0.0],
        [52.0, 10.0],
        [52.0, 36.0],
        [35.0, 48.0],
        [52.0, 80.0],
        [34.0, 80.0],
        [15.0, 50.0],
        [15.0, 80.0],
        [0.0, 80.0],
    ];
    let hole = [
        [15.0, 14.0],
        [35.0, 14.0],
        [38.0, 20.0],
        [38.0, 30.0],
        [33.0, 34.0],
        [15.0, 34.0],
    ];
    for mirrored in [false, true] {
        for quarter in 0..4 {
            let mut film = phantom(false, false, 0, false);
            let mut pixels = film.image.pixels().to_vec();
            for y in 60..200 {
                for x in 20..160 {
                    let (mut u, mut v) = (x as f64 - 88.0, y as f64 - 130.0);
                    for _ in 0..quarter {
                        (u, v) = (v, -u);
                    }
                    if mirrored {
                        u = -u;
                    }
                    let (u, v) = (u + 26.0, v + 40.0);
                    if in_polygon(u, v, &outside) && !in_polygon(u, v, &hole) {
                        pixels[y * 512 + x] = 57000;
                    }
                }
            }
            film.image = GreyImage::new(512, 768, pixels, Placement::FILM).unwrap();
            let (found, _) = lead_marker::find(&film);
            assert_eq!(found.len(), 1, "mirror {mirrored}, q {quarter}: {found:?}");
            assert_eq!(found[0].letter, MarkerLetter::R);
            assert_eq!(found[0].mirrored, mirrored);
        }
    }
}

#[test]
fn noisy_mirrored_front_proposes_the_answer_key_side() {
    use shapes::film_geometry::MarkerMaterial;
    let recipe = MadeUpRecipe {
        name: RecipeName::MirroredFront,
        seed: 42,
        tier: Tier::T3,
    };
    let patient = made_up_spines::patient::make_up_patient(recipe.name, recipe.seed).unwrap();
    let film_recipe =
        made_up_spines::recipes::film_recipes(&recipe, u64::from(recipe.seed)).remove(0);
    let spine = made_up_spines::answer_key::spine_for(&patient, FilmKind::Front);
    let layout = made_up_spines::film_layout::FilmLayout::new(&film_recipe, spine);
    let film = made_up_spines::film_painter::paint(
        &film_recipe,
        &layout,
        &patient,
        spine,
        u64::from(recipe.seed),
    );
    let guess = film_geometry::sense_geometry(&decoded(&film)).unwrap();
    assert!(
        guess
            .lead_markers
            .iter()
            .any(|m| m.material == MarkerMaterial::Lead)
    );
    assert_eq!(
        guess.image_right.image_right,
        Some(layout.camera.image_right),
        "{guess:?}"
    );
    assert!(guess.image_right.looks_mirrored);
}
