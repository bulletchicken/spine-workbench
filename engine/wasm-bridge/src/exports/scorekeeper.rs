//! `compare_with_answer_key`.

use crate::{crossing, faults};
use shapes::made_up::AnswerKey;
use shapes::vertebra::FilmMarks;
use wasm_bindgen::prelude::*;

/// Compares placed marks with visible corners in the made-up answer key.
/// Malformed comparison values fail with `case-invalid-value`.
#[wasm_bindgen]
pub fn compare_with_answer_key(marks: JsValue, key: JsValue) -> Result<JsValue, JsValue> {
    let marks: Vec<FilmMarks> = crossing::from_js(marks)?;
    let key: AnswerKey = crossing::from_js(key)?;
    let comparison = scorekeeper::report::answer_key::compare_with_answer_key(&marks, &key)
        .map_err(faults::thrown)?;
    crossing::to_js(&comparison)
}
