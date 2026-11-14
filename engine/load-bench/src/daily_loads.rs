//! Compression through the discs, at three sizes on the locked construct.
//!
//! Patient x anterior, y left, z up; mm, radians, N and N·mm. Applied loads
//! act ON body centres; spatial head increments act ON rods about each head.
//! Screw rotations stay tied; hooks retain their rotational release. Standing
//! uses the seated prestress and stiffened material tangent once; walking and
//! bending scale that response. No daily displacement changes the drawn shape.
//! Anchor forces use the frame turning with the vertebra; rod statics uses
//! spatial increments and moving points. Neither counts a rigid turn as strain.

use crate::chain::{Chain, ChainError, ChainLinearisation, Ground, Pose, Wrench};
use crate::evidence::{Evidence, TableKind, Unit};
use crate::joint_springs::SpringError;
use crate::junctions;
use crate::springback::{self, RodLinearisation, SeatedState, SpringbackError};
use nalgebra::{DMatrix, DVector, Vector3};
use shapes::levels::Level;

/// Standard gravity, a physical unit convention in N/kg.
pub const GRAVITY_N_PER_KG: f64 = 9.80665;
/// Limit of the first-order daily method, in radians at any body. This
/// project bound limits neglected higher-order rotation terms; it is not
/// a physiological threshold or a fitted material parameter.
pub const SMALL_ANGLE_LIMIT_RAD: f64 = 0.1;
/// Project load sites, fixing the interpolation of the trunk slices.
pub const LOAD_LEVELS: [Level; 17] = [
    Level::T1,
    Level::T2,
    Level::T3,
    Level::T4,
    Level::T5,
    Level::T6,
    Level::T7,
    Level::T8,
    Level::T9,
    Level::T10,
    Level::T11,
    Level::T12,
    Level::L1,
    Level::L2,
    Level::L3,
    Level::L4,
    Level::L5,
];

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DailyError {
    #[error("daily-invalid-input: {0}")]
    InvalidInput(&'static str),
    #[error("daily-needs-standard-t1-to-l5-load-sites")]
    LoadSites,
    #[error("daily-missing-or-wrong-unit-evidence: {0}")]
    Evidence(String),
    #[error("daily-small-angle-limit")]
    SmallAngle,
    #[error(transparent)]
    Chain(#[from] ChainError),
    #[error(transparent)]
    Spring(#[from] SpringError),
    #[error(transparent)]
    Solve(#[from] SpringbackError),
}

/// Already sampled values; this module draws nothing.
#[derive(Clone, Copy, Debug)]
pub struct DailyInputs {
    pub weight_kg: f64,
    /// Retained input for the unchanged draw contract; unused by compression.
    pub slice_offset_mm: f64,
    pub preload_factor: f64,
    pub walking_multiple: f64,
    pub bending_multiple: f64,
}

#[derive(Clone, Debug)]
pub struct GravityLoads {
    pub body_loads: Vec<Wrench>,
    pub sources: Vec<String>,
}

/// Strictly first-order increments, with no locked-in wrench added.
#[derive(Clone, Debug)]
pub struct DailyResponse {
    pub body_loads: Vec<Wrench>,
    pub displacement: DVector<f64>,
    /// Spatial first variations, for rod equilibrium with moving head points.
    pub head_wrenches_on_rod: Vec<Vec<Wrench>>,
    /// Increments in each vertebra's turning frame. The locked-in wrench
    /// must be expressed in that body's seated axes before adding these.
    pub head_wrenches_in_body_frame: Vec<Vec<Wrench>>,
    /// Linear head movements, for dP × F0 in rod statics.
    pub head_displacements_mm: Vec<Vec<Vector3<f64>>>,
    /// Free material-frame motion about each supplied rod origin.
    pub rod_displacements: Vec<Wrench>,
    pub rod_origins_mm: Vec<Vector3<f64>>,
    /// Holding-couple increments at each disc, including the sacrum disc.
    pub joint_moments_nmm: Vec<Vector3<f64>>,
    pub relative_residual: f64,
}

impl DailyResponse {
    /// Scale a solved case, retaining geometry and its relative residual.
    pub fn scaled(&self, multiple: f64) -> Result<Self, DailyError> {
        if !multiple.is_finite() || multiple < 0.0 {
            return Err(DailyError::InvalidInput("load-multiple"));
        }
        let mut result = self.clone();
        result.body_loads.iter_mut().for_each(|w| *w *= multiple);
        result.displacement *= multiple;
        for rod in &mut result.head_wrenches_on_rod {
            rod.iter_mut().for_each(|w| *w *= multiple);
        }
        for rod in &mut result.head_wrenches_in_body_frame {
            rod.iter_mut().for_each(|w| *w *= multiple);
        }
        for rod in &mut result.head_displacements_mm {
            rod.iter_mut().for_each(|p| *p *= multiple);
        }
        result
            .rod_displacements
            .iter_mut()
            .for_each(|w| *w *= multiple);
        result
            .joint_moments_nmm
            .iter_mut()
            .for_each(|m| *m *= multiple);
        if result
            .body_loads
            .iter()
            .chain(result.head_wrenches_on_rod.iter().flatten())
            .chain(result.head_wrenches_in_body_frame.iter().flatten())
            .chain(result.rod_displacements.iter())
            .flat_map(|w| w.iter())
            .chain(result.displacement.iter())
            .chain(
                result
                    .head_displacements_mm
                    .iter()
                    .flatten()
                    .flat_map(|p| p.iter()),
            )
            .chain(result.joint_moments_nmm.iter().flat_map(|m| m.iter()))
            .any(|v| !v.is_finite())
        {
            return Err(DailyError::InvalidInput("scaled-response-overflow"));
        }
        check_small_angles(&result.displacement)?;
        Ok(result)
    }
}

#[derive(Clone, Debug)]
pub struct DailyCases {
    pub standing: DailyResponse,
    pub walking_peak: DailyResponse,
    pub bending_forward: DailyResponse,
    /// Unreleased reference spine, without rods. Other activities scale it.
    pub without_construct: DailyResponse,
    pub sources: Vec<String>,
}

/// Reference-pose compression with the same published weight fractions.
/// The offset remains in the input contract but contributes no load.
pub fn gravity_loads(
    evidence: &Evidence,
    chain: &Chain,
    weight_kg: f64,
    slice_offset_mm: f64,
) -> Result<GravityLoads, DailyError> {
    if !slice_offset_mm.is_finite() {
        return Err(DailyError::InvalidInput("weight-or-slice-offset"));
    }
    follower_loads(evidence, chain, &chain.standing(), weight_kg)
}

/// At disc i the cumulative weight above it acts down the upper body's
/// cranial axis through its material disc point. Consecutive cut resultants
/// differ to give the load on a body. Their couples about its centre make
/// the prefix moment exactly zero about every disc, even in a curved column.
/// Head/neck is assigned to T1, arms equally to T1–T4, and trunk fractions
/// interpolate by level. These sites and this balanced load path are project
/// assumptions; muscle forces and residual gravity moments are absent.
pub fn follower_loads(
    evidence: &Evidence,
    chain: &Chain,
    poses: &[Pose],
    weight_kg: f64,
) -> Result<GravityLoads, DailyError> {
    if !weight_kg.is_finite() || weight_kg < 0.0 {
        return Err(DailyError::InvalidInput("weight"));
    }
    if chain.reference.iter().map(|p| p.level).ne(LOAD_LEVELS) {
        return Err(DailyError::LoadSites);
    }
    let mut sources = Vec::new();
    let mut read = |row, term| {
        let value = cell(
            evidence,
            TableKind::DailyLoads,
            row,
            term,
            Unit::BodyWeightFraction,
        )?;
        if !sources.contains(&value.source) {
            sources.push(value.source.clone());
        }
        Ok::<_, DailyError>(value.value)
    };
    let head = read("head-neck", "fraction")?;
    let arm = read("each-arm-hand", "fraction")?;
    let top = read("trunk-slice-endpoints", "t1")?;
    let bottom = read("trunk-slice-endpoints", "l5")?;
    let weight_n = weight_kg * GRAVITY_N_PER_KG;
    let weights: Vec<_> = (0..LOAD_LEVELS.len())
        .map(|i| {
            let slice = top + (bottom - top) * i as f64 / (LOAD_LEVELS.len() - 1) as f64;
            weight_n
                * (slice
                    + if i == 0 { head } else { 0.0 }
                    + if i < 4 { 2.0 * arm / 4.0 } else { 0.0 })
        })
        .collect();
    let body_loads = compression_loads(chain, poses, &weights)?;
    Ok(GravityLoads {
        body_loads,
        sources,
    })
}

/// Difference of the prescribed compression resultants at successive cuts.
/// Kept separate from the population weights for hand-worked short columns.
pub fn compression_loads(
    chain: &Chain,
    poses: &[Pose],
    body_weights_n: &[f64],
) -> Result<Vec<Wrench>, DailyError> {
    if body_weights_n.len() != poses.len()
        || body_weights_n.iter().any(|w| !w.is_finite() || *w < 0.0)
    {
        return Err(DailyError::InvalidInput("body-weights"));
    }
    let frames = chain.joint_motion_frames(poses)?;
    let mut weight_above = 0.0;
    let mut previous_force = Vector3::zeros();
    let mut previous_point = Vector3::zeros();
    let body_loads: Vec<_> = (0..poses.len())
        .map(|i| {
            weight_above += body_weights_n[i];
            let point = frames[i].upper_point_mm;
            let force = -poses[i].rotation.column(2) * weight_above;
            let moment = (point - poses[i].centre_mm).cross(&force)
                - (previous_point - poses[i].centre_mm).cross(&previous_force);
            let mut load = Wrench::zeros();
            load.fixed_rows_mut::<3>(0)
                .copy_from(&(force - previous_force));
            load.fixed_rows_mut::<3>(3).copy_from(&moment);
            previous_force = force;
            previous_point = point;
            load
        })
        .collect();
    if body_loads
        .iter()
        .flat_map(|w| w.iter())
        .any(|v| !v.is_finite())
    {
        return Err(DailyError::InvalidInput("gravity-overflow"));
    }
    Ok(body_loads)
}

/// Retain scale and ribs, with an explicit uniform constitutive multiplier.
/// Correction calls this with 1; daily preload is instead passed to
/// `solve_case_with_material_scale` so it cannot rescale seated prestress.
/// Each release names the joint BELOW a vertebra, including L5/sacrum.
pub fn loaded_chain(
    evidence: &Evidence,
    chain: &Chain,
    preload_factor: f64,
    released_levels: &[Level],
) -> Result<Chain, DailyError> {
    if !preload_factor.is_finite()
        || preload_factor <= 0.0
        || released_levels
            .iter()
            .any(|l| !chain.reference.iter().any(|p| p.level == *l))
    {
        return Err(DailyError::InvalidInput("preload-or-release-level"));
    }
    let mut factors = [1.0; 6];
    if !released_levels.is_empty() {
        for (axis, term) in [
            (2, "compression"),
            (3, "coronal"),
            (4, "flexion"),
            (5, "axial-rotation"),
        ] {
            factors[axis] += cell(
                evidence,
                TableKind::JointSprings,
                "release",
                term,
                Unit::Percent,
            )?
            .value
                / 100.0;
        }
    }
    let stiffness = chain
        .joint_stiffnesses()
        .iter()
        .zip(&chain.reference)
        .map(|(k, p)| {
            k.scaled(
                preload_factor,
                if released_levels.contains(&p.level) {
                    factors
                } else {
                    [1.0; 6]
                },
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Chain::new(
        &chain.reference,
        &stiffness,
        Ground {
            top_mm: chain.ground.centre_mm,
            rotation: chain.ground.rotation,
        },
    )?)
}

/// Standing is solved once on each of the locked and reference systems.
/// Connector preload provides geometric stiffness, never an activity load.
pub fn solve(
    evidence: &Evidence,
    chain: &Chain,
    seated: &SeatedState,
    released_levels: &[Level],
    input: DailyInputs,
) -> Result<DailyCases, DailyError> {
    if !seated.settled {
        return Err(DailyError::InvalidInput("unsettled-seated-state"));
    }
    if !input.walking_multiple.is_finite()
        || input.walking_multiple < 1.0
        || !input.bending_multiple.is_finite()
        || input.bending_multiple < 0.0
    {
        return Err(DailyError::InvalidInput("activity-multiple"));
    }
    if !input.slice_offset_mm.is_finite() {
        return Err(DailyError::InvalidInput("slice-offset"));
    }
    let gravity = follower_loads(evidence, chain, &seated.poses, input.weight_kg)?;
    let daily = loaded_chain(evidence, chain, 1.0, released_levels)?;
    let standing = solve_case_with_material_scale(
        &daily,
        &seated.poses,
        &seated.corrected_rods,
        &seated.head_wrenches_on_rod,
        &gravity.body_loads,
        input.preload_factor,
    )?;
    let reference_loads = follower_loads(evidence, chain, &chain.standing(), input.weight_kg)?;
    let mut without_construct = solve_case_with_material_scale(
        chain,
        &chain.standing(),
        &[],
        &[],
        &reference_loads.body_loads,
        input.preload_factor,
    )?;
    // With no rods, each disc must carry its prescribed cut resultant:
    // compression and exactly zero intrinsic couple. Preserve that analytic
    // zero in the reference response rather than retaining numerical noise.
    without_construct.joint_moments_nmm.fill(Vector3::zeros());
    let walking_peak = standing.scaled(input.walking_multiple)?;
    let bending_forward = standing.scaled(input.bending_multiple)?;
    without_construct.scaled(input.walking_multiple)?;
    without_construct.scaled(input.bending_multiple)?;
    let mut sources = gravity.sources;
    for term in ["lower", "upper"] {
        let source = &cell(
            evidence,
            TableKind::JointSprings,
            "preload",
            term,
            Unit::Ratio,
        )?
        .source;
        if !sources.contains(source) {
            sources.push(source.clone());
        }
    }
    if !released_levels.is_empty() {
        for term in ["compression", "coronal", "flexion", "axial-rotation"] {
            let source = &cell(
                evidence,
                TableKind::JointSprings,
                "release",
                term,
                Unit::Percent,
            )?
            .source;
            if !sources.contains(source) {
                sources.push(source.clone());
            }
        }
    }
    for (row, term, unit) in [
        ("walking", "lower", Unit::Ratio),
        ("walking", "upper", Unit::Ratio),
        ("ventral-flexion", "lower", Unit::Ratio),
        ("ventral-flexion", "upper", Unit::Ratio),
        ("ventral-flexion", "activity-group-upper", Unit::Ratio),
        ("ventral-flexion", "highest-measured-walking", Unit::Ratio),
    ] {
        let source = &cell(evidence, TableKind::DailyLoads, row, term, unit)?.source;
        if !sources.contains(source) {
            sources.push(source.clone());
        }
    }
    Ok(DailyCases {
        standing,
        walking_peak,
        bending_forward,
        without_construct,
        sources,
    })
}

/// Plain added-load solve for full spines or small hand-worked chains.
/// Rods and their preload use matching rod/head order at `poses`. Retains
/// first-order spatial and turning-frame head increments.
pub fn solve_case(
    chain: &Chain,
    poses: &[Pose],
    rods: &[RodLinearisation],
    preload: &[Vec<Wrench>],
    body_loads: &[Wrench],
) -> Result<DailyResponse, DailyError> {
    solve_case_with_material_scale(chain, poses, rods, preload, body_loads, 1.0)
}

/// Added-load solve with stiffened material response and the original seated
/// joint forces in all geometric terms. No equilibrium load is rescaled.
pub fn solve_case_with_material_scale(
    chain: &Chain,
    poses: &[Pose],
    rods: &[RodLinearisation],
    preload: &[Vec<Wrench>],
    body_loads: &[Wrench],
    material_scale: f64,
) -> Result<DailyResponse, DailyError> {
    if body_loads.len() != poses.len()
        || body_loads
            .iter()
            .flat_map(|w| w.iter())
            .any(|v| !v.is_finite())
    {
        return Err(DailyError::InvalidInput("body-loads"));
    }
    let linear = chain.linearise_with_material_scale(poses, material_scale)?;
    let tangent = tangent_matrix(&linear);
    let loads = DVector::from_iterator(
        6 * body_loads.len(),
        body_loads.iter().flat_map(|w| w.iter().copied()),
    );
    let tissue_load = |u: &DVector<f64>| {
        let motion: Vec<_> = u
            .as_slice()
            .as_chunks::<6>()
            .0
            .iter()
            .map(|v| Wrench::from_column_slice(v))
            .collect();
        let loads = if material_scale == 1.0 {
            chain.tangent_load(poses, &motion)?
        } else {
            chain.tangent_load_with_material_scale(poses, &motion, material_scale)?
        };
        Ok(DVector::from_iterator(
            u.len(),
            loads.iter().flat_map(|w| w.iter().copied()),
        ))
    };
    let step = springback::solve_incremental_with_tissue_load(
        poses,
        &tangent,
        &loads,
        rods,
        preload,
        &tissue_load,
    )?;
    check_small_angles(&step.displacement)?;
    let moments = junctions::moment_increments_with_material_scale(
        chain,
        poses,
        &step.displacement,
        material_scale,
    )?;
    let head_wrenches_in_body_frame = rods
        .iter()
        .zip(&step.linear_head_wrenches_on_rod)
        .zip(preload)
        .map(|((rod, increments), initial)| {
            rod.ties
                .iter()
                .zip(increments)
                .zip(initial)
                .map(|((tie, delta), p0)| {
                    let omega = Vector3::from_column_slice(
                        step.displacement.rows(6 * tie.body + 3, 3).as_slice(),
                    );
                    in_turning_frame(&poses[tie.body], &omega, p0, delta)
                })
                .collect()
        })
        .collect();
    let head_displacements_mm = rods
        .iter()
        .map(|rod| {
            rod.ties
                .iter()
                .map(|tie| {
                    let u = Wrench::from_column_slice(
                        step.displacement.rows(6 * tie.body, 6).as_slice(),
                    );
                    u.fixed_rows::<3>(0).into_owned()
                        + u.fixed_rows::<3>(3)
                            .cross(&(tie.point_mm - poses[tie.body].centre_mm))
                })
                .collect()
        })
        .collect();
    Ok(DailyResponse {
        body_loads: body_loads.to_vec(),
        displacement: step.displacement,
        head_wrenches_on_rod: step.linear_head_wrenches_on_rod,
        head_wrenches_in_body_frame,
        head_displacements_mm,
        rod_displacements: step.rod_displacements,
        rod_origins_mm: step.rod_origins_mm,
        joint_moments_nmm: moments,
        relative_residual: step.relative_residual,
    })
}

/// First variation of R_body^T W: remove transport before expressing the
/// increment in seated body axes. Under a common rigid turn dW=omega×W0,
/// this is exactly zero, for both force and couple about the moving head.
fn in_turning_frame(pose: &Pose, omega: &Vector3<f64>, initial: &Wrench, delta: &Wrench) -> Wrench {
    let mut result = Wrench::zeros();
    for start in [0, 3] {
        let value = pose.rotation.transpose()
            * (delta.fixed_rows::<3>(start) - omega.cross(&initial.fixed_rows::<3>(start)));
        result.fixed_rows_mut::<3>(start).copy_from(&value);
    }
    result
}

fn check_small_angles(displacement: &DVector<f64>) -> Result<(), DailyError> {
    if displacement
        .as_slice()
        .as_chunks::<6>()
        .0
        .iter()
        .any(|u| Vector3::new(u[3], u[4], u[5]).norm() > SMALL_ANGLE_LIMIT_RAD)
    {
        return Err(DailyError::SmallAngle);
    }
    Ok(())
}

pub(crate) fn tangent_matrix(linear: &ChainLinearisation) -> DMatrix<f64> {
    let n = linear.diagonal.len();
    let mut matrix = DMatrix::zeros(6 * n, 6 * n);
    for i in 0..n {
        matrix
            .view_mut((6 * i, 6 * i), (6, 6))
            .copy_from(&linear.diagonal[i]);
        if i + 1 < n {
            matrix
                .view_mut((6 * (i + 1), 6 * i), (6, 6))
                .copy_from(&linear.lower[i]);
            matrix
                .view_mut((6 * i, 6 * (i + 1)), (6, 6))
                .copy_from(&linear.lower[i].transpose());
        }
    }
    matrix
}

fn cell<'a>(
    evidence: &'a Evidence,
    table: TableKind,
    row: &str,
    term: &str,
    unit: Unit,
) -> Result<&'a crate::evidence::Value, DailyError> {
    evidence
        .value(table, row, term)
        .filter(|v| v.unit == unit && v.value.is_finite())
        .ok_or_else(|| DailyError::Evidence(format!("{row}/{term}")))
}
