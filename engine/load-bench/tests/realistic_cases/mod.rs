//! Actual made-up geometry → layout screws/contours → plain bench inputs.
//!
//! No reading, anchor, contour or material cell is replaced by a fabricated
//! value. The exact synthetic standing poses are represented as a
//! zero-uncertainty truth fit, following implant-layout's existing tests.
//! All coordinates are patient millimetres: x anterior, y left, z cranial.
//!
//! A layout contour samples its continuous shape at intervals <=2 mm. H
//! instead evaluates a C1 Hermite curve through those supplied samples,
//! parameterised by their chord lengths. Initial free stations therefore
//! use closest points on that H curve, not vertebral index fractions or
//! the layout contour's continuous arclength. The lowest head is pinned at
//! its rod's lower endpoint. This is an initialisation/boundary placement,
//! never an extra fixed station or a modification of the supplied contour.
#![allow(dead_code)]

mod initial_curve;
mod variants;
use initial_curve::InitialCurve;
pub use variants::RodVariant;
pub const VARIANTS: [RodVariant; 3] = variants::VARIANTS;

use forward_model::rotation::matrix_from_rotation_vector;
use implant_layout::curves::SpineReading;
use implant_layout::{rods, screws};
use load_bench::chain::{Chain, Ground, Pose};
use load_bench::evidence::{Evidence, TableKind, Unit};
use load_bench::joint_springs::Stiffness;
use load_bench::rigid_rod::{Anchor, Construct, HeadKind, HeldGuess, Rod};
use load_bench::rod_beams::BeamProperties;
use made_up_spines::patient::make_up_patient;
use nalgebra::{Matrix3, Vector3};
use shapes::implant_layout::{
    AnchorKind, AnchorProposal, ChildFacts, LayoutInput, NamingDoubt, RodMaterial, RodProposal,
    ValueStatus,
};
use shapes::levels::Level;
use shapes::made_up::{RecipeName, VertebraPose};
use shapes::spine_fit::{
    FitAngles, FitRmsMm, FitStandardDeviations, FitStopReason, FittedVertebra, InferredFit,
    SpineFit,
};

pub const CASES: [(RecipeName, u32); 6] = [
    (RecipeName::RightThoracic40, 1),
    (RecipeName::RightThoracic40, 2),
    (RecipeName::DoubleMajor55To45, 1),
    (RecipeName::DoubleMajor55To45, 2),
    (RecipeName::LumbarLeft30, 1),
    (RecipeName::LumbarLeft30, 2),
];

pub struct RealisticCase {
    pub variant: RodVariant,
    pub recipe: RecipeName,
    pub seed: u32,
    pub input: LayoutInput,
    pub reading: SpineReading,
    pub levels: Vec<Level>,
    pub anchors: Vec<AnchorProposal>,
    /// Actual layout return and source metadata. In the 70% probe these
    /// remain full-target, zero-extra proposals; Construct alone receives
    /// the explicitly interpolated contours documented by `variant`.
    pub rod_proposals: Vec<RodProposal>,
    /// Requested full/partial target; outside the span remains standing.
    pub target_poses: Vec<Pose>,
    pub chain: Chain,
    pub construct: Construct,
    pub guess: HeldGuess,
}

/// Build an all-pedicle, bilateral construct over the measured main curve's
/// inclusive end-vertebra span. Joint scales are one; rods are sourced
/// cobalt-chrome at 5.5 mm; both current default contours have zero overbend.
pub fn build(recipe: RecipeName, seed: u32) -> Result<RealisticCase, String> {
    build_variant(recipe, seed, RodVariant::LayoutDefault)
}

/// Same source geometry/materials, with only the explicitly requested
/// contour variant changed. No source/default or curve classification is
/// changed by the no-extra and partial-correction probes.
pub fn build_variant(
    recipe: RecipeName,
    seed: u32,
    variant: RodVariant,
) -> Result<RealisticCase, String> {
    let patient = make_up_patient(recipe, seed).map_err(|e| format!("patient: {e:?}"))?;
    let reference: Vec<_> = patient
        .standing
        .levels
        .iter()
        .zip(&patient.standing.vertebrae)
        .filter(|(level, _)| (Level::T1..=Level::L5).contains(level) && **level != Level::T13)
        .map(|(&level, v)| v.pose(level))
        .collect();
    let input = LayoutInput {
        fit: truth_fit(&reference),
        front: None,
        bend_left: None,
        bend_right: None,
        child: ChildFacts {
            // Chosen synthetic adolescent age, not an inferred patient fact.
            age_years: Some(15.0),
            weight_kg: None,
            activity: None,
        },
        naming: NamingDoubt::AsRead,
        versions: 32,
        seed,
        pedicle_child_share: None,
    };
    let versions = spine_readings::readings::versions(
        &input.fit,
        u64::from(input.seed),
        input.versions,
        0..input.versions,
    )
    .map_err(|e| format!("shared versions: {e}"))?;
    let reading = implant_layout::curves::measure(&input, &versions.nominal)
        .map_err(|e| format!("nominal reading: {e}"))?;
    let measured = versions
        .versions
        .iter()
        .map(|v| implant_layout::curves::measure(&input, &v.fit))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("version reading: {e}"))?;
    let major = reading
        .major
        .and_then(|i| reading.curves.get(i))
        .ok_or("no measured main curve")?;
    let levels: Vec<_> = input.fit.vertebrae[major.upper_index..=major.lower_index]
        .iter()
        .map(|v| v.level)
        .collect();
    let anchors = screws::anchors(&input, &versions.versions, &measured, &levels)
        .map_err(|e| format!("screws: {e}"))?;
    // This explicitly requested mechanics case ties every pedicle. A
    // missing catalogue size is retained in `anchors`; it is not replaced
    // by an invented diameter/length. The plain solver takes neither size.
    // Layout needs a Screw kind to include that supplied geometry in its
    // rod contour, so give that mechanical occupancy to a separate clone.
    let mut geometric_anchors = anchors.clone();
    for anchor in &mut geometric_anchors {
        if anchor.entry_status == ValueStatus::Missing
            || anchor.direction_status == ValueStatus::Missing
        {
            return Err(format!(
                "missing table head geometry at {:?}/{:?}",
                anchor.level, anchor.side
            ));
        }
        anchor.kind = AnchorKind::Screw;
    }
    let choice = rods::choices()
        .into_iter()
        .find(|c| c.material == RodMaterial::CobaltChrome && c.diameter_mm == 5.5)
        .ok_or("no sourced CoCr 5.5 mm layout choice")?;
    let rod_proposals = variants::proposals(
        variant,
        &input,
        &levels,
        &reading,
        &geometric_anchors,
        &choice,
    )?;
    let target_poses = variants::target_poses(variant, &input, &levels, &reading)?;
    let evidence = Evidence::read().map_err(|e| format!("evidence: {e}"))?;
    let modulus = evidence
        .value(TableKind::RodMaterials, "cobalt-chrome", "young-modulus")
        .ok_or("missing cobalt-chrome modulus")?;
    if modulus.unit != Unit::Gpa {
        return Err("cobalt-chrome modulus is not GPa".into());
    }
    let stiffness: Vec<_> = reference
        .iter()
        .map(|p| {
            Stiffness::from_evidence(&evidence, p.level == Level::L5)
                .map_err(|e| format!("joint stiffness: {e}"))
        })
        .collect::<Result<_, _>>()?;
    let chain = Chain::new(
        &reference,
        &stiffness,
        Ground {
            top_mm: patient.standing.pelvis.sacrum_top.centre,
            rotation: patient.standing.pelvis.frame,
        },
    )
    .map_err(|e| format!("chain: {e}"))?;
    let mut construct = Construct {
        rods: Vec::new(),
        anchors: Vec::new(),
    };
    for (rod_index, proposal) in rod_proposals.iter().enumerate() {
        let points_mm =
            variants::points(variant, &input, &levels, &anchors, proposal, &target_poses)?;
        let curve = InitialCurve::new(&points_mm)?;
        for proposal_anchor in anchors.iter().filter(|a| a.side == proposal.side) {
            let body = reference
                .iter()
                .position(|p| p.level == proposal_anchor.level)
                .ok_or("screw level missing from reference")?;
            let pose = Pose::from_vertebra(&reference[body]);
            let head = Vector3::from(proposal_anchor.entry_mm)
                - Vector3::from(proposal_anchor.direction_unit) * proposal.head_offset_mm;
            let station_mm = if proposal_anchor.level == *levels.last().unwrap() {
                curve.length()
            } else {
                curve.closest_station(head)
            };
            construct.anchors.push(Anchor {
                body,
                point_mm: pose.rotation.transpose() * (head - pose.centre_mm),
                rod: rod_index,
                station_mm,
                kind: HeadKind::Screw,
            });
        }
        construct.rods.push(Rod {
            points_mm,
            properties: BeamProperties::from_young_modulus(
                modulus.value * 1000.0,
                choice.diameter_mm,
            ),
        });
    }
    let guess = HeldGuess {
        poses: chain.standing(),
        rod_poses: vec![
            Pose {
                centre_mm: Vector3::zeros(),
                rotation: Matrix3::identity()
            };
            construct.rods.len()
        ],
        stations_mm: construct.anchors.iter().map(|a| a.station_mm).collect(),
    };
    Ok(RealisticCase {
        variant,
        recipe,
        seed,
        input,
        reading,
        levels,
        anchors,
        rod_proposals,
        target_poses,
        chain,
        construct,
        guess,
    })
}

/// Apply the existing made-up truth-fit convention, including its named
/// Euler angles. No radiographic optimisation or measured uncertainty is
/// claimed: zero RMS/SD identify an exact synthetic input for mechanics.
/// No films carrying a scale are supplied, so lengths remain typical-size.
/// Unity placements retain this direct truth fixture's two-view geometry;
/// they are coordinates for the fixture, not a measured film calibration.
pub fn truth_fit(reference: &[VertebraPose]) -> SpineFit {
    SpineFit {
        vertebrae: reference
            .iter()
            .map(|pose| {
                let rotation = matrix_from_rotation_vector(&pose.rotation.into());
                FittedVertebra {
                    level: pose.level,
                    pose: *pose,
                    angles: FitAngles {
                        side_tilt_rad: libm::atan2(-rotation[(1, 2)], rotation[(2, 2)]),
                        forward_lean_rad: libm::asin(rotation[(0, 2)].clamp(-1.0, 1.0)),
                        twist_rad: libm::atan2(-rotation[(0, 1)], rotation[(0, 0)]),
                    },
                    sd: FitStandardDeviations {
                        centre_mm: [0.0; 3],
                        angles_rad: [0.0; 3],
                    },
                    inferred: InferredFit {
                        depth: false,
                        width: false,
                        twist: false,
                    },
                    rms_mm: FitRmsMm {
                        front: Some(0.0),
                        side: Some(0.0),
                    },
                    corner_tilt_rad: None,
                }
            })
            .collect(),
        side_scale: 1.0,
        side_offset_mm: 0.0,
        origin_level: reference.last().expect("a synthetic spine").level,
        iterations: 0,
        converged: true,
        stop_reason: FitStopReason::SmallStep,
        rms_mm: 0.0,
        lengths: shapes::spine_fit::FitLengths::TypicalSize,
        typical_curve_deg: None,
        front_film: shapes::spine_fit::FilmPlacement {
            mm_per_px: [1.0; 2],
            origin_px: [0.0; 2],
            facing_assumed: false,
        },
        side_film: Some(shapes::spine_fit::FilmPlacement {
            mm_per_px: [1.0; 2],
            origin_px: [0.0; 2],
            facing_assumed: false,
        }),
    }
}

/// Alternative initial iterate only: layout's target poses inside the
/// instrumented span, standing outside it, with free stations reprojected
/// to the same supplied contours. Both physical lower pins stay unchanged.
pub fn layout_target_guess(case: &RealisticCase) -> Result<HeldGuess, String> {
    let mut guess = case.guess.clone();
    guess.poses = case.target_poses.clone();
    for (rod, contour) in case.construct.rods.iter().enumerate() {
        let curve = InitialCurve::new(&contour.points_mm)?;
        let bottom = case
            .construct
            .anchors
            .iter()
            .enumerate()
            .filter(|(_, a)| a.rod == rod)
            .max_by_key(|(i, a)| (a.body, *i))
            .map(|(i, _)| i)
            .ok_or("rod without heads")?;
        for (i, anchor) in case
            .construct
            .anchors
            .iter()
            .enumerate()
            .filter(|(_, a)| a.rod == rod)
        {
            if i != bottom {
                let head = guess.poses[anchor.body].centre_mm
                    + guess.poses[anchor.body].rotation * anchor.point_mm;
                guess.stations_mm[i] = curve.closest_station(head);
            }
        }
    }
    Ok(guess)
}
