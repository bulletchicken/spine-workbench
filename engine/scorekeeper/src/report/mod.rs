//! Reports: what was run, on what, and the numbers.

pub mod answer_key;
pub mod json;
pub mod markdown;
pub mod overlay_svg;

use crate::datasets::aasce_2019::LabelFlag;
use crate::metrics::FilmScore;
use serde::Serialize;
use shapes::film::FilmKind;
use shapes::levels::Level;
use std::collections::BTreeMap;

/// Provenance supplied by the host, without paths or original film names.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Provenance {
    pub engine_version: String,
    pub git_revision: Option<String>,
    pub run_started_unix_seconds: Option<u64>,
    pub dataset: String,
    pub split: Option<String>,
    pub mode: String,
    pub listing_sha256: String,
    pub label_sha256: BTreeMap<String, String>,
    pub parameters_sha256: String,
}

/// A row number identifies a film only within its dataset and split.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilmReport {
    pub row: usize,
    pub film: FilmKind,
    pub width: u32,
    pub height: u32,
    pub label_flags: Vec<LabelFlag>,
    pub excluded_levels: Vec<Level>,
    pub score: FilmScore,
    pub corner_mm: Option<Vec<f64>>,
}

/// Empty samples have absent statistics, never a misleading zero error.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Distribution {
    pub count: usize,
    pub mean: Option<f64>,
    pub median: Option<f64>,
    pub p90: Option<f64>,
}

impl Distribution {
    /// P90 uses nearest rank, ceil(0.9 n); median averages the middle pair.
    pub fn from_values(values: &[f64]) -> Self {
        let mut sorted = values.to_vec();
        sorted.sort_by(f64::total_cmp);
        let count = sorted.len();
        Self {
            count,
            mean: average(&sorted),
            median: (count > 0).then(|| (sorted[(count - 1) / 2] + sorted[count / 2]) / 2.0),
            p90: (count > 0).then(|| sorted[(9 * count).div_ceil(10) - 1]),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CobbSummary {
    pub films: usize,
    pub smape_percent: Option<f64>,
    pub mae_degrees: [Option<f64>; 3],
    pub within_5_degrees: Option<f64>,
    pub within_10_degrees: Option<f64>,
    pub maximum_absolute_error: Option<f64>,
    pub median_film_maximum_error: Option<f64>,
    pub within_001_degrees: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub films: usize,
    pub flagged_films: usize,
    pub eligible: usize,
    pub found: usize,
    pub missed: usize,
    pub false_positives: usize,
    pub found_fraction: Option<f64>,
    pub levels_right: usize,
    pub naming_fraction: Option<f64>,
    pub corner_px: Distribution,
    pub corner_normalised: Distribution,
    pub corner_mm: Distribution,
    pub per_level: Vec<LevelSummary>,
    pub under_010_height: Option<f64>,
    pub under_025_height: Option<f64>,
    pub cobb_all: CobbSummary,
    pub cobb_unflagged: CobbSummary,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelSummary {
    pub level: Level,
    pub corner_px: Distribution,
    pub corner_normalised: Distribution,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub provenance: Provenance,
    pub summary: Summary,
    pub films: Vec<FilmReport>,
    pub caveats: Vec<String>,
}

impl Report {
    pub fn new(provenance: Provenance, films: Vec<FilmReport>, caveats: Vec<String>) -> Self {
        let eligible = films.iter().map(|f| f.score.eligible).sum();
        let found = films.iter().map(|f| f.score.found).sum();
        let levels_right = films.iter().map(|f| f.score.naming_ok).sum();
        let pixels: Vec<f64> = films
            .iter()
            .flat_map(|f| f.score.corner_px.iter().copied())
            .collect();
        let normalised: Vec<f64> = films
            .iter()
            .flat_map(|f| f.score.corner_norm.iter().copied())
            .collect();
        let mm: Vec<f64> = films
            .iter()
            .filter_map(|f| f.corner_mm.as_ref())
            .flatten()
            .copied()
            .collect();
        let mut levels: BTreeMap<Level, (Vec<f64>, Vec<f64>)> = BTreeMap::new();
        for level in films.iter().flat_map(|f| &f.score.per_level) {
            let samples = levels.entry(level.level).or_default();
            samples.0.extend_from_slice(&level.pixels);
            samples.1.extend_from_slice(&level.normalised);
        }
        let summary = Summary {
            films: films.len(),
            flagged_films: films.iter().filter(|f| !f.label_flags.is_empty()).count(),
            eligible,
            found,
            missed: eligible - found,
            false_positives: films.iter().map(|f| f.score.false_pos).sum(),
            found_fraction: fraction(found, eligible),
            levels_right,
            naming_fraction: fraction(levels_right, found),
            corner_px: Distribution::from_values(&pixels),
            corner_normalised: Distribution::from_values(&normalised),
            corner_mm: Distribution::from_values(&mm),
            per_level: levels
                .into_iter()
                .map(|(level, (px, normalised))| LevelSummary {
                    level,
                    corner_px: Distribution::from_values(&px),
                    corner_normalised: Distribution::from_values(&normalised),
                })
                .collect(),
            under_010_height: fraction(
                normalised.iter().filter(|&&x| x < 0.10).count(),
                normalised.len(),
            ),
            under_025_height: fraction(
                normalised.iter().filter(|&&x| x < 0.25).count(),
                normalised.len(),
            ),
            cobb_all: cobb_summary(&films, false),
            cobb_unflagged: cobb_summary(&films, true),
        };
        Self {
            provenance,
            summary,
            films,
            caveats,
        }
    }
}

fn average(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

fn fraction(numerator: usize, denominator: usize) -> Option<f64> {
    (denominator != 0).then(|| numerator as f64 / denominator as f64)
}

fn cobb_summary(films: &[FilmReport], unflagged_only: bool) -> CobbSummary {
    let selected: Vec<_> = films
        .iter()
        .filter(|f| !unflagged_only || f.label_flags.is_empty())
        .filter_map(|f| Some((f.score.cobb_pred?, f.score.cobb_true?, f.score.smape_term?)))
        .collect();
    let errors: Vec<[f64; 3]> = selected
        .iter()
        .map(|(pred, truth, _)| std::array::from_fn(|i| (pred[i] - truth[i]).abs()))
        .collect();
    let maxima: Vec<f64> = errors
        .iter()
        .map(|e| e.iter().copied().fold(0.0, f64::max))
        .collect();
    CobbSummary {
        films: selected.len(),
        smape_percent: average(&selected.iter().map(|x| x.2).collect::<Vec<_>>()),
        mae_degrees: std::array::from_fn(|i| {
            average(&errors.iter().map(|e| e[i]).collect::<Vec<_>>())
        }),
        within_5_degrees: fraction(maxima.iter().filter(|&&x| x <= 5.0).count(), maxima.len()),
        within_10_degrees: fraction(maxima.iter().filter(|&&x| x <= 10.0).count(), maxima.len()),
        maximum_absolute_error: maxima.iter().copied().reduce(f64::max),
        median_film_maximum_error: Distribution::from_values(&maxima).median,
        within_001_degrees: fraction(maxima.iter().filter(|&&x| x <= 0.01).count(), maxima.len()),
    }
}
