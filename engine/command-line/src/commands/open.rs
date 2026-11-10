//! `spine open <film>`: reads a film as the app does and prints its size,
//! its kept tags and the scales its tags support.

use crate::{OpenArgs, Outcome, not_built};

pub fn run(_args: &OpenArgs) -> Outcome {
    not_built("open")
}
