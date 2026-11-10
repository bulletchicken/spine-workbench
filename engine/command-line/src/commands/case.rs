//! `spine case pack|unpack|check`: case files on the command line, read and
//! written by the same code the app uses.

use crate::{CaseAction, CaseArgs, Outcome, not_built};

pub fn run(args: &CaseArgs) -> Outcome {
    match args.action {
        CaseAction::Pack { .. } => not_built("case pack"),
        CaseAction::Unpack { .. } => not_built("case unpack"),
        CaseAction::Check { .. } => not_built("case check"),
    }
}
