//! Each vertebra sits on the one below: going up from one centre to the
//! next moves forward by the rise times the tangent of the pair's mean
//! forward lean, give or take the typical spine's share of the rise. Used
//! wherever the side film does not show both of the pair.

use crate::parameters::{COUNT, angle_jacobian, angles};
use crate::problem::Problem;
use crate::residuals::Row;
use crate::toward::LEAN;

pub fn rows(problem: &Problem, numbers: &[f64], differentiate: bool) -> Vec<Row> {
    let Some(typical) = &problem.typical else {
        return Vec::new();
    };
    let share = typical.stacking_spread;
    let mut rows = Vec::new();
    for (i, pair) in problem.observations.windows(2).enumerate() {
        if pair[0].side.is_some() && pair[1].side.is_some() {
            continue;
        }
        let (upper, lower) = (i * COUNT, (i + 1) * COUNT);
        let upper_numbers = &numbers[upper..upper + COUNT];
        let lower_numbers = &numbers[lower..lower + COUNT];
        let rise = numbers[upper + 2] - numbers[lower + 2];
        let ahead = numbers[upper] - numbers[lower];
        let lean = 0.5 * (angles(upper_numbers)[LEAN] + angles(lower_numbers)[LEAN]);
        let tangent = libm::tan(lean);
        // (ahead - rise tan(lean)) / (share rise), written so that each
        // derivative below reads off directly.
        let value = ahead / (share * rise) - tangent / share;
        let mut derivatives = Vec::new();
        if differentiate {
            let d_ahead = 1.0 / (share * rise);
            let d_rise = -ahead / (share * rise * rise);
            let d_lean = -0.5 * (1.0 + tangent * tangent) / share;
            derivatives.extend([
                (upper, d_ahead),
                (lower, -d_ahead),
                (upper + 2, d_rise),
                (lower + 2, -d_rise),
            ]);
            for (start, local) in [(upper, upper_numbers), (lower, lower_numbers)] {
                let jacobian = angle_jacobian(local);
                for axis in 0..3 {
                    derivatives.push((start + 3 + axis, d_lean * jacobian[(LEAN, axis)]));
                }
            }
        }
        rows.push(Row { value, derivatives });
    }
    rows
}
