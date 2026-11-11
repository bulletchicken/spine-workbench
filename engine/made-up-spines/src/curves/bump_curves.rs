//! Bump profiles `u·exp(−u²/2)·√e` for tilt, twist and wedge along the spine.
//!
//! Each curve `c` has its end vertebrae at `u = ±1`, where `u_c = (t − a_c) /
//! s_c` is the distance from its apex `a_c` in half-spans `s_c` (`t` counts
//! vertebrae down from T1). One bump shapes everything, the tilt by the
//! bump and the twist and wedges by its slope:
//!
//! ```text
//! tilt   θ(t) = Σ_c A_c · b(u_c)              b(u) = u·exp(−u²/2)·√e
//! twist  ψ(t) = −Σ_c σ_c ρ_c C_c · b′(u_c) / b′(0)
//!                                             b′(u) / b′(0) = (1 − u²)·exp(−u²/2)
//! ```
//!
//! `b` has its extremes at `u = ±1`, where `b(±1) = ±1`, so with one curve
//! `A = σ·C/2` puts exactly the Cobb angle `C` between the end vertebrae
//! (`σ = +1` for a curve convex to the patient's right, whose tilt is
//! negative above the apex). The wedges follow the tilt's change across
//! each vertebra (see `stack_from_l5`), which is the bump's slope: largest
//! at the apex, as in real curves.
//!
//! The twist takes the bump's slope rather than the bump itself, like the
//! wedges: a vertebra turns most at the apex of its curve and not at all at
//! its end vertebrae (the slope is zero at `u = ±1`), where `b` itself
//! would turn it most and leave the apex unturned. Past the end vertebrae
//! it turns somewhat the other way, at most 0.45 of the apical twist (the
//! slope's least, `−2·e^(−3/2)` at `u = √3`), as junctional vertebrae
//! often do.
//!
//! A bump's tails reach past its end vertebrae (`b(2) ≈ 0.45`), so next to
//! another curve the plain amplitudes `σ·C/2` would measure the wrong
//! angles. The amplitudes are therefore solved together, so that each
//! curve's tilt difference between its own end vertebrae is exactly its
//! Cobb angle.
//!
//! The tails also reach beyond the outermost end vertebrae, where they
//! could carry the tilt on past an end, which would then no longer be the
//! most tilted vertebra, and would leave the last lumbar vertebra tilted on
//! the sacrum (by 20° under a 90° thoracic curve), more than the
//! lumbosacral disc can take. So, as with the splines:
//!
//! - the whole tilt is shifted so the lowest curve is centred on zero;
//! - beyond the outermost end vertebrae the tilt never passes the end's
//!   own tilt in the direction the curve was going;
//! - from half a vertebra below the lowest end vertebra (its lower
//!   endplate) the tilt is blended back to zero at the top of the sacrum
//!   with a smoothstep, whose slope is zero where it starts.

use super::Curve;
use nalgebra::{DMatrix, DVector};

/// `√e`, so that `b(±1) = ±1`.
const ROOT_E: f64 = 1.648_721_270_700_128_2;

/// The bump `u·exp(−u²/2)·√e`.
pub fn bump(u: f64) -> f64 {
    u * libm::exp(-0.5 * u * u) * ROOT_E
}

/// The bump's slope over its slope at the apex, `(1 − u²)·exp(−u²/2)`:
/// 1 at the apex and 0 at the end vertebrae.
pub fn slope(u: f64) -> f64 {
    (1.0 - u * u) * libm::exp(-0.5 * u * u)
}

/// Bump-shaped tilt and twist for a set of curves.
#[derive(Clone, Debug)]
pub struct BumpCurves {
    curves: Vec<Curve>,
    amplitudes: Vec<f64>,
    /// Subtracted from the bumps' sum, to centre the lowest curve.
    offset: f64,
    /// The highest end vertebra, its tilt, and which way the tilt would be
    /// passing it going up (+1 rising).
    top: Option<(f64, f64, f64)>,
    /// The lowest end vertebra, its tilt, which way the tilt would be
    /// passing it going down, and the top of the sacrum.
    bottom: Option<(f64, f64, f64, f64)>,
}

impl BumpCurves {
    /// The bumps for `curves`, with amplitudes solved so each curve's tilt
    /// difference between its end vertebrae is its Cobb angle, blended to
    /// zero at `sacrum` (the position of the top of the sacrum). `None`
    /// when the curves leave the amplitudes undetermined (two curves with
    /// the same end vertebrae).
    pub fn new(curves: &[Curve], sacrum: f64) -> Option<Self> {
        let n = curves.len();
        let difference = |k: &Curve, c: &Curve| {
            bump(half_spans(k, c.lower_end)) - bump(half_spans(k, c.upper_end))
        };
        let matrix = DMatrix::from_fn(n, n, |row, column| {
            difference(&curves[column], &curves[row])
        });
        let target = DVector::from_fn(n, |row, _| {
            curves[row].sign() * curves[row].cobb_deg.to_radians()
        });
        let amplitudes = if n == 0 {
            Vec::new()
        } else {
            let solved = matrix.lu().solve(&target)?;
            if solved.iter().any(|a| !a.is_finite()) {
                return None;
            }
            solved.iter().copied().collect()
        };
        let mut bumps = BumpCurves {
            curves: curves.to_vec(),
            amplitudes,
            offset: 0.0,
            top: None,
            bottom: None,
        };
        let highest = curves
            .iter()
            .min_by(|a, b| a.upper_end.total_cmp(&b.upper_end));
        let lowest = curves
            .iter()
            .max_by(|a, b| a.lower_end.total_cmp(&b.lower_end));
        if let (Some(highest), Some(lowest)) = (highest, lowest) {
            let lower_tilt = 0.5 * lowest.sign() * lowest.cobb_deg.to_radians();
            bumps.offset = bumps.sum(lowest.lower_end) - lower_tilt;
            let upper_tilt = bumps.sum(highest.upper_end) - bumps.offset;
            bumps.top = Some((highest.upper_end, upper_tilt, -highest.sign()));
            bumps.bottom = Some((lowest.lower_end, lower_tilt, lowest.sign(), sacrum));
        }
        Some(bumps)
    }

    /// The bumps' plain sum at `t`.
    fn sum(&self, t: f64) -> f64 {
        self.curves
            .iter()
            .zip(&self.amplitudes)
            .map(|(curve, amplitude)| amplitude * bump(half_spans(curve, t)))
            .sum()
    }

    /// The coronal tilt at `t`, radians.
    pub fn tilt(&self, t: f64) -> f64 {
        let tilt = self.sum(t) - self.offset;
        let held = |end: f64, way: f64| {
            if (tilt - end) * way > 0.0 { end } else { tilt }
        };
        match (self.top, self.bottom) {
            (Some((upper, end, way)), _) if t < upper => held(end, way),
            (_, Some((lower, end, way, sacrum))) if t > lower => {
                let from = lower + 0.5;
                let s = if t <= from {
                    0.0
                } else if sacrum > from {
                    ((t - from) / (sacrum - from)).min(1.0)
                } else {
                    1.0
                };
                held(end, way) * (1.0 - s * s * (3.0 - 2.0 * s))
            }
            _ => tilt,
        }
    }

    /// The twist at `t`, radians: negative (front turned to the patient's
    /// right) at the apex of a curve convex to the right.
    pub fn twist(&self, t: f64) -> f64 {
        self.curves
            .iter()
            .map(|curve| curve.apical_twist() * slope(half_spans(curve, t)))
            .sum()
    }
}

/// `u = (t − a) / s` for a curve whose apex sits midway between its ends.
fn half_spans(curve: &Curve, t: f64) -> f64 {
    let apex = 0.5 * (curve.upper_end + curve.lower_end);
    let half_span = 0.5 * (curve.lower_end - curve.upper_end);
    (t - apex) / half_span
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curves::Curve;
    use shapes::made_up::CurveRegion;

    fn curve(upper_end: f64, lower_end: f64, cobb_deg: f64, convex_right: bool) -> Curve {
        Curve {
            region: CurveRegion::MainThoracic,
            upper_end,
            lower_end,
            apex: 0.5 * (upper_end + lower_end),
            cobb_deg,
            convex_right,
            twist_per_cobb: 0.3,
        }
    }

    #[test]
    fn the_bump_peaks_at_one_with_slope_root_e_at_zero() {
        assert!((bump(1.0) - 1.0).abs() < 1e-15);
        assert!((bump(-1.0) + 1.0).abs() < 1e-15);
        assert_eq!(bump(0.0), 0.0);
        // Central difference: b'(0) = √e ≈ 1.6487.
        let slope = (bump(1e-6) - bump(-1e-6)) / 2e-6;
        assert!((slope - 1.648_721_270_7).abs() < 1e-8, "{slope}");
        // Its extremes are at ±1: smaller on both sides.
        for u in [0.9, 1.1, 0.5, 2.0] {
            assert!(bump(u) < 1.0);
        }
    }

    #[test]
    fn one_curve_gets_half_its_cobb_angle_either_side() {
        // Worked by hand: one curve, b(±1) = ±1, so A = σ·C/2 and the tilt
        // runs from −20° at the upper end to +20° at the lower end.
        let bumps =
            BumpCurves::new(&[curve(5.0, 11.0, 40.0, true)], 17.0).expect("one curve solves");
        assert!((bumps.tilt(5.0).to_degrees() + 20.0).abs() < 1e-12);
        assert!((bumps.tilt(11.0).to_degrees() - 20.0).abs() < 1e-12);
        assert!(bumps.tilt(8.0).abs() < 1e-15);
        // Convex left: the signs swap.
        let left =
            BumpCurves::new(&[curve(5.0, 11.0, 30.0, false)], 17.0).expect("one curve solves");
        assert!((left.tilt(5.0).to_degrees() - 15.0).abs() < 1e-12);
    }

    #[test]
    fn twist_turns_toward_the_convexity_most_at_the_apex() {
        let bumps =
            BumpCurves::new(&[curve(5.0, 11.0, 40.0, true)], 17.0).expect("one curve solves");
        // −ρ·C at the apex = −0.3 · 40° = −12°.
        assert!((bumps.twist(8.0).to_degrees() + 12.0).abs() < 1e-12);
        // Not turned at the end vertebrae.
        assert!(bumps.twist(5.0).abs() < 1e-15 && bumps.twist(11.0).abs() < 1e-15);
        assert!(bumps.twist(7.0) < 0.0 && bumps.twist(9.0) < 0.0);
        // The other way beyond, at most 0.45 of the apical twist: the
        // slope's least is −2·e^(−3/2) ≈ −0.446 at u = √3, times 12° is
        // under 5.4°.
        let beyond = (0..40).map(|i| bumps.twist(11.0 + 0.1 * f64::from(i)).to_degrees());
        assert!(beyond.fold(f64::MIN, f64::max) < 5.4);
    }

    #[test]
    fn neighbouring_curves_each_measure_their_own_angle() {
        let curves = [curve(3.0, 10.0, 55.0, true), curve(10.0, 15.0, 45.0, false)];
        let bumps = BumpCurves::new(&curves, 17.0).expect("two curves solve");
        let thoracic = (bumps.tilt(10.0) - bumps.tilt(3.0)).to_degrees();
        let lumbar = (bumps.tilt(15.0) - bumps.tilt(10.0)).to_degrees();
        assert!((thoracic - 55.0).abs() < 1e-9, "{thoracic}");
        assert!((lumbar + 45.0).abs() < 1e-9, "{lumbar}");
        // The plain amplitudes σ·C/2 would not: the shared end vertebra
        // takes both curves' halves, about 55 + 22.5 ≈ 77°.
        let lumbar_u = |t: f64| half_spans(&curves[1], t);
        let plain =
            27.5 * (bump(1.0) - bump(-1.0)) - 22.5 * (bump(lumbar_u(10.0)) - bump(lumbar_u(3.0)));
        assert!((plain - 77.4).abs() < 0.2, "{plain}");
    }

    #[test]
    fn the_tilt_returns_to_zero_at_the_sacrum_without_passing_the_end() {
        let bumps = BumpCurves::new(&[curve(3.0, 12.0, 90.0, true)], 17.0).expect("one curve");
        assert_eq!(bumps.tilt(17.0), 0.0);
        assert!((bumps.tilt(12.0).to_degrees() - 45.0).abs() < 1e-12);
        let below: Vec<f64> = (0..=50)
            .map(|i| bumps.tilt(12.0 + 0.1 * f64::from(i)))
            .collect();
        assert!(below.windows(2).all(|pair| pair[1] <= pair[0]));
        // Above the lowest end nothing changes.
        assert!((bumps.tilt(2.0) - 45f64.to_radians() * bump(-11.0 / 9.0)).abs() < 1e-15);
    }

    #[test]
    fn a_straight_spine_has_no_tilt_or_twist() {
        let bumps = BumpCurves::new(&[], 17.0).expect("no curves solve");
        assert_eq!(bumps.tilt(4.0), 0.0);
        assert_eq!(bumps.twist(4.0), 0.0);
    }
}
