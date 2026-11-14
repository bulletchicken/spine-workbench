//! Exact elimination of locked screw ties; hooks leave point constraints.
use super::curve::Curve;
use super::minimise::{self, Evaluation, Model, Options};
use super::{Construct, HeadKind, HeldState, LENGTH_MM, RigidState, SolveError, StopReason};
use crate::chain::{Chain, Pose};
use crate::rod_beams::{CondensedRod, polyline_stations};
use crate::springback::{HeadTie, LockedRod, least_energy_loads, prepare_rods};
use forward_model::rotation::matrix_from_rotation_vector;
use nalgebra::{DMatrix, DVector, Matrix3, Vector3};

#[derive(Clone)]
struct State {
    groups: Vec<Pose>,
}
struct ReleasedModel<'a> {
    chain: Chain,
    construct: &'a Construct,
    held: &'a HeldState,
    curves: Vec<Curve>,
    body_group: Vec<usize>,
    rod_group: Vec<usize>,
    origins: Vec<Vector3<f64>>,
    offsets: Vec<usize>,
    axes: Vec<Option<Vector3<f64>>>,
    hooks: Vec<usize>,
    dimension: usize,
}
fn root(parent: &[usize], mut i: usize) -> usize {
    while parent[i] != i {
        i = parent[i];
    }
    i
}
fn arm_hessian(a: Vector3<f64>, f: Vector3<f64>) -> Matrix3<f64> {
    (a * f.transpose() + f * a.transpose()) * 0.5 - Matrix3::identity() * a.dot(&f)
}
impl<'a> ReleasedModel<'a> {
    fn new(
        chain: &Chain,
        construct: &'a Construct,
        held: &'a HeldState,
    ) -> Result<Self, SolveError> {
        let n = chain.reference.len();
        let count = n + construct.rods.len();
        let mut parent: Vec<_> = (0..count).collect();
        for anchor in &construct.anchors {
            if anchor.kind == HeadKind::Screw {
                let a = root(&parent, anchor.body);
                let b = root(&parent, n + anchor.rod);
                parent[b] = a;
            }
        }
        let mut roots = Vec::new();
        let map: Vec<_> = (0..count)
            .map(|i| {
                let r = root(&parent, i);
                if let Some(k) = roots.iter().position(|v| *v == r) {
                    k
                } else {
                    roots.push(r);
                    roots.len() - 1
                }
            })
            .collect();
        let body_group = map[..n].to_vec();
        let rod_group = map[n..].to_vec();
        let mut origins = vec![Vector3::zeros(); roots.len()];
        let mut members = vec![0; roots.len()];
        for (i, pose) in held.poses.iter().enumerate() {
            origins[body_group[i]] += pose.centre_mm;
            members[body_group[i]] += 1;
        }
        for r in 0..construct.rods.len() {
            origins[rod_group[r]] += held.rod_poses[r].centre_mm;
            members[rod_group[r]] += 1;
        }
        for (i, o) in origins.iter_mut().enumerate() {
            *o /= members[i] as f64;
        }
        // A body's lowest member is a stable origin for block rotations.
        // Whole-column turns then need no large centroid translation.
        for (group, origin) in origins.iter_mut().enumerate() {
            if let Some(i) = body_group.iter().rposition(|g| *g == group) {
                *origin = held.poses[i].centre_mm;
            }
        }
        let curves = construct
            .rods
            .iter()
            .map(Curve::new)
            .collect::<Result<Vec<_>, _>>()?;
        let mut axes = vec![None; roots.len()];
        for (group, axis) in axes.iter_mut().enumerate() {
            if !body_group.contains(&group) {
                let r = rod_group.iter().position(|g| *g == group).unwrap();
                if !curves[r].is_curved {
                    *axis = Some(held.rod_poses[r].rotation * curves[r].long);
                }
            }
        }
        let mut dimension = 0;
        let offsets = axes
            .iter()
            .map(|axis| {
                let i = dimension;
                dimension += if axis.is_some() { 5 } else { 6 };
                i
            })
            .collect();
        let hooks = construct
            .anchors
            .iter()
            .enumerate()
            .filter(|(_, a)| a.kind == HeadKind::Hook && body_group[a.body] != rod_group[a.rod])
            .map(|(i, _)| i)
            .collect();
        Ok(Self {
            chain: chain
                .scaled(1.0 / chain.stiffness_scale())
                .map_err(|_| SolveError::InvalidInput("chain"))?
                .for_group_motion(&body_group),
            construct,
            held,
            curves,
            body_group,
            rod_group,
            origins,
            offsets,
            axes,
            hooks,
            dimension,
        })
    }
    fn bases(&self, state: &State, group: usize) -> Vec<Vector3<f64>> {
        if let Some(axis) = self.axes[group] {
            let axis = state.groups[group].rotation * axis;
            let reference = if axis.x.abs() < 0.8 {
                Vector3::x()
            } else {
                Vector3::y()
            };
            let a = (reference - axis * axis.dot(&reference)).normalize();
            vec![a, axis.cross(&a)]
        } else {
            vec![Vector3::x(), Vector3::y(), Vector3::z()]
        }
    }
    fn transform(&self, state: &State, group: usize, pose: &Pose) -> Pose {
        let g = &state.groups[group];
        Pose {
            centre_mm: g.centre_mm + g.rotation * (pose.centre_mm - self.origins[group]),
            rotation: g.rotation * pose.rotation,
        }
    }
    fn bodies(&self, state: &State) -> Vec<Pose> {
        self.held
            .poses
            .iter()
            .enumerate()
            .map(|(i, p)| self.transform(state, self.body_group[i], p))
            .collect()
    }
    fn rods(&self, state: &State) -> Vec<Pose> {
        self.held
            .rod_poses
            .iter()
            .enumerate()
            .map(|(i, p)| self.transform(state, self.rod_group[i], p))
            .collect()
    }
    fn point_jacobian(&self, state: &State, group: usize, point: Vector3<f64>) -> DMatrix<f64> {
        let mut j = DMatrix::zeros(3, self.dimension);
        let offset = self.offsets[group];
        for k in 0..3 {
            j[(k, offset + k)] = 1.0;
        }
        let arm = point - state.groups[group].centre_mm;
        for (k, b) in self.bases(state, group).iter().enumerate() {
            j.view_mut((0, offset + 3 + k), (3, 1))
                .copy_from(&(-arm.cross(b) / LENGTH_MM));
        }
        j
    }
    fn constraints(&self, state: &State) -> (DVector<f64>, DMatrix<f64>) {
        let bodies = self.bodies(state);
        let rods = self.rods(state);
        let mut c = DVector::zeros(self.hooks.len() * 3);
        let mut j = DMatrix::zeros(c.len(), self.dimension);
        for (k, &index) in self.hooks.iter().enumerate() {
            let a = &self.construct.anchors[index];
            let p = bodies[a.body].centre_mm + bodies[a.body].rotation * a.point_mm;
            let q = rods[a.rod].centre_mm
                + rods[a.rod].rotation * self.curves[a.rod].at(self.held.stations_mm[index]).0;
            c.rows_mut(3 * k, 3).copy_from(&((p - q) / LENGTH_MM));
            j.rows_mut(3 * k, 3).copy_from(
                &(self.point_jacobian(state, self.body_group[a.body], p)
                    - self.point_jacobian(state, self.rod_group[a.rod], q)),
            );
        }
        (c, j)
    }
}
impl Model for ReleasedModel<'_> {
    type State = State;
    fn dimension(&self) -> usize {
        self.dimension
    }
    fn constraint_data(&self, state: &State) -> Option<(DVector<f64>, DMatrix<f64>)> {
        Some(self.constraints(state))
    }
    fn value(&self, state: &State) -> Option<(f64, DVector<f64>)> {
        Some((
            self.chain.evaluate(&self.bodies(state)).ok()?.energy_nmm,
            self.constraints(state).0,
        ))
    }
    fn evaluate(&self, state: &State) -> Option<Evaluation> {
        let poses = self.bodies(state);
        let linear = self.chain.linearise(&poses).ok()?;
        let n = poses.len();
        let mut map = DMatrix::zeros(6 * n, self.dimension);
        let mut raw = DMatrix::zeros(6 * n, 6 * n);
        let mut g = DVector::zeros(6 * n);
        for (i, pose) in poses.iter().enumerate() {
            let group = self.body_group[i];
            let offset = self.offsets[group];
            let arm = pose.centre_mm - state.groups[group].centre_mm;
            for a in 0..6 {
                g[6 * i + a] = linear.evaluation.gradient[i][a];
                for b in 0..6 {
                    raw[(6 * i + a, 6 * i + b)] = linear.diagonal[i][(a, b)];
                    if i > 0 {
                        let v = linear.lower[i - 1][(a, b)];
                        raw[(6 * i + a, 6 * (i - 1) + b)] = v;
                        raw[(6 * (i - 1) + b, 6 * i + a)] = v;
                    }
                }
            }
            for k in 0..3 {
                map[(6 * i + k, offset + k)] = LENGTH_MM;
            }
            for (k, b) in self.bases(state, group).iter().enumerate() {
                map.view_mut((6 * i, offset + 3 + k), (3, 1))
                    .copy_from(&(-arm.cross(b)));
                map.view_mut((6 * i + 3, offset + 3 + k), (3, 1))
                    .copy_from(b);
            }
        }
        let gradient = map.transpose() * g;
        let mut hessian = map.transpose() * raw * map;
        for (i, pose) in poses.iter().enumerate() {
            let group = self.body_group[i];
            let offset = self.offsets[group];
            let force = linear.evaluation.gradient[i]
                .fixed_rows::<3>(0)
                .into_owned();
            let h = arm_hessian(pose.centre_mm - state.groups[group].centre_mm, force);
            let bases = self.bases(state, group);
            for (k, a) in bases.iter().enumerate() {
                for (l, b) in bases.iter().enumerate() {
                    hessian[(offset + 3 + k, offset + 3 + l)] += a.dot(&(h * b));
                }
            }
        }
        let (constraint, jacobian) = self.constraints(state);
        Some(Evaluation {
            energy: linear.evaluation.energy_nmm,
            gradient,
            hessian,
            constraint,
            jacobian,
        })
    }
    fn step(&self, state: &State, delta: &DVector<f64>) -> Option<State> {
        let mut next = state.clone();
        for i in 0..state.groups.len() {
            let j = self.offsets[i];
            let mut angle = Vector3::zeros();
            for (k, b) in self.bases(state, i).iter().enumerate() {
                angle += b * delta[j + 3 + k];
            }
            next.groups[i] = Pose {
                centre_mm: state.groups[i].centre_mm
                    + Vector3::new(delta[j], delta[j + 1], delta[j + 2]) * LENGTH_MM,
                rotation: matrix_from_rotation_vector(&angle) * state.groups[i].rotation,
            };
        }
        Some(next)
    }
    fn lagrangian_hessian(
        &self,
        state: &State,
        multipliers: &DVector<f64>,
        evaluation: &Evaluation,
    ) -> Option<DMatrix<f64>> {
        let mut result = evaluation.hessian.clone();
        let bodies = self.bodies(state);
        let rods = self.rods(state);
        for (k, &index) in self.hooks.iter().enumerate() {
            let a = &self.construct.anchors[index];
            let f = Vector3::new(
                multipliers[3 * k],
                multipliers[3 * k + 1],
                multipliers[3 * k + 2],
            );
            let p = bodies[a.body].centre_mm + bodies[a.body].rotation * a.point_mm;
            let q = rods[a.rod].centre_mm
                + rods[a.rod].rotation * self.curves[a.rod].at(self.held.stations_mm[index]).0;
            for (group, point, sign) in [
                (self.body_group[a.body], p, 1.0),
                (self.rod_group[a.rod], q, -1.0),
            ] {
                let j = self.offsets[group];
                let h = arm_hessian(point - state.groups[group].centre_mm, f) * (sign / LENGTH_MM);
                let bases = self.bases(state, group);
                for (u, b) in bases.iter().enumerate() {
                    for (v, c) in bases.iter().enumerate() {
                        result[(j + 3 + u, j + 3 + v)] += b.dot(&(h * c));
                    }
                }
            }
        }
        Some(result)
    }
}

pub(super) fn solve(
    chain: &Chain,
    construct: &Construct,
    held: &HeldState,
    initial: Option<&[Pose]>,
) -> Result<RigidState, SolveError> {
    if !held.settled {
        return Err(SolveError::InvalidInput("H did not settle"));
    }
    if held.poses.len() != chain.reference.len()
        || held.rod_poses.len() != construct.rods.len()
        || held.stations_mm.len() != construct.anchors.len()
    {
        return Err(SolveError::InvalidInput("H dimensions"));
    }
    if construct.rods.is_empty()
        || construct.anchors.is_empty()
        || construct.anchors.iter().any(|a| {
            a.body >= held.poses.len()
                || a.rod >= construct.rods.len()
                || a.point_mm.iter().any(|v| !v.is_finite())
        })
    {
        return Err(SolveError::InvalidInput("R construct"));
    }
    if !held.end_continuation_mm.is_finite() || held.end_continuation_mm < 0.0 {
        return Err(SolveError::InvalidInput("H end continuation"));
    }
    for (rod, contour) in construct.rods.iter().enumerate() {
        let curve = Curve::new(contour)?;
        for (index, anchor) in construct
            .anchors
            .iter()
            .enumerate()
            .filter(|(_, a)| a.rod == rod)
        {
            let station = held.stations_mm[index];
            if !station.is_finite()
                || station < -held.end_continuation_mm
                || station > curve.length() + held.end_continuation_mm
            {
                return Err(SolveError::InvalidInput("H station outside continuation"));
            }
            // The prescribed lower pin is still the original material station;
            // no continuation length is added to its parameter.
            if held.pinned_anchors.contains(&index) && station != anchor.station_mm {
                return Err(SolveError::InvalidInput("H material pin"));
            }
        }
    }
    let model = ReleasedModel::new(chain, construct, held)?;
    let mut groups: Vec<_> = model
        .origins
        .iter()
        .map(|c| Pose {
            centre_mm: *c,
            rotation: Matrix3::identity(),
        })
        .collect();
    if let Some(initial) = initial {
        if initial.len() != chain.reference.len() {
            return Err(SolveError::InvalidInput("R starting poses"));
        }
        for (group, g) in groups.iter_mut().enumerate() {
            if let Some(i) = model.body_group.iter().position(|v| *v == group) {
                let q = initial[i].rotation * held.poses[i].rotation.transpose();
                g.rotation = q;
                g.centre_mm =
                    initial[i].centre_mm - q * (held.poses[i].centre_mm - model.origins[group]);
            }
        }
    }
    let result = minimise::minimise(&model, State { groups }, Options::default());
    let poses = model.bodies(&result.state);
    let rod_poses = model.rods(&result.state);
    let energy = chain
        .evaluate(&poses)
        .map_err(|_| SolveError::InvalidInput("R poses"))?;
    let mut rods = Vec::new();
    let mut anchor_order = Vec::new();
    let mut rod_points_mm = Vec::new();
    let mut rod_head_stations_mm = Vec::new();
    for (r, rod_pose) in rod_poses.iter().enumerate() {
        let curve = &model.curves[r];
        let mut indices: Vec<_> = construct
            .anchors
            .iter()
            .enumerate()
            .filter(|(_, a)| a.rod == r)
            .map(|(i, _)| i)
            .collect();
        indices.sort_by(|&a, &b| held.stations_mm[a].total_cmp(&held.stations_mm[b]));
        if indices
            .windows(2)
            .any(|v| held.stations_mm[v[0]] >= held.stations_mm[v[1]])
        {
            return Err(SolveError::InvalidInput("heads share a rod station"));
        }
        // Preserve every original contour knot and every occupied material
        // parameter, including heads on the straight continuations. Unused
        // free continuation beyond these points carries nothing and is omitted.
        let mut parameters = curve.knots.clone();
        parameters.extend(indices.iter().map(|&i| held.stations_mm[i]));
        parameters.sort_by(f64::total_cmp);
        parameters.dedup_by(|a, b| *a == *b);
        let points: Vec<_> = parameters
            .iter()
            .map(|&s| rod_pose.centre_mm + rod_pose.rotation * curve.at(s).0)
            .collect();
        let arc = polyline_stations(&points)
            .map_err(|_| SolveError::InvalidInput("seated rod geometry"))?;
        let stations: Vec<_> = indices
            .iter()
            .map(|&i| {
                arc[parameters
                    .iter()
                    .position(|s| *s == held.stations_mm[i])
                    .unwrap()]
            })
            .collect();
        let beam = CondensedRod::new(&points, &stations, construct.rods[r].properties)
            .map_err(|e| SolveError::Linear(e.to_string()))?;
        let ties = indices
            .iter()
            .map(|&i| {
                let a = &construct.anchors[i];
                HeadTie {
                    body: a.body,
                    point_mm: poses[a.body].centre_mm + poses[a.body].rotation * a.point_mm,
                    locks_rotation: a.kind == HeadKind::Screw,
                }
            })
            .collect();
        rods.push(LockedRod {
            ties,
            stiffness: beam.stiffness,
        });
        anchor_order.push(indices);
        rod_points_mm.push(points);
        rod_head_stations_mm.push(stations);
    }
    let mut settled = result.settled;
    let mut reason = StopReason::from(result.reason);
    let mut head_wrenches_on_rod = Vec::new();
    if settled {
        let prepared =
            prepare_rods(&poses, &rods).map_err(|e| SolveError::Linear(e.to_string()))?;
        let gradient = DVector::from_iterator(
            6 * poses.len(),
            energy.gradient.iter().flat_map(|w| w.iter().copied()),
        );
        match least_energy_loads(&gradient, &prepared) {
            Ok(loads) => head_wrenches_on_rod = loads,
            Err(_) => {
                settled = false;
                reason = StopReason::InconsistentConstraints;
            }
        }
    }
    Ok(RigidState {
        poses,
        rod_poses,
        rods,
        anchor_order,
        rod_points_mm,
        rod_head_stations_mm,
        head_wrenches_on_rod,
        ground_reaction: energy.ground_reaction,
        energy_nmm: energy.energy_nmm,
        settled,
        reason,
        iterations: result.iterations,
    })
}
