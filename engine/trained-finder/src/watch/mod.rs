//! The stepper and its frames: the trained finder as a state machine that
//! does a bounded slice of work per call and reports frames at fixed
//! points.

mod search;
pub mod trained_pass;

pub(crate) use search::Search;
pub use search::{Found, FoundVertebra};
pub use trained_pass::TrainedPass;
