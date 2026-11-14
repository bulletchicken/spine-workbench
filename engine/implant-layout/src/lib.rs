//! Placement proposals on a fitted spine. Every published level rule is
//! read at once on the same spine and counted over versions of it (the
//! same spine with one draw of what the films cannot fix); then a first
//! placement: the levels, an anchor at each pedicle with its entry point,
//! direction, diameter and length from published tables, and two rods;
//! what needs a look; and the placements one step away. It estimates; a
//! surgeon decides.
//!
//! | Module | Holds |
//! |---|---|
//! | [`evidence`] | the published tables, each value with its source key, read strictly |
//! | [`readings`] | placement classification of shared `spine-readings` versions |
//! | [`curves`] | shared curve geometry adapted to structural tests and curve type |
//! | [`level_rules`] | each level rule and check: a level or why it cannot be read, its inputs, its flip |
//! | [`pairs`] | level pairs from the votes, the stop-short odds, the first pair |
//! | [`pedicles`] | pedicle widths from the population tables, per version |
//! | [`screws`] | entry point, direction, diameter and length, and what needs a look |
//! | [`patterns`] | which pedicles get screws or hooks |
//! | [`rods`] | material, diameter, target shape, overbend and centre line |
//! | [`alternatives`] | the placements one step away from the first |
//! | [`proposal`] | the whole answer put together |
//!
//! Entry point: [`propose_layouts`].

#![forbid(unsafe_code)]

pub mod alternatives;
pub mod curves;
pub mod evidence;
pub mod level_rules;
pub mod pairs;
pub mod patterns;
pub mod pedicles;
pub mod proposal;
pub mod readings;
pub mod rods;
pub mod screws;

use shapes::errors::{EngineError, ErrorCode};
use shapes::implant_layout::{LayoutInput, LayoutProposals};

/// Why no proposals came of an input. Each crosses to the app as an
/// existing error code with this refusal's name in its detail.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum LayoutError {
    /// The named entry point is not built yet.
    #[error("implant-layout-not-built: {0}")]
    NotBuilt(&'static str),
    /// The input is well formed but nothing can be proposed from it; the
    /// reason is a short code.
    #[error("implant-layout-refused: {0}")]
    Refused(String),
    /// A value in the input is not valid: the field's path and why.
    #[error("implant-layout-invalid-value: {field}: {reason}")]
    InvalidValue { field: String, reason: String },
    /// The fit holds no vertebrae to work on.
    #[error("implant-layout-no-spine")]
    NoSpine,
}

impl From<LayoutError> for EngineError {
    fn from(error: LayoutError) -> Self {
        let code = match error {
            LayoutError::NoSpine => ErrorCode::PassNoSpineFound,
            LayoutError::InvalidValue { .. } => ErrorCode::CaseInvalidValue,
            LayoutError::NotBuilt(_) | LayoutError::Refused(_) => ErrorCode::ToolRefused,
        };
        Self::new(code, error.to_string()).with_detail(error.to_string())
    }
}

impl From<spine_readings::ReadingsError> for LayoutError {
    fn from(error: spine_readings::ReadingsError) -> Self {
        match error {
            spine_readings::ReadingsError::NoSpine => Self::NoSpine,
            spine_readings::ReadingsError::InvalidValue { field, reason } => {
                Self::InvalidValue { field, reason }
            }
            spine_readings::ReadingsError::VersionsOutOfRange { versions } => Self::InvalidValue {
                field: "versions".into(),
                reason: format!("out-of-range:{versions}"),
            },
        }
    }
}

/// The readings, every rule's vote, the level pairs, the first placement
/// and the placements one step away from it.
pub fn propose_layouts(input: &LayoutInput) -> Result<LayoutProposals, LayoutError> {
    proposal::propose(input)
}
