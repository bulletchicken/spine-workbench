//! Validated marks and parallel cameras in the provisional film frame.

use crate::FitError;
use forward_model::cameras::parallel::ParallelCamera;
use nalgebra::Vector2;
use shapes::film::FilmKind;
use shapes::film_geometry::{FilmGeometry, ImageRight};
use shapes::levels::Level;
use shapes::marks::{FrontMarks, Mark, SideMarks};
use shapes::pixels::Px;
use shapes::spine_fit::FitInput;
use shapes::vertebra::{FilmMarks, VertebraMarks};
use std::collections::BTreeMap;

pub struct Observations {
    pub level: Level,
    pub front: Option<FrontMarks>,
    pub side: Option<SideMarks>,
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
    pub side: ParallelCamera,
    pub front_spacing: [f64; 2],
    pub side_spacing: [f64; 2],
}

impl Problem {
    pub fn new(input: &FitInput) -> Result<Self, FitError> {
        let front = camera(&input.front_geometry, input.front_size, FilmKind::Front)?;
        let side = camera(&input.side_geometry, input.side_size, FilmKind::Side)?;
        validate_marks(&input.front, FilmKind::Front, front.image_right)?;
        validate_marks(&input.side, FilmKind::Side, side.image_right)?;
        let mut by_level = BTreeMap::new();
        for v in input.front.vertebrae.iter().chain(&input.side.vertebrae) {
            let entry = by_level.entry(v.level).or_insert(Observations {
                level: v.level,
                front: None,
                side: None,
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
        if input.front.vertebrae.is_empty() || input.side.vertebrae.is_empty() {
            return Err(FitError::Underconstrained);
        }
        let origin = |marks: &FilmMarks, camera: &ParallelCamera| {
            let corners = marks
                .vertebrae
                .last()
                .expect("nonempty view")
                .marks
                .corners();
            (upright(camera, corners[2].at) + upright(camera, corners[3].at)) * 0.5
        };
        Ok(Self {
            observations: by_level.into_values().collect(),
            front: ParallelCamera {
                origin: origin(&input.front, &front),
                ..front
            },
            side: ParallelCamera {
                origin: origin(&input.side, &side),
                ..side
            },
            front_spacing: spacing(&input.front_geometry),
            side_spacing: spacing(&input.side_geometry),
        })
    }

    /// Positive offset means add this height to side-film millimetres to
    /// express them in the front film's provisional vertical frame.
    pub fn side_camera(&self, scale_log: f64, offset: f64) -> ParallelCamera {
        let scale = libm::exp(scale_log);
        let spacing = self.side.upright_mm_per_px.map(|s| s * scale);
        ParallelCamera {
            upright_mm_per_px: spacing,
            origin: self.side.origin + Vector2::new(0.0, offset / spacing[1]),
            ..self.side
        }
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
