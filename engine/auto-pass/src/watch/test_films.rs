//! Small painted films for the orchestration integration tests. The key
//! comes from the painter's independent camera and is never a finder input.

use film_reader::decoded_film::DecodedFilm;
use made_up_spines::{answer_key, film_layout::FilmLayout, film_painter, patient, recipes};
use shapes::auto_pass::{Acquisition, AutoPassInput, ChainOptions, FrontFinder, Population};
use shapes::film::{
    DisplayWindow, Film, FilmFacts, FilmImage, FilmKind, PixelValues, SourceFormat, WindowFunction,
    WindowSource,
};
use shapes::made_up::{FilmAnswer, MadeUpRecipe, RecipeName, Tier};

pub fn film(kind: FilmKind) -> (DecodedFilm, Film, FilmAnswer) {
    let recipe = MadeUpRecipe {
        name: RecipeName::Straight,
        seed: 17,
        tier: Tier::T0,
    };
    let patient = patient::make_up_patient(recipe.name, recipe.seed).expect("fixed recipe stacks");
    let spine = answer_key::spine_for(&patient, kind);
    let mut drawing = recipes::film_recipes(&recipe, u64::from(recipe.seed))
        .into_iter()
        .find(|drawing| drawing.kind == kind)
        .expect("four film kinds");
    // One-millimetre detector pixels keep orchestration checks small.
    // These tests measure no subpixel accuracy and set no accuracy gate.
    drawing.mm_per_px = 1.0;
    let layout = FilmLayout::new(&drawing, spine);
    let painted = film_painter::paint(&drawing, &layout, &patient, spine, u64::from(recipe.seed));
    let truth = answer_key::film_answer(&layout, spine, &patient.recipe.template.levels());
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
            centre: 2048.0,
            width: 4096.0,
            function: WindowFunction::Linear,
            from: WindowSource::Percentile,
        },
        exif_quarter_turns: None,
    };
    let record = Film {
        kind,
        image: FilmImage {
            path: kind.png_path().to_owned(),
            width: facts.width,
            height: facts.height,
            bits_stored: facts.bits_stored,
        },
        source: facts.source,
        kept_tags: None,
        values: facts.values.clone(),
        flags: Vec::new(),
        blanked: Vec::new(),
        geometry: layout.geometry,
    };
    (
        DecodedFilm {
            image: painted.image,
            facts,
        },
        record,
        truth,
    )
}

pub fn input(films: Vec<Film>) -> AutoPassInput {
    AutoPassInput {
        case_version: 42,
        films,
        options: ChainOptions {
            acquisition: Acquisition::Separate,
            population: Population::Adult,
            template: None,
        },
        first_vertebra_id: 1000,
        front_finder: FrontFinder::HandBuilt,
    }
}
