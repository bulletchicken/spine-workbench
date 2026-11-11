//! Solver coordinates: rotation vectors, logarithmic sizes and two film unknowns.

use forward_model::rotation::matrix_from_rotation_vector;
use forward_model::vertebra_params::VertebraParams;
use nalgebra::{Matrix3, Vector3};

pub const COUNT: usize = 13;
pub const SIZES: [usize; 5] = [6, 9, 10, 11, 12];

pub fn vertebra(numbers: &[f64]) -> VertebraParams {
    let mut physical = [0.0; COUNT];
    physical.copy_from_slice(&numbers[..COUNT]);
    for index in SIZES {
        physical[index] = libm::exp(physical[index]);
    }
    VertebraParams::from_numbers(physical)
}

/// R = rotX(tilt) rotY(lean) rotZ(twist), with lean in [-pi/2, pi/2].
pub fn angles(numbers: &[f64]) -> [f64; 3] {
    let r = matrix_from_rotation_vector(&Vector3::new(numbers[3], numbers[4], numbers[5]));
    [
        libm::atan2(-r[(1, 2)], r[(2, 2)]),
        libm::atan2(r[(0, 2)], libm::hypot(r[(0, 0)], r[(0, 1)])),
        libm::atan2(-r[(0, 1)], r[(0, 0)]),
    ]
}

pub fn angle_difference(a: f64, b: f64) -> f64 {
    libm::atan2(libm::sin(a - b), libm::cos(a - b))
}

/// Rows are named angles, columns are rotation-vector components.
pub fn angle_jacobian(numbers: &[f64]) -> Matrix3<f64> {
    let mut result = Matrix3::zeros();
    for column in 0..3 {
        let mut plus = numbers.to_vec();
        let mut minus = plus.clone();
        let h = 1e-6;
        plus[3 + column] += h;
        minus[3 + column] -= h;
        let a = angles(&plus);
        let b = angles(&minus);
        for row in 0..3 {
            result[(row, column)] = angle_difference(a[row], b[row]) / (2.0 * h);
        }
    }
    result
}

pub fn usable(numbers: &[f64]) -> bool {
    let n = (numbers.len() - 2) / COUNT;
    numbers.iter().all(|x| x.is_finite())
        && (0..n).all(|i| {
            let v = &numbers[i * COUNT..(i + 1) * COUNT];
            // Numerical domain guards, not anatomical priors.
            SIZES.iter().all(|&j| v[j] > -14.0 && v[j] < 14.0)
                && v[7].abs() < core::f64::consts::PI
                && v[8].abs() < core::f64::consts::PI
        })
        && numbers[n * COUNT].abs() < 14.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use forward_model::rotation::{rot_x, rot_y, rot_z, rotation_vector_from_matrix};
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn named_angles_recover_the_composition(
            tilt in -1.2_f64..1.2, lean in -1.2_f64..1.2, twist in -1.2_f64..1.2
        ) {
            let mut numbers = [0.0; COUNT];
            let rotation = rotation_vector_from_matrix(&(rot_x(tilt) * rot_y(lean) * rot_z(twist)));
            numbers[3..6].copy_from_slice(rotation.as_slice());
            // The sampled lean avoids the Euler singularity. Matrix and
            // vector conversion each round a few times: 1e-12 rad allows it.
            for (got, expected) in angles(&numbers).into_iter().zip([tilt, lean, twist]) {
                prop_assert!((got - expected).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn infinitesimal_rotation_vectors_are_the_named_angles() {
        // At identity, Rx(a) Ry(b) Rz(c) = I + [a,b,c]× to first order.
        assert!(
            (angle_jacobian(&[0.0; COUNT]) - Matrix3::identity())
                .abs()
                .max()
                < 1e-10
        );
    }
}
