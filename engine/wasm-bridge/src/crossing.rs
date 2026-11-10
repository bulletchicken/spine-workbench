//! How values cross into JavaScript and back: serde-wasm-bindgen with maps
//! as objects and `None` as `null`, set up once here.
//!
//! The shapes use only numbers that fit a JavaScript number (no `u64`),
//! string-keyed maps and internally tagged enums, so with these settings
//! the JavaScript object, the generated TypeScript type and `case.json`
//! are the same shape. Byte fields marked `serde_bytes` arrive as a fresh
//! `Uint8Array`.

use crate::faults;
use js_sys::{Array, ArrayBuffer, Object, Reflect};
use serde::Serialize;
use serde::de::DeserializeOwned;
use shapes::errors::{EngineError, ErrorCode};
use wasm_bindgen::{JsCast, JsValue};

/// A Rust value as a plain JavaScript value.
///
/// Every shape serialises; a failure here means a shape broke the rules
/// above, so it is reported as `engine-out-of-sync`.
pub fn to_js<T: Serialize + ?Sized>(value: &T) -> Result<JsValue, JsValue> {
    value.serialize(&serializer()).map_err(|error| {
        faults::thrown(
            EngineError::new(
                ErrorCode::EngineOutOfSync,
                "a value could not be handed to JavaScript",
            )
            .with_detail(error.to_string()),
        )
    })
}

/// A JavaScript value read as a Rust shape, strictly: wrong types, unknown
/// fields and missing fields are refused.
///
/// The TypeScript types make a wrong value a compile error, so one that
/// still arrives means the app and the engine were built from different
/// shapes: `engine-out-of-sync`, with serde's description or the path of
/// the offending field as the detail.
///
/// serde-wasm-bindgen reads only the fields a struct declares, so an
/// unknown field would pass unseen, and it reads a missing field as
/// `None`. So the value read is written back out and compared with what
/// arrived: every field must appear in both, `null` included, as in
/// `case.json`.
pub fn from_js<T: DeserializeOwned + Serialize>(value: JsValue) -> Result<T, JsValue> {
    let mismatch = |detail: String| {
        faults::thrown(
            EngineError::new(
                ErrorCode::EngineOutOfSync,
                "a value from JavaScript does not match the engine's shapes",
            )
            .with_detail(detail),
        )
    };
    let shaped: T = serde_wasm_bindgen::from_value(value.clone())
        .map_err(|error| mismatch(error.to_string()))?;
    let canonical = to_js(&shaped)?;
    match first_field_difference(&value, &canonical, "") {
        Some(difference) => Err(mismatch(difference)),
        None => Ok(shaped),
    }
}

/// The first field that one value has and the other lacks, walking both
/// together, as `"unknown field <path>"` or `"missing field <path>"`.
fn first_field_difference(arrived: &JsValue, canonical: &JsValue, path: &str) -> Option<String> {
    if Array::is_array(arrived) && Array::is_array(canonical) {
        let (arrived, canonical) = (Array::from(arrived), Array::from(canonical));
        return (0..arrived.length().min(canonical.length())).find_map(|index| {
            first_field_difference(
                &arrived.get(index),
                &canonical.get(index),
                &format!("{path}[{index}]"),
            )
        });
    }
    let is_record = |value: &JsValue| {
        value.is_object() && !Array::is_array(value) && !ArrayBuffer::is_view(value)
    };
    if !is_record(arrived) || !is_record(canonical) {
        return None;
    }
    let field = |key: &JsValue| {
        let name = key.as_string().unwrap_or_default();
        if path.is_empty() {
            name
        } else {
            format!("{path}.{name}")
        }
    };
    let get = |object: &JsValue, key: &JsValue| Reflect::get(object, key).unwrap_or_default();
    // Own properties only: every object inherits `constructor`,
    // `toString` and the like, which must not pass for fields.
    let has =
        |object: &JsValue, key: &JsValue| Object::has_own(object.unchecked_ref::<Object>(), key);
    let arrived_keys = Object::keys(arrived.unchecked_ref::<Object>());
    let canonical_keys = Object::keys(canonical.unchecked_ref::<Object>());
    if let Some(key) = arrived_keys.iter().find(|key| !has(canonical, key)) {
        return Some(format!("unknown field {}", field(&key)));
    }
    if let Some(key) = canonical_keys
        .iter()
        .find(|key| !has(arrived, key) || get(arrived, key).is_undefined())
    {
        return Some(format!("missing field {}", field(&key)));
    }
    canonical_keys.iter().find_map(|key| {
        first_field_difference(&get(arrived, &key), &get(canonical, &key), &field(&key))
    })
}

/// The one serializer configuration.
pub fn serializer() -> serde_wasm_bindgen::Serializer {
    serde_wasm_bindgen::Serializer::new()
        .serialize_maps_as_objects(true)
        .serialize_missing_as_null(true)
}
