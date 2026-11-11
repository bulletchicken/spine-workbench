//! Rounded vertebral bodies and pedicle rings, in the painter's own shape.
//!
//! A body is the solid between its two endplate planes and inside an
//! elliptic frustum along its own axis, whose cross-section grows evenly
//! from the lower endplate's size to the upper's, with its edges rounded
//! off by an ellipsoid that trims about 1 mm from each rim corner. Its rims
//! therefore lie on the endplate ellipses (to within the frustum's
//! straight sides), but its outline is not the solver's: the corners are
//! rounded, the walls are cortical bone round a trabecular core, and the
//! posterior elements overlap it.
//!
//! Pedicles are hollow elliptic tubes running front to back, converging
//! toward the midline going forward (8° in the neck and chest, rising from
//! 10° at L1 to 26° at L5), centred on the forward model's pedicle points.
//! On a front film a pedicle seen along its axis shows as a ring.
//!
//! Behind them each vertebra has a lamina (a flattened ellipsoid), a
//! spinous process pointing back and, in the chest, steeply down, and two
//! transverse processes: they overlap the body on a front film, as on real
//! films.

use super::solids::{Capsule, Ellipsoid, Frustum, HalfSpace, Solid};
use super::{CORTEX, TRABECULAR};
use crate::body_sizes::VertebraSize;
use forward_model::endplates::Endplates;
use forward_model::pedicles::pedicle_centres_turned;
use forward_model::vertebra_params::VertebraParams;
use nalgebra::{Matrix3, Vector3};
use shapes::levels::{Level, Region};

/// How far the rounding trims each rim corner, as the ellipsoid's
/// half-lengths over the body's: it passes through the endplate 5% in
/// from the rim and through the wall 10% down from the endplate, about
/// 1 mm on an adult body.
const ROUND_ACROSS: f64 = 1.19;
const ROUND_ALONG: f64 = 1.66;

/// Cortical bone thickness, mm on an adult: walls, then endplates and
/// pedicles.
const WALL: f64 = 1.0;
const ENDPLATE: f64 = 0.8;

/// One pedicle as painted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PedicleTube {
    /// Its centre: the forward model's pedicle point.
    pub centre: Vector3<f64>,
    /// Unit direction of its axis, from back to front.
    pub axis: Vector3<f64>,
    /// Unit direction across it, toward the patient's left as the
    /// vertebra faces.
    pub across: Vector3<f64>,
    pub half_width: f64,
    pub half_height: f64,
    pub length: f64,
}

/// How far a pedicle's axis turns toward the midline going forward.
pub fn convergence(level: Level) -> f64 {
    let degrees = match level.region() {
        Region::Lumbar => {
            let lumbar_index = level.order() - Level::L1.order();
            10.0 + 4.0 * lumbar_index.min(4) as f64
        }
        _ => 8.0,
    };
    degrees.to_radians()
}

/// The vertebra's two pedicles, its left one first.
pub fn pedicles(vertebra: &VertebraParams, level: Level, size: &VertebraSize) -> [PedicleTube; 2] {
    let rotation = vertebra.rotation_matrix();
    let centres = pedicle_centres_turned(vertebra, &rotation);
    let mean_depth = 0.5 * (vertebra.top_half_depth + vertebra.bottom_half_depth);
    let turn = convergence(level);
    let (sin, cos) = (libm::sin(turn), libm::cos(turn));
    let tube = |centre: Vector3<f64>, side: f64| {
        // Forward and toward the midline: the left pedicle (side +1)
        // heads to the vertebra's right going forward.
        let axis = rotation * Vector3::new(cos, -side * sin, 0.0);
        let across = rotation * Vector3::new(side * sin, cos, 0.0);
        PedicleTube {
            centre,
            axis,
            across,
            half_width: 0.5 * size.pedicle_width,
            half_height: 0.5 * size.pedicle_height,
            length: 0.8 * mean_depth,
        }
    };
    [tube(centres[0], 1.0), tube(centres[1], -1.0)]
}

/// The solids of one vertebra and the attenuation each adds, per mm, over
/// soft tissue: the body's cortex and core, the pedicles, the lamina and
/// the processes. `k` scales the cortex with the spine.
pub fn vertebra_solids(
    vertebra: &VertebraParams,
    level: Level,
    size: &VertebraSize,
    k: f64,
) -> Vec<(Solid, f64)> {
    let rotation = vertebra.rotation_matrix();
    let plates = Endplates::turned(vertebra, &rotation);
    let depth = 0.5 * (vertebra.top_half_depth + vertebra.bottom_half_depth);
    let width = 0.5 * (vertebra.top_half_width + vertebra.bottom_half_width);
    let h = vertebra.height;
    let taper = 2.0 * (vertebra.top_half_width - vertebra.bottom_half_width)
        / (h * (vertebra.top_half_width + vertebra.bottom_half_width));
    let top = HalfSpace::behind(plates.top.centre, plates.top.normal);
    let bottom = HalfSpace::behind(plates.bottom.centre, -plates.bottom.normal);
    let most = |a: f64, b: f64| a.max(b);
    let trim = Ellipsoid {
        centre: vertebra.centre,
        axes: rotation,
        radii: Vector3::new(
            ROUND_ACROSS * most(vertebra.top_half_depth, vertebra.bottom_half_depth),
            ROUND_ACROSS * most(vertebra.top_half_width, vertebra.bottom_half_width),
            ROUND_ALONG * 0.5 * h,
        ),
    };
    let half_length = 0.5 * h + 0.3 * width;
    let body = Frustum {
        centre: vertebra.centre,
        axes: rotation,
        half: [depth, width],
        taper,
        faces: vec![top, bottom],
        trim: Some(trim),
        half_length,
    };
    let (wall, endplate) = (WALL * k, ENDPLATE * k);
    let core = Frustum {
        half: [depth - wall, width - wall],
        faces: vec![top.moved_in(endplate), bottom.moved_in(endplate)],
        trim: Some(trim.shrunk(wall)),
        ..body.clone()
    };
    let mut solids = vec![
        (Solid::Frustum(body), CORTEX),
        (Solid::Frustum(core), TRABECULAR - CORTEX),
    ];
    for tube in pedicles(vertebra, level, size) {
        let up = tube.axis.cross(&tube.across);
        let axes = Matrix3::from_columns(&[tube.across, up, tube.axis]);
        let ends = |shrink: f64| {
            vec![
                HalfSpace::behind(tube.centre + tube.axis * (0.5 * tube.length), tube.axis),
                HalfSpace::behind(tube.centre - tube.axis * (0.5 * tube.length), -tube.axis)
                    .moved_in(shrink),
            ]
        };
        let outer = Frustum {
            centre: tube.centre,
            axes,
            half: [tube.half_width, tube.half_height],
            taper: 0.0,
            faces: ends(0.0),
            trim: None,
            half_length: 0.5 * tube.length,
        };
        let inner = Frustum {
            half: [tube.half_width - wall, tube.half_height - wall],
            faces: ends(endplate),
            ..outer.clone()
        };
        solids.push((Solid::Frustum(outer), CORTEX));
        solids.push((Solid::Frustum(inner), TRABECULAR - CORTEX));
    }
    let local = |x: f64, y: f64, z: f64| vertebra.centre + rotation * Vector3::new(x, y, z);
    let region = level.region();
    let behind_pedicles = -1.25 * depth - 0.4 * depth - 2.0 * k;
    solids.push((
        Solid::Ellipsoid(Ellipsoid {
            centre: local(behind_pedicles, 0.0, 0.1 * h),
            axes: rotation,
            radii: Vector3::new(3.0 * k, 0.75 * width, 0.45 * h),
        }),
        0.7 * CORTEX,
    ));
    let (spine_down, spine_long, spine_thick) = match region {
        Region::Cervical => (20f64, 1.2, 2.5),
        Region::Thoracic => (50.0, 1.6, 3.0),
        Region::Lumbar => (10.0, 1.3, 4.0),
    };
    let tilt = spine_down.to_radians();
    let length = spine_long * depth;
    solids.push((
        Solid::Capsule(Capsule {
            start: local(behind_pedicles, 0.0, 0.1 * h),
            end: local(
                behind_pedicles - length * libm::cos(tilt),
                0.0,
                0.1 * h - length * libm::sin(tilt),
            ),
            radius: spine_thick * k,
        }),
        0.6 * CORTEX,
    ));
    let (reach, back, thick) = match region {
        Region::Cervical => (0.6, 0.0, 2.5),
        Region::Thoracic => (0.9, 0.35, 3.0),
        Region::Lumbar => (1.1, 0.0, 3.5),
    };
    for side in [1.0, -1.0] {
        solids.push((
            Solid::Capsule(Capsule {
                start: local(-1.45 * depth, side * 0.8 * width, 0.1 * h),
                end: local(
                    -(1.45 + back) * depth,
                    side * (0.8 + reach) * width,
                    0.1 * h,
                ),
                radius: thick * k,
            }),
            0.6 * CORTEX,
        ));
    }
    solids
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body_sizes::base_sizes;

    fn upright_l3() -> (VertebraParams, VertebraSize) {
        let size = base_sizes(&[Level::L3], 1.0)[0];
        let vertebra = VertebraParams {
            centre: Vector3::new(0.0, 0.0, 100.0),
            rotation: Vector3::zeros(),
            height: size.height,
            wedge_front: 0.0,
            wedge_side: 0.0,
            top_half_width: size.top_half_width,
            top_half_depth: size.top_half_depth,
            bottom_half_width: size.bottom_half_width,
            bottom_half_depth: size.bottom_half_depth,
        };
        (vertebra, size)
    }

    #[test]
    fn pedicles_sit_behind_the_body_and_converge_forward() {
        let (vertebra, size) = upright_l3();
        let [left, right] = pedicles(&vertebra, Level::L3, &size);
        assert!(left.centre.y > 0.0 && right.centre.y < 0.0);
        assert!(left.centre.x < -vertebra.top_half_depth);
        // L3 converges 18°: going forward the left pedicle heads right.
        assert!(left.axis.x > 0.0 && left.axis.y < 0.0 && right.axis.y > 0.0);
        assert!((libm::atan2(-left.axis.y, left.axis.x).to_degrees() - 18.0).abs() < 1e-12);
        assert!(
            (left.axis.norm() - 1.0).abs() < 1e-15 && left.axis.dot(&left.across).abs() < 1e-15
        );
        assert!(left.across.y > 0.0 && right.across.y > 0.0);
    }

    #[test]
    fn the_body_fills_its_endplates_with_rounded_corners() {
        use super::super::solids::Ray;
        let (vertebra, size) = upright_l3();
        let solids = vertebra_solids(&vertebra, Level::L3, &size, 1.0);
        let (body, _) = &solids[0];
        let along = |y: f64, z: f64| {
            body.chord(&Ray {
                origin: Vector3::new(-200.0, y, z),
                direction: Vector3::x(),
            })
        };
        let z_top = 100.0 + 0.5 * vertebra.height;
        // Through the middle the beam crosses the full depth, near enough
        // (the frustum is a little deeper than the mean at mid-height).
        let mean_depth = vertebra.top_half_depth + vertebra.bottom_half_depth;
        assert!((along(0.0, 100.0) - mean_depth).abs() < 0.01 * mean_depth);
        // Just inside the upper endplate in the middle: inside.
        assert!(along(0.0, z_top - 0.2) > 30.0);
        // Above the endplate: outside.
        assert_eq!(along(0.0, z_top + 0.2), 0.0);
        // At the rim corner the rounding has trimmed the body away, but a
        // millimetre and a half in from both edges it is there.
        let w = vertebra.top_half_width;
        assert_eq!(along(w - 0.05, z_top - 0.05), 0.0);
        assert!(along(w - 1.5, z_top - 1.5) > 0.0);
    }

    #[test]
    fn a_front_beam_through_a_pedicle_crosses_its_wall_as_a_ring() {
        use super::super::solids::Ray;
        // Turned 18° to the left, L3's left pedicle points straight along
        // a front beam.
        let (mut vertebra, size) = upright_l3();
        vertebra.rotation = Vector3::new(0.0, 0.0, 18f64.to_radians());
        let solids = vertebra_solids(&vertebra, Level::L3, &size, 1.0);
        let [left, _] = pedicles(&vertebra, Level::L3, &size);
        assert!((left.axis - Vector3::x()).norm() < 1e-12);
        let pedicle_line = |offset: f64| {
            let ray = Ray {
                origin: left.centre + left.across * offset - Vector3::x() * 200.0,
                direction: Vector3::x(),
            };
            solids[2].1 * solids[2].0.chord(&ray) + solids[3].1 * solids[3].0.chord(&ray)
        };
        // Through the hollow less bone than through the wall: a ring.
        let through_wall = pedicle_line(left.half_width - 0.5);
        let through_middle = pedicle_line(0.0);
        assert!(
            through_wall > 1.5 * through_middle,
            "{through_wall} {through_middle}"
        );
    }

    #[test]
    fn thoracic_pedicles_converge_less_than_lumbar_ones() {
        assert!(convergence(Level::T6) < convergence(Level::L1));
        assert!(convergence(Level::L1) < convergence(Level::L5));
        assert_eq!(convergence(Level::L6), convergence(Level::L5));
    }
}
