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
#[cfg(feature = "test-support")]
pub mod test_support;
pub mod thumbnail;
