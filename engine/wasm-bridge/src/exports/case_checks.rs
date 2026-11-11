//! `check_case`.

use crate::crossing;
use shapes::case_file_v1::CaseFileV1;
use wasm_bindgen::prelude::*;

/// Reports the case's open checks without preventing an unfinished case
/// from being saved.
#[wasm_bindgen]
pub fn check_case(case: JsValue) -> Result<JsValue, JsValue> {
    let case: CaseFileV1 = crossing::from_js(case)?;
    crossing::to_js(&case_checks::check_case(&case))
}
