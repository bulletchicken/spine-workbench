//! One zip holding a case, read strictly: `case.json`, one lossless PNG per
//! film and a short `README.txt`. Original DICOM or JPEG bytes are never
//! stored, so no tag or EXIF can travel with a case. `case.json` itself is
//! read and written by `shapes::reading`; this crate adds the zip around
//! it.
//!
//! | Module | Holds |
//! |---|---|
//! | [`layout`] | the allowed entries |
//! | [`film_png`] | films as lossless greyscale PNG |
//! | [`pack`] | writing a case |
//! | [`unpack`] | reading a case |
//! | [`strict`] | the zip-level refusals |
//! | [`versions`] | the version gate and each version |

#![forbid(unsafe_code)]

pub mod film_png;
pub mod layout;
pub mod pack;
pub mod strict;
pub mod unpack;
pub mod versions;
