//! The full point set: the solved chain turned into the marks each film
//! shows.

pub mod back_to_film;
pub mod bending;
pub mod front;
pub mod side;

mod endplates;
mod pedicles;
mod recovery;
mod wall_pairs;
mod walls;
mod widths;

#[cfg(test)]
mod tests;

use crate::handoffs::{FilmPassCache, SettledChain};
use shapes::confidence::{ConfidenceBand, ConfidenceReason};
use shapes::errors::EngineError;
use shapes::film::FilmKind;
use shapes::naming::LevelNaming;
use shapes::vertebra::{FilmMarks, VertebraMarks, VertebraOnFilm};

/// Place a settled chain using the film worker's retained evidence. The
/// caller allocates the first free case-wide id; every returned id is new.
pub fn place_points(
    cache: &FilmPassCache,
    chain: &SettledChain,
    mut naming: LevelNaming,
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
    let guides: Vec<_> = boundaries
        .bodies
        .iter()
        .map(|body| widths::estimate(strip, body))
        .collect();
    let fit = |index: usize, width: f64| {
        walls::fit_body(strip, &boundaries.bodies[index], boundaries.polarity, width).ok()
    };
    let first: Vec<_> = guides
        .iter()
        .enumerate()
        .map(|(i, width)| width.and_then(|width| fit(i, width)))
        .collect();
    // Neighbours vote only against a width outlier. Keep a consistent
    // local fit intact rather than moving supported corners with every
    // small change in a nearby body's estimate.
    let fitted: Vec<_> = first
        .iter()
        .enumerate()
        .map(|(i, own)| {
            let own = own.as_ref()?;
            let mut votes: Vec<_> = first
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i && j.abs_diff(i) <= 2)
                .filter_map(|(_, fit)| fit.as_ref())
                .filter(|fit| {
                    fit.partial
                        .iter()
                        .flatten()
                        .filter(|mark| !mark.inferred)
                        .count()
                        >= 2
                })
                .map(|fit| fit.half_width)
                .collect();
            if votes.len() >= 2 {
                let near = widths::median(&mut votes)?;
                if own.half_width < 0.75 * near || own.half_width > 1.25 * near {
                    let target = guides[i]?.clamp(0.85 * near, 1.15 * near);
                    return fit(i, target);
                }
            }
            Some(own.clone())
        })
        .collect();
    let provisional: Vec<_> = fitted
        .iter()
        .enumerate()
        .map(|(i, fitted)| walls::complete(strip, &boundaries.bodies[i], fitted.as_ref()?, &[]))
        .collect();
    let mut kept = Vec::new();
    for (index, (body, confidence)) in boundaries.bodies.iter().zip(&chain.confidence).enumerate() {
        let Some(fitted) = fitted[index].as_ref() else {
            continue;
        };
        let neighbours: Vec<_> = provisional
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != index && i.abs_diff(index) <= 2)
            .filter_map(|(_, corners)| {
                let c = corners.as_ref()?;
                Some([c.sup[0].at, c.sup[1].at, c.inf[1].at, c.inf[0].at])
            })
            .collect();
        let Some(corners) = walls::complete(strip, body, fitted, &neighbours) else {
            continue;
        };
        let marks = match strip.film {
            FilmKind::Front => VertebraMarks::Front {
                points: front::from_corners(strip, body, &corners, ring_evidence.as_ref())?,
            },
            FilmKind::Side => VertebraMarks::Side {
                points: side::from_corners(strip, &corners)?,
            },
            FilmKind::BendLeft | FilmKind::BendRight => VertebraMarks::Bending {
                lines: bending::from_corners(strip, &corners)?,
            },
        };
        let mut confidence = confidence.clone();
        if marks.corners().iter().any(|mark| mark.inferred) {
            confidence.score *= 0.5;
            confidence.band = ConfidenceBand::Low;
            confidence.calibrated = false;
            if !confidence.reasons.contains(&ConfidenceReason::NoEvidence) {
                confidence.reasons.push(ConfidenceReason::NoEvidence);
            }
        }
        kept.push(index);
        vertebrae.push(VertebraOnFilm {
            id: first_id + vertebrae.len() as u32,
            level: body.level,
            marks,
            confidence,
            outline: None,
        });
    }
    if vertebrae.is_empty() {
        return Err(back_to_film::refused("no vertebra could be placed"));
    }
    if kept.len() != boundaries.bodies.len() {
        let retain = |names: Option<Vec<_>>| {
            names.and_then(|names| {
                (names.len() == boundaries.bodies.len())
                    .then(|| kept.iter().map(|&i| names[i]).collect())
            })
        };
        naming.from_top = retain(naming.from_top);
        naming.from_bottom = retain(naming.from_bottom);
        // A supplied confirmation refers to the old allocation; omission
        // must not silently make that id refer to a different vertebra.
        naming.l5_confirmed_on = None;
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
