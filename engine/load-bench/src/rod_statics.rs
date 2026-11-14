//! Free-rod section loads from head loads, without a constitutive solve.
//!
//! Patient axes and units: x anterior, y patient-left, z up; right-handed;
//! mm, N, N·mm and MPa. Inputs are forces/couples **on the rod**. The force
//! the rod applies to the vertebra is their negative. Pull-out is the
//! projection of that vertebral force opposite a screw's inward axis;
//! push-in has the opposite sign. Right-hand positive x/y/z moments tip an
//! up axis right/forward and turn an anterior axis left, respectively.
//!
//! At a cut, force and moment sum loads on the prefix (head s <= cut s).
//! These are the cut loads the prefix exerts on the suffix. The opposite
//! cut face has opposite signs. A head at the cut has zero force arm but
//! contributes its couple. A free, balanced rod gives the same section
//! loads from the negative suffix sum. Corners use the outgoing tangent,
//! except at the rod's last point. Both sides may be sampled at a corner
//! when distinguishing local bending and torsion matters.

use crate::rod_beams::{RodError, polyline_at, polyline_stations};
use nalgebra::Vector3;

/// This project's tolerance for a supplied section tangent being a unit vector.
const TANGENT_UNIT_TOLERANCE: f64 = 1e-10;

/// Load applied by one head to the rod, at its stated station and point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeadLoad {
    pub station_mm: f64,
    pub position_mm: Vector3<f64>,
    pub force_n: Vector3<f64>,
    pub couple_nmm: Vector3<f64>,
}

/// First-order change of a head's physical point and wrench. Forces and
/// couples are increments ON THE ROD, not the total after seating.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeadIncrement {
    pub point_mm: Vector3<f64>,
    pub force_n: Vector3<f64>,
    pub couple_nmm: Vector3<f64>,
}

/// Movement of the specified material cut and its current unit tangent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SectionIncrement {
    pub point_mm: Vector3<f64>,
    pub tangent_unit: Vector3<f64>,
}

/// Signed section resultants in patient axes. Axial force/torsion follow
/// by projection on `tangent_unit`; normal force is positive in tension.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SectionLoad {
    pub position_mm: Vector3<f64>,
    pub tangent_unit: Vector3<f64>,
    pub force_n: Vector3<f64>,
    pub moment_nmm: Vector3<f64>,
}

/// Section stress magnitudes in N/mm² = MPa. The specified bench envelope
/// adds |N|/A to the bending/torsion von Mises envelope, as in the model's
/// first-order stress rule; this is an envelope, not a fibre location.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceStress {
    /// Signed uniform axial stress, tensile-positive.
    pub axial_mpa: f64,
    pub bending_mpa: f64,
    /// Signed outer-surface torsional shear stress.
    pub torsional_mpa: f64,
    pub von_mises_mpa: f64,
}

/// Sum of the supplied loads and their moments about a patient-frame
/// origin. Used to check a free rod before interpreting its cut loads.
pub fn resultant(heads: &[HeadLoad], origin_mm: Vector3<f64>) -> (Vector3<f64>, Vector3<f64>) {
    let mut force = Vector3::zeros();
    let mut moment = Vector3::zeros();
    for head in heads {
        force += head.force_n;
        moment += (head.position_mm - origin_mm).cross(&head.force_n) + head.couple_nmm;
    }
    (force, moment)
}

/// Prefix statics at a station, with no distributed loads. Head points
/// must be finite; callers may check `resultant` for free-rod balance.
pub fn section_at(
    polyline: &[Vector3<f64>],
    heads: &[HeadLoad],
    station_mm: f64,
) -> Result<SectionLoad, RodError> {
    let stations = polyline_stations(polyline)?;
    let length = stations[stations.len() - 1];
    if heads.iter().any(|head| {
        !head.station_mm.is_finite()
            || head.station_mm < 0.0
            || head.station_mm > length
            || head.position_mm.iter().any(|x| !x.is_finite())
            || head.force_n.iter().any(|x| !x.is_finite())
            || head.couple_nmm.iter().any(|x| !x.is_finite())
    }) {
        return Err(RodError::InvalidHeadStations);
    }
    let (position_mm, tangent_unit) = polyline_at(polyline, station_mm)?;
    let mut force_n = Vector3::zeros();
    let mut moment_nmm = Vector3::zeros();
    for head in heads.iter().filter(|head| head.station_mm <= station_mm) {
        force_n += head.force_n;
        moment_nmm += (head.position_mm - position_mm).cross(&head.force_n) + head.couple_nmm;
    }
    Ok(SectionLoad {
        position_mm,
        tangent_unit,
        force_n,
        moment_nmm,
    })
}

/// First-order statics of moving application points. Retains dP×F0 and
/// omits the second-order dP×dF term. The cut station and reference heads
/// use the state-R polyline; the caller supplies the cut's movement. This
/// is the exact linear moment balance used by the objective elastic step.
pub fn linearised_section_at(
    polyline: &[Vector3<f64>],
    heads: &[HeadLoad],
    increments: &[HeadIncrement],
    station_mm: f64,
    cut: SectionIncrement,
) -> Result<SectionLoad, RodError> {
    if heads.len() != increments.len()
        || increments.iter().any(|d| {
            d.point_mm
                .iter()
                .chain(d.force_n.iter())
                .chain(d.couple_nmm.iter())
                .any(|v| !v.is_finite())
        })
        || cut
            .point_mm
            .iter()
            .chain(cut.tangent_unit.iter())
            .any(|v| !v.is_finite())
        || (cut.tangent_unit.norm_squared() - 1.0).abs() > TANGENT_UNIT_TOLERANCE
    {
        return Err(RodError::InvalidHeadStations);
    }
    let mut section = section_at(polyline, heads, station_mm)?;
    for (head, delta) in heads
        .iter()
        .zip(increments)
        .filter(|(h, _)| h.station_mm <= station_mm)
    {
        section.force_n += delta.force_n;
        section.moment_nmm += (head.position_mm - section.position_mm).cross(&delta.force_n)
            + (delta.point_mm - cut.point_mm).cross(&head.force_n)
            + delta.couple_nmm;
    }
    section.position_mm += cut.point_mm;
    section.tangent_unit = cut.tangent_unit;
    Ok(section)
}

/// Axial, bending and torsion stress of a solid circular section. The
/// section normal is its unit tangent. Transverse shear is omitted by
/// the bench surface rule. `von_mises_mpa = |N|/A + sqrt(σ_b² + 3τ²)`.
pub fn surface_stress(section: &SectionLoad, diameter_mm: f64) -> Result<SurfaceStress, RodError> {
    if !diameter_mm.is_finite()
        || diameter_mm <= 0.0
        || section.force_n.iter().any(|x| !x.is_finite())
        || section.moment_nmm.iter().any(|x| !x.is_finite())
        || section.tangent_unit.iter().any(|x| !x.is_finite())
        || (section.tangent_unit.norm_squared() - 1.0).abs() > TANGENT_UNIT_TOLERANCE
    {
        return Err(RodError::InvalidProperties);
    }
    let axial = -section.force_n.dot(&section.tangent_unit);
    let torque = section.moment_nmm.dot(&section.tangent_unit);
    let bending = (section.moment_nmm - section.tangent_unit * torque).norm();
    let pi = core::f64::consts::PI;
    let area = pi * diameter_mm * diameter_mm / 4.0;
    let d3 = diameter_mm * diameter_mm * diameter_mm;
    let axial_mpa = axial / area;
    let bending_mpa = 32.0 * bending / (pi * d3);
    let torsional_mpa = 16.0 * torque / (pi * d3);
    Ok(SurfaceStress {
        axial_mpa,
        bending_mpa,
        torsional_mpa,
        von_mises_mpa: axial_mpa.abs()
            + libm::sqrt(bending_mpa * bending_mpa + 3.0 * torsional_mpa * torsional_mpa),
    })
}

/// Signed pull-out component, N. `force_on_vertebra_n` is the rod load
/// transmitted to bone; `inward_axis_unit` points from entry into bone.
/// Positive means pull-out; negative means push-in.
pub fn pull_out_component(
    force_on_vertebra_n: Vector3<f64>,
    inward_axis_unit: Vector3<f64>,
) -> f64 {
    -force_on_vertebra_n.dot(&inward_axis_unit)
}

/// Pure Goodman conversion in MPa, usable by subsequent load cases.
/// Mean must be below positive ultimate; amplitude is nonnegative.
/// A zero/negative stress gap, or any invalid input, has no finite answer.
pub fn goodman_equivalent(amplitude_mpa: f64, mean_mpa: f64, ultimate_mpa: f64) -> Option<f64> {
    if !amplitude_mpa.is_finite()
        || amplitude_mpa < 0.0
        || !mean_mpa.is_finite()
        || !ultimate_mpa.is_finite()
        || ultimate_mpa <= 0.0
        || mean_mpa >= ultimate_mpa
    {
        return None;
    }
    let equivalent = amplitude_mpa * ultimate_mpa / (ultimate_mpa - mean_mpa);
    equivalent.is_finite().then_some(equivalent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_turning_loaded_rod_keeps_its_first_order_moment_balance() {
        let points = [Vector3::zeros(), Vector3::z() * 10.0];
        let heads = [
            HeadLoad {
                station_mm: 0.0,
                position_mm: points[0],
                force_n: Vector3::x() * 2.0,
                couple_nmm: Vector3::y() * 20.0,
            },
            HeadLoad {
                station_mm: 10.0,
                position_mm: points[1],
                force_n: -Vector3::x() * 2.0,
                couple_nmm: Vector3::zeros(),
            },
        ];
        let angle = Vector3::y() * 0.1;
        let changes = heads.map(|h| HeadIncrement {
            point_mm: angle.cross(&h.position_mm),
            force_n: angle.cross(&h.force_n),
            couple_nmm: angle.cross(&h.couple_nmm),
        });
        let tangent = (Vector3::z() + angle.cross(&Vector3::z())).normalize();
        let cut = linearised_section_at(
            &points,
            &heads,
            &changes,
            5.0,
            SectionIncrement {
                point_mm: Vector3::x() * 0.5,
                tangent_unit: tangent,
            },
        )
        .unwrap();
        assert!((cut.force_n - Vector3::new(2.0, 0.0, -0.2)).norm() < 1e-14);
        assert!((cut.moment_nmm - Vector3::y() * 10.0).norm() < 1e-14);
        let end = linearised_section_at(
            &points,
            &heads,
            &changes,
            10.0,
            SectionIncrement {
                point_mm: Vector3::x(),
                tangent_unit: tangent,
            },
        )
        .unwrap();
        assert!(end.force_n.norm() < 1e-14 && end.moment_nmm.norm() < 1e-14);
    }

    #[test]
    fn opposing_axial_forces_produce_tension_on_both_cut_faces() {
        let polyline = [Vector3::zeros(), Vector3::x() * 10.0];
        let heads = [
            HeadLoad {
                station_mm: 0.0,
                position_mm: polyline[0],
                force_n: -Vector3::x() * 8.0,
                couple_nmm: Vector3::zeros(),
            },
            HeadLoad {
                station_mm: 10.0,
                position_mm: polyline[1],
                force_n: Vector3::x() * 8.0,
                couple_nmm: Vector3::zeros(),
            },
        ];
        let section = section_at(&polyline, &heads, 5.0).unwrap();
        let stress = surface_stress(&section, 2.0).unwrap();
        assert!((stress.axial_mpa - 8.0 / core::f64::consts::PI).abs() < 1e-14);
        assert_eq!(
            resultant(&heads, Vector3::zeros()),
            (Vector3::zeros(), Vector3::zeros())
        );
    }

    #[test]
    fn a_head_couple_has_a_jump_at_its_station() {
        let polyline = [Vector3::zeros(), Vector3::z() * 10.0];
        let load = HeadLoad {
            station_mm: 5.0,
            position_mm: Vector3::z() * 5.0,
            force_n: Vector3::zeros(),
            couple_nmm: Vector3::x() * 7.0,
        };
        assert_eq!(
            section_at(&polyline, &[load], 4.0).unwrap().moment_nmm,
            Vector3::zeros()
        );
        assert_eq!(
            section_at(&polyline, &[load], 5.0).unwrap().moment_nmm,
            Vector3::x() * 7.0
        );
    }
    #[test]
    fn mirroring_positions_and_forces_mirrors_moments_as_axial_vectors() {
        let mirror = nalgebra::Matrix3::from_diagonal(&Vector3::new(1.0, -1.0, 1.0));
        let points = [Vector3::zeros(), Vector3::new(4.0, 2.0, 20.0)];
        let length = points[1].norm();
        let head = HeadLoad {
            station_mm: 0.0,
            position_mm: points[0],
            force_n: Vector3::new(3.0, -4.0, 7.0),
            couple_nmm: Vector3::new(1.0, 2.0, -3.0),
        };
        let section = section_at(&points, &[head], length * 0.5).unwrap();
        let mirrored = section_at(
            &[mirror * points[0], mirror * points[1]],
            &[HeadLoad {
                position_mm: mirror * head.position_mm,
                force_n: mirror * head.force_n,
                couple_nmm: -mirror * head.couple_nmm,
                ..head
            }],
            length * 0.5,
        )
        .unwrap();
        assert!((mirrored.force_n - mirror * section.force_n).norm() < 1e-14);
        assert!((mirrored.moment_nmm + mirror * section.moment_nmm).norm() < 1e-13);
        let original_stress = surface_stress(&section, 4.0).unwrap();
        let mirrored_stress = surface_stress(&mirrored, 4.0).unwrap();
        assert!((original_stress.von_mises_mpa - mirrored_stress.von_mises_mpa).abs() < 1e-13);
        assert!((original_stress.torsional_mpa + mirrored_stress.torsional_mpa).abs() < 1e-13);
    }
}
