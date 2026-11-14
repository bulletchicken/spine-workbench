//! The lumbar side network, its picture and its fixed landmark channels.

pub mod decode;
pub mod intake;
pub mod network;
mod pass;
pub mod reading;
pub mod resize;
mod run;
pub mod table;
mod to_marks;
pub mod weights;
pub use pass::SidePass;
pub use run::find_on_side_film;

/// Whether the optional side packet was included in this build.
pub fn available() -> bool {
    weights::packet().is_some()
}

pub(crate) fn memory() -> shapes::errors::EngineError {
    shapes::errors::EngineError::new(
        shapes::errors::ErrorCode::EngineOutOfMemory,
        "not enough memory for the side finder",
    )
}

pub(crate) fn lost() -> shapes::errors::EngineError {
    shapes::errors::EngineError::new(
        shapes::errors::ErrorCode::EngineOutOfSync,
        "the side reading is out of order",
    )
}

pub(crate) fn reserve<T>(len: usize) -> Result<Vec<T>, shapes::errors::EngineError> {
    let mut values = Vec::new();
    values.try_reserve_exact(len).map_err(|_| memory())?;
    Ok(values)
}
