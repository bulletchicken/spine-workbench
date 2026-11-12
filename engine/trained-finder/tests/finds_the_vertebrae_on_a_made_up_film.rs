//! The whole finder on the made-up front film the known answers were
//! recorded on.
//!
//! On the film's 8-bit picture, made by the answers' own rule, every stage
//! matches the reference: both readings' peaks and outlines, the frame,
//! the chain, and the final settled vertebrae. (The heat maps themselves
//! are checked from the same pictures in
//! `spine_net_matches_the_known_answers.rs`.)
//!
//! As the product reads it, from its PNG through the opening window, it
//! finds the spine in order with marks the strict reader takes; and
//! watching it one unit at a time or in large slices gives the same frames
//! and the same answer as the one-call entry. No reference answer exists
//! for that path: the window is the engine's own rule. Its answer is the
//! one recorded in `proving-ground/known-answers/trained-pass/front.json`,
//! to the last character, which the browser build is held to as well
//! (`proving-ground/parity/trained-pass.test.ts`), so the two builds give
//! the same bits.

mod geometry_fixtures;
mod made_up_film;

use film_reader::decoded_film::DecodedFilm;
use geometry_fixtures::{compare_chains, links, number, pipeline, quad_gap, reading};
use made_up_film::{RECIPE, made_up_front};
use serde::Serialize;
use shapes::auto_pass::{PassWarningCode, Stage, TrainedPassResult};
use shapes::case_file_v1::CaseFileV1;
use shapes::confidence::{ConfidenceBand, ConfidenceReason};
use shapes::film::{Film, FilmImage, FilmKind};
use shapes::film_geometry::FilmGeometry;
use shapes::levels::Level;
use shapes::made_up::MadeUpRecipe;
use shapes::marks::PedicleMark;
use shapes::naming::NamingFlag;
use shapes::vertebra::VertebraMarks;
use shapes::watch_frames::{LevelConfidence, OutlineQuad, WatchFrame};
use std::rc::Rc;
use std::sync::OnceLock;
use trained_finder::framing::FrameReason;
use trained_finder::outline::Peak;
use trained_finder::run::{find_on_front_film, find_on_picture};
use trained_finder::watch::TrainedPass;
use trained_finder::weights::Weights;

/// Scores and places in fractions of the picture, as the network gives
/// them, within this of the reference.
const READING_TOLERANCE: f64 = 1e-5;
/// Settled corners within this many film pixels of the reference.
const CORNER_TOLERANCE_PX: f64 = 0.02;
const FIRST_ID: u32 = 1000;
/// The product path's answer on this film, from `FIRST_ID`, as recorded.
const RECORDED: &str =
    include_str!("../../../proving-ground/known-answers/trained-pass/front.json");

/// Checks one list of peaks against the reference's: the same cells in the
/// same order, each score and place within tolerance. Returns the largest
/// score gap and the largest place gap.
fn compare_peaks(label: &str, got: &[Peak], want: &[Peak]) -> (f64, f64) {
    let cells = |peaks: &[Peak]| peaks.iter().map(|peak| peak.cell).collect::<Vec<_>>();
    assert_eq!(cells(got), cells(want), "{label}: the same cells in order");
    let (mut score, mut place) = (0.0f64, 0.0f64);
    for (a, b) in got.iter().zip(want) {
        score = score.max((a.score - b.score).abs());
        place = place
            .max(quad_gap(&a.corners, &b.corners))
            .max(geometry_fixtures::point_gap(a.centre, b.centre));
    }
    assert!(score <= READING_TOLERANCE, "{label}: score gap {score:e}");
    assert!(place <= READING_TOLERANCE, "{label}: place gap {place:e}");
    (score, place)
}

#[test]
fn the_whole_find_matches_the_known_answers() {
    let picture = made_up_front().top_eight_bits();
    let size = [picture.width(), picture.height()].map(f64::from);
    let found = find_on_picture(picture).expect("the finder runs");
    let run = pipeline();

    let mut readings = Vec::new();
    for (pass, got) in [
        ("pass1", Some(&found.first)),
        ("pass2", found.second.as_ref()),
    ] {
        let got = got.expect("both readings ran");
        let want = reading(&run[pass]);
        readings.push(compare_peaks(
            &format!("{pass} peaks"),
            &got.peaks,
            &want.peaks,
        ));
        readings.push(compare_peaks(
            &format!("{pass} outlines"),
            &got.outlines,
            &want.outlines,
        ));
    }

    let recorded = &run["frame"];
    let bounds = [0, 1, 2, 3].map(|k| recorded["box"][k].as_i64().expect("an edge"));
    assert_eq!(found.frame.bounds, bounds);
    assert_eq!(recorded["reason"], "small-spine crop");
    assert_eq!(found.frame.reason, FrameReason::SmallSpineCrop);
    assert!(!found.frame.whole_film());
    assert!(found.warnings.is_empty());

    let chain_gap = compare_chains(
        &found.chain,
        &links(&run["chainInCropFractions"]),
        READING_TOLERANCE,
    );

    let finals = run["final"]["outlines"].as_array().expect("a list");
    assert_eq!(found.vertebrae.len(), finals.len());
    let (mut corner_gap, mut score_gap) = (0.0f64, 0.0f64);
    for (index, (got, want)) in found.vertebrae.iter().zip(finals).enumerate() {
        assert_eq!(
            got.level_index as u64,
            want["level_index"].as_u64().expect("a level"),
            "vertebra {index}: level"
        );
        assert_eq!(
            got.filled_in,
            want["filled_in"].as_bool().expect("a flag"),
            "vertebra {index}: filled in"
        );
        score_gap = score_gap.max((got.score - number(&want["score"])).abs());
        // The reference gives fractions of the film; times its size they
        // are pixels measured from its outer corner, as ours are.
        let want_corners = geometry_fixtures::quad(&want["corners"])
            .map(|corner| [corner[0] * size[0], corner[1] * size[1]]);
        corner_gap = corner_gap.max(quad_gap(&got.corners, &want_corners));
    }
    assert!(
        corner_gap <= CORNER_TOLERANCE_PX,
        "corner gap {corner_gap:e} px"
    );
    assert!(score_gap <= READING_TOLERANCE, "score gap {score_gap:e}");
    eprintln!(
        "largest gaps: readings (score, place) {readings:?}; chain {chain_gap:e}; \
         final corners {corner_gap:e} px, final scores {score_gap:e}"
    );
}

/// The made-up film as the product reads it: its PNG decoded by the film
/// reader, and the record made from what the reader found, as `spine pass`
/// makes it.
fn product_film() -> &'static (DecodedFilm, Film) {
    static FILM: OnceLock<(DecodedFilm, Film)> = OnceLock::new();
    FILM.get_or_init(|| {
        let decoded = film_reader::read_film(&made_up_front().png, FilmKind::Front)
            .expect("the made-up PNG reads");
        let facts = &decoded.facts;
        let film = Film {
            kind: facts.kind,
            image: FilmImage {
                path: facts.kind.png_path().to_owned(),
                width: facts.width,
                height: facts.height,
                bits_stored: facts.bits_stored,
            },
            source: facts.source,
            kept_tags: facts.kept_tags.clone(),
            values: facts.values.clone(),
            flags: facts.flags.clone(),
            blanked: Vec::new(),
            geometry: FilmGeometry {
                quarter_turns: facts.exif_quarter_turns.unwrap_or_default(),
                ..FilmGeometry::UNKNOWN
            },
        };
        (decoded, film)
    })
}

/// The one-call answer on the product's film.
fn direct() -> &'static TrainedPassResult {
    static RESULT: OnceLock<TrainedPassResult> = OnceLock::new();
    RESULT.get_or_init(|| {
        let (decoded, film) = product_film();
        find_on_front_film(decoded, film, FIRST_ID).expect("the finder runs")
    })
}

/// A stepped run on the product's film: its answer, every frame, and how
/// many calls it took.
struct Watched {
    result: TrainedPassResult,
    frames: Vec<WatchFrame>,
    calls: u32,
}

fn watch(work_units: u32) -> Watched {
    let (decoded, film) = product_film();
    let mut pass = TrainedPass::new(Rc::new(decoded.clone()), film.clone(), FIRST_ID)
        .expect("the record suits the film");
    let idle = pass.advance(0).expect("a report");
    assert!(!idle.done && idle.frames.is_empty());
    let (mut frames, mut calls) = (Vec::new(), 0);
    loop {
        let step = pass.advance(work_units).expect("the finder runs");
        calls += 1;
        assert!((0.0..=1.0).contains(&step.fraction));
        frames.extend(step.frames);
        if step.done {
            assert_eq!((step.stage, step.fraction), (Stage::Confidence, 1.0));
            break;
        }
    }
    let after = pass.advance(work_units).expect("a report");
    assert!(after.done && after.frames.is_empty());
    Watched {
        result: pass.result().expect("an answer"),
        frames,
        calls,
    }
}

fn one_unit_at_a_time() -> &'static Watched {
    static WATCHED: OnceLock<Watched> = OnceLock::new();
    WATCHED.get_or_init(|| watch(1))
}

#[test]
fn the_product_path_finds_the_spine_in_order() {
    let (_, film) = product_film();
    let result = direct();
    let marks = &result.marks;
    let levels: Vec<Level> = marks.vertebrae.iter().map(|v| v.level).collect();
    let filled = marks
        .vertebrae
        .iter()
        .filter(|v| v.marks.corners().iter().any(|mark| mark.inferred))
        .count();
    eprintln!(
        "the product path found {} vertebrae ({filled} filled in): {:?}; warnings {:?}",
        levels.len(),
        levels.iter().map(|level| level.name()).collect::<Vec<_>>(),
        result.warnings
    );
    assert!((12..=17).contains(&levels.len()));
    assert!(levels.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(levels.iter().all(|level| Level::MEASURED.contains(level)));
    assert_eq!(marks.naming.flags, [NamingFlag::Estimated]);
    assert_eq!(marks.landmarks.c7, None);
    for (index, vertebra) in marks.vertebrae.iter().enumerate() {
        assert_eq!(vertebra.id, FIRST_ID + index as u32);
        let VertebraMarks::Front { points } = &vertebra.marks else {
            panic!("front marks");
        };
        let not_seen = PedicleMark::NotVisible {
            source: shapes::marks::MarkSource::Auto,
        };
        assert_eq!((points.ped_l, points.ped_r), (not_seen, not_seen));
        let confidence = &vertebra.confidence;
        let inferred = points.corners().map(|mark| mark.inferred);
        assert!(inferred.iter().all(|&i| i == inferred[0]));
        let expected_reasons = if inferred[0] {
            assert_eq!(confidence.score, 0.0);
            vec![
                ConfidenceReason::NoEvidence,
                ConfidenceReason::LevelUncertain,
            ]
        } else {
            vec![ConfidenceReason::LevelUncertain]
        };
        assert_eq!(confidence.reasons, expected_reasons);
        assert_eq!(confidence.band, ConfidenceBand::for_score(confidence.score));
        assert!(!confidence.calibrated);
    }
    assert!(
        result
            .warnings
            .iter()
            .all(|w| w.code == PassWarningCode::LowEvidence)
    );

    // The strict reader takes the marks in a case.
    let mut case = CaseFileV1::new("0.1.0");
    case.films = vec![film.clone()];
    case.marks = vec![marks.clone()];
    case.next_vertebra_id = FIRST_ID + levels.len() as u32;
    shapes::reading::check_case_file(&case).expect("the strict reader takes the marks");
    let text = serde_json::to_string(result).expect("the answer is JSON");
    let back: TrainedPassResult = serde_json::from_str(&text).expect("and reads back");
    assert_eq!(back, *result);
}

/// What `front.json` holds: the case the film came from, the first id, the
/// record the pass was given, and its answer.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Recording<'a> {
    recipe: MadeUpRecipe,
    first_vertebra_id: u32,
    film: &'a Film,
    result: &'a TrainedPassResult,
}

#[test]
fn the_product_path_gives_the_recorded_answer() {
    let (_, film) = product_film();
    let recording = Recording {
        recipe: RECIPE,
        first_vertebra_id: FIRST_ID,
        film,
        result: direct(),
    };
    let text = serde_json::to_string_pretty(&recording).expect("the recording is JSON") + "\n";
    if text != RECORDED {
        // A change to the finder on purpose: this text is the new recording.
        println!("{text}");
        panic!(
            "the native trained pass no longer gives the recorded answer; if the change \
             is meant, run this test with --nocapture and put what it prints in front.json"
        );
    }
}

#[test]
fn watching_one_unit_at_a_time_gives_the_direct_answer_and_its_frames() {
    let watched = one_unit_at_a_time();
    let marks = &direct().marks;
    assert_eq!(watched.result, *direct());

    // One unit per call: the intake, the first reading's start and its
    // units, the second reading's start and units when the film was
    // framed, the chain, the names, and one corner patch per vertebra.
    let units = Weights::baked().expect("weights").spine_net().units();
    let count = marks.vertebrae.len() as u32;
    let one_reading = 2 + units + 2 + count;
    let two_readings = one_reading + 1 + units;
    assert!(
        [one_reading, two_readings].contains(&watched.calls),
        "{} calls",
        watched.calls
    );
    eprintln!(
        "{} units ({} readings, {count} patches)",
        watched.calls,
        if watched.calls == one_reading { 1 } else { 2 }
    );

    let levels: Vec<Level> = marks.vertebrae.iter().map(|v| v.level).collect();
    let quads: Vec<OutlineQuad> = marks
        .vertebrae
        .iter()
        .map(|v| OutlineQuad::new(v.level, &v.marks))
        .collect();
    let confidence: Vec<LevelConfidence> = marks
        .vertebrae
        .iter()
        .map(|v| LevelConfidence {
            level: v.level,
            score: v.confidence.score,
            band: v.confidence.band,
        })
        .collect();
    let mut kinds = Vec::new();
    for frame in &watched.frames {
        assert!(frame.is_keyframe());
        match frame {
            WatchFrame::StageStarted { stage, .. } => kinds.push(format!("start {stage:?}")),
            WatchFrame::Warning { warning, stage, .. } => {
                assert_eq!(warning.code, PassWarningCode::LowEvidence);
                assert_eq!(*stage, Stage::Evidence);
            }
            WatchFrame::ChainLocked { bodies, .. } => {
                kinds.push("chain".to_owned());
                // From the lowest upward, each filled-in one inferred.
                let locked: Vec<Level> = bodies.iter().rev().map(|body| body.level).collect();
                assert_eq!(locked, levels);
                for (body, vertebra) in bodies.iter().rev().zip(&marks.vertebrae) {
                    let inferred = vertebra.marks.corners()[0].inferred;
                    assert_eq!(body.inferred, inferred);
                }
            }
            WatchFrame::LevelsNamed {
                naming,
                levels: named,
                ..
            } => {
                kinds.push("names".to_owned());
                assert_eq!(*named, levels);
                assert_eq!(*naming, marks.naming);
            }
            WatchFrame::Confidence { levels: scored, .. } => {
                kinds.push("confidence".to_owned());
                assert_eq!(*scored, confidence);
            }
            WatchFrame::Outlines {
                quads: drawn,
                is_final,
                ..
            } => {
                kinds.push("outlines".to_owned());
                assert!(*is_final);
                assert_eq!(*drawn, quads);
            }
            other => panic!("a frame the trained pass does not send: {other:?}"),
        }
        assert!(matches!(
            frame,
            WatchFrame::StageStarted {
                film: FilmKind::Front,
                ..
            } | WatchFrame::Warning {
                film: FilmKind::Front,
                ..
            } | WatchFrame::ChainLocked {
                film: FilmKind::Front,
                ..
            } | WatchFrame::LevelsNamed {
                film: FilmKind::Front,
                ..
            } | WatchFrame::Confidence {
                film: FilmKind::Front,
                ..
            } | WatchFrame::Outlines {
                film: FilmKind::Front,
                ..
            }
        ));
    }
    assert_eq!(
        kinds,
        [
            "start Evidence",
            "start Chain",
            "chain",
            "start Naming",
            "names",
            "start Confidence",
            "confidence",
            "outlines"
        ]
    );
}

#[test]
fn frames_do_not_depend_on_slice_size() {
    let large = watch(1000);
    let small = one_unit_at_a_time();
    assert_eq!(large.frames.len(), small.frames.len());
    for (index, (a, b)) in large.frames.iter().zip(&small.frames).enumerate() {
        assert!(a == b, "frame {index} changed with the slice size");
    }
    assert_eq!(large.result, small.result);
    assert!(large.calls < small.calls);
}

/// The engine has no clock, so a whole find is timed here, natively, by
/// hand: `cargo test --release -p trained-finder --test
/// finds_the_vertebrae_on_a_made_up_film -- --ignored --nocapture`.
#[test]
#[ignore = "a timing, run by hand in release"]
#[allow(clippy::disallowed_types)]
fn a_whole_find_timed_natively() {
    use std::time::Instant;
    let (decoded, film) = product_film();
    Weights::baked().expect("weights");
    let started = Instant::now();
    let result = find_on_front_film(decoded, film, FIRST_ID).expect("the finder runs");
    let whole = started.elapsed();

    let mut pass = TrainedPass::new(Rc::new(decoded.clone()), film.clone(), FIRST_ID)
        .expect("the record suits the film");
    let mut times = Vec::new();
    loop {
        let started = Instant::now();
        let step = pass.advance(1).expect("the finder runs");
        times.push(started.elapsed().as_secs_f64() * 1000.0);
        if step.done {
            break;
        }
    }
    let count = result.marks.vertebrae.len();
    let units = Weights::baked().expect("weights").spine_net().units() as usize;
    let framed = times.len() == 2 + units + 1 + units + 2 + count;
    // The median and the largest of some units, in milliseconds.
    let spread = |mut some: Vec<f64>| {
        some.sort_by(f64::total_cmp);
        (some[some.len() / 2], some[some.len() - 1])
    };
    let mut network = times[2..2 + units].to_vec();
    if framed {
        network.extend(&times[3 + units..3 + 2 * units]);
    }
    let patches = times.len() - count..times.len();
    eprintln!(
        "whole find: {:.0} ms, {count} vertebrae, {} units",
        whole.as_secs_f64() * 1000.0,
        times.len()
    );
    eprintln!(
        "intake {:.1} ms; first start {:.1} ms; second start {}",
        times[0],
        times[1],
        if framed {
            format!("{:.1} ms", times[2 + units])
        } else {
            "none".to_owned()
        }
    );
    eprintln!(
        "network units: median and largest {:.2?} ms",
        spread(network)
    );
    eprintln!(
        "chain {:.1} ms; names {:.2} ms; corner patches: median and largest {:.1?} ms",
        times[patches.start - 2],
        times[patches.start - 1],
        spread(times[patches].to_vec())
    );
}
