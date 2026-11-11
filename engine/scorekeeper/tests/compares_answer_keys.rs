use scorekeeper::report::answer_key::compare_with_answer_key;
use shapes::confidence::{Confidence, ConfidenceBand};
use shapes::errors::ErrorCode;
use shapes::film::FilmKind;
use shapes::film_geometry::{FilmGeometry, ImageRight};
use shapes::landmarks::Landmarks;
use shapes::levels::Level;
use shapes::made_up::{
    AnswerKey, FilmAnswer, MadeUpRecipe, RecipeName, Tier, TrueLandmarks, TrueMark, TrueMarks,
    TrueVertebra, TruthAngles,
};
use shapes::marks::{EndplateLines, FrontMarks, Mark, MarkSource, PedicleMark, SideMarks};
use shapes::naming::{ChosenBy, LevelNaming, Template};
use shapes::pixels::Px;
use shapes::vertebra::{FilmMarks, VertebraMarks, VertebraOnFilm};

fn true_points(y: f64) -> [TrueMark; 4] {
    [
        [100.0, y],
        [180.0, y],
        [100.0, y + 100.0],
        [180.0, y + 100.0],
    ]
    .map(|at| TrueMark {
        at: Px(at),
        visible: true,
    })
}

fn truth(kind: FilmKind, points: [TrueMark; 4]) -> TrueMarks {
    let [a, b, c, d] = points;
    match kind {
        FilmKind::Front => TrueMarks::Front {
            sup_l: a,
            sup_r: b,
            inf_l: c,
            inf_r: d,
            ped_l: a,
            ped_r: b,
        },
        FilmKind::Side => TrueMarks::Side {
            sup_a: a,
            sup_p: b,
            inf_a: c,
            inf_p: d,
        },
        FilmKind::BendLeft | FilmKind::BendRight => TrueMarks::Bending {
            sup: [a, b],
            inf: [c, d],
        },
    }
}

fn placed(kind: FilmKind, points: [TrueMark; 4], shift: [f64; 2]) -> VertebraMarks {
    let [a, b, c, d] =
        points.map(|point| Mark::auto(Px::new(point.at.x() + shift[0], point.at.y() + shift[1])));
    let pedicle = PedicleMark::NotVisible {
        source: MarkSource::Auto,
    };
    match kind {
        FilmKind::Front => VertebraMarks::Front {
            points: FrontMarks {
                sup_l: a,
                sup_r: b,
                inf_l: c,
                inf_r: d,
                ped_l: pedicle,
                ped_r: pedicle,
            },
        },
        FilmKind::Side => VertebraMarks::Side {
            points: SideMarks {
                sup_a: a,
                sup_p: b,
                inf_a: c,
                inf_p: d,
            },
        },
        FilmKind::BendLeft | FilmKind::BendRight => VertebraMarks::Bending {
            lines: EndplateLines {
                sup: [a, b],
                inf: [c, d],
            },
        },
    }
}

fn fixture(kind: FilmKind) -> (AnswerKey, FilmMarks) {
    let points = true_points(100.0);
    let empty_mark = TrueMark {
        at: Px::new(0.0, 0.0),
        visible: false,
    };
    let answer = AnswerKey {
        recipe: MadeUpRecipe {
            name: RecipeName::Straight,
            seed: 1,
            tier: Tier::T0,
        },
        template: Template::Standard,
        poses: vec![],
        films: vec![FilmAnswer {
            kind,
            geometry: FilmGeometry {
                mm_per_px: Some(0.5),
                ..FilmGeometry::UNKNOWN
            },
            vertebrae: vec![TrueVertebra {
                level: Level::L1,
                marks: truth(kind, points),
            }],
            landmarks: TrueLandmarks {
                c7: empty_mark,
                sacrum_top: [empty_mark; 3],
                hips: [empty_mark; 2],
            },
            bent_poses: None,
        }],
        angles: TruthAngles {
            frontal_cobb: vec![],
            apical_twist: vec![],
            kyphosis_degrees: 0.0,
            lordosis_degrees: 0.0,
        },
    };
    let marks = FilmMarks {
        film: kind,
        vertebrae: vec![VertebraOnFilm {
            id: 7,
            level: Level::L1,
            marks: placed(kind, points, [3.0, 4.0]),
            confidence: Confidence {
                score: 1.0,
                band: ConfidenceBand::High,
                reasons: vec![],
                calibrated: false,
                endplate_spread_px: None,
                margin: None,
            },
            outline: None,
        }],
        landmarks: Landmarks::NONE,
        naming: LevelNaming {
            template: Template::Standard,
            chosen_by: ChosenBy::Engine,
            alternatives: vec![],
            from_top: None,
            from_bottom: None,
            flags: vec![],
            l5_confirmed_on: None,
        },
        named_for: if kind == FilmKind::Side {
            ImageRight::Anterior
        } else {
            ImageRight::PatientLeft
        },
    };
    (answer, marks)
}

#[test]
fn every_view_measures_a_three_four_five_shift() {
    for kind in FilmKind::ALL {
        let (key, marks) = fixture(kind);
        let comparison = compare_with_answer_key(&[marks], &key).unwrap();
        let film = &comparison.films[0];
        assert_eq!((film.expected, film.found, film.levels_right), (1, 1, 1));
        assert_eq!(film.mean_corner_error_px, Some(5.0));
        assert_eq!(film.mean_corner_error_mm, Some(2.5));
        assert_eq!(film.vertebrae[0].max_corner_error_px, Some(5.0));
        assert_eq!(film.vertebrae[0].vertebra_id, Some(7));
    }
}

#[test]
fn missing_films_and_distant_predictions_remain_unmatched() {
    let (key, mut marks) = fixture(FilmKind::Front);
    marks.vertebrae[0].marks = placed(FilmKind::Front, true_points(100.0), [500.0, 0.0]);
    for input in [vec![], vec![marks]] {
        let comparison = compare_with_answer_key(&input, &key).unwrap();
        let film = &comparison.films[0];
        assert_eq!((film.expected, film.found, film.levels_right), (1, 0, 0));
        assert_eq!(film.mean_corner_error_px, None);
        assert_eq!(film.mean_corner_error_mm, None);
        assert_eq!(film.vertebrae[0].vertebra_id, None);
    }
}

#[test]
fn one_prediction_cannot_count_as_two_bones() {
    let (mut key, marks) = fixture(FilmKind::Front);
    let mut second = key.films[0].vertebrae[0];
    second.level = Level::L2;
    key.films[0].vertebrae.push(second);
    let comparison = compare_with_answer_key(&[marks], &key).unwrap();
    let film = &comparison.films[0];
    assert_eq!((film.expected, film.found), (2, 1));
    assert_eq!(
        film.vertebrae
            .iter()
            .filter(|v| v.vertebra_id.is_some())
            .count(),
        1
    );
}

#[test]
fn naming_is_judged_after_position_matching() {
    let (key, mut marks) = fixture(FilmKind::Side);
    marks.vertebrae[0].level = Level::L2;
    let comparison = compare_with_answer_key(&[marks], &key).unwrap();
    assert_eq!(comparison.films[0].found, 1);
    assert_eq!(comparison.films[0].levels_right, 0);
    assert_eq!(comparison.films[0].mean_corner_error_px, Some(5.0));
}

#[test]
fn invisible_corners_are_excluded_and_the_film_mean_weights_corners() {
    let kind = FilmKind::BendLeft;
    let (mut key, mut marks) = fixture(kind);
    let mut points = true_points(100.0);
    for point in &mut points[1..] {
        point.visible = false;
    }
    // The only visible corner on L1 contributes 5; L2 contributes four
    // distances of 10, so the film mean is (5 + 4 * 10) / 5 = 9 exactly.
    // Swap end names in the truth to check that visibility stays attached.
    points.swap(0, 1);
    points.swap(2, 3);
    key.films[0].vertebrae[0].marks = truth(kind, points);
    key.films[0].vertebrae.push(TrueVertebra {
        level: Level::L2,
        marks: truth(kind, true_points(250.0)),
    });
    let mut second = marks.vertebrae[0].clone();
    second.id = 8;
    second.level = Level::L2;
    second.marks = placed(kind, true_points(250.0), [6.0, 8.0]);
    marks.vertebrae.push(second);
    let comparison = compare_with_answer_key(&[marks], &key).unwrap();
    assert_eq!(comparison.films[0].found, 2);
    assert_eq!(comparison.films[0].mean_corner_error_px, Some(9.0));
    assert_eq!(
        comparison.films[0].vertebrae[0].mean_corner_error_px,
        Some(5.0)
    );
}

#[test]
fn invisible_bones_still_count_and_missing_scale_has_no_mm_error() {
    let (mut key, marks) = fixture(FilmKind::Front);
    let points = true_points(100.0).map(|point| TrueMark {
        visible: false,
        ..point
    });
    key.films[0].vertebrae[0].marks = truth(FilmKind::Front, points);
    key.films[0].geometry.mm_per_px = None;
    let comparison = compare_with_answer_key(&[marks], &key).unwrap();
    let film = &comparison.films[0];
    assert_eq!((film.found, film.levels_right), (1, 1));
    assert_eq!(film.mean_corner_error_px, None);
    assert_eq!(film.mean_corner_error_mm, None);
}

#[test]
fn physical_error_uses_horizontal_and_vertical_spacing() {
    let (mut key, marks) = fixture(FilmKind::Front);
    key.films[0].geometry.mm_per_px = Some(2.0);
    key.films[0].geometry.mm_per_px_vertical = Some(3.0);
    let comparison = compare_with_answer_key(&[marks], &key).unwrap();
    // sqrt((3 * 2)^2 + (4 * 3)^2) = sqrt(180); 1e-12 allows rounding.
    let mm = comparison.films[0].mean_corner_error_mm.unwrap();
    assert!((mm - 13.416_407_864_998_739).abs() < 1e-12);
}

#[test]
fn malformed_values_fail_with_the_frozen_error_code() {
    let (key, marks) = fixture(FilmKind::Front);
    let duplicate = compare_with_answer_key(&[marks.clone(), marks.clone()], &key).unwrap_err();
    assert_eq!(duplicate.code, ErrorCode::CaseInvalidValue);
    let mut invalid_marks = marks.clone();
    invalid_marks.vertebrae[0].marks = placed(FilmKind::Front, true_points(100.0), [f64::NAN, 0.0]);
    assert_eq!(
        compare_with_answer_key(&[invalid_marks], &key)
            .unwrap_err()
            .code,
        ErrorCode::CaseInvalidValue
    );
    let mut invalid_key = key.clone();
    invalid_key.films[0].geometry.mm_per_px = Some(-1.0);
    assert_eq!(
        compare_with_answer_key(std::slice::from_ref(&marks), &invalid_key)
            .unwrap_err()
            .code,
        ErrorCode::CaseInvalidValue
    );
    let mut wrong_view = marks;
    wrong_view.vertebrae[0].marks = placed(FilmKind::Side, true_points(100.0), [0.0, 0.0]);
    assert_eq!(
        compare_with_answer_key(&[wrong_view], &key)
            .unwrap_err()
            .code,
        ErrorCode::CaseInvalidValue
    );
}
