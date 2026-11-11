//! One film's metrics, with absent measurements represented explicitly.

use super::{
    Prediction, Quad, cobb_from_corners,
    corner_error::{self, CornerLevelScore},
    level_naming, vertebrae_found,
};
use serde::{Deserialize, Serialize};
use shapes::levels::Level;

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ScoreError {
    #[error(
        "scoring requires at most thirty finite, uniquely named vertebrae and a positive film height"
    )]
    InvalidFilm,
    #[error("corner visibility must contain one four-corner mask per true vertebra")]
    InvalidVisibility,
    #[error("assignment costs must form a finite rectangular matrix of at most thirty by thirty")]
    InvalidAssignment,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScoreFlag {
    CobbInterpolated,
    InsufficientForCobb,
    DegenerateQuad,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FilmScore {
    pub found: usize,
    pub eligible: usize,
    pub false_pos: usize,
    pub unmatched_predictions: usize,
    pub corner_px: Vec<f64>,
    pub corner_norm: Vec<f64>,
    pub per_level: Vec<CornerLevelScore>,
    pub naming_ok: usize,
    pub chain_shift: Option<f64>,
    pub cobb_pred: Option<[f64; 3]>,
    pub cobb_true: Option<[f64; 3]>,
    /// Percentage (0..100), unavailable when there is no reference Cobb.
    pub smape_term: Option<f64>,
    pub flags: Vec<ScoreFlag>,
}

pub fn score_film(
    truth: &[(Level, Quad, bool)],
    cobb_truth: Option<[f64; 3]>,
    prediction: &Prediction,
    image_height: f64,
) -> Result<FilmScore, ScoreError> {
    score_film_with_visibility(
        truth,
        &vec![[true; 4]; truth.len()],
        cobb_truth,
        prediction,
        image_height,
    )
}

/// Visibility masks are in the supplied truth quad's storage order. A
/// canonical left/right swap carries its visibility with it. Visibility
/// changes only corner errors, while eligibility changes all primary
/// geometric and naming measures.
pub fn score_film_with_visibility(
    truth: &[(Level, Quad, bool)],
    visible: &[[bool; 4]],
    cobb_truth: Option<[f64; 3]>,
    prediction: &Prediction,
    image_height: f64,
) -> Result<FilmScore, ScoreError> {
    if visible.len() != truth.len() {
        return Err(ScoreError::InvalidVisibility);
    }
    let mut truth_levels: Vec<_> = truth.iter().map(|(l, _, _)| *l).collect();
    let mut predicted_levels: Vec<_> = prediction.vertebrae.iter().map(|v| v.level).collect();
    truth_levels.sort_unstable();
    predicted_levels.sort_unstable();
    if truth.len() > 30
        || prediction.vertebrae.len() > 30
        || !image_height.is_finite()
        || image_height <= 0.0
        || truth.iter().any(|(_, q, _)| !q.is_finite())
        || prediction
            .vertebrae
            .iter()
            .any(|v| !v.quad.is_finite() || !v.confidence.is_finite())
        || truth_levels.windows(2).any(|p| p[0] == p[1])
        || predicted_levels.windows(2).any(|p| p[0] == p[1])
        || cobb_truth.is_some_and(|v| {
            v.into_iter()
                .any(|a| !a.is_finite() || !(0.0..=180.0).contains(&a))
        })
    {
        return Err(ScoreError::InvalidFilm);
    }
    let mut per_level = Vec::new();
    for (i, &(level, q, eligible)) in truth.iter().enumerate() {
        if !eligible {
            continue;
        }
        let Some(p) = prediction.vertebrae.iter().find(|p| p.level == level) else {
            continue;
        };
        let mut mask = visible[i];
        if q.tl[0] > q.tr[0] {
            mask.swap(0, 1);
        }
        if q.bl[0] > q.br[0] {
            mask.swap(2, 3);
        }
        let pixels: Vec<_> = corner_error::distances(&q, &p.quad)
            .into_iter()
            .zip(mask)
            .filter_map(|(error, seen)| seen.then_some(error))
            .collect();
        let height = q.height();
        let normalised = if height > 0.0 {
            pixels.iter().map(|v| v / height).collect()
        } else {
            Vec::new()
        };
        per_level.push(CornerLevelScore {
            level,
            pixels,
            normalised,
        });
    }
    let predicted_quads: Vec<_> = prediction.vertebrae.iter().map(|v| v.quad).collect();
    let eligible_indices: Vec<_> = truth
        .iter()
        .enumerate()
        .filter_map(|(i, (_, _, eligible))| eligible.then_some(i))
        .collect();
    let eligible_quads: Vec<_> = eligible_indices.iter().map(|&i| truth[i].1).collect();
    // Excluded corrupt labels cannot take a prediction away from eligible
    // truth. Only after the primary assignment do we ignore predictions
    // that match an excluded label instead of counting them as invented.
    let matches: Vec<_> = vertebrae_found::match_quads(&eligible_quads, &predicted_quads)
        .into_iter()
        .map(|(i, j)| (eligible_indices[i], j))
        .collect();
    let remaining_indices: Vec<_> = (0..predicted_quads.len())
        .filter(|j| !matches.iter().any(|&(_, k)| k == *j))
        .collect();
    let remaining_quads: Vec<_> = remaining_indices
        .iter()
        .map(|&j| predicted_quads[j])
        .collect();
    let excluded_quads: Vec<_> = truth
        .iter()
        .filter_map(|(_, q, eligible)| (!eligible).then_some(*q))
        .collect();
    let ignored = vertebrae_found::match_quads(&excluded_quads, &remaining_quads);
    let unmatched: Vec<_> = remaining_indices
        .iter()
        .enumerate()
        .filter(|(j, _)| !ignored.iter().any(|&(_, k)| k == *j))
        .map(|(_, &j)| (j, &predicted_quads[j]))
        .collect();
    let false_pos = unmatched
        .iter()
        .filter(|(_, q)| vertebrae_found::in_spine_strip(q.centre(), &eligible_quads))
        .count();
    let true_levels: Vec<_> = truth.iter().map(|(l, _, _)| *l).collect();
    let predicted_levels: Vec<_> = prediction.vertebrae.iter().map(|v| v.level).collect();
    let naming = level_naming::score(&true_levels, &predicted_levels, &matches);
    let mut flags = Vec::new();
    let mut insufficient = false;
    let cobb_pred = cobb_truth.map(|_| {
        let Some((quads, interpolated)) = cobb_from_corners::fill_missing(prediction) else {
            flags.extend([ScoreFlag::CobbInterpolated, ScoreFlag::InsufficientForCobb]);
            insufficient = true;
            return [0.0; 3];
        };
        if interpolated {
            flags.push(ScoreFlag::CobbInterpolated);
        }
        if quads.iter().any(|q| {
            q.tr[0] + q.br[0] == q.tl[0] + q.bl[0] && q.tr[1] + q.br[1] == q.tl[1] + q.bl[1]
        }) {
            flags.push(ScoreFlag::DegenerateQuad);
        }
        let points = std::array::from_fn(|i| quads[i / 4].points()[i % 4]);
        cobb_from_corners::aasce_cobb(&points, image_height).0
    });
    let smape_term = cobb_pred.zip(cobb_truth).map(|(p, t)| {
        if insufficient {
            100.0
        } else {
            cobb_from_corners::smape_term(p, t)
        }
    });
    Ok(FilmScore {
        found: matches.len(),
        eligible: eligible_quads.len(),
        false_pos,
        unmatched_predictions: unmatched.len(),
        corner_px: per_level
            .iter()
            .flat_map(|s| s.pixels.iter().copied())
            .collect(),
        corner_norm: per_level
            .iter()
            .flat_map(|s| s.normalised.iter().copied())
            .collect(),
        per_level,
        naming_ok: naming.correct,
        chain_shift: naming.chain_shift,
        cobb_pred,
        cobb_true: cobb_truth,
        smape_term,
        flags,
    })
}
