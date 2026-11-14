//! A horizontal superior plane gives a closed-form direction. Angles
//! enter in degrees and the patient's left coordinate is positive y.

use forward_model::vertebra_params::VertebraParams;
use implant_layout::evidence::{AgeBand, Coordinate, PublishedValue, Sex, Table, tables};
use implant_layout::screws::geometry::direction;
use nalgebra::Vector3;
use shapes::levels::Level;
use shapes::spine_readings::BodySide;

fn printed_angle(level: Level, sex: Sex) -> f64 {
    let cell = tables()
        .lookup(
            Table::MedialAngles,
            &Coordinate {
                key: "transverse-angle".into(),
                level: Some(level),
                sex: Some(sex),
                age_band: Some(AgeBand::TwelveToFourteen),
                group: Some("zheng-children".into()),
                ..Coordinate::default()
            },
        )
        .unwrap();
    match cell.value {
        PublishedValue::MeanSd { mean, .. } => mean,
        PublishedValue::PlusMinus { value, .. } => value,
        _ => panic!("the source prints a centre and spread"),
    }
}

#[test]
fn positive_angles_converge_and_the_negative_zheng_angles_diverge_on_both_sides() {
    for level in [Level::T1, Level::T11, Level::T12] {
        // Combine the two sex columns by their arithmetic mean.
        let angle = (printed_angle(level, Sex::Male) + printed_angle(level, Sex::Female)) / 2.0;
        assert_eq!(angle > 0.0, level == Level::T1);
        let pose = VertebraParams {
            centre: Vector3::zeros(),
            rotation: Vector3::zeros(),
            height: 24.0,
            wedge_front: 0.0,
            wedge_side: 0.0,
            top_half_width: 16.0,
            top_half_depth: 12.0,
            bottom_half_width: 16.0,
            bottom_half_depth: 12.0,
        }
        .pose(level);
        for side in [BodySide::Left, BodySide::Right] {
            let actual = direction(&pose, side, angle).unwrap();
            let radians = angle * core::f64::consts::PI / 180.0;
            let sign = if side == BodySide::Left { -1.0 } else { 1.0 };
            let expected = [libm::cos(radians), sign * libm::sin(radians), 0.0];
            for (a, e) in actual.direction_unit.into_iter().zip(expected) {
                assert!((a - e).abs() < 1e-13, "{level:?} {side:?}");
            }
            assert!((actual.convergence_deg - angle.abs()).abs() < 1e-12);
        }
    }
}
