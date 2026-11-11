//! Soft agreement between the front and side chains on each level's height,
//! and bending films solved with the front chain as a prior.
//!
//! Each conditional chain step is exact. Alternating chains and a bounded
//! affine fit is an approximation to the joint optimum; the independent
//! energies provide its lower bound and the returned pair its upper bound.
//! Coupling weights, fit ranges and stopping tolerances are experimental.

use super::dynamic_program::{BodyState, ChainProblem, Solution, solve};
use shapes::auto_pass::{Acquisition, LengthUnit, LevelResidual, Stage, TwoFilmCheck};
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::FilmKind;
use shapes::levels::Level;
use shapes::watch_frames::WatchFrame;

/// Geometry at every lattice row, padding included.
#[derive(Clone, Debug)]
pub struct ViewGeometry {
    /// Height, increasing towards the head. Both views use millimetres
    /// when their scale is known. If either has assumed scale, supply
    /// original film pixel heights for both and set `scale_assumed`.
    pub z: Vec<f64>,
    pub observed: Vec<bool>,
    pub scale_assumed: bool,
}

/// A feasible paired result, its independent baselines and its certificate.
#[derive(Clone, Debug)]
pub struct JointSolution {
    pub front: Solution,
    pub side: Solution,
    pub independent_front: Solution,
    pub independent_side: Solution,
    /// Recompute each posterior with the opposite final chain fixed.
    pub front_conditioned: ChainProblem,
    pub side_conditioned: ChainProblem,
    pub scale: f64,
    pub offset: f64,
    pub upper: f64,
    pub lower: f64,
    pub check: TwoFilmCheck,
    pub frames: Vec<WatchFrame>,
}

/// Bending placements conditioned on the front film's body heights.
#[derive(Clone, Debug)]
pub struct BendingSolution {
    pub solution: Solution,
    pub conditioned: ChainProblem,
    pub scale: f64,
    /// The caller emits `BendingCouplingDropped` when this is true.
    pub dropped: bool,
}

/// Couple a bending film's heights to a front chain of the same template.
/// `row_step` and `front_heights` share length units. If their residual
/// median exceeds 4.5 units, return the independent chain instead.
pub fn solve_bending(
    problem: &ChainProblem,
    row_step: f64,
    front_heights: &[f64],
) -> Result<BendingSolution, EngineError> {
    if !row_step.is_finite()
        || row_step <= 0.0
        || front_heights.len() != problem.unary.len()
        || front_heights
            .iter()
            .any(|height| !height.is_finite() || *height <= 0.0)
    {
        return Err(bad_input(
            "bending heights must be positive and match the chain",
        ));
    }
    let independent = solve(problem)?;
    let mut found = independent.clone();
    let mut scale = 1.0;
    for _ in 0..4 {
        let previous = found.energy + bending_cost(&found.states, row_step, front_heights, scale);
        let conditioned = condition_bending(problem, row_step, front_heights, scale);
        let mut candidate = solve(&conditioned)?;
        candidate.energy = original_energy(problem, &candidate.states);
        let candidate_cost =
            candidate.energy + bending_cost(&candidate.states, row_step, front_heights, scale);
        if candidate_cost <= previous {
            found = candidate;
        }
        let mut fitted = scale;
        for _ in 0..20 {
            let mut numerator = 0.0;
            let mut denominator = 0.0;
            for (state, &reference) in found.states.iter().zip(front_heights) {
                let height = state.height as f64 * row_step;
                let deviation = ((height - fitted * reference) / 1.5).abs();
                let weight = if deviation <= 1.0 {
                    1.0
                } else {
                    1.0 / deviation
                };
                numerator += weight * reference * height;
                denominator += weight * reference * reference;
            }
            let next = (numerator / denominator).clamp(0.85, 1.15);
            if bending_cost(&found.states, row_step, front_heights, next)
                < bending_cost(&found.states, row_step, front_heights, scale)
            {
                scale = next;
            }
            if (next - fitted).abs() < 1.0e-9 {
                break;
            }
            fitted = next;
        }
        let current = found.energy + bending_cost(&found.states, row_step, front_heights, scale);
        if previous - current < 1.0e-3 {
            break;
        }
    }
    let mut residuals: Vec<_> = found
        .states
        .iter()
        .zip(front_heights)
        .map(|(state, reference)| (state.height as f64 * row_step - scale * reference).abs())
        .collect();
    let dropped = median(&mut residuals) > 4.5;
    Ok(BendingSolution {
        solution: if dropped { independent } else { found },
        conditioned: if dropped {
            problem.clone()
        } else {
            condition_bending(problem, row_step, front_heights, scale)
        },
        scale,
        dropped,
    })
}

fn bending_cost(states: &[BodyState], row_step: f64, front_heights: &[f64], scale: f64) -> f64 {
    states
        .iter()
        .zip(front_heights)
        .map(|(state, reference)| {
            0.5 * huber((state.height as f64 * row_step - scale * reference) / 1.5).min(8.0)
        })
        .sum()
}

fn condition_bending(
    problem: &ChainProblem,
    row_step: f64,
    front_heights: &[f64],
    scale: f64,
) -> ChainProblem {
    let mut conditioned = problem.clone();
    for (unary, reference) in conditioned.unary.iter_mut().zip(front_heights) {
        for top in 0..problem.rows {
            for hi in 0..problem.height_count {
                let height = (problem.min_height + hi) as f64 * row_step;
                unary[top * problem.height_count + hi] +=
                    0.5 * huber((height - scale * reference) / 1.5).min(8.0);
            }
        }
    }
    conditioned
}

#[derive(Clone, Copy, Debug)]
struct Alignment {
    scale: f64,
    offset: f64,
}

#[derive(Clone, Copy)]
struct Bounds {
    scale: [f64; 2],
    offset: [f64; 2],
}

#[derive(Clone, Copy)]
struct Centre {
    z: f64,
    observed: bool,
}

struct PairGeometry<'a> {
    front: &'a ViewGeometry,
    side: &'a ViewGeometry,
    tau: f64,
}

/// Solve two films using the same level list. If their surveys chose
/// different counts, the caller compares one paired run for each count
/// and retains the naming disagreement separately.
pub fn solve_pair(
    front: &ChainProblem,
    side: &ChainProblem,
    front_geometry: &ViewGeometry,
    side_geometry: &ViewGeometry,
    levels: &[Level],
    acquisition: Acquisition,
) -> Result<JointSolution, EngineError> {
    validate_geometry(front, front_geometry)?;
    validate_geometry(side, side_geometry)?;
    if front.unary.len() != side.unary.len() || front.unary.len() != levels.len() {
        return Err(bad_input("paired chains need the same levels"));
    }
    let independent_front = solve(front)?;
    let independent_side = solve(side)?;
    let lower = independent_front.energy + independent_side.energy;
    let geometry = PairGeometry {
        front: front_geometry,
        side: side_geometry,
        tau: if acquisition == Acquisition::Simultaneous {
            3.0
        } else {
            6.0
        },
    };
    let initial_centres = geometry.centres(&independent_front, &independent_side);
    let bounds = alignment_bounds(&initial_centres, &geometry, acquisition);
    let initial = refit(
        &initial_centres,
        Alignment {
            scale: 1.0_f64.clamp(bounds.scale[0], bounds.scale[1]),
            offset: (bounds.offset[0] + bounds.offset[1]) * 0.5,
        },
        bounds,
        geometry.tau,
    );
    let period = mean_period(&initial_centres);
    let mut best_front = independent_front.clone();
    let mut best_side = independent_side.clone();
    let mut best_alignment = initial;
    let mut best_upper = lower + coupling(&initial_centres, initial, geometry.tau);
    let mut frames = Vec::new();

    // The three offset starts run in both block orders. This experimental
    // extension prevents a damaged front film from being trusted solely
    // because the side block happened to run first. Every added candidate
    // is still feasible and uses the same certified lower bound.
    let starts = [false, true]
        .into_iter()
        .flat_map(|front_first| [-period, 0.0, period].map(|shift| (front_first, shift)));
    for (start, (front_first, shift)) in starts.enumerate() {
        let mut found_front = independent_front.clone();
        let mut found_side = independent_side.clone();
        let mut alignment = Alignment {
            scale: initial.scale,
            offset: (initial.offset + shift).clamp(bounds.offset[0], bounds.offset[1]),
        };
        let mut upper = lower + coupling(&initial_centres, alignment, geometry.tau);
        for round in 0..4 {
            let before = upper;
            for moving_front in [front_first, !front_first] {
                let (problem, fixed) = if moving_front {
                    (front, &found_side)
                } else {
                    (side, &found_front)
                };
                let conditioned = condition(problem, fixed, &geometry, alignment, moving_front);
                let mut candidate = solve(&conditioned)?;
                candidate.energy = original_energy(problem, &candidate.states);
                let centres = if moving_front {
                    geometry.centres(&candidate, &found_side)
                } else {
                    geometry.centres(&found_front, &candidate)
                };
                let candidate_upper =
                    fixed.energy + candidate.energy + coupling(&centres, alignment, geometry.tau);
                if candidate_upper <= upper {
                    if moving_front {
                        found_front = candidate;
                    } else {
                        found_side = candidate;
                    }
                    upper = candidate_upper;
                }
                let centres = geometry.centres(&found_front, &found_side);
                let fitted = refit(&centres, alignment, bounds, geometry.tau);
                let fitted_upper = found_front.energy
                    + found_side.energy
                    + coupling(&centres, fitted, geometry.tau);
                if fitted_upper <= upper {
                    alignment = fitted;
                    upper = fitted_upper;
                }
                frames.push(progress(
                    if moving_front {
                        FilmKind::Front
                    } else {
                        FilmKind::Side
                    },
                    start,
                    round,
                    upper,
                    lower,
                ));
            }
            if upper < best_upper {
                best_front = found_front.clone();
                best_side = found_side.clone();
                best_alignment = alignment;
                best_upper = upper;
            }
            if before - upper < 1.0e-3 {
                break;
            }
        }
    }

    let centres = geometry.centres(&best_front, &best_side);
    let residuals: Vec<_> = levels
        .iter()
        .zip(&centres)
        .map(|(&level, &(f, s))| LevelResidual {
            level,
            residual: residual(f, s, best_alignment),
        })
        .collect();
    let mut observed_residuals: Vec<_> = centres
        .iter()
        .filter(|(f, s)| f.observed || s.observed)
        .map(|&(f, s)| residual(f, s, best_alignment).abs())
        .collect();
    // The theoretical gap is nonnegative; rounding after reassociation
    // of the same sums can make zero a few ULPs negative.
    let gap = (best_upper - lower).max(0.0);
    let check = TwoFilmCheck {
        certified_gap: gap,
        exact: gap <= 0.25,
        disagree: median(&mut observed_residuals) > 3.0 * geometry.tau
            || on_boundary(best_alignment, bounds)
            || gap > 2.0,
        unit: if front_geometry.scale_assumed || side_geometry.scale_assumed {
            LengthUnit::Px
        } else {
            LengthUnit::Mm
        },
        residuals,
    };
    Ok(JointSolution {
        front_conditioned: condition(front, &best_side, &geometry, best_alignment, true),
        side_conditioned: condition(side, &best_front, &geometry, best_alignment, false),
        front: best_front,
        side: best_side,
        independent_front,
        independent_side,
        scale: best_alignment.scale,
        offset: best_alignment.offset,
        upper: best_upper,
        lower,
        check,
        frames,
    })
}

fn validate_geometry(problem: &ChainProblem, geometry: &ViewGeometry) -> Result<(), EngineError> {
    if geometry.z.len() != problem.rows
        || geometry.observed.len() != problem.rows
        || geometry.z.iter().any(|z| !z.is_finite())
        || geometry.z.len() < 2
    {
        return Err(bad_input(
            "paired geometry must be finite with one entry per row",
        ));
    }
    Ok(())
}

fn bad_input(message: &str) -> EngineError {
    EngineError::new(ErrorCode::EngineOutOfSync, message)
}

impl ViewGeometry {
    fn centre(&self, state: BodyState) -> Centre {
        let row = state.top as f64 + state.height as f64 * 0.5;
        let index = (row as usize).min(self.z.len() - 1);
        let next = (index + 1).min(self.z.len() - 1);
        let fraction = (row - index as f64).clamp(0.0, 1.0);
        Centre {
            z: self.z[index] + fraction * (self.z[next] - self.z[index]),
            observed: self.observed[index] || (fraction > 0.0 && self.observed[next]),
        }
    }
}

impl PairGeometry<'_> {
    fn centres(&self, front: &Solution, side: &Solution) -> Vec<(Centre, Centre)> {
        front
            .states
            .iter()
            .zip(&side.states)
            .map(|(&f, &s)| (self.front.centre(f), self.side.centre(s)))
            .collect()
    }
}

fn alignment_bounds(
    centres: &[(Centre, Centre)],
    geometry: &PairGeometry<'_>,
    acquisition: Acquisition,
) -> Bounds {
    let (first_f, first_s) = centres[0];
    let (last_f, last_s) = centres[centres.len() - 1];
    let midpoint_f = (first_f.z + last_f.z) * 0.5;
    let midpoint_s = (first_s.z + last_s.z) * 0.5;
    let assumed = geometry.front.scale_assumed || geometry.side.scale_assumed;
    let span_f = (last_f.z - first_f.z).abs();
    let span_s = (last_s.z - first_s.z).abs();
    let scale = if assumed && span_f > 1.0e-9 && span_s > 1.0e-9 {
        let ratio = span_s / span_f;
        [0.7 * ratio, 1.4 * ratio]
    } else if acquisition == Acquisition::Simultaneous {
        [0.98, 1.02]
    } else {
        [0.85, 1.15]
    };
    let offset = if acquisition == Acquisition::Simultaneous && !assumed {
        [-10.0, 10.0]
    } else {
        let scale_mid = if assumed && span_f > 1.0e-9 {
            span_s / span_f
        } else {
            1.0
        };
        let midpoint = midpoint_s - scale_mid * midpoint_f;
        [midpoint - 100.0, midpoint + 100.0]
    };
    Bounds { scale, offset }
}

fn mean_period(centres: &[(Centre, Centre)]) -> f64 {
    if centres.len() < 2 {
        return 0.0;
    }
    centres
        .windows(2)
        .map(|pair| (pair[1].1.z - pair[0].1.z).abs())
        .sum::<f64>()
        / (centres.len() - 1) as f64
}

fn huber(value: f64) -> f64 {
    let value = value.abs();
    if value <= 1.0 {
        0.5 * value * value
    } else {
        value - 0.5
    }
}

fn residual(front: Centre, side: Centre, alignment: Alignment) -> f64 {
    side.z - alignment.scale * front.z - alignment.offset
}

fn rung(front: Centre, side: Centre, alignment: Alignment, tau: f64) -> f64 {
    if !front.observed && !side.observed {
        0.0
    } else {
        huber(residual(front, side, alignment) / tau).min(8.0)
    }
}

fn coupling(centres: &[(Centre, Centre)], alignment: Alignment, tau: f64) -> f64 {
    centres
        .iter()
        .map(|&(f, s)| rung(f, s, alignment, tau))
        .sum()
}

fn condition(
    problem: &ChainProblem,
    fixed: &Solution,
    geometry: &PairGeometry<'_>,
    alignment: Alignment,
    moving_front: bool,
) -> ChainProblem {
    let mut conditioned = problem.clone();
    for (body, unary) in conditioned.unary.iter_mut().enumerate() {
        for top in 0..problem.rows {
            for height_index in 0..problem.height_count {
                let state = BodyState {
                    top,
                    height: problem.min_height + height_index,
                };
                let (f, s) = if moving_front {
                    (
                        geometry.front.centre(state),
                        geometry.side.centre(fixed.states[body]),
                    )
                } else {
                    (
                        geometry.front.centre(fixed.states[body]),
                        geometry.side.centre(state),
                    )
                };
                unary[top * problem.height_count + height_index] +=
                    rung(f, s, alignment, geometry.tau);
            }
        }
    }
    conditioned
}

fn original_energy(problem: &ChainProblem, states: &[BodyState]) -> f64 {
    let index =
        |state: BodyState| state.top * problem.height_count + state.height - problem.min_height;
    let mut energy = problem.start[index(states[0])];
    for (body, &state) in states.iter().enumerate() {
        energy += problem.unary[body][index(state)];
        if let Some(&next) = states.get(body + 1) {
            let hi = state.height - problem.min_height;
            let hj = next.height - problem.min_height;
            let bot = state.top + state.height;
            let di = next.top - bot - problem.min_gap;
            energy += problem.ratio[body][hi * problem.height_count + hj];
            energy += problem.gap[bot * problem.gap_count + di];
            energy += problem.disc[body][di * problem.height_count + hj];
        }
    }
    energy + problem.end[index(states[states.len() - 1])]
}

/// Bounded IRLS fits the uncapped Huber objective. Retaining a fit only
/// when the capped objective improves keeps each block monotone too.
fn refit(centres: &[(Centre, Centre)], initial: Alignment, bounds: Bounds, tau: f64) -> Alignment {
    let mut fit = initial;
    let mut best = fit;
    let mut best_cost = coupling(centres, fit, tau);
    for _ in 0..20 {
        let mut total = 0.0;
        let mut mean_x = 0.0;
        let mut mean_y = 0.0;
        let mut weighted = Vec::with_capacity(centres.len());
        for &(front, side) in centres {
            if !front.observed && !side.observed {
                continue;
            }
            let deviation = (residual(front, side, fit) / tau).abs();
            let weight = if deviation <= 1.0 {
                1.0
            } else {
                1.0 / deviation
            };
            total += weight;
            mean_x += weight * front.z;
            mean_y += weight * side.z;
            weighted.push((front.z, side.z, weight));
        }
        if total == 0.0 {
            break;
        }
        mean_x /= total;
        mean_y /= total;
        let mut xx = 0.0;
        let mut xy = 0.0;
        for &(x, y, weight) in &weighted {
            xx += weight * (x - mean_x) * (x - mean_x);
            xy += weight * (x - mean_x) * (y - mean_y);
        }
        let scale =
            if xx > 1.0e-12 { xy / xx } else { fit.scale }.clamp(bounds.scale[0], bounds.scale[1]);
        let candidate = Alignment {
            scale,
            offset: (mean_y - scale * mean_x).clamp(bounds.offset[0], bounds.offset[1]),
        };
        // With an active offset bound, fixing the scale first is not
        // the constrained least-squares optimum. Examine both edges.
        let mut candidates = vec![candidate];
        for offset in bounds.offset {
            let mut numerator = 0.0;
            let mut denominator = 0.0;
            for &(x, y, weight) in &weighted {
                numerator += weight * x * (y - offset);
                denominator += weight * x * x;
            }
            candidates.push(Alignment {
                scale: if denominator > 1.0e-12 {
                    numerator / denominator
                } else {
                    fit.scale
                }
                .clamp(bounds.scale[0], bounds.scale[1]),
                offset,
            });
        }
        let weighted_cost = |candidate: Alignment| {
            weighted
                .iter()
                .map(|&(x, y, weight)| {
                    let error = y - candidate.scale * x - candidate.offset;
                    weight * error * error
                })
                .sum::<f64>()
        };
        let next = candidates
            .into_iter()
            .min_by(|a, b| weighted_cost(*a).total_cmp(&weighted_cost(*b)))
            .expect("the interior fit is present");
        let cost = coupling(centres, next, tau);
        if cost < best_cost {
            best = next;
            best_cost = cost;
        }
        let change = (next.scale - fit.scale).abs() + (next.offset - fit.offset).abs();
        fit = next;
        if change < 1.0e-9 {
            break;
        }
    }
    best
}

fn on_boundary(alignment: Alignment, bounds: Bounds) -> bool {
    bounds
        .scale
        .iter()
        .any(|bound| (alignment.scale - bound).abs() <= 1.0e-9)
        || bounds
            .offset
            .iter()
            .any(|bound| (alignment.offset - bound).abs() <= 1.0e-9)
}

fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[middle - 1] + values[middle]) * 0.5
    } else {
        values[middle]
    }
}

fn progress(film: FilmKind, start: usize, round: u32, upper: f64, lower: f64) -> WatchFrame {
    WatchFrame::JointProgress {
        film,
        stage: Stage::Chain,
        start: start as u32,
        round,
        upper,
        lower,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The four known placements are independent of anatomical priors.
    // A single false side edge is eight rows too low; the other three
    // bodies fix the registration and make that shift inconsistent.
    fn problem(false_edge: bool) -> ChainProblem {
        let rows = 80;
        let gap_count = 24;
        let mut unary = vec![vec![f64::INFINITY; rows]; 4];
        for (body, top) in [6, 24, 42, 60].into_iter().enumerate() {
            unary[body][top] = if false_edge && body == 1 { 0.2 } else { 0.0 };
        }
        if false_edge {
            unary[1][32] = 0.0;
        }
        ChainProblem {
            rows,
            min_height: 6,
            height_count: 1,
            min_gap: 1,
            gap_count,
            unary,
            ratio: vec![vec![0.0]; 3],
            disc: vec![vec![0.0; gap_count]; 3],
            gap: vec![0.0; rows * gap_count],
            start: vec![0.0; rows],
            end: vec![0.0; rows],
        }
    }

    fn geometry(scale: f64, offset: f64, observed: bool) -> ViewGeometry {
        ViewGeometry {
            z: (0..80).map(|row| offset - scale * row as f64).collect(),
            observed: vec![observed; 80],
            scale_assumed: false,
        }
    }

    fn levels() -> [Level; 4] {
        [Level::C7, Level::T1, Level::T2, Level::T3]
    }

    #[test]
    fn the_other_view_removes_a_false_boundary() {
        let result = solve_pair(
            &problem(false),
            &problem(true),
            &geometry(1.0, 0.0, true),
            &geometry(1.0, 0.0, true),
            &levels(),
            Acquisition::Simultaneous,
        )
        .expect("both chains fit");
        assert_eq!(result.independent_side.states[1].top, 32);
        assert_eq!(result.side.states[1].top, 24);
        assert_eq!(result.front.states, result.side.states);
        // The true side placement costs 0.2 and every rung is exactly
        // zero; the independent false placement costs zero.
        assert!((result.upper - 0.2).abs() < 1.0e-12);
        assert_eq!(result.lower, 0.0);
        assert!((result.check.certified_gap - 0.2).abs() < 1.0e-12);
        assert!(result.check.exact);
        assert!(!result.check.disagree);
        let mut previous = None;
        for frame in &result.frames {
            let WatchFrame::JointProgress {
                start,
                upper,
                lower,
                ..
            } = frame
            else {
                panic!("only joint checkpoints belong here");
            };
            if let Some((previous_start, previous_upper)) = previous
                && *start == previous_start
            {
                assert!(*upper <= previous_upper);
            }
            assert!(*upper >= *lower);
            previous = Some((*start, *upper));
        }
    }

    #[test]
    fn affine_registration_recovers_known_projection_geometry() {
        for (scale, offset, acquisition) in [
            (1.0, 4.0, Acquisition::Simultaneous),
            (1.07, -23.0, Acquisition::Separate),
        ] {
            let result = solve_pair(
                &problem(false),
                &problem(false),
                &geometry(1.0, 0.0, true),
                &geometry(scale, offset, true),
                &levels(),
                acquisition,
            )
            .expect("both chains fit");
            // These exact affine observations have zero residual. The
            // tolerance allows only rounding in the f64 regression.
            assert!((result.scale - scale).abs() < 1.0e-12);
            assert!((result.offset - offset).abs() < 1.0e-11);
            assert!(result.check.certified_gap < 1.0e-20);
            assert!(
                result
                    .check
                    .residuals
                    .iter()
                    .all(|r| r.residual.abs() < 1.0e-11)
            );
        }
    }

    #[test]
    fn padding_pairs_add_no_evidence() {
        let result = solve_pair(
            &problem(false),
            &problem(false),
            &geometry(1.0, 0.0, false),
            &geometry(1.2, -300.0, false),
            &levels(),
            Acquisition::Simultaneous,
        )
        .expect("unobserved bodies still fit");
        assert_eq!(result.upper, 0.0);
        assert_eq!(result.lower, 0.0);
        assert_eq!(result.front_conditioned.unary, problem(false).unary);
        assert_eq!(result.side_conditioned.unary, problem(false).unary);
    }

    #[test]
    fn malformed_geometry_is_refused_before_searching() {
        let mut invalid = geometry(1.0, 0.0, true);
        invalid.z[3] = f64::NAN;
        assert!(
            solve_pair(
                &problem(false),
                &problem(false),
                &invalid,
                &geometry(1.0, 0.0, true),
                &levels(),
                Acquisition::Unknown,
            )
            .is_err()
        );
    }

    #[test]
    fn capped_outliers_cannot_make_refitting_increase_the_objective() {
        let centres: Vec<_> = [-60.0, -40.0, -20.0, 0.0]
            .into_iter()
            .enumerate()
            .map(|(index, z)| {
                (
                    Centre { z, observed: true },
                    Centre {
                        z: if index == 2 { 200.0 } else { z + 4.0 },
                        observed: true,
                    },
                )
            })
            .collect();
        let initial = Alignment {
            scale: 1.0,
            offset: 4.0,
        };
        let fitted = refit(
            &centres,
            initial,
            Bounds {
                scale: [0.98, 1.02],
                offset: [-10.0, 10.0],
            },
            3.0,
        );
        assert!(coupling(&centres, fitted, 3.0) <= coupling(&centres, initial, 3.0));
        // Three perfect observations and one capped outlier: 0+0+0+8.
        assert_eq!(coupling(&centres, fitted, 3.0), 8.0);
    }

    #[test]
    fn bending_height_scale_is_fitted_and_incompatible_coupling_is_dropped() {
        let fitted =
            solve_bending(&problem(false), 1.0, &[6.0 / 1.1; 4]).expect("positive front heights");
        // Every body is six rows high, so h_bending / h_front = 1.1.
        assert!((fitted.scale - 1.1).abs() < 1.0e-12);
        assert!(!fitted.dropped);
        let dropped = solve_bending(&problem(false), 1.0, &[30.0; 4])
            .expect("positive but incompatible front heights");
        assert!(dropped.dropped);
        assert_eq!(dropped.conditioned.unary, problem(false).unary);
        assert_eq!(
            dropped.solution.states,
            solve(&problem(false)).unwrap().states
        );
    }
}
