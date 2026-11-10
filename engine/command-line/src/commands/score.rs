//! `spine score <suite>`: reads the suite's dataset (from `local-only/` for
//! real films), runs the scorekeeper, and writes a dated report to
//! `proving-ground/scoreboards/reports/`. `spine compare` compares two such
//! reports with a paired test.

use crate::{CompareArgs, Outcome, ScoreArgs, not_built};

pub fn run(_args: &ScoreArgs) -> Outcome {
    not_built("score")
}

pub fn compare(_args: &CompareArgs) -> Outcome {
    not_built("compare")
}
