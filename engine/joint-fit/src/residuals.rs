//! Mark residuals in stated-film millimetres, differentiated one vertebra at a time.

use crate::parameters::{COUNT, vertebra};
use crate::problem::{Observations, Problem};
use forward_model::cameras::parallel::ParallelCamera;
use forward_model::project::{front_marks, side_marks};
use shapes::marks::Mark;

/// One scalar residual and its nonzero Jacobian entries.
pub struct Row {
    pub value: f64,
    pub derivatives: Vec<(usize, f64)>,
}

pub struct Marks {
    pub weighted: Vec<f64>,
    pub squared_mm: [f64; 2],
    pub squared_spreads: [f64; 2],
    pub front_comparison_squared: f64,
    pub counts: [usize; 2],
}

fn append(
    result: &mut Marks,
    observed: Mark,
    predicted: Mark,
    spacing: [f64; 2],
    sd: f64,
    shared_sd: f64,
    view: usize,
) {
    let sd = sd * if observed.inferred { 3.0 } else { 1.0 };
    let differences = [
        (predicted.at.x() - observed.at.x()) * spacing[0],
        (predicted.at.y() - observed.at.y()) * spacing[1],
    ];
    for difference in differences {
        let normalised = difference / sd;
        result.weighted.push(normalised);
        result.squared_spreads[view] += normalised * normalised;
        if view == 0 {
            result.front_comparison_squared +=
                difference * difference / (sd * sd + shared_sd * shared_sd);
        }
        result.squared_mm[view] += difference * difference;
    }
    result.counts[view] += 1;
}

pub fn marks(
    problem: &Problem,
    observation: &Observations,
    numbers: &[f64],
    side: Option<&ParallelCamera>,
) -> Option<Marks> {
    let v = vertebra(numbers);
    let mut result = Marks {
        weighted: Vec::with_capacity(20),
        squared_mm: [0.0; 2],
        squared_spreads: [0.0; 2],
        front_comparison_squared: 0.0,
        counts: [0; 2],
    };
    if let Some(observed) = observation.front {
        let predicted = front_marks(&v, &problem.front).ok()?;
        let sd = observation.spread_mm[0];
        for (o, p) in observed.corners().into_iter().zip(predicted.corners()) {
            append(
                &mut result,
                o,
                p,
                problem.front_spacing,
                sd,
                observation.shared_mm[0],
                0,
            );
        }
        for (o, p) in [
            (observed.ped_l, predicted.ped_l),
            (observed.ped_r, predicted.ped_r),
        ] {
            if let Some(o) = o.mark() {
                append(
                    &mut result,
                    o,
                    p.mark()?,
                    problem.front_spacing,
                    2.0 * sd,
                    observation.shared_mm[0],
                    0,
                );
            }
        }
    }
    if let Some(observed) = observation.side {
        let predicted = side_marks(&v, side?).ok()?;
        let sd = observation.spread_mm[1];
        for (o, p) in observed.corners().into_iter().zip(predicted.corners()) {
            append(&mut result, o, p, problem.side_spacing, sd, 0.0, 1);
        }
    }
    result
        .weighted
        .iter()
        .all(|x| x.is_finite())
        .then_some(result)
}

pub fn rows(problem: &Problem, numbers: &[f64], differentiate: bool) -> Option<Vec<Row>> {
    let n = problem.observations.len();
    let global = n * COUNT;
    let side = problem.side_camera(numbers[global], numbers[global + 1]);
    let mut rows = Vec::new();
    for (i, observation) in problem.observations.iter().enumerate() {
        let local = &numbers[i * COUNT..(i + 1) * COUNT];
        let values = marks(problem, observation, local, side.as_ref())?.weighted;
        let mut block: Vec<_> = values
            .into_iter()
            .map(|value| Row {
                value,
                derivatives: Vec::with_capacity(COUNT + 2),
            })
            .collect();
        if differentiate {
            for j in 0..COUNT + 2 {
                if j >= COUNT && observation.side.is_none() {
                    continue;
                }
                let index = if j < COUNT {
                    i * COUNT + j
                } else {
                    global + j - COUNT
                };
                let h = 1e-5 * numbers[index].abs().max(1.0);
                let (plus, minus) = if j < COUNT {
                    let mut p = local.to_vec();
                    let mut m = p.clone();
                    p[j] += h;
                    m[j] -= h;
                    (
                        marks(problem, observation, &p, side.as_ref())?,
                        marks(problem, observation, &m, side.as_ref())?,
                    )
                } else {
                    let mut p = [numbers[global], numbers[global + 1]];
                    let mut m = p;
                    p[j - COUNT] += h;
                    m[j - COUNT] -= h;
                    (
                        marks(
                            problem,
                            observation,
                            local,
                            problem.side_camera(p[0], p[1]).as_ref(),
                        )?,
                        marks(
                            problem,
                            observation,
                            local,
                            problem.side_camera(m[0], m[1]).as_ref(),
                        )?,
                    )
                };
                for ((row, p), m) in block.iter_mut().zip(plus.weighted).zip(minus.weighted) {
                    row.derivatives.push((index, (p - m) / (2.0 * h)));
                }
            }
        }
        rows.extend(block);
    }
    Some(rows)
}
