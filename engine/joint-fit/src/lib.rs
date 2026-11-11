//! Fit all marked vertebrae together through the shared parallel forward model.
//!
//! [`fit_spine`] accepts two films in original pixels and returns positive
//! sizes, named angles, patient-frame centres and propagated uncertainty.
//! Missing geometry is refused rather than guessed. Measurement weights
//! and soft priors are fixed; even exact marks can move under those priors.

#![forbid(unsafe_code)]

mod levenberg_marquardt;
mod parameters;
mod patient_origin;
mod priors;
mod problem;
mod residuals;
mod start_values;
mod uncertainty;

use shapes::errors::{EngineError, ErrorCode};
use shapes::film::FilmKind;
use shapes::spine_fit::{
    FitAngles, FitInput, FitRmsMm, FitStopReason, FittedVertebra, InferredFit, SpineFit,
};

/// A named refusal without any film contents or identifying metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FitError {
    #[error("joint-fit-missing-scale: {0:?}")]
    MissingScale(FilmKind),
    #[error("joint-fit-missing-facing: {0:?}")]
    MissingFacing(FilmKind),
    #[error("joint-fit-invalid-scale: {0:?}")]
    InvalidScale(FilmKind),
    #[error("joint-fit-invalid-facing: {0:?}")]
    InvalidFacing(FilmKind),
    #[error("joint-fit-invalid-size: {0:?}")]
    InvalidSize(FilmKind),
    #[error("joint-fit-invalid-marks: {0:?}")]
    InvalidMarks(FilmKind),
    #[error("joint-fit-no-marks")]
    NoMarks,
    #[error(
        "joint-fit-underconstrained: the marks and priors do not determine a finite covariance"
    )]
    Underconstrained,
    #[error("joint-fit-numerical-failure")]
    NumericalFailure,
}

impl From<FitError> for EngineError {
    fn from(error: FitError) -> Self {
        let code = match error {
            FitError::NoMarks => ErrorCode::PassNoSpineFound,
            FitError::Underconstrained | FitError::NumericalFailure => ErrorCode::ToolRefused,
            _ => ErrorCode::CaseInvalidValue,
        };
        Self::new(code, error.to_string()).with_detail(error.to_string())
    }
}

/// Fit both views at once. Output is ordered by level, and `origin_level`
/// names L5 or, when absent, the lowest fitted vertebra. A missing view's
/// RMS is null. RMS values are distances per mark, without prior weights.
pub fn fit_spine(input: &FitInput) -> Result<SpineFit, FitError> {
    let problem = problem::Problem::new(input)?;
    let solution = levenberg_marquardt::solve(&problem, start_values::start(&problem))?;
    let numbers = solution.numbers.as_slice();
    let covariance = uncertainty::covariance(&solution)?;
    let origin = patient_origin::index(&problem);
    let translation = patient_origin::underside(numbers, origin);
    let global = problem.observations.len() * parameters::COUNT;
    let side = problem.side_camera(numbers[global], numbers[global + 1]);
    let mut sum_squared = 0.0;
    let mut count = 0;
    let mut vertebrae = Vec::with_capacity(problem.observations.len());
    for (i, observation) in problem.observations.iter().enumerate() {
        let local = &numbers[i * parameters::COUNT..(i + 1) * parameters::COUNT];
        let [side_tilt_rad, forward_lean_rad, twist_rad] = parameters::angles(local);
        let marks = residuals::marks(&problem, observation, local, &side)
            .ok_or(FitError::NumericalFailure)?;
        sum_squared += marks.squared_mm.iter().sum::<f64>();
        count += marks.counts.iter().sum::<usize>();
        let rms = |view| {
            (marks.counts[view] > 0)
                .then(|| libm::sqrt(marks.squared_mm[view] / marks.counts[view] as f64))
        };
        let mut v = parameters::vertebra(local);
        v.centre -= translation;
        vertebrae.push(FittedVertebra {
            level: observation.level,
            pose: v.pose(observation.level),
            angles: FitAngles {
                side_tilt_rad,
                forward_lean_rad,
                twist_rad,
            },
            sd: uncertainty::standard_deviations(numbers, &covariance, i, origin),
            inferred: InferredFit {
                depth: observation.side.is_none(),
                width: observation.front.is_none(),
                twist: !observation.has_pedicle(),
            },
            rms_mm: FitRmsMm {
                front: rms(0),
                side: rms(1),
            },
        });
    }
    Ok(SpineFit {
        vertebrae,
        side_scale: libm::exp(numbers[global]),
        side_offset_mm: numbers[global + 1],
        origin_level: problem.observations[origin].level,
        iterations: solution.iterations,
        converged: solution.stop_reason != FitStopReason::IterationLimit,
        stop_reason: solution.stop_reason,
        rms_mm: libm::sqrt(sum_squared / count as f64),
    })
}
