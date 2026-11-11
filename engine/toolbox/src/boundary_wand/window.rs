//! Joint endplate placement in a bounded window when a parent cannot be
//! split locally. One dynamic program places every boundary together.

use super::geometry::*;
use super::refused;
use super::split::{Sizes, gap_cost, height_cost, pieces, sample_cost};
use auto_pass::handoffs::FilmPassCache;
use film_reader::decoded_film::DecodedFilm;
use shapes::confidence::ConfidenceReason;
use shapes::errors::{EngineError, ErrorCode};
use shapes::marks::{MarkSource, PedicleMark};
use shapes::pixels::Px;
use shapes::vertebra::{FilmMarks, VertebraMarks};

struct Axis {
    points: Vec<Px>,
    lengths: Vec<f64>,
}
impl Axis {
    fn new(points: Vec<Px>) -> Self {
        let mut lengths = vec![0.0];
        for pair in points.windows(2) {
            lengths.push(lengths.last().copied().unwrap_or(0.0) + length(sub(pair[1], pair[0])));
        }
        Self { points, lengths }
    }
    fn at(&self, s: f64) -> (Px, Px) {
        let i = self
            .lengths
            .partition_point(|x| *x <= s)
            .saturating_sub(1)
            .min(self.points.len() - 2);
        let direction = unit(sub(self.points[i + 1], self.points[i])).unwrap_or(Px::new(0.0, 1.0));
        (
            add(self.points[i], scale(direction, s - self.lengths[i])),
            direction,
        )
    }
    fn project(&self, p: Px) -> f64 {
        let mut best = (f64::INFINITY, 0.0);
        for i in 0..self.points.len() - 1 {
            let (a, b) = (self.points[i], self.points[i + 1]);
            let d = sub(b, a);
            let d2 = dot(d, d);
            if d2 <= 1e-12 {
                continue;
            }
            let t = (dot(sub(p, a), d) / d2).clamp(0.0, 1.0);
            let distance = length(sub(p, lerp(a, b, t)));
            if distance < best.0 {
                best = (
                    distance,
                    self.lengths[i] + t * (self.lengths[i + 1] - self.lengths[i]),
                );
            }
        }
        best.1
    }
}

pub(super) fn split(
    film: &DecodedFilm,
    cache: Option<&FilmPassCache>,
    marks: &mut FilmMarks,
    index: usize,
    at: Px,
    new_id: u32,
    sizes: Sizes,
) -> Result<[Px; 2], EngineError> {
    for radius in 1..=2 {
        let mut lo = index;
        let mut hi = index + 1;
        while lo > index.saturating_sub(radius)
            && !marks.vertebrae[lo - 1]
                .marks
                .corners()
                .iter()
                .any(|m| m.pinned)
        {
            lo -= 1;
        }
        while hi < (index + radius + 1).min(marks.vertebrae.len())
            && !marks.vertebrae[hi].marks.corners().iter().any(|m| m.pinned)
        {
            hi += 1;
        }
        if let Some((mut candidate, line)) =
            solve(film, cache, marks, index, at, new_id, sizes, lo, hi)?
        {
            std::mem::swap(marks, &mut candidate);
            return Ok(line);
        }
    }
    Err(EngineError::new(
        ErrorCode::ToolSplitInfeasible,
        "Unpin nearby corners or edit closer to the problem",
    )
    .with_detail("rechain-infeasible"))
}

#[allow(clippy::too_many_arguments)]
fn solve(
    film: &DecodedFilm,
    cache: Option<&FilmPassCache>,
    marks: &FilmMarks,
    index: usize,
    at: Px,
    new_id: u32,
    sizes: Sizes,
    lo: usize,
    hi: usize,
) -> Result<Option<(FilmMarks, [Px; 2])>, EngineError> {
    let mut points = Vec::new();
    if lo > 0 {
        let q = quad(&marks.vertebrae[lo - 1].marks);
        points.push(midpoint(q[3], q[2]));
    }
    for body in &marks.vertebrae[lo..hi] {
        let q = quad(&body.marks);
        points.extend([midpoint(q[0], q[1]), midpoint(q[3], q[2])]);
    }
    if hi < marks.vertebrae.len() {
        let q = quad(&marks.vertebrae[hi].marks);
        points.push(midpoint(q[0], q[1]));
    }
    let axis = Axis::new(points);
    let step = sizes.height / 50.0;
    let first = quad(&marks.vertebrae[lo].marks);
    let last = quad(&marks.vertebrae[hi - 1].marks);
    let top = axis.project(midpoint(first[0], first[1]));
    let bottom = axis.project(midpoint(last[3], last[2]));
    let origin = if lo == 0 {
        top - 0.25 * sizes.height
    } else {
        0.0
    };
    let end = if hi == marks.vertebrae.len() {
        bottom + 0.25 * sizes.height
    } else {
        *axis.lengths.last().unwrap_or(&bottom)
    };
    let count = ((end - origin) / step).ceil() as usize + 1;
    // Bounds on the window geometry also bound its memory and work.
    if !(3..=1024).contains(&count) {
        return Ok(None);
    }
    let width = marks.vertebrae[lo..hi]
        .iter()
        .map(|v| width(quad(&v.marks)))
        .sum::<f64>()
        / (hi - lo) as f64;
    let profile: Vec<_> = (0..count)
        .map(|p| {
            let (point, u) = axis.at(origin + p as f64 * step);
            (point, Px::new(-u.y(), u.x()), width)
        })
        .collect();
    let costs = sample_cost(film, cache, &profile, step);
    let desired = axis.project(at);
    let output_bodies = super::count_after(hi - lo, shapes::toolbox::BoundaryAction::Split);
    let steps = 2 * output_bodies;
    let mut previous = vec![[f64::INFINITY; 2]; count];
    let mut trace = vec![vec![[None; 2]; count]; steps];
    for (p, cost) in costs.iter().enumerate() {
        let s = origin + p as f64 * step;
        if lo == 0 && (s - top).abs() <= 0.25 * sizes.height {
            previous[p][0] = *cost;
        }
        if lo > 0 && (0.05 * sizes.height..=0.8 * sizes.height).contains(&s) {
            previous[p][0] = cost + gap_cost(s, sizes);
        }
    }
    for (j, trace_row) in trace.iter_mut().enumerate().skip(1) {
        let disc = j.is_multiple_of(2);
        let (minimum, maximum) = if disc { (0.05, 0.8) } else { (0.35, 1.6) };
        let min_steps = (minimum * sizes.height / step).ceil() as usize;
        let max_steps = (maximum * sizes.height / step).floor() as usize;
        let mut next = vec![[f64::INFINITY; 2]; count];
        for p in 0..count {
            let s = origin + p as f64 * step;
            for (q, previous_state) in previous
                .iter()
                .enumerate()
                .take(p.saturating_sub(min_steps) + 1)
                .skip(p.saturating_sub(max_steps))
            {
                if p - q < min_steps {
                    continue;
                }
                let d = (p - q) as f64 * step;
                let forced = disc
                    && ((origin + q as f64 * step + s) * 0.5 - desired).abs() <= 0.2 * sizes.height;
                // The new gap belongs between the two parent pieces.
                if forced && j != 2 * (index - lo + 1) {
                    continue;
                }
                let cost = costs[p]
                    + if disc {
                        gap_cost(d, sizes)
                    } else {
                        height_cost(d, sizes)
                    };
                for (flag, previous_cost) in previous_state.iter().enumerate() {
                    if forced && flag == 1 {
                        continue;
                    }
                    let new_flag = if forced { 1 } else { flag };
                    let candidate = previous_cost + cost;
                    if candidate < next[p][new_flag] {
                        next[p][new_flag] = candidate;
                        trace_row[p][new_flag] = Some((q, flag));
                    }
                }
            }
        }
        previous = next;
    }
    let mut best: Option<(f64, usize)> = None;
    for (p, state) in previous.iter().enumerate() {
        let s = origin + p as f64 * step;
        let allowed = if hi == marks.vertebrae.len() {
            (s - bottom).abs() <= 0.25 * sizes.height
        } else {
            (0.05 * sizes.height..=0.8 * sizes.height).contains(&(end - s))
        };
        let cost = state[1]
            + if hi == marks.vertebrae.len() {
                0.0
            } else {
                gap_cost(end - s, sizes)
            };
        if allowed && cost.is_finite() && best.is_none_or(|(old, _)| cost < old) {
            best = Some((cost, p));
        }
    }
    let Some((_, mut p)) = best else {
        return Ok(None);
    };
    let mut flag = 1;
    let mut positions = vec![0.0; steps];
    for j in (0..steps).rev() {
        positions[j] = origin + p as f64 * step;
        if j > 0 {
            let Some((q, f)) = trace[j][p][flag] else {
                return Ok(None);
            };
            p = q;
            flag = f;
        }
    }
    let mut answer = marks.clone();
    for i in lo..hi {
        let original = &marks.vertebrae[i];
        let body = &mut answer.vertebrae[i];
        let old = original.marks.corners();
        let q = old.map(|m| m.at);
        let k = i - lo + usize::from(i > index);
        let target_top = positions[2 * k];
        let target_bottom = positions[2 * k + if i == index { 3 } else { 1 }];
        let original_top = axis.project(midpoint(q[0], q[1]));
        let original_bottom = axis.project(midpoint(q[3], q[2]));
        let top_move = scale(axis.at(original_top).1, target_top - original_top);
        let bottom_move = scale(axis.at(original_bottom).1, target_bottom - original_bottom);
        let mut corners = old;
        for (c, mark) in corners.iter_mut().enumerate() {
            if !mark.pinned {
                mark.at = add(mark.at, if c < 2 { top_move } else { bottom_move });
                if mark.at != old[c].at {
                    mark.source = MarkSource::BoundaryWand;
                }
            }
        }
        if !valid(corners.map(|m| m.at)) {
            return Ok(None);
        }
        put_corners(&mut body.marks, corners);
        if corners != old {
            body.outline = None;
            body.confidence
                .reasons
                .retain(|r| *r != ConfidenceReason::EditedByUser);
            if let VertebraMarks::Front { points } = &mut body.marks {
                for pedicle in [&mut points.ped_l, &mut points.ped_r] {
                    if let PedicleMark::Visible {
                        at,
                        pinned: false,
                        inferred,
                        source,
                        ..
                    } = pedicle
                    {
                        *at = add(*at, midpoint(top_move, bottom_move));
                        *inferred = true;
                        *source = MarkSource::BoundaryWand;
                    }
                }
            }
        }
    }
    let k = index - lo;
    let parent = quad(&answer.vertebrae[index].marks);
    let parent_top = midpoint(parent[0], parent[1]);
    let u = unit(sub(midpoint(parent[3], parent[2]), parent_top))
        .ok_or_else(|| refused("split-invalid", "The outline has no height"))?;
    let e1 = dot(sub(axis.at(positions[2 * k + 1]).0, parent_top), u);
    let e2 = dot(sub(axis.at(positions[2 * k + 2]).0, parent_top), u);
    let line = match pieces(&mut answer, index, new_id, e1, e2) {
        Ok(line) => line,
        Err(_) => return Ok(None),
    };
    Ok(Some((answer, line)))
}
