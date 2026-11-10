//! Cameras: how a point in the patient frame lands on a film, in original film
//! pixels. Wave 1 has the parallel camera; the point-source and slot-scanner
//! cameras of Wave 2 sit beside it.
//!
//! The camera formulas are written for the film shown upright, in upright
//! pixels, whose width and height in mm swap with the stored spacings when
//! the film's quarter turns are odd. Each camera turns its answer back to
//! original film pixels before returning it: see `shapes::film_geometry`.
//!
//! Everything that projects a vertebra takes any [`Camera`], so a new kind
//! of camera changes no caller. A camera answers three questions: where a
//! point lands, how that landing moves with the point (for fits and for
//! the long direction of an endplate), and where an endplate's two
//! corners land. The last is the camera's own because only an affine
//! projection keeps the rim's centre at the centre of its outline: the
//! parallel camera answers it in closed form
//! ([`corners::affine_corners`](crate::corners::affine_corners)), and a
//! point-source camera must find the ends of its projected rim itself.

pub mod parallel;

use crate::corners::{AxisAlongBeam, CornerAxis};
use crate::endplates::Endplate;
use nalgebra::{Matrix2x3, Vector2, Vector3};

/// Patient millimetres to original film pixels.
pub trait Camera {
    /// Where `point` lands on the film, in original film pixels (x right,
    /// y down, the centre of the top-left pixel at the origin).
    fn project(&self, point: &Vector3<f64>) -> Vector2<f64>;

    /// How the landing point moves as `point` moves: the derivative of
    /// [`project`](Self::project), in film pixels per patient millimetre.
    fn jacobian(&self, point: &Vector3<f64>) -> Matrix2x3<f64>;

    /// Where `endplate`'s two corners land, in the order `axis` names: the
    /// ends of the endplate rim's outline on this film along the
    /// endplate's long direction in the view.
    fn corners(
        &self,
        endplate: &Endplate,
        axis: CornerAxis,
    ) -> Result<[Vector2<f64>; 2], AxisAlongBeam>;
}
