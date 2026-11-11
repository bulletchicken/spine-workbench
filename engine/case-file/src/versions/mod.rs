//! The version gate: a newer version is refused with "made by a newer
//! version"; older versions are upgraded on reading.

pub mod v1;

// Version 1 is the only released version. Its strict reader owns the
// format/version gate, including the distinct newer-version refusal.
pub use v1::read;
