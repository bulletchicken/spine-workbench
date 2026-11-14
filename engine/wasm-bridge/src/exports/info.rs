//! `engine_info` and `last_panic`.

use crate::{crossing, faults};
use shapes::engine_info::EngineInfo;
use wasm_bindgen::prelude::*;

/// The engine's version, its shapes fingerprint and the case format it
/// reads and writes, as an `EngineInfo` object. Every worker reports it at
/// boot; a fingerprint that differs from the app's means a stale engine
/// build.
#[wasm_bindgen]
pub fn engine_info() -> Result<JsValue, JsValue> {
    let mut info = EngineInfo::current(env!("CARGO_PKG_VERSION"));
    info.trained_side_finder = trained_finder::side::available();
    crossing::to_js(&info)
}

/// The message of the panic that ended the last call, or `null`. A worker
/// calls it after catching a `WebAssembly.RuntimeError`.
#[wasm_bindgen]
pub fn last_panic() -> Option<String> {
    faults::last_panic()
}
