//! A cheap bone-density trace on a coarse copy of the film, run before the
//! auto pass. It also reads the collimation bands along the edges and checks
//! whether the brightness looks the right way round.

pub(crate) mod exclusions;
mod profile;

use darkroom::grey_image::GreyImage;
use film_reader::decoded_film::DecodedFilm;
use shapes::errors::{EngineError, ErrorCode};
use shapes::film_geometry::{PolarityGuess, QuarterTurns};
use shapes::pixels::PxRect;

pub use profile::{Profile, TraceRow};

/// A coarse analysis copy. Its placement maps every result back to the
/// unchanged film. Invalid pixels contribute to neither statistics nor
/// the bone trace.
#[derive(Clone, Debug)]
pub struct RoughTrace {
    pub image: GreyImage<f32>,
    pub valid: Vec<bool>,
    pub collimation: Vec<PxRect>,
    pub polarity: PolarityGuess,
}

impl RoughTrace {
    /// Box means, shutter and collimation exclusions, and a brightness
    /// proposal. No brightness or orientation proposal alters the film.
    pub fn new(film: &DecodedFilm) -> Result<Self, EngineError> {
        let (w, h) = (film.image.width(), film.image.height());
        if w == 0 || h == 0 || w != film.facts.width || h != film.facts.height {
            return Err(EngineError::new(
                ErrorCode::GeometryNoSpineFound,
                "geometry needs a nonempty film with matching dimensions",
            ));
        }
        // Eight-pixel boxes are the intake working resolution. Limit the
        // long side on very large films; this is a rough cue, not a mark.
        let factor = (f64::from(w.max(h)) / 768.0)
            .max(8.0)
            .min(f64::from(w.min(h)));
        let image = darkroom::pyramid::shrink(&film.image, factor, factor)
            .expect("a finite shrink factor of at least one");
        let mut valid = exclusions::shutter_mask(&image, film.facts.kept_tags.as_ref());
        let collimation = exclusions::collimation(&image, &valid);
        for (i, keep) in valid.iter_mut().enumerate() {
            let p = image.film_point(
                (i % image.width() as usize) as f64,
                (i / image.width() as usize) as f64,
            );
            *keep &= !collimation.iter().any(|b| {
                p.x() >= b.left && p.x() <= b.right && p.y() >= b.top && p.y() <= b.bottom
            });
        }
        let mut values: Vec<f32> = image
            .pixels()
            .iter()
            .zip(&valid)
            .filter_map(|(&p, &keep)| keep.then_some(p))
            .collect();
        values.sort_by(f32::total_cmp);
        let lo = percentile(&values, 0.01);
        let hi = percentile(&values, 0.99);
        let range = hi - lo;
        let image = image.map(|&p| {
            if range > 0.0 {
                ((p - lo) / range).clamp(0.0, 1.0)
            } else {
                0.0
            }
        });
        let polarity = profile::polarity(&image, &valid);
        Ok(Self {
            image,
            valid,
            collimation,
            polarity,
        })
    }

    /// Body outline and a smooth bone-density trace in a proposed upright
    /// working frame. This does not find vertebrae or name their levels.
    pub fn profile(&self, turns: QuarterTurns) -> Profile {
        profile::build(&self.image, &self.valid, turns)
    }
}

pub(crate) fn percentile(values: &[f32], fraction: f64) -> f32 {
    if values.is_empty() {
        0.0
    } else {
        values[((values.len() - 1) as f64 * fraction) as usize]
    }
}

pub(crate) fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let n = values.len();
    Some((values[(n - 1) / 2] + values[n / 2]) * 0.5)
}
