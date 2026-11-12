//! One-call entry points for native use: the stepper drained, so a native
//! run and a watched one give the same answer.

use crate::watch::{Found, Search, TrainedPass};
use darkroom::grey_image::GreyImage;
use film_reader::decoded_film::DecodedFilm;
use shapes::auto_pass::TrainedPassResult;
use shapes::errors::EngineError;
use shapes::film::Film;
use std::rc::Rc;

/// One unit of work per call when a native caller drains the pass. It
/// changes only how the work is scheduled, never a frame or the answer.
const WORK_UNITS: u32 = 1;

/// The trained finder on one front film, read through its record, run to
/// the end. Vertebra ids start at `first_vertebra_id`.
pub fn find_on_front_film(
    decoded: &DecodedFilm,
    film: &Film,
    first_vertebra_id: u32,
) -> Result<TrainedPassResult, EngineError> {
    let mut pass = TrainedPass::new(Rc::new(decoded.clone()), film.clone(), first_vertebra_id)?;
    while !pass.advance(WORK_UNITS)?.done {}
    pass.result()
}

/// The finder on a picture already upright and in 8 bits, run to the end,
/// with what each stage found. Positions are in the picture's own pixels,
/// measured from its outer top-left corner. This is the pipeline the
/// known answers were recorded from; a film goes through
/// [`find_on_front_film`] instead, which reads it through its record first.
pub fn find_on_picture(picture: GreyImage<u8>) -> Result<Found, EngineError> {
    let mut search = Search::new(picture);
    let mut frames = Vec::new();
    while !search.is_done() {
        search.unit(&mut frames)?;
    }
    Ok(search.found().expect("the search is done"))
}
