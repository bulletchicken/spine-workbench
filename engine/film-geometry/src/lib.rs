//! The two facts every film needs before work starts: how big a pixel is,
//! and which way the film is turned and faces. Every answer here is a
//! proposal with its evidence; the user confirms, and only confirmed facts
//! are saved.
//!
//! | Module | Holds |
//! |---|---|
//! | [`rough_trace`] | a cheap trace on a coarse copy; collimation; the brightness check |
//! | [`scale`] | scale from tags, a ruler or a known object |
//! | [`upright`] | quarter turns and upside-down |
//! | [`side_facing`] | which way a side film faces |
//! | [`lead_marker`] | the lead left/right letter and possible burnt-in text |

#![forbid(unsafe_code)]

pub mod lead_marker;
pub mod rough_trace;
pub mod scale;
pub mod side_facing;
pub mod upright;
