//! The measures every dataset reports. Coordinates stay in original film
//! pixels; malformed inputs fail before a partial score can be produced.

pub mod clicks_to_good_outline;
pub mod cobb_from_corners;
pub mod corner_error;
pub mod level_naming;
mod quad;
mod score;
pub mod vertebrae_found;

pub use quad::Quad;
pub use score::{FilmScore, ScoreError, ScoreFlag, score_film, score_film_with_visibility};
use serde::{Deserialize, Serialize};
use shapes::{levels::Level, vertebra::FilmMarks};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PredictedVertebra {
    pub level: Level,
    pub quad: Quad,
    pub confidence: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Prediction {
    pub vertebrae: Vec<PredictedVertebra>,
}

impl From<&FilmMarks> for Prediction {
    fn from(marks: &FilmMarks) -> Self {
        Self {
            vertebrae: marks
                .vertebrae
                .iter()
                .map(|v| PredictedVertebra {
                    level: v.level,
                    quad: Quad::from_marks(&v.marks).x_canonical(),
                    confidence: v.confidence.score,
                })
                .collect(),
        }
    }
}
