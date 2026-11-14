//! Curve endpoints and distances are assigned by hand, without measuring
//! another synthetic curve to generate the expected answer.

use implant_layout::curves::{Classification, Curve, SpineReading, VertebraReading};
use implant_layout::pedicles::{CurvePosition, position_in_curve, side_factor, width_prior};
use implant_layout::screws::anatomy::{chord, medial_angle};
use shapes::implant_layout::{CannotTellCode, CurveSide};
use shapes::levels::Level;
use shapes::made_up::CurveRegion;
use shapes::spine_readings::BodySide;

fn curve(upper: usize, lower: usize, apex: usize, convex: BodySide) -> Curve {
    Curve {
        upper_index: upper,
        lower_index: lower,
        apex_index: apex,
        region: Some(if upper < 12 {
            CurveRegion::MainThoracic
        } else {
            CurveRegion::ThoracolumbarLumbar
        }),
        convex,
        cobb_deg: 40.0,
        signed_cobb_deg: if convex == BodySide::Right {
            40.0
        } else {
            -40.0
        },
        apical_deviation_mm: 20.0,
        bending_cobb_deg: Err(CannotTellCode::NoBendingFilms),
        straightening_bend: Err(CannotTellCode::NoBendingFilms),
        bending_disc_below_deg: Err(CannotTellCode::NoBendingFilms),
        structural: Ok(true),
    }
}

fn reading(convex: BodySide, double: bool) -> SpineReading {
    let vertebrae = Level::MEASURED
        .iter()
        .enumerate()
        .map(|(i, &level)| {
            // T2/T10 have zero measured twist; the intervening twists are
            // positive. Thus both thoracic neutral distances from T6 are 4.
            let twist = if (1..=9).contains(&i) {
                4.0 - (i as f64 - 5.0).abs()
            } else {
                0.0
            };
            VertebraReading {
                level,
                centre_mm: [0.0, 0.0, 500.0 - 25.0 * i as f64],
                upper_tilt_deg: 0.0,
                lower_tilt_deg: 0.0,
                twist_deg: Ok(twist),
                disc_below_deg: Ok(0.0),
                centre_line_offset_mm: Ok(0.0),
            }
        })
        .collect();
    let mut curves = vec![curve(1, 9, 5, convex)]; // T2..T10, apex T6.
    if double {
        let opposite = if convex == BodySide::Right {
            BodySide::Left
        } else {
            BodySide::Right
        };
        curves.push(curve(12, 16, 14, opposite)); // L1..L5, apex L3.
    }
    SpineReading {
        vertebrae,
        curves,
        major: Some(0),
        csvl_y_mm: Ok(0.0),
        kyphosis_t2_t5_deg: Ok(20.0),
        kyphosis_t5_t12_deg: Ok(20.0),
        kyphosis_t10_l2_deg: Ok(0.0),
        l4_tilt_deg: Ok(0.0),
        classification: Classification {
            curve_type: Err(CannotTellCode::NoBendingFilms),
            lumbar_modifier: Err(CannotTellCode::NoPedicleTable),
            sagittal_modifier: Err(CannotTellCode::NoSideFilm),
        },
        minor_curve_minimum_deg: None,
        sources: Vec::new(),
    }
}

#[test]
fn right_left_and_double_curves_assign_columns_and_distances_by_hand() {
    for convex in [BodySide::Right, BodySide::Left] {
        for double in [false, true] {
            let spine = reading(convex, double);
            for (i, &level) in Level::MEASURED.iter().enumerate() {
                for side in [BodySide::Left, BodySide::Right] {
                    let actual = position_in_curve(&spine, level, side);
                    let thoracic = (1..=9).contains(&i);
                    let lumbar = double && (12..=16).contains(&i);
                    let expected_side = if thoracic {
                        if side == convex {
                            CurveSide::Convex
                        } else {
                            CurveSide::Concave
                        }
                    } else if lumbar {
                        if side == convex {
                            CurveSide::Concave
                        } else {
                            CurveSide::Convex
                        }
                    } else {
                        CurveSide::Outside
                    };
                    assert_eq!(
                        actual.curve_side, expected_side,
                        "{convex:?} double={double} {level:?} {side:?}"
                    );
                    if thoracic {
                        assert_eq!(actual.apex_distance, i.abs_diff(5) as f64);
                        assert_eq!(actual.neutral_distance, Some(4.0));
                    } else if lumbar {
                        assert_eq!(actual.apex_distance, i.abs_diff(14) as f64);
                    } else {
                        assert_eq!(actual, CurvePosition::OUTSIDE);
                    }
                    if level <= Level::T12 {
                        // Gao control widths do not have a curve-side
                        // column. Outside a curve, Upendra's two side
                        // columns supply the project's combined prior.
                        assert!(width_prior(level, Some(14.0), expected_side).is_some());
                        assert!(chord(level, expected_side, None).is_some());
                        assert!(medial_angle(level, expected_side).is_some());
                    } else {
                        assert!(width_prior(level, None, expected_side).is_some());
                        assert!(chord(level, expected_side, None).is_some());
                    }
                }
            }
        }
    }
    // SYNTHESIS 2.8: constant apex +/-1 and neutral +/-1, linear
    // between. With neutral distance 4, the transition is distance1..3.
    assert_eq!(side_factor(0.73, 1.08, 0.0, 4.0), 0.73);
    assert_eq!(side_factor(0.73, 1.08, 1.0, 4.0), 0.73);
    assert!((side_factor(0.73, 1.08, 2.0, 4.0) - 0.905).abs() < 1e-14);
    assert_eq!(side_factor(0.73, 1.08, 3.0, 4.0), 1.08);
    assert_eq!(side_factor(0.73, 1.08, 4.0, 4.0), 1.08);
}

#[test]
fn unreadable_twist_uses_the_end_and_an_overlap_uses_the_nearer_apex() {
    let mut spine = reading(BodySide::Right, false);
    for vertebra in &mut spine.vertebrae {
        vertebra.twist_deg = Err(CannotTellCode::TwistNotMeasured);
    }
    // T2..T10 around T6: both end distances are four levels.
    for level in [Level::T3, Level::T9] {
        let position = position_in_curve(&spine, level, BodySide::Left);
        assert_eq!(position.neutral_distance, Some(4.0));
        assert_eq!(position.curve_side, CurveSide::Concave);
    }
    // A smaller left-convex curve T8..L5, apex T12, overlaps T8..T10.
    // T8 is nearer T6; T10 is nearer T12, regardless of Cobb magnitude.
    let mut second = curve(7, 16, 11, BodySide::Left);
    second.cobb_deg = 10.0;
    spine.curves.push(second);
    let upper = position_in_curve(&spine, Level::T8, BodySide::Left);
    assert_eq!(upper.curve_side, CurveSide::Concave);
    assert_eq!(upper.apex_distance, 2.0);
    let lower = position_in_curve(&spine, Level::T10, BodySide::Left);
    assert_eq!(lower.curve_side, CurveSide::Convex);
    assert_eq!(lower.apex_distance, 2.0);
    assert_eq!(lower.neutral_distance, Some(4.0));
}
