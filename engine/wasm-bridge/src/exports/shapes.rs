//! `read_case_json` and `write_case_json`: the shapes crate's strict reader
//! and canonical writer of `case.json`, on their own.
//!
//! The app saves and opens whole case files through `case_file.rs`. These
//! two let a `case.json` cross on its own, so the contract test can push
//! every golden case file from text into a JavaScript object and back and
//! prove nothing changed on the way: no `null` turned into `undefined`, no
//! field gained or lost, the same bytes written out.

use crate::{crossing, faults};
use ::shapes::case_file_v1::CaseFileV1;
use ::shapes::reading;
use wasm_bindgen::prelude::*;

/// Reads `case.json` text strictly into a `CaseFileV1` object. A refused
/// file throws an `EngineError` whose `detail` is the JSON path of the
/// offending value.
#[wasm_bindgen]
pub fn read_case_json(text: &str) -> Result<JsValue, JsValue> {
    let case = reading::read_case_json(text).map_err(faults::thrown)?;
    crossing::to_js(&case)
}

/// Checks a `CaseFileV1` object as strictly as reading does, then writes
/// it in its one canonical form.
#[wasm_bindgen]
pub fn write_case_json(case: JsValue) -> Result<String, JsValue> {
    let case: CaseFileV1 = crossing::from_js(case)?;
    reading::check_case_file(&case).map_err(faults::thrown)?;
    Ok(reading::write_case_json(&case))
}
