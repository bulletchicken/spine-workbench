//! Pedicle centres from the vertebra's size: the 1.25, 0.78 and 0.18
//! fractions. Starting guesses only.
//!
//! In the vertebra's own frame, measured from its centre, with `+` to its
//! left:
//!
//! ```text
//! (−1.25 · d̄,  ±0.78 · w̄,  +0.18 · h)      d̄ = (dt + db) / 2,  w̄ = (wt + wb) / 2
//! ```
//!
//! These stand until pedicles are measured from a bone set, so a pedicle
//! placed this way is always marked inferred.

use crate::vertebra_params::VertebraParams;
use nalgebra::{Matrix3, Vector3};

/// How far behind the centre, as a fraction of the mean half-depth.
pub const BEHIND: f64 = 1.25;

/// How far to each side, as a fraction of the mean half-width.
pub const ASIDE: f64 = 0.78;

/// How far above the centre, as a fraction of the height.
pub const ABOVE: f64 = 0.18;

/// The two pedicle centres in the patient frame: the vertebra's left one,
/// then its right one.
pub fn pedicle_centres(vertebra: &VertebraParams) -> [Vector3<f64>; 2] {
    pedicle_centres_turned(vertebra, &vertebra.rotation_matrix())
}

/// The two pedicle centres, given the vertebra's rotation matrix already
/// worked out (see [`VertebraParams::rotation_matrix`]).
pub fn pedicle_centres_turned(
    vertebra: &VertebraParams,
    rotation: &Matrix3<f64>,
) -> [Vector3<f64>; 2] {
    let mean_depth = 0.5 * (vertebra.top_half_depth + vertebra.bottom_half_depth);
    let mean_width = 0.5 * (vertebra.top_half_width + vertebra.bottom_half_width);
    let back = -BEHIND * mean_depth;
    let aside = ASIDE * mean_width;
    let up = ABOVE * vertebra.height;
    [
        vertebra.centre + rotation * Vector3::new(back, aside, up),
        vertebra.centre + rotation * Vector3::new(back, -aside, up),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pedicles_sit_behind_beside_and_above_the_centre() {
        // Worked by hand: d̄ = (10 + 12)/2 = 11, w̄ = (15 + 17)/2 = 16, h = 20,
        // so the offsets are (−13.75, ±12.48, 3.6).
        let v = VertebraParams {
            centre: Vector3::new(1.0, 2.0, 3.0),
            rotation: Vector3::zeros(),
            height: 20.0,
            wedge_front: 0.0,
            wedge_side: 0.0,
            top_half_width: 15.0,
            top_half_depth: 10.0,
            bottom_half_width: 17.0,
            bottom_half_depth: 12.0,
        };
        let [left, right] = pedicle_centres(&v);
        assert!((left - Vector3::new(1.0 - 13.75, 2.0 + 12.48, 3.0 + 3.6)).norm() < 1e-13);
        assert!((right - Vector3::new(1.0 - 13.75, 2.0 - 12.48, 3.0 + 3.6)).norm() < 1e-13);
    }
}
