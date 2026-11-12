//! Curves in the front plane, read from the front film's corners before the
//! solve. Each vertebra's mid-line tilt runs from the middle of its right
//! side to the middle of its left side. The end vertebrae are the tilt's
//! extremes between changes of direction, the apex is the vertebra furthest
//! sideways from the line between the end vertebrae's centres, and a
//! curve's size is the tilt difference between its end vertebrae.
//!
//! A change of direction counts only once the tilt has come back from its
//! extreme by a threshold taken from the marks' own spread. A corner spread
//! of `s` mm along each axis moves the middle of a side by `s / √2` and the
//! line between the two middles, `L` mm long, by `s / L` radians of tilt; two
//! vertebrae's tilts then differ by `√2 · s / L` from noise alone. The
//! threshold is four of those, with the median `s / L` over the vertebrae:
//! among the 136 pairs of 17 straight vertebrae, noise alone reaches it with
//! a chance under 1 in 100 (136 times the 6.3 in 100,000 beyond four
//! standard deviations). A straight spine has no curves. A real curve
//! smaller than the threshold is not found; it would have chosen a side
//! profile and a twist close to those of no curve, since both are
//! interpolated from the data's row for no curve.
//!
//! The same film seen in a mirror finds exactly the mirrored curves. Each
//! tilt and centre is summed so that swapping the patient's left and right
//! gives exactly the opposite number, and tilts (or curve sizes) closer than
//! [`TIE_RAD`] count as equal, the first from the top winning. So a run of
//! equal tilts, such as a straight stretch at either end of a curve, gives
//! the same end vertebra from both sides, whatever the rounding in pixels.

use crate::corner_tilt;
use crate::problem::{Problem, millimetres};
use nalgebra::Vector2;

/// Standard deviations of a tilt difference that a change of direction
/// must exceed.
const DEVIATIONS: f64 = 4.0;

/// Tilts closer than this, in radians, are equal: far above the rounding
/// of pixels and millimetres (about 1e-15 radians), far below anything a
/// film shows.
const TIE_RAD: f64 = 1e-9;

/// One curve, by index into the problem's observations.
#[derive(Clone, Debug, PartialEq)]
pub struct Curve {
    pub upper: usize,
    pub lower: usize,
    pub apex: usize,
    /// The tilt difference between the end vertebrae, radians, positive.
    pub size_rad: f64,
    /// The curve bulges toward the patient's left.
    pub convex_left: bool,
    /// Each vertebra strictly between the end vertebrae with its sideways
    /// distance toward the convex side from the line between them, as a
    /// share of the apex's, never below zero.
    pub inside: Vec<(usize, f64)>,
}

/// One vertebra as the front film shows it.
#[derive(Clone, Copy, Debug)]
struct Body {
    index: usize,
    /// Sideways (toward the patient's left) and up, mm.
    centre: Vector2<f64>,
    tilt_rad: f64,
    /// One standard deviation of the tilt from the corner spread.
    tilt_sd_rad: f64,
}

impl Body {
    /// Corners in the order `supL, supR, infR, infL`, each sideways and up
    /// in mm, with one standard deviation of a corner along each axis.
    fn seen(index: usize, corners: [Vector2<f64>; 4], spread_mm: f64) -> Self {
        let [sup_l, sup_r, inf_r, inf_l] = corners;
        let across = corner_tilt::across(corners);
        Self {
            index,
            centre: ((sup_l + sup_r) + (inf_r + inf_l)) * 0.25,
            tilt_rad: corner_tilt::tilt_rad(corners),
            tilt_sd_rad: spread_mm / across.norm(),
        }
    }
}

/// The curves the front film's marks show, top to bottom.
pub fn find(problem: &Problem) -> Vec<Curve> {
    let bodies: Vec<Body> = problem
        .observations
        .iter()
        .enumerate()
        .filter_map(|(index, observation)| {
            let corners = observation
                .front?
                .corners()
                .map(|mark| millimetres(&problem.front, mark));
            Some(Body::seen(index, corners, observation.spread_mm[0]))
        })
        .collect();
    curves(&bodies)
}

/// The largest curve, the first from the top among equals, or `None` with
/// no curve.
pub fn largest(curves: &[Curve]) -> Option<&Curve> {
    curves.iter().fold(None, |best, curve| match best {
        Some(best) if curve.size_rad <= best.size_rad + TIE_RAD => Some(best),
        _ => Some(curve),
    })
}

/// The largest curve's size in degrees, or zero with no curve.
pub fn largest_deg(curves: &[Curve]) -> f64 {
    largest(curves).map_or(0.0, |curve| curve.size_rad.to_degrees())
}

fn curves(bodies: &[Body]) -> Vec<Curve> {
    let mut sds: Vec<f64> = bodies.iter().map(|body| body.tilt_sd_rad).collect();
    sds.sort_by(f64::total_cmp);
    let Some(&middle) = sds.get(sds.len() / 2) else {
        return Vec::new();
    };
    let threshold = DEVIATIONS * core::f64::consts::SQRT_2 * middle;
    let tilts: Vec<f64> = bodies.iter().map(|body| body.tilt_rad).collect();
    turning_points(&tilts, threshold)
        .windows(2)
        .map(|ends| curve(&bodies[ends[0]..=ends[1]]))
        .collect()
}

/// The tilt's extremes between changes of direction, in order. A change
/// counts once the tilt has come back by `threshold` from the extreme, so
/// neighbouring extremes always differ by at least `threshold`. Among tilts
/// equal to within [`TIE_RAD`] the first is the extreme.
fn turning_points(tilts: &[f64], threshold: f64) -> Vec<usize> {
    if !(threshold.is_finite() && threshold > 0.0) {
        return Vec::new();
    }
    let mut points = Vec::new();
    let (mut low, mut high) = (0, 0);
    // The way the tilt is going since the last extreme, once known, and the
    // furthest it has gone that way.
    let mut rising: Option<bool> = None;
    let mut extreme = 0;
    for (i, &tilt) in tilts.iter().enumerate().skip(1) {
        match rising {
            None => {
                if tilt < tilts[low] - TIE_RAD {
                    low = i;
                }
                if tilt > tilts[high] + TIE_RAD {
                    high = i;
                }
                if tilts[high] - tilts[low] >= threshold {
                    let up = high == i;
                    points.push(if up { low } else { high });
                    rising = Some(up);
                    extreme = i;
                }
            }
            Some(up) => {
                let further = if up {
                    tilt > tilts[extreme] + TIE_RAD
                } else {
                    tilt < tilts[extreme] - TIE_RAD
                };
                if further {
                    extreme = i;
                } else if (tilts[extreme] - tilt).abs() >= threshold {
                    points.push(extreme);
                    rising = Some(!up);
                    extreme = i;
                }
            }
        }
    }
    if rising.is_some() {
        points.push(extreme);
    }
    points
}

/// The curve between the first and last of `span`, its end vertebrae.
fn curve(span: &[Body]) -> Curve {
    let (upper, lower) = (span[0], span[span.len() - 1]);
    // Going down a curve convex to the patient's right the tilt rises: above
    // its apex the column heads back to the left going up (tilt below zero),
    // below it to the right.
    let convex_left = lower.tilt_rad < upper.tilt_rad;
    let toward = if convex_left { 1.0 } else { -1.0 };
    let rise = lower.centre.y - upper.centre.y;
    let deviation = |body: &Body| {
        let along = (body.centre.y - upper.centre.y) / rise;
        let line = upper.centre.x + along * (lower.centre.x - upper.centre.x);
        toward * (body.centre.x - line)
    };
    let apex = span
        .iter()
        .max_by(|a, b| deviation(a).total_cmp(&deviation(b)))
        .unwrap_or(&upper);
    let furthest = deviation(apex);
    let inside = span[1..span.len() - 1]
        .iter()
        .map(|body| {
            let share = if furthest > 0.0 {
                (deviation(body) / furthest).max(0.0)
            } else {
                0.0
            };
            (body.index, share)
        })
        .collect();
    Curve {
        upper: upper.index,
        lower: lower.index,
        apex: apex.index,
        size_rad: (lower.tilt_rad - upper.tilt_rad).abs(),
        convex_left,
        inside,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forward_model::cameras::parallel::ParallelCamera;
    use made_up_spines::answer_key::{answer_key, spine_for};
    use made_up_spines::curves::Curve as BuiltCurve;
    use made_up_spines::film_layout::FilmLayout;
    use made_up_spines::patient::make_up_patient;
    use made_up_spines::recipes::film_recipes;
    use shapes::levels::Level;
    use shapes::made_up::{MadeUpRecipe, RecipeName, Tier, TrueMarks};
    use shapes::marks::Mark;

    /// A body of half-width 20 mm whose mid-line passes through `centre`
    /// (sideways, up) at `tilt_deg`, 10 mm above and below it.
    fn drawn(index: usize, centre: [f64; 2], tilt_deg: f64, spread_mm: f64) -> Body {
        let centre = Vector2::new(centre[0], centre[1]);
        let tilt = tilt_deg.to_radians();
        let across = Vector2::new(libm::cos(tilt), libm::sin(tilt)) * 20.0;
        let up = Vector2::new(-libm::sin(tilt), libm::cos(tilt)) * 10.0;
        let corners = [
            centre + across + up,
            centre - across + up,
            centre - across - up,
            centre + across - up,
        ];
        Body::seen(index, corners, spread_mm)
    }

    /// A 0.5 mm corner spread on a 40 mm wide body: 0.0125 rad of tilt, so
    /// the threshold is 4 · √2 · 0.0125 rad = 4.05 degrees.
    const SPREAD_MM: f64 = 0.5;

    #[test]
    fn a_curve_worked_by_hand() {
        // Tilts 11, 14, 6, -2, -14, -12 degrees going down. The first and
        // last steps (3 and 2 degrees) are under the 4.05 degree threshold,
        // so the end vertebrae are 1 and 4, 28 degrees apart. The tilt falls
        // going down, so the curve bulges to the patient's left. The line
        // between the end centres (2, 100) and (4, 25) passes z = 75 at
        // y = 2.667 and z = 50 at y = 3.333: vertebra 2 sits 4.333 mm left
        // of it and vertebra 3, the apex, 5.667 mm, so vertebra 2's share is
        // 13/17.
        let bodies = [
            drawn(0, [0.0, 125.0], 11.0, SPREAD_MM),
            drawn(1, [2.0, 100.0], 14.0, SPREAD_MM),
            drawn(2, [7.0, 75.0], 6.0, SPREAD_MM),
            drawn(3, [9.0, 50.0], -2.0, SPREAD_MM),
            drawn(4, [4.0, 25.0], -14.0, SPREAD_MM),
            drawn(5, [0.0, 0.0], -12.0, SPREAD_MM),
        ];
        let found = curves(&bodies);
        assert_eq!(found.len(), 1, "{found:?}");
        let curve = &found[0];
        assert_eq!((curve.upper, curve.apex, curve.lower), (1, 3, 4));
        assert!(curve.convex_left);
        assert!((curve.size_rad.to_degrees() - 28.0).abs() < 1e-9);
        assert_eq!(curve.inside.len(), 2);
        assert_eq!(curve.inside[0].0, 2);
        assert!((curve.inside[0].1 - 13.0 / 17.0).abs() < 1e-12);
        assert_eq!(curve.inside[1], (3, 1.0));
        assert!((largest_deg(&found) - 28.0).abs() < 1e-9);
        // Mirrored, the same curve bulges to the right.
        let mirrored: Vec<Body> = bodies
            .iter()
            .map(|b| Body {
                centre: Vector2::new(-b.centre.x, b.centre.y),
                tilt_rad: -b.tilt_rad,
                ..*b
            })
            .collect();
        let found = curves(&mirrored);
        assert_eq!(found.len(), 1);
        assert!(!found[0].convex_left);
        assert_eq!(found[0].apex, 3);
    }

    #[test]
    fn a_straight_spine_with_wiggles_under_the_threshold_has_no_curve() {
        // Tilts wandering by up to 4 degrees, under the 4.05 degree
        // threshold from the spread.
        let tilts = [0.0, 2.0, -1.0, 1.5, -2.0, 2.0, 0.5, -1.5, 1.0];
        let bodies: Vec<Body> = tilts
            .iter()
            .enumerate()
            .map(|(i, &tilt)| drawn(i, [0.3 * tilt, -25.0 * i as f64], tilt, SPREAD_MM))
            .collect();
        assert!(curves(&bodies).is_empty());
        assert_eq!(largest_deg(&[]), 0.0);
        // A spread four times tighter finds them.
        let tight: Vec<Body> = bodies
            .iter()
            .map(|b| Body {
                tilt_sd_rad: b.tilt_sd_rad / 4.0,
                ..*b
            })
            .collect();
        assert!(!curves(&tight).is_empty());
    }

    #[test]
    fn runs_of_equal_tilt_end_a_curve_at_the_same_vertebra_from_both_sides() {
        // One curve whose end vertebrae sit in runs of equal tilt, +20
        // degrees above and -20 below, each tilt off by a few units of its
        // last digit as rounding leaves it, and its mirror image off by
        // other amounts. Both find the first vertebra of each run from the
        // top as the ends (0 and 12), the same apex and the same shares.
        let degrees = [
            20.0, 20.0, 20.0, 20.0, 18.8, 15.3, 10.0, 3.5, -3.5, -10.0, -15.3, -18.8, -20.0, -20.0,
            -20.0, -20.0, -20.0,
        ];
        let tilts: Vec<f64> = degrees.iter().map(|d: &f64| d.to_radians()).collect();
        let mut sideways = vec![0.0; tilts.len()];
        for k in (0..tilts.len() - 1).rev() {
            // rotX(θ) leans the column toward the patient's right going up.
            sideways[k] = sideways[k + 1] - 25.0 * libm::tan(0.5 * (tilts[k] + tilts[k + 1]));
        }
        let bodies = |sign: f64, errors: [f64; 17]| -> Vec<Body> {
            (0..tilts.len())
                .map(|k| Body {
                    index: k,
                    centre: Vector2::new(sign * sideways[k], 25.0 * (16 - k) as f64),
                    tilt_rad: sign * (tilts[k] + errors[k]),
                    tilt_sd_rad: 0.0125,
                })
                .collect()
        };
        let e = 1e-16;
        let drawn = curves(&bodies(
            1.0,
            [
                0.0,
                2.0 * e,
                0.0,
                4.0 * e,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                -2.0 * e,
                0.0,
                0.0,
                -4.0 * e,
            ],
        ));
        let mirrored = curves(&bodies(
            -1.0,
            [
                0.0,
                0.0,
                2.0 * e,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                3.0 * e,
                0.0,
                0.0,
            ],
        ));
        assert_eq!(drawn.len(), 1, "{drawn:?}");
        assert_eq!((drawn[0].upper, drawn[0].lower), (0, 12));
        assert!(drawn[0].convex_left);
        let unmirrored: Vec<Curve> = mirrored
            .into_iter()
            .map(|c| Curve {
                convex_left: !c.convex_left,
                ..c
            })
            .collect();
        assert_eq!(drawn, unmirrored);
        // Curves of equal size: the first from the top is the largest.
        let other = Curve {
            size_rad: drawn[0].size_rad + 1e-15,
            ..drawn[0].clone()
        };
        let both = [drawn[0].clone(), other];
        assert_eq!(largest(&both), Some(&both[0]));
    }

    /// The front film's bodies from T1 down, exactly as the made-up spine
    /// projects them, with a 1 mm corner spread, and the recipe's curves.
    fn made_up(name: RecipeName, seed: u32) -> (Vec<Body>, Vec<BuiltCurve>) {
        let recipe = MadeUpRecipe {
            name,
            seed,
            tier: Tier::T0,
        };
        let patient = make_up_patient(name, seed).expect("the recipe stacks");
        let film = &film_recipes(&recipe, u64::from(seed))[0];
        let layout = FilmLayout::new(film, spine_for(&patient, film.kind));
        let key = answer_key(&recipe, &patient, std::slice::from_ref(&layout));
        let answer = &key.films[0];
        let camera = ParallelCamera::for_film(&answer.geometry, layout.size, Vector2::zeros())
            .expect("a known camera");
        let mut bodies = Vec::new();
        for vertebra in answer.vertebrae.iter().filter(|v| v.level >= Level::T1) {
            let TrueMarks::Front {
                sup_l,
                sup_r,
                inf_l,
                inf_r,
                ..
            } = vertebra.marks
            else {
                unreachable!("a front film");
            };
            let corners =
                [sup_l, sup_r, inf_r, inf_l].map(|m| millimetres(&camera, Mark::auto(m.at)));
            bodies.push(Body::seen(bodies.len(), corners, 1.0));
        }
        (bodies, patient.recipe.curves)
    }

    #[test]
    fn made_up_curves_are_found_where_their_recipes_put_them() {
        // Exact marks with a 1 mm spread, seed 3. Measured: the right
        // thoracic recipe's 40 degree curve, T6 to T12 convex right, is
        // found there at 39.81 degrees. The double major's 55 and 45 degree
        // curves, T6 to T12 convex right and T12 to L3 convex left, are
        // found there at 53.69 and 43.76. The recipes set each vertebra's
        // own tilt; the corners show it turned and leaning, which reads a
        // little less. The bumps' tails also make curves the recipes did
        // not name, each bulging the other way, as compensating curves do:
        // T1 to T6 (17.75 and 17.37 degrees), T12 to L5 (18.60) and L3 to
        // L5 (16.19). Limits: each end vertebra where the recipe put it, the
        // convex side as built, the size within 2 degrees of the recipe's.
        for name in [RecipeName::RightThoracic40, RecipeName::DoubleMajor55To45] {
            let (bodies, built) = made_up(name, 3);
            let found = curves(&bodies);
            let summary: Vec<_> = found
                .iter()
                .map(|c| {
                    (
                        c.upper,
                        c.apex,
                        c.lower,
                        c.size_rad.to_degrees(),
                        c.convex_left,
                    )
                })
                .collect();
            eprintln!(
                "{name:?}: built {built:?}, found (upper, apex, lower, size, left) {summary:?}"
            );
            for built in built {
                // Recipe positions count vertebrae down from T1, as the
                // bodies do.
                let index = |t: f64| libm::round(t) as usize;
                let (upper, lower) = (index(built.upper_end), index(built.lower_end));
                let curve = found
                    .iter()
                    .find(|c| c.upper == upper && c.lower == lower)
                    .unwrap_or_else(|| panic!("{name:?}: no curve from {upper} to {lower}"));
                assert!(
                    (curve.size_rad.to_degrees() - built.cobb_deg).abs() < 2.0,
                    "{name:?}: {curve:?}"
                );
                assert_eq!(curve.convex_left, !built.convex_right, "{name:?}");
                assert!(upper < curve.apex && curve.apex < lower);
            }
        }
    }
}
