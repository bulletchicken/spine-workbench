//! Mark, landmark and level names are written exactly as the conventions
//! spell them, everywhere: supL, supR, infL, infR, pedL, pedR on front
//! films; supA, supP, infA, infP on side films; sup and inf, patient-left
//! and patient-right on bending films; sacL, sacR, sacA, sacP for the
//! landmarks; C1 to C7, T1 to T13, L1 to L7 for the levels.

use serde::Serialize;
use serde_json::Value;
use shapes::landmarks::{LandmarkName, SacrumTop};
use shapes::levels::Level;
use shapes::marks::{
    EndplateLine, EndplateLines, FrontMarks, LineEnd, Mark, MarkName, MarkSource, PedicleMark,
    SideMarks,
};
use shapes::pixels::Px;
use shapes::toolbox::PinTarget;
use std::collections::BTreeSet;

fn keys(value: &impl Serialize) -> BTreeSet<String> {
    match serde_json::to_value(value).expect("a shape serialises") {
        Value::Object(map) => map.keys().cloned().collect(),
        other => panic!("expected an object, got {other}"),
    }
}

fn set<const N: usize>(names: [&str; N]) -> BTreeSet<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

fn spelling(value: &impl Serialize) -> String {
    match serde_json::to_value(value).expect("a name serialises") {
        Value::String(name) => name,
        other => panic!("expected a string, got {other}"),
    }
}

fn mark() -> Mark {
    Mark::auto(Px::new(1.0, 2.0))
}

#[test]
fn front_marks_are_named_exactly() {
    let front = FrontMarks {
        sup_l: mark(),
        sup_r: mark(),
        inf_l: mark(),
        inf_r: mark(),
        ped_l: PedicleMark::NotVisible {
            source: MarkSource::Auto,
        },
        ped_r: PedicleMark::NotVisible {
            source: MarkSource::Hand,
        },
    };
    assert_eq!(
        keys(&front),
        set(["supL", "supR", "infL", "infR", "pedL", "pedR"])
    );
}

#[test]
fn side_marks_are_named_exactly() {
    let side = SideMarks {
        sup_a: mark(),
        sup_p: mark(),
        inf_a: mark(),
        inf_p: mark(),
    };
    assert_eq!(keys(&side), set(["supA", "supP", "infA", "infP"]));
}

#[test]
fn bending_films_carry_endplate_lines() {
    let lines = EndplateLines {
        sup: [mark(), mark()],
        inf: [mark(), mark()],
    };
    assert_eq!(keys(&lines), set(["sup", "inf"]));
}

#[test]
fn bending_points_are_named_by_line_and_end() {
    let target = PinTarget::Endplate {
        vertebra: 7,
        line: EndplateLine::Inf,
        end: LineEnd::PatientLeft,
    };
    assert_eq!(
        serde_json::to_value(target).expect("a pin target serialises"),
        serde_json::json!({ "on": "endplate", "vertebra": 7, "line": "inf", "end": "patient-left" })
    );
}

#[test]
fn mark_names_match_the_mark_fields() {
    let names: Vec<String> = MarkName::FRONT
        .iter()
        .chain(MarkName::SIDE.iter())
        .map(|name| {
            assert_eq!(spelling(name), name.name());
            spelling(name)
        })
        .collect();
    assert_eq!(
        names,
        [
            "supL", "supR", "infL", "infR", "pedL", "pedR", "supA", "supP", "infA", "infP"
        ]
    );
}

#[test]
fn landmark_names_follow_the_corner_style() {
    let names: Vec<String> = [
        LandmarkName::SacL,
        LandmarkName::SacR,
        LandmarkName::SacA,
        LandmarkName::SacP,
    ]
    .iter()
    .map(spelling)
    .collect();
    assert_eq!(names, ["sacL", "sacR", "sacA", "sacP"]);

    let front = SacrumTop::Front {
        sac_l: mark(),
        sac_r: mark(),
    };
    assert_eq!(keys(&front), set(["view", "sacL", "sacR"]));
    let side = SacrumTop::Side {
        sac_a: mark(),
        sac_p: mark(),
    };
    assert_eq!(keys(&side), set(["view", "sacA", "sacP"]));
}

#[test]
fn level_names_are_exact() {
    let names: Vec<String> = Level::ALL.iter().map(spelling).collect();
    assert_eq!(
        names,
        [
            "C1", "C2", "C3", "C4", "C5", "C6", "C7", "T1", "T2", "T3", "T4", "T5", "T6", "T7",
            "T8", "T9", "T10", "T11", "T12", "T13", "L1", "L2", "L3", "L4", "L5", "L6", "L7"
        ]
    );
}

#[test]
fn a_mark_cannot_be_written_any_other_way() {
    let wrong = r#"{"supl":{"at":[1.0,2.0],"source":"auto","pinned":false,"inferred":false}}"#;
    assert!(serde_json::from_str::<SideMarks>(wrong).is_err());
}
