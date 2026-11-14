//! A rod is its contour before seating. Only a contour with no overbend
//! interpolates every target head; an overbent rod keeps the two end heads
//! and bows posterior to the target in between.

use forward_model::endplates::Endplates;
use forward_model::rotation::{
    matrix_from_rotation_vector, rot_x, rot_y, rot_z, rotation_vector_from_matrix,
};
use forward_model::vertebra_params::VertebraParams;
use implant_layout::curves::{Classification, Curve, SpineReading};
use implant_layout::evidence::{Coordinate, PublishedValue, Table, tables};
use implant_layout::rods::{default_choice, propose, propose_with_overbend, target_spine};
use made_up_spines::patient::make_up_patient;
use nalgebra::Vector3;
use shapes::implant_layout::{
    AnchorKind, AnchorProposal, CannotTellCode, ChildFacts, CurveSide, LayoutInput, NamingDoubt,
    PedicleEstimate, ValueStatus,
};
use shapes::levels::Level;
use shapes::made_up::{CurveRegion, RecipeName, VertebraPose};
use shapes::spine_fit::{
    FitAngles, FitRmsMm, FitStandardDeviations, FitStopReason, FittedVertebra, InferredFit,
    SpineFit,
};
use shapes::spine_readings::{Band, BodySide};

fn fixture() -> (SpineFit, SpineReading, Vec<AnchorProposal>) {
    let levels = [
        Level::T5,
        Level::T6,
        Level::T7,
        Level::T8,
        Level::T9,
        Level::T10,
        Level::T11,
        Level::T12,
    ];
    let vertebrae = levels
        .iter()
        .enumerate()
        .map(|(i, level)| {
            let lean = (0.5 - i as f64 / 7.0) * 20.0_f64.to_radians();
            let params = VertebraParams {
                centre: Vector3::new(0.0, 0.0, (8 - i) as f64 * 24.0),
                rotation: rotation_vector_from_matrix(&rot_y(lean)),
                height: 20.0,
                wedge_front: 0.0,
                wedge_side: 0.0,
                top_half_width: 15.0,
                top_half_depth: 10.0,
                bottom_half_width: 15.0,
                bottom_half_depth: 10.0,
            };
            FittedVertebra {
                level: *level,
                pose: params.pose(*level),
                angles: FitAngles {
                    side_tilt_rad: 0.0,
                    forward_lean_rad: lean,
                    twist_rad: 0.0,
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
        .collect();
    let fit = SpineFit {
        vertebrae,
        side_scale: 1.0,
        side_offset_mm: 0.0,
        origin_level: Level::T12,
        iterations: 0,
        converged: true,
        stop_reason: FitStopReason::SmallStep,
        rms_mm: 0.0,
        lengths: shapes::spine_fit::FitLengths::Measured,
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
    };
    let reading = SpineReading {
        vertebrae: Vec::new(),
        curves: vec![Curve {
            upper_index: 0,
            lower_index: 7,
            apex_index: 3,
            region: Some(CurveRegion::MainThoracic),
            convex: BodySide::Right,
            cobb_deg: 40.0,
            signed_cobb_deg: 40.0,
            apical_deviation_mm: -10.0,
            bending_cobb_deg: Err(CannotTellCode::NoBendingFilms),
            straightening_bend: Err(CannotTellCode::NoBendingFilms),
            bending_disc_below_deg: Err(CannotTellCode::NoBendingFilms),
            structural: Ok(true),
        }],
        major: Some(0),
        csvl_y_mm: Ok(0.0),
        kyphosis_t2_t5_deg: Err(CannotTellCode::NoSideFilm),
        kyphosis_t5_t12_deg: Ok(20.0),
        kyphosis_t10_l2_deg: Ok(0.0),
        l4_tilt_deg: Err(CannotTellCode::NoCurve),
        classification: Classification {
            curve_type: Err(CannotTellCode::NoCurve),
            lumbar_modifier: Err(CannotTellCode::NoCurve),
            sagittal_modifier: Err(CannotTellCode::NoSideFilm),
        },
        minor_curve_minimum_deg: None,
        sources: Vec::new(),
    };
    let anchors = fit
        .vertebrae
        .iter()
        .flat_map(|v| {
            [BodySide::Left, BodySide::Right].map(|side| {
                let y = if side == BodySide::Left { 12.0 } else { -12.0 };
                AnchorProposal {
                    wider_than_pedicle: Vec::new(),
                    screw_here: None,
                    diameter_sources: Vec::new(),
                    length_sources: Vec::new(),
                    direction_sources: Vec::new(),
                    look_sources: Vec::new(),
                    level: v.level,
                    side,
                    kind: AnchorKind::Screw,
                    entry_mm: [v.pose.centre[0] - 20.0, y, v.pose.centre[2]],
                    entry_status: ValueStatus::FromTemplate,
                    direction_unit: [1.0, 0.0, 0.0],
                    direction_status: ValueStatus::FromTables,
                    medial_angle: None,
                    medial_angles_beside: Vec::new(),
                    convergence_deg: Band {
                        p10: 0.0,
                        p50: 0.0,
                        p90: 0.0,
                    },
                    diameter_mm: Some(5.0),
                    diameter_also_seen: Vec::new(),
                    diameter_status: ValueStatus::FromTables,
                    width_ratio: None,
                    diameters_beside: Vec::new(),
                    screw_to_pedicle: None,
                    chord_mm: None,
                    chord_share: None,
                    length_mm: Some(30.0),
                    length_also_seen: Vec::new(),
                    length_status: ValueStatus::FromTables,
                    usual_length_mm: None,
                    pedicle: PedicleEstimate {
                        sources: Vec::new(),
                        outer_width_mm: None,
                        inner_width_mm: None,
                        versions_too_narrow: 0,
                        versions_inner_under_1_mm: 0,
                        curve_side: CurveSide::Outside,
                        status: ValueStatus::Missing,
                    },
                    looks: Vec::new(),
                    sources: Vec::new(),
                }
            })
        })
        .collect();
    (fit, reading, anchors)
}

fn distance_to_polyline(head: Vector3<f64>, points: &[[f64; 3]]) -> f64 {
    points
        .windows(2)
        .map(|pair| {
            let a = Vector3::from(pair[0]);
            let d = Vector3::from(pair[1]) - a;
            let t = ((head - a).dot(&d) / d.dot(&d)).clamp(0.0, 1.0);
            (head - a - d * t).norm()
        })
        .fold(f64::INFINITY, f64::min)
}

// This fixture is already at the target: all relative coronal/axial
// angles vanish, and its sagittal total is inside the band. Rotating no
// joint leaves every standing material point and head unchanged.
fn target_heads(
    fit: &SpineFit,
    anchors: &[AnchorProposal],
    side: BodySide,
    levels: &[Level],
    offset: f64,
) -> Vec<Vector3<f64>> {
    fit.vertebrae
        .iter()
        .filter(|v| levels.contains(&v.level))
        .map(|v| {
            let anchor = anchors
                .iter()
                .find(|a| a.level == v.level && a.side == side)
                .unwrap();
            Vector3::from(anchor.entry_mm) - Vector3::from(anchor.direction_unit) * offset
        })
        .collect()
}

fn published_scalar(table: Table, key: &str, group: &str) -> f64 {
    match tables()
        .lookup(
            table,
            &Coordinate {
                key: key.into(),
                group: Some(group.into()),
                ..Coordinate::default()
            },
        )
        .unwrap()
        .value
    {
        PublishedValue::Scalar { value } => value,
        _ => panic!("the published value is scalar"),
    }
}

fn published_extra() -> f64 {
    published_scalar(Table::RodChoices, "overcontour", "concave")
}

fn x_at_z(points: &[[f64; 3]], z: f64) -> f64 {
    points
        .windows(2)
        .find_map(|pair| {
            let (a, b) = (pair[0], pair[1]);
            if (a[2].min(b[2])..=a[2].max(b[2])).contains(&z) && a[2] != b[2] {
                let f = (z - a[2]) / (b[2] - a[2]);
                Some(a[0] + f * (b[0] - a[0]))
            } else {
                None
            }
        })
        .expect("the intermediate head lies inside the contour's z span")
}

#[test]
fn an_explicit_extra_bows_the_concave_rod_while_the_convex_one_keeps_every_head() {
    use implant_layout::rods::SAMPLE_STEP_MM;
    let (fit, reading, anchors) = fixture();
    let levels: Vec<_> = fit.vertebrae.iter().map(|v| v.level).collect();
    let rods = propose_with_overbend(
        &fit,
        &levels,
        &reading,
        &anchors,
        &default_choice().unwrap(),
        published_extra(),
    )
    .unwrap();
    for rod in rods {
        let heads = target_heads(&fit, &anchors, rod.side, &levels, rod.head_offset_mm);
        assert_eq!(rod.standing_points_mm.len(), rod.points_mm.len());
        for head in &heads {
            assert!(distance_to_polyline(*head, &rod.standing_points_mm) < 1e-9);
        }
        for head in [heads[0], heads[heads.len() - 1]] {
            assert!(
                distance_to_polyline(head, &rod.points_mm) < 1e-9,
                "the end head stays on the contoured rod"
            );
        }
        if rod.side == BodySide::Right {
            assert_eq!(rod.overbend_deg, 0.0);
            for head in heads {
                assert!(distance_to_polyline(head, &rod.points_mm) < 1e-9);
            }
        } else {
            assert!(rod.overbend_deg > 0.0);
            for head in &heads[1..heads.len() - 1] {
                assert!(
                    x_at_z(&rod.points_mm, head.z) < head.x - 1e-6,
                    "the overbent contour must bow posterior to the target head"
                );
            }
        }
        assert!(
            rod.points_mm.windows(2).all(
                |p| (Vector3::from(p[1]) - Vector3::from(p[0])).norm() <= SAMPLE_STEP_MM + 1e-9
            )
        );
    }
}

#[test]
fn contours_without_overbend_keep_every_noncoplanar_target_head() {
    let (fit, reading, mut anchors) = fixture();
    // Set the requested extra to zero without changing the curve side:
    // the concave contour then has exactly the same head ties as the convex.
    for (i, anchor) in anchors.iter_mut().enumerate() {
        let row = i / 2;
        anchor.entry_mm[1] += (if row % 2 == 0 { 1.0 } else { -1.0 }) * row as f64 * 0.3;
    }
    let levels: Vec<_> = fit.vertebrae.iter().map(|v| v.level).collect();
    let rods = propose_with_overbend(
        &fit,
        &levels,
        &reading,
        &anchors,
        &default_choice().unwrap(),
        0.0,
    )
    .unwrap();
    assert!(rods.iter().any(|rod| rod.curve_side == CurveSide::Concave));
    for rod in rods {
        assert_eq!(rod.overbend_deg, 0.0);
        let heads = target_heads(&fit, &anchors, rod.side, &levels, rod.head_offset_mm);
        assert!(heads.windows(2).any(|p| p[0].y != p[1].y));
        for head in heads {
            assert!(distance_to_polyline(head, &rod.points_mm) < 1e-9);
        }
    }
}

fn add_lumbar_levels(fit: &mut SpineFit, anchors: &mut Vec<AnchorProposal>) {
    let prototype = fit.vertebrae[0].clone();
    let anchor_prototype = anchors[0].clone();
    for (i, level) in [Level::L1, Level::L2, Level::L3, Level::L4, Level::L5]
        .into_iter()
        .enumerate()
    {
        let mut v = prototype.clone();
        v.level = level;
        let params = VertebraParams {
            centre: Vector3::new(0.0, 0.0, -24.0 * i as f64),
            rotation: Vector3::zeros(),
            height: 20.0,
            wedge_front: 0.0,
            wedge_side: 0.0,
            top_half_width: 15.0,
            top_half_depth: 10.0,
            bottom_half_width: 15.0,
            bottom_half_depth: 10.0,
        };
        v.pose = params.pose(level);
        v.angles.forward_lean_rad = 0.0;
        fit.vertebrae.push(v);
        for side in [BodySide::Left, BodySide::Right] {
            let mut anchor = anchor_prototype.clone();
            anchor.level = level;
            anchor.side = side;
            anchor.entry_mm = [
                -20.0,
                if side == BodySide::Left { 12.0 } else { -12.0 },
                -24.0 * i as f64,
            ];
            anchors.push(anchor);
        }
    }
    fit.origin_level = Level::L5;
}

#[test]
fn a_lumbar_only_construct_is_not_overbent_even_on_the_concave_side() {
    use implant_layout::rods::clamped_spline;
    let (mut fit, reading, mut anchors) = fixture();
    add_lumbar_levels(&mut fit, &mut anchors);
    let levels = [Level::L1, Level::L2, Level::L3, Level::L4, Level::L5];
    let rods = propose(
        &fit,
        &levels,
        &reading,
        &anchors,
        &default_choice().unwrap(),
    )
    .unwrap();
    assert!(rods.iter().any(|r| r.curve_side == CurveSide::Concave));
    for rod in rods {
        assert_eq!(rod.overbend_deg, 0.0);
        let heads = target_heads(&fit, &anchors, rod.side, &levels, rod.head_offset_mm);
        for &head in &heads {
            assert!(distance_to_polyline(head, &rod.points_mm) < 1e-9);
        }
        let target = clamped_spline(&heads).unwrap();
        assert_eq!(rod.points_mm.len(), target.len());
        for (a, b) in rod.points_mm.iter().zip(target) {
            assert!((Vector3::from(*a) - Vector3::from(b)).norm() < 1e-9);
        }
    }
}

#[test]
fn reflected_spines_exchange_the_two_contours_without_changing_their_shape() {
    let (mut fit, reading, mut anchors) = fixture();
    // Nonzero lateral offsets and body coronal/axial angles exercise the
    // joint correction as well as the later contour reflection.
    for (i, v) in fit.vertebrae.iter_mut().enumerate() {
        v.pose.centre[1] = i as f64 * 0.4;
        let coronal = (i as f64 - 3.5) * 2.0_f64.to_radians();
        let axial = (i as f64 - 3.5) * 1.0_f64.to_radians();
        v.pose.rotation = rotation_vector_from_matrix(
            &(rot_x(coronal) * rot_y(v.angles.forward_lean_rad) * rot_z(axial)),
        )
        .into();
        v.angles.side_tilt_rad = coronal;
        v.angles.twist_rad = axial;
        for anchor in anchors.iter_mut().filter(|a| a.level == v.level) {
            anchor.entry_mm[1] += v.pose.centre[1];
        }
    }
    let mut mirrored = fit.clone();
    for v in &mut mirrored.vertebrae {
        v.pose.centre[1] *= -1.0;
        v.pose.rotation[0] *= -1.0;
        v.pose.rotation[2] *= -1.0;
        v.pose.wedge_front *= -1.0;
        v.angles.side_tilt_rad *= -1.0;
        v.angles.twist_rad *= -1.0;
    }
    let mut mirrored_reading = reading.clone();
    mirrored_reading.curves[0].convex = BodySide::Left;
    mirrored_reading.curves[0].signed_cobb_deg *= -1.0;
    mirrored_reading.curves[0].apical_deviation_mm *= -1.0;
    let mut mirrored_anchors = anchors.clone();
    for anchor in &mut mirrored_anchors {
        anchor.side = if anchor.side == BodySide::Left {
            BodySide::Right
        } else {
            BodySide::Left
        };
        anchor.entry_mm[1] *= -1.0;
        anchor.direction_unit[1] *= -1.0;
    }
    mirrored_anchors.sort_by_key(|a| (a.level, a.side));
    let levels: Vec<_> = fit.vertebrae.iter().map(|v| v.level).collect();
    let original = propose(
        &fit,
        &levels,
        &reading,
        &anchors,
        &default_choice().unwrap(),
    )
    .unwrap();
    let reflected = propose(
        &mirrored,
        &levels,
        &mirrored_reading,
        &mirrored_anchors,
        &default_choice().unwrap(),
    )
    .unwrap();
    for a in original {
        let b = reflected.iter().find(|b| b.side != a.side).unwrap();
        assert_eq!(a.overbend_deg, b.overbend_deg);
        assert!((a.length_mm - b.length_mm).abs() < 1e-9);
        assert_eq!(a.points_mm.len(), b.points_mm.len());
        for (p, q) in a.points_mm.iter().zip(&b.points_mm) {
            assert!((p[0] - q[0]).abs() < 1e-9);
            assert!((p[1] + q[1]).abs() < 1e-9);
            assert!((p[2] - q[2]).abs() < 1e-9);
        }
        assert_eq!(a.standing_points_mm.len(), b.standing_points_mm.len());
        for (p, q) in a.standing_points_mm.iter().zip(&b.standing_points_mm) {
            assert!((p[0] - q[0]).abs() < 1e-9);
            assert!((p[1] + q[1]).abs() < 1e-9);
            assert!((p[2] - q[2]).abs() < 1e-9);
        }
    }
}

#[test]
fn the_first_rods_have_no_extra_and_pass_through_the_standing_heads_of_an_already_target_spine() {
    let (fit, reading, anchors) = fixture();
    let levels: Vec<_> = fit.vertebrae.iter().map(|v| v.level).collect();
    let target = target_spine(&fit, &levels, &reading).unwrap();
    for (expected, actual) in fit.vertebrae.iter().zip(&target.poses) {
        assert!((Vector3::from(actual.centre) - Vector3::from(expected.pose.centre)).norm() < 1e-9);
        let a = matrix_from_rotation_vector(&actual.rotation.into());
        let b = matrix_from_rotation_vector(&expected.pose.rotation.into());
        assert!((a - b).abs().max() < 1e-12);
    }
    let rods = propose(
        &fit,
        &levels,
        &reading,
        &anchors,
        &default_choice().unwrap(),
    )
    .unwrap();
    for rod in rods {
        assert_eq!(rod.overbend_deg, 0.0);
        assert_eq!(rod.standing_points_mm.len(), rod.points_mm.len());
        for (standing, target) in rod.standing_points_mm.iter().zip(&rod.points_mm) {
            assert!((Vector3::from(*standing) - Vector3::from(*target)).norm() < 1e-9);
        }
        assert!(
            !rod.sources.iter().any(|s| s == "cidambi-2012"),
            "no unapplied overbend is cited"
        );
        for head in target_heads(&fit, &anchors, rod.side, &levels, rod.head_offset_mm) {
            assert!(distance_to_polyline(head, &rod.points_mm) < 1e-9);
        }
    }
}

#[test]
fn standing_lines_keep_the_occupied_heads_in_the_target_lines_order() {
    let (mut fit, reading, mut anchors) = fixture();
    for (i, v) in fit.vertebrae.iter_mut().enumerate() {
        v.pose.centre[1] = 0.4 * (i * i) as f64;
        let coronal = (i as f64 - 7.0) * 0.03;
        v.pose.rotation =
            rotation_vector_from_matrix(&(rot_x(coronal) * rot_y(v.angles.forward_lean_rad)))
                .into();
        v.angles.side_tilt_rad = coronal;
        for anchor in anchors.iter_mut().filter(|a| a.level == v.level) {
            anchor.entry_mm[1] += v.pose.centre[1];
            if i == 0 {
                anchor.kind = AnchorKind::Hook;
            } else if i == 3 {
                anchor.kind = AnchorKind::None;
            }
        }
    }
    let levels: Vec<_> = fit.vertebrae.iter().map(|v| v.level).collect();
    let target = target_spine(&fit, &levels, &reading).unwrap();
    let rods = propose(
        &fit,
        &levels,
        &reading,
        &anchors,
        &default_choice().unwrap(),
    )
    .unwrap();
    for rod in rods {
        assert_eq!(rod.standing_points_mm.len(), rod.points_mm.len());
        assert!(
            rod.standing_points_mm
                .iter()
                .zip(&rod.points_mm)
                .any(|(a, b)| { (Vector3::from(*a) - Vector3::from(*b)).norm() > 1.0 })
        );
        let mut previous = None;
        for anchor in anchors
            .iter()
            .filter(|a| a.side == rod.side && a.kind != AnchorKind::None)
        {
            let standing = Vector3::from(anchor.entry_mm)
                - Vector3::from(anchor.direction_unit)
                    * if anchor.kind == AnchorKind::Screw {
                        rod.head_offset_mm
                    } else {
                        0.0
                    };
            let index = rod
                .standing_points_mm
                .iter()
                .position(|p| (Vector3::from(*p) - standing).norm() < 1e-9)
                .expect("each occupied standing head is a drawing knot");
            assert!(previous.is_none_or(|before| before < index));
            previous = Some(index);
            let before = fit
                .vertebrae
                .iter()
                .find(|v| v.level == anchor.level)
                .unwrap();
            let after = target
                .poses
                .iter()
                .find(|p| p.level == anchor.level)
                .unwrap();
            let expected = Vector3::from(after.centre)
                + matrix_from_rotation_vector(&after.rotation.into())
                    * matrix_from_rotation_vector(&before.pose.rotation.into()).transpose()
                    * (standing - Vector3::from(before.pose.centre));
            assert!((Vector3::from(rod.points_mm[index]) - expected).norm() < 1e-8);
        }
    }
}

fn truth_fit(reference: &[VertebraPose]) -> SpineFit {
    let mut fit = fixture().0;
    fit.vertebrae = reference
        .iter()
        .map(|&pose| {
            let rotation = matrix_from_rotation_vector(&pose.rotation.into());
            FittedVertebra {
                level: pose.level,
                pose,
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
        .collect();
    fit.origin_level = reference.last().unwrap().level;
    fit
}

fn layout_input(fit: SpineFit, seed: u32) -> LayoutInput {
    LayoutInput {
        fit,
        front: None,
        bend_left: None,
        bend_right: None,
        child: ChildFacts {
            age_years: Some(15.0),
            weight_kg: None,
            activity: None,
        },
        naming: NamingDoubt::AsRead,
        versions: 1,
        seed,
        pedicle_child_share: None,
    }
}

// Independently invert Rx(coronal)*Ry(sagittal)*Rz(axial). These are
// relative body angles, not the bench's principal deformation log.
fn relative_angles(lower: &VertebraPose, upper: &VertebraPose) -> [f64; 3] {
    let r = matrix_from_rotation_vector(&lower.rotation.into()).transpose()
        * matrix_from_rotation_vector(&upper.rotation.into());
    [
        libm::atan2(-r[(1, 2)], r[(2, 2)]),
        libm::asin(r[(0, 2)].clamp(-1.0, 1.0)),
        libm::atan2(-r[(0, 1)], r[(0, 0)]),
    ]
}

#[test]
fn three_made_up_shapes_turn_joints_without_translating_their_material_points() {
    for recipe in [
        RecipeName::RightThoracic40,
        RecipeName::DoubleMajor55To45,
        RecipeName::LumbarLeft30,
    ] {
        for seed in [1, 2] {
            let patient = make_up_patient(recipe, seed).unwrap();
            let reference: Vec<_> = patient
                .standing
                .levels
                .iter()
                .zip(&patient.standing.vertebrae)
                .filter(|(level, _)| {
                    (Level::T1..=Level::L5).contains(level) && **level != Level::T13
                })
                .map(|(&level, v)| v.pose(level))
                .collect();
            let input = layout_input(truth_fit(&reference), seed);
            let reading = implant_layout::curves::measure(&input, &input.fit).unwrap();
            let major = &reading.curves[reading.major.unwrap()];
            let first = major.upper_index;
            let last = major.lower_index;
            let instrumented = &reference[first..=last];
            let levels: Vec<_> = instrumented.iter().map(|p| p.level).collect();
            let target = target_spine(&input.fit, &levels, &reading).unwrap();
            assert_eq!(
                *target.poses.last().unwrap(),
                *instrumented.last().unwrap(),
                "lowest instrumented pose is the standing pose"
            );
            // Kyphosis is the projected angle of the actual endplate
            // normals, not the sum of finite relative Euler Y angles.
            // An endpoint above the span is carried rigidly with its top
            // body; an endpoint below it retains its standing normal.
            let top_change = matrix_from_rotation_vector(&target.poses[0].rotation.into())
                * matrix_from_rotation_vector(&instrumented[0].rotation.into()).transpose();
            let endpoint_normal = |level, top| {
                let index = reference.iter().position(|p| p.level == level).unwrap();
                let pose = if (first..=last).contains(&index) {
                    &target.poses[index - first]
                } else {
                    &reference[index]
                };
                let plates = Endplates::of(&VertebraParams::from(pose));
                let normal = if top {
                    plates.top.normal
                } else {
                    plates.bottom.normal
                };
                if index < first {
                    top_change * normal
                } else {
                    normal
                }
            };
            let a = endpoint_normal(Level::T5, true);
            let b = endpoint_normal(Level::T12, false);
            let actual = (libm::atan2(a.x, a.z) - libm::atan2(b.x, b.z)).to_degrees();
            let goal = reading.kyphosis_t5_t12_deg.unwrap().clamp(
                published_scalar(
                    Table::LevelRuleThresholds,
                    "normal-kyphosis-min",
                    "sagittal-modifier",
                ),
                published_scalar(
                    Table::LevelRuleThresholds,
                    "normal-kyphosis-max",
                    "sagittal-modifier",
                ),
            );
            assert!(
                (actual - goal).abs() < 1e-7,
                "the projected T5/T12 target is met: {actual}, expected {goal}"
            );
            let mut scale: Option<f64> = None;
            for i in 0..instrumented.len() - 1 {
                let upper = &instrumented[i];
                let lower = &instrumented[i + 1];
                let ru = matrix_from_rotation_vector(&upper.rotation.into());
                let rl = matrix_from_rotation_vector(&lower.rotation.into());
                let point = (Endplates::of(&VertebraParams::from(upper)).bottom.centre
                    + Endplates::of(&VertebraParams::from(lower)).top.centre)
                    * 0.5;
                let a = ru.transpose() * (point - Vector3::from(upper.centre));
                let b = rl.transpose() * (point - Vector3::from(lower.centre));
                let tu = &target.poses[i];
                let tl = &target.poses[i + 1];
                let upper_point =
                    Vector3::from(tu.centre) + matrix_from_rotation_vector(&tu.rotation.into()) * a;
                let lower_point =
                    Vector3::from(tl.centre) + matrix_from_rotation_vector(&tl.rotation.into()) * b;
                assert!(
                    (upper_point - lower_point).norm() < 1e-9,
                    "{recipe:?}/{seed} {:?}/{:?}: material joint separation",
                    upper.level,
                    lower.level
                );
                let standing = relative_angles(lower, upper);
                let changed = relative_angles(tl, tu);
                assert!(changed[0].abs() < 1e-9, "coronal component is removed");
                assert!(
                    (changed[2] - standing[2]).abs() < 1e-9,
                    "axial component is kept"
                );
                let eligible =
                    (Level::T5..=Level::T11).contains(&upper.level) && lower.level <= Level::T12;
                if eligible && standing[1].abs() > 1e-9 {
                    let ratio = changed[1] / standing[1];
                    if let Some(previous) = scale {
                        assert!((ratio - previous).abs() < 1e-8, "one common sagittal scale");
                    } else {
                        scale = Some(ratio);
                    }
                } else if !eligible {
                    assert!(
                        (changed[1] - standing[1]).abs() < 1e-9,
                        "T12/L1 and every other unscaled joint keep sagittal angle"
                    );
                }
            }
        }
    }
}

#[test]
fn relabelling_only_lumbar_levels_does_not_move_the_joint_target() {
    let (mut fit, reading, mut anchors) = fixture();
    add_lumbar_levels(&mut fit, &mut anchors);
    let levels: Vec<_> = fit.vertebrae.iter().map(|v| v.level).collect();
    let original = target_spine(&fit, &levels, &reading).unwrap();
    let mut renamed = fit.clone();
    for v in &mut renamed.vertebrae {
        if v.level >= Level::L1 {
            let new = Level::ALL[v.level.order() + 1];
            v.level = new;
            v.pose.level = new;
        }
    }
    renamed.origin_level = Level::L6;
    let levels: Vec<_> = renamed.vertebrae.iter().map(|v| v.level).collect();
    let result = target_spine(&renamed, &levels, &reading).unwrap();
    for (a, b) in original.poses.iter().zip(&result.poses) {
        assert!((Vector3::from(a.centre) - Vector3::from(b.centre)).norm() < 1e-9);
        assert!(
            (matrix_from_rotation_vector(&a.rotation.into())
                - matrix_from_rotation_vector(&b.rotation.into()))
            .abs()
            .max()
                < 1e-12
        );
    }
}
