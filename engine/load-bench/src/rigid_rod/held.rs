//! The sliding, freely swivelling held-state minimum.
use super::curve::Curve;
use super::minimise::{self, Evaluation, FirstOrder, Model, Options};
use super::twist::SwingTwist;
use super::{Construct, HeldGuess, HeldOptions, HeldState, LENGTH_MM, SolveError, StopReason};
use crate::chain::{Chain, Pose, Wrench};
use forward_model::rotation::matrix_from_rotation_vector;
use nalgebra::{DMatrix, DVector, Matrix3, Vector3};

/// Project proximity for refusing a free head at the admitted material end,
/// including an otherwise numerically settled state. This is not an end stop.
const CONTOUR_END_PROXIMITY_MM: f64 = 1e-6;
/// Project numerical threshold for fitting a gapped starting iterate's head
/// line and midpoint. This changes no constraint or final acceptance test.
const INITIAL_HEAD_POINT_FIT_GAP_MM: f64 = 1e-6;

pub(super) struct HeldModel<'a> {
    chain: Chain,
    construct: &'a Construct,
    curves: Vec<Curve>,
    rod_offsets: Vec<Option<usize>>,
    slide_offsets: Vec<Option<usize>>,
    rolls: Vec<Option<usize>>,
    pub pinned: Vec<usize>,
    dimension: usize,
    constraints: usize,
    end_continuation_mm: f64,
}

fn symmetric_arm(arm: Vector3<f64>, force: Vector3<f64>) -> Matrix3<f64> {
    (arm * force.transpose() + force * arm.transpose()) * 0.5
        - Matrix3::identity() * arm.dot(&force)
}

impl<'a> HeldModel<'a> {
    fn new(
        chain: &Chain,
        construct: &'a Construct,
        options: &HeldOptions,
    ) -> Result<Self, SolveError> {
        if !options.end_continuation_mm.is_finite() || options.end_continuation_mm < 0.0 {
            return Err(SolveError::InvalidInput("end continuation"));
        }
        if construct.rods.is_empty() || construct.anchors.is_empty() {
            return Err(SolveError::InvalidInput("empty construct"));
        }
        let curves = construct
            .rods
            .iter()
            .map(Curve::new)
            .collect::<Result<Vec<_>, _>>()?;
        let mut pinned = Vec::new();
        for (rod, curve) in curves.iter().enumerate() {
            let heads: Vec<_> = construct
                .anchors
                .iter()
                .enumerate()
                .filter(|(_, a)| a.rod == rod)
                .collect();
            if heads.len() < 2 || heads.iter().all(|(_, a)| a.body == heads[0].1.body) {
                return Err(SolveError::SingularConstruct(
                    "rod needs heads on two bodies",
                ));
            }
            let bottom = heads
                .iter()
                .max_by(|(i, a), (j, b)| a.body.cmp(&b.body).then(i.cmp(j)))
                .unwrap()
                .0;
            pinned.push(bottom);
            for (_, a) in heads {
                if a.body >= chain.reference.len()
                    || a.point_mm.iter().any(|x| !x.is_finite())
                    || !a.station_mm.is_finite()
                    || a.station_mm < 0.0
                    || a.station_mm > curve.length()
                {
                    return Err(SolveError::InvalidInput("anchor"));
                }
            }
        }
        if construct.anchors.iter().any(|a| a.rod >= curves.len()) {
            return Err(SolveError::InvalidInput("anchor rod"));
        }
        let mut dimension = chain.reference.len() * 6;
        let rod_offsets = curves
            .iter()
            .map(|c| {
                if options.fixed_rods {
                    None
                } else {
                    let i = dimension;
                    dimension += if c.is_curved { 6 } else { 5 };
                    Some(i)
                }
            })
            .collect();
        let slide_offsets = construct
            .anchors
            .iter()
            .enumerate()
            .map(|(i, _)| {
                if options.fixed_stations || pinned.contains(&i) {
                    None
                } else {
                    let j = dimension;
                    dimension += 1;
                    Some(j)
                }
            })
            .collect();
        let mut constraints = 3 * construct.anchors.len();
        let rolls = curves
            .iter()
            .map(|c| {
                if options.fixed_rods || !c.is_curved {
                    None
                } else {
                    let i = constraints;
                    constraints += 1;
                    Some(i)
                }
            })
            .collect();
        let normalised = chain
            .scaled(1.0 / chain.stiffness_scale())
            .map_err(|_| SolveError::InvalidInput("chain scale"))?;
        Ok(Self {
            chain: normalised,
            construct,
            curves,
            rod_offsets,
            slide_offsets,
            rolls,
            pinned,
            dimension,
            constraints,
            end_continuation_mm: options.end_continuation_mm,
        })
    }

    fn constraints(&self, state: &HeldGuess) -> Option<(DVector<f64>, DMatrix<f64>)> {
        self.assemble_constraints(state, true)
    }

    fn assemble_constraints(
        &self,
        state: &HeldGuess,
        derivatives: bool,
    ) -> Option<(DVector<f64>, DMatrix<f64>)> {
        if state.poses.len() != self.chain.reference.len()
            || state.rod_poses.len() != self.curves.len()
            || state.stations_mm.len() != self.construct.anchors.len()
        {
            return None;
        }
        let mut residual = DVector::zeros(self.constraints);
        let mut jacobian = DMatrix::zeros(
            self.constraints,
            if derivatives { self.dimension } else { 0 },
        );
        for (i, a) in self.construct.anchors.iter().enumerate() {
            let body = &state.poses[a.body];
            let rod = &state.rod_poses[a.rod];
            let s = state.stations_mm[i];
            if !s.is_finite()
                || s < -self.end_continuation_mm
                || s > self.curves[a.rod].length() + self.end_continuation_mm
            {
                return None;
            }
            let (point, derivative, _) = self.curves[a.rod].at(s);
            let arm = body.rotation * a.point_mm;
            let rod_arm = rod.rotation * point;
            residual
                .rows_mut(3 * i, 3)
                .copy_from(&((body.centre_mm + arm - rod.centre_mm - rod_arm) / LENGTH_MM));
            if !derivatives {
                continue;
            }
            for axis in 0..3 {
                jacobian[(3 * i + axis, 6 * a.body + axis)] = 1.0;
            }
            jacobian
                .view_mut((3 * i, 6 * a.body + 3), (3, 3))
                .copy_from(&(-arm.cross_matrix() / LENGTH_MM));
            if let Some(j) = self.rod_offsets[a.rod] {
                for axis in 0..3 {
                    jacobian[(3 * i + axis, j + axis)] = -1.0;
                }
                for (k, basis) in self.curves[a.rod]
                    .rotation_basis(&rod.rotation)
                    .iter()
                    .enumerate()
                {
                    jacobian.view_mut((3 * i, j + 3 + k), (3, 1)).copy_from(
                        &((rod_arm - rod.rotation * self.curves[a.rod].centre()).cross(basis)
                            / LENGTH_MM),
                    );
                }
            }
            if let Some(j) = self.slide_offsets[i] {
                jacobian
                    .view_mut((3 * i, j), (3, 1))
                    .copy_from(&(-rod.rotation * derivative));
            }
        }
        for (r, row) in self.rolls.iter().enumerate() {
            if let Some(row) = row {
                let twist =
                    SwingTwist::new(&state.rod_poses[r].rotation, self.curves[r].long).ok()?;
                residual[*row] = twist.angle_rad();
                if !derivatives {
                    continue;
                }
                let j = self.rod_offsets[r]?;
                let gradient = twist.gradient();
                for (k, basis) in self.curves[r]
                    .rotation_basis(&state.rod_poses[r].rotation)
                    .iter()
                    .enumerate()
                {
                    jacobian[(*row, j + 3 + k)] = gradient.dot(basis);
                }
            }
        }
        Some((residual, jacobian))
    }
}

impl Model for HeldModel<'_> {
    type State = HeldGuess;
    fn dimension(&self) -> usize {
        self.dimension
    }
    fn constraint_data(&self, state: &HeldGuess) -> Option<(DVector<f64>, DMatrix<f64>)> {
        self.constraints(state)
    }
    fn first_order(&self, state: &HeldGuess) -> Option<FirstOrder> {
        let e = self.chain.evaluate(&state.poses).ok()?;
        let mut gradient = DVector::zeros(self.dimension);
        for (i, wrench) in e.gradient.iter().enumerate() {
            for axis in 0..6 {
                gradient[6 * i + axis] = wrench[axis] * if axis < 3 { LENGTH_MM } else { 1.0 };
            }
        }
        let (constraint, jacobian) = self.constraints(state)?;
        Some(FirstOrder {
            energy: e.energy_nmm,
            gradient,
            constraint,
            jacobian,
        })
    }
    fn step_measure(&self, state: &HeldGuess, delta: &DVector<f64>) -> f64 {
        // Bound local strain changes, rather than a distant body's absolute
        // translation during an otherwise small whole-column rotation.
        let mut largest = 0.0_f64;
        for i in 0..state.poses.len() {
            for axis in 0..3 {
                let below = if i + 1 < state.poses.len() {
                    delta[6 * (i + 1) + axis]
                } else {
                    0.0
                };
                largest = largest.max((delta[6 * i + axis] - below).abs());
                largest = largest.max(delta[6 * i + 3 + axis].abs());
            }
        }
        for (r, offset) in self.rod_offsets.iter().enumerate() {
            if let Some(offset) = offset {
                for k in 0..self.curves[r]
                    .rotation_basis(&state.rod_poses[r].rotation)
                    .len()
                {
                    largest = largest.max(delta[offset + 3 + k].abs());
                }
            }
        }
        for offset in self.slide_offsets.iter().flatten() {
            largest = largest.max(delta[*offset].abs());
        }
        largest
    }
    fn value(&self, state: &HeldGuess) -> Option<(f64, DVector<f64>)> {
        let energy = self.chain.evaluate(&state.poses).ok()?.energy_nmm;
        let (constraint, _) = self.assemble_constraints(state, false)?;
        Some((energy, constraint))
    }
    fn evaluate(&self, state: &HeldGuess) -> Option<Evaluation> {
        let linear = self.chain.linearise(&state.poses).ok()?;
        let mut gradient = DVector::zeros(self.dimension);
        let mut hessian = DMatrix::zeros(self.dimension, self.dimension);
        for i in 0..state.poses.len() {
            for a in 0..6 {
                let sa = if a < 3 { LENGTH_MM } else { 1.0 };
                gradient[6 * i + a] = linear.evaluation.gradient[i][a] * sa;
                for b in 0..6 {
                    let sb = if b < 3 { LENGTH_MM } else { 1.0 };
                    hessian[(6 * i + a, 6 * i + b)] = linear.diagonal[i][(a, b)] * sa * sb;
                    if i > 0 {
                        let v = linear.lower[i - 1][(a, b)] * sa * sb;
                        hessian[(6 * i + a, 6 * (i - 1) + b)] = v;
                        hessian[(6 * (i - 1) + b, 6 * i + a)] = v;
                    }
                }
            }
        }
        let (constraint, jacobian) = self.constraints(state)?;
        Some(Evaluation {
            energy: linear.evaluation.energy_nmm,
            gradient,
            hessian,
            constraint,
            jacobian,
        })
    }
    fn step(&self, state: &HeldGuess, delta: &DVector<f64>) -> Option<HeldGuess> {
        let mut next = state.clone();
        for (i, body) in state.poses.iter().enumerate() {
            let mut step = Wrench::from_column_slice(&delta.as_slice()[6 * i..6 * i + 6]);
            for k in 0..3 {
                step[k] *= LENGTH_MM;
            }
            next.poses[i] = body.incremented(&step);
        }
        for (r, offset) in self.rod_offsets.iter().enumerate() {
            if let Some(j) = offset {
                let basis = self.curves[r].rotation_basis(&state.rod_poses[r].rotation);
                let mut turn = Vector3::zeros();
                for (k, b) in basis.iter().enumerate() {
                    turn += b * delta[j + 3 + k];
                }
                let q = matrix_from_rotation_vector(&turn);
                let centre = state.rod_poses[r].centre_mm
                    + state.rod_poses[r].rotation * self.curves[r].centre();
                next.rod_poses[r] = Pose {
                    centre_mm: centre
                        + Vector3::new(delta[*j], delta[j + 1], delta[j + 2]) * LENGTH_MM
                        + q * (state.rod_poses[r].centre_mm - centre),
                    rotation: q * state.rod_poses[r].rotation,
                };
            }
        }
        for (i, j) in self.slide_offsets.iter().enumerate() {
            if let Some(j) = j {
                next.stations_mm[i] += delta[*j] * LENGTH_MM;
            }
        }
        // Every consumer next calls value/constraint_data, which validates
        // the domain. Do not assemble the same trial's Jacobian twice.
        Some(next)
    }
    fn lagrangian_hessian(
        &self,
        state: &HeldGuess,
        multipliers: &DVector<f64>,
        evaluation: &Evaluation,
    ) -> Option<DMatrix<f64>> {
        let mut h = evaluation.hessian.clone();
        for (i, a) in self.construct.anchors.iter().enumerate() {
            let f = Vector3::new(
                multipliers[3 * i],
                multipliers[3 * i + 1],
                multipliers[3 * i + 2],
            );
            let arm = state.poses[a.body].rotation * a.point_mm;
            h.view_mut((6 * a.body + 3, 6 * a.body + 3), (3, 3))
                .add_assign(symmetric_arm(arm, f) / LENGTH_MM);
            if let Some(j) = self.rod_offsets[a.rod] {
                let (p, d, _) = self.curves[a.rod].at(state.stations_mm[i]);
                let rotation = state.rod_poses[a.rod].rotation;
                let bases = self.curves[a.rod].rotation_basis(&rotation);
                let hh =
                    -symmetric_arm(rotation * (p - self.curves[a.rod].centre()), f) / LENGTH_MM;
                for (k, b) in bases.iter().enumerate() {
                    for (l, c) in bases.iter().enumerate() {
                        h[(j + 3 + k, j + 3 + l)] += b.dot(&(hh * c));
                    }
                }
                if let Some(s) = self.slide_offsets[i] {
                    let mixed = -(rotation * d).cross(&f);
                    for (k, b) in bases.iter().enumerate() {
                        let v = b.dot(&mixed);
                        h[(j + 3 + k, s)] += v;
                        h[(s, j + 3 + k)] += v;
                    }
                    h[(s, s)] -= LENGTH_MM
                        * self.curves[a.rod].curvature_work(state.stations_mm[i], &rotation, f);
                }
            } else if let Some(s) = self.slide_offsets[i] {
                h[(s, s)] -= LENGTH_MM
                    * self.curves[a.rod].curvature_work(
                        state.stations_mm[i],
                        &state.rod_poses[a.rod].rotation,
                        f,
                    );
            }
        }
        for (r, row) in self.rolls.iter().enumerate() {
            if let Some(row) = row {
                let twist =
                    SwingTwist::new(&state.rod_poses[r].rotation, self.curves[r].long).ok()?;
                let hh = twist.hessian() * multipliers[*row];
                let j = self.rod_offsets[r]?;
                for k in 0..3 {
                    for l in 0..3 {
                        h[(j + 3 + k, j + 3 + l)] += hh[(k, l)];
                    }
                }
            }
        }
        Some(h)
    }
}
use std::ops::AddAssign;

pub(super) fn solve(
    chain: &Chain,
    construct: &Construct,
    options: &HeldOptions,
    initial: Option<&HeldGuess>,
) -> Result<HeldState, SolveError> {
    let model = HeldModel::new(chain, construct, options)?;
    let mut start = initial.cloned().unwrap_or_else(|| HeldGuess {
        poses: chain.standing(),
        rod_poses: vec![
            Pose {
                centre_mm: Vector3::zeros(),
                rotation: Matrix3::identity()
            };
            construct.rods.len()
        ],
        stations_mm: construct.anchors.iter().map(|a| a.station_mm).collect(),
    });
    // A starting guess changes the iterate, never a prescribed material tie.
    // This also keeps fixed-station diagnostic solves independent of a caller's
    // stale slide values.
    if start.stations_mm.len() == construct.anchors.len() {
        for (i, anchor) in construct.anchors.iter().enumerate() {
            if options.fixed_stations || model.pinned.contains(&i) {
                start.stations_mm[i] = anchor.station_mm;
            }
        }
    }
    if start.poses.len() != chain.reference.len()
        || start.rod_poses.len() != construct.rods.len()
        || start.stations_mm.len() != construct.anchors.len()
    {
        return Err(SolveError::InvalidInput("starting state dimensions"));
    }
    // A supplied rod is the holder's zero. For a caller's rotated starting
    // guess, remove only its swing--twist component, retaining its visible
    // chord direction and centroid. No tissue freedom is prescribed by this
    // initial equality restoration.
    let mut restored = false;
    for (r, curve) in model.curves.iter().enumerate() {
        if model.rolls[r].is_none() {
            continue;
        }
        let rod = &start.rod_poses[r];
        let twist = SwingTwist::new(&rod.rotation, curve.long)
            .map_err(|error| SolveError::InvalidInput(error.code()))?;
        if twist.angle_rad().abs() <= 1e-12 {
            continue;
        }
        let rotation = twist.without_twist();
        let centre = rod.centre_mm + rod.rotation * curve.centre();
        start.rod_poses[r] = Pose {
            centre_mm: centre - rotation * curve.centre(),
            rotation,
        };
        restored = true;
    }
    let Some((initial_residual, _)) = model.constraints(&start) else {
        return Err(SolveError::InvalidInput("starting state"));
    };
    let maximum_head_gap_mm = initial_residual.as_slice()[..3 * construct.anchors.len()]
        .as_chunks::<3>()
        .0
        .iter()
        .map(|gap| (gap[0] * gap[0] + gap[1] * gap[1] + gap[2] * gap[2]).sqrt() * LENGTH_MM)
        .fold(0.0_f64, f64::max);
    // The supplied holder can already satisfy zero twist while its head
    // points are far from the standing bodies. Restore those point gaps as
    // an initial iterate too. The shortest head-line turn retains its free
    // swivel; no station, rod pose, or body freedom is subsequently fixed.
    // A feasible resting input is left bit for bit alone.
    if restored || maximum_head_gap_mm > INITIAL_HEAD_POINT_FIT_GAP_MM {
        for (body_index, body) in start.poses.iter_mut().enumerate() {
            let anchors: Vec<_> = construct
                .anchors
                .iter()
                .enumerate()
                .filter(|(_, a)| a.body == body_index)
                .collect();
            if anchors.is_empty() {
                continue;
            }
            let points: Vec<_> = anchors
                .iter()
                .map(|(i, a)| {
                    let rod = &start.rod_poses[a.rod];
                    rod.centre_mm + rod.rotation * model.curves[a.rod].at(start.stations_mm[*i]).0
                })
                .collect();
            if anchors.len() >= 2 {
                let from = body.rotation * (anchors[1].1.point_mm - anchors[0].1.point_mm);
                let to = points[1] - points[0];
                if from.norm() > 0.0 && to.norm() > 0.0 {
                    let from = from.normalize();
                    let to = to.normalize();
                    let cross = from.cross(&to);
                    if cross.norm() > 1e-12 {
                        let q = matrix_from_rotation_vector(
                            &(cross.normalize() * libm::atan2(cross.norm(), from.dot(&to))),
                        );
                        body.rotation = q * body.rotation;
                    }
                }
            }
            let point_mean = points.iter().copied().sum::<Vector3<f64>>() / anchors.len() as f64;
            let local_mean = anchors
                .iter()
                .map(|(_, a)| a.point_mm)
                .sum::<Vector3<f64>>()
                / anchors.len() as f64;
            body.centre_mm = point_mean - body.rotation * local_mean;
        }
    }
    let mut warm_iterations = 0;
    // Preserve an exactly resting input through the warm passes. The strict
    // minimiser below still checks every KKT row and reduced stiffness.
    let resting_feasible = model.first_order(&start).is_some_and(|e| {
        e.energy == 0.0
            && e.gradient.iter().all(|v| *v == 0.0)
            && e.constraint.abs().max() <= Options::default().constraint_tolerance
    });
    if chain.reference.len() > 8 && !options.fixed_rods && !resting_feasible {
        let warm = super::augmented::warm_start(&model, start.clone(), chain.reference.len());
        if warm.constraint_max.is_finite()
            && warm.stationarity_max.is_finite()
            && warm.multipliers.iter().all(|v| v.is_finite())
        {
            start = warm.state;
            warm_iterations = warm.iterations;
        }
        if let Some(warm) = super::joint_coordinates::newton_polish(&model, chain, start.clone())
            && warm.constraint_max.is_finite()
            && warm.stationarity_max.is_finite()
        {
            start = warm.state;
            warm_iterations += warm.iterations;
        }
    }
    let result = minimise::minimise(&model, start, Options::default());
    let energy = chain
        .evaluate(&result.state.poses)
        .map_err(|_| SolveError::InvalidInput("returned poses"))?;
    let contour_end_anchors: Vec<_> = model
        .slide_offsets
        .iter()
        .enumerate()
        .filter_map(|(i, variable)| {
            let s = result.state.stations_mm[i];
            let length = model.curves[construct.anchors[i].rod].length();
            (variable.is_some()
                && (s + model.end_continuation_mm).min(length + model.end_continuation_mm - s)
                    <= CONTOUR_END_PROXIMITY_MM)
                .then_some(i)
        })
        .collect();
    let settled = result.settled && contour_end_anchors.is_empty();
    let reason = if contour_end_anchors.is_empty() {
        StopReason::from(result.reason)
    } else {
        StopReason::HeadAtContourEnd
    };
    let scale = chain.stiffness_scale();
    let forces = if settled {
        (0..construct.anchors.len())
            .map(|i| {
                Vector3::new(
                    result.multipliers[3 * i],
                    result.multipliers[3 * i + 1],
                    result.multipliers[3 * i + 2],
                ) * (scale / LENGTH_MM)
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut torques = vec![Vector3::zeros(); construct.rods.len()];
    for (r, row) in model.rolls.iter().enumerate() {
        if let Some(row) = row
            && settled
        {
            let twist = SwingTwist::new(&result.state.rod_poses[r].rotation, model.curves[r].long)
                .map_err(|error| SolveError::InvalidInput(error.code()))?;
            torques[r] = twist.holding_torque(result.multipliers[*row] * scale);
        }
    }
    let pin_tangential_force_n = if settled {
        model
            .pinned
            .iter()
            .map(|&i| {
                let a = &construct.anchors[i];
                let (_, d, _) = model.curves[a.rod].at(result.state.stations_mm[i]);
                forces[i].dot(&(result.state.rod_poses[a.rod].rotation * d).normalize())
            })
            .collect()
    } else {
        Vec::new()
    };
    if !settled {
        torques.clear();
    }
    Ok(HeldState {
        poses: result.state.poses,
        rod_poses: result.state.rod_poses,
        stations_mm: result.state.stations_mm,
        end_continuation_mm: model.end_continuation_mm,
        head_forces_on_rod: forces,
        holding_torques_nmm: torques,
        pinned_anchors: model.pinned,
        contour_end_anchors,
        pin_tangential_force_n,
        ground_reaction: energy.ground_reaction,
        energy_nmm: energy.energy_nmm,
        settled,
        reason,
        iterations: warm_iterations + result.iterations,
        constraint_max_mm: result.constraint_max * LENGTH_MM,
        stationarity_max: result.stationarity_max,
    })
}
