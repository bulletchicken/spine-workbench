//! Measurements on a fitted spine through the browser bridge.

use crate::{crossing, faults};
use ::shapes::errors::EngineError;
use ::shapes::spine_readings::ReadingsInput;
use wasm_bindgen::prelude::*;

/// Measures the spine as fitted, with bands over the requested versions.
#[wasm_bindgen]
pub fn measure_spine(input: JsValue) -> Result<JsValue, JsValue> {
    let input: ReadingsInput = crossing::from_js(input)?;
    let readings = spine_readings::measure_spine(&input)
        .map_err(|error| faults::thrown(EngineError::from(error)))?;
    crossing::to_js(&readings)
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_measurement_door_is_exported() {
        let _: fn(wasm_bindgen::JsValue) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue> =
            super::measure_spine;
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod browser_tests {
    use js_sys::{Object, Reflect};
    use wasm_bindgen::JsValue;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn a_malformed_reading_request_is_refused_at_the_door() {
        let input = Object::new();
        Reflect::set(&input, &JsValue::from_str("extra"), &JsValue::TRUE).expect("a field");
        let error = super::measure_spine(input.into()).expect_err("no readings input");
        assert_eq!(
            Reflect::get(&error, &JsValue::from_str("code")).expect("error code"),
            JsValue::from_str("engine-out-of-sync")
        );
    }
}
