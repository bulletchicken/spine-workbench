//! Reading a case file.

use crate::{film_png, layout, strict, versions};
use shapes::case_file_v1::OpenedCase;
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::{FilmKind, FilmPng};

/// Nothing is returned until every entry, the record and all PNGs pass.
pub fn unpack_case(bytes: &[u8]) -> Result<OpenedCase, EngineError> {
    let mut archive = strict::archive(bytes)?;
    for required in [layout::CASE_JSON, layout::README] {
        if archive.index_for_name(required).is_none() {
            return Err(strict::invalid(required));
        }
    }
    let json = strict::read_entry(&mut archive, layout::CASE_JSON)?;
    let text = std::str::from_utf8(&json).map_err(|_| strict::invalid(layout::CASE_JSON))?;
    let case = versions::read(text)?;
    // Consume the note too, so damaged compressed data or CRCs cannot hide.
    strict::read_entry(&mut archive, layout::README)?;
    for kind in FilmKind::ALL {
        if archive.index_for_name(kind.png_path()).is_some()
            && !case.films.iter().any(|film| film.kind == kind)
        {
            return Err(strict::unknown_entry().with_detail(kind.png_path()));
        }
    }
    let mut films = Vec::new();
    for film in &case.films {
        let path = film.kind.png_path();
        if archive.index_for_name(path).is_none() {
            return Err(EngineError::new(
                ErrorCode::CaseMissingFilm,
                "case is missing a listed film",
            )
            .with_detail(path));
        }
        let png = strict::read_entry(&mut archive, path)?;
        film_png::validate(&png, film)?;
        films.push(FilmPng {
            kind: film.kind,
            png,
        });
    }
    Ok(OpenedCase { case, films })
}
