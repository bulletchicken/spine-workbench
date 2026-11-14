//! The native entry drains the same pass used by the film worker.
use film_reader::decoded_film::DecodedFilm;
use shapes::{auto_pass::TrainedPassResult, errors::EngineError, film::Film};
use std::rc::Rc;

pub fn find_on_side_film(
    decoded: &DecodedFilm,
    film: &Film,
    first_vertebra_id: u32,
) -> Result<TrainedPassResult, EngineError> {
    let mut pass = super::SidePass::new(Rc::new(decoded.clone()), film.clone(), first_vertebra_id)?;
    while !pass.advance(1)?.done {}
    pass.result()
}
