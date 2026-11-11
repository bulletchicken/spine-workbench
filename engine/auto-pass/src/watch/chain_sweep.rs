//! An owned forward sweep, pausing inside each body's message tables.
//! The arithmetic and tie order are the exact split recurrence used by
//! `chain_search::dynamic_program`; only the storage lifetime differs.

use crate::chain_search::dynamic_program::{BodyState, ChainProblem, Solution, SweepCheckpoint};
use shapes::errors::{EngineError, ErrorCode};

const ROWS_PER_UNIT: usize = 16;

pub(super) struct Sweep {
    problem: ChainProblem,
    forward: Vec<Vec<f64>>,
    parents: Vec<Vec<usize>>,
    checkpoints: Vec<SweepCheckpoint>,
    next: Vec<f64>,
    parent: Vec<usize>,
    q: Vec<f64>,
    q_parent: Vec<usize>,
    row: usize,
    ratios: bool,
}

impl Sweep {
    pub fn new(problem: ChainProblem) -> Result<Self, EngineError> {
        problem.validate()?;
        let count = problem.state_count();
        Ok(Self {
            problem,
            forward: Vec::new(),
            parents: Vec::new(),
            checkpoints: Vec::new(),
            next: vec![f64::INFINITY; count],
            parent: vec![usize::MAX; count],
            q: vec![f64::INFINITY; count],
            q_parent: vec![usize::MAX; count],
            row: 0,
            ratios: false,
        })
    }

    pub fn done(&self) -> bool {
        self.forward.len() == self.problem.unary.len()
    }

    pub fn fraction(&self) -> f64 {
        let within = self.row as f64 / self.problem.rows as f64;
        let within = if self.forward.is_empty() {
            within
        } else if self.ratios {
            0.5 * within
        } else {
            0.5 + 0.5 * within
        };
        ((self.forward.len() as f64 + within) / self.problem.unary.len() as f64).min(1.0)
    }

    /// One bounded row block. A checkpoint exists only after a complete
    /// body's forward message, regardless of the caller's batching.
    pub fn advance(&mut self) -> Option<SweepCheckpoint> {
        if self.done() {
            return None;
        }
        let p = &self.problem;
        let v = self.forward.len();
        let h_count = p.height_count;
        let end = (self.row + ROWS_PER_UNIT).min(p.rows);
        if self.ratios {
            for bot in self.row.max(p.min_height)..end {
                for lower_h in 0..h_count {
                    let qi = bot * h_count + lower_h;
                    for upper_h in (0..h_count).rev() {
                        let height = p.min_height + upper_h;
                        if bot < height {
                            continue;
                        }
                        let previous = (bot - height) * h_count + upper_h;
                        let cost = self.forward[v - 1][previous]
                            + p.ratio[v - 1][upper_h * h_count + lower_h];
                        if cost < self.q[qi] {
                            self.q[qi] = cost;
                            self.q_parent[qi] = previous;
                        }
                    }
                }
            }
        } else {
            for i in self.row * h_count..end * h_count {
                if !p.valid_state(i) {
                    continue;
                }
                if v == 0 {
                    self.next[i] = p.start[i] + p.unary[0][i];
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
                    let candidate = (self.q[qi] + p.gap[bot * p.gap_count + di])
                        + p.disc[v - 1][di * h_count + hi];
                    if candidate < best || (candidate == best && self.q_parent[qi] < self.parent[i])
                    {
                        best = candidate;
                        self.parent[i] = self.q_parent[qi];
                    }
                }
                self.next[i] = best + p.unary[v][i];
            }
        }
        self.row = end;
        if end != p.rows {
            return None;
        }
        self.row = 0;
        if self.ratios {
            self.ratios = false;
            return None;
        }
        let mut boundary_energy = vec![f64::INFINITY; p.rows];
        let mut best = None;
        for (i, &cost) in self.next.iter().enumerate() {
            if !cost.is_finite() {
                continue;
            }
            let state = p.state(i);
            let bot = state.top + state.height;
            boundary_energy[bot] = boundary_energy[bot].min(cost);
            if best.is_none_or(|j| cost < self.next[j]) {
                best = Some(i);
            }
        }
        self.parents.push(std::mem::replace(
            &mut self.parent,
            vec![usize::MAX; p.state_count()],
        ));
        let path = best.map(|i| self.backtrack(v, i)).unwrap_or_default();
        self.forward.push(std::mem::replace(
            &mut self.next,
            vec![f64::INFINITY; self.problem.state_count()],
        ));
        let checkpoint = SweepCheckpoint {
            body: v,
            path,
            boundary_energy,
        };
        self.checkpoints.push(checkpoint.clone());
        self.q.fill(f64::INFINITY);
        self.q_parent.fill(usize::MAX);
        self.ratios = true;
        Some(checkpoint)
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

    pub fn finish(self) -> Result<(ChainProblem, Solution), EngineError> {
        if !self.done() {
            return Err(EngineError::new(
                ErrorCode::EngineOutOfSync,
                "chain sweep is unfinished",
            ));
        }
        let p = &self.problem;
        let last = self.forward.last().expect("a validated chain is nonempty");
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
                ErrorCode::PassNoSpineFound,
                "no finite chain was found",
            ));
        }
        let states = self.backtrack(p.unary.len() - 1, best);
        Ok((
            self.problem,
            Solution {
                states,
                energy,
                sweeps: self.checkpoints,
                forward: self.forward,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain_search::dynamic_program;

    #[test]
    fn bounded_rows_keep_the_exact_known_chain_and_prefixes() {
        // Two two-cell bodies separated by one cell. Rewards fix their
        // tops at 3 and 6; every other placement costs at least 10 more.
        let rows = 40;
        let mut unary = vec![vec![10.0; rows]; 2];
        unary[0][3] = -2.0;
        unary[1][6] = -5.0;
        let p = ChainProblem {
            rows,
            min_height: 2,
            height_count: 1,
            min_gap: 1,
            gap_count: 1,
            unary,
            ratio: vec![vec![0.0]],
            disc: vec![vec![0.0]],
            gap: vec![0.0; rows],
            start: vec![0.0; rows],
            end: vec![0.0; rows],
        };
        let direct = dynamic_program::solve(&p).unwrap();
        let mut bounded = Sweep::new(p).unwrap();
        assert!(bounded.advance().is_none());
        assert!(bounded.fraction() > 0.0);
        let mut count = 0;
        while !bounded.done() {
            count += usize::from(bounded.advance().is_some());
        }
        let (_, answer) = bounded.finish().unwrap();
        assert_eq!(count, 2);
        assert_eq!(answer.energy, -7.0);
        assert_eq!(
            answer.states,
            [
                BodyState { top: 3, height: 2 },
                BodyState { top: 6, height: 2 }
            ]
        );
        assert_eq!(answer.sweeps, direct.sweeps);
        assert_eq!(answer.forward, direct.forward);
    }
}
