//! `FilmDesk`: one decoded film and its cached results, kept in the worker
//! that owns the film. A frozen contract.
//!
//! `FilmDesk` is a JavaScript class. Each export file adds its own
//! `#[wasm_bindgen] impl FilmDesk` block, so the class spans files owned
//! by different slices without anyone editing this one: `open`, `facts`,
//! `thumbnail` and `display_pixels` live in `exports/film_reader.rs`, the
//! geometry calls in `exports/film_geometry.rs`, and so on.
//!
//! The film and the cache are shared through `Rc`, so a `FilmPass` started
//! from a desk holds the same film without copying it, and finishing the
//! pass fills the desk's cache. Nothing on a desk is ever saved: marks live
//! in the app, and every request carries the film's current marks.

use auto_pass::handoffs::FilmPassCache;
use film_reader::decoded_film::DecodedFilm;
use shapes::film_geometry::GeometryGuess;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::wasm_bindgen;

/// One film laid out in a worker while the user works on it.
#[wasm_bindgen]
pub struct FilmDesk {
    /// The film as read: never turned, flipped, resized or masked.
    #[wasm_bindgen(skip)]
    pub film: Rc<DecodedFilm>,
    /// What has been worked out about the film so far.
    #[wasm_bindgen(skip)]
    pub cache: Rc<RefCell<DeskCache>>,
}

/// Results kept so later calls on the same film need not repeat them.
#[derive(Debug, Default)]
pub struct DeskCache {
    /// The engine's geometry proposals, once sensed.
    pub geometry: Option<GeometryGuess>,
    /// The film pass's maps, centre line, strip and barcode, once a pass
    /// has finished; the adjusting tools read them.
    pub film_pass: Option<FilmPassCache>,
}

impl FilmDesk {
    /// A desk for a film just read, with nothing cached yet.
    pub fn new(film: DecodedFilm) -> Self {
        FilmDesk {
            film: Rc::new(film),
            cache: Rc::new(RefCell::new(DeskCache::default())),
        }
    }
}
