//! Known answers use rectangles and independently worked distances/angles.

use scorekeeper::metrics::{
    PredictedVertebra, Prediction, Quad, ScoreFlag, score_film, score_film_with_visibility,
};
use scorekeeper::metrics::{
    cobb_from_corners::{self, CobbBranch},
    corner_error, vertebrae_found,
};
use shapes::levels::Level;

fn rectangle(x: f64, y: f64, w: f64, h: f64) -> Quad {
    Quad {
        tl: [x, y],
        tr: [x + w, y],
        bl: [x, y + h],
        br: [x + w, y + h],
    }
}

fn chain() -> Vec<(Level, Quad, bool)> {
    Level::MEASURED
        .into_iter()
        .enumerate()
        .map(|(i, level)| {
            (
                level,
                rectangle(100.0, 50.0 + i as f64 * 50.0, 60.0, 40.0),
                true,
            )
        })
        .collect()
}

fn prediction(truth: &[(Level, Quad, bool)]) -> Prediction {
    Prediction {
        vertebrae: truth
            .iter()
            .map(|&(level, quad, _)| PredictedVertebra {
                level,
                quad,
                confidence: 1.0,
            })
            .collect(),
    }
}

fn translate(q: Quad, dx: f64, dy: f64) -> Quad {
    let [tl, tr, bl, br] = q.points().map(|p| [p[0] + dx, p[1] + dy]);
    Quad { tl, tr, bl, br }
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} differs from {expected}"
    );
}

fn tilted_chain(alpha: [f64; 17], offset: [f64; 17]) -> [[f64; 2]; 68] {
    std::array::from_fn(|i| {
        let j = i / 4;
        let [x, y] = [[-60.0, -30.0], [60.0, -30.0], [-60.0, 30.0], [60.0, 30.0]][i % 4];
        let a = alpha[j] * std::f64::consts::PI / 180.0;
        [
            500.0 + offset[j] + x * libm::cos(a) - y * libm::sin(a),
            100.0 + 100.0 * j as f64 + x * libm::sin(a) + y * libm::cos(a),
        ]
    })
}

#[test]
fn faithful_cobb_uses_bottom_midpoints_and_all_signed_cosines() {
    let alpha_a = [
        0., -5., -10., -15., -20., -15., -10., -5., 0., 5., 10., 15., 20., 15., 10., 5., 0.,
    ];
    let offset_a = [
        0., 10., 25., 45., 70., 90., 105., 115., 120., 115., 105., 90., 70., 45., 25., 10., 5.,
    ];
    let a = tilted_chain(alpha_a, offset_a);
    let (angles, branch) = cobb_from_corners::aasce_cobb(&a, 2000.0);
    assert_eq!(branch, CobbBranch::UpperS);
    // Analytical angle differences are exact; 1e-6 degrees allows inverse
    // cosine roundoff near parallel endplates, not geometric approximation.
    for (actual, expected) in angles.into_iter().zip([40., 20., 20.]) {
        near(actual, expected, 1e-6);
    }
    let b = tilted_chain(
        [
            0., 4., 8., 12., 8., 4., 0., -6., -12., -18., -24., -18., -12., -6., 0., 6., 10.,
        ],
        [
            0., -10., -25., -40., -50., -55., -50., -35., -15., 10., 35., 55., 65., 60., 45., 25.,
            10.,
        ],
    );
    for (height, expected, branch_expected) in [
        (1450.0, [36., 12., 0.], CobbBranch::LowerS),
        (3000.0, [36., 12., 34.], CobbBranch::UpperS),
    ] {
        let (angles, branch) = cobb_from_corners::aasce_cobb(&b, height);
        assert_eq!(branch, branch_expected);
        for (actual, expected) in angles.into_iter().zip(expected) {
            near(actual, expected, 1e-6);
        }
    }
    let mut c = a;
    c.swap(64, 65);
    c.swap(66, 67);
    let (angles, _) = cobb_from_corners::aasce_cobb(&c, 2000.0);
    for (actual, expected) in angles.into_iter().zip([180., 0., 0.]) {
        near(actual, expected, 1e-6);
    }
}

#[test]
fn straight_cobb_keeps_ieee_s_test_and_degenerate_rows_finite() {
    let straight = tilted_chain([0.; 17], [0.; 17]);
    let (angles, branch) = cobb_from_corners::aasce_cobb(&straight, 2000.0);
    assert_eq!(angles, [0.; 3]);
    assert_eq!(branch, CobbBranch::UpperS);
    let mut degenerate = straight;
    degenerate[1] = degenerate[0];
    degenerate[3] = degenerate[2];
    assert_eq!(
        cobb_from_corners::aasce_cobb(&degenerate, 2000.0).0,
        [0.; 3]
    );
}

#[test]
fn smape_divides_after_summing_all_three_angles() {
    near(
        cobb_from_corners::smape_term([10., 20., 30.], [12., 18., 30.]),
        100.0 / 30.0,
        1e-12,
    );
    assert_eq!(
        cobb_from_corners::smape_term([0.; 3], [10., 20., 30.]),
        100.0
    );
    assert_eq!(cobb_from_corners::smape_term([0.; 3], [0.; 3]), 0.0);
    near(
        cobb_from_corners::smape(&[[0.; 3], [10., 20., 30.]], &[[0.; 3], [12., 18., 30.]]).unwrap(),
        50.0 / 30.0,
        1e-12,
    );
    assert_eq!(cobb_from_corners::smape(&[], &[]), None);
    assert_eq!(cobb_from_corners::smape(&[[0.; 3]], &[]), None);
}

#[test]
fn perfect_and_three_four_shifts_have_known_scores() {
    let truth = chain();
    let mut pred = prediction(&truth);
    let perfect = score_film(&truth, Some([0.; 3]), &pred, 1000.0).unwrap();
    assert_eq!(
        (perfect.found, perfect.eligible, perfect.naming_ok),
        (17, 17, 17)
    );
    assert_eq!(perfect.corner_px, vec![0.; 68]);
    assert_eq!(perfect.smape_term, Some(0.));
    assert_eq!(perfect.chain_shift, Some(0.));
    for p in &mut pred.vertebrae {
        p.quad = translate(p.quad, 3., 4.);
    }
    let shifted = score_film(&truth, Some([0.; 3]), &pred, 1000.).unwrap();
    assert_eq!(shifted.found, 17);
    assert_eq!(shifted.corner_px, vec![5.; 68]);
    assert_eq!(shifted.corner_norm, vec![0.125; 68]);
    for p in &mut pred.vertebrae {
        std::mem::swap(&mut p.quad.tl, &mut p.quad.tr);
    }
    assert_eq!(
        score_film(&truth, None, &pred, 1000.).unwrap().corner_px,
        shifted.corner_px
    );
}

#[test]
fn naming_is_matched_by_geometry_and_detects_one_step_shift() {
    let truth = chain();
    let mut pred = prediction(&truth);
    let mut shifted = Level::MEASURED[1..].to_vec();
    shifted.push(Level::L6);
    for (p, level) in pred.vertebrae.iter_mut().zip(shifted) {
        p.level = level;
    }
    let score = score_film(&truth, None, &pred, 1000.).unwrap();
    assert_eq!(score.found, 17);
    assert_eq!(score.naming_ok, 0);
    assert_eq!(score.chain_shift, Some(1.));
}

#[test]
fn convex_hulls_and_assignment_have_hand_worked_answers() {
    let unit = rectangle(0., 0., 1., 1.);
    assert_eq!(vertebrae_found::quad_iou(&unit, &unit), 1.);
    assert_eq!(
        vertebrae_found::quad_iou(&unit, &translate(unit, 2., 0.)),
        0.
    );
    near(
        vertebrae_found::quad_iou(&unit, &translate(unit, 0.5, 0.)),
        1. / 3.,
        1e-12,
    );
    let crossed = Quad {
        tl: [0., 0.],
        tr: [2., 0.],
        bl: [2., 1.],
        br: [0., 1.],
    };
    assert_eq!(
        vertebrae_found::quad_iou(&crossed, &rectangle(0., 0., 2., 1.)),
        1.
    );
    assert_eq!(
        vertebrae_found::quad_iou(&rectangle(0., 0., 0., 0.), &unit),
        0.
    );
    assert_eq!(
        vertebrae_found::hungarian(&[vec![4., 1., 3.], vec![2., 0., 5.], vec![3., 2., 2.]])
            .unwrap(),
        vec![Some(1), Some(0), Some(2)]
    );
    assert_eq!(
        vertebrae_found::hungarian(&[vec![3.], vec![1.]]).unwrap(),
        vec![None, Some(0)]
    );
    assert_eq!(
        vertebrae_found::hungarian(&[vec![3., 1.]]).unwrap(),
        vec![Some(1)]
    );
    assert!(vertebrae_found::hungarian(&[vec![1.], vec![]]).is_err());
    assert!(vertebrae_found::hungarian(&[vec![f64::NAN]]).is_err());
}

#[test]
fn false_positives_require_unmatched_centres_in_the_spine_strip() {
    let truth = vec![(Level::T1, rectangle(0., 0., 40., 40.), true)];
    let pred = Prediction {
        vertebrae: vec![
            PredictedVertebra {
                level: Level::T1,
                quad: truth[0].1,
                confidence: 1.,
            },
            PredictedVertebra {
                level: Level::T2,
                quad: rectangle(39., 0., 10., 10.),
                confidence: 1.,
            },
            PredictedVertebra {
                level: Level::T3,
                quad: rectangle(500., 0., 10., 10.),
                confidence: 1.,
            },
        ],
    };
    let score = score_film(&truth, None, &pred, 1000.).unwrap();
    assert_eq!(
        (score.found, score.false_pos, score.unmatched_predictions),
        (1, 1, 2)
    );
    let excluded = score_film(&[(Level::T1, truth[0].1, false)], None, &pred, 1000.).unwrap();
    assert_eq!(
        (excluded.found, excluded.eligible, excluded.false_pos),
        (0, 0, 0)
    );
}

#[test]
fn invisible_corners_follow_canonical_swaps_without_losing_found_counts() {
    let q = Quad {
        tl: [40., 0.],
        tr: [0., 0.],
        bl: [40., 40.],
        br: [0., 40.],
    };
    let truth = [(Level::T1, q, true)];
    let mut pred = prediction(&truth);
    // Only the raw TL (canonical TR) moves, and it is invisible.
    pred.vertebrae[0].quad.tl[0] += 5.;
    let score =
        score_film_with_visibility(&truth, &[[false, true, true, true]], None, &pred, 1000.)
            .unwrap();
    assert_eq!(score.corner_px, vec![0.; 3]);
    assert_eq!((score.found, score.naming_ok), (1, 1));
    let hidden = score_film_with_visibility(&truth, &[[false; 4]], None, &pred, 1000.).unwrap();
    assert!(hidden.corner_px.is_empty());
    assert_eq!(hidden.found, 1);
}

#[test]
fn missing_cobb_quads_interpolate_and_translate_nearest_end_shapes() {
    let truth = chain();
    let mut pred = prediction(&truth);
    pred.vertebrae = vec![pred.vertebrae[3].clone(), pred.vertebrae[10].clone()];
    let (filled, interpolated) = cobb_from_corners::fill_missing(&pred).unwrap();
    assert!(interpolated);
    for (q, (_, expected, _)) in filled.into_iter().zip(&truth) {
        assert_eq!(q, *expected);
    }
    let score = score_film(&truth, Some([0.; 3]), &pred, 1000.).unwrap();
    assert_eq!(score.corner_px.len(), 8);
    assert!(score.flags.contains(&ScoreFlag::CobbInterpolated));
    pred.vertebrae.truncate(1);
    let insufficient = score_film(&truth, Some([0.; 3]), &pred, 1000.).unwrap();
    assert_eq!(insufficient.smape_term, Some(100.));
    assert_eq!(insufficient.cobb_pred, Some([0.; 3]));
}

#[test]
fn invalid_scores_fail_and_empty_samples_do_not_look_perfect() {
    let truth = chain();
    let empty = score_film(&truth, None, &Prediction::default(), 1000.).unwrap();
    assert_eq!(empty.found, 0);
    assert_eq!(empty.chain_shift, None);
    assert_eq!(corner_error::summarise(&empty.corner_px), None);
    assert_eq!(corner_error::fraction_under(&[], 0.1), None);
    let summary = corner_error::summarise(&[1., 2., 3., 4., 5., 6., 7., 8., 9., 10.]).unwrap();
    assert_eq!((summary.mean, summary.median, summary.p90), (5.5, 5.5, 9.));
    assert_eq!(
        corner_error::fraction_under(&[0.0, 0.1, 0.2], 0.1),
        Some(1. / 3.)
    );
    let mut pred = prediction(&truth);
    pred.vertebrae[0].quad.tl[0] = f64::NAN;
    assert!(score_film(&truth, None, &pred, 1000.).is_err());
    assert!(score_film(&truth, None, &Prediction::default(), 0.).is_err());
}

#[test]
fn excluded_labels_cannot_take_matches_from_eligible_truth() {
    let valid = rectangle(0., 0., 40., 40.);
    let corrupt = translate(valid, 1., 0.);
    let truth = [(Level::T1, valid, true), (Level::T2, corrupt, false)];
    // The prediction is closer to the corrupt label but still clearly
    // finds T1 (IoU 39/41). Excluded T2 must not take that match away.
    let pred = Prediction {
        vertebrae: vec![PredictedVertebra {
            level: Level::T1,
            quad: corrupt,
            confidence: 1.,
        }],
    };
    let score = score_film(&truth, None, &pred, 100.).unwrap();
    assert_eq!(
        (
            score.found,
            score.eligible,
            score.naming_ok,
            score.false_pos
        ),
        (1, 1, 1, 0)
    );
}

#[test]
fn non_s_cobb_uses_end_levels_for_secondary_angles() {
    let alpha: [f64; 17] = [
        0., 4., 8., 12., 16., 20., 16., 12., 8., 4., 0., -5., -10., -5., 0., 0., 0.,
    ];
    let points = std::array::from_fn(|i| {
        let j = i / 4;
        let upper = i % 4 < 2;
        let right = i % 2 == 1;
        let y = 100. + 100. * j as f64 + if upper { -30. } else { 30. };
        let x = y * 0.25;
        let dx = if right { 60. } else { -60. };
        [
            x + dx,
            y + dx * libm::tan(alpha[j] * std::f64::consts::PI / 180.),
        ]
    });
    // All 34 endplate midpoints lie on x=y/4, hence every S-test ll is
    // zero. Mid-lines span -10 to +20 degrees, with level ends at zero.
    let (angles, branch) = cobb_from_corners::aasce_cobb(&points, 2000.);
    assert_eq!(branch, CobbBranch::NotS);
    for (actual, expected) in angles.into_iter().zip([30., 20., 10.]) {
        near(actual, expected, 1e-6);
    }
}

#[test]
fn cobb_end_fill_translates_instead_of_extrapolating_shape() {
    let a = rectangle(0., 100., 40., 20.);
    let b = rectangle(0., 200., 80., 40.);
    let pred = Prediction {
        vertebrae: vec![
            PredictedVertebra {
                level: Level::T2,
                quad: a,
                confidence: 1.,
            },
            PredictedVertebra {
                level: Level::T4,
                quad: b,
                confidence: 1.,
            },
        ],
    };
    let (filled, _) = cobb_from_corners::fill_missing(&pred).unwrap();
    // Centres differ by (20,110) across two level steps. T1 keeps T2's
    // 40x20 shape translated by (-10,-55); T3 interpolates each corner.
    assert_eq!(filled[0], translate(a, -10., -55.));
    assert_eq!(filled[2], rectangle(0., 150., 60., 30.));
    assert_eq!(filled[4], translate(b, 10., 55.));
}
