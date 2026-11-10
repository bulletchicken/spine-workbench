//! Stage 1, evidence maps: from one film to the maps every later stage reads
//! (phase edges, a gradient fallback, ridges and disc-gap valleys, phase
//! symmetry, and pedicle rings on front films), on one working grid, with a
//! validity mask, the polarity check and the noise floor. Hands on a
//! `FilmEvidence`.

pub mod edges;
pub mod pedicle_rings;
pub mod ridges;
