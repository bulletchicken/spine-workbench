//! Where each film stands against the fitted spine, from the camera the fit
//! projected through: the scale per stored pixel, and the upright pixel
//! where the reported patient origin lands.

use forward_model::cameras::parallel::ParallelCamera;
use forward_model::patient_frame::screen_right;
use nalgebra::Vector3;
use shapes::spine_fit::FilmPlacement;

/// `translation` is the patient origin in the frame the camera was built
/// in: every reported centre is the fitted centre less it.
pub fn of(camera: &ParallelCamera, translation: &Vector3<f64>) -> FilmPlacement {
    let [su, sv] = camera.upright_mm_per_px;
    // The upright pixel's width and height are the stored pixel's swapped
    // when the quarter turns are odd.
    let mm_per_px = if camera.quarter_turns.get().is_multiple_of(2) {
        [su, sv]
    } else {
        [sv, su]
    };
    let across = translation.dot(&screen_right(camera.image_right));
    FilmPlacement {
        facing_assumed: false,
        mm_per_px,
        origin_px: [
            camera.origin.x + across / su,
            camera.origin.y - translation.z / sv,
        ],
    }
}

/// A change of the common length gauge leaves every film pixel fixed.
pub fn in_units(camera: &ParallelCamera, translation: &Vector3<f64>, unit: f64) -> FilmPlacement {
    let placement = of(camera, translation);
    FilmPlacement {
        mm_per_px: placement.mm_per_px.map(|s| s * unit),
        ..placement
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::answer_keys::{anisotropic_input, input};
    use crate::parameters::{COUNT, vertebra};
    use crate::problem::Problem;
    use crate::{from_films, levenberg_marquardt, report, start_values};
    use forward_model::project::{front_marks, side_marks};
    use forward_model::vertebra_params::VertebraParams;
    use nalgebra::Vector2;
    use shapes::film_geometry::{FilmGeometry, ImageRight, QuarterTurns};
    use shapes::made_up::RecipeName;
    use shapes::marks::{Mark, PedicleMark};
    use shapes::pixels::Px;
    use shapes::spine_fit::{FilmToFit, FitFromFilms, FitInput, MarkSpread, TypicalSource};
    use shapes::vertebra::VertebraMarks;

    /// The front film of a made-up pair alone, with the typical spine.
    fn front_only(input: FitInput) -> FitFromFilms {
        FitFromFilms {
            front: FilmToFit {
                facing_assumed: false,
                marks: input.front,
                geometry: input.front_geometry,
                size: input.front_size,
                mark_spread: MarkSpread::Millimetres {
                    sd_mm: 1.0,
                    shared_sd_mm: 0.0,
                },
            },
            side: None,
            typical: TypicalSource::BuiltIn,
            twist_readings: Vec::new(),
        }
    }

    /// The same film stored turned a quarter turn anticlockwise, so that
    /// one clockwise quarter turn shows it upright again.
    fn stored_turned(film: &mut FilmToFit) {
        let [width, height] = film.size;
        let last = f64::from(width) - 1.0;
        let turn = |at: Px| Px::new(at.y(), last - at.x());
        for v in &mut film.marks.vertebrae {
            let VertebraMarks::Front { points } = &mut v.marks else {
                unreachable!("a front film");
            };
            for mark in [
                &mut points.sup_l,
                &mut points.sup_r,
                &mut points.inf_l,
                &mut points.inf_r,
            ] {
                mark.at = turn(mark.at);
            }
            for pedicle in [&mut points.ped_l, &mut points.ped_r] {
                if let PedicleMark::Visible { at, .. } = pedicle {
                    *at = turn(*at);
                }
            }
        }
        let geometry = &mut film.geometry;
        let along_row = geometry.mm_per_px.expect("a scaled film");
        let down_column = geometry.mm_per_px_vertical.unwrap_or(along_row);
        geometry.mm_per_px = Some(down_column);
        geometry.mm_per_px_vertical = Some(along_row);
        geometry.quarter_turns = QuarterTurns::new(1).expect("one turn");
        film.size = [height, width];
    }

    /// The camera rebuilt from nothing but a reported placement and the
    /// film's own geometry (facing and quarter turns) and stored size.
    fn rebuilt(
        placement: FilmPlacement,
        geometry: &FilmGeometry,
        size: [u32; 2],
    ) -> ParallelCamera {
        let geometry = FilmGeometry {
            mm_per_px: Some(placement.mm_per_px[0]),
            mm_per_px_vertical: Some(placement.mm_per_px[1]),
            ..geometry.clone()
        };
        ParallelCamera::for_film(&geometry, size, Vector2::from(placement.origin_px))
            .expect("a known camera")
    }

    fn gap(a: [Mark; 4], b: [Mark; 4]) -> f64 {
        a.iter()
            .zip(&b)
            .map(|(p, q)| libm::hypot(p.at.x() - q.at.x(), p.at.y() - q.at.y()))
            .fold(0.0, f64::max)
    }

    /// Solve the problem and return the largest distance, in film pixels,
    /// between the fit's own projected corners of each vertebra and the
    /// corners of its reported pose projected through the rebuilt camera,
    /// on the front film and on the side film.
    fn largest_gap(
        problem: &Problem,
        front: (&FilmGeometry, [u32; 2]),
        side: Option<(&FilmGeometry, [u32; 2])>,
    ) -> [f64; 2] {
        let solution = levenberg_marquardt::solve(problem, start_values::start(problem))
            .expect("the problem solves");
        let fit = report(problem, &solution).expect("the solution reports");
        assert_eq!(fit.side_film.is_some(), side.is_some());
        let numbers = solution.numbers.as_slice();
        let global = problem.observations.len() * COUNT;
        let own_side = problem.side_camera(numbers[global], numbers[global + 1]);
        let front_camera = rebuilt(fit.front_film, front.0, front.1);
        let side_camera = fit
            .side_film
            .zip(side)
            .map(|(placement, (geometry, size))| rebuilt(placement, geometry, size));
        let mut largest = [0.0_f64; 2];
        for (i, fitted) in fit.vertebrae.iter().enumerate() {
            let own = vertebra(&numbers[i * COUNT..(i + 1) * COUNT]);
            let reported = VertebraParams::from(&fitted.pose);
            let corners = |v: &VertebraParams, camera: &ParallelCamera| {
                front_marks(v, camera).expect("front marks").corners()
            };
            largest[0] = largest[0].max(gap(
                corners(&own, &problem.front),
                corners(&reported, &front_camera),
            ));
            if let (Some(own_camera), Some(camera)) = (&own_side, &side_camera) {
                let corners = |v: &VertebraParams, camera: &ParallelCamera| {
                    side_marks(v, camera).expect("side marks").corners()
                };
                largest[1] =
                    largest[1].max(gap(corners(&own, own_camera), corners(&reported, camera)));
            }
        }
        largest
    }

    fn one_film(input: &FitFromFilms) -> [f64; 2] {
        let (problem, _, _) = from_films::prepare(input).expect("the film prepares");
        largest_gap(&problem, (&input.front.geometry, input.front.size), None)
    }

    #[test]
    fn the_reported_placements_rebuild_the_cameras_the_fit_used() {
        // Each film's camera is rebuilt from its reported scale and origin
        // pixel with the film's own facing, quarter turns and stored size,
        // and each fitted vertebra's reported pose is projected through it.
        // Measured largest gap to the fit's own projection: under 1e-9 px
        // in every case. Limit 1e-6 px.
        let mut gaps = Vec::new();
        let scaled = front_only(input(RecipeName::RightThoracic40, 3, 0.0).0);
        gaps.push(("a film with a scale", one_film(&scaled)));
        let mut unscaled = scaled.clone();
        unscaled.front.geometry.mm_per_px = None;
        gaps.push(("a film without one", one_film(&unscaled)));
        let mirrored = front_only(input(RecipeName::MirroredFront, 3, 0.0).0);
        assert_eq!(
            mirrored.front.geometry.image_right,
            Some(ImageRight::PatientRight)
        );
        gaps.push((
            "a front film facing the patient's right",
            one_film(&mirrored),
        ));
        let mut turned = front_only(anisotropic_input(RecipeName::RightThoracic40, 3).0);
        stored_turned(&mut turned.front);
        gaps.push(("a film with a quarter turn", one_film(&turned)));
        let upside_down = front_only(input(RecipeName::UpsideDown, 3, 0.0).0);
        assert_eq!(upside_down.front.geometry.quarter_turns.get(), 2);
        gaps.push(("a film with two quarter turns", one_film(&upside_down)));
        // Both films, through the two-film entry, with pixels twice as tall
        // as wide, so a side scale and offset are fitted.
        let (pair, _) = anisotropic_input(RecipeName::LumbarLeft30, 3);
        let problem = Problem::new(&pair).expect("the pair is valid");
        gaps.push((
            "a front and side pair",
            largest_gap(
                &problem,
                (&pair.front_geometry, pair.front_size),
                Some((&pair.side_geometry, pair.side_size)),
            ),
        ));
        // All four calibration combinations rebuild the actual solver
        // cameras, including the Side-only measured length gauge. A
        // reconstruction error is numerical only: limit 1e-6 pixels.
        let pair = front_only(input(RecipeName::RightThoracic40, 3, 0.0).0);
        let original = input(RecipeName::RightThoracic40, 3, 0.0).0;
        let side = FilmToFit {
            facing_assumed: true,
            marks: original.side,
            geometry: original.side_geometry,
            size: original.side_size,
            mark_spread: pair.front.mark_spread,
        };
        for front_scaled in [false, true] {
            for side_scaled in [false, true] {
                let mut request = pair.clone();
                request.side = Some(side.clone());
                if !front_scaled {
                    request.front.geometry.mm_per_px = None;
                }
                if !side_scaled {
                    request.side.as_mut().expect("Side").geometry.mm_per_px = None;
                }
                let (problem, _, _) = from_films::prepare(&request).expect("pair prepares");
                gaps.push((
                    "ordinary pair",
                    largest_gap(
                        &problem,
                        (&request.front.geometry, request.front.size),
                        request
                            .side
                            .as_ref()
                            .map(|film| (&film.geometry, film.size)),
                    ),
                ));
            }
        }
        eprintln!("largest gap, front and side, px: {gaps:?}");
        for (case, [front, side]) in gaps {
            assert!(front < 1e-6 && side < 1e-6, "{case}: {front} {side}");
        }
    }
}
