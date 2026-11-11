//! Chain smoothness, within-body size sanity, wedges and relative film scale.

use crate::parameters::{COUNT, SIZES, angle_difference, angle_jacobian, angles};
use crate::problem::Problem;
use crate::residuals::Row;

// These broad standard deviations let observed marks decide the pose.
// Priors chiefly resolve directions that missing views or pedicles leave
// unobserved; even a weak prior still decides when there is no data.
const CENTRE_SMOOTHNESS_SD_MM: f64 = 50.0;
const ANGLE_SMOOTHNESS_SD_RAD: f64 = 50.0 * core::f64::consts::PI / 180.0;
const LOG_SIZE_SMOOTHNESS_SD: f64 = 1.0;
const TOP_BOTTOM_LOG_SIZE_SD: f64 = 1.5;
const WEDGE_SD_RAD: f64 = 100.0 * core::f64::consts::PI / 180.0;
// Relative film calibration remains anchored to its stated scale.
const SIDE_SCALE_SD: f64 = 0.03;

pub fn rows(problem: &Problem, numbers: &[f64], differentiate: bool) -> Vec<Row> {
    let n = problem.observations.len();
    let mut rows = Vec::new();
    let angle_values: Vec<_> = (0..n)
        .map(|i| angles(&numbers[i * COUNT..(i + 1) * COUNT]))
        .collect();
    let angle_derivatives: Vec<_> = if differentiate {
        (0..n)
            .map(|i| angle_jacobian(&numbers[i * COUNT..(i + 1) * COUNT]))
            .collect()
    } else {
        Vec::new()
    };
    let mut linear = |entries: &[(usize, f64)], sd: f64| {
        rows.push(Row {
            value: entries
                .iter()
                .map(|&(i, coefficient)| numbers[i] * coefficient / sd)
                .sum(),
            derivatives: if differentiate {
                entries
                    .iter()
                    .map(|&(i, coefficient)| (i, coefficient / sd))
                    .collect()
            } else {
                Vec::new()
            },
        });
    };
    for i in 0..n {
        let base = i * COUNT;
        linear(&[(base + 7, 1.0)], WEDGE_SD_RAD);
        linear(&[(base + 8, 1.0)], WEDGE_SD_RAD);
        linear(
            &[(base + 9, 1.0), (base + 11, -1.0)],
            TOP_BOTTOM_LOG_SIZE_SD,
        );
        linear(
            &[(base + 10, 1.0), (base + 12, -1.0)],
            TOP_BOTTOM_LOG_SIZE_SD,
        );
    }
    for i in 1..n.saturating_sub(1) {
        for field in (0..3).chain(SIZES) {
            let sd = if field < 3 {
                CENTRE_SMOOTHNESS_SD_MM
            } else {
                LOG_SIZE_SMOOTHNESS_SD
            };
            linear(
                &[
                    ((i - 1) * COUNT + field, 1.0),
                    (i * COUNT + field, -2.0),
                    ((i + 1) * COUNT + field, 1.0),
                ],
                sd,
            );
        }
    }
    let mut angle_row = |entries: &[(usize, f64)], angle: usize| {
        let sd = ANGLE_SMOOTHNESS_SD_RAD;
        let reference = angle_values[entries[0].0][angle];
        let value = entries
            .iter()
            .map(|&(i, coefficient)| {
                coefficient * angle_difference(angle_values[i][angle], reference) / sd
            })
            .sum();
        let mut derivatives = Vec::new();
        if differentiate {
            for &(i, coefficient) in entries {
                for axis in 0..3 {
                    derivatives.push((
                        i * COUNT + 3 + axis,
                        coefficient * angle_derivatives[i][(angle, axis)] / sd,
                    ));
                }
            }
        }
        rows.push(Row { value, derivatives });
    };
    for i in 1..n.saturating_sub(1) {
        angle_row(&[(i - 1, 1.0), (i, -2.0), (i + 1, 1.0)], 2);
    }
    // A missing view cannot determine its lean/tilt. Interpolate that
    // named angle from neighbours with the same weak angular scale;
    // at a chain end use the nearest neighbour (there is no bracketing pair).
    for (i, observation) in problem.observations.iter().enumerate() {
        for (missing, angle) in [
            (observation.front.is_none(), 0),
            (observation.side.is_none(), 1),
        ] {
            if !missing || n < 2 {
                continue;
            }
            if i == 0 {
                angle_row(&[(0, 1.0), (1, -1.0)], angle);
            } else if i == n - 1 {
                angle_row(&[(i, 1.0), (i - 1, -1.0)], angle);
            } else {
                angle_row(&[(i - 1, 1.0), (i, -2.0), (i + 1, 1.0)], angle);
            }
        }
    }
    let scale = libm::exp(numbers[n * COUNT]);
    rows.push(Row {
        value: (scale - 1.0) / SIDE_SCALE_SD,
        derivatives: if differentiate {
            vec![(n * COUNT, scale / SIDE_SCALE_SD)]
        } else {
            Vec::new()
        },
    });
    rows
}
