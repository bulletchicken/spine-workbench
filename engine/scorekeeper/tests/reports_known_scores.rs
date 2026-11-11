use scorekeeper::datasets::aasce_2019::LabelFlag;
use scorekeeper::metrics::{PredictedVertebra, Prediction, Quad, score_film};
use scorekeeper::report::{Distribution, FilmReport, Provenance, Report, json, markdown};
use shapes::{film::FilmKind, levels::Level};
use std::collections::BTreeMap;

fn row(row: usize, shift: [f64; 2]) -> FilmReport {
    let truth = Quad {
        tl: [0.0, 0.0],
        tr: [100.0, 0.0],
        bl: [0.0, 100.0],
        br: [100.0, 100.0],
    };
    let moved = |p: [f64; 2]| [p[0] + shift[0], p[1] + shift[1]];
    let quad = Quad {
        tl: moved(truth.tl),
        tr: moved(truth.tr),
        bl: moved(truth.bl),
        br: moved(truth.br),
    };
    let pred = Prediction {
        vertebrae: vec![PredictedVertebra {
            level: Level::T1,
            quad,
            confidence: 1.0,
        }],
    };
    let mut score =
        score_film(&[(Level::T1, truth, true)], None, &pred, 1000.0).expect("valid geometry");
    // Summary aggregation is checked with independent arithmetic, not Cobb.
    score.cobb_true = Some([12.0, 18.0, 30.0]);
    score.cobb_pred = Some(if row == 1 {
        [10.0, 20.0, 30.0]
    } else {
        [0.0; 3]
    });
    score.smape_term = Some(if row == 1 { 100.0 / 30.0 } else { 100.0 });
    FilmReport {
        row,
        film: FilmKind::Front,
        width: 1000,
        height: 1000,
        label_flags: if row == 1 {
            vec![]
        } else {
            vec![LabelFlag::OutsideImage]
        },
        excluded_levels: vec![],
        score,
        corner_mm: None,
    }
}

#[test]
fn reports_weight_corner_samples_and_keep_both_cobb_populations() {
    let provenance = Provenance {
        engine_version: "0.1.0".into(),
        git_revision: None,
        run_started_unix_seconds: None,
        dataset: "hand-made".into(),
        split: None,
        mode: "known answers".into(),
        listing_sha256: json::sha256(b""),
        label_sha256: BTreeMap::new(),
        parameters_sha256: json::sha256(b"abc"),
    };
    let report = Report::new(
        provenance,
        vec![row(1, [3.0, 4.0]), row(2, [6.0, 8.0])],
        vec!["Test fixtures only.".into()],
    );
    let s = &report.summary;
    assert_eq!(s.found, 2);
    assert_eq!(s.missed, 0);
    assert_eq!(s.flagged_films, 1);
    assert_eq!(s.corner_px.count, 8);
    assert_eq!(s.corner_px.mean, Some(7.5));
    assert_eq!(s.corner_px.median, Some(7.5));
    assert_eq!(s.corner_px.p90, Some(10.0));
    assert_eq!(s.under_010_height, Some(0.5));
    assert_eq!(s.under_025_height, Some(1.0));
    assert_eq!(s.per_level.len(), 1);
    assert_eq!(s.per_level[0].corner_px.mean, Some(7.5));
    // 1e-12 covers only the rounding in these short sums and divisions.
    assert!((s.cobb_all.smape_percent.unwrap() - 155.0 / 3.0).abs() < 1e-12);
    assert!((s.cobb_unflagged.smape_percent.unwrap() - 10.0 / 3.0).abs() < 1e-12);
    assert_eq!(s.cobb_all.mae_degrees, [Some(7.0), Some(10.0), Some(15.0)]);
    assert_eq!(s.cobb_all.within_5_degrees, Some(0.5));
    assert_eq!(s.cobb_all.maximum_absolute_error, Some(30.0));
    let encoded = json::render(&report).expect("serialises");
    assert!(encoded.ends_with('\n'));
    let value: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    assert_eq!(value["summary"]["found"], 2);
    assert!(value["summary"]["cornerMm"]["mean"].is_null());
    let text = markdown::render(&report);
    assert!(text.contains("Unflagged films: Cobb n=1"));
    assert!(text.contains("Test fixtures only."));
}

#[test]
fn empty_statistics_are_absent_and_hashes_use_standard_sha256() {
    let empty = Distribution::from_values(&[]);
    assert_eq!(empty.count, 0);
    assert_eq!(empty.mean, None);
    assert_eq!(empty.median, None);
    assert_eq!(empty.p90, None);
    assert_eq!(
        json::sha256(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
