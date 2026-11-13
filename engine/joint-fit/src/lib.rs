//! Fit all marked vertebrae together through the shared parallel forward model.
//!
//! [`fit_spine`] accepts two films in original pixels and returns positive
//! sizes, named angles, patient-frame centres and propagated uncertainty.
//! Missing geometry is refused rather than guessed. Measurement weights
//! and soft priors are fixed; even exact marks can move under those priors.
//!
//! [`fit_from_films`] accepts one front film or a front and side pair, each
//! with its own mark spread. Without a side film it takes what one film
//! cannot show from the built-in typical spine, with that spine's spread.

#![forbid(unsafe_code)]

#[cfg(test)]
#[expect(
    dead_code,
    reason = "the integration tests' answer keys; the unit tests use some"
)]
#[path = "../tests/answer_keys/mod.rs"]
mod answer_keys;
mod corner_tilt;
mod curve_twist;
mod estimated_names;
mod film_placement;
mod from_films;
mod front_curves;
mod gauge;
mod levenberg_marquardt;
mod mark_spread;
mod parameters;
mod patient_origin;
mod priors;
mod problem;
mod residuals;
mod scale;
mod stacking;
mod start_values;
mod toward;
mod twist_readings;
mod typical_rows;
mod typical_spine;
mod uncertainty;

pub use from_films::fit_from_films;
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::FilmKind;
use shapes::spine_fit::{
    FitAngles, FitInput, FitLengths, FitRmsMm, FitStopReason, FittedVertebra, InferredFit, SpineFit,
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
    #[error("joint-fit-too-few-vertebrae: a spine needs at least two")]
    TooFewVertebrae,
    #[error("joint-fit-invalid-mark-spread: {0:?}")]
    InvalidMarkSpread(FilmKind),
    #[error("joint-fit-invalid-twist-readings")]
    InvalidTwistReadings,
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
    fit(&problem::Problem::new(input)?)
}

/// Solve a validated problem. Lengths are said to be measured; a caller
/// that gave a film a provisional scale says otherwise.
fn fit(problem: &problem::Problem) -> Result<SpineFit, FitError> {
    report(
        problem,
        &levenberg_marquardt::solve(problem, start_values::start(problem))?,
    )
}

/// The solved problem as the shared result, with each film's placement
/// and each front-marked vertebra's tilt as its corners show it.
fn report(
    problem: &problem::Problem,
    solution: &levenberg_marquardt::Solution,
) -> Result<SpineFit, FitError> {
    let numbers = solution.numbers.as_slice();
    let covariance = uncertainty::covariance(solution, problem.rescale_covariance)?;
    let origin = patient_origin::index(problem);
    let translation = patient_origin::underside(numbers, origin);
    let global = problem.observations.len() * parameters::COUNT;
    let unit = if problem.side_is_length_anchor {
        libm::exp(-numbers[global])
    } else {
        1.0
    };
    let side = problem.side_camera(numbers[global], numbers[global + 1]);
    let mut sum_squared = 0.0;
    let mut count = 0;
    let mut vertebrae = Vec::with_capacity(problem.observations.len());
    for (i, observation) in problem.observations.iter().enumerate() {
        let local = &numbers[i * parameters::COUNT..(i + 1) * parameters::COUNT];
        let [side_tilt_rad, forward_lean_rad, twist_rad] = parameters::angles(local);
        let marks = residuals::marks(problem, observation, local, side.as_ref())
            .ok_or(FitError::NumericalFailure)?;
        sum_squared += marks.squared_mm.iter().sum::<f64>();
        count += marks.counts.iter().sum::<usize>();
        let rms = |view| {
            (marks.counts[view] > 0)
                .then(|| libm::sqrt(marks.squared_mm[view] / marks.counts[view] as f64))
        };
        let mut v = parameters::vertebra(local);
        v.centre -= translation;
        let mut sd = uncertainty::standard_deviations(numbers, &covariance, i, origin);
        if problem.side_is_length_anchor {
            // The measured Side fixes the common gauge. Its fitted ratio
            // is correlated with the provisional centres, so propagate
            // that quotient before converting to the measured units.
            let origin_jacobian = patient_origin::jacobian(numbers, origin);
            for axis in 0..3 {
                let mut derivative = nalgebra::DVector::zeros(numbers.len());
                for j in 0..parameters::COUNT {
                    derivative[origin * parameters::COUNT + j] = -origin_jacobian[(axis, j)];
                }
                derivative[i * parameters::COUNT + axis] += 1.0;
                derivative[global] -= v.centre[axis];
                sd.centre_mm[axis] =
                    libm::sqrt(derivative.dot(&(&covariance * &derivative)).max(0.0));
            }
        }
        mark_spread::add_shared(&mut sd, observation);
        v.centre *= unit;
        v.height *= unit;
        v.top_half_width *= unit;
        v.top_half_depth *= unit;
        v.bottom_half_width *= unit;
        v.bottom_half_depth *= unit;
        sd.centre_mm = sd.centre_mm.map(|s| s * unit);
        vertebrae.push(FittedVertebra {
            level: observation.level,
            pose: v.pose(observation.level),
            angles: FitAngles {
                side_tilt_rad,
                forward_lean_rad,
                twist_rad,
            },
            sd,
            inferred: InferredFit {
                depth: observation.side.is_none(),
                width: observation.front.is_none(),
                twist: !observation.has_pedicle() && observation.twist_reading.is_none(),
            },
            rms_mm: FitRmsMm {
                front: rms(0).map(|s| s * unit),
                side: rms(1).map(|s| s * unit),
            },
            corner_tilt_rad: corner_tilt::of(problem, observation),
        });
    }
    Ok(SpineFit {
        vertebrae,
        side_scale: libm::exp(numbers[global]) * unit,
        side_offset_mm: numbers[global + 1] * unit,
        origin_level: problem.observations[origin].level,
        iterations: solution.iterations,
        converged: solution.stop_reason != FitStopReason::IterationLimit,
        stop_reason: solution.stop_reason,
        rms_mm: libm::sqrt(sum_squared / count as f64) * unit,
        lengths: FitLengths::Measured,
        typical_curve_deg: None,
        front_film: film_placement::in_units(&problem.front, &translation, unit),
        side_film: side
            .as_ref()
            .map(|camera| film_placement::in_units(camera, &translation, unit)),
    })
}
