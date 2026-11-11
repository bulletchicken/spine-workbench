//! A second, different curve generator built from splines.
//!
//! The tilt is a monotone cubic Hermite spline through knots at the end
//! vertebrae and apex of every curve: neighbouring end vertebrae differ by
//! exactly the curve's Cobb angle, each held for half a vertebra either
//! side so both its endplates keep its tilt, and each apex sits at the
//! midpoint value (anywhere between the ends, so a curve may be lopsided).
//! The slopes are
//! Fritsch and Butland's weighted harmonic mean (SIAM J. Sci. Stat.
//! Comput. 1984), zero wherever the knots turn, so the spline never
//! overshoots a knot.
//!
//! Past the outermost end vertebrae the tilt must not carry on the way it
//! was going, or those ends would stop being the most tilted vertebrae and
//! the curves would measure more than asked. Below, it has to be straight
//! again at the top of the sacrum, so the lowest curve is centred on zero:
//! its lower end is then tilted away from zero, and coming back to zero
//! turns back. Above, the tilt returns to zero over three vertebrae when
//! that turns back too, and otherwise stays as it is at the top end.
//!
//! The twist is a smooth hump per curve, zero at its end vertebrae and
//! `−σ·ρ·C` at its apex, again a Hermite spline with zero slopes.
//!
//! Nothing here shares a formula with [`super::bump_curves`]: tests can
//! build spines both ways and check that what they measure does not depend
//! on how the spine was bent.

use super::Curve;

/// How many vertebrae past the outermost end vertebrae the tilt takes to
/// return to zero.
const RETURN_SPAN: f64 = 3.0;

/// One knot: position along the spine and value.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Knot {
    t: f64,
    value: f64,
}

/// A piecewise cubic Hermite curve through knots in increasing `t`, with
/// a slope at each knot; constant beyond the first and last knots.
#[derive(Clone, Debug)]
struct Hermite {
    knots: Vec<Knot>,
    slopes: Vec<f64>,
}

impl Hermite {
    /// Fritsch–Butland slopes: zero at the two ends and wherever the
    /// secants change sign, otherwise the weighted harmonic mean of the
    /// two secants, which keeps every piece monotone.
    fn monotone(knots: Vec<Knot>) -> Self {
        let n = knots.len();
        let mut slopes = vec![0.0; n];
        for i in 1..n.saturating_sub(1) {
            let (h0, h1) = (knots[i].t - knots[i - 1].t, knots[i + 1].t - knots[i].t);
            let d0 = (knots[i].value - knots[i - 1].value) / h0;
            let d1 = (knots[i + 1].value - knots[i].value) / h1;
            if d0 * d1 > 0.0 {
                slopes[i] = 3.0 * (h0 + h1) / ((2.0 * h1 + h0) / d0 + (h1 + 2.0 * h0) / d1);
            }
        }
        Hermite { knots, slopes }
    }

    /// Every slope zero: smooth humps between knots.
    fn flat(knots: Vec<Knot>) -> Self {
        let slopes = vec![0.0; knots.len()];
        Hermite { knots, slopes }
    }

    fn at(&self, t: f64) -> f64 {
        let (Some(first), Some(last)) = (self.knots.first(), self.knots.last()) else {
            return 0.0;
        };
        if t <= first.t {
            return first.value;
        }
        if t >= last.t {
            return last.value;
        }
        let i = self
            .knots
            .windows(2)
            .position(|pair| t < pair[1].t)
            .unwrap_or(self.knots.len() - 2);
        let (k0, k1) = (self.knots[i], self.knots[i + 1]);
        let h = k1.t - k0.t;
        let s = (t - k0.t) / h;
        let (s2, s3) = (s * s, s * s * s);
        (2.0 * s3 - 3.0 * s2 + 1.0) * k0.value
            + (s3 - 2.0 * s2 + s) * h * self.slopes[i]
            + (-2.0 * s3 + 3.0 * s2) * k1.value
            + (s3 - s2) * h * self.slopes[i + 1]
    }
}

/// Spline-shaped tilt and twist for a set of curves.
#[derive(Clone, Debug)]
pub struct SplineCurves {
    tilt: Hermite,
    twist: Hermite,
}

impl SplineCurves {
    /// The splines for `curves`, the tilt back to zero no lower than
    /// `sacrum` (the position of the top of the sacrum). `None` unless the
    /// curves are listed top to bottom, each with its apex strictly
    /// between its ends, and no two overlap (they may share an end
    /// vertebra).
    pub fn new(curves: &[Curve], sacrum: f64) -> Option<Self> {
        let well_placed = curves
            .iter()
            .all(|c| c.upper_end < c.apex && c.apex < c.lower_end);
        let in_order = curves
            .windows(2)
            .all(|pair| pair[0].lower_end <= pair[1].upper_end);
        if !well_placed || !in_order {
            return None;
        }
        Some(SplineCurves {
            tilt: Hermite::monotone(tilt_knots(curves, sacrum)),
            twist: Hermite::flat(twist_knots(curves)),
        })
    }

    /// The coronal tilt at `t`, radians.
    pub fn tilt(&self, t: f64) -> f64 {
        self.tilt.at(t)
    }

    /// The twist at `t`, radians.
    pub fn twist(&self, t: f64) -> f64 {
        self.twist.at(t)
    }
}

/// Knots at every end vertebra (held for half a vertebra either side)
/// and apex, the lowest curve centred on zero, then the way back to zero
/// beyond the outermost ends (see the module documentation).
fn tilt_knots(curves: &[Curve], sacrum: f64) -> Vec<Knot> {
    // Each end vertebra, then each apex, with its value before centring.
    let mut points: Vec<(Knot, bool)> = Vec::new();
    let mut level = 0.0;
    let mut last_rise = 0.0;
    for curve in curves {
        let rise = curve.sign() * curve.cobb_deg.to_radians();
        if points
            .last()
            .is_none_or(|(last, _)| last.t < curve.upper_end)
        {
            points.push((
                Knot {
                    t: curve.upper_end,
                    value: level,
                },
                true,
            ));
        }
        points.push((
            Knot {
                t: curve.apex,
                value: level + 0.5 * rise,
            },
            false,
        ));
        level += rise;
        last_rise = rise;
        points.push((
            Knot {
                t: curve.lower_end,
                value: level,
            },
            true,
        ));
    }
    let centre = level - 0.5 * last_rise;
    let mut knots: Vec<Knot> = Vec::new();
    for (knot, is_end) in points {
        let value = knot.value - centre;
        if is_end {
            knots.push(Knot {
                t: knot.t - 0.5,
                value,
            });
            knots.push(Knot {
                t: knot.t + 0.5,
                value,
            });
        } else {
            knots.push(Knot { t: knot.t, value });
        }
    }
    if let (Some(first), Some(apex), Some(last)) = (
        knots.first().copied(),
        knots.get(2).copied(),
        knots.last().copied(),
    ) {
        let turns_back = first.value * (first.value - apex.value) > 0.0;
        knots.insert(
            0,
            Knot {
                t: first.t + 0.5 - RETURN_SPAN,
                value: if turns_back { 0.0 } else { first.value },
            },
        );
        if last.t < sacrum {
            knots.push(Knot {
                t: (last.t - 0.5 + RETURN_SPAN).min(sacrum).max(last.t + 0.5),
                value: 0.0,
            });
        }
    }
    knots
}

/// Zero at every end vertebra, the apical twist at every apex.
fn twist_knots(curves: &[Curve]) -> Vec<Knot> {
    let mut knots: Vec<Knot> = Vec::new();
    for curve in curves {
        if knots.last().is_none_or(|last| last.t < curve.upper_end) {
            knots.push(Knot {
                t: curve.upper_end,
                value: 0.0,
            });
        }
        knots.push(Knot {
            t: curve.apex,
            value: curve.apical_twist(),
        });
        knots.push(Knot {
            t: curve.lower_end,
            value: 0.0,
        });
    }
    knots
}

#[cfg(test)]
mod tests {
    use super::*;
    use shapes::made_up::CurveRegion;

    fn curve(upper_end: f64, apex: f64, lower_end: f64, cobb_deg: f64, right: bool) -> Curve {
        Curve {
            region: CurveRegion::MainThoracic,
            upper_end,
            lower_end,
            apex,
            cobb_deg,
            convex_right: right,
            twist_per_cobb: 0.25,
        }
    }

    fn degrees(splines: &SplineCurves, t: f64) -> f64 {
        splines.tilt(t).to_degrees()
    }

    #[test]
    fn one_lopsided_curve_measures_its_angle_between_its_ends() {
        let splines =
            SplineCurves::new(&[curve(4.0, 9.0, 11.0, 40.0, true)], 17.0).expect("a valid curve");
        assert!((degrees(&splines, 4.0) + 20.0).abs() < 1e-12);
        assert!((degrees(&splines, 11.0) - 20.0).abs() < 1e-12);
        assert!(degrees(&splines, 9.0).abs() < 1e-12);
        // Back to straight three vertebrae beyond each end.
        assert!(splines.tilt(1.0).abs() < 1e-15 && splines.tilt(14.0).abs() < 1e-15);
    }

    #[test]
    fn the_spline_never_passes_its_end_vertebrae() {
        let curves = [
            curve(1.0, 3.2, 5.0, 25.0, false),
            curve(5.0, 8.4, 11.0, 70.0, true),
            curve(11.0, 13.0, 16.0, 45.0, false),
        ];
        let splines = SplineCurves::new(&curves, 20.0).expect("valid curves");
        let (low, high) = (0..=400)
            .map(|step| degrees(&splines, -3.0 + 0.05 * f64::from(step)))
            .fold((f64::MAX, f64::MIN), |(lo, hi), v| (lo.min(v), hi.max(v)));
        let ends = [1.0, 5.0, 11.0, 16.0].map(|t| degrees(&splines, t));
        let end_low = ends.iter().copied().fold(f64::MAX, f64::min);
        let end_high = ends.iter().copied().fold(f64::MIN, f64::max);
        assert!((low - end_low).abs() < 1e-9 && (high - end_high).abs() < 1e-9);
        assert!((ends[1] - ends[0] + 25.0).abs() < 1e-9);
        assert!((ends[2] - ends[1] - 70.0).abs() < 1e-9);
        assert!((ends[3] - ends[2] + 45.0).abs() < 1e-9);
        // The lowest curve is centred on zero.
        assert!((ends[2] - 22.5).abs() < 1e-9 && (ends[3] + 22.5).abs() < 1e-9);
    }

    #[test]
    fn small_outer_curves_measure_no_more_than_asked() {
        // A small upper curve next to a large one: its upper end must stay
        // its most tilted vertebra, so the largest tilt difference within a
        // vertebra of its ends is exactly its Cobb angle.
        let curves = [
            curve(1.0, 2.3, 4.0, 8.0, false),
            curve(4.0, 7.9, 11.0, 28.0, true),
            curve(11.0, 13.0, 15.0, 13.0, false),
        ];
        let splines = SplineCurves::new(&curves, 17.0).expect("valid curves");
        for c in &curves {
            let steps = (20.0 * (c.lower_end - c.upper_end + 2.0)) as u32;
            let samples: Vec<f64> = (0..=steps)
                .map(|i| degrees(&splines, c.upper_end - 1.0 + 0.05 * f64::from(i)))
                .collect();
            let low = samples.iter().copied().fold(f64::MAX, f64::min);
            let high = samples.iter().copied().fold(f64::MIN, f64::max);
            assert!(
                (high - low - c.cobb_deg).abs() < 1e-9,
                "{c:?}: {}",
                high - low
            );
        }
    }

    #[test]
    fn between_knots_the_tilt_only_rises_or_only_falls() {
        let splines =
            SplineCurves::new(&[curve(3.0, 5.0, 12.0, 60.0, true)], 17.0).expect("a valid curve");
        let samples: Vec<f64> = (0..=180)
            .map(|i| splines.tilt(3.0 + 0.05 * f64::from(i)))
            .collect();
        assert!(samples.windows(2).all(|pair| pair[1] >= pair[0] - 1e-15));
    }

    #[test]
    fn the_tilt_is_straight_again_by_the_top_of_the_sacrum() {
        let splines =
            SplineCurves::new(&[curve(10.0, 12.0, 15.0, 45.0, false)], 17.0).expect("a curve");
        assert_eq!(splines.tilt(17.0), 0.0);
        assert!((splines.tilt(15.0).to_degrees() + 22.5).abs() < 1e-12);
    }

    #[test]
    fn twist_peaks_at_the_apex_and_vanishes_at_the_ends() {
        let splines =
            SplineCurves::new(&[curve(4.0, 9.0, 11.0, 40.0, true)], 17.0).expect("a valid curve");
        assert!((splines.twist(9.0).to_degrees() + 10.0).abs() < 1e-12);
        assert_eq!(splines.twist(4.0), 0.0);
        assert_eq!(splines.twist(11.0), 0.0);
        assert_eq!(splines.twist(15.0), 0.0);
        assert!(splines.twist(6.0) < 0.0 && splines.twist(6.0) > splines.twist(9.0));
    }

    #[test]
    fn misplaced_curves_are_refused() {
        assert!(SplineCurves::new(&[curve(4.0, 3.0, 11.0, 40.0, true)], 17.0).is_none());
        let overlapping = [
            curve(1.0, 4.0, 8.0, 30.0, false),
            curve(6.0, 9.0, 12.0, 30.0, true),
        ];
        assert!(SplineCurves::new(&overlapping, 17.0).is_none());
    }
}
