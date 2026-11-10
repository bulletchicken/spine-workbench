//! Endplate centres, the half-wedge tilt of each endplate, and the rim ovals
//! the corners are taken from.
//!
//! In the vertebra's own frame, with `ẑ` its up axis, the endplate centres
//! sit at `±h/2 · ẑ` and each endplate takes half of each wedge, the top
//! one way and the bottom the other:
//!
//! ```text
//! top normal     n_t = rotX(+ωf/2) · rotY(+ωs/2) · ẑ
//! bottom normal  n_b = rotX(−ωf/2) · rotY(−ωs/2) · ẑ
//! ```
//!
//! Both point up. An endplate's in-plane axes are
//! `ê_w = normalise(ŷ − (ŷ · n) n)`, across toward the vertebra's left, and
//! `ê_d = n × ê_w`, front to back. Its rim is the ellipse with half-axes `w`
//! along `ê_w` and `d` along `ê_d`. Everything is then turned into the
//! patient frame by the vertebra's rotation.

use crate::vertebra_params::VertebraParams;
use nalgebra::{Matrix3, Vector3};

/// One endplate in the patient frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Endplate {
    pub centre: Vector3<f64>,
    /// Unit normal, pointing up out of the top endplate and up into the
    /// bottom one.
    pub normal: Vector3<f64>,
    /// Unit in-plane axis across the endplate, toward the vertebra's left.
    pub width_axis: Vector3<f64>,
    /// Unit in-plane axis front to back: `normal × width_axis`.
    pub depth_axis: Vector3<f64>,
    pub half_width: f64,
    pub half_depth: f64,
}

impl Endplate {
    /// The rim point at angle `t`:
    /// `centre + half_width · cos t · width_axis + half_depth · sin t · depth_axis`.
    pub fn rim_point(&self, t: f64) -> Vector3<f64> {
        self.centre
            + self.width_axis * (self.half_width * libm::cos(t))
            + self.depth_axis * (self.half_depth * libm::sin(t))
    }
}

/// A vertebra's two endplates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Endplates {
    pub top: Endplate,
    pub bottom: Endplate,
}

impl Endplates {
    /// Both endplates of `vertebra`, in the patient frame.
    pub fn of(vertebra: &VertebraParams) -> Self {
        Self::turned(vertebra, &vertebra.rotation_matrix())
    }

    /// Both endplates, given the vertebra's rotation matrix already worked
    /// out (see [`VertebraParams::rotation_matrix`]).
    pub fn turned(vertebra: &VertebraParams, rotation: &Matrix3<f64>) -> Self {
        let (sin_f, cos_f) = half_angle(vertebra.wedge_front);
        let (sin_s, cos_s) = half_angle(vertebra.wedge_side);
        // rotX(θ) · rotY(φ) · ẑ = (sin φ, −sin θ cos φ, cos θ cos φ); the
        // bottom takes −θ and −φ.
        let top_normal = Vector3::new(sin_s, -sin_f * cos_s, cos_f * cos_s);
        let bottom_normal = Vector3::new(-sin_s, sin_f * cos_s, cos_f * cos_s);
        let up = rotation.column(2) * (0.5 * vertebra.height);
        Endplates {
            top: endplate(
                vertebra.centre + up,
                &top_normal,
                rotation,
                vertebra.top_half_width,
                vertebra.top_half_depth,
            ),
            bottom: endplate(
                vertebra.centre - up,
                &bottom_normal,
                rotation,
                vertebra.bottom_half_width,
                vertebra.bottom_half_depth,
            ),
        }
    }
}

/// Sine and cosine of half an angle.
fn half_angle(angle: f64) -> (f64, f64) {
    let half = 0.5 * angle;
    (libm::sin(half), libm::cos(half))
}

/// An endplate from its normal in the vertebra's own frame. With `n` a
/// unit vector, `|ŷ − n_y n|² = 1 − n_y²`, so the width axis needs no
/// general normalisation.
fn endplate(
    centre: Vector3<f64>,
    normal: &Vector3<f64>,
    rotation: &Matrix3<f64>,
    half_width: f64,
    half_depth: f64,
) -> Endplate {
    let ny = normal.y;
    let width_axis = (Vector3::y() - normal * ny) / (1.0 - ny * ny).sqrt();
    let depth_axis = normal.cross(&width_axis);
    Endplate {
        centre,
        normal: rotation * normal,
        width_axis: rotation * width_axis,
        depth_axis: rotation * depth_axis,
        half_width,
        half_depth,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rotation::{rot_x, rot_y};

    fn straight() -> VertebraParams {
        VertebraParams {
            centre: Vector3::new(0.0, 10.0, 100.0),
            rotation: Vector3::zeros(),
            height: 20.0,
            wedge_front: 0.0,
            wedge_side: 0.0,
            top_half_width: 15.0,
            top_half_depth: 10.0,
            bottom_half_width: 16.0,
            bottom_half_depth: 11.0,
        }
    }

    #[test]
    fn a_straight_vertebra_has_level_endplates_h_apart() {
        let e = Endplates::of(&straight());
        assert_eq!(e.top.centre, Vector3::new(0.0, 10.0, 110.0));
        assert_eq!(e.bottom.centre, Vector3::new(0.0, 10.0, 90.0));
        for plate in [e.top, e.bottom] {
            assert_eq!(plate.normal, Vector3::z());
            assert_eq!(plate.width_axis, Vector3::y());
            // ẑ × ŷ = −x̂: front to back.
            assert_eq!(plate.depth_axis, -Vector3::x());
        }
        assert_eq!((e.top.half_width, e.top.half_depth), (15.0, 10.0));
        assert_eq!((e.bottom.half_width, e.bottom.half_depth), (16.0, 11.0));
    }

    #[test]
    fn the_top_normal_is_rot_x_after_rot_y() {
        // Worked by hand with ωf/2 = ωs/2 = 30°:
        // rotY(30°)ẑ = (½, 0, √3/2); rotX(30°) of that = (½, −√3/4, ¾).
        // The other order would give (√3/4, −½, ¾).
        let v = VertebraParams {
            wedge_front: core::f64::consts::FRAC_PI_3,
            wedge_side: core::f64::consts::FRAC_PI_3,
            ..straight()
        };
        let e = Endplates::of(&v);
        let expected_top = Vector3::new(0.5, -(3.0_f64.sqrt()) / 4.0, 0.75);
        let expected_bottom = Vector3::new(-0.5, 3.0_f64.sqrt() / 4.0, 0.75);
        assert!((e.top.normal - expected_top).norm() < 1e-15);
        assert!((e.bottom.normal - expected_bottom).norm() < 1e-15);
        let composed = rot_x(v.wedge_front / 2.0) * rot_y(v.wedge_side / 2.0) * Vector3::z();
        assert!((e.top.normal - composed).norm() < 1e-15);
    }

    #[test]
    fn every_endplate_frame_is_right_handed_and_unit() {
        let v = VertebraParams {
            rotation: Vector3::new(0.3, -0.7, 1.2),
            wedge_front: 0.4,
            wedge_side: -0.3,
            ..straight()
        };
        let e = Endplates::of(&v);
        for plate in [e.top, e.bottom] {
            for axis in [plate.normal, plate.width_axis, plate.depth_axis] {
                assert!((axis.norm() - 1.0).abs() < 1e-15);
            }
            assert!(plate.normal.dot(&plate.width_axis).abs() < 1e-15);
            assert!((plate.normal.cross(&plate.width_axis) - plate.depth_axis).norm() < 1e-15);
        }
        // The centres stay h apart along the vertebra's own up axis.
        let between = e.top.centre - e.bottom.centre;
        assert!((between - v.rotation_matrix().column(2) * v.height).norm() < 1e-13);
    }

    #[test]
    fn rim_points_lie_on_the_endplate_plane_at_the_half_sizes() {
        let v = VertebraParams {
            rotation: Vector3::new(-0.2, 0.5, 0.1),
            wedge_front: 0.2,
            ..straight()
        };
        let top = Endplates::of(&v).top;
        assert!((top.rim_point(0.0) - (top.centre + top.width_axis * 15.0)).norm() < 1e-13);
        let quarter = top.rim_point(core::f64::consts::FRAC_PI_2);
        assert!((quarter - (top.centre + top.depth_axis * 10.0)).norm() < 1e-13);
        for t in [0.3, 1.7, 4.0] {
            assert!((top.rim_point(t) - top.centre).dot(&top.normal).abs() < 1e-13);
        }
    }
}
