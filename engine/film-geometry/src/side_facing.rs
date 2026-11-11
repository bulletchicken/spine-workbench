//! Which way a side film faces: the upper back bulges backward. A proposal;
//! the user confirms.

use crate::rough_trace::Profile;
use crate::upright::file_image_right;
use film_reader::decoded_film::DecodedFilm;
use shapes::film::IntakeFlag;
use shapes::film_geometry::{
    Certainty, FacingCue, FacingEvidence, ImageRight, ImageRightGuess, QuarterTurns,
};

/// Infer the anterior side from a thoracic bulge and from the bone ridge
/// lying behind the trunk outline. The latter is an experimental proxy
/// for posterior elements, not a segmentation of individual structures.
/// Conflicting evidence reduces certainty instead of setting a fact.
pub fn guess(film: &DecodedFilm, profile: &Profile, turns: QuarterTurns) -> ImageRightGuess {
    // A threshold may select just the bone ridge instead of soft tissue.
    // Such a narrow component cannot provide an independent trunk centre.
    let body = profile
        .midline()
        .filter(|(_, width)| *width >= 0.15 * profile.width as f64);
    let width = body
        .map_or(0.4 * profile.width as f64, |(_, width)| width)
        .max(1.0);
    let range = |a: f64, b: f64| {
        &profile.rows[(a * profile.height as f64) as usize
            ..((b * profile.height as f64) as usize).min(profile.rows.len())]
    };
    let mean_spine = |a, b| {
        let values: Vec<_> = range(a, b).iter().filter_map(|row| row.spine_x).collect();
        (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
    };
    let posterior = {
        let offsets: Vec<_> = range(0.25, 0.75)
            .iter()
            .filter_map(|row| row.centre.zip(row.spine_x).map(|(c, s)| c - s))
            .collect();
        (body.is_some() && !offsets.is_empty()).then(|| {
            (offsets.iter().sum::<f64>() / offsets.len() as f64 / (0.12 * width)).clamp(-1.0, 1.0)
        })
    };
    let bulge = mean_spine(0.12, 0.22)
        .zip(mean_spine(0.55, 0.65))
        .zip(mean_spine(0.30, 0.43))
        .map(|((top, bottom), middle)| {
            ((0.5 * (top + bottom) - middle) / (0.10 * width)).clamp(-1.0, 1.0)
        });
    let score = 0.7 * posterior.unwrap_or(0.0) + 0.3 * bulge.unwrap_or(0.0);
    let image_right = if score.abs() >= 0.12 {
        Some(if score > 0.0 {
            ImageRight::Anterior
        } else {
            ImageRight::Posterior
        })
    } else {
        None
    };
    let file_says = file_image_right(film, turns);
    let agreement = file_says
        .zip(image_right)
        .map(|(file, guess)| if file == guess { 1.0 } else { -1.0 });
    let conflict =
        agreement == Some(-1.0) || posterior.zip(bulge).is_some_and(|(p, b)| p * b < -0.1);
    let certainty = if image_right.is_none() {
        Certainty::None
    } else if conflict || score.abs() < 0.3 {
        Certainty::Low
    } else if agreement == Some(1.0) && score.abs() >= 0.5 {
        Certainty::High
    } else {
        Certainty::Medium
    };
    let signed = |cue: Option<f64>| {
        image_right.and_then(|right| {
            cue.map(|strength| {
                if right == ImageRight::Anterior {
                    strength
                } else {
                    -strength
                }
            })
        })
    };
    ImageRightGuess {
        image_right,
        certainty,
        file_says,
        evidence: vec![
            FacingEvidence {
                cue: FacingCue::FileOrientation,
                strength: agreement,
            },
            FacingEvidence {
                cue: FacingCue::UpperBackBulge,
                strength: signed(bulge),
            },
            FacingEvidence {
                cue: FacingCue::PosteriorElements,
                strength: signed(posterior),
            },
        ],
        looks_mirrored: film.facts.flags.contains(&IntakeFlag::ExifMirrored),
    }
}
