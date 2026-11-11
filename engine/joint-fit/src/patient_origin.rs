//! Move the fitted spine's L5 underside to the patient-frame origin.

use crate::parameters::{COUNT, vertebra};
use crate::problem::Problem;
use forward_model::endplates::Endplates;
use nalgebra::{Matrix3xX, Vector3};
use shapes::levels::Level;

pub fn index(problem: &Problem) -> usize {
    problem
        .observations
        .iter()
        .position(|v| v.level == Level::L5)
        .unwrap_or(problem.observations.len() - 1)
}

pub fn underside(numbers: &[f64], origin: usize) -> Vector3<f64> {
    Endplates::of(&vertebra(&numbers[origin * COUNT..(origin + 1) * COUNT]))
        .bottom
        .centre
}

/// Origin derivatives include its centre, rotation and logarithmic height.
pub fn jacobian(numbers: &[f64], origin: usize) -> Matrix3xX<f64> {
    let mut result = Matrix3xX::zeros(COUNT);
    let start = origin * COUNT;
    let local = &numbers[start..start + COUNT];
    for j in 0..COUNT {
        let h = 1e-5 * local[j].abs().max(1.0);
        let mut plus = local.to_vec();
        let mut minus = plus.clone();
        plus[j] += h;
        minus[j] -= h;
        let a = Endplates::of(&vertebra(&plus)).bottom.centre;
        let b = Endplates::of(&vertebra(&minus)).bottom.centre;
        result.set_column(j, &((a - b) / (2.0 * h)));
    }
    result
}
