//! The points besides the vertebrae: C7, the top of the sacrum and the hip
//! centres.

pub mod c7;
pub mod hip_centres;
pub mod sacrum_top;

use crate::handoffs::{FilmPassCache, SettledChain};
use shapes::landmarks::Landmarks;
use shapes::vertebra::VertebraOnFilm;

/// Place the independent landmarks, then derive C7 from the placed chain.
pub fn place(
    cache: &FilmPassCache,
    chain: &SettledChain,
    vertebrae: &[VertebraOnFilm],
) -> Landmarks {
    Landmarks {
        c7: c7::from_vertebrae(vertebrae),
        sacrum_top: sacrum_top::place(
            &cache.strip,
            chain.boundaries.sacrum,
            chain.boundaries.polarity,
        ),
        hips: hip_centres::find_for_cache(cache, &chain.boundaries),
    }
}
