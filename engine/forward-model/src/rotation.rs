//! Rotation vector to rotation matrix and back, and the three rotations
//! about the patient axes.
//!
//! A rotation vector is the rotation's axis times its angle in radians,
//! turning by the right-hand rule. A vertebra's rotation vector takes its
//! own axes to the patient's (see `docs/conventions/signs-and-wedges.md`):
//!
//! - [`rot_x`]`(+θ)` tips the column toward the patient's right (−y) going
//!   up, and lowers the right side of a horizontal plane;
//! - [`rot_y`]`(+φ)` tips the column forward going up, and lowers the front;
//! - [`rot_z`]`(+ψ)` turns the front of the body toward the patient's left.
//!
//! Both directions are smooth everywhere a fit can wander: near a zero
//! angle each uses a form that has no cancellation and no division by
//! zero, so a gradient never meets a jump.

use nalgebra::{Matrix3, Vector3};

/// Rotation about the patient's x axis (forward) by `angle` radians.
pub fn rot_x(angle: f64) -> Matrix3<f64> {
    let (s, c) = (libm::sin(angle), libm::cos(angle));
    Matrix3::new(1.0, 0.0, 0.0, 0.0, c, -s, 0.0, s, c)
}

/// Rotation about the patient's y axis (to the left) by `angle` radians.
pub fn rot_y(angle: f64) -> Matrix3<f64> {
    let (s, c) = (libm::sin(angle), libm::cos(angle));
    Matrix3::new(c, 0.0, s, 0.0, 1.0, 0.0, -s, 0.0, c)
}

/// Rotation about the patient's z axis (up) by `angle` radians.
pub fn rot_z(angle: f64) -> Matrix3<f64> {
    let (s, c) = (libm::sin(angle), libm::cos(angle));
    Matrix3::new(c, -s, 0.0, s, c, 0.0, 0.0, 0.0, 1.0)
}

/// The rotation matrix of a rotation vector, by Rodrigues' formula:
/// `R = I + (sin θ / θ) K + ((1 − cos θ) / θ²) K²`, with `θ = |r|` and `K`
/// the cross-product matrix of `r`.
///
/// `(1 − cos θ) / θ²` is computed as `½ (sin(θ/2) / (θ/2))²`, which keeps
/// full precision as θ shrinks; at exactly zero both factors take their
/// limits, 1 and ½.
pub fn matrix_from_rotation_vector(r: &Vector3<f64>) -> Matrix3<f64> {
    let theta = r.norm();
    let (first, second) = if theta > 0.0 {
        let half = 0.5 * theta;
        let sinc_half = libm::sin(half) / half;
        (libm::sin(theta) / theta, 0.5 * sinc_half * sinc_half)
    } else {
        (1.0, 0.5)
    };
    let k = r.cross_matrix();
    Matrix3::identity() + k * first + k * k * second
}

/// The rotation vector of a rotation matrix, with angle in `[0, π]`.
///
/// The angle is `atan2(|s|, c)`, where `s = sin θ · axis` is read from the
/// matrix's skew part and `c = cos θ = (trace − 1) / 2`, which is accurate
/// at every angle. Below a quarter turn the axis comes from the skew part.
/// From a quarter turn up it comes from the symmetric part,
/// `(R + Rᵀ)/2 − c·I = (1 − c)·axis·axisᵀ`, because the skew part fades to
/// nothing at a half turn; the skew part still picks the sign. The two
/// ways agree where they meet, so the answer has no jump. At exactly a
/// half turn `r` and `−r` are the same rotation, and the one returned has
/// its largest component positive.
///
/// `m` must be a rotation matrix (orthonormal, determinant +1).
pub fn rotation_vector_from_matrix(m: &Matrix3<f64>) -> Vector3<f64> {
    let skew = Vector3::new(
        m[(2, 1)] - m[(1, 2)],
        m[(0, 2)] - m[(2, 0)],
        m[(1, 0)] - m[(0, 1)],
    ) * 0.5;
    let s = skew.norm();
    let c = 0.5 * (m.trace() - 1.0);
    let theta = libm::atan2(s, c);
    if c > 0.0 {
        let scale = if s > 0.0 { theta / s } else { 1.0 };
        return skew * scale;
    }
    let symmetric = (m + m.transpose()) * 0.5 - Matrix3::identity() * c;
    let k = (0..3)
        .max_by(|&i, &j| symmetric[(i, i)].total_cmp(&symmetric[(j, j)]))
        .unwrap_or(0);
    let axis = symmetric.column(k) / (symmetric[(k, k)] * (1.0 - c)).sqrt();
    let sign = if axis.dot(&skew) < 0.0 { -1.0 } else { 1.0 };
    axis * (sign * theta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f64::consts::{FRAC_PI_2, PI};

    fn largest_difference(a: &Matrix3<f64>, b: &Matrix3<f64>) -> f64 {
        (a - b).abs().max()
    }

    #[test]
    fn a_vector_along_an_axis_is_the_rotation_about_that_axis() {
        let angle = 0.7;
        let cases = [
            (Vector3::new(angle, 0.0, 0.0), rot_x(angle)),
            (Vector3::new(0.0, angle, 0.0), rot_y(angle)),
            (Vector3::new(0.0, 0.0, angle), rot_z(angle)),
        ];
        for (r, expected) in cases {
            assert!(largest_difference(&matrix_from_rotation_vector(&r), &expected) < 1e-15);
        }
    }

    #[test]
    fn small_angles_match_the_axis_rotations_to_a_few_ulps() {
        // Worked by hand: for r = θ x̂, K has −θ at (1, 2) and +θ at
        // (2, 1), so (sin θ / θ) K puts ∓sin θ there; K² is −θ² at (1, 1)
        // and (2, 2), and ½ (sin(θ/2) / (θ/2))² · −θ² = −2 sin²(θ/2), so
        // the diagonal is 1 − 2 sin²(θ/2) = cos θ. That is rotX(θ) exactly,
        // and likewise about ŷ and ẑ, so the two may differ only by
        // rounding: a few roundings per entry, allowed 4 ulps of the
        // entry. Replacing either factor by its second-order series
        // (1 − θ²/6, ½ − θ²/24) is off by about θ⁴/120 relative, which is
        // some 40 ulps at θ = 1e-3 and 4·10⁵ ulps at 1e-2.
        let close = |a: f64, b: f64| (a - b).abs() <= 4.0 * f64::EPSILON * a.abs().max(b.abs());
        for magnitude in [1e-4, 1e-3, 1e-2, 0.1] {
            for angle in [magnitude, -magnitude] {
                let cases = [
                    ("x", Vector3::new(angle, 0.0, 0.0), rot_x(angle)),
                    ("y", Vector3::new(0.0, angle, 0.0), rot_y(angle)),
                    ("z", Vector3::new(0.0, 0.0, angle), rot_z(angle)),
                ];
                for (axis, r, expected) in cases {
                    let m = matrix_from_rotation_vector(&r);
                    for (got, want) in m.iter().zip(expected.iter()) {
                        assert!(
                            close(*got, *want),
                            "about {axis} by {angle}: {got:e} against {want:e}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_quarter_turn_about_z_takes_forward_to_the_left() {
        // Worked by hand: rotZ(π/2) sends x̂ to ŷ and ŷ to −x̂.
        let r = matrix_from_rotation_vector(&Vector3::new(0.0, 0.0, FRAC_PI_2));
        assert!((r * Vector3::x() - Vector3::y()).norm() < 1e-15);
        assert!((r * Vector3::y() + Vector3::x()).norm() < 1e-15);
    }

    #[test]
    fn a_zero_vector_is_no_rotation() {
        assert_eq!(
            matrix_from_rotation_vector(&Vector3::zeros()),
            Matrix3::identity()
        );
        assert_eq!(
            rotation_vector_from_matrix(&Matrix3::identity()),
            Vector3::zeros()
        );
    }

    #[test]
    fn tiny_angles_keep_their_precision() {
        // For θ = 1e-9 the matrix differs from I + K by θ²/2 ≈ 5e-19 at
        // most, and the vector must come back to within rounding.
        let r = Vector3::new(1e-9, -2e-9, 0.5e-9);
        let m = matrix_from_rotation_vector(&r);
        let expected = Matrix3::identity() + r.cross_matrix();
        assert!(largest_difference(&m, &expected) < 1e-17);
        let back = rotation_vector_from_matrix(&m);
        assert!((back - r).norm() < 1e-24);
    }

    #[test]
    fn round_trips_hold_at_every_angle_including_a_half_turn() {
        let axis = Vector3::new(0.3, -0.5, 0.8).normalize();
        let angles = [
            1e-12,
            1e-6,
            0.1,
            1.0,
            FRAC_PI_2 - 1e-9,
            FRAC_PI_2,
            FRAC_PI_2 + 1e-9,
            2.5,
            PI - 1e-6,
            PI - 1e-9,
        ];
        for angle in angles {
            let r = axis * angle;
            let m = matrix_from_rotation_vector(&r);
            let back = rotation_vector_from_matrix(&m);
            assert!(
                largest_difference(&matrix_from_rotation_vector(&back), &m) < 1e-12,
                "angle {angle}"
            );
            assert!((back - r).norm() < 1e-9 * angle.max(1.0), "angle {angle}");
        }
    }

    #[test]
    fn an_exact_half_turn_gives_a_vector_of_length_pi() {
        let m = rot_y(PI);
        let back = rotation_vector_from_matrix(&m);
        assert!((back.norm() - PI).abs() < 1e-15);
        assert!(largest_difference(&matrix_from_rotation_vector(&back), &m) < 1e-15);
    }

    #[test]
    fn composed_rotations_survive_the_round_trip() {
        let m = rot_x(0.4) * rot_y(-0.25) * rot_z(1.1);
        let back = matrix_from_rotation_vector(&rotation_vector_from_matrix(&m));
        assert!(largest_difference(&back, &m) < 1e-15);
    }
}
