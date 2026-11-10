//! The watchable top-down vertebra search.
//!
//! A **film pass** runs stages 1 to 3 on each film, in that film's worker.
//! One **chain pass** runs stages 4 to 6 over every film's strip, in the
//! spine worker. Each film's worker then places the points. The stages
//! hand on the batons in [`handoffs`], which is frozen, so the film stages
//! and the chain stages can be built at the same time. Every result leaves
//! the pass in original film pixels.
//!
//! | Module | Holds |
//! |---|---|
//! | [`handoffs`] | the batons between stages and their byte encodings |
//! | [`settings`] | every tunable weight |
//! | [`evidence`] | stage 1: evidence maps |
//! | [`centre_line`] | stage 2: the spine's centre line |
//! | [`strip`] | stage 3: the spine unrolled, and its barcode |
//! | [`chain_search`] | stage 4: the exact chain search |
//! | [`level_naming`] | stage 5: naming from both ends |
//! | [`confidence`] | stage 6: confidence per vertebra |
//! | [`points`] | the full point set on each film |
//! | [`landmarks`] | C7, the sacrum top, hip centres |
//! | [`watch`] | the steppers and their frames |
//! | [`run`] | one-call entry points for native use |

#![forbid(unsafe_code)]

pub mod centre_line;
pub mod chain_search;
pub mod confidence;
pub mod evidence;
pub mod handoffs;
pub mod landmarks;
pub mod level_naming;
pub mod points;
pub mod run;
pub mod settings;
pub mod strip;
pub mod watch;
