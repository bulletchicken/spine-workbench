//! The patient frame: millimetres, x forward, y to the patient's left, z up,
//! with the origin at the centre of the underside of L5.
//!
//! The axes are right-handed (x × y = z). Every three-dimensional position
//! and direction in this crate is a [`Vector3`] in this frame. See
//! `docs/conventions/patient-frame.md`.

use nalgebra::Vector3;
use shapes::film_geometry::ImageRight;

/// Forward: the patient's anterior direction, +x.
pub const ANTERIOR: Vector3<f64> = Vector3::new(1.0, 0.0, 0.0);

/// The patient's left, +y.
pub const PATIENT_LEFT: Vector3<f64> = Vector3::new(0.0, 1.0, 0.0);

/// Up, toward the head, +z.
pub const UP: Vector3<f64> = Vector3::new(0.0, 0.0, 1.0);

/// The patient direction that points to screen right on a film shown
/// upright: `+y` for patient left, `−y` for patient right, `+x` for
/// anterior, `−x` for posterior.
pub fn screen_right(image_right: ImageRight) -> Vector3<f64> {
    match image_right {
        ImageRight::PatientLeft => PATIENT_LEFT,
        ImageRight::PatientRight => -PATIENT_LEFT,
        ImageRight::Anterior => ANTERIOR,
        ImageRight::Posterior => -ANTERIOR,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_axes_are_right_handed() {
        assert_eq!(ANTERIOR.cross(&PATIENT_LEFT), UP);
    }

    #[test]
    fn mirrored_films_look_along_opposite_directions() {
        for right in [
            ImageRight::PatientLeft,
            ImageRight::PatientRight,
            ImageRight::Anterior,
            ImageRight::Posterior,
        ] {
            assert_eq!(screen_right(right.mirrored()), -screen_right(right));
        }
    }
}
