//! `spine open <film>`: reads a film as the app does and prints its size,
//! value map and kept tags, including the file's scale tags, as JSON.
//! The input path is used only to open the bytes, never to identify the
//! format or to produce output or diagnostics.

use crate::{OpenArgs, Outcome};
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::FilmKind;
use std::fs::File;
use std::io::{Read, Write};

pub fn run(args: &OpenArgs) -> Outcome {
    let input = File::open(&args.film).map_err(|_| "could not open the film input")?;
    let mut bytes = Vec::new();
    input
        .take(film_reader::MAX_FILE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "could not read the film input")?;
    if bytes.len() > film_reader::MAX_FILE_BYTES {
        return Err(EngineError::new(ErrorCode::FilmTooLarge, "the film file is too large").into());
    }
    // This command inspects a file, without choosing a case slot. The
    // front slot supplies the required kind without guessing from a path.
    let film = film_reader::read_film(&bytes, FilmKind::Front)?;
    let json =
        serde_json::to_string_pretty(&film.facts).map_err(|_| "could not write the film facts")?;
    writeln!(std::io::stdout().lock(), "{json}").map_err(|_| "could not write the film facts")?;
    Ok(())
}
