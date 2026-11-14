//! Fixed landmark channels become lumbar candidates in original film pixels.

use shapes::confidence::{Confidence, ConfidenceBand, ConfidenceReason};
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::{Film, FilmKind};
use shapes::landmarks::Landmarks;
use shapes::levels::Level;
use shapes::marks::{Mark, MarkSource, SideMarks};
use shapes::pixels::{Px, is_near_film};
use shapes::vertebra::{FilmMarks, VertebraMarks, VertebraOnFilm};

pub(super) const LEVELS: [Level; 5] = [Level::L1, Level::L2, Level::L3, Level::L4, Level::L5];

pub(super) fn film_marks(
    points: [[f64; 2]; 20],
    film: &Film,
    first: u32,
) -> Result<FilmMarks, EngineError> {
    let mut vertebrae = super::reserve(5)?;
    for (i, level) in LEVELS.into_iter().enumerate() {
        let mut corners = [Mark {
            at: Px::new(0.0, 0.0),
            source: MarkSource::Auto,
            pinned: false,
            inferred: false,
        }; 4];
        for (j, corner) in corners.iter_mut().enumerate() {
            corner.at = Px(points[i * 4 + j]);
            if !is_near_film(corner.at, film.image.width, film.image.height) {
                return Err(EngineError::new(
                    ErrorCode::CaseInvalidValue,
                    "a side candidate lies far outside its film",
                ));
            }
        }
        let [sup_a, sup_p, inf_a, inf_p] = corners;
        vertebrae.push(VertebraOnFilm {
            id: first + i as u32,
            level,
            marks: VertebraMarks::Side {
                points: SideMarks {
                    sup_a,
                    sup_p,
                    inf_a,
                    inf_p,
                },
            },
            confidence: Confidence {
                score: 0.0,
                band: ConfidenceBand::Low,
                reasons: vec![ConfidenceReason::LevelUncertain],
                calibrated: false,
                endplate_spread_px: None,
                margin: None,
            },
            outline: None,
        });
    }
    Ok(FilmMarks {
        film: FilmKind::Side,
        vertebrae,
        landmarks: Landmarks::NONE,
        naming: crate::to_marks::estimated_naming(),
        named_for: film.geometry.image_right.ok_or_else(super::lost)?,
    })
}
