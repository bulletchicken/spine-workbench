//! One residual row pulling a value toward a mean, in units of its spread.

use crate::parameters::{COUNT, angle_difference, angle_jacobian, angles};
use crate::residuals::Row;

/// A mean and one standard deviation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spread {
    pub mean: f64,
    pub sd: f64,
}

/// Index of forward lean and of twist among the named angles.
pub const LEAN: usize = 1;
pub const TWIST: usize = 2;

/// A sum of solver numbers, each times its coefficient, toward `target`.
pub fn linear(
    numbers: &[f64],
    entries: &[(usize, f64)],
    target: Spread,
    differentiate: bool,
) -> Row {
    let sum: f64 = entries.iter().map(|&(i, c)| numbers[i] * c).sum();
    Row {
        value: (sum - target.mean) / target.sd,
        derivatives: if differentiate {
            entries.iter().map(|&(i, c)| (i, c / target.sd)).collect()
        } else {
            Vec::new()
        },
    }
}

/// One named angle of vertebra `index` toward `target`.
pub fn angle(
    numbers: &[f64],
    index: usize,
    angle: usize,
    target: Spread,
    differentiate: bool,
) -> Row {
    let local = &numbers[index * COUNT..(index + 1) * COUNT];
    Row {
        value: angle_difference(angles(local)[angle], target.mean) / target.sd,
        derivatives: if differentiate {
            let jacobian = angle_jacobian(local);
            (0..3)
                .map(|axis| {
                    (
                        index * COUNT + 3 + axis,
                        jacobian[(angle, axis)] / target.sd,
                    )
                })
                .collect()
        } else {
            Vec::new()
        },
    }
}
