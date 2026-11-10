//! Choosing with numbers. Datasets are parsed from bytes and reports are
//! returned as values: this crate never touches the disk, so it also runs
//! in the browser for the live answer-key readout. `spine score` does the
//! reading and writing.
//!
//! | Module | Holds |
//! |---|---|
//! | [`datasets`] | AASCE 2019, VertebraeKeypoints50, hand-marked cases, made-up spines |
//! | [`metrics`] | corner error, vertebrae found, level naming, Cobb, clicks to a good outline |
//! | [`simulated_user`] | scripted clicks for the tools, and the by-hand baseline |
//! | [`suites`] | the fixed sets of cases each score is run on |
//! | [`report`] | Markdown, JSON and SVG overlays |

#![forbid(unsafe_code)]

pub mod datasets;
pub mod metrics;
pub mod report;
pub mod simulated_user;
pub mod suites;
