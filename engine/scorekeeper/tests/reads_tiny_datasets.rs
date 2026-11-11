//! Hand-built tables only: no local dataset is needed to run these tests.

use scorekeeper::datasets::aasce_2019::{LabelFlag, label_flags, parse_labels};
use scorekeeper::datasets::{DatasetError, made_up};
use scorekeeper::metrics::Quad;
use shapes::confidence::{Confidence, ConfidenceBand};
use shapes::film::FilmKind;
use shapes::film_geometry::{FilmGeometry, ImageRight};
use shapes::landmarks::Landmarks;
use shapes::levels::Level;
use shapes::made_up::{FilmAnswer, TrueLandmarks, TrueMark, TrueMarks, TrueVertebra};
use shapes::marks::{FrontMarks, Mark, MarkSource, PedicleMark};
use shapes::naming::{ChosenBy, LevelNaming, Template};
use shapes::pixels::Px;
use shapes::vertebra::{FilmMarks, VertebraMarks, VertebraOnFilm};

fn table() -> Vec<u8> {
    // Every rectangle is x=20..60, height=10, y=10+20*j on an 80×400 film.
    let mut values = vec![0.0; 136];
    for j in 0..17 {
        values[4 * j..4 * j + 4].copy_from_slice(&[0.25, 0.75, 0.25, 0.75]);
        let top = (10.0 + 20.0 * j as f64) / 400.0;
        values[68 + 4 * j..72 + 4 * j].copy_from_slice(&[top, top, top + 0.025, top + 0.025]);
    }
    format!(
        "{}\n",
        values
            .iter()
            .map(f64::to_string)
            .collect::<Vec<_>>()
            .join(",")
    )
    .into_bytes()
}

fn rectangle(x: f64, y: f64) -> Quad {
    Quad {
        tl: [x, y],
        tr: [x + 40.0, y],
        bl: [x, y + 10.0],
        br: [x + 40.0, y + 10.0],
    }
}

#[test]
fn reads_fractional_axes_in_separate_halves_and_preserves_row_alignment() {
    let data = [table(), table()].concat();
    let labels = parse_labels(
        b"drawing-a.jpg\ndrawing-b.jpg\n",
        &data,
        b"12,3,4\n15,5,6\n",
    )
    .expect("two drawings");
    assert_eq!(labels.len(), 2);
    assert_eq!(labels.filename(1), Some("drawing-b.jpg"));
    let film = labels.film(1, 80, 400).expect("valid dimensions");
    assert_eq!(film.row, 2);
    assert_eq!(film.cobb, [15.0, 5.0, 6.0]);
    // Decimal fractions can round at multiplication; 1e-12 px exceeds that
    // roundoff but is far smaller than any pixel-placement error.
    for (j, q) in film.truth.iter().enumerate() {
        let expected = rectangle(20.0, 10.0 + 20.0 * j as f64);
        for (actual, want) in [q.tl, q.tr, q.bl, q.br].into_iter().flatten().zip(
            [expected.tl, expected.tr, expected.bl, expected.br]
                .into_iter()
                .flatten(),
        ) {
            assert!((actual - want).abs() < 1e-12);
        }
    }
    assert!(!film.flags.is_flagged());
    assert!(
        film.truth_for_scoring()
            .iter()
            .all(|(_, _, eligible)| *eligible)
    );
}

#[test]
fn rejects_partial_nonfinite_or_unsafe_tables_without_echoing_fields() {
    assert!(matches!(
        parse_labels(b"drawing.jpg\n", &table(), b"0,0,0\n0,0,0\n"),
        Err(DatasetError::RowCount)
    ));
    assert!(matches!(
        parse_labels(b"drawing.jpg\n", b"0,0\n", b"0,0,0\n"),
        Err(DatasetError::Columns { expected: 136, .. })
    ));
    assert!(matches!(
        parse_labels(b"drawing.jpg\n", &table(), b"NaN,0,0\n"),
        Err(DatasetError::Number { column: 1, .. })
    ));
    assert!(matches!(
        parse_labels(b"drawing.jpg\n", &table(), b"181,0,0\n"),
        Err(DatasetError::Number { .. })
    ));
    assert!(matches!(
        parse_labels(b"", b"", b""),
        Err(DatasetError::Empty)
    ));
    for name in [
        "../private.jpg",
        "sub/private.jpg",
        "sub\\private.jpg",
        "C:private.jpg",
    ] {
        let error = parse_labels(name.as_bytes(), &table(), b"0,0,0\n")
            .err()
            .expect("unsafe name");
        assert!(matches!(error, DatasetError::FileName { row: 1 }));
        assert!(!format!("{error:?} {error}").contains("private"));
    }
    assert!(matches!(
        parse_labels(
            b"same.jpg\nsame.jpg\n",
            &[table(), table()].concat(),
            b"0,0,0\n0,0,0\n"
        ),
        Err(DatasetError::DuplicateFileName { row: 2 })
    ));
    let labels = parse_labels(b"drawing.jpg\n", &table(), b"0,0,0\n").expect("one drawing");
    assert!(matches!(
        labels.film(0, 0, 400),
        Err(DatasetError::Dimensions)
    ));
    assert!(matches!(
        labels.film(1, 80, 400),
        Err(DatasetError::RowUnavailable)
    ));
}

#[test]
fn flags_the_unique_order_defect_and_keeps_each_other_defect_local() {
    let mut q = std::array::from_fn(|j| rectangle(20.0, 40.0 + 40.0 * j as f64));
    q[16] = rectangle(20.0, 0.0);
    let flags = label_flags(&q, 100, 800);
    assert_eq!(flags.per_vertebra[16], vec![LabelFlag::LevelOrder]);
    assert!(flags.per_vertebra[..16].iter().all(Vec::is_empty));
    q[16] = rectangle(20.0, 680.0);
    std::mem::swap(&mut q[4].tl, &mut q[4].tr);
    std::mem::swap(&mut q[4].bl, &mut q[4].br);
    std::mem::swap(&mut q[6].tl, &mut q[6].bl);
    std::mem::swap(&mut q[6].tr, &mut q[6].br);
    q[8].tl[0] = -1.0;
    q[10].tr = q[10].tl;
    q[10].br = q[10].bl;
    let flags = label_flags(&q, 100, 800);
    assert_eq!(flags.per_vertebra[4], vec![LabelFlag::LrSwapped]);
    assert_eq!(flags.per_vertebra[6], vec![LabelFlag::TopBelowBottom]);
    assert_eq!(flags.per_vertebra[8], vec![LabelFlag::OutsideImage]);
    assert_eq!(flags.per_vertebra[10], vec![LabelFlag::DegenerateQuad]);
    assert_eq!(
        flags.per_vertebra.iter().filter(|v| !v.is_empty()).count(),
        4
    );
}

#[test]
fn ambiguous_equal_centres_flag_both_neighbours() {
    let mut q = std::array::from_fn(|j| rectangle(20.0, 40.0 * j as f64));
    q[9] = q[8];
    let flags = label_flags(&q, 100, 800);
    assert_eq!(flags.per_vertebra[8], vec![LabelFlag::LevelOrder]);
    assert_eq!(flags.per_vertebra[9], vec![LabelFlag::LevelOrder]);
    assert_eq!(
        flags.per_vertebra.iter().filter(|v| !v.is_empty()).count(),
        2
    );
}

fn marks(quads: &[(Level, Quad)]) -> FilmMarks {
    FilmMarks {
        film: FilmKind::Front,
        landmarks: Landmarks::NONE,
        named_for: ImageRight::PatientRight,
        naming: LevelNaming {
            template: Template::Standard,
            chosen_by: ChosenBy::Engine,
            alternatives: vec![],
            from_top: None,
            from_bottom: None,
            flags: vec![],
            l5_confirmed_on: None,
        },
        vertebrae: quads
            .iter()
            .enumerate()
            .map(|(j, (level, q))| {
                let mark = |p: [f64; 2]| Mark::auto(Px::new(p[0], p[1]));
                VertebraOnFilm {
                    id: j as u32,
                    level: *level,
                    marks: VertebraMarks::Front {
                        points: FrontMarks {
                            sup_l: mark(q.tl),
                            sup_r: mark(q.tr),
                            inf_l: mark(q.bl),
                            inf_r: mark(q.br),
                            ped_l: PedicleMark::NotVisible {
                                source: MarkSource::Auto,
                            },
                            ped_r: PedicleMark::NotVisible {
                                source: MarkSource::Auto,
                            },
                        },
                    },
                    confidence: Confidence {
                        score: 1.0,
                        band: ConfidenceBand::High,
                        reasons: vec![],
                        calibrated: false,
                        endplate_spread_px: None,
                        margin: None,
                    },
                    outline: None,
                }
            })
            .collect(),
    }
}

#[test]
fn passes_only_the_film_to_a_supplied_finder_and_scores_its_marks() {
    let labels = parse_labels(b"drawing.jpg\n", &table(), b"0,0,0\n").expect("one drawing");
    let film = labels.film(0, 80, 400).expect("film");
    let expected: Vec<_> = Level::MEASURED
        .into_iter()
        .enumerate()
        .map(|(j, l)| (l, rectangle(20.0, 10.0 + 20.0 * j as f64)))
        .collect();
    let mut calls = 0;
    let score = film
        .score_with_finder(b"hand-written-film", |input| {
            calls += 1;
            assert_eq!(input.bytes, b"hand-written-film");
            assert_eq!(
                (input.width, input.height, input.kind),
                (80, 400, FilmKind::Front)
            );
            assert_eq!(input.geometry.mm_per_px, None);
            Ok(marks(&expected))
        })
        .expect("score");
    assert_eq!(calls, 1);
    assert_eq!((score.found, score.naming_ok, score.eligible), (17, 17, 17));
    assert!(score.corner_px.iter().all(|e| *e < 1e-12));
    assert_eq!(score.smape_term, Some(0.0));
}

#[test]
fn made_up_corners_respect_visibility_and_both_pixel_spacings() {
    let true_mark = |x, y, visible| TrueMark {
        at: Px::new(x, y),
        visible,
    };
    let origin = true_mark(0.0, 0.0, false);
    let mut geometry = FilmGeometry::UNKNOWN;
    geometry.mm_per_px = Some(2.0);
    geometry.mm_per_px_vertical = Some(1.0);
    let answer = FilmAnswer {
        kind: FilmKind::Front,
        geometry,
        bent_poses: None,
        vertebrae: vec![TrueVertebra {
            level: Level::T1,
            marks: TrueMarks::Front {
                sup_l: true_mark(60.0, 10.0, false),
                sup_r: true_mark(20.0, 10.0, true),
                inf_l: true_mark(60.0, 50.0, true),
                inf_r: true_mark(20.0, 50.0, true),
                ped_l: origin,
                ped_r: origin,
            },
        }],
        landmarks: TrueLandmarks {
            c7: origin,
            sacrum_top: [origin; 3],
            hips: [origin; 2],
        },
    };
    let placed = marks(&[(
        Level::T1,
        Quad {
            tl: [23.0, 14.0],
            tr: [63.0, 14.0],
            bl: [23.0, 54.0],
            br: [63.0, 54.0],
        },
    )]);
    let result = made_up::score_marks(&answer, 100, &placed).expect("score");
    assert_eq!((result.score.eligible, result.score.found), (1, 1));
    assert_eq!(result.score.corner_px, vec![5.0; 3]);
    assert_eq!(result.corner_mm.len(), 3);
    // A (3,4) pixel shift is (6,4) mm here: sqrt(52), not 5*2.
    assert!(
        result
            .corner_mm
            .iter()
            .all(|e| (*e - libm::sqrt(52.0)).abs() < 1e-12)
    );
    assert_eq!(result.score.cobb_true, None);
}
