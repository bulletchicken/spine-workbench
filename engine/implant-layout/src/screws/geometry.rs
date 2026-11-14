//! Entry implied by a table chord on a generic elliptical body.
//! Positions are patient-frame mm, x anterior, y left, z cranial.

use crate::LayoutError;
use forward_model::endplates::Endplates;
use forward_model::pedicles::pedicle_centres;
use forward_model::vertebra_params::VertebraParams;
use nalgebra::Vector3;
use shapes::made_up::VertebraPose;
use shapes::spine_readings::BodySide;

/// This project's numerical tolerance for a normalised polynomial.
const ROOT_TOLERANCE: f64 = 1e-11;
/// This project's fixed bisection count, independent of platform clocks.
const ROOT_STEPS: usize = 64;

/// One table-implied path on the generic fitted body, never a bone landmark.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trajectory {
    pub entry_mm: [f64; 3],
    pub direction_unit: [f64; 3],
    /// Unsigned angle to the patient's sagittal plane, in degrees.
    pub convergence_deg: f64,
}

/// One path direction independent of whether the generic body supplies
/// an anterior exit for that path. A drawn divergent angle can still
/// contribute to a convergence band when an entry cannot be formed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Direction {
    pub direction_unit: [f64; 3],
    /// Unsigned angle to the patient's sagittal plane, in degrees.
    pub convergence_deg: f64,
}

/// Turn the superior-plane path towards the vertebral midline. Negative
/// printed angles diverge; the direction remains readable even if that
/// ray misses the generic body. This project's sign convention uses a
/// negative right-hand turn on the left and a positive one on the right.
pub fn direction(
    pose: &VertebraPose,
    side: BodySide,
    medial_angle_deg: f64,
) -> Result<Direction, LayoutError> {
    if !medial_angle_deg.is_finite() {
        return Err(invalid("trajectory", "angle-must-be-finite"));
    }
    let params = checked_params(pose)?;
    let normal = Endplates::of(&params).top.normal;
    let forward = Vector3::from(straight_forward(pose)?);
    let sign = match side {
        BodySide::Left => -1.0,
        BodySide::Right => 1.0,
    };
    let angle = sign * medial_angle_deg.to_radians();
    let unit = unit(forward * libm::cos(angle) + normal.cross(&forward) * libm::sin(angle))?;
    Ok(Direction {
        direction_unit: unit.into(),
        convergence_deg: libm::asin(unit.y.abs().clamp(0.0, 1.0)).to_degrees(),
    })
}

/// Project the vertebra's anterior axis onto the superior endplate plane.
/// This uses the forward model's own endplate and rotation definitions.
pub fn straight_forward(pose: &VertebraPose) -> Result<[f64; 3], LayoutError> {
    let params = checked_params(pose)?;
    let normal = Endplates::of(&params).top.normal;
    let anterior = params.rotation_matrix().column(0).into_owned();
    unit(anterior - normal * anterior.dot(&normal)).map(Into::into)
}

/// This project's own entry inference: rotate the straight-forward path
/// medially about the superior normal; intersect its ray from the template
/// pedicle centre with the body's anterior surface; subtract the chord.
/// In the right-handed frame a positive medial angle turns the left path
/// by a negative right-hand rotation and the right path by a positive one.
/// Both paths therefore turn towards the midline.
///
/// The generic surface is this project's ruled elliptical loft: corresponding
/// endplate rim vectors and centres vary linearly between the two endplates.
/// A ray without an anterior intersection is refused, never given an invented
/// entry. The chord changes entry only; it cannot change the direction.
pub fn trajectory(
    pose: &VertebraPose,
    side: BodySide,
    medial_angle_deg: f64,
    chord_mm: f64,
) -> Result<Trajectory, LayoutError> {
    if !medial_angle_deg.is_finite() || !chord_mm.is_finite() || chord_mm <= 0.0 {
        return Err(invalid(
            "trajectory",
            "angle-and-chord-must-be-finite-and-chord-positive",
        ));
    }
    let params = checked_params(pose)?;
    let endplates = Endplates::of(&params);
    let path = direction(pose, side, medial_angle_deg)?;
    let direction = Vector3::from(path.direction_unit);
    let centre = pedicle_centres(&params)[usize::from(side == BodySide::Right)];
    let anterior = params.rotation_matrix().column(0).into_owned();
    let exit = anterior_exit(&endplates, centre, direction, anterior)
        .ok_or_else(|| LayoutError::Refused("no-anterior-body-exit".into()))?;
    let entry = exit - direction * chord_mm;
    if entry.iter().any(|value| !value.is_finite()) {
        return Err(invalid("trajectory", "entry-overflow"));
    }
    Ok(Trajectory {
        entry_mm: entry.into(),
        direction_unit: direction.into(),
        convergence_deg: path.convergence_deg,
    })
}

fn checked_params(pose: &VertebraPose) -> Result<VertebraParams, LayoutError> {
    let params = VertebraParams::from(pose);
    if params.to_numbers().iter().any(|value| !value.is_finite())
        || params.height <= 0.0
        || [
            params.top_half_width,
            params.top_half_depth,
            params.bottom_half_width,
            params.bottom_half_depth,
        ]
        .iter()
        .any(|value| *value <= 0.0)
        || [params.wedge_front, params.wedge_side]
            .iter()
            .any(|value| value.abs() >= core::f64::consts::PI)
    {
        return Err(invalid("trajectory.pose", "invalid-generic-body"));
    }
    Ok(params)
}

fn unit(value: Vector3<f64>) -> Result<Vector3<f64>, LayoutError> {
    let length = libm::sqrt(value.dot(&value));
    if !length.is_finite() || length <= f64::EPSILON {
        Err(invalid("trajectory", "degenerate-direction"))
    } else {
        Ok(value / length)
    }
}

// For a loft slice, P+t*d = C(s)+a*U(s)+b*V(s), a²+b²=1.
// Cramer's rule makes a,b ratios of quadratics in s. Squaring gives a
// quartic whose roots are all candidate ray/surface intersections. A slice
// parallel to the ray is handled separately rather than dividing by zero.
fn anterior_exit(
    endplates: &Endplates,
    start: Vector3<f64>,
    direction: Vector3<f64>,
    anterior: Vector3<f64>,
) -> Option<Vector3<f64>> {
    let bottom = endplates.bottom;
    let top = endplates.top;
    let centre = [bottom.centre, top.centre - bottom.centre];
    let width = [
        bottom.width_axis * bottom.half_width,
        top.width_axis * top.half_width - bottom.width_axis * bottom.half_width,
    ];
    let depth = [
        bottom.depth_axis * bottom.half_depth,
        top.depth_axis * top.half_depth - bottom.depth_axis * bottom.half_depth,
    ];
    let offset = [start - centre[0], -centre[1]];
    let denominator = determinant_polynomial(width, depth, direction);
    let a = determinant_polynomial(offset, depth, direction);
    let b = determinant_polynomial(width, offset, direction);
    let mut polynomial = vec![0.0; 5];
    for i in 0..3 {
        for j in 0..3 {
            polynomial[i + j] += a[i] * a[j] + b[i] * b[j] - denominator[i] * denominator[j];
        }
    }
    let mut candidates = Vec::new();
    for s in roots_on_unit_interval(&polynomial) {
        let c = centre[0] + centre[1] * s;
        let u = width[0] + width[1] * s;
        let v = depth[0] + depth[1] * s;
        let n = u.cross(&v);
        let divisor = direction.dot(&n);
        let scale = libm::sqrt(n.dot(&n));
        if scale == 0.0 || !scale.is_finite() {
            continue;
        }
        if divisor.abs() > ROOT_TOLERANCE * scale {
            let t = (c - start).dot(&n) / divisor;
            let point = start + direction * t;
            let (a, b) = ellipse_coordinates(point - c, u, v)?;
            if (a * a + b * b - 1.0).abs() < 1e-6 {
                candidates.push((t, point));
            }
        } else if (start - c).dot(&n).abs() <= ROOT_TOLERANCE * scale * (start - c).norm().max(1.0)
        {
            let (a0, b0) = ellipse_coordinates(start - c, u, v)?;
            let (ad, bd) = ellipse_coordinates(direction, u, v)?;
            let quadratic = ad * ad + bd * bd;
            let linear = 2.0 * (a0 * ad + b0 * bd);
            let constant = a0 * a0 + b0 * b0 - 1.0;
            let discriminant = linear * linear - 4.0 * quadratic * constant;
            if quadratic > 0.0 && discriminant >= 0.0 {
                for sign in [-1.0, 1.0] {
                    let t = (-linear + sign * libm::sqrt(discriminant)) / (2.0 * quadratic);
                    candidates.push((t, start + direction * t));
                }
            }
        }
    }
    candidates
        .into_iter()
        .filter(|(t, point)| {
            *t >= 0.0
                && t.is_finite()
                && (*point - (top.centre + bottom.centre) * 0.5).dot(&anterior) > 0.0
        })
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, point)| point)
}

fn determinant_polynomial(
    a: [Vector3<f64>; 2],
    b: [Vector3<f64>; 2],
    direction: Vector3<f64>,
) -> [f64; 3] {
    [
        direction.dot(&a[0].cross(&b[0])),
        direction.dot(&(a[0].cross(&b[1]) + a[1].cross(&b[0]))),
        direction.dot(&a[1].cross(&b[1])),
    ]
}

fn ellipse_coordinates(
    point: Vector3<f64>,
    u: Vector3<f64>,
    v: Vector3<f64>,
) -> Option<(f64, f64)> {
    let (uu, uv, vv) = (u.dot(&u), u.dot(&v), v.dot(&v));
    let determinant = uu * vv - uv * uv;
    if !determinant.is_finite() || determinant <= 0.0 {
        return None;
    }
    let (pu, pv) = (point.dot(&u), point.dot(&v));
    Some((
        (pu * vv - pv * uv) / determinant,
        (pv * uu - pu * uv) / determinant,
    ))
}

fn polynomial_value(coefficients: &[f64], value: f64) -> f64 {
    coefficients
        .iter()
        .rev()
        .fold(0.0, |sum, coefficient| sum * value + coefficient)
}

// A polynomial is monotone between its derivative's real roots. Recursive
// partitioning therefore catches tangencies as well as sign-changing roots.
fn roots_on_unit_interval(coefficients: &[f64]) -> Vec<f64> {
    let scale = coefficients
        .iter()
        .fold(0.0_f64, |scale, value| scale.max(value.abs()));
    if !scale.is_finite() || scale == 0.0 {
        return Vec::new();
    }
    let mut polynomial: Vec<_> = coefficients.iter().map(|value| value / scale).collect();
    while polynomial.len() > 1
        && polynomial
            .last()
            .is_some_and(|value| value.abs() <= f64::EPSILON)
    {
        polynomial.pop();
    }
    if polynomial.len() == 1 {
        return Vec::new();
    }
    let derivative: Vec<_> = polynomial
        .iter()
        .enumerate()
        .skip(1)
        .map(|(i, value)| *value * i as f64)
        .collect();
    let mut partitions = vec![0.0];
    partitions.extend(roots_on_unit_interval(&derivative));
    partitions.push(1.0);
    partitions.sort_by(f64::total_cmp);
    let mut roots = Vec::new();
    for &point in &partitions {
        if polynomial_value(&polynomial, point).abs() <= ROOT_TOLERANCE {
            roots.push(point);
        }
    }
    for interval in partitions.windows(2) {
        let (mut low, mut high) = (interval[0], interval[1]);
        let mut below = polynomial_value(&polynomial, low);
        let above = polynomial_value(&polynomial, high);
        if below * above >= 0.0 {
            continue;
        }
        for _ in 0..ROOT_STEPS {
            let middle = (low + high) * 0.5;
            let value = polynomial_value(&polynomial, middle);
            if value == 0.0 {
                low = middle;
                high = middle;
                break;
            }
            if below * value <= 0.0 {
                high = middle;
            } else {
                low = middle;
                below = value;
            }
        }
        roots.push((low + high) * 0.5);
    }
    roots.sort_by(f64::total_cmp);
    roots.dedup_by(|a, b| (*a - *b).abs() <= ROOT_TOLERANCE);
    roots
}

fn invalid(field: &str, reason: &str) -> LayoutError {
    LayoutError::InvalidValue {
        field: field.into(),
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forward_model::rotation::{rot_x, rot_y, rotation_vector_from_matrix};
    use shapes::levels::Level;

    fn body() -> VertebraPose {
        VertebraParams {
            centre: Vector3::zeros(),
            rotation: Vector3::zeros(),
            height: 24.0,
            wedge_front: 0.0,
            wedge_side: 0.0,
            top_half_width: 16.0,
            bottom_half_width: 16.0,
            top_half_depth: 12.0,
            bottom_half_depth: 12.0,
        }
        .pose(Level::T7)
    }

    #[test]
    fn left_and_right_paths_mirror_and_turn_towards_the_midline() {
        let pose = body();
        let left = trajectory(&pose, BodySide::Left, 16.0, 32.0).unwrap();
        let right = trajectory(&pose, BodySide::Right, 16.0, 32.0).unwrap();
        for axis in [0, 2] {
            assert!((left.entry_mm[axis] - right.entry_mm[axis]).abs() < 1e-10);
            assert!((left.direction_unit[axis] - right.direction_unit[axis]).abs() < 1e-14);
        }
        assert!((left.entry_mm[1] + right.entry_mm[1]).abs() < 1e-10);
        assert!((left.direction_unit[1] + right.direction_unit[1]).abs() < 1e-14);
        assert!(left.direction_unit[1] < 0.0);
        assert!(right.direction_unit[1] > 0.0);
        assert_eq!(left.convergence_deg, right.convergence_deg);
    }

    #[test]
    fn the_unturned_path_lies_in_the_superior_plane_after_pose_and_wedge() {
        let mut pose = body();
        pose.rotation = rotation_vector_from_matrix(&(rot_x(0.2) * rot_y(-0.1))).into();
        pose.wedge_front = 0.1;
        pose.wedge_side = -0.2;
        let normal = Endplates::of(&VertebraParams::from(&pose)).top.normal;
        let path = Vector3::from(straight_forward(&pose).unwrap());
        assert!(path.dot(&normal).abs() < 1e-14);
        assert!((path.norm() - 1.0).abs() < 1e-14);
        let turned = trajectory(&pose, BodySide::Left, 16.0, 32.0).unwrap();
        assert!(Vector3::from(turned.direction_unit).dot(&normal).abs() < 1e-14);
    }

    #[test]
    fn a_zero_angle_reaches_the_anterior_ellipse_at_the_template_pedicle_height() {
        let pose = body();
        let chord = 32.0;
        let path = trajectory(&pose, BodySide::Left, 0.0, chord).unwrap();
        let pedicle = pedicle_centres(&VertebraParams::from(&pose))[0];
        // Horizontal path keeps y,z. The ellipse x²/12²+y²/16²=1
        // independently gives its anterior x, and entry subtracts the chord.
        let exit_x = pose.top_half_depth
            * libm::sqrt(
                1.0 - (pedicle.y / pose.top_half_width) * (pedicle.y / pose.top_half_width),
            );
        assert!((path.entry_mm[0] - (exit_x - chord)).abs() < 1e-10);
        assert!((path.entry_mm[1] - pedicle.y).abs() < 1e-10);
        assert!((path.entry_mm[2] - pedicle.z).abs() < 1e-10);
    }

    #[test]
    fn changing_the_chord_moves_entry_exactly_along_the_same_ray() {
        let a = trajectory(&body(), BodySide::Right, 16.0, 32.0).unwrap();
        let b = trajectory(&body(), BodySide::Right, 16.0, 39.0).unwrap();
        assert_eq!(a.direction_unit, b.direction_unit);
        for axis in 0..3 {
            assert!(
                (b.entry_mm[axis] - a.entry_mm[axis] + 7.0 * a.direction_unit[axis]).abs() < 1e-12
            );
        }
        assert!(trajectory(&body(), BodySide::Left, 180.0, 32.0).is_err());
        assert!(trajectory(&body(), BodySide::Left, 0.0, f64::NAN).is_err());
    }

    #[test]
    fn a_wedged_tapered_loft_reaches_a_point_worked_from_its_rims() {
        let mut pose = body();
        pose.wedge_front = 0.12;
        pose.wedge_side = -0.18;
        pose.bottom_half_width = 18.0;
        pose.bottom_half_depth = 14.0;
        let plates = Endplates::of(&VertebraParams::from(&pose));
        let fraction = 0.63;
        let blend = |a: Vector3<f64>, b: Vector3<f64>| a * (1.0 - fraction) + b * fraction;
        let centre = blend(plates.bottom.centre, plates.top.centre);
        let width = blend(
            plates.bottom.width_axis * plates.bottom.half_width,
            plates.top.width_axis * plates.top.half_width,
        );
        let depth = blend(
            plates.bottom.depth_axis * plates.bottom.half_depth,
            plates.top.depth_axis * plates.top.half_depth,
        );
        // (3/5)^2+(-4/5)^2=1: this is exactly on the loft's anterior rim.
        let expected = centre + width * 0.6 - depth * 0.8;
        let direction = Vector3::x();
        let actual =
            anterior_exit(&plates, expected - direction * 40.0, direction, direction).unwrap();
        assert!((actual - expected).norm() < 1e-8);
    }

    #[test]
    fn rigid_pose_changes_turn_and_translate_the_same_implied_entry() {
        let base = trajectory(&body(), BodySide::Left, 16.0, 32.0).unwrap();
        let rotation = rot_x(0.2) * rot_y(-0.1);
        let translation = Vector3::new(17.0, -8.0, 101.0);
        let mut pose = body();
        pose.rotation = rotation_vector_from_matrix(&rotation).into();
        pose.centre = translation.into();
        let actual = trajectory(&pose, BodySide::Left, 16.0, 32.0).unwrap();
        let expected_entry = rotation * Vector3::from(base.entry_mm) + translation;
        let expected_direction = rotation * Vector3::from(base.direction_unit);
        assert!((Vector3::from(actual.entry_mm) - expected_entry).norm() < 1e-8);
        assert!((Vector3::from(actual.direction_unit) - expected_direction).norm() < 1e-13);
    }

    #[test]
    fn a_ray_that_misses_the_body_still_has_a_direction() {
        let path = direction(&body(), BodySide::Left, 180.0).unwrap();
        assert!(path.direction_unit[0] < 0.0);
        assert!(trajectory(&body(), BodySide::Left, 180.0, 32.0).is_err());
    }

    #[test]
    fn quartic_partitioning_keeps_a_double_root_and_a_crossing() {
        // (s-1/4)^2 (s-3/4) = s³-1.25s²+0.4375s-0.046875.
        let roots = roots_on_unit_interval(&[-0.046875, 0.4375, -1.25, 1.0]);
        assert_eq!(roots.len(), 2);
        assert!((roots[0] - 0.25).abs() < 1e-10);
        assert!((roots[1] - 0.75).abs() < 1e-10);
    }
}
