use super::*;
use crate::chain::{Ground, Wrench};
use crate::joint_springs::Stiffness;
use shapes::levels::Level;
use shapes::made_up::VertebraPose;

fn two_body_chain(scale: f64) -> Chain {
    let reference: Vec<_> = [(Level::L4, 60.0), (Level::L5, 20.0)]
        .into_iter()
        .map(|(level, z)| VertebraPose {
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
        })
        .collect();
    let stiffness = Stiffness::new([100.0; 6]).unwrap();
    Chain::new(
        &reference,
        &[stiffness; 2],
        Ground {
            top_mm: Vector3::zeros(),
            rotation: Matrix3::identity(),
        },
    )
    .unwrap()
    .scaled(scale)
    .unwrap()
}

fn held_at(poses: Vec<Pose>) -> HeldState {
    HeldState {
        poses,
        rod_poses: Vec::new(),
        stations_mm: Vec::new(),
        end_continuation_mm: 20.0,
        head_forces_on_rod: Vec::new(),
        holding_torques_nmm: Vec::new(),
        pinned_anchors: Vec::new(),
        contour_end_anchors: Vec::new(),
        pin_tangential_force_n: Vec::new(),
        ground_reaction: Wrench::zeros(),
        energy_nmm: 0.0,
        settled: true,
        reason: StopReason::Converged,
        iterations: 0,
        constraint_max_mm: 0.0,
        stationarity_max: 0.0,
    }
}

#[test]
fn each_joint_wrench_uses_its_group_origin_before_terms_cancel() {
    // After normalisation, every translational spring is 1 N/mm.
    // The upper/lower centres are (3,0,60)/(2,0,20), their disc is at
    // z=40, and the lower/ground attachment is at z=5. Thus joint 0
    // gives (Fx,My)=(1,-20) on the upper body and (-1,-20) below.
    // Joint 1 gives (2,-30) on the lower body. Translation coordinates
    // multiply forces by 20 mm. These are individual terms, not the
    // lower body's cancelled resultant (Fx,My)=(1,-50).
    for scale in [1.0, 7.0] {
        let chain = two_body_chain(scale);
        let mut poses = chain.standing();
        poses[0].centre_mm.x = 3.0;
        poses[1].centre_mm.x = 2.0;
        let held = held_at(poses);
        let construct = Construct {
            rods: Vec::new(),
            anchors: Vec::new(),
        };
        let mut model = ReleasedModel::new(&chain, &construct, &held).unwrap();
        let initial = |model: &ReleasedModel<'_>| State {
            groups: model
                .origins
                .iter()
                .map(|origin| Pose {
                    centre_mm: *origin,
                    rotation: Matrix3::identity(),
                })
                .collect(),
        };
        assert_eq!(model.constraints(&initial(&model)).0.len(), 0);
        assert_eq!(model.balanced_term_scale(&initial(&model)), Some(40.0));

        // Re-express the upper group's coordinates 200 mm below its
        // centre, preserving both physical poses: My=-20+200*1=180.
        let upper = model.body_group[0];
        model.origins[upper].z -= 200.0;
        assert_eq!(model.balanced_term_scale(&initial(&model)), Some(180.0));
        model.origins[upper].z += 200.0;

        // At a lower-group origin 100 mm below its centre, the two
        // individual moments are -20-100=-120 and -30+200=170.
        // Their sum is only 50; summing before taking a maximum fails.
        let lower = model.body_group[1];
        model.origins[lower].z -= 100.0;
        let state = initial(&model);
        assert_eq!(model.bodies(&state), held.poses);
        assert_eq!(model.balanced_term_scale(&state), Some(170.0));
        assert_eq!(model.evaluate(&state).unwrap().gradient.amax(), 50.0);
    }
}

#[test]
fn a_joint_inside_one_rigid_group_does_not_supply_a_balance_scale() {
    let chain = two_body_chain(1.0);
    let mut poses = chain.standing();
    poses[0].centre_mm.x = 300.0;
    poses[1].centre_mm.x = 2.0;
    let mut held = held_at(poses);
    held.rod_poses.push(Pose {
        centre_mm: Vector3::zeros(),
        rotation: Matrix3::identity(),
    });
    let construct = Construct {
        rods: vec![super::super::Rod {
            points_mm: vec![Vector3::new(0.0, 0.0, 60.0), Vector3::new(0.0, 0.0, 20.0)],
            properties: crate::rod_beams::BeamProperties::from_young_modulus(200_000.0, 5.5),
        }],
        anchors: (0..2)
            .map(|body| super::super::Anchor {
                body,
                rod: 0,
                point_mm: Vector3::zeros(),
                station_mm: 40.0 * body as f64,
                kind: HeadKind::Screw,
            })
            .collect(),
    };
    let model = ReleasedModel::new(&chain, &construct, &held).unwrap();
    assert_eq!(model.body_group[0], model.body_group[1]);
    let state = State {
        groups: model
            .origins
            .iter()
            .map(|origin| Pose {
                centre_mm: *origin,
                rotation: Matrix3::identity(),
            })
            .collect(),
    };
    // Only the lower/ground spring changes under this group's motion:
    // (Fx,My)=(2,-30), with no arm from the lowest member to the origin.
    // The severely strained internal disc is constant and contributes zero.
    assert_eq!(model.balanced_term_scale(&state), Some(40.0));
}
