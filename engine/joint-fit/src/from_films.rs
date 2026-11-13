//! One front film, or a front and side pair, fitted as one spine, with the
//! typical spine for what the films cannot show.

use crate::problem::{Film, Problem};
use crate::{
    FitError, curve_twist, estimated_names, fit, front_curves, mark_spread, scale, twist_readings,
    typical_spine,
};
use shapes::levels::Level;
use shapes::spine_fit::{FitFromFilms, FitLengths, SpineFit, TypicalSource};
use std::collections::BTreeSet;

/// Fit Front first, using Side when it supplies a usable, converged pair.
/// A Side set aside by the fit leaves exactly the Front-only problem and
/// result, including its length source, levels, spreads and placements.
pub fn fit_from_films(input: &FitFromFilms) -> Result<SpineFit, FitError> {
    let (problem, lengths, curve_deg) = prepare(input)?;
    let paired = problem.side.is_some();
    let solved = finish(input, &problem, lengths, curve_deg);
    if paired && !matches!(&solved, Ok(fit) if fit.converged) {
        let (problem, lengths, curve_deg) = prepare_selected(input, None)?;
        return finish(input, &problem, lengths, curve_deg);
    }
    solved
}

fn finish(
    input: &FitFromFilms,
    problem: &Problem,
    lengths: FitLengths,
    curve_deg: f64,
) -> Result<SpineFit, FitError> {
    let mut result = fit(problem)?;
    result.lengths = lengths;
    result.typical_curve_deg = problem.typical.is_some().then_some(curve_deg);
    result.front_film.facing_assumed = input.front.facing_assumed;
    if let Some(placement) = &mut result.side_film {
        placement.facing_assumed = input.side.as_ref().expect("a used Side").facing_assumed;
    }
    Ok(result)
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
    use crate::{levenberg_marquardt, report, start_values, uncertainty};
    use made_up_spines::seeded::Stream;
    use shapes::made_up::RecipeName;
    use shapes::marks::{MarkSource, PedicleMark};
    use shapes::pixels::Px;
    use shapes::spine_fit::{FilmToFit, MarkSpread};
    use shapes::vertebra::VertebraMarks;

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
