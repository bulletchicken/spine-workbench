//! The parallel camera: `u = u0 + across / mmPerPx` and
//! `v = v0 − z / mmPerPx`, where `across` is the patient direction the film's
//! `imageRight` names.
//!
//! In full, on the film shown upright (after its quarter turns, before any
//! mirroring):
//!
//! ```text
//! u = u0 + across / su        across = p · screen_right(imageRight)
//! v = v0 − z / sv
//! ```
//!
//! with `su`, `sv` the upright pixel's width and height in mm and `(u0, v0)`
//! the upright pixel where the patient origin lands. The answer is then
//! turned back to original film pixels, the inverse of the app's
//! `uprightMatrix` with mirroring off. With `w = width − 1` and
//! `h = height − 1` of the film as stored:
//!
//! | quarter turns | film x | film y |
//! |---|---|---|
//! | 0 | u | v |
//! | 1 | v | h − u |
//! | 2 | w − u | h − v |
//! | 3 | w − v | u |

use crate::cameras::Camera;
use crate::corners::{AxisAlongBeam, CornerAxis, affine_corners};
use crate::endplates::Endplate;
use crate::patient_frame::screen_right;
use nalgebra::{Matrix2, Matrix2x3, Vector2, Vector3};
use shapes::film_geometry::{FilmGeometry, ImageRight, QuarterTurns};

/// A film taken with parallel rays: every point lands straight across from
/// where it is, at the same scale whatever its depth.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParallelCamera {
    /// The patient direction pointing to screen right once the film is
    /// upright.
    pub image_right: ImageRight,
    /// Clockwise quarter turns that show the film upright.
    pub quarter_turns: QuarterTurns,
    /// The film's width and height in pixels as stored, which undoing the
    /// turns needs.
    pub film_size: [u32; 2],
    /// The upright pixel's width and height in mm: `su` and `sv`.
    pub upright_mm_per_px: [f64; 2],
    /// The upright pixel `(u0, v0)` where the patient origin lands.
    pub origin: Vector2<f64>,
}

impl ParallelCamera {
    /// The camera of a film from its saved geometry, its stored size and
    /// the upright pixel where the patient origin lands. The stored
    /// spacings (`mmPerPx` along a row, `mmPerPxVertical` or `mmPerPx` down
    /// a column) become the upright pixel's width and height, swapped when
    /// the quarter turns are odd.
    ///
    /// `None` while the film's scale or its facing is unknown, or when a
    /// spacing is not a positive finite number.
    pub fn for_film(
        geometry: &FilmGeometry,
        film_size: [u32; 2],
        origin: Vector2<f64>,
    ) -> Option<Self> {
        let along_row = geometry.mm_per_px?;
        let down_column = geometry.mm_per_px_vertical.unwrap_or(along_row);
        let usable = |s: f64| s.is_finite() && s > 0.0;
        if !usable(along_row) || !usable(down_column) {
            return None;
        }
        let upright_mm_per_px = if geometry.quarter_turns.get().is_multiple_of(2) {
            [along_row, down_column]
        } else {
            [down_column, along_row]
        };
        Some(ParallelCamera {
            image_right: geometry.image_right?,
            quarter_turns: geometry.quarter_turns,
            film_size,
            upright_mm_per_px,
            origin,
        })
    }

    /// The linear part of the map from upright pixels to film pixels.
    fn turn_back(&self) -> Matrix2<f64> {
        match self.quarter_turns.get() {
            0 => Matrix2::new(1.0, 0.0, 0.0, 1.0),
            1 => Matrix2::new(0.0, 1.0, -1.0, 0.0),
            2 => Matrix2::new(-1.0, 0.0, 0.0, -1.0),
            _ => Matrix2::new(0.0, -1.0, 1.0, 0.0),
        }
    }

    /// Upright pixels to original film pixels.
    fn upright_to_film(&self, u: f64, v: f64) -> Vector2<f64> {
        let w = f64::from(self.film_size[0]) - 1.0;
        let h = f64::from(self.film_size[1]) - 1.0;
        match self.quarter_turns.get() {
            0 => Vector2::new(u, v),
            1 => Vector2::new(v, h - u),
            2 => Vector2::new(w - u, h - v),
            _ => Vector2::new(w - v, u),
        }
    }
}

impl Camera for ParallelCamera {
    fn project(&self, point: &Vector3<f64>) -> Vector2<f64> {
        let [su, sv] = self.upright_mm_per_px;
        let across = point.dot(&screen_right(self.image_right));
        self.upright_to_film(self.origin.x + across / su, self.origin.y - point.z / sv)
    }

    fn jacobian(&self, _point: &Vector3<f64>) -> Matrix2x3<f64> {
        let [su, sv] = self.upright_mm_per_px;
        let right = screen_right(self.image_right) / su;
        let upright = Matrix2x3::new(right.x, right.y, right.z, 0.0, 0.0, -1.0 / sv);
        self.turn_back() * upright
    }

    fn corners(
        &self,
        endplate: &Endplate,
        axis: CornerAxis,
    ) -> Result<[Vector2<f64>; 2], AxisAlongBeam> {
        affine_corners(
            endplate,
            self.project(&endplate.centre),
            &self.jacobian(&endplate.centre),
            axis,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry(turns: u8, image_right: ImageRight) -> FilmGeometry {
        FilmGeometry {
            mm_per_px: Some(0.4),
            mm_per_px_vertical: Some(0.5),
            quarter_turns: QuarterTurns::new(turns).expect("0 to 3 turns"),
            image_right: Some(image_right),
            ..FilmGeometry::UNKNOWN
        }
    }

    #[test]
    fn odd_turns_swap_the_stored_spacings() {
        let origin = Vector2::new(1000.0, 3000.0);
        let even =
            ParallelCamera::for_film(&geometry(2, ImageRight::PatientLeft), [2000, 4000], origin);
        let odd =
            ParallelCamera::for_film(&geometry(3, ImageRight::PatientLeft), [4000, 2000], origin);
        assert_eq!(even.map(|c| c.upright_mm_per_px), Some([0.4, 0.5]));
        assert_eq!(odd.map(|c| c.upright_mm_per_px), Some([0.5, 0.4]));
    }

    #[test]
    fn an_unknown_scale_or_facing_gives_no_camera() {
        let origin = Vector2::zeros();
        let no_scale = FilmGeometry {
            mm_per_px: None,
            ..geometry(0, ImageRight::Anterior)
        };
        let no_facing = FilmGeometry {
            image_right: None,
            ..geometry(0, ImageRight::Anterior)
        };
        let zero = FilmGeometry {
            mm_per_px_vertical: Some(0.0),
            ..geometry(0, ImageRight::Anterior)
        };
        for g in [no_scale, no_facing, zero] {
            assert_eq!(ParallelCamera::for_film(&g, [100, 100], origin), None);
        }
    }

    #[test]
    fn the_jacobian_is_the_change_in_the_projection() {
        // Every row of the turn table, every facing: for an affine map the
        // jacobian times a step must equal the change in the landing point.
        let step = Vector3::new(3.0, -7.0, 11.0);
        let from = Vector3::new(-20.0, 40.0, 300.0);
        for turns in 0..4_u8 {
            for right in [
                ImageRight::PatientLeft,
                ImageRight::PatientRight,
                ImageRight::Anterior,
                ImageRight::Posterior,
            ] {
                let size = if turns.is_multiple_of(2) {
                    [2000, 4000]
                } else {
                    [4000, 2000]
                };
                let camera = ParallelCamera::for_film(
                    &geometry(turns, right),
                    size,
                    Vector2::new(1000.0, 3000.0),
                )
                .expect("the geometry is complete");
                let moved = camera.project(&(from + step)) - camera.project(&from);
                assert!((moved - camera.jacobian(&from) * step).norm() < 1e-9);
            }
        }
    }
}
