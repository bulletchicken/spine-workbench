//! Bytes in, one greyscale film out. A film is recognised by its bytes,
//! never its file name; a DICOM file gives up only the tags on the
//! keep-list, and everything else is dropped while it is parsed.
//!
//! The result is a [`decoded_film::DecodedFilm`]: the stored values, bone
//! bright and exact, and the facts the file states. The case file's PNG
//! holds exactly these values, so a reopened case reads the same pixels.
//!
//! | Module | Holds |
//! |---|---|
//! | [`decoded_film`] | `DecodedFilm`, what every later step starts from |
//! | [`sniff`] | PNG, JPEG or DICOM, by magic bytes |
//! | [`png_jpeg`] | 8- and 16-bit greyscale; colour to luminance |
//! | [`dicom`] | the keep-list, pixel data, grey levels, scale tags |
//! | [`display_pixels`] | the half-float copy for the GPU |
//! | [`thumbnail`] | the small preview for the film slots |
//! | `test_support` | test DICOM writers (feature `test-support`) |

#![forbid(unsafe_code)]

pub mod decoded_film;
pub mod dicom;
pub mod display_pixels;
pub mod png_jpeg;
pub mod sniff;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
pub mod thumbnail;

use decoded_film::DecodedFilm;
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::{Film, FilmKind, MAX_FILM_SIDE, SourceFormat};

/// Maximum encoded input size. Limits apply before any codec is entered.
pub const MAX_FILE_BYTES: usize = 128 * 1024 * 1024;

/// Read by signature alone. No file name is accepted or retained.
pub fn read_film(bytes: &[u8], kind: FilmKind) -> Result<DecodedFilm, EngineError> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err(too_large());
    }
    match sniff::sniff(bytes)? {
        SourceFormat::Dicom => dicom::read(bytes, kind),
        format => png_jpeg::read(bytes, kind, format),
    }
}

/// Reopen a saved, metadata-free PNG and restore the original value map.
/// The pixels are already bone bright, so the map is never applied twice.
pub fn read_saved_film(bytes: &[u8], saved: &Film) -> Result<DecodedFilm, EngineError> {
    let mut decoded = read_film(bytes, saved.kind)?;
    if decoded.facts.source != SourceFormat::Png
        || decoded.facts.width != saved.image.width
        || decoded.facts.height != saved.image.height
        || decoded.facts.bits_stored != saved.image.bits_stored
    {
        return Err(EngineError::new(
            ErrorCode::CaseFilmSizeMismatch,
            "the saved PNG does not match its film record",
        ));
    }
    decoded.facts.source = saved.source;
    decoded.facts.kept_tags = saved.kept_tags.clone();
    decoded.facts.values = saved.values.clone();
    decoded.facts.flags = saved.flags.clone();
    decoded.facts.default_window = dicom::grey_levels::default_window(
        &decoded.image,
        decoded.facts.bits_stored,
        &decoded.facts.values,
        decoded.facts.kept_tags.as_ref(),
    );
    Ok(decoded)
}

pub(crate) fn invalid_film() -> EngineError {
    EngineError::new(ErrorCode::FilmCorrupt, "the film data is invalid")
}

pub(crate) fn too_large() -> EngineError {
    EngineError::new(
        ErrorCode::FilmTooLarge,
        "the film exceeds the intake limits",
    )
}

pub(crate) fn check_dimensions(
    width: u32,
    height: u32,
    file_bytes: usize,
    bytes_per_pixel: usize,
) -> Result<(), EngineError> {
    let pixels = u64::from(width) * u64::from(height);
    let need = 2 * file_bytes as u64 + pixels * bytes_per_pixel as u64;
    if width == 0 || height == 0 {
        return Err(invalid_film());
    }
    if width > MAX_FILM_SIDE
        || height > MAX_FILM_SIDE
        || pixels > 40_000_000
        || need > 900 * 1024 * 1024
    {
        return Err(too_large());
    }
    Ok(())
}

pub(crate) fn allocate<T>(len: usize) -> Result<Vec<T>, EngineError> {
    let mut values = Vec::new();
    values.try_reserve_exact(len).map_err(|_| {
        EngineError::new(
            ErrorCode::EngineOutOfMemory,
            "not enough memory to read the film",
        )
    })?;
    Ok(values)
}
