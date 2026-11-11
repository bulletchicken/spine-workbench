//! Fixed-row tilt consistency. The row optimum relaxes the neighbouring
//! tilt coupling; this second exact, small chain is a two-stage
//! approximation to a joint row-and-angle search.

use super::dynamic_program::BodyState;
use super::gap_score::LatticeEvidence;

pub struct TiltSolution {
    pub angles: Vec<[f64; 2]>,
    pub inconsistent: Vec<bool>,
    pub limit_hits: usize,
    pub sacral_tilt: Option<f64>,
}

pub fn settle(e: &LatticeEvidence, states: &[BodyState], sacrum: Option<usize>) -> TiltSolution {
    let t = e.tilts.len();
    let n = states.len() * 2;
    let mut costs = vec![vec![f64::INFINITY; t]; n];
    let mut parents = vec![vec![0; t]; n];
    for plate in 0..n {
        let body = states[plate / 2];
        let row = if plate % 2 == 0 {
            body.top
        } else {
            body.top + body.height
        };
        for current in 0..t {
            let unary = -if plate % 2 == 0 {
                e.top_score(row, current)
            } else {
                e.bottom_score(row, current)
            };
            if plate == 0 {
                costs[plate][current] = unary;
                continue;
            }
            for previous in 0..t {
                let angle = e.tilts[current];
                let before = e.tilts[previous];
                let pair = if plate % 2 == 1 {
                    if (angle - before).abs() <= 20.0_f64.to_radians() + 1e-12 {
                        0.0
                    } else {
                        f64::INFINITY
                    }
                } else {
                    let upper = states[plate / 2 - 1];
                    let gap = (row - upper.top - upper.height) as f64 * e.step;
                    disc_cost(angle, before, gap, e.half_width[row])
                };
                let candidate = costs[plate - 1][previous] + pair + unary;
                if candidate < costs[plate][current] {
                    costs[plate][current] = candidate;
                    parents[plate][current] = previous;
                }
            }
        }
    }
    let mut terminal = costs[n - 1].clone();
    let mut plateau_angles = vec![0.0; t];
    let mut plateau_inconsistent = false;
    if let Some(row) = sacrum {
        let last = states[states.len() - 1];
        let gap = (row - last.top - last.height) as f64 * e.step;
        for (i, cost) in terminal.iter_mut().enumerate() {
            let (transition, angle) =
                plateau_transition(e.sacral_options(row), e.tilts[i], gap, e.half_width[row]);
            *cost += transition;
            plateau_angles[i] = angle;
        }
        if terminal.iter().all(|v| !v.is_finite()) {
            terminal.clone_from(&costs[n - 1]);
            plateau_angles.fill(e.sacrum(row).1);
            plateau_inconsistent = true;
        }
    }
    let mut selected = terminal
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.total_cmp(b.1))
        .map(|(i, _)| i)
        .unwrap_or(0);
    let sacral_tilt = sacrum.map(|_| plateau_angles[selected]);
    let mut angles = vec![[0.0; 2]; states.len()];
    let mut indices = vec![[0; 2]; states.len()];
    let mut limit_hits = 0;
    for plate in (0..n).rev() {
        angles[plate / 2][plate % 2] = e.tilts[selected];
        indices[plate / 2][plate % 2] = selected;
        limit_hits += usize::from(selected == 0 || selected + 1 == t);
        selected = parents[plate][selected];
    }
    let mut inconsistent: Vec<_> = states
        .iter()
        .zip(&indices)
        .map(|(body, chosen)| {
            let lost = e.body_edges(body.top, body.height).0
                - e.top_score(body.top, chosen[0])
                - e.bottom_score(body.top + body.height, chosen[1]);
            lost > 2.0
        })
        .collect();
    if plateau_inconsistent {
        *inconsistent.last_mut().expect("a nonempty solved chain") = true;
    }
    TiltSolution {
        angles,
        inconsistent,
        limit_hits,
        sacral_tilt,
    }
}

fn plateau_transition(options: &[(f64, f64)], upper: f64, gap: f64, half_width: f64) -> (f64, f64) {
    let mut best = (f64::INFINITY, 0.0);
    for &(cost, angle) in options {
        let total = cost + disc_cost(angle, upper, gap, half_width);
        if total < best.0 {
            best = (total, angle);
        }
    }
    best
}

fn disc_cost(lower: f64, upper: f64, gap: f64, half_width: f64) -> f64 {
    if gap < 0.6 * half_width * (libm::tan(lower) - libm::tan(upper)).abs() + 0.5 {
        f64::INFINITY
    } else {
        let difference = (lower - upper) / 10.0_f64.to_radians();
        0.3 * (difference * difference).min(9.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_weaker_sacral_angle_is_kept_when_the_strongest_would_cross() {
        // A 5-mm gap at half-width 15 cannot connect a horizontal body
        // to a 45° plateau: the 9-mm tilt excursion exceeds the gap.
        // The 10° candidate has 9*tan(10°)+0.5 ≈ 2.09 mm excursion.
        let options = [(-6.0, 45.0_f64.to_radians()), (-4.0, 10.0_f64.to_radians())];
        let chosen = plateau_transition(&options, 0.0, 5.0, 15.0);
        assert_eq!(chosen.1, 10.0_f64.to_radians());
        // One 10° difference costs 0.3, so the weaker line totals -3.7.
        assert!((chosen.0 + 3.7).abs() < 1e-12);
    }
}
