//! Orientation answers come from the painting camera, independently of
//! the coarse image cues. Turns and facing are discrete: no tolerance.

use film_geometry::{rough_trace::RoughTrace, side_facing, upright};
use film_reader::decoded_film::DecodedFilm;
use made_up_spines::{
    answer_key::spine_for, film_layout::FilmLayout, film_painter::paint, patient::make_up_patient,
    recipes::film_recipes,
};
use shapes::film::{
    DisplayWindow, FilmFacts, FilmKind, PixelValues, SourceFormat, WindowFunction, WindowSource,
};
use shapes::film_geometry::{Certainty, ImageRight, QuarterTurns};
use shapes::made_up::{MadeUpRecipe, RecipeName, Tier};

fn film(
    name: RecipeName,
    kind: FilmKind,
    seed: u32,
    turns: Option<u8>,
) -> (DecodedFilm, FilmLayout) {
    let recipe = MadeUpRecipe {
        name,
        seed,
        tier: Tier::T2,
    };
    let patient = make_up_patient(name, seed).expect("made-up patient stacks");
    let mut exposure = film_recipes(&recipe, u64::from(seed))
        .into_iter()
        .find(|f| f.kind == kind)
        .expect("view recipe");
    if let Some(turns) = turns {
        exposure.quarter_turns = QuarterTurns::new(turns).expect("quarter turn");
    }
    let spine = spine_for(&patient, kind);
    let layout = FilmLayout::new(&exposure, spine);
    let painted = paint(&exposure, &layout, &patient, spine, u64::from(seed));
    let facts = FilmFacts {
        kind,
        width: painted.image.width(),
        height: painted.image.height(),
        bits_stored: painted.bits_stored,
        source: SourceFormat::Png,
        kept_tags: None,
        values: PixelValues::AS_STORED,
        flags: Vec::new(),
        default_window: DisplayWindow::CentreWidth {
            centre: 32767.5,
            width: 65535.0,
            function: WindowFunction::Linear,
            from: WindowSource::Percentile,
        },
        exif_quarter_turns: None,
    };
    (
        DecodedFilm {
            image: painted.image,
            facts,
        },
        layout,
    )
}

#[test]
fn named_upside_down_and_both_quarter_turns_are_proposed_from_pixels() {
    for (name, turns, seed) in [
        (RecipeName::UpsideDown, None, 3),
        (RecipeName::UpsideDown, None, 7),
        (RecipeName::UpsideDown, None, 19),
        (RecipeName::Straight, Some(1), 3),
        (RecipeName::Straight, Some(3), 3),
        (RecipeName::Straight, None, 3),
        (RecipeName::RightThoracic40, Some(1), 7),
        (RecipeName::RightThoracic40, Some(3), 19),
    ] {
        let (mut film, truth) = film(name, FilmKind::Front, seed, turns);
        let pixels = film.image.clone();
        let trace = RoughTrace::new(&film).expect("coarse image");
        let guess = upright::guess(&film, &trace);
        assert_eq!(
            guess.quarter_turns, truth.geometry.quarter_turns,
            "{name:?} seed {seed}: {guess:?}"
        );
        assert_ne!(guess.certainty, Certainty::None, "{name:?}");
        assert_eq!(film.image, pixels, "only a view turn is proposed");
        // A conflicting EXIF header remains visible as negative evidence
        // and cannot overrule a clear anatomical cue.
        film.facts.exif_quarter_turns = QuarterTurns::new((guess.quarter_turns.get() + 2) % 4);
        let conflict = upright::guess(&film, &trace);
        assert_eq!(conflict.quarter_turns, truth.geometry.quarter_turns);
        assert_eq!(conflict.certainty, Certainty::Low);
        assert!(conflict.evidence.iter().any(|e| e.cue
            == shapes::film_geometry::UprightCue::ExifOrientation
            && e.strength == Some(-1.0)));
    }
}

#[test]
fn left_and_right_facing_sides_are_proposed_from_their_anatomy() {
    for (name, seed) in [RecipeName::SideFacingLeft, RecipeName::Straight]
        .into_iter()
        .flat_map(|name| [3, 7, 19].map(|seed| (name, seed)))
    {
        let (film, truth) = film(name, FilmKind::Side, seed, None);
        let trace = RoughTrace::new(&film).expect("coarse image");
        let upright = upright::guess(&film, &trace);
        assert_eq!(
            upright.quarter_turns, truth.geometry.quarter_turns,
            "{upright:?}"
        );
        let profile = trace.profile(upright.quarter_turns);
        let guess = side_facing::guess(&film, &profile, upright.quarter_turns);
        assert_eq!(
            guess.image_right, truth.geometry.image_right,
            "{name:?} seed {seed}: {guess:?}"
        );
        assert_ne!(guess.certainty, Certainty::None);
        assert_eq!(guess.file_says, None, "there are no file orientation tags");
    }
}

#[test]
fn side_facing_is_unchanged_by_a_quarter_turned_storage_grid() {
    let (film, _) = film(RecipeName::SideFacingLeft, FilmKind::Side, 3, Some(1));
    let trace = RoughTrace::new(&film).expect("coarse image");
    let turns = upright::guess(&film, &trace).quarter_turns;
    assert_eq!(turns.get(), 1);
    assert_eq!(
        side_facing::guess(&film, &trace.profile(turns), turns).image_right,
        Some(ImageRight::Posterior)
    );
}
