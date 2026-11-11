//! AASCE 2019: `landmarks.csv` (136 numbers per row, 68 x then 68 y, each 0 to
//! 1), `angles.csv` and `filenames.csv`, with the per-vertebra label flags.

use super::{DatasetError, ScoringFilm};
use crate::metrics::{FilmScore, Prediction, Quad, score_film};
use serde::{Deserialize, Serialize};
use shapes::errors::EngineError;
use shapes::film::FilmKind;
use shapes::film_geometry::FilmGeometry;
use shapes::levels::Level;
use shapes::vertebra::FilmMarks;
use std::collections::BTreeSet;

/// Original tables, kept only while the host resolves their image files.
/// Deliberately neither printable nor serialisable: filenames may contain
/// identifying text. Reports use one-based row numbers instead.
pub struct AasceLabels {
    filenames: Vec<String>,
    landmarks: Vec<[f64; 136]>,
    angles: Vec<[f64; 3]>,
}

/// A problem in an individual labelled vertebra.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LabelFlag {
    LevelOrder,
    TopBelowBottom,
    LrSwapped,
    OutsideImage,
    DegenerateQuad,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelFlags {
    pub per_vertebra: [Vec<LabelFlag>; 17],
}

impl LabelFlags {
    pub fn is_flagged(&self) -> bool {
        self.per_vertebra.iter().any(|flags| !flags.is_empty())
    }

    /// Unique film-level flags in a stable order.
    pub fn flags(&self) -> Vec<LabelFlag> {
        self.per_vertebra
            .iter()
            .flatten()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

/// Labels scaled into original image pixels, retaining their published order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AasceFilm {
    /// One-based position in the split, never the source filename.
    pub row: usize,
    pub width: u32,
    pub height: u32,
    pub truth: [Quad; 17],
    pub cobb: [f64; 3],
    pub flags: LabelFlags,
}

impl AasceLabels {
    pub fn len(&self) -> usize {
        self.filenames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.filenames.is_empty()
    }

    /// For local file lookup only. Do not include this value in reports or errors.
    pub fn filename(&self, index: usize) -> Option<&str> {
        self.filenames.get(index).map(String::as_str)
    }

    /// Scale fractions by the actual image size, without a half-pixel offset.
    pub fn film(&self, index: usize, width: u32, height: u32) -> Result<AasceFilm, DatasetError> {
        if width == 0 || height == 0 {
            return Err(DatasetError::Dimensions);
        }
        let values = self
            .landmarks
            .get(index)
            .ok_or(DatasetError::RowUnavailable)?;
        let point = |k: usize| {
            [
                values[k] * f64::from(width),
                values[68 + k] * f64::from(height),
            ]
        };
        let truth = std::array::from_fn(|j| Quad {
            tl: point(4 * j),
            tr: point(4 * j + 1),
            bl: point(4 * j + 2),
            br: point(4 * j + 3),
        });
        if truth
            .iter()
            .flat_map(|q| [q.tl, q.tr, q.bl, q.br])
            .flatten()
            .any(|v| !v.is_finite())
        {
            return Err(DatasetError::Number {
                table: "landmarks",
                row: index + 1,
                column: 0,
            });
        }
        Ok(AasceFilm {
            row: index + 1,
            width,
            height,
            flags: label_flags(&truth, width, height),
            truth,
            cobb: self.angles[index],
        })
    }
}

impl AasceFilm {
    pub fn truth_for_scoring(&self) -> Vec<(Level, Quad, bool)> {
        Level::MEASURED
            .into_iter()
            .enumerate()
            .map(|(j, level)| (level, self.truth[j], self.flags.per_vertebra[j].is_empty()))
            .collect()
    }

    /// Run any film-to-marks function without exposing the labels to it.
    pub fn score_with_finder(
        &self,
        bytes: &[u8],
        mut finder: impl FnMut(&ScoringFilm<'_>) -> Result<FilmMarks, EngineError>,
    ) -> Result<FilmScore, DatasetError> {
        let input = ScoringFilm {
            kind: FilmKind::Front,
            bytes,
            width: self.width,
            height: self.height,
            geometry: FilmGeometry::UNKNOWN,
        };
        let marks = finder(&input).map_err(|e| DatasetError::Finder(e.code))?;
        if marks.film != FilmKind::Front
            || marks.vertebrae.iter().any(|v| !v.marks.suits(marks.film))
        {
            return Err(DatasetError::FilmKind);
        }
        Ok(score_film(
            &self.truth_for_scoring(),
            Some(self.cobb),
            &Prediction::from(&marks),
            f64::from(self.height),
        )?)
    }
}

/// Read all three headerless tables together; a partial dataset is an error.
pub fn parse_labels(
    filenames: &[u8],
    landmarks: &[u8],
    angles: &[u8],
) -> Result<AasceLabels, DatasetError> {
    let names = records(filenames, "filenames", 1)?;
    let landmarks = numbers::<136>(landmarks, "landmarks", false)?;
    let angles = numbers::<3>(angles, "angles", true)?;
    if names.len() != landmarks.len() || names.len() != angles.len() {
        return Err(DatasetError::RowCount);
    }
    if names.is_empty() {
        return Err(DatasetError::Empty);
    }
    let mut filenames = Vec::with_capacity(names.len());
    let mut seen = BTreeSet::new();
    for (index, record) in names.into_iter().enumerate() {
        let name = &record[0];
        if name.is_empty()
            || name == "."
            || name == ".."
            || name
                .chars()
                .any(|c| c.is_control() || matches!(c, '/' | '\\' | ':'))
        {
            return Err(DatasetError::FileName { row: index + 1 });
        }
        if !seen.insert(name.to_owned()) {
            return Err(DatasetError::DuplicateFileName { row: index + 1 });
        }
        filenames.push(name.to_owned());
    }
    Ok(AasceLabels {
        filenames,
        landmarks,
        angles,
    })
}

fn records(
    bytes: &[u8],
    table: &'static str,
    columns: usize,
) -> Result<Vec<csv::StringRecord>, DatasetError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(bytes);
    reader
        .records()
        .enumerate()
        .map(|(index, record)| {
            let row = index + 1;
            let record = record.map_err(|_| DatasetError::Csv { table, row })?;
            if record.len() != columns {
                return Err(DatasetError::Columns {
                    table,
                    row,
                    expected: columns,
                });
            }
            Ok(record)
        })
        .collect()
}

fn numbers<const N: usize>(
    bytes: &[u8],
    table: &'static str,
    angles: bool,
) -> Result<Vec<[f64; N]>, DatasetError> {
    records(bytes, table, N)?
        .into_iter()
        .enumerate()
        .map(|(index, record)| {
            let mut row = [0.0; N];
            for (column, field) in record.iter().enumerate() {
                let error = || DatasetError::Number {
                    table,
                    row: index + 1,
                    column: column + 1,
                };
                let value: f64 = field.trim().parse().map_err(|_| error())?;
                if !value.is_finite() || (angles && !(0.0..=180.0).contains(&value)) {
                    return Err(error());
                }
                row[column] = value;
            }
            Ok(row)
        })
        .collect()
}

/// Detect label defects before canonicalising corners for geometric metrics.
pub fn label_flags(quads: &[Quad; 17], width: u32, height: u32) -> LabelFlags {
    let mut per_vertebra: [Vec<LabelFlag>; 17] = std::array::from_fn(|_| Vec::new());
    let centre_y = quads.map(|q| (q.tl[1] + q.tr[1] + q.bl[1] + q.br[1]) / 4.0);
    let bad_pairs: Vec<_> = centre_y
        .windows(2)
        .enumerate()
        .filter_map(|(j, p)| (p[0] >= p[1]).then_some(j))
        .collect();
    if !bad_pairs.is_empty() {
        let repairs: Vec<_> = (0..17)
            .filter(|&removed| {
                let remaining: Vec<_> = centre_y
                    .iter()
                    .enumerate()
                    .filter_map(|(j, y)| (j != removed).then_some(y))
                    .collect();
                remaining.windows(2).all(|p| p[0] < p[1])
            })
            .collect();
        if repairs.len() == 1 {
            per_vertebra[repairs[0]].push(LabelFlag::LevelOrder);
        } else {
            for j in bad_pairs {
                for flags in &mut per_vertebra[j..=j + 1] {
                    if !flags.contains(&LabelFlag::LevelOrder) {
                        flags.push(LabelFlag::LevelOrder);
                    }
                }
            }
        }
    }
    for (q, flags) in quads.iter().zip(&mut per_vertebra) {
        if q.tl[1] + q.tr[1] > q.bl[1] + q.br[1] {
            flags.push(LabelFlag::TopBelowBottom);
        }
        if q.tl[0] > q.tr[0] || q.bl[0] > q.br[0] {
            flags.push(LabelFlag::LrSwapped);
        }
        if [q.tl, q.tr, q.bl, q.br].iter().any(|p| {
            p[0] < 0.0 || p[0] > f64::from(width) || p[1] < 0.0 || p[1] > f64::from(height)
        }) {
            flags.push(LabelFlag::OutsideImage);
        }
        if q.tr[0] + q.br[0] == q.tl[0] + q.bl[0] && q.tr[1] + q.br[1] == q.tl[1] + q.bl[1] {
            flags.push(LabelFlag::DegenerateQuad);
        }
    }
    LabelFlags { per_vertebra }
}
