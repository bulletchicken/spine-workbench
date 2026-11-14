//! Solver output reaches the same anchor reader used by every loaded stop.

use super::super::assembly::Placement;
use super::super::stiffness_calibration::realistic_cases;
use super::anchors;
use crate::chain::{Chain, Ground, Wrench};
use crate::daily_loads;
use crate::evidence::{Evidence, TableKind};
use crate::joint_springs::Stiffness;
use crate::rigid_rod::{self, Anchor, Construct, HeadKind, HeldOptions, RigidState, Rod};
use crate::rod_beams::{BeamProperties, polyline_stations};
use crate::springback::{self, SeatedState};
use crate::versions::{self, Draw};
use nalgebra::{Matrix3, Vector3};
use shapes::implant_layout::{AnchorKind, ChildFacts, ImplantLayout, LayoutChange};
use shapes::levels::Level;
use shapes::made_up::{RecipeName, VertebraPose};
use shapes::spine_readings::BodySide;

struct Fixture {
    evidence: Evidence,
    chain: Chain,
    layout: ImplantLayout,
    placement: Placement,
    draw: Draw,
    rigid: RigidState,
    seated: SeatedState,
}

fn prestressed_fixture() -> Fixture {
    let evidence = Evidence::read().unwrap();
    let reference: Vec<_> = [Level::L2, Level::L3, Level::L4, Level::L5]
        .into_iter()
        .enumerate()
        .map(|(i, level)| VertebraPose {
            level,
            centre: [0.0, 0.0, 140.0 - 40.0 * i as f64],
            rotation: [0.0; 3],
            height: 20.0,
            wedge_front: 0.0,
            wedge_side: 0.0,
            top_half_width: 12.0,
            top_half_depth: 8.0,
            bottom_half_width: 12.0,
            bottom_half_depth: 8.0,
        })
        .collect();
    let chain = Chain::new(
        &reference,
        &[Stiffness::new([80.0, 120.0, 200.0, 3000.0, 5000.0, 4000.0]).unwrap(); 4],
        Ground {
            top_mm: Vector3::zeros(),
            rotation: Matrix3::identity(),
        },
    )
    .unwrap();
    let mut deformation = vec![Wrench::zeros(); 4];
    // A symmetric axial correction has an exactly linear equilibrium:
    // springback adds no finite-rotation remainder to the prestressed state.
    deformation[1][2] = 0.5;
    let target = chain.poses_from_deformations(&deformation).unwrap();
    let mut construct = Construct {
        rods: Vec::new(),
        anchors: Vec::new(),
    };
    for (rod, side) in [9.0, -9.0].into_iter().enumerate() {
        let local = Vector3::new(0.0, side, 0.0);
        let points: Vec<_> = [1, 2]
            .into_iter()
            .map(|body| target[body].centre_mm + target[body].rotation * local)
            .collect();
        let stations = polyline_stations(&points).unwrap();
        construct.rods.push(Rod {
            points_mm: points,
            properties: BeamProperties::from_young_modulus(70000.0, 5.0),
        });
        for (head, body) in [1, 2].into_iter().enumerate() {
            construct.anchors.push(Anchor {
                body,
                point_mm: local,
                rod,
                station_mm: stations[head],
                kind: HeadKind::Screw,
            });
        }
    }
    let held = rigid_rod::solve_held(
        &chain,
        &construct,
        &HeldOptions {
            fixed_stations: true,
            ..Default::default()
        },
        None,
    )
    .unwrap();
    assert!(held.settled, "{:?}", held.reason);
    let rigid = rigid_rod::solve_rigid(&chain, &construct, &held, None).unwrap();
    assert!(rigid.settled, "{:?}", rigid.reason);
    let seated = springback::solve(&chain, &rigid.poses, &rigid.rods, &held.poses).unwrap();
    assert!(seated.settled);
    assert!(
        seated
            .head_wrenches_on_rod
            .iter()
            .flatten()
            .all(|w| w.fixed_rows::<3>(0).norm() > 0.01)
    );
    // Only the anchor metadata is used by this reader. Reuse a complete
    // sourced proposal, replacing its positions by the explicit short column.
    let template = realistic_cases::build(RecipeName::RightThoracic40, 1).unwrap();
    let mut anchors = Vec::new();
    for body in [1, 2] {
        for (side, y) in [(BodySide::Left, 9.0), (BodySide::Right, -9.0)] {
            let mut anchor = template.anchors[0].clone();
            anchor.level = reference[body].level;
            anchor.side = side;
            anchor.kind = AnchorKind::Screw;
            anchor.entry_mm = [0.0, y, reference[body].centre[2]];
            anchors.push(anchor);
        }
    }
    let layout = ImplantLayout {
        id: "short-column".into(),
        change: LayoutChange::First {},
        upper: Level::L3,
        lower: Level::L4,
        anchors,
        rods: Vec::new(),
        released_levels: Vec::new(),
    };
    let placement = Placement {
        construct,
        anchor_indices: vec![0, 2, 1, 3],
        rod_indices: vec![0, 1],
    };
    let draw = versions::draw(
        &realistic_cases::truth_fit(&reference),
        &ChildFacts {
            age_years: Some(15.0),
            weight_kg: Some(40.0),
            activity: None,
        },
        1,
        1,
        0,
        &evidence,
    )
    .unwrap();
    Fixture {
        evidence,
        chain,
        layout,
        placement,
        draw,
        rigid,
        seated,
    }
}

impl Fixture {
    fn anchor_forces(&self, response: Option<&daily_loads::DailyResponse>) -> Vec<f64> {
        anchors(
            &self.layout,
            &self.draw,
            &self.evidence,
            &self.placement,
            &self.rigid,
            &self.seated,
            response,
        )
        .unwrap()
        .0
    }

    fn solve(&self, loads: &[Wrench]) -> daily_loads::DailyResponse {
        daily_loads::solve_case(
            &self.chain,
            &self.seated.poses,
            &self.seated.corrected_rods,
            &self.seated.head_wrenches_on_rod,
            loads,
        )
        .unwrap()
    }
}

#[test]
fn a_solver_rigid_turn_keeps_the_reported_anchor_force() {
    let fixture = prestressed_fixture();
    // Loading only the lowest instrumented body leaves the unloaded bodies
    // above it following rigidly. This induces a turn without rod strain.
    let mut loads = vec![Wrench::zeros(); 4];
    loads[2][4] = 0.3;
    let response = fixture.solve(&loads);
    let base_shift = Vector3::from_column_slice(response.displacement.rows(12, 3).as_slice());
    let base_turn = Vector3::from_column_slice(response.displacement.rows(15, 3).as_slice());
    for body in [0, 1, 2] {
        let shift = Vector3::from_column_slice(response.displacement.rows(6 * body, 3).as_slice());
        let turn =
            Vector3::from_column_slice(response.displacement.rows(6 * body + 3, 3).as_slice());
        let arm = fixture.seated.poses[body].centre_mm - fixture.seated.poses[2].centre_mm;
        assert!((turn - base_turn).norm() < 1e-12);
        assert!((shift - base_shift - base_turn.cross(&arm)).norm() < 1e-10);
    }
    let mut largest_spatial_increment = 0.0_f64;
    for (r, rod) in fixture.seated.corrected_rods.iter().enumerate() {
        for (h, tie) in rod.ties.iter().enumerate() {
            let turn = Vector3::from_column_slice(
                response.displacement.rows(6 * tie.body + 3, 3).as_slice(),
            );
            let locked = fixture.seated.head_wrenches_on_rod[r][h]
                .fixed_rows::<3>(0)
                .into_owned();
            let spatial = response.head_wrenches_on_rod[r][h].fixed_rows::<3>(0);
            let expected = turn.cross(&locked);
            assert!(expected.norm() > 1e-8);
            assert!(
                (spatial - expected).norm() / locked.norm() < 1e-9,
                "rod {r} head {h}: relative {}, spatial {}, expected {}, body turn {}",
                (spatial - expected).norm() / locked.norm(),
                spatial.transpose(),
                expected.transpose(),
                turn.transpose()
            );
            assert!(
                response.head_wrenches_in_body_frame[r][h]
                    .fixed_rows::<3>(0)
                    .norm()
                    / locked.norm()
                    < 1e-9
            );
            largest_spatial_increment = largest_spatial_increment.max(spatial.norm());
        }
    }
    assert!(largest_spatial_increment > 1e-8);
    for (before, after) in fixture
        .anchor_forces(None)
        .iter()
        .zip(fixture.anchor_forces(Some(&response)))
    {
        assert!((after / before - 1.0).abs() < 1e-9);
    }
}

#[test]
fn bending_reports_the_locked_force_once_plus_the_scaled_solver_increment() {
    let fixture = prestressed_fixture();
    let loads = vec![Wrench::new(0.2, -0.1, -0.4, 0.05, 0.3, -0.02); 4];
    let standing = fixture.solve(&loads);
    let multiple = fixture
        .evidence
        .value(TableKind::DailyLoads, "ventral-flexion", "upper")
        .unwrap()
        .value;
    // Solve the larger load independently, so a broken response scaling
    // cannot provide both sides of the comparison.
    let bending = fixture.solve(&loads.iter().map(|w| w * multiple).collect::<Vec<_>>());
    let reported = fixture.anchor_forces(Some(&bending));
    let activity = fixture.anchor_forces(Some(&standing.scaled(multiple).unwrap()));
    let rods_in = fixture.anchor_forces(None);
    let mut changed = false;
    for (r, order) in fixture.rigid.anchor_order.iter().enumerate() {
        for (h, &mechanical) in order.iter().enumerate() {
            let index = fixture.placement.anchor_indices[mechanical];
            let body = fixture.seated.corrected_rods[r].ties[h].body;
            let locked = fixture.seated.poses[body].rotation.transpose()
                * fixture.seated.head_wrenches_on_rod[r][h].fixed_rows::<3>(0);
            let added = standing.head_wrenches_in_body_frame[r][h].fixed_rows::<3>(0);
            // |F0 + m dF|² = |F0|² + 2m F0·dF + m²|dF|².
            let expected_squared = locked.norm_squared()
                + 2.0 * multiple * locked.dot(&added)
                + multiple * multiple * added.norm_squared();
            assert!((reported[index] * reported[index] / expected_squared - 1.0).abs() < 1e-9);
            assert!((activity[index] / reported[index] - 1.0).abs() < 1e-9);
            changed |= (reported[index] - rods_in[index]).abs() > 1e-3;
        }
    }
    assert!(changed);
}
