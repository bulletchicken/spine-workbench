//! One passive gravity shape at three sizes on the locked seated construct.
//!
//! Patient x anterior, y left, z up; mm, radians, N and N·mm. Applied loads
//! act ON body centres; returned head increments act ON rods about each head.
//! Screw rotations stay tied; hooks retain their rotational release. Standing
//! uses the full seated tissue and loaded-connector tangent once; walking and
//! bending scale that response. No daily displacement changes the drawn shape.
//! Add locked-in wrenches before taking force norms or stress envelopes.

use crate::chain::{Chain, ChainError, ChainLinearisation, Ground, Pose, Wrench};
use crate::evidence::{Evidence, TableKind, Unit};
use crate::joint_springs::SpringError;
use crate::junctions;
use crate::springback::{self, RodLinearisation, SeatedState, SpringbackError};
use nalgebra::{DMatrix, DVector, Vector3};
use shapes::levels::Level;

/// Standard gravity, a physical unit convention in N/kg.
pub const GRAVITY_N_PER_KG: f64 = 9.80665;
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
    pub head_wrenches_on_rod: Vec<Vec<Wrench>>,
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

/// Head/neck acts at T1's centre; both arms are shared equally at the
/// centres of T1–T4. These sites are project choices. Trunk fractions
/// interpolate by level, with a patient-anterior centroid offset. All 17
/// sites are required rather than silently discarding absent body mass.
pub fn gravity_loads(
    evidence: &Evidence,
    chain: &Chain,
    weight_kg: f64,
    slice_offset_mm: f64,
) -> Result<GravityLoads, DailyError> {
    if !weight_kg.is_finite() || weight_kg < 0.0 || !slice_offset_mm.is_finite() {
        return Err(DailyError::InvalidInput("weight-or-slice-offset"));
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
    let body_loads: Vec<_> = (0..LOAD_LEVELS.len())
        .map(|i| {
            let slice = top + (bottom - top) * i as f64 / (LOAD_LEVELS.len() - 1) as f64;
            let fraction =
                slice + if i == 0 { head } else { 0.0 } + if i < 4 { 2.0 * arm / 4.0 } else { 0.0 };
            // (+offset,0,0) × (0,0,-slice*BW) gives a positive My.
            Wrench::new(
                0.0,
                0.0,
                -fraction * weight_n,
                0.0,
                slice_offset_mm * slice * weight_n,
                0.0,
            )
        })
        .collect();
    if body_loads
        .iter()
        .flat_map(|w| w.iter())
        .any(|v| !v.is_finite())
    {
        return Err(DailyError::InvalidInput("gravity-overflow"));
    }
    Ok(GravityLoads {
        body_loads,
        sources,
    })
}

/// Retain the correction chain's scale and ribs. Preload multiplies all
/// terms; each released level names the joint BELOW that vertebra, including
/// L5/sacrum. Release changes CC translation and AP/ML/CC moments only.
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
    let gravity = gravity_loads(evidence, chain, input.weight_kg, input.slice_offset_mm)?;
    let daily = loaded_chain(evidence, chain, input.preload_factor, released_levels)?;
    let before = loaded_chain(evidence, chain, input.preload_factor, &[])?;
    let standing = solve_case(
        &daily,
        &seated.poses,
        &seated.corrected_rods,
        &seated.head_wrenches_on_rod,
        &gravity.body_loads,
    )?;
    let without_construct = solve_case(&before, &before.standing(), &[], &[], &gravity.body_loads)?;
    let walking_peak = standing.scaled(input.walking_multiple)?;
    let bending_forward = standing.scaled(input.bending_multiple)?;
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
        ("bending-forward", "multiple", Unit::Ratio),
        ("slice-offset", "mean", Unit::Mm),
        ("slice-offset", "sd", Unit::Mm),
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
/// first-order physical head increments, with their exact weight linearity.
pub fn solve_case(
    chain: &Chain,
    poses: &[Pose],
    rods: &[RodLinearisation],
    preload: &[Vec<Wrench>],
    body_loads: &[Wrench],
) -> Result<DailyResponse, DailyError> {
    if body_loads.len() != poses.len()
        || body_loads
            .iter()
            .flat_map(|w| w.iter())
            .any(|v| !v.is_finite())
    {
        return Err(DailyError::InvalidInput("body-loads"));
    }
    let linear = chain.linearise(poses)?;
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
        let loads = chain.tangent_load(poses, &motion)?;
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
    let moments = junctions::moment_increments(chain, poses, &step.displacement)?;
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
        head_displacements_mm,
        rod_displacements: step.rod_displacements,
        rod_origins_mm: step.rod_origins_mm,
        joint_moments_nmm: moments,
        relative_residual: step.relative_residual,
    })
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
