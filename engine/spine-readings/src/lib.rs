//! Measurements on one fitted spine. The nominal reading and uncertainty
//! over versions are distinct.

#![forbid(unsafe_code)]

pub mod bands;
pub mod curves;
pub mod evidence;
mod plates;
pub mod readings;
mod report;
mod sagittal;

pub use report::measure_spine;

use shapes::errors::{EngineError, ErrorCode};

/// A named refusal without any film contents.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ReadingsError {
    #[error("spine-readings-no-spine")]
    NoSpine,
    #[error("spine-readings-invalid-value: {field}: {reason}")]
    InvalidValue { field: String, reason: String },
    #[error("spine-readings-versions-out-of-range: {versions}")]
    VersionsOutOfRange { versions: u32 },
}

impl ReadingsError {
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::NoSpine => ErrorCode::PassNoSpineFound,
            Self::InvalidValue { .. } | Self::VersionsOutOfRange { .. } => {
                ErrorCode::CaseInvalidValue
            }
        }
    }
}

impl From<ReadingsError> for EngineError {
    fn from(error: ReadingsError) -> Self {
        Self::new(error.code(), error.to_string()).with_detail(error.to_string())
    }
}
