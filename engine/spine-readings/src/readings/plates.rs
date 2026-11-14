//! Encode independent projected plate tilts in a version's fitted geometry.

use crate::ReadingsError;
use crate::curves::geometry::{lower_coronal_tilt_deg, sagittal_tilt_deg, upper_coronal_tilt_deg};
use crate::plates::PlateSpread;
use forward_model::endplates::Endplates;
use forward_model::rotation::{rot_x, rot_y, rot_z, rotation_vector_from_matrix};
use forward_model::vertebra_params::VertebraParams;
use nalgebra::Vector3;
use shapes::spine_fit::FittedVertebra;

use super::NamedSampler;

fn rotation(v: &mut FittedVertebra) {
    let a = v.angles;
    v.pose.rotation = rotation_vector_from_matrix(
        &(rot_x(a.side_tilt_rad) * rot_y(a.forward_lean_rad) * rot_z(a.twist_rad)),
    )
    .into();
}

fn side(v: &FittedVertebra) -> [f64; 2] {
    let p = Endplates::of(&VertebraParams::from(&v.pose));
    [
        sagittal_tilt_deg(&p.top.normal).to_radians(),
        sagittal_tilt_deg(&p.bottom.normal).to_radians(),
    ]
}

fn invalid() -> ReadingsError {
    ReadingsError::InvalidValue {
        field: "fit".into(),
        reason: "plate-version-outside-geometry".into(),
    }
}

pub(super) fn perturb(
    v: &mut FittedVertebra,
    original: &FittedVertebra,
    spread: PlateSpread,
    sampler: &NamedSampler,
    i: usize,
    k: u32,
    has_side: bool,
) -> Result<(), ReadingsError> {
    let mut offsets = [[0.0; 2]; 2];
    for (plane, name, sds) in [(0, "front", spread.front), (1, "side", spread.side)] {
        for (plate, label) in ["upper", "lower"].into_iter().enumerate() {
            let z = sampler.normal(&format!("versions-geometry/{i}/plate/{name}/{label}"), k)?;
            if plane == 0 || (has_side && !v.inferred.depth) {
                offsets[plane][plate] = sds[plate] * z;
            }
        }
    }
    let twist = if v.inferred.twist {
        0.0
    } else {
        v.sd.angles_rad[2] * sampler.normal(&format!("versions-geometry/{i}/angle/2"), k)?
    };
    if offsets.iter().flatten().all(|x| *x == 0.0) && twist == 0.0 {
        return Ok(());
    }
    v.angles.twist_rad += twist;
    if offsets.iter().flatten().all(|x| *x == 0.0)
        && original.pose.wedge_front == 0.0
        && original.pose.wedge_side == 0.0
    {
        // Axial rotation leaves flat plates' normals unchanged. Rebuilding
        // those normals from projected angles would lose their direction
        // at a ninety-degree lean, where the front projection vanishes.
        rotation(v);
        return Ok(());
    }
    let target_side = side(original);
    let target_side = [
        target_side[0] + offsets[1][0],
        target_side[1] + offsets[1][1],
    ];
    if let Some(tilt) = original.corner_tilt_rad {
        let mean = 0.5 * (offsets[0][0] + offsets[0][1]);
        v.corner_tilt_rad = Some(tilt + mean);
        v.angles.side_tilt_rad += mean;
        v.pose.wedge_front += offsets[0][0] - offsets[0][1];
        rotation(v);
        if has_side && !v.inferred.depth {
            match_side(v, target_side)?;
        }
    } else {
        let target_front = [
            upper_coronal_tilt_deg(original).to_radians() + offsets[0][0],
            lower_coronal_tilt_deg(original).to_radians() + offsets[0][1],
        ];
        if !has_side || v.inferred.depth {
            // Front draws cannot turn typical lean or a typical side wedge
            // into measured variation. Only the two front freedoms move.
            rotation(v);
            match_front(v, target_front)?;
            return Ok(());
        }
        // Both upward normals are fixed by their two projected angles.
        // Their bisector is the body's up axis; the original twist fixes
        // the remaining rotation, and the two local tilts give the wedges.
        let normal = |front: f64, side: f64| {
            Vector3::new(
                libm::sin(side) * libm::cos(front),
                -libm::sin(front) * libm::cos(side),
                libm::cos(side) * libm::cos(front),
            )
            .normalize()
        };
        let top = normal(target_front[0], target_side[0]);
        let bottom = normal(target_front[1], target_side[1]);
        let up = (top + bottom).normalize();
        v.angles.side_tilt_rad = libm::atan2(-up.y, up.z);
        v.angles.forward_lean_rad = libm::atan2(up.x, libm::hypot(up.y, up.z));
        let r = rot_x(v.angles.side_tilt_rad)
            * rot_y(v.angles.forward_lean_rad)
            * rot_z(v.angles.twist_rad);
        let local = r.transpose() * top;
        v.pose.wedge_front = 2.0 * libm::atan2(-local.y, local.z);
        v.pose.wedge_side = 2.0 * libm::atan2(local.x, libm::hypot(local.y, local.z));
        rotation(v);
        let actual_front = [
            upper_coronal_tilt_deg(v).to_radians(),
            lower_coronal_tilt_deg(v).to_radians(),
        ];
        if actual_front
            .iter()
            .zip(target_front)
            .chain(side(v).iter().zip(target_side))
            .any(|(actual, target)| !actual.is_finite() || (actual - target).abs() > 1e-9)
        {
            return Err(invalid());
        }
    }
    if [v.pose.wedge_front, v.pose.wedge_side]
        .iter()
        .any(|x| !x.is_finite() || x.abs() >= core::f64::consts::PI)
    {
        return Err(invalid());
    }
    Ok(())
}

// With the side geometry held fixed, match the two front projections by
// varying side tilt and front wedge only.
fn match_front(v: &mut FittedVertebra, target: [f64; 2]) -> Result<(), ReadingsError> {
    const STEP: f64 = 1e-6;
    let front = |v: &FittedVertebra| {
        [
            upper_coronal_tilt_deg(v).to_radians(),
            lower_coronal_tilt_deg(v).to_radians(),
        ]
    };
    for _ in 0..12 {
        let at = front(v);
        let residual = [target[0] - at[0], target[1] - at[1]];
        if residual.iter().all(|r| r.abs() < 1e-12) {
            if v.pose.wedge_front.abs() >= core::f64::consts::PI {
                return Err(invalid());
            }
            return Ok(());
        }
        let mut jacobian = [[0.0; 2]; 2];
        for axis in [0, 1] {
            let mut shifted = v.clone();
            if axis == 0 {
                shifted.angles.side_tilt_rad += STEP;
            } else {
                shifted.pose.wedge_front += STEP;
            }
            rotation(&mut shifted);
            let changed = front(&shifted);
            for row in 0..2 {
                jacobian[row][axis] = (changed[row] - at[row]) / STEP;
            }
        }
        let [[a, b], [c, d]] = jacobian;
        let determinant = a * d - b * c;
        if !determinant.is_finite() || determinant.abs() < 1e-10 {
            return Err(invalid());
        }
        v.angles.side_tilt_rad += (d * residual[0] - b * residual[1]) / determinant;
        v.pose.wedge_front += (a * residual[1] - c * residual[0]) / determinant;
        rotation(v);
    }
    Err(invalid())
}

// Front corner tilts and their wedge are already fixed. Match the two
// sagittal projections using the shortest local step in lean, side wedge
// and body tilt. The extra body freedom matters when axial twist leaves a
// side wedge almost edge-on. Corner-based front readings stay unchanged.
fn match_side(v: &mut FittedVertebra, target: [f64; 2]) -> Result<(), ReadingsError> {
    const STEP: f64 = 1e-6;
    // Cap and backtrack Newton steps near a profile seen almost end-on.
    // These are numerical controls, unrelated to the measurement spread.
    const MAX_STEP: f64 = 0.5;
    for _ in 0..48 {
        let at = side(v);
        let residual = [target[0] - at[0], target[1] - at[1]];
        if residual.iter().all(|r| r.abs() < 1e-12) {
            return Ok(());
        }
        let mut jacobian = [[0.0; 3]; 2];
        for axis in [0, 1, 2] {
            let mut shifted = v.clone();
            if axis == 0 {
                shifted.angles.forward_lean_rad += STEP;
            } else if axis == 1 {
                shifted.pose.wedge_side += STEP;
            } else {
                shifted.angles.side_tilt_rad += STEP;
            }
            rotation(&mut shifted);
            let changed = side(&shifted);
            for row in 0..2 {
                jacobian[row][axis] = (changed[row] - at[row]) / STEP;
            }
        }
        // Jᵀ (J Jᵀ)⁻¹ r is the least-length step that solves J delta = r.
        let dot = |a: [f64; 3], b: [f64; 3]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
        let a = dot(jacobian[0], jacobian[0]);
        let b = dot(jacobian[0], jacobian[1]);
        let c = b;
        let d = dot(jacobian[1], jacobian[1]);
        let determinant = a * d - b * c;
        if !determinant.is_finite() || determinant.abs() < 1e-14 {
            break;
        }
        let multiplier = [
            (d * residual[0] - b * residual[1]) / determinant,
            (a * residual[1] - c * residual[0]) / determinant,
        ];
        let delta = [0, 1, 2]
            .map(|axis| jacobian[0][axis] * multiplier[0] + jacobian[1][axis] * multiplier[1]);
        let largest = delta
            .iter()
            .fold(0.0_f64, |largest, x| largest.max(x.abs()));
        let mut fraction = (MAX_STEP / largest).min(1.0);
        let error = residual[0] * residual[0] + residual[1] * residual[1];
        let mut accepted = false;
        for _ in 0..24 {
            let mut candidate = v.clone();
            candidate.angles.forward_lean_rad += fraction * delta[0];
            candidate.pose.wedge_side += fraction * delta[1];
            candidate.angles.side_tilt_rad += fraction * delta[2];
            if candidate.pose.wedge_side.abs() < core::f64::consts::PI {
                rotation(&mut candidate);
                let changed = side(&candidate);
                let remaining = [target[0] - changed[0], target[1] - changed[1]];
                let next_error = remaining[0] * remaining[0] + remaining[1] * remaining[1];
                if next_error.is_finite() && next_error < error {
                    *v = candidate;
                    accepted = true;
                    break;
                }
            }
            fraction *= 0.5;
        }
        if !accepted {
            break;
        }
    }
    side_at_zero_tilt(v, target)
}

// At zero body tilt, the sagittal projections are lean plus and minus
// atan((cos(twist) tan(side_wedge/2) + sin(twist) sin(front_wedge/2))
//      / cos(front_wedge/2)). This gives a constructive solution when the
// shortest local steps stall with a wedge seen nearly end-on.
// This fallback does not keep the body's frontal tilt: it sets that tilt
// to zero in the 3D pose used by placements and loads. Corner-based front
// plate readings still keep their independently drawn tilts.
fn side_at_zero_tilt(v: &mut FittedVertebra, target: [f64; 2]) -> Result<(), ReadingsError> {
    let twist = v.angles.twist_rad;
    let cosine = libm::cos(twist);
    if cosine.abs() < 1e-10 {
        return Err(invalid());
    }
    let a = 0.5 * v.pose.wedge_front;
    let mean = 0.5 * (target[0] + target[1]);
    let difference = 0.5 * (target[0] - target[1]);
    let tangent = (libm::cos(a) * libm::tan(difference) - libm::sin(twist) * libm::sin(a)) / cosine;
    v.angles.side_tilt_rad = 0.0;
    v.angles.forward_lean_rad = mean;
    v.pose.wedge_side = 2.0 * libm::atan(tangent);
    rotation(v);
    if side(v)
        .iter()
        .zip(target)
        .all(|(actual, expected)| actual.is_finite() && (actual - expected).abs() < 1e-10)
    {
        Ok(())
    } else {
        Err(invalid())
    }
}
