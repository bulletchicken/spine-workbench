//! Placements put under load on versions of a fitted spine. Films cannot
//! show how stiff a spine is, so every placement runs on the same versions
//! (the versions `implant-layout` counts over, plus a draw of stiffness,
//! bone strength, rod yield, weight and steps a day) and placements are
//! compared by paired counts and ranges, never by one number. The rods are
//! brought in rigid (order 0), then allowed to flex (order 1); daily loads
//! are linear cases on the locked construct; a year of walking compares
//! each rod's swing with the level rods survived on the test bench. It
//! estimates and compares; a surgeon decides.
//!
//! | Module | Holds |
//! |---|---|
//! | [`evidence`] | the published tables, each value with its source key, read strictly |
//! | [`chain`] | vertebrae on the sacrum, a joint at each disc, the standing shape as rest |
//! | [`joint_springs`] | each joint's six stiffnesses, rib cage, preload and release |
//! | [`chain_solver`] | the block-tridiagonal-with-border Cholesky solve |
//! | [`rigid_rod`] | order 0: the spine brought to rigid rods |
//! | [`rod_beams`] | linear beams for the rods between heads |
//! | [`springback`] | order 1: the rods allowed to flex |
//! | [`rod_statics`] | each rod's internal moments and surface stress |
//! | [`daily_loads`] | standing, walking and bending forward on the locked construct |
//! | [`anchor_demand`] | anchor forces against published strength |
//! | [`junctions`] | the joints next to the construct against the spine before |
//! | [`fatigue`] | a year of walking against the test-bench level |
//! | [`versions`] | what each version draws for the unknowns |
//! | [`batch`] | every placement on one slice of versions |
//! | [`compare`] | ranges, tags, shares, ghosts and paired counts |
//!
//! Entry points: [`run_load_bench`] and [`compare_layouts`].

#![forbid(unsafe_code)]

pub mod anchor_demand;
pub mod batch;
pub mod chain;
pub mod chain_solver;
pub mod compare;
pub mod daily_loads;
pub mod evidence;
pub mod fatigue;
pub mod joint_springs;
pub mod junctions;
pub mod rigid_rod;
pub mod rod_beams;
pub mod rod_statics;
pub mod springback;
pub mod versions;

use shapes::errors::{EngineError, ErrorCode};
use shapes::load_bench::{BenchBatch, BenchCompareInput, BenchInput, BenchResult};

/// Why the bench gave no answer. Each crosses to the app as an existing
/// error code with this refusal's name in its detail.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum BenchError {
    /// The named entry point is not built yet.
    #[error("load-bench-not-built: {0}")]
    NotBuilt(&'static str),
    /// The input is well formed but cannot be run; the reason is a short
    /// code.
    #[error("load-bench-refused: {0}")]
    Refused(String),
    /// A value in the input is not valid: the field's path and why.
    #[error("load-bench-invalid-value: {field}: {reason}")]
    InvalidValue { field: String, reason: String },
    /// The fit holds no vertebrae to work on.
    #[error("load-bench-no-spine")]
    NoSpine,
}

impl From<BenchError> for EngineError {
    fn from(error: BenchError) -> Self {
        let code = match error {
            BenchError::NoSpine => ErrorCode::PassNoSpineFound,
            BenchError::InvalidValue { .. } => ErrorCode::CaseInvalidValue,
            BenchError::NotBuilt(_) | BenchError::Refused(_) => ErrorCode::ToolRefused,
        };
        Self::new(code, error.to_string()).with_detail(error.to_string())
    }
}

/// Every placement on one slice of versions, through every stop.
pub fn run_load_bench(input: &BenchInput) -> Result<BenchBatch, BenchError> {
    batch::run(input)
}

/// Every batch of a comparison turned into ranges, tags, shares and paired
/// counts against the first placement.
pub fn compare_layouts(input: &BenchCompareInput) -> Result<BenchResult, BenchError> {
    compare::compare(input)
}
