//! `spine pass KIND=PATH...`: runs the whole auto pass on one to four films
//! with no browser, and prints every film's marks as JSON.

use crate::{Outcome, PassArgs, not_built};

pub fn run(_args: &PassArgs) -> Outcome {
    not_built("pass")
}
