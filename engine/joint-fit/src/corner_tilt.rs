//! The tilt a front film shows, read straight from a body's four corners:
//! the line from the middle of its right side to the middle of its left
//! side, in the patient's own sense. Side tilt `rotX(+θ)` raises the left
//! side of the body, so the tilt is positive when the left side is higher,
//! whichever way the film faces.

use crate::problem::{Observations, Problem, millimetres};
use nalgebra::Vector2;

/// From the middle of the right side to the middle of the left side, with
/// corners in the order `supL, supR, infR, infL`, each toward the patient's
/// left and up, in millimetres. Each side's two corners are added before
/// the sides are taken apart, so the same corners named the other way round
/// give exactly the opposite line.
pub fn across(corners: [Vector2<f64>; 4]) -> Vector2<f64> {
    let [sup_l, sup_r, inf_r, inf_l] = corners;
    ((sup_l + inf_l) - (sup_r + inf_r)) * 0.5
}

/// The tilt of [`across`], radians.
pub fn tilt_rad(corners: [Vector2<f64>; 4]) -> f64 {
    let line = across(corners);
    libm::atan2(line.y, line.x)
}

/// The tilt the front film shows for one vertebra, or `None` when it has no
/// front marks.
pub fn of(problem: &Problem, observation: &Observations) -> Option<f64> {
    let corners = observation.front?.corners();
    Some(tilt_rad(
        corners.map(|mark| millimetres(&problem.front, mark)),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use forward_model::cameras::parallel::ParallelCamera;
    use forward_model::project::front_marks;
    use forward_model::vertebra_params::VertebraParams;
    use nalgebra::Vector3;
    use shapes::film_geometry::{FilmGeometry, ImageRight};
    use shapes::marks::Mark;
    use shapes::pixels::Px;

    fn camera(image_right: ImageRight) -> ParallelCamera {
        let geometry = FilmGeometry {
            mm_per_px: Some(0.5),
            image_right: Some(image_right),
            ..FilmGeometry::UNKNOWN
        };
        ParallelCamera::for_film(&geometry, [400, 600], Vector2::new(200.0, 500.0))
            .expect("a known camera")
    }

    #[test]
    fn a_tilt_worked_by_hand() {
        // A front film facing the patient's left, so the patient's left is
        // on the right of the picture. Corners in pixels: supL (230, 380),
        // supR (170, 400), infR (170, 440), infL (230, 420). The middle of
        // the left side is (230, 400) and of the right side (170, 420): the
        // left side is 20 px higher over 60 px across, at 0.5 mm a pixel
        // 10 mm over 30 mm, so the tilt is atan(1/3) = +18.435 degrees, the
        // way rotX(+θ) tilts a body.
        let pixels = [
            (230.0, 380.0),
            (170.0, 400.0),
            (170.0, 440.0),
            (230.0, 420.0),
        ];
        let corners = |camera: &ParallelCamera, order: [usize; 4]| {
            order.map(|i| millimetres(camera, Mark::auto(Px::new(pixels[i].0, pixels[i].1))))
        };
        let left = camera(ImageRight::PatientLeft);
        let tilt = tilt_rad(corners(&left, [0, 1, 2, 3]));
        assert!((tilt.to_degrees() - 18.435).abs() < 1e-3, "{tilt}");
        assert!((tilt - libm::atan(1.0 / 3.0)).abs() < 1e-15);
        // The same pixels on a film facing the patient's right: the higher
        // side of the picture is now the patient's right, so the corners
        // named supL and infL are the two on the left of the picture, and
        // the tilt is exactly the opposite.
        let right = camera(ImageRight::PatientRight);
        let mirrored = tilt_rad(corners(&right, [1, 0, 3, 2]));
        assert_eq!(mirrored, -tilt);
    }

    #[test]
    fn a_body_tilted_by_side_tilt_shows_that_tilt() {
        // A body turned by rotX(+0.1) alone, its left side raised, drawn by
        // the forward model on films facing either way: its corners show a
        // tilt of +0.1 radians on both, the same sign as its side tilt.
        let body = VertebraParams {
            centre: Vector3::new(0.0, 4.0, 30.0),
            rotation: Vector3::new(0.1, 0.0, 0.0),
            height: 20.0,
            wedge_front: 0.0,
            wedge_side: 0.0,
            top_half_width: 15.0,
            top_half_depth: 12.0,
            bottom_half_width: 15.0,
            bottom_half_depth: 12.0,
        };
        for facing in [ImageRight::PatientLeft, ImageRight::PatientRight] {
            let camera = camera(facing);
            let marks = front_marks(&body, &camera).expect("front marks");
            let tilt = tilt_rad(marks.corners().map(|mark| millimetres(&camera, mark)));
            assert!((tilt - 0.1).abs() < 1e-12, "{facing:?}: {tilt}");
        }
    }
}
