//! The typical twist of each vertebra in the largest curve: the typical
//! spine's apex twist for the curve's size, turned toward the convex side
//! and shared along the curve in proportion to each vertebra's sideways
//! distance from the line between the end vertebrae. Only the largest curve
//! is given one, because the cohort's twist describes the major curve. Its
//! end vertebrae and every other vertebra keep the twist of no curve.
//!
//! The body turns toward the convexity. Twist rotZ(+ψ) turns the front of
//! the body toward the patient's left, so a curve convex to the left gives
//! its vertebrae a positive twist and one convex to the right a negative
//! twist, the rule the made-up spines follow too.

use crate::front_curves::Curve;
use crate::toward::Spread;
use crate::typical_spine::{Typical, TypicalSpine};

/// Replace the twist of each vertebra inside the largest curve, keeping
/// the spread the typical spine gives at that curve's apex.
pub fn apply(typical: &mut Typical, largest: Option<&Curve>, spine: &TypicalSpine) {
    let Some(curve) = largest else {
        return;
    };
    let apex = spine.apex_twist_rad(curve.size_rad.to_degrees());
    let toward = if curve.convex_left { 1.0 } else { -1.0 };
    for &(index, share) in &curve.inside {
        typical.levels[index].twist_rad = Spread {
            mean: toward * apex.mean * share,
            sd: apex.sd,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typical_spine::built_in;
    use shapes::levels::Level;

    #[test]
    fn a_curve_convex_to_the_left_turns_its_bodies_left_by_hand() {
        // A 28 degree curve from T5 (index 1) to T8 (index 4), convex to
        // the patient's left, apex T7 (index 3) and T6 at 13/17 of its
        // sideways distance. The typical spine's row for 28 degrees gives
        // an apex twist of 9.3 degrees, spread 5.1. Turned toward the left,
        // which is positive twist: T7 +9.3, T6 +9.3 · 13/17 = +7.112,
        // and the end vertebrae and those outside keep zero, spread 4.5.
        let spine = built_in();
        let levels = [
            Level::T4,
            Level::T5,
            Level::T6,
            Level::T7,
            Level::T8,
            Level::T9,
        ];
        let mut typical = spine.for_levels(levels.into_iter(), 28.0, false);
        let curve = Curve {
            upper: 1,
            lower: 4,
            apex: 3,
            size_rad: 28.0_f64.to_radians(),
            convex_left: true,
            inside: vec![(2, 13.0 / 17.0), (3, 1.0)],
        };
        apply(&mut typical, Some(&curve), spine);
        let twist: Vec<Spread> = typical
            .levels
            .iter()
            .map(|level| Spread {
                mean: level.twist_rad.mean.to_degrees(),
                sd: level.twist_rad.sd.to_degrees(),
            })
            .collect();
        for (got, expected) in twist.iter().zip([0.0, 0.0, 7.1118, 9.3, 0.0, 0.0]) {
            assert!((got.mean - expected).abs() < 1e-4, "{twist:?}");
        }
        for (got, expected) in twist.iter().zip([4.5, 4.5, 5.1, 5.1, 4.5, 4.5]) {
            assert!((got.sd - expected).abs() < 1e-9, "{twist:?}");
        }
        // Convex to the right, the same curve turns them the other way.
        let mut right = spine.for_levels(levels.into_iter(), 28.0, false);
        let mirrored = Curve {
            convex_left: false,
            ..curve
        };
        apply(&mut right, Some(&mirrored), spine);
        assert!((right.levels[3].twist_rad.mean.to_degrees() + 9.3).abs() < 1e-9);
    }

    #[test]
    fn no_curve_leaves_every_twist_at_zero() {
        let spine = built_in();
        let mut typical =
            spine.for_levels([Level::T1, Level::T2, Level::T3].into_iter(), 0.0, false);
        apply(&mut typical, None, spine);
        assert!(typical.levels.iter().all(|l| l.twist_rad.mean == 0.0));
    }

    #[test]
    fn apex_twist_is_interpolated_and_held_at_the_ends() {
        // Rows (15, 6.7, 4.5) and (28, 9.3, 5.1): at 21.5 degrees, halfway,
        // 8.0 give or take 4.8. Beyond the last row (73.7) it is held.
        let spine = built_in();
        let between = spine.apex_twist_rad(21.5);
        assert!((between.mean.to_degrees() - 8.0).abs() < 1e-9);
        assert!((between.sd.to_degrees() - 4.8).abs() < 1e-9);
        let beyond = spine.apex_twist_rad(120.0);
        assert!((beyond.mean.to_degrees() - 21.3).abs() < 1e-9);
        assert_eq!(spine.apex_twist_rad(0.0).mean, 0.0);
    }
}
