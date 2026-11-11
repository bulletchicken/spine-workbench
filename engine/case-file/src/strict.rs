//! The zip-level refusals: the exact entry list, size and ratio limits, no
//! path tricks, and each PNG the size its film record says.

use crate::layout;
use shapes::errors::{EngineError, ErrorCode};
use std::io::{Cursor, Read};
use zip::ZipArchive;

// Limits include room for all four full-length films. PNGs are already
// compressed; the writer stores them without a second compression layer.
pub const MAX_CASE_JSON_BYTES: u64 = 8 * 1024 * 1024;
pub const MAX_README_BYTES: u64 = 4096;
pub const MAX_FILM_PNG_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_TOTAL_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_COMPRESSION_RATIO: u64 = 1000;
/// At most forty million 16-bit pixels per film, before any allocation.
pub const MAX_DECODED_BYTES: usize = 80_000_000;

pub(crate) fn invalid(detail: &str) -> EngineError {
    EngineError::new(ErrorCode::CaseInvalidValue, "invalid case content").with_detail(detail)
}

pub(crate) fn too_large(detail: &str) -> EngineError {
    EngineError::new(
        ErrorCode::CaseTooLarge,
        "case exceeds a size or compression limit",
    )
    .with_detail(detail)
}

pub(crate) fn not_a_zip() -> EngineError {
    EngineError::new(ErrorCode::CaseNotAZip, "case is not a readable zip")
}

pub(crate) fn unknown_entry() -> EngineError {
    // Never copy a supplied path, comment or parser message into an error.
    EngineError::new(
        ErrorCode::CaseUnknownEntry,
        "case contains an unexpected entry",
    )
}

pub(crate) fn allocation() -> EngineError {
    EngineError::new(
        ErrorCode::EngineOutOfMemory,
        "not enough memory to read the case",
    )
}

pub(crate) fn entry_limit(name: &str) -> u64 {
    match name {
        layout::CASE_JSON => MAX_CASE_JSON_BYTES,
        layout::README => MAX_README_BYTES,
        _ => MAX_FILM_PNG_BYTES,
    }
}

/// Inspect headers before decompressing any entry. Header consistency
/// catches duplicate names, central/local path substitutions and overlaps.
pub(crate) fn archive(bytes: &[u8]) -> Result<ZipArchive<Cursor<&[u8]>>, EngineError> {
    if bytes.len() as u64 > MAX_ARCHIVE_BYTES {
        return Err(too_large("archive"));
    }
    bound_directory(bytes)?;
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| not_a_zip())?;
    if archive.len() > 6 {
        return Err(unknown_entry());
    }
    let mut total = 0_u64;
    for index in 0..archive.len() {
        let file = archive.by_index_raw(index).map_err(|_| not_a_zip())?;
        if !layout::is_allowed(file.name_raw()) || !file.is_file() {
            return Err(unknown_entry());
        }
        // Allowed names are ASCII constants, never an input filename.
        let name = std::str::from_utf8(file.name_raw()).map_err(|_| unknown_entry())?;
        if file.encrypted()
            || !matches!(
                file.compression(),
                zip::CompressionMethod::Stored | zip::CompressionMethod::Deflated
            )
        {
            return Err(not_a_zip());
        }
        total = total
            .checked_add(file.size())
            .ok_or_else(|| too_large("archive"))?;
        if file.size() > entry_limit(name)
            || file.size() > file.compressed_size().saturating_mul(MAX_COMPRESSION_RATIO)
        {
            return Err(too_large(name));
        }
        if total > MAX_TOTAL_BYTES {
            return Err(too_large("archive"));
        }
    }
    archive
        .validate_header_consistency()
        .map_err(|_| unknown_entry())?;
    Ok(archive)
}

/// The classic end record bounds directory allocation in the zip reader.
/// Bound both classic and ZIP64 counts before the zip crate reserves
/// storage for attacker-controlled headers.
fn bound_directory(bytes: &[u8]) -> Result<(), EngineError> {
    let start = bytes.len().saturating_sub(22 + usize::from(u16::MAX));
    let Some(end) = bytes.len().checked_sub(22) else {
        return Err(not_a_zip());
    };
    for offset in (start..=end).rev() {
        if &bytes[offset..offset + 4] != b"PK\x05\x06" {
            continue;
        }
        let short = |at| u16::from_le_bytes([bytes[offset + at], bytes[offset + at + 1]]);
        if offset + 22 + usize::from(short(20)) != bytes.len() {
            continue;
        }
        if short(4) != 0 || short(6) != 0 || short(8) != short(10) {
            return Err(not_a_zip());
        }
        if short(10) > 6 {
            return Err(unknown_entry());
        }
        if offset >= 20 && &bytes[offset - 20..offset - 16] == b"PK\x06\x07" {
            bound_zip64(bytes, offset - 20)?;
        } else if bytes[offset + 12..offset + 16] == [255; 4]
            || bytes[offset + 16..offset + 20] == [255; 4]
        {
            return Err(not_a_zip());
        }
        return Ok(());
    }
    Err(not_a_zip())
}

fn bound_zip64(bytes: &[u8], locator: usize) -> Result<(), EngineError> {
    let long = |at| u64::from_le_bytes(bytes[at..at + 8].try_into().expect("bounded ZIP64 field"));
    let start = usize::try_from(long(locator + 8)).map_err(|_| too_large("archive"))?;
    if start > locator.saturating_sub(56) || &bytes[start..start + 4] != b"PK\x06\x06" {
        return Err(not_a_zip());
    }
    let count = long(start + 32);
    if count > 6 || long(start + 24) > 6 {
        return Err(too_large("archive"));
    }
    if long(start + 4) != (locator - start - 12) as u64
        || long(start + 24) != count
        || bytes[start + 16..start + 24] != [0; 8]
        || bytes[locator + 4..locator + 8] != [0; 4]
        || bytes[locator + 16..locator + 20] != [1, 0, 0, 0]
    {
        return Err(not_a_zip());
    }
    if long(start + 40) > MAX_ARCHIVE_BYTES || long(start + 48) > MAX_ARCHIVE_BYTES {
        return Err(too_large("archive"));
    }
    Ok(())
}

/// Read through EOF (including the CRC check), with a second bound on
/// actual inflated bytes so a dishonest length cannot grow the buffer.
pub(crate) fn read_entry(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
) -> Result<Vec<u8>, EngineError> {
    let file = archive.by_name(name).map_err(|_| not_a_zip())?;
    let expected = file.size();
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(expected as usize)
        .map_err(|_| allocation())?;
    file.take(expected + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| not_a_zip())?;
    if bytes.len() as u64 != expected {
        return Err(not_a_zip());
    }
    Ok(bytes)
}
