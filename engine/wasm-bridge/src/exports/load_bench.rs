//! `run_load_bench` and `compare_layouts`: the same load bench natively
//! and through the browser bridge.

use crate::{crossing, faults};
use ::shapes::errors::EngineError;
use ::shapes::load_bench::{BenchCompareInput, BenchInput};
use wasm_bindgen::prelude::*;

/// Runs every placement on one slice of versions, through every stop.
#[wasm_bindgen]
pub fn run_load_bench(input: JsValue) -> Result<JsValue, JsValue> {
    let input: BenchInput = crossing::from_js(input)?;
    let batch = load_bench::run_load_bench(&input)
        .map_err(|error| faults::thrown(EngineError::from(error)))?;
    crossing::to_js(&batch)
}

/// Compares every placement with the first over all the batches.
#[wasm_bindgen]
pub fn compare_layouts(input: JsValue) -> Result<JsValue, JsValue> {
    let input: BenchCompareInput = crossing::from_js(input)?;
    let result = load_bench::compare_layouts(&input)
        .map_err(|error| faults::thrown(EngineError::from(error)))?;
    crossing::to_js(&result)
}
