//! `fit_spine` and `fit_from_films`: the same joint fit natively and
//! through the browser bridge.

use crate::{crossing, faults};
use ::shapes::errors::EngineError;
use ::shapes::spine_fit::{FitFromFilms, FitInput};
use wasm_bindgen::prelude::*;

/// Fits both films' marks as one spine, with uncertainty for every level.
#[wasm_bindgen]
pub fn fit_spine(input: JsValue) -> Result<JsValue, JsValue> {
    let input: FitInput = crossing::from_js(input)?;
    let fit =
        joint_fit::fit_spine(&input).map_err(|error| faults::thrown(EngineError::from(error)))?;
    crossing::to_js(&fit)
}

/// Fits one front film, or a front and side pair, as one spine, with the
/// typical spine for what the films cannot show when asked for.
#[wasm_bindgen]
pub fn fit_from_films(input: JsValue) -> Result<JsValue, JsValue> {
    let input: FitFromFilms = crossing::from_js(input)?;
    let fit = joint_fit::fit_from_films(&input)
        .map_err(|error| faults::thrown(EngineError::from(error)))?;
    crossing::to_js(&fit)
}
