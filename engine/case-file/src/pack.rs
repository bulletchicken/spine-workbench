//! Writing a case file.

use crate::{film_png, layout, strict, versions};
use shapes::case_file_v1::CaseFileV1;
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::FilmPng;
use std::collections::BTreeSet;
use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;

/// Validate before writing, then store only canonical JSON, freshly
/// encoded pixels and the fixed note. Source filenames never enter here.
pub fn pack_case(case: &CaseFileV1, films: &[FilmPng]) -> Result<Vec<u8>, EngineError> {
    let json = versions::v1::write(case)?;
    if json.len() as u64 > strict::MAX_CASE_JSON_BYTES {
        return Err(strict::too_large(layout::CASE_JSON));
    }
    let mut seen = BTreeSet::new();
    for png in films {
        if !seen.insert(png.kind) || !case.films.iter().any(|film| film.kind == png.kind) {
            return Err(strict::unknown_entry().with_detail(png.kind.png_path()));
        }
    }
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .last_modified_time(zip::DateTime::default())
        .unix_permissions(0o644);
    zip.start_file(
        layout::CASE_JSON,
        options.compression_method(zip::CompressionMethod::Deflated),
    )
    .map_err(|_| strict::not_a_zip())?;
    zip.write_all(json.as_bytes())
        .map_err(|_| strict::not_a_zip())?;
    // Record order is kept in JSON; archive order is fixed by film kind.
    for kind in shapes::film::FilmKind::ALL {
        let Some(film) = case.films.iter().find(|film| film.kind == kind) else {
            continue;
        };
        let source = films.iter().find(|png| png.kind == kind).ok_or_else(|| {
            EngineError::new(ErrorCode::CaseMissingFilm, "case is missing a listed film")
                .with_detail(kind.png_path())
        })?;
        let png = film_png::clean(&source.png, film)?;
        if png.len() as u64 > strict::MAX_FILM_PNG_BYTES {
            return Err(strict::too_large(kind.png_path()));
        }
        zip.start_file(
            kind.png_path(),
            options.compression_method(zip::CompressionMethod::Stored),
        )
        .map_err(|_| strict::not_a_zip())?;
        zip.write_all(&png).map_err(|_| strict::not_a_zip())?;
    }
    zip.start_file(
        layout::README,
        options.compression_method(zip::CompressionMethod::Stored),
    )
    .map_err(|_| strict::not_a_zip())?;
    zip.write_all(layout::README_TEXT.as_bytes())
        .map_err(|_| strict::not_a_zip())?;
    let bytes = zip.finish().map_err(|_| strict::not_a_zip())?.into_inner();
    // Apply precisely the opening limits to our own output too. This is
    // header-only: PNGs have already been decoded and encoded above.
    strict::archive(&bytes)?;
    Ok(bytes)
}
