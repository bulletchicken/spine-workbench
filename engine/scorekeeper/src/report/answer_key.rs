//! Live comparison with a made-up spine's answer key. Matching is by
//! position, one to one, so a wrong level name cannot hide a found bone.

use crate::datasets::made_up::true_corners;
use crate::metrics::Quad;
use crate::metrics::vertebrae_found::match_quads;
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::FilmKind;
use shapes::made_up::{
    AnswerKey, AnswerKeyComparison, FilmAnswer, FilmComparison, TrueMark, TrueMarks,
    VertebraComparison,
};
use shapes::vertebra::FilmMarks;

/// Compare every answer-key film with its placed marks. Missing films and
/// bones remain in the answer, with no corner error rather than a zero.
///
/// Found bones use convex-hull IoU of at least 0.5 and one-to-one matching.
/// Corner pairs are x-canonical, with visibility travelling with the true
/// point. Film means weight each visible corner equally. Physical errors
/// use both stored pixel spacings, when available.
pub fn compare_with_answer_key(
    marks: &[FilmMarks],
    key: &AnswerKey,
) -> Result<AnswerKeyComparison, EngineError> {
    validate(marks, key)?;
    let films = key
        .films
        .iter()
        .map(|answer| compare_film(answer, marks.iter().find(|film| film.film == answer.kind)))
        .collect::<Result<_, _>>()?;
    Ok(AnswerKeyComparison { films })
}

fn compare_film(
    answer: &FilmAnswer,
    marks: Option<&FilmMarks>,
) -> Result<FilmComparison, EngineError> {
    let placed = marks.map_or(&[][..], |film| film.vertebrae.as_slice());
    let true_points: Vec<_> = answer
        .vertebrae
        .iter()
        .map(|vertebra| canonical_true_corners(vertebra.marks))
        .collect();
    let truth: Vec<_> = true_points.iter().map(quad).collect();
    let predicted: Vec<_> = placed
        .iter()
        .map(|vertebra| Quad::from_marks(&vertebra.marks).x_canonical())
        .collect();
    let pairs = match_quads(&truth, &predicted);
    let mut all_px = Vec::new();
    let mut all_mm = Vec::new();
    let mut vertebrae = Vec::with_capacity(truth.len());
    for (index, true_vertebra) in answer.vertebrae.iter().enumerate() {
        let prediction = pairs
            .iter()
            .find_map(|&(actual, prediction)| (actual == index).then_some(prediction));
        let mut errors = Vec::new();
        if let Some(prediction) = prediction {
            for (actual, found) in true_points[index]
                .iter()
                .zip(predicted[prediction].points())
            {
                if actual.visible {
                    let dx = found[0] - actual.at.x();
                    let dy = found[1] - actual.at.y();
                    let error = libm::hypot(dx, dy);
                    if !error.is_finite() {
                        return Err(invalid("corner distances must be finite"));
                    }
                    errors.push(error);
                    all_px.push(error);
                    if let Some(horizontal) = answer.geometry.mm_per_px {
                        let vertical = answer.geometry.mm_per_px_vertical.unwrap_or(horizontal);
                        let mm = libm::hypot(dx * horizontal, dy * vertical);
                        if !mm.is_finite() {
                            return Err(invalid("physical corner distances must be finite"));
                        }
                        all_mm.push(mm);
                    }
                }
            }
        }
        vertebrae.push(VertebraComparison {
            level: true_vertebra.level,
            vertebra_id: prediction.map(|index| placed[index].id),
            level_right: prediction.is_some_and(|index| placed[index].level == true_vertebra.level),
            mean_corner_error_px: mean(&errors),
            max_corner_error_px: errors.into_iter().reduce(f64::max),
        });
    }
    Ok(FilmComparison {
        film: answer.kind,
        expected: answer.vertebrae.len() as u32,
        found: pairs.len() as u32,
        levels_right: vertebrae
            .iter()
            .filter(|vertebra| vertebra.level_right)
            .count() as u32,
        mean_corner_error_px: mean(&all_px),
        mean_corner_error_mm: mean(&all_mm),
        vertebrae,
    })
}

fn mean(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().map(|value| value / values.len() as f64).sum())
}

fn canonical_true_corners(marks: TrueMarks) -> [TrueMark; 4] {
    let mut points = true_corners(&marks);
    if points[0].at.x() > points[1].at.x() {
        points.swap(0, 1);
    }
    if points[2].at.x() > points[3].at.x() {
        points.swap(2, 3);
    }
    points
}

fn quad(points: &[TrueMark; 4]) -> Quad {
    let [tl, tr, bl, br] = points.map(|point| point.at.0);
    Quad { tl, tr, bl, br }
}

fn validate(marks: &[FilmMarks], key: &AnswerKey) -> Result<(), EngineError> {
    for (index, answer) in key.films.iter().enumerate() {
        if key.films[..index]
            .iter()
            .any(|film| film.kind == answer.kind)
        {
            return Err(invalid("answer-key films must not repeat"));
        }
        if answer.vertebrae.len() > 30 {
            return Err(invalid("a film may hold at most 30 vertebrae"));
        }
        for (index, vertebra) in answer.vertebrae.iter().enumerate() {
            if answer.vertebrae[..index]
                .iter()
                .any(|prior| prior.level == vertebra.level)
                || !truth_suits(vertebra.marks, answer.kind)
                || !quad(&canonical_true_corners(vertebra.marks)).is_finite()
            {
                return Err(invalid(
                    "answer-key corners must be finite, have unique levels and suit the film",
                ));
            }
        }
        let geometry = &answer.geometry;
        if geometry
            .mm_per_px
            .into_iter()
            .chain(geometry.mm_per_px_vertical)
            .any(|scale| !scale.is_finite() || scale <= 0.0)
            || (geometry.mm_per_px.is_none() && geometry.mm_per_px_vertical.is_some())
        {
            return Err(invalid(
                "pixel spacings must be finite and positive, with a horizontal spacing",
            ));
        }
    }
    for (index, film) in marks.iter().enumerate() {
        if marks[..index].iter().any(|prior| prior.film == film.film)
            || !key.films.iter().any(|answer| answer.kind == film.film)
        {
            return Err(invalid(
                "placed films must be unique and present in the answer key",
            ));
        }
        if film.vertebrae.len() > 30 {
            return Err(invalid("a film may hold at most 30 vertebrae"));
        }
        for (index, vertebra) in film.vertebrae.iter().enumerate() {
            if !vertebra.marks.suits(film.film)
                || !Quad::from_marks(&vertebra.marks).is_finite()
                || film.vertebrae[..index]
                    .iter()
                    .any(|prior| prior.id == vertebra.id)
            {
                return Err(invalid(
                    "placed corners must be finite, have unique ids and suit the film",
                ));
            }
        }
    }
    Ok(())
}

fn truth_suits(marks: TrueMarks, film: FilmKind) -> bool {
    matches!(
        (marks, film),
        (TrueMarks::Front { .. }, FilmKind::Front)
            | (TrueMarks::Side { .. }, FilmKind::Side)
            | (
                TrueMarks::Bending { .. },
                FilmKind::BendLeft | FilmKind::BendRight
            )
    )
}

fn invalid(message: &'static str) -> EngineError {
    EngineError::new(ErrorCode::CaseInvalidValue, message)
}
