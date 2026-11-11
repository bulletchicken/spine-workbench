//! One-call entry points for native use: the film pass, the chain pass and
//! placing points, each run to the end without watching.

use crate::handoffs::{FilmPassCache, SettledChain};
use crate::settings::{BARCODE_SIZE_FACTOR, DEFAULT_WORK_UNITS};
use crate::watch::{ChainPass, FilmPass};
use crate::{centre_line, evidence, points, strip};
use film_reader::decoded_film::DecodedFilm;
use shapes::auto_pass::{
    AutoPassInput, AutoPassResult, ChainAnswer, ChainOptions, FilmChain, FilmPassResult,
};
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::Film;
use shapes::vertebra::FilmMarks;
use shapes::watch_frames::WatchFrame;
use std::rc::Rc;

/// Run the film stages directly, retaining the same evidence the film
/// worker keeps for placing points and subsequent edits.
pub fn film_pass(
    decoded: &DecodedFilm,
    film: &Film,
) -> Result<(FilmPassResult, FilmPassCache), EngineError> {
    let (evidence, _) = evidence::build_for_film(decoded, film)?;
    let centre_line = centre_line::trace(&evidence, &film.geometry)?;
    let strip = strip::unroll::build(&decoded.image, &evidence, &centre_line, &film.geometry)?;
    let barcode = strip::barcode::build(&strip, BARCODE_SIZE_FACTOR)?;
    let mut warnings = Vec::new();
    for warning in evidence
        .warnings
        .iter()
        .chain(&centre_line.warnings)
        .chain(strip::warnings(&strip).iter())
    {
        if !warnings.contains(warning) {
            warnings.push(warning.clone());
        }
    }
    let result = FilmPassResult {
        film: film.kind,
        strip: strip.to_bytes(),
        scale_assumed: strip.scale_assumed,
        warnings,
    };
    Ok((
        result,
        FilmPassCache {
            evidence,
            centre_line,
            strip,
            barcode,
        },
    ))
}

/// Solve the worker batons without observing their checkpoints. The same
/// state machine is used so watch scheduling cannot select another chain.
pub fn chain_pass(
    films: &[FilmPassResult],
    options: &ChainOptions,
) -> Result<ChainAnswer, EngineError> {
    let mut pass = ChainPass::new(films.to_vec(), options.clone())?;
    while !pass.advance(DEFAULT_WORK_UNITS)?.done {}
    pass.result()
}

/// Turn one chain baton into marks using its film worker's retained cache.
pub fn place_points(
    cache: &FilmPassCache,
    chain: &FilmChain,
    first_id: u32,
) -> Result<FilmMarks, EngineError> {
    if chain.film != cache.strip.film {
        return Err(refused("the chain belongs to another film"));
    }
    let settled = SettledChain::from_bytes(&chain.chain)?;
    points::place_points(cache, &settled, chain.naming.clone(), first_id)
}

/// The complete automatic pass, in the order of `input.films`. Only the
/// image and film record enter a finder; an answer key has no entry here.
pub fn auto_pass(
    input: &AutoPassInput,
    decoded: &[DecodedFilm],
) -> Result<AutoPassResult, EngineError> {
    validate(input, decoded)?;
    let mut results = Vec::with_capacity(decoded.len());
    let mut caches = Vec::with_capacity(decoded.len());
    for (image, film) in decoded.iter().zip(&input.films) {
        let (result, cache) = film_pass(image, film)?;
        results.push(result);
        caches.push(cache);
    }
    let chain = chain_pass(&results, &input.options)?;
    finish(input, &caches, chain)
}

/// Record every fixed checkpoint without applying worker-side thinning.
/// Film streams are recorded in input order; live workers may interleave
/// them while preserving the order within each film.
pub fn watch_auto_pass(
    input: &AutoPassInput,
    decoded: &[DecodedFilm],
    work_units: u32,
) -> Result<(AutoPassResult, Vec<WatchFrame>), EngineError> {
    validate(input, decoded)?;
    if work_units == 0 {
        return Err(refused("a draining run needs at least one work unit"));
    }
    let mut results = Vec::with_capacity(decoded.len());
    let mut caches = Vec::with_capacity(decoded.len());
    let mut frames = Vec::new();
    for (image, film) in decoded.iter().zip(&input.films) {
        let mut pass = FilmPass::new(Rc::new(image.clone()), film.clone())?;
        loop {
            let step = pass.advance(work_units)?;
            frames.extend(step.frames);
            if step.done {
                break;
            }
        }
        results.push(pass.result()?);
        caches.push(pass.take_cache()?);
    }
    let mut pass = ChainPass::new(results, input.options.clone())?;
    loop {
        let step = pass.advance(work_units)?;
        frames.extend(step.frames);
        if step.done {
            break;
        }
    }
    Ok((finish(input, &caches, pass.result()?)?, frames))
}

fn validate(input: &AutoPassInput, decoded: &[DecodedFilm]) -> Result<(), EngineError> {
    if input.films.is_empty()
        || input.films.len() > 4
        || decoded.len() != input.films.len()
        || input.films.iter().enumerate().any(|(index, film)| {
            film.kind != decoded[index].facts.kind
                || input.films[..index].iter().any(|old| old.kind == film.kind)
        })
    {
        return Err(refused("a pass needs one to four distinct, matching films"));
    }
    Ok(())
}

fn finish(
    input: &AutoPassInput,
    caches: &[FilmPassCache],
    chain: ChainAnswer,
) -> Result<AutoPassResult, EngineError> {
    if chain.films.len() != caches.len() {
        return Err(refused("the solved chains do not match the films"));
    }
    let mut next_id = input.first_vertebra_id;
    let mut marks = Vec::with_capacity(caches.len());
    let mut warnings = Vec::new();
    // The worker conductor first receives every film's warnings, then
    // the chain's. Preserve that order and stable deduplication natively.
    for cache in caches {
        for warning in cache
            .evidence
            .warnings
            .iter()
            .chain(&cache.centre_line.warnings)
            .chain(strip::warnings(&cache.strip).iter())
        {
            if !warnings.contains(warning) {
                warnings.push(warning.clone());
            }
        }
    }
    for (film, cache) in input.films.iter().zip(caches) {
        let found = chain
            .films
            .iter()
            .find(|found| found.film == film.kind)
            .ok_or_else(|| refused("a solved film is missing"))?;
        let placed = place_points(cache, found, next_id)?;
        next_id = next_id
            .checked_add(placed.vertebrae.len() as u32)
            .ok_or_else(|| refused("vertebra ids exhausted"))?;
        for warning in &found.warnings {
            if !warnings.contains(warning) {
                warnings.push(warning.clone());
            }
        }
        marks.push(placed);
    }
    Ok(AutoPassResult {
        case_version: input.case_version,
        marks,
        two_films: chain.two_films,
        warnings,
    })
}

fn refused(message: &str) -> EngineError {
    EngineError::new(ErrorCode::EngineOutOfSync, message)
}
