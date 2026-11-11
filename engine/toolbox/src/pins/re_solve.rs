//! A local linear similarity/disc fit around an exact pin.
//!
//! The shape term follows Igarashi et al., SIGGRAPH 2005: each body's
//! unpinned corners follow a fitted similarity, with disc-vector and
//! stay-put terms coupling neighbouring bodies. All terms are linear;
//! pinned coordinates are eliminated before a dense Cholesky solve.
//!
//! The target's other corners participate, with the first-ring stay-put
//! weight, so the body as well as its neighbours can respond to the pin.
//! The initial two-neighbour window grows to at most four if motion at
//! its edge exceeds 0.04 body heights or 1 mm. These weights are
//! experimental. There is no image attraction in this fit.
//!
//! A rejected numerical solve falls back to the already validated direct
//! pin. A geometric guard shortens only the unpinned response, never the
//! user's pinned position. Thus every successful answer holds all pins
//! exactly, even at the convexity limit.

use super::{geometry, pedicles};
use nalgebra::{DMatrix, DVector};
use shapes::errors::{EngineError, ErrorCode};
use shapes::film_geometry::FilmGeometry;
use shapes::marks::{Mark, MarkSource};
use shapes::pixels::Px;
use shapes::vertebra::{FilmMarks, VertebraMarks};

const REGULARISER: f64 = 1e-6;

#[derive(Clone)]
struct Body {
    before: [Px; 4],
    held: [Px; 4],
    coordinates: [Option<usize>; 8],
    similarity: Option<[usize; 4]>,
    height: f64,
}

/// Refit the target and its neighbours, using the directly applied pin
/// in `after` as the fixed condition.
pub(super) fn around(
    before: &FilmMarks,
    after: &mut FilmMarks,
    target: usize,
    geometry: Option<&FilmGeometry>,
) -> Result<(), EngineError> {
    let original: Vec<_> = before
        .vertebrae
        .iter()
        .map(|v| v.marks.corners().map(|m| m.at))
        .collect();
    let direct: Vec<_> = after
        .vertebrae
        .iter()
        .map(|v| v.marks.corners().map(|m| m.at))
        .collect();
    let start = target.saturating_sub(4);
    let end = (target + 5).min(original.len());
    if original[start..end]
        .iter()
        .any(|quad| !geometry::valid(*quad, *quad))
    {
        return Err(EngineError::new(
            ErrorCode::ToolPinInvalid,
            "the pin's neighbourhood contains an invalid outline",
        ));
    }
    let mut fitted = direct.clone();
    for radius in 2..=4 {
        let Some(solution) = solve(before, after, target, radius) else {
            break;
        };
        let spread = edge_spread(&original, &solution, target, radius, geometry);
        fitted = solution;
        if !spread {
            break;
        }
    }
    if !fits(&fitted, &original) {
        let (mut lo, mut hi) = (0.0, 1.0);
        for _ in 0..8 {
            let mid = 0.5 * (lo + hi);
            if fits(&blend(&direct, &fitted, mid), &original) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        fitted = blend(&direct, &fitted, lo);
    }
    for (index, vertebra) in after.vertebrae.iter_mut().enumerate() {
        let old = original[index];
        let next = fitted[index];
        let mut corners = vertebra.marks.corners();
        for corner in 0..4 {
            // Elimination already holds these. Skipping the assignment also
            // retains their exact bits, source and inference state.
            if !corners[corner].pinned && geometry::distance(next[corner], old[corner]) > 1e-10 {
                corners[corner].at = next[corner];
                corners[corner].source = MarkSource::Auto;
                corners[corner].inferred = true;
            }
        }
        set_corners(&mut vertebra.marks, corners);
        let next = vertebra.marks.corners().map(|m| m.at);
        if old != next {
            vertebra.outline = None;
            if let VertebraMarks::Front { points } = &mut vertebra.marks {
                pedicles::transfer(&mut points.ped_l, old, next);
                pedicles::transfer(&mut points.ped_r, old, next);
            }
        }
    }
    Ok(())
}

fn set_corners(marks: &mut VertebraMarks, q: [Mark; 4]) {
    match marks {
        VertebraMarks::Front { points } => {
            points.sup_l = q[0];
            points.sup_r = q[1];
            points.inf_r = q[2];
            points.inf_l = q[3];
        }
        VertebraMarks::Side { points } => {
            points.sup_a = q[0];
            points.sup_p = q[1];
            points.inf_p = q[2];
            points.inf_a = q[3];
        }
        VertebraMarks::Bending { lines } => {
            lines.sup = [q[0], q[1]];
            lines.inf = [q[3], q[2]];
        }
    }
}

fn fits(candidate: &[[Px; 4]], original: &[[Px; 4]]) -> bool {
    candidate
        .iter()
        .zip(original)
        .all(|(new, old)| new == old || geometry::valid(*new, *old))
}

fn blend(direct: &[[Px; 4]], fitted: &[[Px; 4]], fraction: f64) -> Vec<[Px; 4]> {
    direct
        .iter()
        .zip(fitted)
        .map(|(base, next)| {
            std::array::from_fn(|i| {
                Px::new(
                    base[i].x() + fraction * (next[i].x() - base[i].x()),
                    base[i].y() + fraction * (next[i].y() - base[i].y()),
                )
            })
        })
        .collect()
}

fn edge_spread(
    old: &[[Px; 4]],
    new: &[[Px; 4]],
    target: usize,
    radius: usize,
    geometry: Option<&FilmGeometry>,
) -> bool {
    let mut edge_heights = Vec::new();
    let mut displacement = 0.0f64;
    for index in [
        target.checked_sub(radius),
        target.checked_add(radius).filter(|i| *i < old.len()),
    ]
    .into_iter()
    .flatten()
    {
        edge_heights.push(geometry::height(old[index]));
        for c in 0..4 {
            displacement = displacement.max(geometry::distance(old[index][c], new[index][c]));
        }
    }
    if edge_heights.is_empty() {
        return false;
    }
    let relative = 0.04 * edge_heights.iter().sum::<f64>() / edge_heights.len() as f64;
    let millimetre = geometry
        .and_then(|g| {
            g.mm_per_px
                .map(|sx| sx.min(g.mm_per_px_vertical.unwrap_or(sx)))
        })
        .filter(|spacing| spacing.is_finite() && *spacing > 0.0)
        .map_or(0.0, |spacing| 1.0 / spacing);
    displacement > relative.max(millimetre)
}

fn solve(
    before: &FilmMarks,
    after: &FilmMarks,
    target: usize,
    radius: usize,
) -> Option<Vec<[Px; 4]>> {
    let start = target.saturating_sub(radius);
    let end = (target + radius + 1).min(before.vertebrae.len());
    let mut count = 0;
    let bodies: Vec<_> = before
        .vertebrae
        .iter()
        .zip(&after.vertebrae)
        .enumerate()
        .map(|(i, (old, new))| {
            let old = old.marks.corners().map(|m| m.at);
            let new = new.marks.corners();
            let active = (start..end).contains(&i);
            let coordinates = std::array::from_fn(|axis| {
                if active && !new[axis / 2].pinned {
                    let index = count;
                    count += 1;
                    Some(index)
                } else {
                    None
                }
            });
            let similarity = active.then(|| {
                let base = count;
                count += 4;
                [base, base + 1, base + 2, base + 3]
            });
            Body {
                before: old,
                held: new.map(|m| m.at),
                coordinates,
                similarity,
                height: geometry::height(old),
            }
        })
        .collect();
    let mut equations = NormalEquations::new(count);
    for (i, body) in bodies.iter().enumerate().take(end).skip(start) {
        let weight = 1.0 / (body.height * body.height);
        let stay = if i.abs_diff(target) <= 1 { 0.05 } else { 0.5 };
        for corner in 0..4 {
            for axis in 0..2 {
                let mut shape = Row::default();
                shape.coordinate(body, corner, axis, 1.0);
                shape.similarity(body, body.before[corner], axis, -1.0, true);
                equations.add(&shape, weight);
                let mut stationary = Row::default();
                stationary.coordinate(body, corner, axis, 1.0);
                equations.add(&stationary, stay * weight);
            }
        }
    }
    // Include the two gaps to fixed vertebrae outside the active window.
    for upper in start.saturating_sub(1)..end.min(bodies.len().saturating_sub(1)) {
        let lower = upper + 1;
        let mean_height = 0.5 * (bodies[upper].height + bodies[lower].height);
        for (inf, sup) in [(3, 0), (2, 1)] {
            let a = bodies[upper].before[inf];
            let b = bodies[lower].before[sup];
            let gap = Px::new(b.x() - a.x(), b.y() - a.y());
            for axis in 0..2 {
                let mut disc = Row::default();
                disc.coordinate(&bodies[lower], sup, axis, 1.0);
                disc.coordinate(&bodies[upper], inf, axis, -1.0);
                disc.similarity(&bodies[upper], gap, axis, -0.5, false);
                disc.similarity(&bodies[lower], gap, axis, -0.5, false);
                equations.add(&disc, 0.5 / (mean_height * mean_height));
            }
        }
    }
    let solution = equations.matrix.cholesky()?.solve(&equations.rhs);
    if !solution.iter().all(|x| x.is_finite()) {
        return None;
    }
    Some(
        bodies
            .iter()
            .map(|body| {
                std::array::from_fn(|corner| {
                    Px::new(
                        body.coordinates[2 * corner].map_or(body.held[corner].x(), |i| {
                            body.before[corner].x() + solution[i]
                        }),
                        body.coordinates[2 * corner + 1].map_or(body.held[corner].y(), |i| {
                            body.before[corner].y() + solution[i]
                        }),
                    )
                })
            })
            .collect(),
    )
}

#[derive(Default)]
struct Row {
    terms: Vec<(usize, f64)>,
    constant: f64,
}

impl Row {
    fn coordinate(&mut self, body: &Body, corner: usize, axis: usize, coefficient: f64) {
        if let Some(index) = body.coordinates[2 * corner + axis] {
            self.terms.push((index, coefficient));
        } else {
            self.constant +=
                coefficient * (body.held[corner].0[axis] - body.before[corner].0[axis]);
        }
    }

    fn similarity(
        &mut self,
        body: &Body,
        point: Px,
        axis: usize,
        coefficient: f64,
        translate: bool,
    ) {
        if let Some([a, b, tx, ty]) = body.similarity {
            let (scale, rotation, translation) = if axis == 0 {
                (point.x(), -point.y(), tx)
            } else {
                (point.y(), point.x(), ty)
            };
            self.terms.push((a, coefficient * scale));
            self.terms.push((b, coefficient * rotation));
            if translate {
                self.terms.push((translation, coefficient));
            }
        }
    }
}

struct NormalEquations {
    matrix: DMatrix<f64>,
    rhs: DVector<f64>,
}

impl NormalEquations {
    fn new(count: usize) -> Self {
        Self {
            matrix: DMatrix::identity(count, count) * REGULARISER,
            rhs: DVector::zeros(count),
        }
    }

    fn add(&mut self, row: &Row, weight: f64) {
        for &(i, a) in &row.terms {
            self.rhs[i] -= weight * a * row.constant;
            for &(j, b) in &row.terms {
                self.matrix[(i, j)] += weight * a * b;
            }
        }
    }
}
