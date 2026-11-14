//! Hand balances, locked freedoms and the three exact linear-case laws.

use forward_model::project::{front_marks, side_marks};
use load_bench::chain::{Chain, Ground, Pose, Wrench};
use load_bench::daily_loads::{self, DailyInputs, GRAVITY_N_PER_KG, LOAD_LEVELS};
use load_bench::evidence::{Evidence, TableKind};
use load_bench::joint_springs::{Stiffness, log_left_derivative};
use load_bench::junctions;
use load_bench::rigid_rod::{self, Anchor, Construct, HeadKind, HeldOptions, Rod};
use load_bench::rod_beams::{BeamProperties, CondensedRod, polyline_stations};
use load_bench::springback::{self, HeadTie, LockedRod};
use made_up_spines::film_layout::FilmLayout;
use made_up_spines::patient::make_up_patient;
use made_up_spines::recipes::film_recipes;
use nalgebra::{DVector, Matrix3, Vector3};
use shapes::confidence::{Confidence, ConfidenceBand};
use shapes::film::FilmKind;
use shapes::landmarks::Landmarks;
use shapes::levels::Level;
use shapes::made_up::{MadeUpRecipe, RecipeName, Tier, VertebraPose};
use shapes::naming::{ChosenBy, LevelNaming, Template};
use shapes::spine_fit::{FilmToFit, FitFromFilms, FitLengths, MarkSpread, TypicalSource};
use shapes::vertebra::{FilmMarks, VertebraMarks, VertebraOnFilm};

fn vertebra(level: Level, z: f64) -> VertebraPose {
    VertebraPose {
        level,
        centre: [0.0, 0.0, z],
        rotation: [0.0; 3],
        height: 20.0,
        wedge_front: 0.0,
        wedge_side: 0.0,
        top_half_width: 12.0,
        top_half_depth: 8.0,
        bottom_half_width: 12.0,
        bottom_half_depth: 8.0,
    }
}

fn chain(reference: &[VertebraPose], stiffness: Stiffness) -> Chain {
    Chain::new(
        reference,
        &vec![stiffness; reference.len()],
        Ground {
            top_mm: Vector3::zeros(),
            rotation: Matrix3::identity(),
        },
    )
    .unwrap()
}

fn small_chain() -> Chain {
    let reference: Vec<_> = [Level::L2, Level::L3, Level::L4, Level::L5]
        .iter()
        .enumerate()
        .map(|(i, &l)| vertebra(l, 140.0 - 40.0 * i as f64))
        .collect();
    chain(
        &reference,
        Stiffness::new([80.0, 120.0, 200.0, 3000.0, 5000.0, 4000.0]).unwrap(),
    )
}

fn locked_rods(poses: &[Pose], bodies: &[usize], locks_rotation: bool) -> Vec<LockedRod> {
    // Synthetic section and head offsets, not a catalogue choice.
    [-9.0, 9.0]
        .iter()
        .map(|&side| {
            let points: Vec<_> = bodies
                .iter()
                .map(|&i| poses[i].centre_mm + poses[i].rotation * Vector3::new(-15.0, side, 0.0))
                .collect();
            let beam = CondensedRod::new(
                &points,
                &polyline_stations(&points).unwrap(),
                BeamProperties {
                    young_modulus_mpa: 70000.0,
                    shear_modulus_mpa: 28000.0,
                    diameter_mm: 5.0,
                    shear_coefficient: 0.85,
                },
            )
            .unwrap();
            LockedRod {
                ties: bodies
                    .iter()
                    .zip(points)
                    .map(|(&body, point_mm)| HeadTie {
                        body,
                        point_mm,
                        locks_rotation,
                    })
                    .collect(),
                stiffness: beam.stiffness,
            }
        })
        .collect()
}

#[test]
fn compression_keeps_the_published_weight_and_has_no_offset_moment() {
    let evidence = Evidence::read().unwrap();
    let reference: Vec<_> = LOAD_LEVELS
        .iter()
        .enumerate()
        .map(|(i, &l)| vertebra(l, 660.0 - 40.0 * i as f64))
        .collect();
    let chain = chain(&reference, Stiffness::new([100.0; 6]).unwrap());
    let read = |row, term| {
        evidence
            .value(TableKind::DailyLoads, row, term)
            .unwrap()
            .value
    };
    let top = read("trunk-slice-endpoints", "t1");
    let bottom = read("trunk-slice-endpoints", "l5");
    let head = read("head-neck", "fraction");
    let arm = read("each-arm-hand", "fraction");
    let offset = read("slice-offset", "mean");
    let weight = 40.0 * GRAVITY_N_PER_KG;
    let loads = daily_loads::gravity_loads(&evidence, &chain, 40.0, offset).unwrap();
    // Arithmetic progression: sum = n(first+last)/2. The straight column
    // carries only compression, whatever the unused offset draw says.
    let slices = 17.0 * (top + bottom) / 2.0;
    let sum = loads.body_loads.iter().copied().sum::<Wrench>();
    assert!((sum[2] + weight * (slices + head + 2.0 * arm)).abs() < 1e-12);
    assert_eq!(sum[4], 0.0);
    assert_eq!(
        loads.body_loads,
        daily_loads::gravity_loads(&evidence, &chain, 40.0, -100.0)
            .unwrap()
            .body_loads
    );
    assert_eq!(sum[0], 0.0);
    assert_eq!(sum[1], 0.0);
    assert_eq!(sum[3], 0.0);
    assert_eq!(sum[5], 0.0);
    assert!((loads.body_loads[0][2] + weight * (top + head + arm / 2.0)).abs() < 1e-12);
    assert!((loads.body_loads[4][2] + weight * (top + (bottom - top) / 4.0)).abs() < 1e-12);
    assert!(loads.sources.iter().all(|s| evidence.source(s).is_some()));
    let bare =
        daily_loads::solve_case(&chain, &chain.standing(), &[], &[], &loads.body_loads).unwrap();
    // Every cut is on the compression line, so every intrinsic disc couple
    // is zero, including at the sacrum.
    assert!(bare.joint_moments_nmm.iter().all(|m| m.norm() < 1e-7));
    assert!((bare.joint_moments_nmm[16].y - sum[4]).abs() < 1e-7);
}

#[test]
fn axial_force_is_shared_by_the_hand_worked_parallel_stiffnesses() {
    let reference = [vertebra(Level::L4, 60.0), vertebra(Level::L5, 20.0)];
    let chain = chain(
        &reference,
        Stiffness::new([30.0, 30.0, 30.0, 3000.0, 3000.0, 3000.0]).unwrap(),
    );
    let poses = chain.standing();
    let points: Vec<_> = poses.iter().map(|p| p.centre_mm).collect();
    // Diameter 2 gives area pi. E=30*40/pi gives EA/L=30 N/mm,
    // in parallel with the 30 N/mm joint, followed by the ground spring.
    let beam = CondensedRod::new(
        &points,
        &polyline_stations(&points).unwrap(),
        BeamProperties {
            young_modulus_mpa: 1200.0 / core::f64::consts::PI,
            shear_modulus_mpa: 150.0,
            diameter_mm: 2.0,
            shear_coefficient: 0.8,
        },
    )
    .unwrap();
    let rod = LockedRod {
        ties: points
            .iter()
            .enumerate()
            .map(|(body, &point_mm)| HeadTie {
                body,
                point_mm,
                locks_rotation: true,
            })
            .collect(),
        stiffness: beam.stiffness,
    };
    let rods = springback::prepare_rods(&poses, &[rod]).unwrap();
    let loads = [Wrench::new(0.0, 0.0, -12.0, 0.0, 0.0, 0.0), Wrench::zeros()];
    let response =
        daily_loads::solve_case(&chain, &poses, &rods, &[vec![Wrench::zeros(); 2]], &loads)
            .unwrap();
    assert!((response.displacement[2] + 0.6).abs() < 1e-12);
    assert!((response.displacement[8] + 0.4).abs() < 1e-12);
    assert!((response.head_wrenches_on_rod[0][0][2] + 6.0).abs() < 1e-11);
    assert!((response.head_wrenches_on_rod[0][1][2] - 6.0).abs() < 1e-11);
}

#[test]
fn three_body_follower_compression_has_the_hand_worked_head_forces() {
    let reference = [
        vertebra(Level::L3, 100.0),
        vertebra(Level::L4, 60.0),
        vertebra(Level::L5, 20.0),
    ];
    let chain = chain(
        &reference,
        Stiffness::new([30.0, 30.0, 30.0, 3000.0, 3000.0, 3000.0]).unwrap(),
    );
    let poses = chain.standing();
    let loads = daily_loads::compression_loads(&chain, &poses, &[12.0, 6.0, 3.0]).unwrap();
    // Above the three cuts: 12, 18, 21 N downward. Straight centred cuts
    // have no moment, independently of the constitutive model.
    let cuts = chain.joint_motion_frames(&poses).unwrap();
    for (i, cut) in cuts.iter().enumerate() {
        let mut force = Vector3::zeros();
        let mut moment = Vector3::zeros();
        for j in 0..=i {
            let f = loads[j].fixed_rows::<3>(0).into_owned();
            force += f;
            moment +=
                loads[j].fixed_rows::<3>(3) + (poses[j].centre_mm - cut.upper_point_mm).cross(&f);
        }
        assert_eq!(force, Vector3::new(0.0, 0.0, -[12.0, 18.0, 21.0][i]));
        assert_eq!(moment, Vector3::zeros());
    }
    // Each rod has EA/L=15 N/mm. The rod pair and 30 N/mm disc each
    // carry half the 12 and 18 N crossing the two instrumented joints.
    // Per rod: head forces -3, -1.5, +4.5 N. The bottom disc carries all
    // 21 N, so u_bottom=-.7, u_middle=-1, u_top=-1.2 mm.
    let locked: Vec<_> = [-9.0, 9.0]
        .iter()
        .map(|&side| {
            let points: Vec<_> = poses
                .iter()
                .map(|p| p.centre_mm + Vector3::new(0.0, side, 0.0))
                .collect();
            let beam = CondensedRod::new(
                &points,
                &polyline_stations(&points).unwrap(),
                BeamProperties {
                    young_modulus_mpa: 600.0 / core::f64::consts::PI,
                    shear_modulus_mpa: 75.0,
                    diameter_mm: 2.0,
                    shear_coefficient: 0.8,
                },
            )
            .unwrap();
            LockedRod {
                ties: points
                    .into_iter()
                    .enumerate()
                    .map(|(body, point_mm)| HeadTie {
                        body,
                        point_mm,
                        locks_rotation: true,
                    })
                    .collect(),
                stiffness: beam.stiffness,
            }
        })
        .collect();
    let rods = springback::prepare_rods(&poses, &locked).unwrap();
    let response = daily_loads::solve_case(
        &chain,
        &poses,
        &rods,
        &vec![vec![Wrench::zeros(); 3]; 2],
        &loads,
    )
    .unwrap();
    for (body, expected) in [-1.2, -1.0, -0.7].iter().enumerate() {
        assert!((response.displacement[6 * body + 2] - expected).abs() < 1e-12);
    }
    for rod in &response.head_wrenches_in_body_frame {
        for (w, expected) in rod.iter().zip([-3.0, -1.5, 4.5]) {
            assert!((w[2] - expected).abs() < 1e-10);
            assert!(w.fixed_rows::<2>(0).norm() < 1e-10);
        }
    }
}

#[test]
fn compression_has_zero_prefix_moment_at_every_disc_of_a_curved_column() {
    let mut reference = small_chain().reference;
    for (i, p) in reference.iter_mut().enumerate() {
        p.centre[0] = [3.0, -1.0, 2.0, 0.0][i];
        p.centre[1] = [2.0, 5.0, -3.0, 0.0][i];
        p.rotation = [0.02 * i as f64, -0.03, 0.01];
    }
    let chain = chain(&reference, Stiffness::new([100.0; 6]).unwrap());
    let poses = chain.standing();
    let loads = daily_loads::compression_loads(&chain, &poses, &[12.0, 6.0, 3.0, 4.0]).unwrap();
    for (i, cut) in chain
        .joint_motion_frames(&poses)
        .unwrap()
        .iter()
        .enumerate()
    {
        let mut moment = Vector3::zeros();
        let mut force = Vector3::zeros();
        for j in 0..=i {
            let f = loads[j].fixed_rows::<3>(0).into_owned();
            force += f;
            moment +=
                loads[j].fixed_rows::<3>(3) + (poses[j].centre_mm - cut.upper_point_mm).cross(&f);
        }
        assert!(moment.norm() < 1e-12);
        assert!(force.cross(&poses[i].rotation.column(2)).norm() < 1e-12);
    }
}

#[test]
fn a_daily_rotation_past_the_method_limit_is_refused_including_scaled_cases() {
    use load_bench::daily_loads::{DailyError, SMALL_ANGLE_LIMIT_RAD};
    let chain = chain(
        &[vertebra(Level::L5, 20.0)],
        Stiffness::new([100.0; 6]).unwrap(),
    );
    let poses = chain.standing();
    // Force-free single-disc statics: theta = M / k_rotation. The body
    // centre translates with its disc pivot; no material lever is fitted.
    let load = Wrench::new(
        0.0,
        0.0,
        0.0,
        100.0 * SMALL_ANGLE_LIMIT_RAD * 1.01,
        0.0,
        0.0,
    );
    assert!(matches!(
        daily_loads::solve_case(&chain, &poses, &[], &[], &[load]),
        Err(DailyError::SmallAngle)
    ));
    let response = daily_loads::solve_case(&chain, &poses, &[], &[], &[load * 0.6]).unwrap();
    assert!(matches!(response.scaled(2.0), Err(DailyError::SmallAngle)));
}

#[test]
fn screws_carry_couples_and_hooks_release_them() {
    let chain = small_chain();
    let poses = chain.standing();
    let mut loads = vec![Wrench::zeros(); 4];
    loads[1][4] = 10.0;
    let mut screw_peak = 0.0_f64;
    for locks in [true, false] {
        let rods = springback::prepare_rods(&poses, &locked_rods(&poses, &[1, 2], locks)).unwrap();
        let response = daily_loads::solve_case(
            &chain,
            &poses,
            &rods,
            &vec![vec![Wrench::zeros(); 2]; 2],
            &loads,
        )
        .unwrap();
        let peak = response
            .head_wrenches_on_rod
            .iter()
            .flatten()
            .map(|w| w.fixed_rows::<3>(3).norm())
            .fold(0.0_f64, f64::max);
        if locks {
            screw_peak = peak;
        } else {
            assert_eq!(peak, 0.0);
        }
    }
    assert!(screw_peak > 0.1);
}

#[test]
fn release_uses_the_printed_axes_and_junctions_use_the_outside_discs() {
    let evidence = Evidence::read().unwrap();
    let chain = small_chain();
    let preload = evidence
        .value(TableKind::JointSprings, "preload", "lower")
        .unwrap()
        .value;
    let changed = daily_loads::loaded_chain(&evidence, &chain, preload, &[Level::L3]).unwrap();
    let before = chain.joint_stiffnesses();
    let after = changed.joint_stiffnesses();
    for i in 0..4 {
        for (axis, term) in [
            None,
            None,
            Some("compression"),
            Some("coronal"),
            Some("flexion"),
            Some("axial-rotation"),
        ]
        .iter()
        .enumerate()
        {
            let fraction = if i == 1 {
                term.map(|term| {
                    1.0 + evidence
                        .value(TableKind::JointSprings, "release", term)
                        .unwrap()
                        .value
                        / 100.0
                })
                .unwrap_or(1.0)
            } else {
                1.0
            };
            assert!((after[i].0[axis] - before[i].0[axis] * preload * fraction).abs() < 1e-10);
        }
    }
    let empty = daily_loads::solve_case(&chain, &chain.standing(), &[], &[], &[Wrench::zeros(); 4])
        .unwrap();
    let levels = [Level::L2, Level::L3, Level::L4, Level::L5];
    assert_eq!(
        junctions::ratios(&levels, Level::L2, Level::L5, &empty, &empty).unwrap(),
        junctions::JunctionRatios {
            above: None,
            below: None
        }
    );
    let mut before = empty.clone();
    let mut after = empty;
    before.joint_moments_nmm = [3.0, 10.0, 4.0, 9.0].map(|m| Vector3::x() * m).to_vec();
    after.joint_moments_nmm = [6.0, 50.0, 12.0, 90.0].map(|m| Vector3::x() * m).to_vec();
    assert_eq!(
        junctions::ratios(&levels, Level::L3, Level::L4, &after, &before).unwrap(),
        junctions::JunctionRatios {
            above: Some(2.0),
            below: Some(3.0)
        }
    );
    // Ending at L5 still leaves the explicitly modelled L5/sacrum disc.
    assert_eq!(
        junctions::ratios(&levels, Level::L2, Level::L5, &after, &before).unwrap(),
        junctions::JunctionRatios {
            above: None,
            below: Some(10.0)
        }
    );
}

#[test]
fn weight_scales_every_daily_wrench_and_common_stiffness_scaling_changes_none() {
    let chain = small_chain();
    let mut deformation = vec![Wrench::zeros(); 4];
    deformation[1][4] = 0.05;
    let rigid = chain.poses_from_deformations(&deformation).unwrap();
    let state =
        springback::solve(&chain, &rigid, &locked_rods(&rigid, &[1, 2], true), &rigid).unwrap();
    // Keep both the base and doubled synthetic cases within the stated
    // small-angle method; the separate refusal test covers larger turns.
    let loads = vec![Wrench::new(0.2, -0.1, -0.4, 0.05, 0.3, -0.02); 4];
    let response = daily_loads::solve_case(
        &chain,
        &state.poses,
        &state.corrected_rods,
        &state.head_wrenches_on_rod,
        &loads,
    )
    .unwrap();
    let doubled = daily_loads::solve_case(
        &chain,
        &state.poses,
        &state.corrected_rods,
        &state.head_wrenches_on_rod,
        &loads.iter().map(|w| w * 2.0).collect::<Vec<_>>(),
    )
    .unwrap();
    assert!((&doubled.displacement - &response.displacement * 2.0).norm() < 1e-10);
    for (a, b) in response
        .head_wrenches_on_rod
        .iter()
        .flatten()
        .zip(doubled.head_wrenches_on_rod.iter().flatten())
    {
        assert!((b - a * 2.0).norm() < 1e-9);
    }
    let before = daily_loads::solve_case(&chain, &chain.standing(), &[], &[], &loads).unwrap();
    let levels = [Level::L2, Level::L3, Level::L4, Level::L5];
    let ratio = junctions::ratios(&levels, Level::L3, Level::L4, &response, &before).unwrap();
    let twice = junctions::ratios(
        &levels,
        Level::L3,
        Level::L4,
        &doubled,
        &before.scaled(2.0).unwrap(),
    )
    .unwrap();
    assert!((ratio.above.unwrap() - twice.above.unwrap()).abs() < 1e-10);
    assert!((ratio.below.unwrap() - twice.below.unwrap()).abs() < 1e-10);
    let scale = 4.0;
    let rods: Vec<_> = state
        .corrected_rods
        .iter()
        .map(|r| LockedRod {
            ties: r.ties.clone(),
            stiffness: &r.stiffness * scale,
        })
        .collect();
    let prepared = springback::prepare_rods(&state.poses, &rods).unwrap();
    let preload: Vec<Vec<_>> = state
        .head_wrenches_on_rod
        .iter()
        .map(|r| r.iter().map(|w| w * scale).collect())
        .collect();
    let scaled = daily_loads::solve_case(
        &chain.scaled(scale).unwrap(),
        &state.poses,
        &prepared,
        &preload,
        &loads,
    )
    .unwrap();
    assert!((&scaled.displacement * scale - &response.displacement).norm() < 1e-9);
    for (a, b) in response
        .head_wrenches_on_rod
        .iter()
        .flatten()
        .zip(scaled.head_wrenches_on_rod.iter().flatten())
    {
        assert!((b - a).norm() < 1e-8);
    }
    for (a, b) in response
        .joint_moments_nmm
        .iter()
        .zip(&scaled.joint_moments_nmm)
    {
        assert!((a - b).norm() < 1e-8);
    }
}

#[test]
fn disc_couple_increments_match_an_independent_log_spring_derivative() {
    let chain = small_chain();
    let mut q = vec![Wrench::zeros(); 4];
    q[1] = Wrench::new(0.3, -0.2, 0.1, 0.4, -0.3, 0.2);
    q[2] = Wrench::new(-0.2, 0.1, 0.0, -0.1, 0.2, 0.15);
    let poses = chain.poses_from_deformations(&q).unwrap();
    let u = DVector::from_fn(24, |i, _| (i as f64 - 10.0) / 17.0);
    let calculated = junctions::moment_increments(&chain, &poses, &u).unwrap();
    let couples = |sign: f64| {
        let moved: Vec<_> = poses
            .iter()
            .enumerate()
            .map(|(i, p)| {
                p.incremented(
                    &(Wrench::from_column_slice(u.rows(6 * i, 6).as_slice()) * (sign * 1e-6)),
                )
            })
            .collect();
        let evaluated = chain.evaluate(&moved).unwrap();
        evaluated
            .deformation
            .iter()
            .zip(chain.joint_stiffnesses())
            .enumerate()
            .map(|(i, (q, k))| {
                let phi = q.fixed_rows::<3>(3).into_owned();
                let local = log_left_derivative(&phi).transpose()
                    * k.0.fixed_rows::<3>(3).component_mul(&phi);
                moved.get(i + 1).unwrap_or(&chain.ground).rotation * local
            })
            .collect::<Vec<_>>()
    };
    for (i, (plus, minus)) in couples(1.0).iter().zip(couples(-1.0)).enumerate() {
        assert!(
            ((plus - minus) / 2e-6 - calculated[i]).norm() < 2e-5,
            "joint {i}"
        );
    }
}

#[test]
fn a_supplied_rest_fitting_contour_needs_no_holding_force() {
    let mut reference = small_chain().reference;
    // A deliberately oblique contour plane: the chord points down z and
    // its middle bows in both x and y. All heads fit exactly at rest.
    reference[1].centre[0] = 3.0;
    reference[1].centre[1] = 2.0;
    let chain = chain(
        &reference,
        Stiffness::new([80.0, 120.0, 200.0, 3000.0, 5000.0, 4000.0]).unwrap(),
    );
    let standing = chain.standing();
    let mut construct = Construct {
        rods: Vec::new(),
        anchors: Vec::new(),
    };
    for (rod, side) in [-9.0, 9.0].into_iter().enumerate() {
        let points: Vec<_> = (0..3)
            .map(|i| standing[i].centre_mm + Vector3::new(-15.0, side, 0.0))
            .collect();
        let stations = polyline_stations(&points).unwrap();
        for (body, &station_mm) in stations.iter().enumerate() {
            construct.anchors.push(Anchor {
                body,
                point_mm: Vector3::new(-15.0, side, 0.0),
                rod,
                station_mm,
                kind: HeadKind::Screw,
            });
        }
        construct.rods.push(Rod {
            points_mm: points,
            properties: BeamProperties::from_young_modulus(70000.0, 5.0),
        });
    }
    let held = rigid_rod::solve_held(&chain, &construct, &HeldOptions::default(), None).unwrap();
    // Zero is the independently known global minimum of the nonnegative
    // joint energy. The supplied rest placement is feasible under the
    // specified zero twist from the supplied rod orientation.
    assert!(
        held.settled,
        "rest-fitting supplied contour: {:?}",
        held.reason
    );
    assert!(
        held.energy_nmm.abs() < 1e-9,
        "rest-fitting supplied contour energy {}",
        held.energy_nmm
    );
    assert!(held.head_forces_on_rod.iter().all(|f| f.norm() < 1e-9));
}

fn run_fitted_case(front_only: bool) {
    let evidence = Evidence::read().unwrap();
    let recipe = MadeUpRecipe {
        name: RecipeName::Straight,
        seed: 7,
        tier: Tier::T0,
    };
    let patient = make_up_patient(recipe.name, recipe.seed).unwrap();
    let films = film_recipes(&recipe, u64::from(recipe.seed));
    let to_fit = |kind| {
        let film = films.iter().find(|f| f.kind == kind).unwrap();
        let layout = FilmLayout::new(film, &patient.standing);
        let vertebrae = patient
            .standing
            .levels
            .iter()
            .zip(&patient.standing.vertebrae)
            .filter(|(l, _)| LOAD_LEVELS.contains(l))
            .enumerate()
            .map(|(i, (&level, v))| {
                let marks = if kind == FilmKind::Front {
                    VertebraMarks::Front {
                        points: front_marks(v, &layout.camera).unwrap(),
                    }
                } else {
                    VertebraMarks::Side {
                        points: side_marks(v, &layout.camera).unwrap(),
                    }
                };
                VertebraOnFilm {
                    id: i as u32 + 1,
                    level,
                    marks,
                    confidence: Confidence {
                        score: 1.0,
                        band: ConfidenceBand::High,
                        reasons: Vec::new(),
                        calibrated: false,
                        endplate_spread_px: None,
                        margin: None,
                    },
                    outline: None,
                }
            })
            .collect();
        FilmToFit {
            marks: FilmMarks {
                film: kind,
                vertebrae,
                landmarks: Landmarks::NONE,
                naming: LevelNaming {
                    template: Template::Standard,
                    chosen_by: ChosenBy::User,
                    alternatives: Vec::new(),
                    from_top: None,
                    from_bottom: None,
                    flags: Vec::new(),
                    l5_confirmed_on: None,
                },
                named_for: layout.geometry.image_right.unwrap(),
            },
            geometry: layout.geometry,
            size: layout.size,
            mark_spread: MarkSpread::ShareOfHeight {
                share: 0.03,
                shared_share: 0.0,
            },
            facing_assumed: false,
        }
    };
    let mut front = to_fit(FilmKind::Front);
    if front_only {
        front.geometry.mm_per_px = None;
        front.geometry.mm_per_px_vertical = None;
        front.geometry.scale = None;
        front.geometry.confirmed.scale = false;
    }
    let fit = joint_fit::fit_from_films(&FitFromFilms {
        front,
        side: (!front_only).then(|| to_fit(FilmKind::Side)),
        typical: TypicalSource::BuiltIn,
        twist_readings: Vec::new(),
    })
    .unwrap();
    assert_eq!(fit.side_film.is_none(), front_only);
    assert_eq!(
        fit.lengths,
        if front_only {
            FitLengths::TypicalSize
        } else {
            FitLengths::Measured
        }
    );
    let reference: Vec<_> = fit.vertebrae.iter().map(|v| v.pose).collect();
    let stiffness: Vec<_> = reference
        .iter()
        .map(|p| Stiffness::from_evidence(&evidence, p.level == Level::L5).unwrap())
        .collect();
    // The film has no sacrum marks. An explicit ground at the fit's
    // origin, aligned with the lowest body, is a synthetic test boundary.
    let chain = Chain::new(
        &reference,
        &stiffness,
        Ground {
            top_mm: Vector3::zeros(),
            rotation: Pose::from_vertebra(reference.last().unwrap()).rotation,
        },
    )
    .unwrap();
    let standing = chain.standing();
    let modulus = evidence
        .value(TableKind::RodMaterials, "cobalt-chrome", "young-modulus")
        .unwrap()
        .value
        * 1000.0;
    let mut construct = Construct {
        rods: Vec::new(),
        anchors: Vec::new(),
    };
    for (rod, side) in [-8.0, 8.0].into_iter().enumerate() {
        let points: Vec<_> = [5, 7]
            .into_iter()
            .map(|i| standing[i].centre_mm + standing[i].rotation * Vector3::new(-12.0, side, 0.0))
            .collect();
        let stations = polyline_stations(&points).unwrap();
        for (head, body) in [5, 7].into_iter().enumerate() {
            construct.anchors.push(Anchor {
                body,
                point_mm: Vector3::new(-12.0, side, 0.0),
                rod,
                station_mm: stations[head],
                kind: HeadKind::Screw,
            });
        }
        construct.rods.push(Rod {
            points_mm: points,
            properties: BeamProperties::from_young_modulus(modulus, 4.0),
        });
    }
    let held = rigid_rod::solve_held(&chain, &construct, &HeldOptions::default(), None).unwrap();
    assert!(held.settled, "{:?}", held.reason);
    let rigid = rigid_rod::solve_rigid(&chain, &construct, &held, None).unwrap();
    assert!(rigid.settled, "{:?}", rigid.reason);
    let seated = springback::solve(&chain, &rigid.poses, &rigid.rods, &held.poses).unwrap();
    let read = |row, term| {
        evidence
            .value(TableKind::DailyLoads, row, term)
            .unwrap()
            .value
    };
    let input = DailyInputs {
        weight_kg: 40.0,
        slice_offset_mm: read("slice-offset", "mean"),
        preload_factor: evidence
            .value(TableKind::JointSprings, "preload", "lower")
            .unwrap()
            .value,
        walking_multiple: read("walking", "lower"),
        bending_multiple: read("ventral-flexion", "lower"),
    };
    let cases =
        daily_loads::solve(&evidence, &chain, &seated, &[Level::T6, Level::T7], input).unwrap();
    let double = daily_loads::solve(
        &evidence,
        &chain,
        &seated,
        &[Level::T6, Level::T7],
        DailyInputs {
            weight_kg: 80.0,
            ..input
        },
    )
    .unwrap();
    for (a, b) in cases
        .standing
        .head_wrenches_on_rod
        .iter()
        .flatten()
        .zip(double.standing.head_wrenches_on_rod.iter().flatten())
    {
        assert!((a * 2.0 - b).norm() < 1e-7);
    }
    let levels: Vec<_> = reference.iter().map(|p| p.level).collect();
    let r = junctions::ratios(
        &levels,
        Level::T6,
        Level::T8,
        &cases.standing,
        &cases.without_construct,
    )
    .unwrap();
    let r2 = junctions::ratios(
        &levels,
        Level::T6,
        Level::T8,
        &double.standing,
        &double.without_construct,
    )
    .unwrap();
    assert_eq!(
        r,
        junctions::JunctionRatios {
            above: None,
            below: None
        }
    );
    assert_eq!(r2, r);
    for (a, b) in cases
        .standing
        .head_wrenches_on_rod
        .iter()
        .flatten()
        .zip(cases.walking_peak.head_wrenches_on_rod.iter().flatten())
    {
        assert_eq!(a * input.walking_multiple, *b);
    }
    assert!(cases.sources.iter().all(|s| evidence.source(s).is_some()));
    let demand = load_bench::anchor_demand::at_head(
        &evidence,
        Level::T6,
        shapes::implant_layout::AnchorKind::Screw,
        (seated.head_wrenches_on_rod[0][0] + cases.standing.head_wrenches_on_rod[0][0])
            .fixed_rows::<3>(0)
            .into_owned(),
        1.0,
        0.5,
    )
    .unwrap();
    assert!(demand.demand.is_finite());
    // Continue through rod statics to the annual comparison. This minimal
    // two-head rod has one span and no holding-roll constraint.
    let rod = &seated.corrected_rods[0];
    let polyline = &rod.head_points_mm;
    let stations = polyline_stations(polyline).unwrap();
    let heads: Vec<_> = rod
        .ties
        .iter()
        .enumerate()
        .map(|(i, tie)| {
            let w = &seated.head_wrenches_on_rod[0][i];
            load_bench::rod_statics::HeadLoad {
                station_mm: stations[i],
                position_mm: tie.point_mm,
                force_n: w.fixed_rows::<3>(0).into_owned(),
                couple_nmm: w.fixed_rows::<3>(3).into_owned(),
            }
        })
        .collect();
    let cut = stations[1] / 2.0;
    let locked = load_bench::rod_statics::section_at(polyline, &heads, cut).unwrap();
    let increments: Vec<_> = cases.standing.head_wrenches_on_rod[0]
        .iter()
        .enumerate()
        .map(|(i, w)| load_bench::rod_statics::HeadIncrement {
            point_mm: cases.standing.head_displacements_mm[0][i],
            force_n: w.fixed_rows::<3>(0).into_owned(),
            couple_nmm: w.fixed_rows::<3>(3).into_owned(),
        })
        .collect();
    let cut_motion = (increments[0].point_mm + increments[1].point_mm) / 2.0;
    let total = load_bench::rod_statics::linearised_section_at(
        polyline,
        &heads,
        &increments,
        cut,
        load_bench::rod_statics::SectionIncrement {
            point_mm: cut_motion,
            tangent_unit: locked.tangent_unit,
        },
    )
    .unwrap();
    let daily = load_bench::rod_statics::SectionLoad {
        force_n: total.force_n - locked.force_n,
        moment_nmm: total.moment_nmm - locked.moment_nmm,
        ..locked
    };
    let locked_stress = load_bench::rod_statics::surface_stress(&locked, 4.0)
        .unwrap()
        .von_mises_mpa;
    let daily_stress = load_bench::rod_statics::surface_stress(&daily, 4.0)
        .unwrap()
        .von_mises_mpa;
    let material = shapes::implant_layout::RodMaterial::CobaltChrome;
    let ultimate = evidence
        .bench_strengths(material)
        .unwrap()
        .ultimate_lower
        .value;
    let steps = read("steps-if-absent", "lower");
    let year = load_bench::fatigue::year_of_walking(
        &evidence,
        material,
        load_bench::fatigue::WalkingInput {
            steps_per_day: steps,
            walking_multiple: input.walking_multiple,
            contour_knock_down: evidence
                .value(TableKind::Fatigue, "contour-knock-down", "lower")
                .unwrap()
                .value,
            ultimate_mpa: ultimate,
        },
        locked_stress,
        daily_stress,
    )
    .unwrap();
    assert_eq!(year.cycles_per_year, steps * read("year", "days"));
    assert!(matches!(
        year.comparison,
        load_bench::fatigue::FatigueComparison::Ratio(_)
    ));
}

#[test]
fn a_front_film_without_scale_runs_through_the_daily_cases() {
    run_fitted_case(true);
}

#[test]
fn two_films_run_through_the_daily_cases() {
    run_fitted_case(false);
}
