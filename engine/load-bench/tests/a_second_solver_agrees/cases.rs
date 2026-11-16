//! Made-up plain inputs shared by the mechanics integration checks.
#![allow(dead_code)]

use forward_model::rotation::{matrix_from_rotation_vector, rotation_vector_from_matrix};
use load_bench::chain::{Chain, Ground, Pose};
use load_bench::evidence::{Evidence, TableKind, Unit};
use load_bench::joint_springs::Stiffness;
use load_bench::rigid_rod::{Anchor, Construct, HeadKind, HeldGuess, Rod};
use load_bench::rod_beams::{BeamProperties, polyline_stations};
use made_up_spines::patient::make_up_patient;
use nalgebra::{Matrix3, Vector3};
use shapes::{
    levels::Level,
    made_up::{RecipeName, VertebraPose},
};
use std::ops::RangeInclusive;

pub const HEAD_BACK_MM: f64 = -30.0;
pub const HEAD_HALF_WIDTH_MM: f64 = 15.0;

pub struct Case {
    pub chain: Chain,
    pub construct: Construct,
    pub target: Vec<Pose>,
    pub guess: HeldGuess,
    pub diagonals: Vec<[f64; 6]>,
    pub ground: Ground,
}

pub fn material() -> BeamProperties {
    let evidence = Evidence::read().expect("bundled material evidence");
    let modulus = evidence
        .value(TableKind::RodMaterials, "cobalt-chrome", "young-modulus")
        .expect("material modulus");
    assert_eq!(modulus.unit, Unit::Gpa);
    // The diameter is a chosen synthetic geometry, not a sourced material cell.
    BeamProperties::from_young_modulus(modulus.value * 1000.0, 5.5)
}

pub fn rods_through(target: &[Pose], bodies: impl IntoIterator<Item = usize>) -> Construct {
    let body_indices: Vec<_> = bodies.into_iter().collect();
    let properties = material();
    let mut rods = Vec::new();
    let mut anchors = Vec::new();
    for side in 0..2 {
        let local = Vector3::new(
            HEAD_BACK_MM,
            if side == 0 {
                HEAD_HALF_WIDTH_MM
            } else {
                -HEAD_HALF_WIDTH_MM
            },
            0.0,
        );
        let heads: Vec<_> = body_indices
            .iter()
            .map(|&body| target[body].centre_mm + target[body].rotation * local)
            .collect();
        // Synthetic half-span overhangs keep every freely sliding head away
        // from a physical contour endpoint in these mechanics checks.
        let mut points = Vec::with_capacity(heads.len() + 2);
        points.push(heads[0] - (heads[1] - heads[0]) * 0.5);
        points.extend_from_slice(&heads);
        points
            .push(heads[heads.len() - 1] + (heads[heads.len() - 1] - heads[heads.len() - 2]) * 0.5);
        let all_stations = polyline_stations(&points).expect("head contour");
        let stations = all_stations[1..all_stations.len() - 1].to_vec();
        rods.push(Rod {
            points_mm: points,
            properties,
        });
        for (&body, &station) in body_indices.iter().zip(&stations) {
            anchors.push(Anchor {
                body,
                point_mm: local,
                rod: side,
                station_mm: station,
                kind: HeadKind::Screw,
            });
        }
    }
    Construct { anchors, rods }
}

pub fn build(
    reference: Vec<VertebraPose>,
    target: Vec<Pose>,
    ground: Ground,
    scale: f64,
    bodies: RangeInclusive<usize>,
) -> Case {
    let evidence = Evidence::read().expect("joint evidence");
    let stiffness: Vec<_> = reference
        .iter()
        .map(|p| {
            Stiffness::from_evidence(&evidence, p.level == Level::L5)
                .expect("joint row")
                .scaled(scale, [1.0; 6])
                .expect("scale")
        })
        .collect();
    let diagonals = stiffness
        .iter()
        .map(|k| core::array::from_fn(|i| k.0[i]))
        .collect();
    let chain = Chain::new(&reference, &stiffness, ground.clone()).expect("plain chain");
    let construct = rods_through(&target, bodies);
    let stations_mm = construct.anchors.iter().map(|a| a.station_mm).collect();
    let guess = HeldGuess {
        poses: target.clone(),
        rod_poses: vec![
            Pose {
                centre_mm: Vector3::zeros(),
                rotation: Matrix3::identity()
            };
            construct.rods.len()
        ],
        stations_mm,
    };
    Case {
        chain,
        construct,
        target,
        guess,
        diagonals,
        ground,
    }
}

pub fn small(scale: f64, bend_x_mm: f64, bend_y_mm: f64) -> Case {
    let levels = [Level::L1, Level::L2, Level::L3, Level::L4, Level::L5];
    let reference: Vec<_> = levels
        .iter()
        .enumerate()
        .map(|(i, &level)| VertebraPose {
            level,
            centre: [0.0, 0.0, 10.0 + (4 - i) as f64 * 25.0],
            rotation: [0.0; 3],
            height: 20.0,
            wedge_front: 0.0,
            wedge_side: 0.0,
            top_half_width: 15.0,
            top_half_depth: 10.0,
            bottom_half_width: 15.0,
            bottom_half_depth: 10.0,
        })
        .collect();
    let target: Vec<_> = reference
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let profile = [0.0, 0.75, 1.0, 0.75, 0.0][i];
            Pose {
                centre_mm: Vector3::from(p.centre)
                    + Vector3::new(bend_x_mm * profile, bend_y_mm * profile, 0.0),
                rotation: Matrix3::identity(),
            }
        })
        .collect();
    build(
        reference,
        target,
        Ground {
            top_mm: Vector3::new(0.0, 0.0, -5.0),
            rotation: Matrix3::identity(),
        },
        scale,
        0..=4,
    )
}

pub fn made_up(recipe: RecipeName, seed: u32, scale: f64) -> Case {
    let patient = make_up_patient(recipe, seed).expect("made-up patient");
    let reference: Vec<_> = patient
        .standing
        .levels
        .iter()
        .zip(&patient.standing.vertebrae)
        .filter(|(l, _)| **l >= Level::T1)
        .map(|(&l, p)| p.pose(l))
        .collect();
    let target: Vec<_> = patient
        .bent_left
        .levels
        .iter()
        .zip(&patient.bent_left.vertebrae)
        .filter(|(l, _)| **l >= Level::T1)
        .map(|(_, p)| Pose {
            centre_mm: p.centre,
            rotation: p.rotation_matrix(),
        })
        .collect();
    let count = reference.len();
    build(
        reference,
        target,
        Ground {
            top_mm: patient.standing.pelvis.sacrum_top.centre,
            rotation: patient.standing.pelvis.frame,
        },
        scale,
        0..=count - 1,
    )
}

pub fn mirrored(case: &Case) -> Case {
    let mirror = Matrix3::from_diagonal(&Vector3::new(1.0, -1.0, 1.0));
    let reference: Vec<_> = case
        .chain
        .reference
        .iter()
        .map(|p| {
            let r = matrix_from_rotation_vector(&Vector3::from(p.rotation));
            VertebraPose {
                centre: (mirror * Vector3::from(p.centre)).into(),
                rotation: rotation_vector_from_matrix(&(mirror * r * mirror)).into(),
                ..*p
            }
        })
        .collect();
    let target: Vec<_> = case
        .target
        .iter()
        .map(|p| Pose {
            centre_mm: mirror * p.centre_mm,
            rotation: mirror * p.rotation * mirror,
        })
        .collect();
    let stiffness: Vec<_> = case
        .diagonals
        .iter()
        .map(|values| Stiffness::new(*values).unwrap())
        .collect();
    let ground = Ground {
        top_mm: mirror * case.ground.top_mm,
        rotation: mirror * case.ground.rotation * mirror,
    };
    let chain = Chain::new(&reference, &stiffness, ground.clone()).unwrap();
    let mut construct = case.construct.clone();
    for rod in &mut construct.rods {
        for p in &mut rod.points_mm {
            *p = mirror * (*p);
        }
    }
    for anchor in &mut construct.anchors {
        anchor.point_mm = mirror * anchor.point_mm;
    }
    let guess = HeldGuess {
        poses: target.clone(),
        rod_poses: vec![
            Pose {
                centre_mm: Vector3::zeros(),
                rotation: Matrix3::identity()
            };
            construct.rods.len()
        ],
        stations_mm: construct.anchors.iter().map(|a| a.station_mm).collect(),
    };
    Case {
        chain,
        construct,
        target,
        guess,
        diagonals: case.diagonals.clone(),
        ground,
    }
}

pub fn assert_poses_close(a: &[Pose], b: &[Pose], tolerance: f64) {
    assert_eq!(a.len(), b.len());
    for (i, (a, b)) in a.iter().zip(b).enumerate() {
        assert!(
            (a.centre_mm - b.centre_mm).norm() < tolerance,
            "body {i} centre error {}",
            (a.centre_mm - b.centre_mm).norm()
        );
        assert!(
            (a.rotation - b.rotation).norm() < tolerance,
            "body {i} rotation error {}",
            (a.rotation - b.rotation).norm()
        );
    }
}

/// Independent constraint evaluation. The cubic basis is evaluated directly
/// on the nominal contour; outside it the endpoint unit tangent is continued.
pub fn curve_at(points: &[Vector3<f64>], s: f64) -> (Vector3<f64>, Vector3<f64>) {
    let mut knots = vec![0.0];
    for pair in points.windows(2) {
        knots.push(knots.last().copied().unwrap() + (pair[1] - pair[0]).norm());
    }
    if s < 0.0 {
        let tangent = (points[1] - points[0]).normalize();
        return (points[0] + tangent * s, tangent);
    }
    let last = points.len() - 1;
    if s > knots[last] {
        let tangent = (points[last] - points[last - 1]).normalize();
        return (points[last] + tangent * (s - knots[last]), tangent);
    }
    let i = knots
        .partition_point(|k| *k <= s)
        .saturating_sub(1)
        .min(points.len() - 2);
    let slope = |j: usize| {
        let a = j.saturating_sub(1);
        let b = (j + 1).min(points.len() - 1);
        (points[b] - points[a]) / (knots[b] - knots[a])
    };
    let h = knots[i + 1] - knots[i];
    let t = (s - knots[i]) / h;
    let position = points[i] * ((1.0 + 2.0 * t) * (1.0 - t) * (1.0 - t))
        + slope(i) * (h * t * (1.0 - t) * (1.0 - t))
        + points[i + 1] * (t * t * (3.0 - 2.0 * t))
        + slope(i + 1) * (h * t * t * (t - 1.0));
    let derivative = points[i] * (6.0 * t * (t - 1.0) / h)
        + slope(i) * ((1.0 - t) * (1.0 - 3.0 * t))
        + points[i + 1] * (6.0 * t * (1.0 - t) / h)
        + slope(i + 1) * (t * (3.0 * t - 2.0));
    (position, derivative)
}

/// A curved frontal rest shape against sagittal rod contours gives the
/// holders a nonzero roll reaction; pure sagittal symmetry would hide it.
pub fn holding_case(scale: f64) -> Case {
    let base = small(1.0, -1.0, -1.0);
    let reference: Vec<_> = base
        .chain
        .reference
        .iter()
        .zip(&base.target)
        .map(|(p, target)| {
            let mut pose = *p;
            pose.centre[1] = target.centre_mm.y;
            pose
        })
        .collect();
    let target: Vec<_> = base
        .target
        .iter()
        .map(|p| Pose {
            centre_mm: Vector3::new(p.centre_mm.x, 0.0, p.centre_mm.z),
            rotation: p.rotation,
        })
        .collect();
    build(reference, target, base.ground, scale, 0..=4)
}

/// A peaked correction for force-sign checks. The discrete fourth
/// difference at the middle is 0-4*0.2+6-4*0.2+0=4.4, unlike a parabola's
/// zero fourth difference. This gives a nonzero leading apical force.
pub fn peaked(scale: f64, bend_x_mm: f64, bend_y_mm: f64) -> Case {
    let base = small(scale, 0.0, 0.0);
    let mut target = base.chain.standing();
    for (i, pose) in target.iter_mut().enumerate() {
        let profile = [0.0, 0.2, 1.0, 0.2, 0.0][i];
        pose.centre_mm += Vector3::new(bend_x_mm * profile, bend_y_mm * profile, 0.0);
    }
    build(base.chain.reference, target, base.ground, scale, 0..=4)
}
