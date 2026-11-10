//! Every watch-frame kind, and whether the worker may thin it. The
//! contract keeps every frame except two kinds: `joint-progress` frames,
//! and `evidence` frames with `final: false` whose `map` is not
//! `wavefront`. The table below is that rule written out by hand, one row
//! per case; the last test fails when a kind is added to `WatchFrame`
//! without a row here, so a new kind cannot arrive undecided.

use serde_json::Value;
use shapes::auto_pass::{PassWarning, PassWarningCode, Stage};
use shapes::confidence::ConfidenceBand;
use shapes::film::FilmKind;
use shapes::levels::Level;
use shapes::naming::{ChosenBy, LevelNaming, Template};
use shapes::pixels::{Px, PxRect};
use shapes::watch_frames::{
    ChainBody, LevelConfidence, MapKind, OutlineQuad, PreviewTile, Segment, TileEncoding,
    WatchFrame, WidthSample,
};
use std::collections::BTreeSet;

const FILM: FilmKind = FilmKind::Front;

fn px() -> Px {
    Px::new(10.0, 20.0)
}

fn rect() -> PxRect {
    PxRect {
        left: 0.0,
        top: 0.0,
        right: 8.0,
        bottom: 8.0,
    }
}

fn tile() -> PreviewTile {
    PreviewTile {
        width: 1,
        height: 1,
        origin: px(),
        step_across: Px::new(1.0, 0.0),
        step_down: Px::new(0.0, 1.0),
        encoding: TileEncoding::Unit,
        data: vec![128],
    }
}

fn evidence(map: MapKind, is_final: bool) -> WatchFrame {
    WatchFrame::Evidence {
        film: FILM,
        stage: Stage::Evidence,
        map,
        step: 1,
        steps: 9,
        is_final,
        scale_mm: Some(2.0),
        tile: tile(),
    }
}

fn body() -> ChainBody {
    let line = Segment { a: px(), b: px() };
    ChainBody {
        level: Level::T1,
        sup: line,
        inf: line,
        inferred: false,
    }
}

fn outlines(is_final: bool) -> WatchFrame {
    WatchFrame::Outlines {
        film: FILM,
        stage: Stage::Confidence,
        quads: vec![OutlineQuad {
            level: Level::T1,
            corners: [px(); 4],
        }],
        is_final,
    }
}

/// Every case of the rule: a label, a frame, and whether it may be thinned.
fn decisions() -> Vec<(&'static str, WatchFrame, bool)> {
    vec![
        (
            "stage-started",
            WatchFrame::StageStarted {
                film: FILM,
                stage: Stage::Evidence,
            },
            false,
        ),
        (
            "evidence, unfinished ridge map",
            evidence(MapKind::Ridge, false),
            true,
        ),
        (
            "evidence, unfinished phase edges",
            evidence(MapKind::PhaseEdge, false),
            true,
        ),
        (
            "evidence, unfinished ring map",
            evidence(MapKind::Ring, false),
            true,
        ),
        (
            "evidence, finished ridge map",
            evidence(MapKind::Ridge, true),
            false,
        ),
        (
            "evidence, spreading search",
            evidence(MapKind::Wavefront, false),
            false,
        ),
        (
            "evidence, finished search",
            evidence(MapKind::Wavefront, true),
            false,
        ),
        (
            "end-zones",
            WatchFrame::EndZones {
                film: FILM,
                stage: Stage::CentreLine,
                top: rect(),
                bottom: rect(),
                top_cropped: false,
                bottom_cropped: true,
            },
            false,
        ),
        (
            "centre-line, coarse",
            WatchFrame::CentreLine {
                film: FILM,
                stage: Stage::CentreLine,
                points: vec![px(), px()],
                refined: false,
            },
            false,
        ),
        (
            "centre-line, refined",
            WatchFrame::CentreLine {
                film: FILM,
                stage: Stage::CentreLine,
                points: vec![px(), px()],
                refined: true,
            },
            false,
        ),
        (
            "width",
            WatchFrame::Width {
                film: FILM,
                stage: Stage::CentreLine,
                samples: vec![WidthSample {
                    left: px(),
                    right: px(),
                    confidence: 0.5,
                }],
            },
            false,
        ),
        (
            "strip-shape",
            WatchFrame::StripShape {
                film: FILM,
                stage: Stage::Strip,
                centres: vec![px()],
                normals: vec![Px::new(1.0, 0.0)],
                column_px: 1.0,
                columns: 8,
            },
            false,
        ),
        (
            "strip-rows",
            WatchFrame::StripRows {
                film: FILM,
                stage: Stage::Strip,
                first_row: 0,
                rows: 1,
                columns: 8,
                image: vec![0; 8],
                valid: vec![0xff],
            },
            false,
        ),
        (
            "barcode",
            WatchFrame::Barcode {
                film: FILM,
                stage: Stage::Strip,
                values: 1.0f32.to_le_bytes().to_vec(),
                high_pass: 0.0f32.to_le_bytes().to_vec(),
            },
            false,
        ),
        (
            "sweep",
            WatchFrame::Sweep {
                film: FILM,
                stage: Stage::Chain,
                level: Level::T1,
                heat: vec![0; 256],
                tentative: vec![body()],
            },
            false,
        ),
        (
            "template-scored",
            WatchFrame::TemplateScored {
                film: FILM,
                stage: Stage::Chain,
                template: Template::Standard,
                score: 1.0,
                chance: 0.9,
            },
            false,
        ),
        (
            "joint-progress",
            WatchFrame::JointProgress {
                film: FILM,
                stage: Stage::Chain,
                start: 0,
                round: 1,
                upper: 2.0,
                lower: 1.0,
            },
            true,
        ),
        (
            "chain-locked",
            WatchFrame::ChainLocked {
                film: FILM,
                stage: Stage::Chain,
                bodies: vec![body()],
            },
            false,
        ),
        (
            "levels-named",
            WatchFrame::LevelsNamed {
                film: FILM,
                stage: Stage::Naming,
                naming: LevelNaming {
                    template: Template::Standard,
                    chosen_by: ChosenBy::Engine,
                    alternatives: Vec::new(),
                    from_top: Some(vec![Level::T1]),
                    from_bottom: Some(vec![Level::T1]),
                    flags: Vec::new(),
                    l5_confirmed_on: None,
                },
                levels: vec![Level::T1],
            },
            false,
        ),
        (
            "confidence",
            WatchFrame::Confidence {
                film: FILM,
                stage: Stage::Confidence,
                levels: vec![LevelConfidence {
                    level: Level::T1,
                    score: 0.8,
                    band: ConfidenceBand::High,
                }],
            },
            false,
        ),
        ("outlines, on the way", outlines(false), false),
        ("outlines, final", outlines(true), false),
        (
            "warning",
            WatchFrame::Warning {
                film: FILM,
                stage: Stage::Strip,
                warning: PassWarning {
                    code: PassWarningCode::ScaleAssumed,
                    film: FILM,
                    level: None,
                    region: None,
                },
            },
            false,
        ),
    ]
}

fn kind_of(frame: &WatchFrame) -> String {
    match serde_json::to_value(frame).expect("a frame serialises") {
        Value::Object(map) => match map.get("kind") {
            Some(Value::String(kind)) => kind.clone(),
            other => panic!("a frame names its kind, got {other:?}"),
        },
        other => panic!("expected an object, got {other}"),
    }
}

#[test]
fn only_unfinished_previews_and_joint_progress_may_be_thinned() {
    for (label, frame, may_be_thinned) in decisions() {
        assert_eq!(
            !frame.is_keyframe(),
            may_be_thinned,
            "{label}: may it be thinned?"
        );
    }
}

#[test]
fn every_frame_kind_has_a_decision() {
    let schema = serde_json::to_value(schemars::schema_for!(WatchFrame)).expect("a schema is JSON");
    let variants = schema["oneOf"]
        .as_array()
        .expect("a tagged enum's schema lists its variants");
    let in_the_shapes: BTreeSet<String> = variants
        .iter()
        .map(|variant| {
            variant["properties"]["kind"]["const"]
                .as_str()
                .expect("each variant names its kind")
                .to_owned()
        })
        .collect();
    let decided: BTreeSet<String> = decisions()
        .iter()
        .map(|(_, frame, _)| kind_of(frame))
        .collect();
    assert_eq!(decided, in_the_shapes, "every kind has a row in the table");
}
