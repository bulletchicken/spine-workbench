//! Made-up spines as a dataset, scored against their answer keys.

use super::{DatasetError, ScoringFilm};
use crate::metrics::{FilmScore, Prediction, Quad, score_film_with_visibility};
use serde::{Deserialize, Serialize};
use shapes::errors::EngineError;
use shapes::film::FilmKind;
use shapes::made_up::{FilmAnswer, MadeUpRecipe, TrueMark, TrueMarks};
use shapes::vertebra::FilmMarks;

/// Geometric scores on a generated film, with visible corner errors also in mm.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MadeUpFilmScore {
    pub film: FilmKind,
    pub score: FilmScore,
    pub corner_mm: Vec<f64>,
}

/// The four true corners in storage order, retaining visibility for each.
/// Patient-side names are canonicalised by the metric, not by this adapter.
pub fn true_corners(marks: &TrueMarks) -> [TrueMark; 4] {
    match *marks {
        TrueMarks::Front {
            sup_l,
            sup_r,
            inf_l,
            inf_r,
            ..
        } => [sup_l, sup_r, inf_l, inf_r],
        TrueMarks::Side {
            sup_a,
            sup_p,
            inf_a,
            inf_p,
        } => [sup_a, sup_p, inf_a, inf_p],
        TrueMarks::Bending { sup, inf } => [sup[0], sup[1], inf[0], inf[1]],
    }
}

fn quad(points: [TrueMark; 4]) -> Quad {
    let [tl, tr, bl, br] = points.map(|p| [p.at.x(), p.at.y()]);
    Quad { tl, tr, bl, br }
}

/// Compare returned marks with an existing generated film's answer key.
/// Anatomical curve angles in the key are not the three-column AASCE
/// benchmark, so this adapter leaves that benchmark's Cobb fields absent.
pub fn score_marks(
    answer: &FilmAnswer,
    image_height: u32,
    marks: &FilmMarks,
) -> Result<MadeUpFilmScore, DatasetError> {
    if marks.film != answer.kind || marks.vertebrae.iter().any(|v| !v.marks.suits(marks.film)) {
        return Err(DatasetError::FilmKind);
    }
    if image_height == 0 {
        return Err(DatasetError::Dimensions);
    }
    let corners: Vec<_> = answer
        .vertebrae
        .iter()
        .map(|v| true_corners(&v.marks))
        .collect();
    let truth: Vec<_> = answer
        .vertebrae
        .iter()
        .zip(&corners)
        .map(|(v, p)| (v.level, quad(*p), true))
        .collect();
    let visible: Vec<_> = corners.iter().map(|p| p.map(|mark| mark.visible)).collect();
    let prediction = Prediction::from(marks);
    let score =
        score_film_with_visibility(&truth, &visible, None, &prediction, f64::from(image_height))?;
    let mut corner_mm = Vec::new();
    if let Some(sx) = answer.geometry.mm_per_px {
        let sy = answer.geometry.mm_per_px_vertical.unwrap_or(sx);
        if !sx.is_finite() || !sy.is_finite() || sx <= 0.0 || sy <= 0.0 {
            return Err(DatasetError::Scale);
        }
        for ((level, _, _), raw) in truth.iter().zip(&corners) {
            let Some(predicted) = prediction.vertebrae.iter().find(|v| v.level == *level) else {
                continue;
            };
            let mut points = *raw;
            if points[0].at.x() > points[1].at.x() {
                points.swap(0, 1);
            }
            if points[2].at.x() > points[3].at.x() {
                points.swap(2, 3);
            }
            let p = predicted.quad.x_canonical();
            for (target, placed) in points.into_iter().zip([p.tl, p.tr, p.bl, p.br]) {
                if target.visible {
                    corner_mm.push(libm::hypot(
                        (placed[0] - target.at.x()) * sx,
                        (placed[1] - target.at.y()) * sy,
                    ));
                }
            }
        }
    }
    Ok(MadeUpFilmScore {
        film: answer.kind,
        score,
        corner_mm,
    })
}

/// Generate one case, then run the supplied finder on each of its four films.
/// Only pixels, their size and geometry cross into the finder; truth stays here.
pub fn score(
    recipe: &MadeUpRecipe,
    mut finder: impl FnMut(&ScoringFilm<'_>) -> Result<FilmMarks, EngineError>,
) -> Result<Vec<MadeUpFilmScore>, DatasetError> {
    let case = made_up_spines::make_up_case(recipe).map_err(|_| DatasetError::MadeUp)?;
    case.films
        .iter()
        .zip(&case.answer_key.films)
        .map(|(painted, answer)| {
            let bytes = made_up_spines::film_png::encode(painted);
            let input = ScoringFilm {
                kind: painted.kind,
                bytes: &bytes,
                width: painted.image.width(),
                height: painted.image.height(),
                geometry: answer.geometry.clone(),
            };
            let marks = finder(&input).map_err(|e| DatasetError::Finder(e.code))?;
            score_marks(answer, input.height, &marks)
        })
        .collect()
}
