//! Validated marks and parallel cameras in the provisional film frame.

use crate::FitError;
use crate::typical_spine::Typical;
use forward_model::cameras::parallel::ParallelCamera;
use nalgebra::Vector2;
use shapes::film::FilmKind;
use shapes::film_geometry::{FilmGeometry, ImageRight};
use shapes::levels::Level;
use shapes::marks::{FrontMarks, Mark, SideMarks};
use shapes::pixels::Px;
use shapes::spine_fit::{FitInput, TwistReading};
use shapes::vertebra::{FilmMarks, VertebraMarks};
use std::collections::BTreeMap;

pub struct Observations {
    pub level: Level,
    pub front: Option<FrontMarks>,
    pub side: Option<SideMarks>,
    /// One standard deviation of a placed corner on the front and the side
    /// film, mm.
    pub spread_mm: [f64; 2],
    /// The part of a corner's error all four corners share on the front and
    /// the side film, mm.
    pub shared_mm: [f64; 2],
    pub twist_reading: Option<TwistReading>,
}

impl Observations {
    pub fn has_pedicle(&self) -> bool {
        self.front
            .is_some_and(|m| m.ped_l.mark().is_some() || m.ped_r.mark().is_some())
    }
}

pub struct Problem {
    pub observations: Vec<Observations>,
    pub front: ParallelCamera,
    pub side: Option<ParallelCamera>,
    pub front_spacing: [f64; 2],
    pub side_spacing: [f64; 2],
    /// Only two stated calibrations justify the relative-scale prior.
    /// An absent Side still pins its unused scale to one.
    pub calibrated_pair: bool,
    /// Report the common length gauge in the measured Side's units when
    /// the Front spacing was provisional.
    pub side_is_length_anchor: bool,
    /// Each observation's typical values, when a typical spine was asked for.
    pub typical: Option<Typical>,
    /// Scale the covariance by the residual variance, as the two-film fit
    /// with its fixed mark weights does.
    pub rescale_covariance: bool,
}

/// One film's marks, geometry and stored size, as the problem reads them.
pub struct Film<'a> {
    pub marks: &'a FilmMarks,
    pub geometry: &'a FilmGeometry,
    pub size: [u32; 2],
}

impl Problem {
    pub fn new(input: &FitInput) -> Result<Self, FitError> {
        let problem = Self::of_films(
            Film {
                marks: &input.front,
                geometry: &input.front_geometry,
                size: input.front_size,
            },
            Some(Film {
                marks: &input.side,
                geometry: &input.side_geometry,
                size: input.side_size,
            }),
        )?;
        if input.front.vertebrae.is_empty() || input.side.vertebrae.is_empty() {
            return Err(FitError::Underconstrained);
        }
        Ok(problem)
    }

    /// Cameras, validated marks and observations by level, with a corner
    /// spread of 1 mm and none of it shared, no twist readings and no
    /// typical values.
    pub fn of_films(front: Film, side: Option<Film>) -> Result<Self, FitError> {
        let front_camera = camera(front.geometry, front.size, FilmKind::Front)?;
        let side_camera = side
            .as_ref()
            .map(|film| camera(film.geometry, film.size, FilmKind::Side))
            .transpose()?;
        validate_marks(front.marks, FilmKind::Front, front_camera.image_right)?;
        if let (Some(film), Some(camera)) = (&side, &side_camera) {
            validate_marks(film.marks, FilmKind::Side, camera.image_right)?;
        }
        let side_vertebrae = side.as_ref().map_or(&[][..], |film| &film.marks.vertebrae);
        let mut by_level = BTreeMap::new();
        for v in front.marks.vertebrae.iter().chain(side_vertebrae) {
            let entry = by_level.entry(v.level).or_insert(Observations {
                level: v.level,
                front: None,
                side: None,
                spread_mm: [1.0, 1.0],
                shared_mm: [0.0, 0.0],
                twist_reading: None,
            });
            match v.marks {
                VertebraMarks::Front { points } => entry.front = Some(points),
                VertebraMarks::Side { points } => entry.side = Some(points),
                VertebraMarks::Bending { .. } => unreachable!("validated view"),
            }
        }
        if by_level.is_empty() {
            return Err(FitError::NoMarks);
        }
        // An empty film has no lowest vertebra; callers refuse it before
        // its camera is used.
        let origin = |marks: &FilmMarks, camera: &ParallelCamera| {
            let Some(last) = marks.vertebrae.last() else {
                return Vector2::zeros();
            };
            let corners = last.marks.corners();
            (upright(camera, corners[2].at) + upright(camera, corners[3].at)) * 0.5
        };
        Ok(Self {
            observations: by_level.into_values().collect(),
            front: ParallelCamera {
                origin: origin(front.marks, &front_camera),
                ..front_camera
            },
            side: side
                .as_ref()
                .zip(side_camera)
                .map(|(film, camera)| ParallelCamera {
                    origin: origin(film.marks, &camera),
                    ..camera
                }),
            front_spacing: spacing(front.geometry),
            side_spacing: side
                .as_ref()
                .map_or([1.0, 1.0], |film| spacing(film.geometry)),
            typical: None,
            calibrated_pair: true,
            side_is_length_anchor: false,
            rescale_covariance: true,
        })
    }

    /// Positive offset means add this height to side-film millimetres to
    /// express them in the front film's provisional vertical frame. `None`
    /// when there is no side film.
    pub fn side_camera(&self, scale_log: f64, offset: f64) -> Option<ParallelCamera> {
        let side = self.side?;
        let scale = libm::exp(scale_log);
        let spacing = side.upright_mm_per_px.map(|s| s * scale);
        Some(ParallelCamera {
            upright_mm_per_px: spacing,
            origin: side.origin + Vector2::new(0.0, offset / spacing[1]),
            ..side
        })
    }
}

fn spacing(geometry: &FilmGeometry) -> [f64; 2] {
    let x = geometry.mm_per_px.expect("validated scale");
    [x, geometry.mm_per_px_vertical.unwrap_or(x)]
}

fn camera(
    geometry: &FilmGeometry,
    size: [u32; 2],
    film: FilmKind,
) -> Result<ParallelCamera, FitError> {
    if geometry.mm_per_px.is_none() {
        return Err(FitError::MissingScale(film));
    }
    let facing = geometry.image_right.ok_or(FitError::MissingFacing(film))?;
    if facing.suits_front_view() != (film == FilmKind::Front) {
        return Err(FitError::InvalidFacing(film));
    }
    if size.contains(&0) {
        return Err(FitError::InvalidSize(film));
    }
    ParallelCamera::for_film(geometry, size, Vector2::zeros()).ok_or(FitError::InvalidScale(film))
}

fn validate_marks(marks: &FilmMarks, film: FilmKind, facing: ImageRight) -> Result<(), FitError> {
    if marks.film != film || marks.named_for != facing {
        return Err(FitError::InvalidMarks(film));
    }
    let mut previous = None;
    for v in &marks.vertebrae {
        if !v.marks.suits(film) || previous.is_some_and(|level| level >= v.level) {
            return Err(FitError::InvalidMarks(film));
        }
        previous = Some(v.level);
        let mut points = v.marks.corners().to_vec();
        if let VertebraMarks::Front { points: m } = v.marks {
            points.extend([m.ped_l.mark(), m.ped_r.mark()].into_iter().flatten());
        }
        if points
            .iter()
            .any(|m| !m.at.x().is_finite() || !m.at.y().is_finite())
        {
            return Err(FitError::InvalidMarks(film));
        }
    }
    Ok(())
}

pub fn upright(camera: &ParallelCamera, point: Px) -> Vector2<f64> {
    let w = f64::from(camera.film_size[0]) - 1.0;
    let h = f64::from(camera.film_size[1]) - 1.0;
    let (x, y) = (point.x(), point.y());
    match camera.quarter_turns.get() {
        0 => Vector2::new(x, y),
        1 => Vector2::new(h - y, x),
        2 => Vector2::new(w - x, h - y),
        _ => Vector2::new(y, w - x),
    }
}

/// Horizontal patient coordinate (x or y), then height, in millimetres.
pub fn millimetres(camera: &ParallelCamera, mark: Mark) -> Vector2<f64> {
    let pixel = upright(camera, mark.at) - camera.origin;
    let sign = match camera.image_right {
        ImageRight::PatientLeft | ImageRight::Anterior => 1.0,
        ImageRight::PatientRight | ImageRight::Posterior => -1.0,
    };
    Vector2::new(
        sign * pixel.x * camera.upright_mm_per_px[0],
        -pixel.y * camera.upright_mm_per_px[1],
    )
}
