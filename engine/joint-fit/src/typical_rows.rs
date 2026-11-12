//! Rows toward the typical spine for each vertebra the side film does not
//! show: forward lean, depth over width at both endplates, side wedge, and
//! twist where no pedicle and no reading says otherwise.

use crate::parameters::COUNT;
use crate::problem::Problem;
use crate::residuals::Row;
use crate::toward::{self, LEAN, TWIST};

pub fn rows(problem: &Problem, numbers: &[f64], differentiate: bool) -> Vec<Row> {
    let Some(typical) = &problem.typical else {
        return Vec::new();
    };
    let mut rows = Vec::new();
    for (i, (observation, level)) in problem.observations.iter().zip(&typical.levels).enumerate() {
        if observation.side.is_some() {
            continue;
        }
        let base = i * COUNT;
        rows.push(toward::angle(
            numbers,
            i,
            LEAN,
            level.lean_rad,
            differentiate,
        ));
        for (depth, width) in [(base + 10, base + 9), (base + 12, base + 11)] {
            rows.push(toward::linear(
                numbers,
                &[(depth, 1.0), (width, -1.0)],
                level.log_depth_to_width,
                differentiate,
            ));
        }
        rows.push(toward::linear(
            numbers,
            &[(base + 8, 1.0)],
            level.side_wedge_rad,
            differentiate,
        ));
        if !observation.has_pedicle() && observation.twist_reading.is_none() {
            rows.push(toward::angle(
                numbers,
                i,
                TWIST,
                level.twist_rad,
                differentiate,
            ));
        }
    }
    rows
}
