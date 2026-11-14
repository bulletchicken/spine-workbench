//! Interpolating contour geometry, separate from its moving rigid frame.
use super::{Rod, STRAIGHT_RELATIVE_TOLERANCE, SolveError};
use crate::rod_beams::polyline_stations;
use nalgebra::{Matrix3, Vector3};

#[derive(Clone)]
pub(super) struct Curve {
    pub knots: Vec<f64>,
    pub points: Vec<Vector3<f64>>,
    slopes: Vec<Vector3<f64>>,
    pub long: Vector3<f64>,
    pub is_curved: bool,
}
impl Curve {
    pub fn new(rod: &Rod) -> Result<Self, SolveError> {
        if [
            rod.properties.young_modulus_mpa,
            rod.properties.shear_modulus_mpa,
            rod.properties.diameter_mm,
            rod.properties.shear_coefficient,
        ]
        .iter()
        .any(|v| !v.is_finite() || *v <= 0.0)
        {
            return Err(SolveError::InvalidInput("rod-properties"));
        }
        let knots = polyline_stations(&rod.points_mm)
            .map_err(|_| SolveError::InvalidInput("rod contour"))?;
        let points = rod.points_mm.clone();
        let length = *knots.last().unwrap();
        let chord = points[points.len() - 1] - points[0];
        if chord.norm() < length * 1e-8 {
            return Err(SolveError::InvalidInput("rod long direction"));
        }
        let long = chord.normalize();
        let peak = points
            .iter()
            .map(|p| (*p - points[0]).cross(&long))
            .max_by(|a, b| a.norm_squared().total_cmp(&b.norm_squared()))
            .unwrap();
        let is_curved = peak.norm() > STRAIGHT_RELATIVE_TOLERANCE * length;
        let slopes = (0..points.len())
            .map(|i| {
                let a = i.saturating_sub(1);
                let b = (i + 1).min(points.len() - 1);
                (points[b] - points[a]) / (knots[b] - knots[a])
            })
            .collect();
        Ok(Self {
            knots,
            points,
            slopes,
            long,
            is_curved,
        })
    }
    pub fn centre(&self) -> Vector3<f64> {
        self.points.iter().copied().sum::<Vector3<f64>>() / self.points.len() as f64
    }
    pub fn length(&self) -> f64 {
        *self.knots.last().unwrap()
    }
    pub fn at(&self, s: f64) -> (Vector3<f64>, Vector3<f64>, Vector3<f64>) {
        if s < 0.0 {
            let tangent = self.slopes[0].normalize();
            return (self.points[0] + tangent * s, tangent, Vector3::zeros());
        }
        let length = self.length();
        if s > length {
            let tangent = self.slopes[self.slopes.len() - 1].normalize();
            return (
                self.points[self.points.len() - 1] + tangent * (s - length),
                tangent,
                Vector3::zeros(),
            );
        }
        let i = self
            .knots
            .partition_point(|k| *k <= s)
            .saturating_sub(1)
            .min(self.points.len() - 2);
        let h = self.knots[i + 1] - self.knots[i];
        let t = (s - self.knots[i]) / h;
        let t2 = t * t;
        let t3 = t2 * t;
        let a = self.points[i];
        let b = self.points[i + 1];
        let va = self.slopes[i];
        let vb = self.slopes[i + 1];
        let p = a * (2.0 * t3 - 3.0 * t2 + 1.0)
            + va * (h * (t3 - 2.0 * t2 + t))
            + b * (-2.0 * t3 + 3.0 * t2)
            + vb * (h * (t3 - t2));
        let d = a * ((6.0 * t2 - 6.0 * t) / h)
            + va * (3.0 * t2 - 4.0 * t + 1.0)
            + b * ((-6.0 * t2 + 6.0 * t) / h)
            + vb * (3.0 * t2 - 2.0 * t);
        let dd = a * ((12.0 * t - 6.0) / (h * h))
            + va * ((6.0 * t - 4.0) / h)
            + b * ((-12.0 * t + 6.0) / (h * h))
            + vb * ((6.0 * t - 2.0) / h);
        (p, d, dd)
    }
    /// The adverse constraint curvature for `-force dot r(s)`. At a C1
    /// knot, only the slide's diagonal Hessian term jumps. Choosing the
    /// larger work makes its negative no greater than either one-sided
    /// tangent, so a positive reduced matrix certifies both directions.
    pub fn curvature_work(&self, s: f64, rotation: &Matrix3<f64>, force: Vector3<f64>) -> f64 {
        let work = force.dot(&(rotation * self.at(s).2));
        if s == 0.0 || s == self.length() {
            // The straight continuation has zero curvature on the other
            // side of each original end; certify both C1 one-sided tangents.
            work.max(0.0)
        } else if let Ok(i) = self.knots.binary_search_by(|k| k.total_cmp(&s))
            && i > 0
            && i + 1 < self.knots.len()
        {
            // Interior chord stations are positive. The preceding float
            // selects the left segment without a project-sized offset.
            let left = f64::from_bits(s.to_bits() - 1);
            work.max(force.dot(&(rotation * self.at(left).2)))
        } else {
            work
        }
    }
    pub fn rotation_basis(&self, r: &Matrix3<f64>) -> Vec<Vector3<f64>> {
        if self.is_curved {
            return vec![Vector3::x(), Vector3::y(), Vector3::z()];
        }
        let axis = r * self.long;
        let reference = if axis.x.abs() < 0.8 {
            Vector3::x()
        } else {
            Vector3::y()
        };
        let first = (reference - axis * axis.dot(&reference)).normalize();
        vec![first, axis.cross(&first)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rod_beams::BeamProperties;

    #[test]
    fn a_contour_knot_checks_both_one_sided_curvatures() {
        // Chords 1 and sqrt(2), secants (0,0,1) and (1,0,1).
        // The middle tangent is (1,0,2)/(1+sqrt(2)). Hermite's
        // x curvature is 4/(1+sqrt(2)) on the left and 2sqrt(2)-2
        // on the right. Their signs reverse with the applied force.
        let root_two = 2.0_f64.sqrt();
        let curve = Curve::new(&Rod {
            points_mm: vec![Vector3::zeros(), Vector3::z(), Vector3::new(1.0, 0.0, 2.0)],
            properties: BeamProperties {
                young_modulus_mpa: 1.0,
                shear_modulus_mpa: 1.0,
                diameter_mm: 1.0,
                shear_coefficient: 1.0,
            },
        })
        .unwrap();
        assert!((curve.at(1.0).2.x - (2.0 * root_two - 2.0)).abs() < 1e-14);
        assert!(
            (curve.curvature_work(1.0, &Matrix3::identity(), Vector3::x())
                - 4.0 / (1.0 + root_two))
                .abs()
                < 1e-14
        );
        assert!(
            (curve.curvature_work(1.0, &Matrix3::identity(), -Vector3::x()) + 2.0 * root_two - 2.0)
                .abs()
                < 1e-14
        );
    }
    #[test]
    fn linear_end_continuations_preserve_the_original_contour_and_frame() {
        let rod = Rod {
            points_mm: vec![Vector3::zeros(), Vector3::z(), Vector3::new(1.0, 0.0, 2.0)],
            properties: BeamProperties {
                young_modulus_mpa: 1.0,
                shear_modulus_mpa: 1.0,
                diameter_mm: 1.0,
                shear_coefficient: 1.0,
            },
        };
        let original = rod.points_mm.clone();
        let curve = Curve::new(&rod).unwrap();
        let knots = curve.knots.clone();
        let length = curve.length();
        let long = curve.long;
        let is_curved = curve.is_curved;
        let centre = curve.centre();
        let samples: Vec<_> = [0.0, 0.4, 1.0, 1.7, length].map(|s| curve.at(s)).into();
        let first = curve.at(0.0);
        let last = curve.at(length);
        for extra in [0.5, 20.0, 40.0] {
            let before = curve.at(-extra);
            let after = curve.at(length + extra);
            assert!((before.0 - (first.0 - first.1.normalize() * extra)).norm() < 1e-14);
            assert!((after.0 - (last.0 + last.1.normalize() * extra)).norm() < 1e-14);
            assert!((before.1.norm() - 1.0).abs() < 1e-14);
            assert!((after.1.norm() - 1.0).abs() < 1e-14);
            assert_eq!(before.2, Vector3::zeros());
            assert_eq!(after.2, Vector3::zeros());
            assert!((before.1 - first.1).norm() < 1e-14);
            assert!((after.1 - last.1).norm() < 1e-14);
        }
        assert_eq!(rod.points_mm, original);
        assert_eq!(curve.points, original);
        assert_eq!(curve.knots, knots);
        assert_eq!(curve.length(), length);
        assert_eq!(curve.long, long);
        assert_eq!(curve.is_curved, is_curved);
        assert_eq!(curve.centre(), centre);
        for (index, s) in [0.0, 0.4, 1.0, 1.7, length].into_iter().enumerate() {
            assert_eq!(curve.at(s), samples[index]);
        }
    }

    #[test]
    fn original_ends_check_interior_and_zero_continuation_curvature() {
        let curve = Curve::new(&Rod {
            points_mm: vec![Vector3::zeros(), Vector3::z(), Vector3::new(1.0, 0.0, 2.0)],
            properties: BeamProperties {
                young_modulus_mpa: 1.0,
                shear_modulus_mpa: 1.0,
                diameter_mm: 1.0,
                shear_coefficient: 1.0,
            },
        })
        .unwrap();
        for s in [0.0, curve.length()] {
            for force in [Vector3::x(), -Vector3::x()] {
                let interior = force.dot(&curve.at(s).2);
                assert_eq!(
                    curve.curvature_work(s, &Matrix3::identity(), force),
                    interior.max(0.0)
                );
            }
        }
    }
}
