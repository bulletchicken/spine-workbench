//! With no side film, no mark sees front-to-back place or the side film's
//! registration. One row pins the origin vertebra's front-to-back place and
//! one pins the absent registration; neither changes any reported value.

use crate::parameters::COUNT;
use crate::patient_origin;
use crate::problem::Problem;
use crate::residuals::Row;
use crate::toward::{self, Spread};

// Any spread fixes a pure gauge; the reported centres are measured from the
// origin vertebra's underside, so this value never reaches them.
const GAUGE: Spread = Spread { mean: 0.0, sd: 1.0 };

pub fn rows(problem: &Problem, numbers: &[f64], differentiate: bool) -> Vec<Row> {
    if problem.side.is_some() {
        return Vec::new();
    }
    let origin = patient_origin::index(problem) * COUNT;
    let offset = problem.observations.len() * COUNT + 1;
    vec![
        toward::linear(numbers, &[(origin, 1.0)], GAUGE, differentiate),
        toward::linear(numbers, &[(offset, 1.0)], GAUGE, differentiate),
    ]
}
