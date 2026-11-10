//! The thirteen numbers that describe one vertebra (centre, rotation vector,
//! height, the two wedges and the four endplate half-sizes) and the sign of
//! each.
//!
//! | Numbers | Field | Meaning |
//! |---|---|---|
//! | 3 | `centre` | the midpoint between the two endplate centres, mm |
//! | 3 | `rotation` | rotation vector taking the vertebra's own axes to the patient's, rad |
//! | 1 | `height` | the distance between the two endplate centres, mm |
//! | 1 | `wedge_front` | front-view wedge, rad: **positive when the patient's left side is taller** |
//! | 1 | `wedge_side` | side-view wedge, rad: **positive when the front is shorter** |
//! | 2 | `top_half_width`, `top_half_depth` | the upper endplate's half-sizes, mm |
//! | 2 | `bottom_half_width`, `bottom_half_depth` | the lower endplate's half-sizes, mm |
//!
//! The vertebra's own axes follow the patient's pattern: its x forward, its
//! y to its left, its z up along the line between its endplate centres.
//! See `docs/conventions/signs-and-wedges.md`.

use crate::rotation::matrix_from_rotation_vector;
use nalgebra::{Matrix3, Vector3};
use shapes::levels::Level;
use shapes::made_up::VertebraPose;

/// One vertebra as thirteen numbers, in the patient frame (mm and radians).
///
/// # Allowed values
///
/// Every number finite; the height and the four half-sizes greater than
/// zero; each wedge strictly between −π and π (a real wedge is a few
/// tenths of a radian). Rotation components may take any finite value.
///
/// Nothing that projects a vertebra checks these: a fit calls the model
/// many thousands of times with numbers it keeps in range itself. Outside
/// them a projection still returns `Ok`, with marks that mean nothing:
///
/// - a NaN or infinite number leaves some of the marks NaN or infinite;
/// - a negative height puts the top endplate below the bottom one, so the
///   `sup` and `inf` marks trade places;
/// - on a vertebra with no turn and no wedge, a zero half-width gives that
///   endplate NaN corners on a front or bending film, and a zero
///   half-depth NaN corners on a side film (the rim has no extent along
///   the axis that names its corners);
/// - a front wedge of ±π with no side wedge tips both endplates on their
///   sides, leaving no width axis, and gives NaN corners.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VertebraParams {
    /// The midpoint between the two endplate centres.
    pub centre: Vector3<f64>,
    /// Rotation vector (axis times angle) taking the vertebra's own axes to
    /// the patient's.
    pub rotation: Vector3<f64>,
    /// The distance between the two endplate centres.
    pub height: f64,
    /// Front-view wedge: positive when the patient's left side is taller.
    pub wedge_front: f64,
    /// Side-view wedge: positive when the front is shorter.
    pub wedge_side: f64,
    pub top_half_width: f64,
    pub top_half_depth: f64,
    pub bottom_half_width: f64,
    pub bottom_half_depth: f64,
}

impl VertebraParams {
    /// How many numbers describe a vertebra.
    pub const COUNT: usize = 13;

    /// The matrix taking the vertebra's own axes to the patient's: its
    /// columns are the vertebra's forward, left and up directions.
    pub fn rotation_matrix(&self) -> Matrix3<f64> {
        matrix_from_rotation_vector(&self.rotation)
    }

    /// The thirteen numbers in a fixed order: centre x, y, z; rotation
    /// x, y, z; height; front wedge; side wedge; top half-width, top
    /// half-depth; bottom half-width, bottom half-depth.
    pub fn to_numbers(&self) -> [f64; Self::COUNT] {
        [
            self.centre.x,
            self.centre.y,
            self.centre.z,
            self.rotation.x,
            self.rotation.y,
            self.rotation.z,
            self.height,
            self.wedge_front,
            self.wedge_side,
            self.top_half_width,
            self.top_half_depth,
            self.bottom_half_width,
            self.bottom_half_depth,
        ]
    }

    /// The vertebra from thirteen numbers in the order of
    /// [`to_numbers`](Self::to_numbers).
    pub fn from_numbers(n: [f64; Self::COUNT]) -> Self {
        VertebraParams {
            centre: Vector3::new(n[0], n[1], n[2]),
            rotation: Vector3::new(n[3], n[4], n[5]),
            height: n[6],
            wedge_front: n[7],
            wedge_side: n[8],
            top_half_width: n[9],
            top_half_depth: n[10],
            bottom_half_width: n[11],
            bottom_half_depth: n[12],
        }
    }

    /// This vertebra as the saved pose of `level`.
    pub fn pose(&self, level: Level) -> VertebraPose {
        VertebraPose {
            level,
            centre: self.centre.into(),
            rotation: self.rotation.into(),
            height: self.height,
            wedge_front: self.wedge_front,
            wedge_side: self.wedge_side,
            top_half_width: self.top_half_width,
            top_half_depth: self.top_half_depth,
            bottom_half_width: self.bottom_half_width,
            bottom_half_depth: self.bottom_half_depth,
        }
    }
}

impl From<&VertebraPose> for VertebraParams {
    fn from(pose: &VertebraPose) -> Self {
        VertebraParams {
            centre: pose.centre.into(),
            rotation: pose.rotation.into(),
            height: pose.height,
            wedge_front: pose.wedge_front,
            wedge_side: pose.wedge_side,
            top_half_width: pose.top_half_width,
            top_half_depth: pose.top_half_depth,
            bottom_half_width: pose.bottom_half_width,
            bottom_half_depth: pose.bottom_half_depth,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> VertebraParams {
        VertebraParams::from_numbers([
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0,
        ])
    }

    #[test]
    fn the_numbers_keep_their_order() {
        let v = sample();
        assert_eq!(v.centre, Vector3::new(1.0, 2.0, 3.0));
        assert_eq!(v.rotation, Vector3::new(4.0, 5.0, 6.0));
        assert_eq!((v.height, v.wedge_front, v.wedge_side), (7.0, 8.0, 9.0));
        assert_eq!((v.top_half_width, v.top_half_depth), (10.0, 11.0));
        assert_eq!((v.bottom_half_width, v.bottom_half_depth), (12.0, 13.0));
        assert_eq!(VertebraParams::from_numbers(v.to_numbers()), v);
    }

    #[test]
    fn a_saved_pose_reads_back_unchanged() {
        let v = sample();
        let pose = v.pose(Level::T7);
        assert_eq!(pose.level, Level::T7);
        assert_eq!(pose.centre, [1.0, 2.0, 3.0]);
        assert_eq!(pose.rotation, [4.0, 5.0, 6.0]);
        assert_eq!(VertebraParams::from(&pose), v);
    }
}
