//! Swing--twist of the rod's rotation relative to its supplied placement.
//!
//! The unit axis is the ORIGINAL supplied endpoint chord, in the supplied
//! contour's patient axes. It stays fixed as heads slide. For unit quaternion
//! q=(w,v), z=v.axis and D=w²+z², the signed twist is 2 atan2(z,w).
//! The scalar and its derivatives use the hemisphere w>=0. A 180-degree
//! swing perpendicular to the chord has D=0 and no defined twist; it is refused.
//! Derivatives are in patient-axis radians for Exp(omega)*R, including the
//! fixed exponential chart's second derivative. They do not depend on the
//! rod's centroid or its translation. Physical holder torque is -lambda*grad;
//! after a finite swing that vector generally differs from the current chord.

use nalgebra::{Matrix3, Vector3};

/// Project numerical proximity to the undefined swing--twist projection.
const MINIMUM_PROJECTION_SQUARED: f64 = 1e-24;
/// Project rigid-matrix tolerance, matching the chain's plain-pose validation.
const ROTATION_TOLERANCE: f64 = 1e-10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TwistError {
    InvalidRotation,
    AmbiguousSwing,
}
impl TwistError {
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidRotation => "holder rotation",
            Self::AmbiguousSwing => "holder swing-twist ambiguity",
        }
    }
}

pub(super) struct SwingTwist {
    scalar: f64,
    vector: Vector3<f64>,
    axis: Vector3<f64>,
    projection: f64,
    denominator: f64,
}

impl SwingTwist {
    pub fn new(rotation: &Matrix3<f64>, axis: Vector3<f64>) -> Result<Self, TwistError> {
        if rotation.iter().chain(axis.iter()).any(|v| !v.is_finite())
            || (rotation.transpose() * rotation - Matrix3::identity())
                .abs()
                .max()
                >= ROTATION_TOLERANCE
            || (rotation.determinant() - 1.0).abs() >= ROTATION_TOLERANCE
            || (axis.norm_squared() - 1.0).abs() >= ROTATION_TOLERANCE
        {
            return Err(TwistError::InvalidRotation);
        }
        // Largest-component quaternion extraction uses only square roots;
        // it stays regular for rotations close to a half turn.
        let trace = rotation.trace();
        let (mut scalar, mut vector) = if trace > 0.0 {
            let s = 2.0 * (1.0 + trace).sqrt();
            (
                s * 0.25,
                Vector3::new(
                    (rotation[(2, 1)] - rotation[(1, 2)]) / s,
                    (rotation[(0, 2)] - rotation[(2, 0)]) / s,
                    (rotation[(1, 0)] - rotation[(0, 1)]) / s,
                ),
            )
        } else {
            let mut i = 0;
            if rotation[(1, 1)] > rotation[(i, i)] {
                i = 1;
            }
            if rotation[(2, 2)] > rotation[(i, i)] {
                i = 2;
            }
            let j = (i + 1) % 3;
            let k = (i + 2) % 3;
            let s = 2.0 * (1.0 + rotation[(i, i)] - rotation[(j, j)] - rotation[(k, k)]).sqrt();
            let mut vector = Vector3::zeros();
            vector[i] = s * 0.25;
            vector[j] = (rotation[(j, i)] + rotation[(i, j)]) / s;
            vector[k] = (rotation[(k, i)] + rotation[(i, k)]) / s;
            ((rotation[(k, j)] - rotation[(j, k)]) / s, vector)
        };
        let norm = (scalar * scalar + vector.norm_squared()).sqrt();
        scalar /= norm;
        vector /= norm;
        if scalar < 0.0 {
            scalar = -scalar;
            vector = -vector;
        }
        let projection = vector.dot(&axis);
        let denominator = scalar * scalar + projection * projection;
        if denominator <= MINIMUM_PROJECTION_SQUARED {
            return Err(TwistError::AmbiguousSwing);
        }
        Ok(Self {
            scalar,
            vector,
            axis,
            projection,
            denominator,
        })
    }

    pub fn angle_rad(&self) -> f64 {
        2.0 * libm::atan2(self.projection, self.scalar)
    }

    pub fn gradient(&self) -> Vector3<f64> {
        let b = self.axis * self.scalar + self.vector.cross(&self.axis);
        (b * self.scalar + self.vector * self.projection) / self.denominator
    }

    pub fn hessian(&self) -> Matrix3<f64> {
        let b = self.axis * self.scalar + self.vector.cross(&self.axis);
        let squared = self.denominator * self.denominator;
        let h = (self.vector * self.vector.transpose() - b * b.transpose())
            * (self.projection * self.scalar / squared)
            + (self.vector * b.transpose() + b * self.vector.transpose())
                * ((self.scalar * self.scalar - self.projection * self.projection)
                    / (2.0 * squared));
        (h + h.transpose()) * 0.5
    }

    pub fn holding_torque(&self, multiplier_nmm: f64) -> Vector3<f64> {
        -self.gradient() * multiplier_nmm
    }

    /// Remove only the starting guess's twist. Right multiplication by the
    /// inverse twist retains the visible chord direction; the caller retains
    /// the rod centroid when installing this orientation.
    pub fn without_twist(&self) -> Matrix3<f64> {
        let scalar = self.denominator.sqrt();
        let vector = ((self.vector - self.axis * self.projection) * self.scalar
            - self.vector.cross(&self.axis) * self.projection)
            / scalar;
        Matrix3::identity() * (scalar * scalar - vector.norm_squared())
            + vector * vector.transpose() * 2.0
            + vector.cross_matrix() * (2.0 * scalar)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forward_model::rotation::matrix_from_rotation_vector;

    #[test]
    fn swing_twist_value_sign_and_external_torque_follow_the_supplied_chord() {
        let chord = Vector3::new(0.3, -0.4, 0.5).normalize();
        let rest = SwingTwist::new(&Matrix3::identity(), chord).unwrap();
        assert_eq!(rest.angle_rad(), 0.0);
        assert!((rest.gradient() - chord).norm() < 1e-15);
        assert_eq!(rest.hessian(), Matrix3::zeros());
        let turn = SwingTwist::new(&matrix_from_rotation_vector(&(chord * 0.4)), chord).unwrap();
        assert!((turn.angle_rad() - 0.4).abs() < 1e-14);
        assert!((turn.gradient() - chord).norm() < 1e-14);
        assert!((turn.holding_torque(3.0) + chord * 3.0).norm() < 1e-14);
        assert!((turn.without_twist() - Matrix3::identity()).norm() < 1e-14);
    }

    #[test]
    fn both_twist_derivatives_match_independent_fixed_chart_differences() {
        let chord = Vector3::new(0.3, 0.4, 0.5).normalize();
        let rotation = matrix_from_rotation_vector(&Vector3::new(0.7, -0.4, 0.2));
        let value = SwingTwist::new(&rotation, chord).unwrap();
        let hessian = value.hessian();
        for column in 0..3 {
            let mut step = Vector3::zeros();
            step[column] = 1e-6;
            let at = |step: Vector3<f64>| {
                SwingTwist::new(&(matrix_from_rotation_vector(&step) * rotation), chord).unwrap()
            };
            let plus = at(step);
            let minus = at(-step);
            let first = (plus.angle_rad() - minus.angle_rad()) / 2e-6;
            assert!((first - value.gradient()[column]).abs() < 2e-9);
            let fixed_gradient = |step: Vector3<f64>, current: SwingTwist| {
                let cross = step.cross_matrix();
                let left = Matrix3::identity() + cross * 0.5 + cross * cross / 6.0;
                left.transpose() * current.gradient()
            };
            let second = (fixed_gradient(step, plus) - fixed_gradient(-step, minus)) / 2e-6;
            assert!((second - hessian.column(column)).norm() < 2e-9);
        }
        let swing = value.without_twist();
        assert!((swing * chord - rotation * chord).norm() < 1e-14);
        assert!(SwingTwist::new(&swing, chord).unwrap().angle_rad().abs() < 1e-14);
    }

    #[test]
    fn the_undefined_half_turn_swing_is_refused() {
        let rotation = matrix_from_rotation_vector(&(Vector3::x() * core::f64::consts::PI));
        assert!(matches!(
            SwingTwist::new(&rotation, Vector3::z()),
            Err(TwistError::AmbiguousSwing)
        ));
        let twist = SwingTwist::new(&rotation, Vector3::x()).unwrap();
        assert!((twist.angle_rad().abs() - core::f64::consts::PI).abs() < 1e-14);
        assert!((twist.without_twist() - Matrix3::identity()).norm() < 1e-14);
    }
}
