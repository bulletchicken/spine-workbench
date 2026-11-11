//! Exact min-sum search, split at the inferior endplate. No state is
//! pruned: each transition costs O(K H² + K H D), rather than O(K H² D).
//! Hard exclusions are positive infinity. Ties choose the smallest
//! predecessor top, then height; the terminal tie uses the same order.

use shapes::errors::{EngineError, ErrorCode};

/// A body occupies `[top, top + height)` lattice cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyState {
    pub top: usize,
    pub height: usize,
}

/// Finite costs or positive infinity, with no evidence implicit in the
/// solver. State index is `top * height_count + height - min_height`.
#[derive(Clone, Debug)]
pub struct ChainProblem {
    pub rows: usize,
    pub min_height: usize,
    pub height_count: usize,
    pub min_gap: usize,
    pub gap_count: usize,
    /// One K×H table per body.
    pub unary: Vec<Vec<f64>>,
    /// One H×H table per transition: upper height, then lower height.
    pub ratio: Vec<Vec<f64>>,
    /// One D×H table per transition: gap, then lower height.
    pub disc: Vec<Vec<f64>>,
    /// K×D table: inferior boundary, then gap.
    pub gap: Vec<f64>,
    /// K×H terminal costs, excluding the body's unary.
    pub start: Vec<f64>,
    pub end: Vec<f64>,
}

impl ChainProblem {
    pub fn state_count(&self) -> usize {
        self.rows * self.height_count
    }

    pub fn state(&self, index: usize) -> BodyState {
        BodyState {
            top: index / self.height_count,
            height: self.min_height + index % self.height_count,
        }
    }

    pub fn index(&self, state: BodyState) -> usize {
        state.top * self.height_count + state.height - self.min_height
    }

    pub fn valid_state(&self, index: usize) -> bool {
        let state = self.state(index);
        state.top + state.height < self.rows
    }

    /// Reject malformed tables before indexing or allocating messages.
    pub fn validate(&self) -> Result<(), EngineError> {
        let bad = || EngineError::new(ErrorCode::EngineOutOfSync, "invalid chain energy tables");
        let h = self.height_count;
        let d = self.gap_count;
        let n = self.unary.len();
        let Some(kh) = self.rows.checked_mul(h) else {
            return Err(bad());
        };
        let Some(kd) = self.rows.checked_mul(d) else {
            return Err(bad());
        };
        let Some(hh) = h.checked_mul(h) else {
            return Err(bad());
        };
        let Some(dh) = d.checked_mul(h) else {
            return Err(bad());
        };
        if n == 0
            || self.rows == 0
            || h == 0
            || d == 0
            || self.min_height == 0
            || self.min_gap == 0
            || self.min_height.checked_add(h).is_none_or(|v| v > self.rows)
            || self.min_gap.checked_add(d).is_none_or(|v| v > self.rows)
            || self.unary.iter().any(|v| v.len() != kh)
            || self.ratio.len() != n - 1
            || self.disc.len() != n - 1
            || self.ratio.iter().any(|v| v.len() != hh)
            || self.disc.iter().any(|v| v.len() != dh)
            || self.gap.len() != kd
            || self.start.len() != kh
            || self.end.len() != kh
        {
            return Err(bad());
        }
        if self
            .unary
            .iter()
            .chain(&self.ratio)
            .chain(&self.disc)
            .chain([&self.gap, &self.start, &self.end])
            .flatten()
            .any(|v| v.is_nan() || *v == f64::NEG_INFINITY)
        {
            return Err(bad());
        }
        Ok(())
    }
}

/// An actual forward-message checkpoint, independent of call batching.
#[derive(Clone, Debug, PartialEq)]
pub struct SweepCheckpoint {
    pub body: usize,
    pub path: Vec<BodyState>,
    /// Minimum prefix energy at each inferior boundary.
    pub boundary_energy: Vec<f64>,
}

#[derive(Clone, Debug)]
pub struct Solution {
    pub states: Vec<BodyState>,
    pub energy: f64,
    pub sweeps: Vec<SweepCheckpoint>,
    /// Forward messages retained for exact min-marginals.
    pub forward: Vec<Vec<f64>>,
}

/// One complete body per advance, for the worker's stage stepper.
pub struct ForwardSweep<'a> {
    problem: &'a ChainProblem,
    forward: Vec<Vec<f64>>,
    parents: Vec<Vec<usize>>,
    checkpoints: Vec<SweepCheckpoint>,
}

impl<'a> ForwardSweep<'a> {
    pub fn new(problem: &'a ChainProblem) -> Result<Self, EngineError> {
        problem.validate()?;
        Ok(Self {
            problem,
            forward: Vec::new(),
            parents: Vec::new(),
            checkpoints: Vec::new(),
        })
    }

    /// `None` after the last body. No clock or requested slice size
    /// changes the checkpoints or the arithmetic order.
    pub fn advance(&mut self) -> Option<&SweepCheckpoint> {
        let p = self.problem;
        let v = self.forward.len();
        if v == p.unary.len() {
            return None;
        }
        let h_count = p.height_count;
        let mut next = vec![f64::INFINITY; p.state_count()];
        let mut parent = vec![usize::MAX; p.state_count()];
        if v == 0 {
            for (i, value) in next.iter_mut().enumerate() {
                if p.valid_state(i) {
                    *value = p.start[i] + p.unary[0][i];
                }
            }
        } else {
            let mut q = vec![f64::INFINITY; p.state_count()];
            let mut q_parent = vec![usize::MAX; p.state_count()];
            for bot in p.min_height..p.rows {
                for lower_h in 0..h_count {
                    let qi = bot * h_count + lower_h;
                    // Larger height means smaller predecessor top.
                    for upper_h in (0..h_count).rev() {
                        let height = p.min_height + upper_h;
                        if bot < height {
                            continue;
                        }
                        let previous = (bot - height) * h_count + upper_h;
                        let cost = self.forward[v - 1][previous]
                            + p.ratio[v - 1][upper_h * h_count + lower_h];
                        if cost < q[qi] {
                            q[qi] = cost;
                            q_parent[qi] = previous;
                        }
                    }
                }
            }
            for (i, value) in next.iter_mut().enumerate() {
                if !p.valid_state(i) {
                    continue;
                }
                let state = p.state(i);
                let hi = i % h_count;
                let mut best = f64::INFINITY;
                for di in 0..p.gap_count {
                    let gap = p.min_gap + di;
                    if state.top < gap {
                        continue;
                    }
                    let bot = state.top - gap;
                    let qi = bot * h_count + hi;
                    let candidate =
                        (q[qi] + p.gap[bot * p.gap_count + di]) + p.disc[v - 1][di * h_count + hi];
                    if candidate < best || (candidate == best && q_parent[qi] < parent[i]) {
                        best = candidate;
                        parent[i] = q_parent[qi];
                    }
                }
                *value = best + p.unary[v][i];
            }
        }
        self.parents.push(parent);
        let mut boundary_energy = vec![f64::INFINITY; p.rows];
        let mut best = None;
        for (i, &cost) in next.iter().enumerate() {
            if !cost.is_finite() {
                continue;
            }
            let state = p.state(i);
            let bot = state.top + state.height;
            boundary_energy[bot] = boundary_energy[bot].min(cost);
            if best.is_none_or(|j| cost < next[j]) {
                best = Some(i);
            }
        }
        let path = best.map(|i| self.backtrack(v, i)).unwrap_or_default();
        self.forward.push(next);
        self.checkpoints.push(SweepCheckpoint {
            body: v,
            path,
            boundary_energy,
        });
        self.checkpoints.last()
    }

    fn backtrack(&self, body: usize, mut index: usize) -> Vec<BodyState> {
        let mut path = Vec::with_capacity(body + 1);
        for v in (0..=body).rev() {
            path.push(self.problem.state(index));
            if v > 0 {
                index = self.parents[v][index];
            }
        }
        path.reverse();
        path
    }

    pub fn finish(self) -> Result<Solution, EngineError> {
        let p = self.problem;
        if self.forward.len() != p.unary.len() {
            return Err(EngineError::new(
                ErrorCode::EngineOutOfSync,
                "chain sweep is unfinished",
            ));
        }
        let last = self.forward.last().expect("nonempty validated chain");
        let mut energy = f64::INFINITY;
        let mut best = 0;
        for (i, value) in last.iter().enumerate() {
            let candidate = *value + p.end[i];
            if candidate < energy {
                energy = candidate;
                best = i;
            }
        }
        if !energy.is_finite() {
            return Err(EngineError::new(
                ErrorCode::ToolConstraintsConflict,
                "no finite chain satisfies the energy tables",
            ));
        }
        let states = self.backtrack(p.unary.len() - 1, best);
        Ok(Solution {
            states,
            energy,
            sweeps: self.checkpoints,
            forward: self.forward,
        })
    }
}

pub fn solve(problem: &ChainProblem) -> Result<Solution, EngineError> {
    let mut sweep = ForwardSweep::new(problem)?;
    while sweep.advance().is_some() {}
    sweep.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain_search::posterior::infer;
    use proptest::prelude::*;

    fn problem(seed: u64, n: usize, rows: usize, heights: usize) -> ChainProblem {
        let mut state = seed;
        let mut table = |size| {
            (0..size)
                .map(|_| {
                    state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                    // Binary fractions make every addition exact; forbidden states
                    // exercise infinities without relying on float tolerances.
                    if state.is_multiple_of(29) {
                        f64::INFINITY
                    } else {
                        ((state >> 32) % 25) as f64 / 8.0 - 1.5
                    }
                })
                .collect::<Vec<_>>()
        };
        ChainProblem {
            rows,
            min_height: 2,
            height_count: heights,
            min_gap: 1,
            gap_count: 2,
            unary: (0..n).map(|_| table(rows * heights)).collect(),
            ratio: (1..n).map(|_| table(heights * heights)).collect(),
            disc: (1..n).map(|_| table(2 * heights)).collect(),
            gap: table(rows * 2),
            start: table(rows * heights),
            end: table(rows * heights),
        }
    }

    /// Independent, unsplit recurrence; visits every predecessor pair.
    fn direct(p: &ChainProblem) -> (f64, Vec<BodyState>) {
        let h = p.height_count;
        let mut messages = vec![vec![f64::INFINITY; p.state_count()]; p.unary.len()];
        let mut parents = vec![vec![usize::MAX; p.state_count()]; p.unary.len()];
        for (i, value) in messages[0].iter_mut().enumerate() {
            if p.valid_state(i) {
                *value = p.start[i] + p.unary[0][i];
            }
        }
        for v in 1..p.unary.len() {
            for j in 0..p.state_count() {
                if !p.valid_state(j) {
                    continue;
                }
                let lower = p.state(j);
                let mut best = f64::INFINITY;
                for (i, &prefix) in messages[v - 1].iter().enumerate() {
                    let upper = p.state(i);
                    let Some(gap) = lower.top.checked_sub(upper.top + upper.height) else {
                        continue;
                    };
                    if gap < p.min_gap || gap >= p.min_gap + p.gap_count {
                        continue;
                    }
                    let di = gap - p.min_gap;
                    let candidate = ((prefix + p.ratio[v - 1][(i % h) * h + j % h])
                        + p.gap[(upper.top + upper.height) * p.gap_count + di])
                        + p.disc[v - 1][di * h + j % h];
                    if candidate < best {
                        best = candidate;
                        parents[v][j] = i;
                    }
                }
                messages[v][j] = best + p.unary[v][j];
            }
        }
        let last = p.unary.len() - 1;
        let mut best = f64::INFINITY;
        let mut index = 0;
        for (i, &prefix) in messages[last].iter().enumerate() {
            let cost = prefix + p.end[i];
            if cost < best {
                best = cost;
                index = i;
            }
        }
        if !best.is_finite() {
            return (best, Vec::new());
        }
        let mut states = Vec::new();
        for v in (0..=last).rev() {
            states.push(p.state(index));
            if v > 0 {
                index = parents[v][index];
            }
        }
        states.reverse();
        (best, states)
    }

    fn enumerate(
        p: &ChainProblem,
        path: &mut Vec<usize>,
        energy: f64,
        paths: &mut Vec<(Vec<usize>, f64)>,
    ) {
        let v = path.len();
        if v == p.unary.len() {
            let energy = energy + p.end[*path.last().unwrap()];
            if energy.is_finite() {
                paths.push((path.clone(), energy));
            }
            return;
        }
        for i in 0..p.state_count() {
            if !p.valid_state(i) {
                continue;
            }
            let mut cost = p.unary[v][i];
            if let Some(&previous) = path.last() {
                let a = p.state(previous);
                let b = p.state(i);
                let Some(gap) = b.top.checked_sub(a.top + a.height) else {
                    continue;
                };
                if gap < p.min_gap || gap >= p.min_gap + p.gap_count {
                    continue;
                }
                let h = p.height_count;
                let di = gap - p.min_gap;
                cost += p.ratio[v - 1][(previous % h) * h + i % h]
                    + p.disc[v - 1][di * h + i % h]
                    + p.gap[(a.top + a.height) * p.gap_count + di];
            } else {
                cost += p.start[i];
            }
            if (energy + cost).is_finite() {
                path.push(i);
                enumerate(p, path, energy + cost, paths);
                path.pop();
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(60))]
        #[test]
        fn split_search_and_posterior_match_enumeration(seed in 0u64..100000) {
            let p=problem(seed,3,15,2);
            let (energy,states)=direct(&p);
            let solved=solve(&p).unwrap();
            prop_assert_eq!(solved.energy,energy);
            prop_assert_eq!(&solved.states,&states);
            let mut paths=Vec::new();
            enumerate(&p,&mut Vec::new(),0.0,&mut paths);
            let z:f64=paths.iter().map(|(_,e)| libm::exp(-e)).sum();
            let posterior=infer(&p,&states,1.0,1.0).unwrap();
            // Every path was enumerated independently. 1e-10 covers the
            // different order of f64 sums, not a placement tolerance.
            prop_assert!((posterior.log_partition-libm::log(z)).abs()<1e-10);
            for v in 0..3 {
                for i in 0..p.state_count() {
                    let expected:f64=paths.iter().filter(|(path,_)| path[v]==i).map(|(_,e)| libm::exp(-e)/z).sum();
                    prop_assert!((expected-posterior.probabilities[v][i]).abs()<1e-10);
                    let minimum=paths.iter().filter(|(path,_)| path[v]==i).map(|(_,e)| *e).fold(f64::INFINITY,f64::min);
                    prop_assert_eq!(minimum,posterior.min_marginals[v][i]);
                }
            }
        }
    }

    #[test]
    fn full_chain_split_matches_direct_and_checkpoints_do_not_depend_on_batching() {
        for seed in 0..5 {
            let p = problem(seed, 18, 90, 4);
            let solved = solve(&p).unwrap();
            let expected = direct(&p);
            assert_eq!((solved.energy, solved.states.clone()), expected);
            let mut stepped = ForwardSweep::new(&p).unwrap();
            for _ in 0..6 {
                for _ in 0..3 {
                    assert!(stepped.advance().is_some());
                }
            }
            assert!(stepped.advance().is_none());
            let stepped = stepped.finish().unwrap();
            assert_eq!(stepped.states, solved.states);
            assert_eq!(stepped.sweeps, solved.sweeps);
            assert_eq!(solved.sweeps.len(), 18);
        }
    }

    #[test]
    fn impossible_and_malformed_problems_return_errors() {
        let mut p = problem(7, 3, 15, 2);
        p.unary[1].fill(f64::INFINITY);
        assert_eq!(
            solve(&p).unwrap_err().code,
            ErrorCode::ToolConstraintsConflict
        );
        p.unary[1][0] = f64::NAN;
        assert_eq!(solve(&p).unwrap_err().code, ErrorCode::EngineOutOfSync);
    }
}
