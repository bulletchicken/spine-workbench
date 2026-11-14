//! One front film, or a front and side pair, fitted as one spine, with the
//! typical spine for what the films cannot show.

use crate::problem::{Film, Problem};
use crate::{
    FitError, curve_twist, estimated_names, front_curves, mark_spread, scale, twist_readings,
    typical_spine,
};
use shapes::levels::Level;
use shapes::spine_fit::{FitFromFilms, FitLengths, SpineFit, TypicalSource};
use std::collections::BTreeSet;

const PAIR_RMS_LIMIT: f64 = 1.75;
const FRONT_RMS_ALLOWANCE: f64 = 0.5;

/// Use Side only when a converged pair explains the placed marks. Both
/// the all-mark and Side-only normalised Euclidean RMS must be <= 1.75.
/// Count every fitted corner and visible pedicle once. Inferred marks take
/// three own spreads, pedicles two. For an uncalibrated Side with millimetre
/// spread, acceptance distances use its fitted spacing; solver weights are
/// unchanged. Height-relative spreads scale with the image and cancel out.
///
/// The yardstick is fitted honest noise: six full/partial 1 mm cases score
/// 0.733 to 0.860 overall, rather than the RMS of their input noise. Exact
/// marks score 0.010 and the partial lumbar pair 0.064. The limit leaves
/// about twice that observed honest error; it is not a coverage guarantee.
/// In one fixed noise series, full pairs first fail at 2.5 times the stated
/// noise (all 1.756, Side 1.757); lumbar pairs at 2 times (Side 2.083).
/// The bad ordinary intake scores 2.781 overall and 2.633 on Side.
///
/// Front's comparison RMS may rise by at most 0.5 over its Front-only
/// reference. In both fits each Front mark uses sqrt(own_sd² + shared_sd²),
/// so a shared body shift is admitted on the same basis. The six 1 mm cases
/// rise by at most 0.172; ten trained-noise seeds reach 0.477. The bad
/// intake rises from 1.335 to 2.922. A reference uses built-in typical values for unseen directions. If it
/// fails or does not converge, skip only this comparison: the other checks
/// still apply. A refused Side leaves exactly the original Front-only
/// result, including its named error when no typical values were requested.
pub fn fit_from_films(input: &FitFromFilms) -> Result<SpineFit, FitError> {
    let (problem, lengths, curve_deg) = prepare(input)?;
    if problem.side.is_none() {
        return finish(input, &problem, lengths, curve_deg).map(|answer| answer.fit);
    }
    let pair = finish(input, &problem, lengths, curve_deg);
    let front = front_reference(input);
    let reference = front.as_ref().map(|answer| answer.rms.front());
    if let Ok(pair) = pair
        && pair.fit.converged
        && pair.heights_agree
        && residuals_agree(pair.rms.all(), pair.rms.front(), pair.rms.side(), reference)
    {
        return Ok(pair.fit);
    }
    if input.typical == TypicalSource::BuiltIn
        && let Some(front) = front
    {
        return Ok(front.fit);
    }
    let (problem, lengths, curve_deg) = prepare_selected(input, None)?;
    finish(input, &problem, lengths, curve_deg).map(|answer| answer.fit)
}

fn front_reference(input: &FitFromFilms) -> Option<Finished> {
    // The reference needs typical values for its unseen directions. A
    // failure here cannot replace a valid pair or the caller's fallback.
    let mut front_input = input.clone();
    front_input.side = None;
    front_input.typical = TypicalSource::BuiltIn;
    prepare_selected(&front_input, None)
        .and_then(|(problem, lengths, curve)| finish(&front_input, &problem, lengths, curve))
        .ok()
        .filter(|answer| answer.fit.converged && answer.rms.front().is_finite())
}

/// Residual rules only; convergence and shared heights are checked separately.
fn residuals_agree(all: f64, front: f64, side: f64, reference: Option<f64>) -> bool {
    [all, front, side]
        .iter()
        .all(|r| r.is_finite() && *r >= 0.0)
        && all <= PAIR_RMS_LIMIT
        && side <= PAIR_RMS_LIMIT
        && reference.is_none_or(|r| r.is_finite() && r >= 0.0 && front <= r + FRONT_RMS_ALLOWANCE)
}

#[derive(Default)]
struct MarkRms {
    squared: [f64; 2],
    count: [usize; 2],
    front_comparison_squared: f64,
}

impl MarkRms {
    fn all(&self) -> f64 {
        libm::sqrt(self.squared.iter().sum::<f64>() / self.count.iter().sum::<usize>() as f64)
    }

    fn side(&self) -> f64 {
        libm::sqrt(self.squared[1] / self.count[1] as f64)
    }

    fn front(&self) -> f64 {
        libm::sqrt(self.front_comparison_squared / self.count[0] as f64)
    }
}

struct Finished {
    fit: SpineFit,
    rms: MarkRms,
    heights_agree: bool,
}

fn finish(
    input: &FitFromFilms,
    problem: &Problem,
    lengths: FitLengths,
    curve_deg: f64,
) -> Result<Finished, FitError> {
    use crate::parameters::COUNT;
    let solution = crate::levenberg_marquardt::solve(problem, crate::start_values::start(problem))?;
    let numbers = solution.numbers.as_slice();
    let global = problem.observations.len() * COUNT;
    let side = problem.side_camera(numbers[global], numbers[global + 1]);
    let side_patient_mm = input.side.as_ref().is_some_and(|film| {
        film.geometry.mm_per_px.is_none()
            && matches!(
                film.mark_spread,
                shapes::spine_fit::MarkSpread::Millimetres { .. }
            )
    });
    // Only acceptance statistics change units; the objective and covariance
    // still use the existing provisional Side spacing.
    let side_scale = if side_patient_mm {
        libm::exp(numbers[global])
    } else {
        1.0
    };
    let mut rms = MarkRms::default();
    for (i, observation) in problem.observations.iter().enumerate() {
        let marks = crate::residuals::marks(
            problem,
            observation,
            &numbers[i * COUNT..(i + 1) * COUNT],
            side.as_ref(),
        )
        .ok_or(FitError::NumericalFailure)?;
        for view in 0..2 {
            let units = if view == 1 { side_scale } else { 1.0 };
            rms.squared[view] += marks.squared_spreads[view] * units * units;
            rms.count[view] += marks.counts[view];
        }
        rms.front_comparison_squared += marks.front_comparison_squared;
    }
    let mut result = crate::report(problem, &solution)?;
    result.lengths = lengths;
    result.typical_curve_deg = problem.typical.is_some().then_some(curve_deg);
    result.front_film.facing_assumed = input.front.facing_assumed;
    if let Some(placement) = &mut result.side_film {
        placement.facing_assumed = input.side.as_ref().expect("a used Side").facing_assumed;
    }
    Ok(Finished {
        fit: result,
        rms,
        heights_agree: pair_agrees(problem, numbers, side_patient_mm),
    })
}

/// Both views observe the same centre height. Compare those heights in
/// the fitted registration, allowing three summed corner spreads RMS.
/// Keep a whole corner's spread for a centre: outlining errors need not
/// average away. Shared placement error is included too. Width, depth,
/// pedicles and typical values cannot establish agreement between views.
/// Any inferred corner triples that view's budget, as in the mark residuals.
/// This sets aside films whose centre heights disagree after registration;
/// it does not establish matching names or identity. With no spacing on
/// either film, free Side scale and offset can align a chain named one level off or a
/// short lumbar film from another spine by a straight-line map of heights.
fn pair_agrees(problem: &Problem, numbers: &[f64], side_patient_mm: bool) -> bool {
    use crate::parameters::COUNT;
    use crate::problem::millimetres;
    let global = problem.observations.len() * COUNT;
    let Some(side) = problem.side_camera(numbers[global], numbers[global + 1]) else {
        return true;
    };
    let ratio = libm::exp(numbers[global]);
    let mut squared = 0.0;
    let mut shared = 0;
    for observation in &problem.observations {
        let (Some(front_marks), Some(side_marks)) = (observation.front, observation.side) else {
            continue;
        };
        let front_corners = front_marks.corners();
        let side_corners = side_marks.corners();
        let front_factor = if front_corners.iter().any(|m| m.inferred) {
            3.0
        } else {
            1.0
        };
        let side_factor = if side_corners.iter().any(|m| m.inferred) {
            3.0
        } else {
            1.0
        };
        let front_height = front_corners
            .map(|m| millimetres(&problem.front, m).y)
            .iter()
            .sum::<f64>()
            * 0.25;
        let side_height = side_corners
            .map(|m| millimetres(&side, m).y)
            .iter()
            .sum::<f64>()
            * 0.25;
        let [front_sd, side_sd] = observation.spread_mm;
        let [front_shared, side_shared] = observation.shared_mm;
        // Sum the two films' error budgets; either outline may be off.
        let side_budget_units = if side_patient_mm { 1.0 } else { ratio };
        let spread = front_factor * (front_sd + front_shared)
            + side_budget_units * side_factor * (side_sd + side_shared);
        let difference = front_height - side_height;
        squared += difference * difference / (spread * spread);
        shared += 1;
    }
    shared >= 2 && squared / shared as f64 <= 9.0
}

/// Validate Front independently, so a bad Side never hides a Front error.
/// Two Side levels are needed to determine its relative vertical scale.
pub fn prepare(input: &FitFromFilms) -> Result<(Problem, FitLengths, f64), FitError> {
    let typical = match input.typical {
        TypicalSource::BuiltIn => Some(typical_spine::built_in()),
        TypicalSource::None => None,
    };
    let (geometry, _) = scale::front_geometry(&input.front, typical)?;
    let mut front = Problem::of_films(
        Film {
            marks: &input.front.marks,
            geometry: &geometry,
            size: input.front.size,
        },
        None,
    )?;
    mark_spread::attach(&mut front, input.front.mark_spread, None)?;
    let side = input
        .side
        .as_ref()
        .filter(|film| film.marks.vertebrae.len() >= 2);
    match prepare_selected(input, side) {
        Err(
            FitError::MissingFacing(shapes::film::FilmKind::Side)
            | FitError::InvalidFacing(shapes::film::FilmKind::Side)
            | FitError::InvalidSize(shapes::film::FilmKind::Side)
            | FitError::InvalidScale(shapes::film::FilmKind::Side)
            | FitError::InvalidMarks(shapes::film::FilmKind::Side)
            | FitError::InvalidMarkSpread(shapes::film::FilmKind::Side),
        ) => prepare_selected(input, None),
        answer => answer,
    }
}

fn prepare_selected(
    input: &FitFromFilms,
    side: Option<&shapes::spine_fit::FilmToFit>,
) -> Result<(Problem, FitLengths, f64), FitError> {
    let levels: BTreeSet<Level> = input
        .front
        .marks
        .vertebrae
        .iter()
        .chain(side.map_or(&[][..], |film| &film.marks.vertebrae))
        .map(|v| v.level)
        .collect();
    match levels.len() {
        0 => return Err(FitError::NoMarks),
        1 => return Err(FitError::TooFewVertebrae),
        _ => {}
    }
    let typical = match input.typical {
        TypicalSource::BuiltIn => Some(typical_spine::built_in()),
        TypicalSource::None => None,
    };
    let (front_geometry, front_lengths) = scale::front_geometry(&input.front, typical)?;
    let side_geometry = side.map(|film| scale::side_geometry(film, &front_geometry));
    let mut problem = Problem::of_films(
        Film {
            marks: &input.front.marks,
            geometry: &front_geometry,
            size: input.front.size,
        },
        side.zip(side_geometry.as_ref())
            .map(|(film, geometry)| Film {
                marks: &film.marks,
                geometry,
                size: film.size,
            }),
    )?;
    if problem.side.is_none() && typical.is_none() {
        return Err(FitError::Underconstrained);
    }
    problem.calibrated_pair = input.front.geometry.mm_per_px.is_some()
        && side.is_some_and(|film| film.geometry.mm_per_px.is_some());
    problem.side_is_length_anchor = input.front.geometry.mm_per_px.is_none()
        && side.is_some_and(|film| film.geometry.mm_per_px.is_some());
    let lengths = if problem.side_is_length_anchor {
        FitLengths::Measured
    } else {
        front_lengths
    };
    mark_spread::attach(
        &mut problem,
        input.front.mark_spread,
        side.map(|film| film.mark_spread),
    )?;
    twist_readings::attach(&mut problem, &input.twist_readings)?;
    let curves = front_curves::find(&problem);
    let curve_deg = front_curves::largest_deg(&curves);
    let estimated = estimated_names::names_are_estimates(&input.front.marks);
    problem.typical = typical.map(|spine| {
        let levels = problem.observations.iter().map(|o| o.level);
        let mut values = spine.for_levels(levels, curve_deg, estimated);
        curve_twist::apply(&mut values, front_curves::largest(&curves), spine);
        values
    });
    problem.rescale_covariance = false;
    Ok((problem, lengths, curve_deg))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::answer_keys::input;
    use crate::parameters::COUNT;
    use crate::{fit, levenberg_marquardt, report, start_values, uncertainty};
    use made_up_spines::seeded::Stream;
    use shapes::made_up::RecipeName;
    use shapes::marks::{MarkSource, PedicleMark};
    use shapes::pixels::Px;
    use shapes::spine_fit::{FilmToFit, MarkSpread};
    use shapes::vertebra::VertebraMarks;

    fn pair_request(original: shapes::spine_fit::FitInput) -> FitFromFilms {
        FitFromFilms {
            front: FilmToFit {
                marks: original.front,
                geometry: original.front_geometry,
                size: original.front_size,
                mark_spread: MarkSpread::Millimetres {
                    sd_mm: 0.1,
                    shared_sd_mm: 0.0,
                },
                facing_assumed: false,
            },
            side: Some(FilmToFit {
                marks: original.side,
                geometry: original.side_geometry,
                size: original.side_size,
                mark_spread: MarkSpread::Millimetres {
                    sd_mm: 1.0,
                    shared_sd_mm: 0.0,
                },
                facing_assumed: false,
            }),
            typical: TypicalSource::BuiltIn,
            twist_readings: Vec::new(),
        }
    }

    fn assessed(request: &FitFromFilms) -> Finished {
        let (problem, lengths, curve) = prepare(request).unwrap();
        finish(request, &problem, lengths, curve).unwrap()
    }

    fn skewed_lumbar_side() -> FitFromFilms {
        let mut request = pair_request(input(RecipeName::Straight, 3, 0.0).0);
        let side = request.side.as_mut().unwrap();
        side.marks.vertebrae.retain(|v| v.level >= Level::L1);
        let spacing = side.geometry.mm_per_px.unwrap();
        for v in &mut side.marks.vertebrae {
            let VertebraMarks::Side { points } = &mut v.marks else {
                unreachable!("Side")
            };
            for mark in [&mut points.sup_a, &mut points.sup_p] {
                mark.at = Px::new(mark.at.x(), mark.at.y() - 4.0 / spacing);
            }
            for mark in [&mut points.inf_a, &mut points.inf_p] {
                mark.at = Px::new(mark.at.x(), mark.at.y() + 4.0 / spacing);
            }
        }
        request
    }

    #[test]
    fn a_short_skewed_side_cannot_hide_under_a_good_full_front() {
        let request = skewed_lumbar_side();
        let pair = assessed(&request);
        let reference = front_reference(&request).unwrap();
        eprintln!(
            "skewed short Side: all={} front={} side={} reference={} heights={}",
            pair.rms.all(),
            pair.rms.front(),
            pair.rms.side(),
            reference.rms.front(),
            pair.heights_agree
        );
        assert!(pair.fit.converged && pair.heights_agree);
        assert!(pair.rms.all() <= PAIR_RMS_LIMIT);
        assert!(pair.rms.front() <= reference.rms.front() + FRONT_RMS_ALLOWANCE);
        assert!(pair.rms.side() > PAIR_RMS_LIMIT);
        let fit = fit_from_films(&request).unwrap();
        assert!(fit.side_film.is_none());
        assert_eq!(fit, reference.fit);
    }

    #[test]
    fn a_refused_pair_without_typical_values_returns_the_original_front_only_error() {
        let mut request = skewed_lumbar_side();
        request.typical = TypicalSource::None;
        let mut alone = request.clone();
        alone.side = None;
        assert_eq!(fit_from_films(&alone), Err(FitError::Underconstrained));
        assert_eq!(fit_from_films(&request), fit_from_films(&alone));
    }

    #[test]
    fn a_side_only_twist_can_fit_even_when_the_front_reference_is_invalid() {
        let mut request = pair_request(crate::answer_keys::upright_input());
        request
            .front
            .marks
            .vertebrae
            .retain(|v| v.level >= Level::L4);
        request.typical = TypicalSource::None;
        request
            .twist_readings
            .push(shapes::spine_fit::TwistReading {
                level: Level::L3,
                twist_rad: 0.0,
                sd_rad: 0.05,
            });
        assert!(front_reference(&request).is_none());
        let mut alone = request.clone();
        alone.side = None;
        assert_eq!(fit_from_films(&alone), Err(FitError::Underconstrained));
        let fit = fit_from_films(&request).unwrap();
        assert!(fit.converged && fit.side_film.is_some());
    }

    #[test]
    fn resampling_an_unscaled_side_keeps_its_patient_millimetre_rms() {
        let mut original = pair_request(input(RecipeName::RightThoracic40, 4, 0.0).0);
        original.front.geometry.mm_per_px = None;
        let side = original.side.as_mut().unwrap();
        side.geometry.mm_per_px = None;
        let spacing = crate::answer_keys::input(RecipeName::RightThoracic40, 4, 0.0)
            .0
            .side_geometry
            .mm_per_px
            .unwrap();
        let mut stream = Stream::new("honest Side resolution", 19);
        for v in &mut side.marks.vertebrae {
            let VertebraMarks::Side { points } = &mut v.marks else {
                unreachable!("Side")
            };
            for mark in [
                &mut points.sup_a,
                &mut points.sup_p,
                &mut points.inf_a,
                &mut points.inf_p,
            ] {
                mark.at = Px::new(
                    mark.at.x() + stream.normal(0.0, 1.0) / spacing,
                    mark.at.y() + stream.normal(0.0, 1.0) / spacing,
                );
            }
        }
        let mut rms = Vec::new();
        for density in [0.5, 1.0, 2.0] {
            let mut request = original.clone();
            let side = request.side.as_mut().unwrap();
            side.size = side.size.map(|n| (f64::from(n) * density) as u32);
            for v in &mut side.marks.vertebrae {
                let VertebraMarks::Side { points } = &mut v.marks else {
                    unreachable!("Side")
                };
                for mark in [
                    &mut points.sup_a,
                    &mut points.sup_p,
                    &mut points.inf_a,
                    &mut points.inf_p,
                ] {
                    mark.at = Px::new(mark.at.x() * density, mark.at.y() * density);
                }
            }
            let pair = assessed(&request);
            eprintln!(
                "Side resolution density={density}: all={} front={} side={}",
                pair.rms.all(),
                pair.rms.front(),
                pair.rms.side()
            );
            rms.push(pair.rms.side());
            assert!(fit_from_films(&request).unwrap().side_film.is_some());
        }
        let low = rms.iter().copied().fold(f64::INFINITY, f64::min);
        let high = rms.iter().copied().fold(0.0, f64::max);
        assert!(
            high - low < 0.1,
            "patient Side RMS across resolutions: {rms:?}"
        );
    }

    #[test]
    fn noise_beyond_its_stated_spread_is_refused_when_fitted_residuals_exceed_the_limits() {
        use shapes::spine_fit::MarkSpread;
        for lumbar_only in [false, true] {
            let mut verdicts = Vec::new();
            for multiple in [1.0, 1.5, 2.0, 2.5, 3.0] {
                let mut request = pair_request(input(RecipeName::RightThoracic40, 4, multiple).0);
                request.front.mark_spread = MarkSpread::Millimetres {
                    sd_mm: 1.0,
                    shared_sd_mm: 0.0,
                };
                request.front.geometry.mm_per_px = None;
                let side = request.side.as_mut().unwrap();
                side.geometry.mm_per_px = None;
                if lumbar_only {
                    side.marks.vertebrae.retain(|v| v.level >= Level::L1);
                }
                let pair = assessed(&request);
                let used = fit_from_films(&request).unwrap().side_film.is_some();
                eprintln!(
                    "noise multiple={multiple} lumbar_only={lumbar_only}: all={} front={} side={} reference={:?} used={used}",
                    pair.rms.all(),
                    pair.rms.front(),
                    pair.rms.side(),
                    front_reference(&request).map(|f| f.rms.front())
                );
                verdicts.push(used);
            }
            assert!(verdicts[0], "honest noise must pass: {verdicts:?}");
            assert!(
                !verdicts[4],
                "three times the stated noise must fail: {verdicts:?}"
            );
        }
    }

    #[test]
    fn every_residual_rule_has_its_own_inclusive_limit() {
        let epsilon = 1e-9;
        for (all, front, side, reference, expected) in [
            (1.75 - epsilon, 0.5, 0.5, Some(0.5), true),
            (1.75, 0.5, 0.5, Some(0.5), true),
            (1.75 + epsilon, 0.5, 0.5, Some(0.5), false),
            (1.0, 0.5, 1.75 - epsilon, Some(0.5), true),
            (1.0, 0.5, 1.75, Some(0.5), true),
            (1.0, 0.5, 1.75 + epsilon, Some(0.5), false),
            (1.0, 1.0 - epsilon, 0.5, Some(0.5), true),
            (1.0, 1.0, 0.5, Some(0.5), true),
            (1.0, 1.0 + epsilon, 0.5, Some(0.5), false),
            (1.0, 2.0, 1.0, None, true),
            (1.0, 2.0, 1.0, Some(0.5), false),
            (1.75 + epsilon, 0.5, 0.5, None, false),
            (1.0, 0.5, 1.75 + epsilon, None, false),
            (f64::NAN, 0.5, 0.5, None, false),
            (1.0, 0.5, f64::INFINITY, None, false),
            (1.0, 0.5, 0.5, Some(f64::NAN), false),
        ] {
            assert_eq!(
                residuals_agree(all, front, side, reference),
                expected,
                "all={all} front={front} side={side} reference={reference:?}"
            );
        }
    }

    #[test]
    fn a_pair_without_a_converged_finite_solution_retries_front_alone() {
        let original = crate::answer_keys::upright_input();
        let spread = MarkSpread::Millimetres {
            sd_mm: 1.0,
            shared_sd_mm: 0.0,
        };
        let input = FitFromFilms {
            front: FilmToFit {
                marks: original.front,
                geometry: original.front_geometry,
                size: original.front_size,
                mark_spread: spread,
                facing_assumed: true,
            },
            side: Some(FilmToFit {
                marks: original.side,
                geometry: shapes::film_geometry::FilmGeometry {
                    mm_per_px: Some(1e-200),
                    ..original.side_geometry
                },
                size: original.side_size,
                mark_spread: spread,
                facing_assumed: false,
            }),
            typical: TypicalSource::BuiltIn,
            twist_readings: Vec::new(),
        };
        // A positive finite camera passes validation, but this extreme
        // calibration cannot give a finite converged covariance. Confirm
        // that the actual pair solve fails before exercising the retry.
        let (problem, _, _) = prepare(&input).expect("valid pair inputs");
        assert!(problem.side.is_some());
        let pair = fit(&problem);
        assert!(
            !matches!(pair, Ok(ref result) if result.converged),
            "{pair:?}"
        );
        let mut alone = input.clone();
        alone.side = None;
        assert_eq!(fit_from_films(&input), fit_from_films(&alone));
        // Front errors still win even when Side cannot be solved.
        let mut bad_front = input;
        bad_front.front.geometry.image_right = None;
        assert_eq!(
            fit_from_films(&bad_front),
            Err(FitError::MissingFacing(shapes::film::FilmKind::Front))
        );
    }

    /// Finder-like front marks, T1 to L5: no pedicles, each corner moved
    /// along each film axis by a normal step of 0.1 of its vertebra's
    /// height on the film, the same spread given to the fit. Names counted
    /// from both ends, or estimates as the trained finder gives them.
    fn finder_like(estimates: bool) -> FitFromFilms {
        let (input, _) = input(RecipeName::RightThoracic40, 2, 0.0);
        let mut marks = input.front;
        let mut stream = Stream::new("corners as a share of height", 2);
        for v in &mut marks.vertebrae {
            let VertebraMarks::Front { points } = &mut v.marks else {
                unreachable!("a front film");
            };
            for pedicle in [&mut points.ped_l, &mut points.ped_r] {
                *pedicle = PedicleMark::NotVisible {
                    source: MarkSource::Auto,
                };
            }
            let middle = |a: Px, b: Px| ((a.x() + b.x()) * 0.5, (a.y() + b.y()) * 0.5);
            let top = middle(points.sup_l.at, points.sup_r.at);
            let bottom = middle(points.inf_l.at, points.inf_r.at);
            let sd = 0.1 * libm::hypot(top.0 - bottom.0, top.1 - bottom.1);
            for mark in [
                &mut points.sup_l,
                &mut points.sup_r,
                &mut points.inf_l,
                &mut points.inf_r,
            ] {
                let (dx, dy) = (stream.normal(0.0, sd), stream.normal(0.0, sd));
                mark.at = Px::new(mark.at.x() + dx, mark.at.y() + dy);
            }
        }
        if estimates {
            // Named as the trained finder names: counted from neither end, flagged as estimates.
            marks.naming.from_top = None;
            marks.naming.from_bottom = None;
            marks.naming.flags = vec![shapes::naming::NamingFlag::Estimated];
        }
        assert_eq!(estimated_names::names_are_estimates(&marks), estimates);
        FitFromFilms {
            front: FilmToFit {
                facing_assumed: false,
                marks,
                geometry: input.front_geometry,
                size: input.front_size,
                mark_spread: MarkSpread::ShareOfHeight {
                    share: 0.1,
                    shared_share: 0.0,
                },
            },
            side: None,
            typical: TypicalSource::BuiltIn,
            twist_readings: Vec::new(),
        }
    }

    /// Every name one level higher in the standard template: T1 becomes
    /// C7 and L1 becomes T12.
    fn named_one_level_high(input: &FitFromFilms) -> FitFromFilms {
        let mut shifted = input.clone();
        for v in &mut shifted.front.marks.vertebrae {
            v.level = match v.level {
                Level::L1 => Level::T12,
                level => level.above().expect("a level above"),
            };
        }
        shifted
    }

    /// The fit, with one standard deviation of each vertebra's front wedge
    /// and of its top and bottom half-widths from the same covariance as
    /// the reported ones (the result reports none for them).
    fn fitted(input: &FitFromFilms) -> (SpineFit, Vec<[f64; 3]>) {
        let (problem, _, _) = prepare(input).expect("the marks prepare");
        let solution = levenberg_marquardt::solve(&problem, start_values::start(&problem))
            .expect("the marks fit");
        let fit = report(&problem, &solution).expect("the fit reports");
        assert!(fit.converged);
        let covariance =
            uncertainty::covariance(&solution, problem.rescale_covariance).expect("a covariance");
        let sd = |index: usize| libm::sqrt(covariance[(index, index)]);
        let more = fit
            .vertebrae
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let at = i * COUNT;
                // Half-widths are solved as logarithms.
                [
                    sd(at + 7),
                    v.pose.top_half_width * sd(at + 9),
                    v.pose.bottom_half_width * sd(at + 11),
                ]
            })
            .collect();
        (fit, more)
    }

    /// The largest change from naming every vertebra one level high, in
    /// units of the right-named fit's own sd: forward lean, then centre y,
    /// centre z, side tilt, front wedge and half-widths.
    fn largest_shift(input: &FitFromFilms) -> [f64; 6] {
        let (right, right_sd) = fitted(input);
        let (high, _) = fitted(&named_one_level_high(input));
        // L5 is absent from the shifted names, so the lowest bone, the
        // same one, is the origin in both fits.
        assert_eq!(high.origin_level, Level::L4);
        let mut largest = [0.0_f64; 6];
        for ((a, b), more) in right.vertebrae.iter().zip(&high.vertebrae).zip(right_sd) {
            let shifts = [
                (a.angles.forward_lean_rad - b.angles.forward_lean_rad).abs() / a.sd.angles_rad[1],
                (a.pose.centre[1] - b.pose.centre[1]).abs() / a.sd.centre_mm[1],
                (a.pose.centre[2] - b.pose.centre[2]).abs() / a.sd.centre_mm[2],
                (a.angles.side_tilt_rad - b.angles.side_tilt_rad).abs() / a.sd.angles_rad[0],
                (a.pose.wedge_front - b.pose.wedge_front).abs() / more[0],
                ((a.pose.top_half_width - b.pose.top_half_width).abs() / more[1])
                    .max((a.pose.bottom_half_width - b.pose.bottom_half_width).abs() / more[2]),
            ];
            for (most, shift) in largest.iter_mut().zip(shifts) {
                *most = most.max(shift);
            }
        }
        largest
    }

    #[test]
    fn names_that_are_estimates_move_by_less_than_their_spread() {
        // Finder-like marks whose names are estimates, then every name one
        // level too high. Measured largest change in sds of the right-named
        // fit: forward lean 0.865, centre y 0.015, centre z 0.018, side tilt
        // 0.081, front wedge 0.065, half-widths 0.083. Limits: 1 for lean,
        // 0.5 for the rest. Lean and twist change how a tilt shows on the
        // film, so what is measured moves a little too.
        let largest = largest_shift(&finder_like(true));
        eprintln!(
            "names estimated, one level off: lean, y, z, tilt, front wedge, half-widths in sds {largest:?}"
        );
        assert!(largest[0] < 1.0, "lean {}", largest[0]);
        for shift in &largest[1..] {
            assert!(*shift < 0.5, "{largest:?}");
        }
    }

    #[test]
    fn names_that_are_counted_keep_the_typical_spread() {
        // The same marks with names counted from both ends: no widening, so
        // a name one level off moves forward lean by more than its spread
        // where the typical side profile turns by more than that between
        // neighbouring levels. Measured largest change: 1.77 sds.
        let largest = largest_shift(&finder_like(false));
        eprintln!(
            "names counted, one level off: lean, y, z, tilt, front wedge, half-widths in sds {largest:?}"
        );
        assert!(largest[0] > 1.0, "lean {}", largest[0]);
    }
}
