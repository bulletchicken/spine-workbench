//! The made-up front film the known answers were recorded on,
//! `spine make-up straight --seed 7 --tier t2`, painted again from its
//! recipe. It is not kept in git. Its PNG is checked against the recorded
//! SHA-256 before anything is compared, so a change in the painter is never
//! taken for one in the finder. Painting it takes seconds in a test build,
//! so each test binary paints it once and shares it.

#![allow(dead_code)]

use darkroom::grey_image::{GreyImage, Placement};
use made_up_spines::film_painter::PaintedFilm;
use made_up_spines::{answer_key, film_layout::FilmLayout, film_painter, film_png, patient};
use sha2::{Digest, Sha256};
use shapes::film::FilmKind;
use shapes::made_up::{MadeUpRecipe, RecipeName, Tier};
use std::sync::OnceLock;

/// The film as painted, and its PNG.
pub struct MadeUpFront {
    /// Stored in more than 8 bits, bone bright.
    pub painted: PaintedFilm,
    /// What the app would be handed.
    pub png: Vec<u8>,
}

/// The case the film is the front film of.
pub const RECIPE: MadeUpRecipe = MadeUpRecipe {
    name: RecipeName::Straight,
    seed: 7,
    tier: Tier::T2,
};

/// The film, painted the first time it is asked for.
pub fn made_up_front() -> &'static MadeUpFront {
    static FILM: OnceLock<MadeUpFront> = OnceLock::new();
    FILM.get_or_init(paint)
}

fn paint() -> MadeUpFront {
    let recipe = RECIPE;
    let patient = patient::make_up_patient(recipe.name, recipe.seed).expect("a made-up patient");
    let spine = answer_key::spine_for(&patient, FilmKind::Front);
    let drawing = made_up_spines::recipes::film_recipes(&recipe, u64::from(recipe.seed))
        .into_iter()
        .find(|film| film.kind == FilmKind::Front)
        .expect("a front film recipe");
    let layout = FilmLayout::new(&drawing, spine);
    let painted = film_painter::paint(&drawing, &layout, &patient, spine, u64::from(recipe.seed));
    let png = film_png::encode(&painted);
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../known-answers/manifest.json"))
            .expect("manifest.json reads");
    assert_eq!(
        sha256(&png),
        manifest["filmSha256"].as_str().expect("a fingerprint"),
        "the painted film is not the one the answers were recorded on"
    );
    assert_eq!(
        manifest["toEightBits"],
        "each 16-bit value shifted right by eight"
    );
    assert!(painted.bits_stored > 8, "the film is stored in 16 bits");
    MadeUpFront { painted, png }
}

impl MadeUpFront {
    /// The film in 8 bits by the known answers' own rule: each 16-bit value
    /// as the PNG holds it, shifted right by eight. That rule belongs to
    /// the answers alone; the finder reads a deep film through its opening
    /// window.
    pub fn top_eight_bits(&self) -> GreyImage<u8> {
        let image = &self.painted.image;
        let shift = 16 - u32::from(self.painted.bits_stored);
        let pixels = image
            .pixels()
            .iter()
            .map(|&value| ((value << shift) >> 8) as u8)
            .collect();
        GreyImage::new(image.width(), image.height(), pixels, Placement::FILM)
            .expect("one value per pixel")
    }
}

pub fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
