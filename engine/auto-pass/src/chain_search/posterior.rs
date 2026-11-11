//! Exact min-marginals and scaled sum-product messages on the same graph
//! as the row search. Position confidence is separate from naming.

use super::dynamic_program::{BodyState, ChainProblem, solve};
use crate::handoffs::BodyCertainty;
use shapes::errors::{EngineError, ErrorCode};

#[derive(Clone, Debug)]
pub struct Posterior {
    pub certainty: Vec<BodyCertainty>,
    pub probabilities: Vec<Vec<f64>>,
    pub min_marginals: Vec<Vec<f64>>,
    pub log_partition: f64,
}

/// The selected states may come from a paired solve. The margin always
/// uses this conditioned problem's exact optimum. `step` converts rows
/// to the strip's length units; the right-body box is 0.3 body heights.
pub fn infer(
    p: &ChainProblem,
    selected: &[BodyState],
    step: f64,
    temperature: f64,
) -> Result<Posterior, EngineError> {
    p.validate()?;
    if selected.len() != p.unary.len()
        || !step.is_finite()
        || step <= 0.0
        || !temperature.is_finite()
        || temperature <= 0.0
        || selected.iter().any(|s| {
            s.height < p.min_height
                || s.height >= p.min_height + p.height_count
                || s.top >= p.rows
                || s.top + s.height >= p.rows
        })
    {
        return Err(EngineError::new(
            ErrorCode::EngineOutOfSync,
            "invalid chain posterior inputs",
        ));
    }
    let solution = solve(p)?;
    let n = p.unary.len();
    let h = p.height_count;
    let d = p.gap_count;
    let size = p.state_count();
    let mut backward = vec![vec![f64::INFINITY; size]; n];
    backward[n - 1].clone_from(&p.end);
    for v in (0..n - 1).rev() {
        let mut s = vec![f64::INFINITY; size];
        for bot in 0..p.rows {
            for hi in 0..h {
                for di in 0..d {
                    let top = bot + p.min_gap + di;
                    if top >= p.rows {
                        continue;
                    }
                    let j = top * h + hi;
                    if !p.valid_state(j) {
                        continue;
                    }
                    let candidate = ((p.gap[bot * d + di] + p.disc[v][di * h + hi])
                        + p.unary[v + 1][j])
                        + backward[v + 1][j];
                    s[bot * h + hi] = s[bot * h + hi].min(candidate);
                }
            }
        }
        for (i, value) in backward[v].iter_mut().enumerate() {
            if !p.valid_state(i) {
                continue;
            }
            let state = p.state(i);
            for next_h in 0..h {
                let candidate =
                    p.ratio[v][(i % h) * h + next_h] + s[(state.top + state.height) * h + next_h];
                *value = value.min(candidate);
            }
        }
    }
    let min_marginals: Vec<Vec<f64>> = solution
        .forward
        .iter()
        .zip(&backward)
        .map(|(a, b)| a.iter().zip(b).map(|(x, y)| x + y).collect())
        .collect();

    let (probabilities, log_partition) =
        scaled_probabilities(p, temperature).or_else(|_| log_probabilities(p, temperature))?;
    let mut certainty = Vec::with_capacity(n);
    for v in 0..n {
        let chosen = selected[v];
        let delta = 0.3 * chosen.height as f64;
        let mut posterior = 0.0;
        let mut mean_top = 0.0;
        let mut mean_bot = 0.0;
        let mut outside = f64::INFINITY;
        for (i, &mass) in probabilities[v].iter().enumerate() {
            let state = p.state(i);
            let top = state.top as f64;
            let bot = (state.top + state.height) as f64;
            mean_top += mass * top;
            mean_bot += mass * bot;
            if (top - chosen.top as f64).abs() <= delta
                && (bot - (chosen.top + chosen.height) as f64).abs() <= delta
            {
                posterior += mass;
            } else {
                outside = outside.min(min_marginals[v][i]);
            }
        }
        let mut variance_top = 0.0;
        let mut variance_bot = 0.0;
        for (i, &mass) in probabilities[v].iter().enumerate() {
            let state = p.state(i);
            let top = state.top as f64 - mean_top;
            let bot = (state.top + state.height) as f64 - mean_bot;
            variance_top += mass * top * top;
            variance_bot += mass * bot * bot;
        }
        certainty.push(BodyCertainty {
            posterior: posterior.clamp(0.0, 1.0),
            sup_spread: libm::sqrt(variance_top) * step,
            inf_spread: libm::sqrt(variance_bot) * step,
            // No alternative is represented by the largest finite gap,
            // so the frozen byte hand-off remains valid JSON.
            margin: if outside.is_finite() {
                (outside - solution.energy).max(0.0)
            } else {
                f64::MAX
            },
        });
    }
    Ok(Posterior {
        certainty,
        probabilities,
        min_marginals,
        log_partition,
    })
}

/// The usual bounded-cost path precomputes exponentials and uses scaled
/// probability messages, avoiding an exponential in each transition.
fn scaled_probabilities(
    p: &ChainProblem,
    temperature: f64,
) -> Result<(Vec<Vec<f64>>, f64), EngineError> {
    let n = p.unary.len();
    let h = p.height_count;
    let d = p.gap_count;
    let size = p.state_count();
    // A constant shift for each factor keeps every exponent at most 0;
    // stage normalisation removes the same constants from every path.
    let unary: Vec<_> = p.unary.iter().map(|v| weights(v, temperature)).collect();
    let ratio: Vec<_> = p.ratio.iter().map(|v| weights(v, temperature)).collect();
    let disc: Vec<_> = p.disc.iter().map(|v| weights(v, temperature)).collect();
    let gap = weights(&p.gap, temperature);
    let start = weights(&p.start, temperature);
    let end = weights(&p.end, temperature);
    let mut alpha = vec![vec![0.0; size]; n];
    for (i, value) in alpha[0].iter_mut().enumerate() {
        if p.valid_state(i) {
            *value = unary[0].0[i] * start.0[i];
        }
    }
    let mut log_partition = normalise(&mut alpha[0])? - (unary[0].1 + start.1) / temperature;
    for v in 1..n {
        let mut q = vec![0.0; size];
        for bot in p.min_height..p.rows {
            for next_h in 0..h {
                for hi in 0..h {
                    let height = p.min_height + hi;
                    if bot >= height {
                        q[bot * h + next_h] +=
                            alpha[v - 1][(bot - height) * h + hi] * ratio[v - 1].0[hi * h + next_h];
                    }
                }
            }
        }
        for i in 0..size {
            if !p.valid_state(i) {
                continue;
            }
            let top = i / h;
            for di in 0..d {
                let distance = p.min_gap + di;
                if top >= distance {
                    let bot = top - distance;
                    alpha[v][i] +=
                        q[bot * h + i % h] * gap.0[bot * d + di] * disc[v - 1].0[di * h + i % h];
                }
            }
            alpha[v][i] *= unary[v].0[i];
        }
        log_partition += normalise(&mut alpha[v])?
            - (unary[v].1 + ratio[v - 1].1 + disc[v - 1].1 + gap.1) / temperature;
    }
    let terminal: f64 = alpha[n - 1].iter().zip(&end.0).map(|(a, b)| a * b).sum();
    if terminal <= 0.0 || !terminal.is_finite() {
        return Err(no_mass());
    }
    log_partition += libm::log(terminal) - end.1 / temperature;
    let mut beta = vec![vec![0.0; size]; n];
    beta[n - 1].clone_from(&end.0);
    normalise(&mut beta[n - 1])?;
    for v in (0..n - 1).rev() {
        let mut s = vec![0.0; size];
        for bot in 0..p.rows {
            for hi in 0..h {
                for di in 0..d {
                    let top = bot + p.min_gap + di;
                    if top >= p.rows {
                        continue;
                    }
                    let j = top * h + hi;
                    if p.valid_state(j) {
                        s[bot * h + hi] += gap.0[bot * d + di]
                            * disc[v].0[di * h + hi]
                            * unary[v + 1].0[j]
                            * beta[v + 1][j];
                    }
                }
            }
        }
        for (i, value) in beta[v].iter_mut().enumerate() {
            if !p.valid_state(i) {
                continue;
            }
            let state = p.state(i);
            for next_h in 0..h {
                *value +=
                    ratio[v].0[(i % h) * h + next_h] * s[(state.top + state.height) * h + next_h];
            }
        }
        normalise(&mut beta[v])?;
    }
    let mut probabilities = alpha;
    for (marginal, backward) in probabilities.iter_mut().zip(&beta) {
        for (a, b) in marginal.iter_mut().zip(backward) {
            *a *= b;
        }
        normalise(marginal)?;
    }
    Ok((probabilities, log_partition))
}

/// Hard exclusions can make every low-cost factor entry unreachable.
/// If probability-space multiplication then underflows, repeat the same
/// split graph with log messages. This rare path preserves finite chains
/// without clamping their costs or changing the posterior temperature.
fn log_probabilities(
    p: &ChainProblem,
    temperature: f64,
) -> Result<(Vec<Vec<f64>>, f64), EngineError> {
    let n = p.unary.len();
    let h = p.height_count;
    let d = p.gap_count;
    let size = p.state_count();
    let mut alpha = vec![vec![f64::NEG_INFINITY; size]; n];
    for (i, value) in alpha[0].iter_mut().enumerate() {
        if p.valid_state(i) {
            *value = -(p.unary[0][i] + p.start[i]) / temperature;
        }
    }
    let mut log_partition = normalise_log(&mut alpha[0])?;
    for v in 1..n {
        let mut q = vec![f64::NEG_INFINITY; size];
        for bot in p.min_height..p.rows {
            for next_h in 0..h {
                for hi in 0..h {
                    let height = p.min_height + hi;
                    if bot >= height {
                        let candidate = alpha[v - 1][(bot - height) * h + hi]
                            - p.ratio[v - 1][hi * h + next_h] / temperature;
                        let value = &mut q[bot * h + next_h];
                        *value = log_add(*value, candidate);
                    }
                }
            }
        }
        for (i, value) in alpha[v].iter_mut().enumerate() {
            if !p.valid_state(i) {
                continue;
            }
            let top = i / h;
            for di in 0..d {
                let distance = p.min_gap + di;
                if top >= distance {
                    let bot = top - distance;
                    let candidate = q[bot * h + i % h]
                        - (p.gap[bot * d + di] + p.disc[v - 1][di * h + i % h]) / temperature;
                    *value = log_add(*value, candidate);
                }
            }
            *value -= p.unary[v][i] / temperature;
        }
        log_partition += normalise_log(&mut alpha[v])?;
    }
    let terminal = alpha[n - 1]
        .iter()
        .zip(&p.end)
        .fold(f64::NEG_INFINITY, |total, (a, end)| {
            log_add(total, a - end / temperature)
        });
    if !terminal.is_finite() {
        return Err(no_mass());
    }
    log_partition += terminal;
    let mut beta = vec![vec![f64::NEG_INFINITY; size]; n];
    for (value, end) in beta[n - 1].iter_mut().zip(&p.end) {
        *value = -end / temperature;
    }
    normalise_log(&mut beta[n - 1])?;
    for v in (0..n - 1).rev() {
        let mut s = vec![f64::NEG_INFINITY; size];
        for bot in 0..p.rows {
            for hi in 0..h {
                for di in 0..d {
                    let top = bot + p.min_gap + di;
                    if top >= p.rows {
                        continue;
                    }
                    let j = top * h + hi;
                    if p.valid_state(j) {
                        let candidate = beta[v + 1][j]
                            - (p.gap[bot * d + di] + p.disc[v][di * h + hi] + p.unary[v + 1][j])
                                / temperature;
                        let value = &mut s[bot * h + hi];
                        *value = log_add(*value, candidate);
                    }
                }
            }
        }
        for (i, value) in beta[v].iter_mut().enumerate() {
            if !p.valid_state(i) {
                continue;
            }
            let state = p.state(i);
            for next_h in 0..h {
                let candidate = s[(state.top + state.height) * h + next_h]
                    - p.ratio[v][(i % h) * h + next_h] / temperature;
                *value = log_add(*value, candidate);
            }
        }
        normalise_log(&mut beta[v])?;
    }
    for (marginal, backward) in alpha.iter_mut().zip(&beta) {
        for (a, b) in marginal.iter_mut().zip(backward) {
            *a += b;
        }
        normalise_log(marginal)?;
        for value in marginal {
            *value = libm::exp(*value);
        }
    }
    Ok((alpha, log_partition))
}

fn log_add(a: f64, b: f64) -> f64 {
    if a == f64::NEG_INFINITY {
        return b;
    }
    if b == f64::NEG_INFINITY {
        return a;
    }
    a.max(b) + libm::log1p(libm::exp(-(a - b).abs()))
}

fn normalise_log(values: &mut [f64]) -> Result<f64, EngineError> {
    let total = values.iter().copied().fold(f64::NEG_INFINITY, log_add);
    if !total.is_finite() {
        return Err(no_mass());
    }
    for value in values {
        *value -= total;
    }
    Ok(total)
}

fn weights(cost: &[f64], temperature: f64) -> (Vec<f64>, f64) {
    let minimum = cost
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .fold(f64::INFINITY, f64::min);
    let shift = if minimum.is_finite() { minimum } else { 0.0 };
    (
        cost.iter()
            .map(|c| libm::exp(-(c - shift) / temperature))
            .collect(),
        shift,
    )
}

fn normalise(values: &mut [f64]) -> Result<f64, EngineError> {
    let total: f64 = values.iter().sum();
    if total <= 0.0 || !total.is_finite() {
        return Err(no_mass());
    }
    for value in values {
        *value /= total;
    }
    Ok(libm::log(total))
}

fn no_mass() -> EngineError {
    EngineError::new(
        ErrorCode::PassNoSpineFound,
        "chain posterior has no finite probability mass",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exactly two complete chains, one row apart. Their energies are
    /// 1000 and 1000 + ln(3), so their masses are analytically 3/4 and 1/4.
    fn two_chains(bodies: usize) -> ChainProblem {
        let rows = bodies * 2 + 3;
        let mut unary = vec![vec![f64::INFINITY; rows]; bodies];
        for (v, table) in unary.iter_mut().enumerate() {
            table[2 + 2 * v] = if v == 0 { 1000.0 } else { 0.0 };
            table[3 + 2 * v] = if v == 0 { 1_001.098_612_288_668 } else { 0.0 };
        }
        // This cheap state can never reach the next body. For a one-body
        // chain it is instead excluded by the start constraint.
        unary[0][0] = 0.0;
        let mut start = vec![0.0; rows];
        if bodies == 1 {
            start[0] = f64::INFINITY;
        }
        ChainProblem {
            rows,
            min_height: 1,
            height_count: 1,
            min_gap: 1,
            gap_count: 1,
            unary,
            ratio: vec![vec![0.0]; bodies - 1],
            disc: vec![vec![0.0]; bodies - 1],
            gap: vec![0.0; rows],
            start,
            end: vec![0.0; rows],
        }
    }

    #[test]
    fn unreachable_cheap_states_cannot_erase_the_posterior() {
        for bodies in [1, 3] {
            let p = two_chains(bodies);
            assert!(scaled_probabilities(&p, 1.0).is_err());
            let solution = solve(&p).unwrap();
            let posterior = infer(&p, &solution.states, 1.0, 1.0).unwrap();
            // Decimal reference values are the closed forms ln(4/3)-1000
            // and sqrt(3)/4. 1e-10 allows cancellation near energy 1000.
            assert!((posterior.log_partition + 999.712_317_927_548_2).abs() < 1e-10);
            for (v, certainty) in posterior.certainty.iter().enumerate() {
                assert!((posterior.probabilities[v][2 + 2 * v] - 0.75).abs() < 1e-10);
                assert!((posterior.probabilities[v][3 + 2 * v] - 0.25).abs() < 1e-10);
                assert!((certainty.posterior - 0.75).abs() < 1e-10);
                assert!((certainty.sup_spread - 0.433_012_701_892_219_3).abs() < 1e-10);
                assert!((certainty.inf_spread - certainty.sup_spread).abs() < 1e-14);
                assert!((certainty.margin - 1.098_612_288_668_109_8).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn log_fallback_agrees_with_scaled_messages_when_both_are_safe() {
        let mut p = two_chains(3);
        p.unary[0][2] -= 1000.0;
        p.unary[0][3] -= 1000.0;
        let (fast, fast_log_z) = scaled_probabilities(&p, 0.7).unwrap();
        let (fallback, fallback_log_z) = log_probabilities(&p, 0.7).unwrap();
        assert!((fast_log_z - fallback_log_z).abs() < 1e-13);
        for (fast, fallback) in fast.iter().flatten().zip(fallback.iter().flatten()) {
            assert!((fast - fallback).abs() < 1e-13);
        }
    }
}
