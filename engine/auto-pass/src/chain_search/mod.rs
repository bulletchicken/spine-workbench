//! Stage 4, the exact chain search: every vertebra from C7 to L5 found at once
//! on the strip, for each vertebra count. Hands on `ChainBoundaries`.

pub mod dynamic_program;
pub mod gap_score;
pub mod height_prior;
pub mod paired;
pub mod posterior;
pub mod survey;
mod tilts;
pub mod two_views;
