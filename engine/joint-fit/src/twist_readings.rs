//! Twists measured outside the fit: checked, matched to their levels and
//! pulled toward with their own spread.

use crate::FitError;
use crate::problem::Problem;
use crate::residuals::Row;
use crate::toward::{self, Spread, TWIST};
use shapes::spine_fit::TwistReading;

/// Each reading must be finite with a positive spread, name a fitted level,
/// and be the only reading for it.
pub fn attach(problem: &mut Problem, readings: &[TwistReading]) -> Result<(), FitError> {
    for reading in readings {
        if !(reading.twist_rad.is_finite() && reading.sd_rad.is_finite() && reading.sd_rad > 0.0) {
            return Err(FitError::InvalidTwistReadings);
        }
        let observation = problem
            .observations
            .iter_mut()
            .find(|observation| observation.level == reading.level)
            .ok_or(FitError::InvalidTwistReadings)?;
        if observation.twist_reading.replace(*reading).is_some() {
            return Err(FitError::InvalidTwistReadings);
        }
    }
    Ok(())
}

pub fn rows(problem: &Problem, numbers: &[f64], differentiate: bool) -> Vec<Row> {
    problem
        .observations
        .iter()
        .enumerate()
        .filter_map(|(i, observation)| {
            let reading = observation.twist_reading?;
            let target = Spread {
                mean: reading.twist_rad,
                sd: reading.sd_rad,
            };
            Some(toward::angle(numbers, i, TWIST, target, differentiate))
        })
        .collect()
}
