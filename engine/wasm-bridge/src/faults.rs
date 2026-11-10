//! Engine errors as thrown JavaScript values, and the last panic message.
//!
//! An expected failure leaves an export as `Err(thrown(error))`, which
//! wasm-bindgen throws as a plain `EngineError` object. A panic cannot be
//! caught inside WebAssembly: it ends in a trap that the worker sees as a
//! `WebAssembly.RuntimeError`. So the panic hook stores the message first,
//! and the worker reads it with `last_panic()` before it reports the crash
//! and closes.

use serde::Serialize;
use shapes::errors::EngineError;
use std::cell::RefCell;
use std::sync::Once;
use wasm_bindgen::JsValue;

thread_local! {
    static LAST_PANIC: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// An engine error as the value an export throws.
pub fn thrown(error: EngineError) -> JsValue {
    error
        .serialize(&crate::crossing::serializer())
        .unwrap_or_else(|_| JsValue::from_str(&error.to_string()))
}

/// Installs the panic hook once. Later calls do nothing.
///
/// The hook stores the panic's message and location, then hands over to
/// the hook that was there before, so native test output is unchanged.
pub fn capture_panics() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            LAST_PANIC.with(|last| *last.borrow_mut() = Some(info.to_string()));
            previous(info);
        }));
    });
}

/// The message of the most recent panic on this thread, if there was one.
pub fn last_panic() -> Option<String> {
    LAST_PANIC.with(|last| last.borrow().clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panic_leaves_its_message_behind() {
        capture_panics();
        let outcome = std::panic::catch_unwind(|| panic!("the strip has no rows"));
        assert!(outcome.is_err());
        let message = last_panic().expect("the hook stored the panic");
        assert!(message.contains("the strip has no rows"), "{message}");
        assert!(message.contains("faults.rs"), "{message}");
    }

    #[test]
    fn installing_twice_keeps_one_hook() {
        capture_panics();
        capture_panics();
        let _ = std::panic::catch_unwind(|| panic!("first"));
        let _ = std::panic::catch_unwind(|| panic!("second"));
        let message = last_panic().expect("the hook stored the panic");
        assert!(message.contains("second"), "{message}");
    }
}
