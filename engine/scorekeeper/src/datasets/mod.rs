//! Labelled datasets, parsed from bytes. Nothing here touches the disk; the
//! command line reads the files.

pub mod aasce_2019;
pub mod hand_marked;
pub mod made_up;
pub mod vertebrae_keypoints_50;

use crate::metrics::ScoreError;
use shapes::errors::ErrorCode;
use shapes::film::FilmKind;
use shapes::film_geometry::FilmGeometry;

/// Encoded pixels and their geometry, with no answer key exposed to a finder.
/// The finder decodes the bytes through the film reader when it is available.
pub struct ScoringFilm<'a> {
    pub kind: FilmKind,
    pub bytes: &'a [u8],
    pub width: u32,
    pub height: u32,
    pub geometry: FilmGeometry,
}

/// Dataset failures name table positions, never source filenames or field values.
#[derive(Debug, thiserror::Error)]
pub enum DatasetError {
    #[error("invalid {table} CSV at row {row}")]
    Csv { table: &'static str, row: usize },
    #[error("{table} row {row} must have {expected} columns")]
    Columns {
        table: &'static str,
        row: usize,
        expected: usize,
    },
    #[error("invalid number in {table} row {row}, column {column}")]
    Number {
        table: &'static str,
        row: usize,
        column: usize,
    },
    #[error("the filename, landmark and angle tables have different row counts")]
    RowCount,
    #[error("the dataset has no rows")]
    Empty,
    #[error("unsafe image filename at row {row}")]
    FileName { row: usize },
    #[error("repeated image filename at row {row}")]
    DuplicateFileName { row: usize },
    #[error("film dimensions must be positive")]
    Dimensions,
    #[error("film spacing must be finite and positive")]
    Scale,
    #[error("dataset row is out of range")]
    RowUnavailable,
    #[error("finder failed with {0:?}")]
    Finder(ErrorCode),
    #[error("finder returned marks for another film")]
    FilmKind,
    #[error("made-up spine could not be generated")]
    MadeUp,
    #[error(transparent)]
    Score(#[from] ScoreError),
}
