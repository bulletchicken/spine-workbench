//! The curve profiles a made-up spine is bent with. Two different generators,
//! so a test can never agree with itself.
//!
//! Positions along the spine are counted in vertebrae, `t`, downward from
//! T1 at 0: the centre of the `i`-th vertebra below T1 is at `t = i`, its
//! upper endplate at `i − ½`. C7 is at −1 and the cervical spine runs on
//! upward; the lumbar spine follows the last thoracic vertebra.
//!
//! - [`CoronalProfile`] gives the tilt (the rotation about the patient's
//!   forward axis, which bends the spine to the side) and the twist at
//!   any `t`, from either generator.
//! - [`SagittalProfile`] gives the forward heading: the sacral slope,
//!   lordosis, kyphosis and the cervical curve.

pub mod bump_curves;
pub mod spline_curves;

use bump_curves::BumpCurves;
use shapes::made_up::CurveRegion;
use spline_curves::SplineCurves;

/// One scoliotic curve as a recipe asks for it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Curve {
    pub region: CurveRegion,
    /// The upper end vertebra, as a position `t`.
    pub upper_end: f64,
    /// The lower end vertebra.
    pub lower_end: f64,
    /// The apex, strictly between the ends. The bump generator always
    /// puts it midway; the spline generator honours it.
    pub apex: f64,
    /// The Cobb angle asked for, degrees.
    pub cobb_deg: f64,
    /// True when the curve bulges to the patient's right.
    pub convex_right: bool,
    /// Apical twist per degree of Cobb angle.
    pub twist_per_cobb: f64,
}

impl Curve {
    /// +1 for a curve convex to the right, −1 for one convex to the left.
    pub fn sign(&self) -> f64 {
        if self.convex_right { 1.0 } else { -1.0 }
    }

    /// The twist at the apex, radians: the body turns toward the
    /// convexity, so a curve convex to the right turns its front to the
    /// patient's right (negative).
    pub fn apical_twist(&self) -> f64 {
        -self.sign() * self.twist_per_cobb * self.cobb_deg.to_radians()
    }
}

/// Which generator bends a spine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurveGenerator {
    Bump,
    Spline,
}

/// Tilt and twist along the spine, from either generator.
#[derive(Clone, Debug)]
pub enum CoronalProfile {
    Bump(BumpCurves),
    Spline(SplineCurves),
}

impl CoronalProfile {
    /// The profile of `curves` from `generator`, straight again at
    /// `sacrum`, the position of the top of the sacrum. `None` when the
    /// curves cannot be shaped that way: see [`BumpCurves::new`] and
    /// [`SplineCurves::new`].
    pub fn new(generator: CurveGenerator, curves: &[Curve], sacrum: f64) -> Option<Self> {
        match generator {
            CurveGenerator::Bump => BumpCurves::new(curves, sacrum).map(CoronalProfile::Bump),
            CurveGenerator::Spline => SplineCurves::new(curves, sacrum).map(CoronalProfile::Spline),
        }
    }

    /// The coronal tilt at `t`, radians, the angle of `rotX`: positive
    /// where the column heads toward the patient's right going up.
    pub fn tilt(&self, t: f64) -> f64 {
        match self {
            CoronalProfile::Bump(bumps) => bumps.tilt(t),
            CoronalProfile::Spline(splines) => splines.tilt(t),
        }
    }

    /// The twist at `t`, radians, the angle of `rotZ`: positive where the
    /// front of the body turns toward the patient's left.
    pub fn twist(&self, t: f64) -> f64 {
        match self {
            CoronalProfile::Bump(bumps) => bumps.twist(t),
            CoronalProfile::Spline(splines) => splines.twist(t),
        }
    }
}

/// The forward heading of the column, `Φ(t)`, as the angle of `rotY`:
/// positive where the column heads forward going up.
///
/// It is the sacral slope at the top of the sacrum and changes going up
/// in four stretches: lordosis is spent over the lumbar spine (30%, 25%,
/// 20% and 15% of it in the four lowest segments, the rest up to the top
/// of L1), kyphosis over T4 to the last thoracic vertebra with a density
/// `exp(−((t − 6)/3)²)` that peaks at T7, a small forward lean over T1 to
/// T4, and the cervical lordosis over C2 to C7.
#[derive(Clone, Debug)]
pub struct SagittalProfile {
    sacral_slope: f64,
    stretches: Vec<Stretch>,
}

/// One stretch of the spine over which the heading changes by `change`
/// going up, from `low` (below) to `high` (above).
#[derive(Clone, Copy, Debug)]
struct Stretch {
    high: f64,
    low: f64,
    change: f64,
    shape: StretchShape,
}

#[derive(Clone, Copy, Debug)]
enum StretchShape {
    Even,
    Peaked { centre: f64, width: f64 },
}

/// The angles a sagittal profile is built from, all in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SagittalAngles {
    pub sacral_slope: f64,
    /// L1 top to S1 top.
    pub lordosis: f64,
    /// T4 top to the last thoracic vertebra's bottom.
    pub kyphosis: f64,
    /// The extra forward lean over T1 to T4.
    pub upper_thoracic: f64,
    /// C2 to C7, positive for the usual backward bend.
    pub cervical_lordosis: f64,
}

/// Where kyphosis is densest: T7.
const KYPHOSIS_PEAK: f64 = 6.0;
const KYPHOSIS_WIDTH: f64 = 3.0;
/// The top of T4.
const T4_TOP: f64 = 2.5;
/// The top of C2, six vertebrae above T1.
const C2_TOP: f64 = -6.5;

impl SagittalProfile {
    /// The heading for a spine with `thoracic` thoracic and `lumbar` lumbar
    /// vertebrae.
    pub fn new(angles: &SagittalAngles, thoracic: usize, lumbar: usize) -> Self {
        let last_lumbar = (thoracic + lumbar) as f64 - 1.0;
        let l1_top = thoracic as f64 - 0.5;
        let lordosis = angles.lordosis.to_radians();
        let even = |high: f64, low: f64, change: f64| Stretch {
            high,
            low,
            change,
            shape: StretchShape::Even,
        };
        let mut stretches = vec![
            even(last_lumbar, last_lumbar + 1.0, -0.30 * lordosis),
            even(last_lumbar - 1.0, last_lumbar, -0.25 * lordosis),
            even(last_lumbar - 2.0, last_lumbar - 1.0, -0.20 * lordosis),
            even(last_lumbar - 3.0, last_lumbar - 2.0, -0.15 * lordosis),
            even(l1_top, last_lumbar - 3.0, -0.10 * lordosis),
            Stretch {
                high: T4_TOP,
                low: l1_top,
                change: angles.kyphosis.to_radians(),
                shape: StretchShape::Peaked {
                    centre: KYPHOSIS_PEAK,
                    width: KYPHOSIS_WIDTH,
                },
            },
            even(-0.5, T4_TOP, angles.upper_thoracic.to_radians()),
            even(C2_TOP, -0.5, -angles.cervical_lordosis.to_radians()),
        ];
        stretches.retain(|stretch| stretch.high < stretch.low);
        SagittalProfile {
            sacral_slope: angles.sacral_slope.to_radians(),
            stretches,
        }
    }

    /// The heading at `t`, radians.
    pub fn heading(&self, t: f64) -> f64 {
        self.sacral_slope
            + self
                .stretches
                .iter()
                .map(|stretch| stretch.change * stretch.spent_above(t))
                .sum::<f64>()
    }
}

impl Stretch {
    /// How much of the stretch's change has been spent going up from below
    /// it to `t`: 0 below it, 1 above it.
    fn spent_above(&self, t: f64) -> f64 {
        if t >= self.low {
            return 0.0;
        }
        if t <= self.high {
            return 1.0;
        }
        match self.shape {
            StretchShape::Even => (self.low - t) / (self.low - self.high),
            StretchShape::Peaked { centre, width } => {
                let cumulative = |x: f64| libm::erf((x - centre) / width);
                (cumulative(self.low) - cumulative(t))
                    / (cumulative(self.low) - cumulative(self.high))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ANGLES: SagittalAngles = SagittalAngles {
        sacral_slope: 40.0,
        lordosis: 50.0,
        kyphosis: 40.0,
        upper_thoracic: 5.0,
        cervical_lordosis: 20.0,
    };

    fn degrees(profile: &SagittalProfile, t: f64) -> f64 {
        profile.heading(t).to_degrees()
    }

    #[test]
    fn the_heading_steps_through_each_angle_in_turn() {
        // Worked by hand for 12 thoracic and 5 lumbar vertebrae: the top
        // of S1 is at t = 17, L1's top at 11.5, T4's at 2.5.
        let profile = SagittalProfile::new(&ANGLES, 12, 5);
        assert!((degrees(&profile, 17.0) - 40.0).abs() < 1e-12);
        // 30% of the lordosis spent over L5: 40 − 15 at L5's centre.
        assert!((degrees(&profile, 16.0) - 25.0).abs() < 1e-12);
        // All of it by the top of L1: 40 − 50.
        assert!((degrees(&profile, 11.5) + 10.0).abs() < 1e-12);
        // All of the kyphosis by the top of T4: −10 + 40.
        assert!((degrees(&profile, 2.5) - 30.0).abs() < 1e-12);
        // Then the upper thoracic lean, then the cervical lordosis.
        assert!((degrees(&profile, -0.5) - 35.0).abs() < 1e-12);
        assert!((degrees(&profile, -6.5) - 15.0).abs() < 1e-12);
        assert!((degrees(&profile, -9.0) - 15.0).abs() < 1e-12);
    }

    #[test]
    fn kyphosis_is_densest_at_t7() {
        let profile = SagittalProfile::new(&ANGLES, 12, 5);
        let step = |t: f64| degrees(&profile, t - 0.5) - degrees(&profile, t + 0.5);
        assert!(step(6.0) > step(4.0) && step(6.0) > step(9.0));
    }

    #[test]
    fn counts_move_the_lumbar_stretches_with_the_last_vertebra() {
        let six_lumbar = SagittalProfile::new(&ANGLES, 12, 6);
        assert!((degrees(&six_lumbar, 18.0) - 40.0).abs() < 1e-12);
        assert!((degrees(&six_lumbar, 11.5) + 10.0).abs() < 1e-12);
        let eleven_thoracic = SagittalProfile::new(&ANGLES, 11, 5);
        assert!((degrees(&eleven_thoracic, 10.5) + 10.0).abs() < 1e-12);
        assert!((degrees(&eleven_thoracic, 2.5) - 30.0).abs() < 1e-12);
    }
}
