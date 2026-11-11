//! Spines and films with known answers, for tests, scoring and "Try a
//! made-up spine" in the app: a recipe and a seed give four films and an
//! answer key, the same natively and in the browser.
//!
//! The films are painted with their own body shape and a second curve
//! generator, so tests built on them cannot simply agree with the solver.
//! Accuracy on made-up films is never clinical accuracy.
//!
//! | Module | Holds |
//! |---|---|
//! | [`recipes`] | the named recipes and the ranges drawn from |
//! | [`seeded`] | the seeded random streams every draw comes from |
//! | [`curves`] | two independent curve generators |
//! | [`body_sizes`] | vertebra, disc and pedicle sizes |
//! | [`stack_from_l5`] | stacking vertebrae upward from L5 |
//! | [`patient`] | a recipe's spine standing and bent, drawn until it stacks |
//! | [`film_layout`] | each film's camera, size and field |
//! | [`film_painter`] | camera, spine, surroundings and detector |
//! | [`film_png`] | painted films as the PNGs a case stores |
//! | [`click_noise`] | noise on true marks, standing in for clicks |
//! | [`answer_key`] | the true marks, levels, geometry, landmarks and angles |
//!
//! Entry points: [`make_up_spine`] gives what crosses to the app (four PNGs
//! and the answer key); [`make_up_case`] keeps the painted films as images,
//! with where the lead letter, ruler and ball were put, for the engine's
//! own tests; [`make_up_answer_key`] gives the answer key alone, without
//! painting.

#![forbid(unsafe_code)]

pub mod answer_key;
pub mod body_sizes;
pub mod click_noise;
pub mod curves;
pub mod film_layout;
pub mod film_painter;
pub mod film_png;
pub mod patient;
pub mod recipes;
pub mod seeded;
pub mod stack_from_l5;

use film_layout::FilmLayout;
use film_painter::PaintedFilm;
use patient::{Patient, make_up_patient};
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::FilmPng;
use shapes::made_up::{AnswerKey, MadeUpRecipe, MadeUpSpine};

/// Why no made-up spine came of a recipe.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum MadeUpError {
    /// Every attempt at the recipe failed to stack.
    #[error("no spine could be built from the recipe in {attempts} attempts: {last}")]
    Unsatisfiable { attempts: u32, last: String },
}

impl From<MadeUpError> for EngineError {
    fn from(error: MadeUpError) -> Self {
        let MadeUpError::Unsatisfiable { last, .. } = &error;
        EngineError::new(ErrorCode::MadeUpUnsatisfiable, error.to_string())
            .with_detail(last.clone())
    }
}

/// A made-up case with its films still as images.
#[derive(Clone, Debug, PartialEq)]
pub struct MadeUpCase {
    pub patient: Patient,
    /// Front, side, bend left, bend right.
    pub layouts: Vec<FilmLayout>,
    pub films: Vec<PaintedFilm>,
    pub answer_key: AnswerKey,
}

fn patient_and_layouts(recipe: &MadeUpRecipe) -> Result<(Patient, Vec<FilmLayout>), MadeUpError> {
    let patient = make_up_patient(recipe.name, recipe.seed).map_err(|failure| {
        MadeUpError::Unsatisfiable {
            attempts: failure.attempts,
            last: format!("{:?}", failure.last),
        }
    })?;
    let layouts = recipes::film_recipes(recipe, u64::from(recipe.seed))
        .iter()
        .map(|film| FilmLayout::new(film, answer_key::spine_for(&patient, film.kind)))
        .collect();
    Ok((patient, layouts))
}

/// The answer key of a made-up case, without painting its films.
pub fn make_up_answer_key(recipe: &MadeUpRecipe) -> Result<AnswerKey, MadeUpError> {
    let (patient, layouts) = patient_and_layouts(recipe)?;
    Ok(answer_key::answer_key(recipe, &patient, &layouts))
}

/// A made-up case: the patient, the four painted films and the answer key.
pub fn make_up_case(recipe: &MadeUpRecipe) -> Result<MadeUpCase, MadeUpError> {
    let (patient, layouts) = patient_and_layouts(recipe)?;
    let seed = u64::from(recipe.seed);
    let films = recipes::film_recipes(recipe, seed)
        .iter()
        .zip(&layouts)
        .map(|(film, layout)| {
            film_painter::paint(
                film,
                layout,
                &patient,
                answer_key::spine_for(&patient, film.kind),
                seed,
            )
        })
        .collect();
    let answer_key = answer_key::answer_key(recipe, &patient, &layouts);
    Ok(MadeUpCase {
        patient,
        layouts,
        films,
        answer_key,
    })
}

/// A made-up case as it crosses to the app: each film's PNG and the
/// answer key.
pub fn make_up_spine(recipe: &MadeUpRecipe) -> Result<MadeUpSpine, MadeUpError> {
    let case = make_up_case(recipe)?;
    Ok(MadeUpSpine {
        films: case
            .films
            .iter()
            .map(|film| FilmPng {
                kind: film.kind,
                png: film_png::encode(film),
            })
            .collect(),
        answer_key: case.answer_key,
    })
}
