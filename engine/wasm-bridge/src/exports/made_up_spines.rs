//! `make_up_spine`: a made-up case from a recipe, as the spine worker's
//! `make-up-spine` job hands it to the app.
//!
//! The four films cross as PNG bytes (each a fresh `Uint8Array`), so they
//! open through the same path as any dropped film, and the answer key as a
//! plain object. The same recipe gives exactly what `spine make-up` writes
//! natively.

use crate::{crossing, faults};
use ::shapes::errors::EngineError;
use ::shapes::made_up::MadeUpRecipe;
use wasm_bindgen::prelude::*;

/// Makes up a spine from a `MadeUpRecipe`: `{ films, answerKey }`. Throws
/// `made-up-unsatisfiable` when no spine could be built from the recipe.
#[wasm_bindgen]
pub fn make_up_spine(recipe: JsValue) -> Result<JsValue, JsValue> {
    let recipe: MadeUpRecipe = crossing::from_js(recipe)?;
    let spine = made_up_spines::make_up_spine(&recipe)
        .map_err(|error| faults::thrown(EngineError::from(error)))?;
    crossing::to_js(&spine)
}
