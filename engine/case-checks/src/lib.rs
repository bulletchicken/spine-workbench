//! Is the spine complete and in order? Each check returns findings, each a
//! warning or blocking, pointing at the film and vertebra concerned.
//!
//! | Module | Holds |
//! |---|---|
//! | [`presence`] | every vertebra present |
//! | [`order`] | levels in order, top to bottom |
//! | [`off_by_one`] | a likely off-by-one level |
//! | [`films_agree`] | front and side level sets match |
//! | [`confirmations`] | left/right, scale and L5 confirmed |

#![forbid(unsafe_code)]

pub mod confirmations;
pub mod films_agree;
pub mod off_by_one;
pub mod order;
pub mod presence;
