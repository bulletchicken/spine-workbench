//! The closed-form corner: where an endplate rim touches its outline on a
//! film, `t = atan2(d·B, d·A)` and the corners at `c ± (A cos t + B sin t)`.
//!
//! An endplate's rim is an ellipse. Under a projection whose local linear
//! part is `P`, it lands on the film as the ellipse
//! `c′ + A cos t + B sin t`, with `c′` the projected centre,
//! `A = P(w · ê_w)` and `B = P(d · ê_d)`. A corner of the vertebral body on
//! that film is an end of this ellipse along the endplate's long direction
//! in the view, `d̂`: the rim point where `d̂ · (A cos t + B sin t)` is
//! largest (one corner) or smallest (the other). That is
//!
//! ```text
//! t* = atan2(d̂ · B, d̂ · A)
//! corners = c′ ± (A cos t* + B sin t*)
//! ```
//!
//! Front and bending films take the long direction across the endplate,
//! `d̂ = P ê_w`, so the `+` corner is the patient's left one; side films take
//! it front to back, `d̂ = −P ê_d`, so the `+` corner is the anterior one
//! (`ê_d` points to the back). Neither choice needs a branch: the `+`
//! corner's offset from `c′` has `d̂ · offset = |(d̂ · A, d̂ · B)| ≥ 0`, so
//! it is always the end furthest along `d̂` and keeps its name however the
//! vertebra turns. For an unturned vertebra on a front view, `d̂ · B = 0`
//! and `d̂ · A = w |P ê_w|² > 0`, so `t* = 0` and the `+` corner is the end
//! of `A`, the left one. On a side view, `d̂ · A = 0` and
//! `d̂ · B = −d |P ê_d|² < 0`, so `t* = −π/2` and the `+` corner is the end
//! of `−B`, the anterior one. A test checks the closed form
//! against a brute-force walk around the rim
//! (`tests/corners_match_a_rim_walk.rs`).

use crate::endplates::Endplate;
use core::fmt;
use nalgebra::{Matrix2x3, Vector2, Vector3};

/// Which way along an endplate its two corners on a film lie, and the
/// order they come in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CornerAxis {
    /// Across the endplate: patient left, then patient right. Front and
    /// bending films.
    LeftRight,
    /// Front to back: anterior, then posterior. Side films.
    FrontBack,
}

impl CornerAxis {
    /// The unit direction in the endplate's plane that points to the first
    /// corner: the width axis (to the vertebra's left), or the depth axis
    /// reversed (to its front).
    pub fn toward_first(self, endplate: &Endplate) -> Vector3<f64> {
        match self {
            CornerAxis::LeftRight => endplate.width_axis,
            CornerAxis::FrontBack => -endplate.depth_axis,
        }
    }
}

/// The endplate axis that names the corners points straight along the
/// beam, so the film shows no ends along it. A vertebra turned a quarter
/// turn about its own up axis, seen on a front film, is the one way this
/// happens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AxisAlongBeam;

impl fmt::Display for AxisAlongBeam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the endplate axis that names the corners points along the beam")
    }
}

impl std::error::Error for AxisAlongBeam {}

/// The two ends of the ellipse `centre + a cos t + b sin t` along `long`:
/// first the end furthest along `long`, then the one furthest against it.
///
/// `cos t*` and `sin t*` are taken as `(d̂·A, d̂·B) / |(d̂·A, d̂·B)|`, which
/// is `atan2(d̂·B, d̂·A)` without the transcendental calls; `long` need
/// not be a unit vector, since `t*` does not depend on its length. The
/// ellipse must have some extent along `long` (`d̂·A` and `d̂·B` not both
/// zero).
pub fn closed_form(
    centre: Vector2<f64>,
    a: Vector2<f64>,
    b: Vector2<f64>,
    long: Vector2<f64>,
) -> [Vector2<f64>; 2] {
    let (along_a, along_b) = (long.dot(&a), long.dot(&b));
    let reach = (along_a * along_a + along_b * along_b).sqrt();
    let offset = (a * along_a + b * along_b) / reach;
    [centre + offset, centre - offset]
}

/// An endplate's two corners on a film whose projection near the endplate
/// is `centre_px + jacobian · (p − endplate.centre)`. Exact for any camera
/// whose projection is affine, such as the parallel camera.
///
/// The endplate's half-sizes must be positive. Fails when the axis that
/// names the corners projects to less than `1e-9` of the jacobian's size
/// (its Frobenius norm): the endplate is then seen end-on along that axis
/// and its corners are undefined.
pub fn affine_corners(
    endplate: &Endplate,
    centre_px: Vector2<f64>,
    jacobian: &Matrix2x3<f64>,
    axis: CornerAxis,
) -> Result<[Vector2<f64>; 2], AxisAlongBeam> {
    let long = jacobian * axis.toward_first(endplate);
    if long.norm() <= 1e-9 * jacobian.norm() {
        return Err(AxisAlongBeam);
    }
    let a = jacobian * (endplate.width_axis * endplate.half_width);
    let b = jacobian * (endplate.depth_axis * endplate.half_depth);
    Ok(closed_form(centre_px, a, b, long))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_upright_ellipse_ends_at_its_long_half_axis() {
        // Worked by hand: A = (30, 0), B = (0, 10), d̂ = (1, 0) gives t* = 0
        // and the corners (130, 50) and (70, 50).
        let corners = closed_form(
            Vector2::new(100.0, 50.0),
            Vector2::new(30.0, 0.0),
            Vector2::new(0.0, 10.0),
            Vector2::new(1.0, 0.0),
        );
        assert_eq!(
            corners,
            [Vector2::new(130.0, 50.0), Vector2::new(70.0, 50.0)]
        );
    }

    #[test]
    fn a_turned_ellipse_ends_at_its_widest_reach() {
        // Worked by hand: A = (3, 4), B = (−8, 6) (perpendicular, |A| = 5,
        // |B| = 10), d̂ = (1, 0). d̂·A = 3, d̂·B = −8, so cos t* = 3/√73,
        // sin t* = −8/√73 and the offset is (9 + 64, 12 − 48)/√73 =
        // (73, −36)/√73. Its x is √73, the ellipse's half-extent along x:
        // √(3² + 8²).
        let corners = closed_form(
            Vector2::zeros(),
            Vector2::new(3.0, 4.0),
            Vector2::new(-8.0, 6.0),
            Vector2::new(2.0, 0.0),
        );
        let root = 73.0_f64.sqrt();
        let expected = Vector2::new(73.0 / root, -36.0 / root);
        assert!((corners[0] - expected).norm() < 1e-14);
        assert!((corners[1] + expected).norm() < 1e-14);
    }

    #[test]
    fn an_axis_along_the_beam_has_no_corners() {
        let plate = Endplate {
            centre: Vector3::zeros(),
            normal: Vector3::z(),
            width_axis: Vector3::x(),
            depth_axis: Vector3::y(),
            half_width: 15.0,
            half_depth: 10.0,
        };
        // A front film: across is y, up is z, x is the beam.
        let front = Matrix2x3::new(0.0, 2.0, 0.0, 0.0, 0.0, -2.0);
        assert_eq!(
            affine_corners(&plate, Vector2::zeros(), &front, CornerAxis::LeftRight),
            Err(AxisAlongBeam)
        );
        assert!(affine_corners(&plate, Vector2::zeros(), &front, CornerAxis::FrontBack).is_ok());
    }
}
