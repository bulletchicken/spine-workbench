//! The full point set: the solved chain turned into the marks each film
//! shows.

pub mod back_to_film;
pub mod bending;
pub mod front;
pub mod side;

mod endplates;
mod pedicles;
mod walls;

#[cfg(test)]
mod tests;

use crate::handoffs::{FilmPassCache, SettledChain};
use shapes::errors::EngineError;
use shapes::film::FilmKind;
use shapes::naming::LevelNaming;
use shapes::vertebra::{FilmMarks, VertebraMarks, VertebraOnFilm};

/// Place a settled chain using the film worker's retained evidence. The
/// caller allocates the first free case-wide id; every returned id is new.
pub fn place_points(
    cache: &FilmPassCache,
    chain: &SettledChain,
    naming: LevelNaming,
    first_id: u32,
) -> Result<FilmMarks, EngineError> {
    let strip = &cache.strip;
    let boundaries = &chain.boundaries;
    back_to_film::validate(strip)?;
    if boundaries.film != strip.film
        || cache.evidence.film != strip.film
        || cache.centre_line.film != strip.film
        || boundaries.template != naming.template
        || boundaries.bodies.len() != chain.confidence.len()
        || boundaries
            .bodies
            .windows(2)
            .any(|pair| pair[0].level >= pair[1].level)
    {
        return Err(back_to_film::refused(
            "the chain and film cache do not agree",
        ));
    }
    let count = u32::try_from(boundaries.bodies.len())
        .map_err(|_| back_to_film::refused("too many vertebrae"))?;
    first_id
        .checked_add(count)
        .ok_or_else(|| back_to_film::refused("vertebra ids exhausted"))?;
    let mut vertebrae = Vec::with_capacity(count as usize);
    let ring_evidence = if strip.film == FilmKind::Front {
        pedicles::evidence(strip)
    } else {
        None
    };
    for (index, (body, confidence)) in boundaries.bodies.iter().zip(&chain.confidence).enumerate() {
        let polarity = boundaries.polarity;
        let marks = match strip.film {
            FilmKind::Front => VertebraMarks::Front {
                points: front::place_with_rings(strip, body, polarity, ring_evidence.as_ref())?,
            },
            FilmKind::Side => VertebraMarks::Side {
                points: side::place(strip, body, polarity)?,
            },
            FilmKind::BendLeft | FilmKind::BendRight => VertebraMarks::Bending {
                lines: bending::place(strip, body, polarity)?,
            },
        };
        vertebrae.push(VertebraOnFilm {
            id: first_id + index as u32,
            level: body.level,
            marks,
            confidence: confidence.clone(),
            outline: None,
        });
    }
    let landmarks = crate::landmarks::place(cache, chain, &vertebrae);
    Ok(FilmMarks {
        film: strip.film,
        vertebrae,
        landmarks,
        naming,
        named_for: back_to_film::facing(strip)?,
    })
}
