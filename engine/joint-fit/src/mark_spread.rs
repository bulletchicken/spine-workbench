//! Each film's corner spread in millimetres, for each vertebra it shows,
//! and the part of it all four corners share.

use crate::FitError;
use crate::problem::{Observations, Problem, millimetres};
use forward_model::cameras::parallel::ParallelCamera;
use shapes::film::FilmKind;
use shapes::marks::Mark;
use shapes::spine_fit::{FitStandardDeviations, MarkSpread};

/// Replace the default 1 mm with each film's own spread, and the default
/// of no shared part with each film's own. A share of height uses the
/// distance between the vertebra's endplate midpoints on the film.
pub fn attach(
    problem: &mut Problem,
    front: MarkSpread,
    side: Option<MarkSpread>,
) -> Result<(), FitError> {
    let Problem {
        observations,
        front: front_camera,
        side: side_camera,
        ..
    } = problem;
    for observation in observations {
        if let Some(marks) = observation.front {
            [observation.spread_mm[0], observation.shared_mm[0]] =
                corner_sd(front, front_camera, marks.corners(), FilmKind::Front)?;
        }
        if let (Some(marks), Some(camera), Some(spread)) = (observation.side, &*side_camera, side) {
            [observation.spread_mm[1], observation.shared_mm[1]] =
                corner_sd(spread, camera, marks.corners(), FilmKind::Side)?;
        }
    }
    Ok(())
}

/// The shared part adds in quadrature to the spread of the centre across
/// each film and up it: y and z on the front film, x and z on the side
/// film. A box that sits off as a whole moves its centre and nothing else.
pub fn add_shared(sd: &mut FitStandardDeviations, observation: &Observations) {
    let [front, side] = observation.shared_mm;
    for (axis, shared) in [(1, front), (2, front), (0, side), (2, side)] {
        if shared > 0.0 {
            let centre = sd.centre_mm[axis];
            sd.centre_mm[axis] = libm::sqrt(centre * centre + shared * shared);
        }
    }
}

/// A corner's own spread and the shared part, in millimetres.
fn corner_sd(
    spread: MarkSpread,
    camera: &ParallelCamera,
    corners: [Mark; 4],
    film: FilmKind,
) -> Result<[f64; 2], FitError> {
    let (given, shared) = match spread {
        MarkSpread::Millimetres {
            sd_mm,
            shared_sd_mm,
        } => (sd_mm, shared_sd_mm),
        MarkSpread::ShareOfHeight {
            share,
            shared_share,
        } => (share, shared_share),
    };
    if !(given.is_finite() && given > 0.0 && shared.is_finite() && shared >= 0.0) {
        return Err(FitError::InvalidMarkSpread(film));
    }
    let unit = match spread {
        MarkSpread::Millimetres { .. } => 1.0,
        MarkSpread::ShareOfHeight { .. } => {
            let [a, b, c, d] = corners.map(|m| millimetres(camera, m));
            ((a + b) * 0.5 - (c + d) * 0.5).norm()
        }
    };
    let sd = given * unit;
    if !(sd.is_finite() && sd > 0.0) {
        return Err(FitError::InvalidMarks(film));
    }
    Ok([sd, shared * unit])
}
