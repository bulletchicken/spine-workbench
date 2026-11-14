//! Side routing and build capability cross the wire without assumptions.

use serde_json::{Value, json};
use shapes::SideFinder;
use shapes::auto_pass::{AutoPassInput, FrontFinder};
use shapes::engine_info::EngineInfo;
use shapes::errors::{EngineError, ErrorCode};

fn old_input() -> Value {
    json!({
        "caseVersion": 7,
        "films": [],
        "options": {
            "acquisition": "unknown",
            "population": "adult",
            "template": null
        },
        "firstVertebraId": 23,
        "frontFinder": "trained"
    })
}

#[test]
fn side_finder_names_and_default_are_exact() {
    assert_eq!(SideFinder::default(), SideFinder::HandBuilt);
    for (finder, name) in [
        (SideFinder::Trained, "trained"),
        (SideFinder::HandBuilt, "hand-built"),
    ] {
        assert_eq!(serde_json::to_value(finder).unwrap(), json!(name));
        assert_eq!(
            serde_json::from_value::<SideFinder>(json!(name)).unwrap(),
            finder
        );
    }
    for invalid in [
        json!(null),
        json!("unknown"),
        json!("handBuilt"),
        json!(false),
    ] {
        assert!(serde_json::from_value::<SideFinder>(invalid).is_err());
    }
}

#[test]
fn omitted_side_finder_keeps_hand_built_and_the_front_choice() {
    let old = old_input();
    let input: AutoPassInput = serde_json::from_value(old.clone()).unwrap();
    assert_eq!(input.side_finder, SideFinder::HandBuilt);
    assert_eq!(input.front_finder, FrontFinder::Trained);
    let mut written = serde_json::to_value(&input).unwrap();
    assert_eq!(written["sideFinder"], json!("hand-built"));
    written.as_object_mut().unwrap().remove("sideFinder");
    assert_eq!(written, old);
}

#[test]
fn either_side_choice_roundtrips_independently_of_the_front_choice() {
    for front in ["trained", "hand-built"] {
        for side in ["trained", "hand-built"] {
            let mut value = old_input();
            value["frontFinder"] = json!(front);
            value["sideFinder"] = json!(side);
            let input: AutoPassInput = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(serde_json::to_value(&input).unwrap(), value);
        }
    }
}

#[test]
fn null_unknown_or_mistyped_side_finders_are_refused() {
    for invalid in [json!(null), json!("unknown"), json!("HandBuilt"), json!(0)] {
        let mut value = old_input();
        value["sideFinder"] = invalid;
        assert!(serde_json::from_value::<AutoPassInput>(value).is_err());
    }
    let mut unknown_field = old_input();
    unknown_field["side_finder"] = json!("trained");
    assert!(serde_json::from_value::<AutoPassInput>(unknown_field).is_err());

    let mut missing_front = old_input();
    missing_front.as_object_mut().unwrap().remove("frontFinder");
    assert!(serde_json::from_value::<AutoPassInput>(missing_front).is_err());
}

#[test]
fn old_engine_info_defaults_to_unavailable() {
    let old = json!({
        "engineVersion": "old",
        "shapesFingerprint": "old-shapes",
        "caseFormat": "spine-case",
        "caseVersion": 1
    });
    let info: EngineInfo = serde_json::from_value(old.clone()).unwrap();
    assert!(!info.trained_side_finder);
    let mut written = serde_json::to_value(info).unwrap();
    assert_eq!(written["trainedSideFinder"], json!(false));
    written.as_object_mut().unwrap().remove("trainedSideFinder");
    assert_eq!(written, old);
}

#[test]
fn current_engine_info_always_writes_a_boolean_capability() {
    let mut info = EngineInfo::current("contract-test");
    assert!(!info.trained_side_finder);
    for available in [false, true] {
        info.trained_side_finder = available;
        let written = serde_json::to_value(&info).unwrap();
        assert_eq!(
            written,
            json!({
                "engineVersion": "contract-test",
                "shapesFingerprint": shapes::SHAPES_FINGERPRINT,
                "caseFormat": shapes::CASE_FORMAT,
                "caseVersion": shapes::CASE_VERSION,
                "trainedSideFinder": available
            })
        );
        assert_eq!(serde_json::from_value::<EngineInfo>(written).unwrap(), info);
    }
    for invalid in [json!(null), json!("false"), json!(0)] {
        let mut value = serde_json::to_value(&info).unwrap();
        value["trainedSideFinder"] = invalid;
        assert!(serde_json::from_value::<EngineInfo>(value).is_err());
    }
}

#[test]
fn side_refusal_codes_keep_the_engine_error_shape() {
    for (code, name) in [
        (ErrorCode::SideFinderUnavailable, "side-finder-unavailable"),
        (ErrorCode::PassFacingRequired, "pass-facing-required"),
    ] {
        assert_eq!(code.name(), name);
        assert_eq!(serde_json::to_value(code).unwrap(), json!(name));
        assert_eq!(
            serde_json::from_value::<ErrorCode>(json!(name)).unwrap(),
            code
        );
        let error = EngineError::new(code, "side request refused").with_detail("side");
        let written = serde_json::to_value(&error).unwrap();
        assert_eq!(
            written,
            json!({
                "code": name,
                "message": "side request refused",
                "detail": "side"
            })
        );
        assert_eq!(
            serde_json::from_value::<EngineError>(written).unwrap(),
            error
        );
    }
}

#[cfg(feature = "typescript")]
#[test]
fn typescript_input_is_optional_but_capability_output_is_required() {
    use ts_rs::{Config, TS};

    let config = Config::new();
    let input = AutoPassInput::decl(&config);
    assert!(input.contains("sideFinder?: SideFinder,"), "{input}");
    let info = EngineInfo::decl(&config);
    assert!(info.contains("trainedSideFinder: boolean,"), "{info}");
    assert!(!info.contains("trainedSideFinder?"), "{info}");
    assert!(!info.contains("boolean | null"), "{info}");
}
