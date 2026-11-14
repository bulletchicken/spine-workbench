//! Six joint springs in the lower body's frame, in N/mm and N·mm/radian.
//!
//! Patient axes follow `forward-model`: x anterior, y patient-left, z up.
//! Positive moments obey the right-hand rule: +Mx tips up toward −y,
//! +My tips up toward +x, +Mz turns anterior toward patient-left. A spring's
//! gradient is the force needed to hold its deformation; its force on the
//! upper vertebra is the negative gradient. The lower vertebra receives the
//! opposite force and the balancing moment at the common disc point.
//!
//! Rotation deformation is the exact principal log of the current relative
//! rotation times the inverse standing relative rotation. It is expressed
//! in the current lower frame. The energy is smooth for errors below π;
//! the principal-log cut at π is expressly outside the differentiable domain.

use crate::evidence::{Evidence, TableKind, Unit, nmm_per_degree_to_nmm_per_radian};
use forward_model::rotation::rotation_vector_from_matrix;
use nalgebra::{Matrix3, SVector, Vector3};

/// A joint's diagonal, translations first (N/mm), rotations last (N·mm/rad).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stiffness(pub SVector<f64, 6>);

/// Missing evidence and inadmissible stiffness remain explicit.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SpringError {
    #[error("missing-joint-evidence: {0}")]
    MissingEvidence(String),
    #[error("joint-evidence-unit: {0}")]
    WrongUnit(String),
    #[error("joint-stiffness-must-be-finite-and-positive")]
    InvalidStiffness,
}

impl Stiffness {
    /// Plain positive stiffnesses; no prior or unit conversion is assumed.
    pub fn new(values: [f64; 6]) -> Result<Self, SpringError> {
        if values.iter().any(|v| !v.is_finite() || *v <= 0.0) {
            return Err(SpringError::InvalidStiffness);
        }
        Ok(Self(values.into()))
    }

    /// Read Tachi's AP/ML/CC columns through the evidence reader. The axis
    /// mapping follows Table 1's headings: AP moment is lateral bending,
    /// ML moment is flexion/extension, CC moment is axial rotation.
    /// `lumbosacral` selects the distinct L5/S1 row, never a fallback.
    pub fn from_evidence(evidence: &Evidence, lumbosacral: bool) -> Result<Self, SpringError> {
        let row = if lumbosacral {
            "l5-s1"
        } else {
            "t1-2-to-l4-5-intact"
        };
        let terms = ["ap", "ml", "cc", "moment-1", "moment-2", "moment-3"];
        let mut values = [0.0; 6];
        for (i, term) in terms.iter().enumerate() {
            let cell = evidence
                .value(TableKind::JointSprings, row, term)
                .ok_or_else(|| SpringError::MissingEvidence(format!("{row}/{term}")))?;
            values[i] = match (i < 3, cell.unit) {
                (true, Unit::NPerMm) => cell.value,
                (false, Unit::NmmPerDegree) => nmm_per_degree_to_nmm_per_radian(cell.value),
                _ => return Err(SpringError::WrongUnit(format!("{row}/{term}"))),
            };
        }
        Self::new(values)
    }

    /// Explicit multipliers: overall scale, translation xyz, rotation xyz.
    /// Rib, release and preload choices belong to the caller; no population
    /// number is embedded here. This also preserves exact uniform scaling.
    pub fn scaled(&self, scale: f64, factors: [f64; 6]) -> Result<Self, SpringError> {
        Self::new(core::array::from_fn(|i| self.0[i] * scale * factors[i]))
    }

    /// Half the scalar product of deformation and its conjugate response.
    pub fn energy(&self, deformation: &SVector<f64, 6>) -> f64 {
        0.5 * deformation.dot(&self.response(deformation))
    }

    /// Conjugate force/moment in the spring frame; a resisting force on
    /// the deforming body has the opposite sign.
    pub fn response(&self, deformation: &SVector<f64, 6>) -> SVector<f64, 6> {
        self.0.component_mul(deformation)
    }
}

/// Principal log, radians in the current lower frame. Argument order is
/// reference upper, reference lower, current upper, current lower.
pub fn rotation_deformation(
    reference_upper: &Matrix3<f64>,
    reference_lower: &Matrix3<f64>,
    current_upper: &Matrix3<f64>,
    current_lower: &Matrix3<f64>,
) -> Vector3<f64> {
    let reference = reference_lower.transpose() * reference_upper;
    rotation_vector_from_matrix(
        &(current_lower.transpose() * current_upper * reference.transpose()),
    )
}

/// Derivative of the log under a left-multiplied rotation increment.
/// The series near zero avoids subtracting two nearly equal numbers.
pub fn log_left_derivative(rotation: &Vector3<f64>) -> Matrix3<f64> {
    let coefficient = log_coefficients(rotation.norm_squared()).0;
    let skew = rotation.cross_matrix();
    Matrix3::identity() - skew * 0.5 + skew * skew * coefficient
}

/// Coefficient of [phi]^2 in J_l(phi)^-1 and its derivative with respect
/// to phi.phi. The switch is a project numerical choice, not a material cell.
fn log_coefficients(squared: f64) -> (f64, f64) {
    const SERIES_SQUARED_ANGLE: f64 = 0.25;
    if squared < SERIES_SQUARED_ANGLE {
        // Rational series obtained by dividing cos(theta/2) by
        // sin(theta/2)/(theta/2), then forming (1-half*cot(half))/theta^2.
        const A: [f64; 9] = [
            1.0 / 12.0,
            1.0 / 720.0,
            1.0 / 30240.0,
            1.0 / 1209600.0,
            1.0 / 47900160.0,
            691.0 / 1307674368000.0,
            1.0 / 74724249600.0,
            3617.0 / 10670622842880000.0,
            43867.0 / 5109094217170944000.0,
        ];
        let mut value = A[8];
        let mut derivative = 0.0;
        for &coefficient in A[..8].iter().rev() {
            derivative = derivative * squared + value;
            value = value * squared + coefficient;
        }
        (value, derivative)
    } else {
        let half = squared.sqrt() * 0.5;
        let sin = libm::sin(half);
        let half_cot = half * libm::cos(half) / sin;
        (
            (1.0 - half_cot) / squared,
            (0.5 * half_cot + 0.5 * half * half / (sin * sin) - 1.0) / (squared * squared),
        )
    }
}

/// Rotational holding couple and exact second derivative in the local
/// exponential retraction. Unlike the derivative of a moving spatial
/// moment, this tangent is symmetric: its moving-basis term is included.
pub(crate) fn rotation_response_tangent(
    rotation: &Vector3<f64>,
    stiffness: &Stiffness,
) -> (Vector3<f64>, Matrix3<f64>) {
    let weights = Matrix3::from_diagonal(&stiffness.0.fixed_rows::<3>(3).into_owned());
    let response = weights * rotation;
    let skew = rotation.cross_matrix();
    let (a, da) = log_coefficients(rotation.norm_squared());
    let log_derivative = Matrix3::identity() - skew * 0.5 + skew * skew * a;
    let couple = log_derivative.transpose() * response;
    let directional = log_derivative.transpose() * weights
        - response.cross_matrix() * 0.5
        - (skew * response).cross_matrix() * a
        - skew * response.cross_matrix() * a
        + (skew * skew * response) * rotation.transpose() * (2.0 * da);
    let spatial_derivative = directional * log_derivative;
    // Differentiating moments in a moving spatial basis is not itself an
    // energy Hessian. The exponential chart adds +.5[couple]x; the result
    // is mathematically symmetric, so the average removes only round-off.
    let chart_hessian = spatial_derivative + couple.cross_matrix() * 0.5;
    let tangent = (chart_hessian + chart_hessian.transpose()) * 0.5;
    (couple, tangent)
}

/// Change the constitutive stiffness about the current strain, keeping its
/// existing holding couple. The log's force-dependent curvature is unchanged.
pub(crate) fn rotation_response_tangent_with_material_scale(
    rotation: &Vector3<f64>,
    stiffness: &Stiffness,
    material_scale: f64,
) -> (Vector3<f64>, Matrix3<f64>) {
    let (couple, tangent) = rotation_response_tangent(rotation, stiffness);
    if material_scale == 1.0 {
        return (couple, tangent);
    }
    let weights = Matrix3::from_diagonal(&stiffness.0.fixed_rows::<3>(3).into_owned());
    let derivative = log_left_derivative(rotation);
    let material = derivative.transpose() * weights * derivative;
    let added = material * (material_scale - 1.0);
    (couple, tangent + (added + added.transpose()) * 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;
    use forward_model::rotation::{matrix_from_rotation_vector, rot_x, rot_y, rot_z};

    #[test]
    fn the_log_derivative_matches_an_independent_central_difference() {
        let phi = Vector3::new(0.7, -0.4, 0.2);
        let rotation = matrix_from_rotation_vector(&phi);
        for axis in 0..3 {
            let mut perturbation = Vector3::zeros();
            perturbation[axis] = 1e-6;
            let plus = rotation_vector_from_matrix(
                &(matrix_from_rotation_vector(&perturbation) * rotation),
            );
            let minus = rotation_vector_from_matrix(
                &(matrix_from_rotation_vector(&(-perturbation)) * rotation),
            );
            assert!(
                ((plus - minus) / 2e-6 - log_left_derivative(&phi).column(axis)).norm() < 2e-10
            );
        }
    }

    #[test]
    fn a_common_finite_rotation_does_not_change_joint_energy() {
        let r0 = rot_y(0.4);
        let r1 = rot_z(-0.3) * r0;
        let r2 = rot_x(0.6) * r1;
        let q = rot_y(-0.9) * rot_z(0.5);
        let a = rotation_deformation(&r0, &Matrix3::identity(), &r2, &r1);
        let b = rotation_deformation(&(q * r0), &q, &(q * r2), &(q * r1));
        assert!((a - b).norm() < 1e-15);
    }

    #[test]
    fn table_axes_and_units_follow_the_reader() {
        let evidence = Evidence::read().expect("bundled evidence");
        for basal in [false, true] {
            let row = if basal {
                "l5-s1"
            } else {
                "t1-2-to-l4-5-intact"
            };
            let k = Stiffness::from_evidence(&evidence, basal).expect("six columns");
            for (i, term) in ["ap", "ml", "cc", "moment-1", "moment-2", "moment-3"]
                .iter()
                .enumerate()
            {
                let cell = evidence
                    .value(TableKind::JointSprings, row, term)
                    .expect("cell");
                let displacement = if i < 3 {
                    1.0
                } else {
                    core::f64::consts::PI / 180.0
                };
                assert!((k.0[i] * displacement - cell.value).abs() < 1e-10);
            }
        }
    }
    #[test]
    fn exact_rotational_tangent_matches_an_independent_fixed_chart_gradient_difference() {
        let stiffness = Stiffness::new([100.0, 200.0, 300.0, 4000.0, 5000.0, 6000.0]).unwrap();
        let weights = Matrix3::from_diagonal(&stiffness.0.fixed_rows::<3>(3).into_owned());
        let axis = Vector3::new(0.7, -0.4, 0.2).normalize();
        for angle in [0.0, 1e-8, 0.499, 0.501, 1.7, 2.9] {
            let phi = axis * angle;
            let base = matrix_from_rotation_vector(&phi);
            let (moment, tangent) = rotation_response_tangent(&phi, &stiffness);
            assert!(
                (moment - log_left_derivative(&phi).transpose() * weights * phi).norm() < 1e-12
            );
            assert_eq!((tangent - tangent.transpose()).abs().max(), 0.0);
            for column in 0..3 {
                let h = 2e-6;
                let mut epsilon = Vector3::zeros();
                epsilon[column] = h;
                let at = |step: Vector3<f64>| {
                    let rotated = matrix_from_rotation_vector(&step) * base;
                    let psi = rotation_vector_from_matrix(&rotated);
                    let spatial = log_left_derivative(&psi).transpose() * weights * psi;
                    let cross = step.cross_matrix();
                    let exponential_differential =
                        Matrix3::identity() + cross * 0.5 + cross * cross / 6.0;
                    exponential_differential.transpose() * spatial
                };
                let difference = (at(epsilon) - at(-epsilon)) / (2.0 * h);
                let error = (difference - tangent.column(column)).abs().max();
                assert!(error < 2e-5, "angle {angle} column {column} error {error}");
            }
        }
    }
}
