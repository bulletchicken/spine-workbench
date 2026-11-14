//! Seating H, locking, and rigid release R, before elastic springback.
//!
//! Patient axes: x anterior, y patient-left, z superior, right-handed.
//! Lengths are mm, rotations radians, forces N, moments N·mm. A positive
//! x moment tips an up axis toward −y; +y tips it anterior; +z turns an
//! anterior axis toward patient-left. Head loads here act ON THE ROD;
//! opposite loads act on the vertebra. Bone-on-screw reaction is opposite
//! screw-on-vertebra force; positive pull-out is the latter projected
//! opposite the inward screw axis. Head moments exist after locking; the
//! outward bench contract reports resultant head force only.
//!
//! H minimises tissue energy with freely swivelling heads on rigid curves.
//! The lowest head station of each rod is pinned; other stations slide.
//! Its tangential pin reaction is returned separately. A curved rod has zero
//! swing--twist angle about its ORIGINAL supplied endpoint chord, measured
//! from the supplied placement. The chord stays fixed as heads slide. The
//! other five rigid motions stay free. The scalar multiplier's physical
//! gradient gives the external holding torque; after a finite swing it is
//! generally not parallel to the current chord. A 180-degree swing normal
//! to the supplied chord has undefined twist and is refused. Straight rods
//! have no holder row and only five visible rigid motions. Diagnostic fixed-station and
//! fixed-contour modes expose the remaining head-line turn directly.
//!
//! R locks each screw's six relative freedoms at H (hooks retain only three
//! translations), removes both holders, and minimises the same tissue
//! energy. Screw-connected bodies and rods are represented as rigid groups;
//! hooks leave point constraints between groups. This exact elimination of
//! locked ties avoids counting redundant constraints as mechanisms.
//! Outside the rods, joints return to their standing rest shape in R.
//! There are no muscles or spontaneous unfused-curve changes in this model;
//! shape comparisons are read inside the instrumented levels.
//!
//! H admits a project-selected 20 mm straight continuation along each unit
//! endpoint tangent. Nominal stations remain 0..L and the lower pin remains
//! at its supplied nominal station. No point, supplied chord or nominal length
//! is changed. Heads reaching the admitted continuation end are refused.
//! On locking, unused free continuation carries no load and is omitted from
//! the beam geometry beyond the outermost occupied or original knot.
//!
//! The curve is an interpolating cubic Hermite curve through the supplied
//! polyline, parameterised by its chord lengths. Centred secants give C1
//! tangents at interior knots; endpoint tangents are one-sided secants.
//! At a free station exactly on a knot, the slide curvature uses the
//! adverse of its two one-sided values. Only that Hessian diagonal jumps,
//! so positive reduced stiffness then suffices on both sides of the knot.
//! No contour plane is inferred or imposed. Parameters are mm of reference
//! chord, rather than exact spline arclength.
//! Endpoints bound the physical rod: a head that cannot seat inside them
//! returns an unsettled result. Beam meshes retain knots and seated heads.
//!
//! Both nonlinear solves normalise tissue stiffness before iterating, carry
//! uniform scale analytically into loads, and use fixed iteration caps.
//! Translation coordinates and residuals use a named 20 mm length scale.
//! The constrained minimiser checks reduced positive definiteness, rather
//! than rejecting a bilateral pair's kinematic head-line freedom.
//! This certifies a regular local constrained minimum. Distant initial
//! guesses can enter different basins; global uniqueness is not claimed.
//!
//! Project numerical choices: chains longer than eight bodies first take
//! at most 6 × 8 bordered augmented-energy steps, then at most 20 Newton
//! steps in exact joint-deformation coordinates. Those passes initialise
//! the strict body-coordinate minimiser; their loose stopping levels never
//! accept physical loads. Initial head gaps above a project-selected 1e-6 mm
//! receive a head-line swing and midpoint fit before those passes; this
//! changes only the initial guess, never its free turn or station constraints.
//! Already feasible resting inputs go directly to the same strict checks.
//! The final solve allows 80 steps, 24 backtracks
//! and 14 damping trials. It requires scaled equality residual <= 2e-12
//! and full KKT residual <= 1e-12 + 2e-11 times the largest balanced term,
//! plus an undamped positive definite reduced Lagrangian tangent. A cap
//! leaves `settled=false` with a reason and empty load arrays. These work
//! limits use no elapsed time. The detailed damping, trust and restoration
//! choices are documented in [`minimise`].

mod augmented;
mod curve;
mod held;
mod joint_coordinates;
pub mod minimise;
mod released;
mod twist;

use crate::chain::{Chain, Pose, Wrench};
use crate::rod_beams::BeamProperties;
use crate::springback::LockedRod;
use nalgebra::Vector3;

/// Project coordinate scale for mixed translation/rotation optimisation.
pub const LENGTH_MM: f64 = 20.0;
/// Project tolerance for a contour's departure from a straight line.
pub const STRAIGHT_RELATIVE_TOLERANCE: f64 = 1e-10;

/// A screw locks rotation at H; a hook never does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeadKind {
    Screw,
    Hook,
}

/// Plain head input. `point_mm` is LOCAL to the vertebra, measured from
/// its centre; rod indices and body indices refer to their input arrays.
#[derive(Clone, Debug)]
pub struct Anchor {
    pub body: usize,
    pub point_mm: Vector3<f64>,
    pub rod: usize,
    pub station_mm: f64,
    pub kind: HeadKind,
}

/// Stress-free contour in its initial patient-frame placement. E and G
/// are supplied through the evidence reader by the caller, in MPa.
#[derive(Clone, Debug)]
pub struct Rod {
    pub points_mm: Vec<Vector3<f64>>,
    pub properties: BeamProperties,
}

#[derive(Clone, Debug)]
pub struct Construct {
    pub anchors: Vec<Anchor>,
    pub rods: Vec<Rod>,
}

/// Project-selected straight continuation beyond each nominal contour end
/// while seating H. No published overhang value was found. The proposal's
/// head-to-head points, material stations, length and supplied chord stay fixed.
pub const DEFAULT_END_CONTINUATION_MM: f64 = 20.0;

/// Fixed modes are diagnostic subproblems, never a hidden boundary
/// condition in the default H → R correction.
#[derive(Clone, Debug)]
pub struct HeldOptions {
    pub fixed_stations: bool,
    pub fixed_rods: bool,
    /// Straight material continuation at each end, mm. Finite and nonnegative;
    /// this changes only the admitted H station domain, not the supplied curve.
    pub end_continuation_mm: f64,
}

impl Default for HeldOptions {
    fn default() -> Self {
        Self {
            fixed_stations: false,
            fixed_rods: false,
            end_continuation_mm: DEFAULT_END_CONTINUATION_MM,
        }
    }
}

/// Optional starting point, with stations in anchor order. Rod pose maps
/// contour coordinates to patient coordinates; identity is the input contour.
#[derive(Clone, Debug)]
pub struct HeldGuess {
    pub poses: Vec<Pose>,
    pub rod_poses: Vec<Pose>,
    pub stations_mm: Vec<f64>,
}

/// A nonlinear cap is visible; no singular solve is reported as settled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    Converged,
    IterationLimit,
    LineSearch,
    InconsistentConstraints,
    SingularReducedStiffness,
    InvalidInput,
    /// A free head reached the admitted straight continuation's end.
    /// No extra material or axial end-stop reaction is invented.
    HeadAtContourEnd,
}

impl From<minimise::Reason> for StopReason {
    fn from(value: minimise::Reason) -> Self {
        match value {
            minimise::Reason::Converged => Self::Converged,
            minimise::Reason::IterationLimit => Self::IterationLimit,
            minimise::Reason::LineSearch => Self::LineSearch,
            minimise::Reason::Inconsistent => Self::InconsistentConstraints,
            minimise::Reason::Singular => Self::SingularReducedStiffness,
            minimise::Reason::Invalid => Self::InvalidInput,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SolveError {
    #[error("correction-invalid-input: {0}")]
    InvalidInput(&'static str),
    #[error("correction-singular-construct: {0}")]
    SingularConstruct(&'static str),
    #[error("correction-linear-failure: {0}")]
    Linear(String),
}

/// Seated and held. Forces follow input anchor order; stations use each
/// reference curve's chord parameter. Holding torque acts on the rod.
#[derive(Clone, Debug)]
pub struct HeldState {
    pub poses: Vec<Pose>,
    pub rod_poses: Vec<Pose>,
    pub stations_mm: Vec<f64>,
    /// H admitted stations in [-extent, nominal_length+extent], mm, with
    /// nominal stations and the original lower material pin unchanged.
    pub end_continuation_mm: f64,
    /// Complete force, including any pinned station's tangential reaction.
    pub head_forces_on_rod: Vec<Vector3<f64>>,
    pub holding_torques_nmm: Vec<Vector3<f64>>,
    pub pinned_anchors: Vec<usize>,
    /// Free stations at the end of the admitted straight continuation.
    /// Such a state is refused even if its numerical KKT checks passed.
    /// These are diagnostics, not additional pins or bearing constraints.
    pub contour_end_anchors: Vec<usize>,
    /// Diagnostic projections of the forces above, not additional loads.
    pub pin_tangential_force_n: Vec<f64>,
    pub ground_reaction: Wrench,
    pub energy_nmm: f64,
    pub settled: bool,
    pub reason: StopReason,
    pub iterations: u32,
    pub constraint_max_mm: f64,
    pub stationarity_max: f64,
}

/// Locked and released, with free rigid rods. Head wrenches are in each
/// rod's sorted head order; `anchor_order` maps those entries to input anchors.
#[derive(Clone, Debug)]
pub struct RigidState {
    pub poses: Vec<Pose>,
    pub rod_poses: Vec<Pose>,
    pub rods: Vec<LockedRod>,
    pub anchor_order: Vec<Vec<usize>>,
    /// Seated curve knots and head points, in patient axes, for rod statics.
    pub rod_points_mm: Vec<Vec<Vector3<f64>>>,
    /// Beam-polyline arclengths, distinct from H's reference chord parameters.
    pub rod_head_stations_mm: Vec<Vec<f64>>,
    /// Forces and couples ON each rod, with the moment about its head.
    /// To transfer to a body centre, negate force and couple plus force arm.
    pub head_wrenches_on_rod: Vec<Vec<Wrench>>,
    pub ground_reaction: Wrench,
    pub energy_nmm: f64,
    pub settled: bool,
    pub reason: StopReason,
    pub iterations: u32,
}

/// Minimise H. Invalid plain inputs return an error; numerical nonconvergence
/// returns a state with `settled=false` and a reason, with no invented loads.
pub fn solve_held(
    chain: &Chain,
    construct: &Construct,
    options: &HeldOptions,
    initial: Option<&HeldGuess>,
) -> Result<HeldState, SolveError> {
    held::solve(chain, construct, options, initial)
}

/// Lock at H and remove the holders, using the general rigid-group minimum.
/// `initial` optionally supplies starting body poses for an independence check.
pub fn solve_rigid(
    chain: &Chain,
    construct: &Construct,
    held: &HeldState,
    initial: Option<&[Pose]>,
) -> Result<RigidState, SolveError> {
    released::solve(chain, construct, held, initial)
}
