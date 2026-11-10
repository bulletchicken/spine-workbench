//! `spine frames [--check]`: records the watch frames of a real auto pass
//! on the right-thoracic-40 made-up spine into
//! `proving-ground/known-answers/recorded-frames/`, where the app's
//! stand-in engine replays them. `--check` fails on any drift.

use crate::{FramesArgs, Outcome, not_built};

pub fn run(_args: &FramesArgs) -> Outcome {
    not_built("frames")
}
