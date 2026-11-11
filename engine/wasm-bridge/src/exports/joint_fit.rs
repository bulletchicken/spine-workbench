//! `fit_spine`: the same joint fit natively and through the browser bridge.

use crate::{crossing, faults};
use ::shapes::errors::EngineError;
use ::shapes::spine_fit::FitInput;
use wasm_bindgen::prelude::*;

/// Fits both films' marks as one spine, with uncertainty for every level.
#[wasm_bindgen]
pub fn fit_spine(input: JsValue) -> Result<JsValue, JsValue> {
    let input: FitInput = crossing::from_js(input)?;
    let fit =
        joint_fit::fit_spine(&input).map_err(|error| faults::thrown(EngineError::from(error)))?;
    crossing::to_js(&fit)
}
