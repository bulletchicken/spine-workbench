//! Strict reading refuses everything a damaged or hand-edited case file
//! could carry: unknown fields, a wrong format or version, missing fields,
//! non-finite numbers, bad levels, points far off the film and values that
//! contradict each other. Each refusal names the offending JSON path.

use serde_json::{Value, json};
use shapes::case_file_v1::CaseFileV1;
use shapes::errors::{EngineError, ErrorCode};
use shapes::reading::{check_case_file, read_case_json};

const FOUR_FILMS: &str =
    include_str!("../../../proving-ground/known-answers/case-files/valid/four-films.json");

fn four_films() -> Value {
    serde_json::from_str(FOUR_FILMS).expect("the golden file is JSON")
}

fn read(value: &Value) -> Result<CaseFileV1, EngineError> {
    read_case_json(&serde_json::to_string(value).expect("a value serialises"))
}

/// Reads the golden four-film case after `change`, expecting a refusal.
fn refused(change: impl FnOnce(&mut Value)) -> EngineError {
    let mut value = four_films();
    change(&mut value);
    read(&value).expect_err("the changed case should be refused")
}

fn assert_refused(error: &EngineError, code: ErrorCode, detail: &str) {
    assert_eq!(error.code, code, "{error}");
    assert_eq!(error.detail.as_deref(), Some(detail), "{error}");
}

#[test]
fn the_unchanged_golden_case_reads() {
    assert!(read(&four_films()).is_ok());
}

#[test]
fn unknown_fields_are_refused_at_any_depth() {
    let error = refused(|case| case["notes"] = json!("free text"));
    assert_refused(&error, ErrorCode::CaseUnknownField, "notes");

    let error = refused(|case| case["films"][1]["geometry"]["originalName"] = json!("x"));
    assert_refused(
        &error,
        ErrorCode::CaseUnknownField,
        "films[1].geometry.originalName",
    );

    let error = refused(|case| case["films"][0]["keptTags"]["patientName"] = json!("x"));
    assert_refused(
        &error,
        ErrorCode::CaseUnknownField,
        "films[0].keptTags.patientName",
    );
}

#[test]
fn unknown_fields_inside_tagged_values_are_refused() {
    let error = refused(|case| {
        case["marks"][0]["vertebrae"][0]["marks"]["points"]["pedL"]["note"] = json!(1);
    });
    assert_eq!(error.code, ErrorCode::CaseUnknownField, "{error}");
}

#[test]
fn a_wrong_or_missing_format_is_refused() {
    let error = refused(|case| case["format"] = json!("spine-case-2"));
    assert_refused(&error, ErrorCode::CaseWrongFormat, "format");

    let error = read_case_json("[1, 2, 3]").expect_err("not an object");
    assert_refused(&error, ErrorCode::CaseWrongFormat, "format");
}

#[test]
fn versions_other_than_one_are_refused() {
    let error = refused(|case| case["version"] = json!(3));
    assert_refused(&error, ErrorCode::CaseNewerVersion, "version");

    for version in [json!(0), json!("1"), json!(1.5), json!(null)] {
        let error = refused(|case| case["version"] = version);
        assert_refused(&error, ErrorCode::CaseInvalidValue, "version");
    }

    let error = refused(|case| {
        case.as_object_mut().expect("an object").remove("version");
    });
    assert_refused(&error, ErrorCode::CaseWrongFormat, "version");
}

#[test]
fn every_field_must_be_written_out_null_included() {
    let error = refused(|case| {
        case["films"][1]["geometry"]
            .as_object_mut()
            .expect("an object")
            .remove("mmPerPxVertical");
    });
    assert_refused(
        &error,
        ErrorCode::CaseInvalidValue,
        "films[1].geometry.mmPerPxVertical",
    );
}

#[test]
fn non_finite_numbers_are_refused() {
    let text = FOUR_FILMS.replacen("\"mmPerPx\": 0.125", "\"mmPerPx\": NaN", 1);
    assert_ne!(text, FOUR_FILMS);
    let error = read_case_json(&text).expect_err("NaN is not JSON");
    assert_eq!(error.code, ErrorCode::CaseInvalidValue);

    let mut case = read(&four_films()).expect("the golden case reads");
    case.films[1].geometry.mm_per_px = Some(f64::NAN);
    let error = check_case_file(&case).expect_err("NaN is not a scale");
    assert_refused(
        &error,
        ErrorCode::CaseInvalidValue,
        "films[1].geometry.mmPerPx",
    );

    let mut case = read(&four_films()).expect("the golden case reads");
    case.marks[0].vertebrae[0].confidence.score = f64::INFINITY;
    let error = check_case_file(&case).expect_err("infinity is not a score");
    assert_refused(
        &error,
        ErrorCode::CaseInvalidValue,
        "marks[0].vertebrae[0].confidence.score",
    );
}

#[test]
fn bad_level_names_are_refused() {
    for level in ["T14", "l5", "S1", ""] {
        let error = refused(|case| case["marks"][1]["vertebrae"][0]["level"] = json!(level));
        assert_refused(
            &error,
            ErrorCode::CaseInvalidValue,
            "marks[1].vertebrae[0].level",
        );
    }
}

#[test]
fn repeated_levels_are_refused() {
    let error = refused(|case| case["marks"][1]["vertebrae"][1]["level"] = json!("L4"));
    assert_refused(
        &error,
        ErrorCode::CaseInvalidValue,
        "marks[1].vertebrae[1].level",
    );
}

#[test]
fn a_transitional_level_needs_its_vertebra_count() {
    let error = refused(|case| case["marks"][1]["vertebrae"][1]["level"] = json!("L6"));
    assert_refused(
        &error,
        ErrorCode::CaseInvalidValue,
        "marks[1].vertebrae[1].level",
    );
    let mut value = four_films();
    value["marks"][1]["vertebrae"][1]["level"] = json!("L6");
    value["marks"][1]["naming"]["template"] = json!("extra-lumbar");
    assert!(read(&value).is_ok());
}

#[test]
fn points_far_off_their_film_are_refused() {
    let error = refused(|case| {
        case["marks"][1]["landmarks"]["hips"][0]["centre"]["at"] = json!([900.0, -3300.0]);
    });
    assert_refused(
        &error,
        ErrorCode::CaseInvalidValue,
        "marks[1].landmarks.hips[0].centre.at",
    );
    let mut value = four_films();
    value["marks"][1]["landmarks"]["hips"][0]["centre"]["at"] = json!([900.0, -600.0]);
    assert!(
        read(&value).is_ok(),
        "an inferred point just off the film is fine"
    );
}

#[test]
fn c7_is_exactly_the_centre_of_the_c7_vertebra() {
    // The bending film's C7 corners average to (609, 2292.5).
    let c7 = "marks[2].landmarks.c7";
    let error = refused(|case| case["marks"][2]["landmarks"]["c7"] = Value::Null);
    assert_refused(&error, ErrorCode::CaseInvalidValue, c7);
    let error = refused(|case| {
        case["marks"][1]["landmarks"]["c7"] = case["marks"][2]["landmarks"]["c7"].clone();
    });
    assert_refused(&error, ErrorCode::CaseInvalidValue, "marks[1].landmarks.c7");
    let error = refused(|case| case["marks"][2]["landmarks"]["c7"]["at"] = json!([609.0, 2292.6]));
    assert_refused(&error, ErrorCode::CaseInvalidValue, &format!("{c7}.at"));
    let error = refused(|case| case["marks"][2]["landmarks"]["c7"]["pinned"] = json!(true));
    assert_refused(&error, ErrorCode::CaseInvalidValue, &format!("{c7}.pinned"));
    let error = refused(|case| case["marks"][2]["landmarks"]["c7"]["source"] = json!("hand"));
    assert_refused(&error, ErrorCode::CaseInvalidValue, &format!("{c7}.source"));
    let error = refused(|case| case["marks"][2]["landmarks"]["c7"]["inferred"] = json!(true));
    assert_refused(
        &error,
        ErrorCode::CaseInvalidValue,
        &format!("{c7}.inferred"),
    );

    let mut value = four_films();
    value["marks"][2]["landmarks"]["c7"]["at"] = json!([609.0005, 2292.5]);
    assert!(read(&value).is_ok(), "rounding in the last places is fine");
    let mut value = four_films();
    value["marks"][2]["vertebrae"][0]["marks"]["lines"]["sup"][0]["inferred"] = json!(true);
    value["marks"][2]["landmarks"]["c7"]["inferred"] = json!(true);
    assert!(read(&value).is_ok(), "an inferred corner makes c7 inferred");
}

#[test]
fn only_pedicles_can_be_not_visible() {
    let error = refused(|case| {
        case["marks"][0]["vertebrae"][0]["marks"]["points"]["supL"] =
            json!({ "seen": "not-visible", "source": "auto" });
    });
    assert_eq!(error.code, ErrorCode::CaseUnknownField, "{error}");
}

#[test]
fn marks_must_suit_their_film() {
    let error = refused(|case| {
        let front = case["marks"][0]["vertebrae"][0]["marks"].clone();
        case["marks"][1]["vertebrae"][0]["marks"] = front;
    });
    assert_refused(
        &error,
        ErrorCode::CaseInvalidValue,
        "marks[1].vertebrae[0].marks.kind",
    );

    let error = refused(|case| case["marks"][1]["namedFor"] = json!("patient-left"));
    assert_refused(&error, ErrorCode::CaseInvalidValue, "marks[1].namedFor");

    let error = refused(|case| case["films"][1]["geometry"]["imageRight"] = json!("patient-right"));
    assert_refused(
        &error,
        ErrorCode::CaseInvalidValue,
        "films[1].geometry.imageRight",
    );
}

#[test]
fn films_and_their_marks_must_match_up() {
    let error = refused(|case| case["films"][1]["kind"] = json!("front"));
    assert_eq!(error.code, ErrorCode::CaseInvalidValue, "{error}");

    let error = refused(|case| {
        let films = case["films"].as_array_mut().expect("an array");
        films.retain(|film| film["kind"] != json!("side"));
    });
    assert_refused(&error, ErrorCode::CaseInvalidValue, "marks[1].film");

    let error = refused(|case| case["films"][0]["image"]["path"] = json!("films/side.png"));
    assert_refused(&error, ErrorCode::CaseInvalidValue, "films[0].image.path");
}

#[test]
fn vertebra_ids_are_unique_and_never_reused() {
    let error = refused(|case| case["marks"][1]["vertebrae"][0]["id"] = json!(1));
    assert_refused(
        &error,
        ErrorCode::CaseInvalidValue,
        "marks[1].vertebrae[0].id",
    );

    let error = refused(|case| case["nextVertebraId"] = json!(5));
    assert_refused(&error, ErrorCode::CaseInvalidValue, "nextVertebraId");
}

#[test]
fn only_dicom_films_have_kept_tags() {
    let error = refused(|case| {
        let tags = case["films"][0]["keptTags"].clone();
        case["films"][1]["keptTags"] = tags;
    });
    assert_refused(&error, ErrorCode::CaseInvalidValue, "films[1].keptTags");

    let error = refused(|case| case["films"][0]["keptTags"]["manufacturer"] = json!("a\nb"));
    assert_refused(
        &error,
        ErrorCode::CaseInvalidValue,
        "films[0].keptTags.manufacturer",
    );
}

#[test]
fn confirmations_need_what_they_confirm() {
    let error = refused(|case| case["films"][3]["geometry"]["confirmed"]["scale"] = json!(true));
    assert_refused(
        &error,
        ErrorCode::CaseInvalidValue,
        "films[3].geometry.confirmed.scale",
    );

    let error =
        refused(|case| case["films"][1]["geometry"]["confirmed"]["leftRight"] = json!(true));
    assert_refused(
        &error,
        ErrorCode::CaseInvalidValue,
        "films[1].geometry.confirmed.leftRight",
    );
}

#[test]
fn a_band_never_claims_more_than_its_score() {
    let error =
        refused(|case| case["marks"][1]["vertebrae"][1]["confidence"]["band"] = json!("high"));
    assert_refused(
        &error,
        ErrorCode::CaseInvalidValue,
        "marks[1].vertebrae[1].confidence.band",
    );
}

#[test]
fn hips_follow_the_film() {
    let error = refused(|case| case["marks"][0]["landmarks"]["hips"][1]["side"] = json!("left"));
    assert_refused(
        &error,
        ErrorCode::CaseInvalidValue,
        "marks[0].landmarks.hips",
    );

    let error = refused(|case| {
        case["marks"][0]["landmarks"]["sacrumTop"] =
            case["marks"][1]["landmarks"]["sacrumTop"].clone();
    });
    assert_refused(
        &error,
        ErrorCode::CaseInvalidValue,
        "marks[0].landmarks.sacrumTop.view",
    );
}

#[test]
fn quarter_turns_stop_at_three() {
    let error = refused(|case| case["films"][2]["geometry"]["quarterTurns"] = json!(4));
    assert_refused(
        &error,
        ErrorCode::CaseInvalidValue,
        "films[2].geometry.quarterTurns",
    );
}

#[test]
fn a_key_written_twice_is_refused() {
    let text = FOUR_FILMS.replacen(
        "\"nextVertebraId\": 8",
        "\"nextVertebraId\": 8,\n  \"nextVertebraId\": 999",
        1,
    );
    let error = read_case_json(&text).expect_err("the second spelling must not win");
    assert_refused(&error, ErrorCode::CaseInvalidValue, "nextVertebraId");

    let text = FOUR_FILMS.replacen(
        "\"format\": \"spine-case\"",
        "\"format\": \"not-a-case\",\n  \"format\": \"spine-case\"",
        1,
    );
    let error = read_case_json(&text).expect_err("the second spelling must not win");
    assert_refused(&error, ErrorCode::CaseInvalidValue, "format");

    let text = FOUR_FILMS.replacen(
        "\"kind\": \"front\",",
        "\"kind\": \"side\",\n\"kind\": \"front\",",
        1,
    );
    let error = read_case_json(&text).expect_err("repeated keys are refused at any depth");
    assert_refused(&error, ErrorCode::CaseInvalidValue, "films[0].kind");
}

#[test]
fn edits_may_leave_levels_beyond_the_template_when_flagged() {
    let flagged = |case: &mut Value| {
        case["marks"][1]["naming"]["flags"] = json!(["non-standard-count"]);
        case["marks"][1]["naming"]["fromBottom"] = Value::Null;
    };
    for level in ["L6", "L7"] {
        let mut value = four_films();
        flagged(&mut value);
        value["marks"][1]["vertebrae"][1]["level"] = json!(level);
        assert!(read(&value).is_ok(), "{level} after a split");
    }
    let mut value = four_films();
    flagged(&mut value);
    value["marks"][1]["vertebrae"][0]["level"] = json!("C6");
    value["marks"][1]["vertebrae"][1]["level"] = json!("C7");
    // The vertebra now named C7 brings its derived c7 with it.
    value["marks"][1]["landmarks"]["c7"] =
        json!({ "at": [724.25, 2687.5], "source": "auto", "pinned": false, "inferred": false });
    assert!(read(&value).is_ok(), "C6 after names shifted upward");

    // Shifting never invents a thirteenth thoracic vertebra.
    let error = refused(|case| {
        flagged(case);
        case["marks"][1]["vertebrae"][0]["level"] = json!("T13");
    });
    assert_refused(
        &error,
        ErrorCode::CaseInvalidValue,
        "marks[1].vertebrae[0].level",
    );
}

#[test]
fn both_counts_name_every_vertebra_from_the_template() {
    let error = refused(|case| case["marks"][1]["naming"]["fromBottom"] = json!(["L5"]));
    assert_refused(
        &error,
        ErrorCode::CaseInvalidValue,
        "marks[1].naming.fromBottom",
    );

    let error = refused(|case| case["marks"][1]["naming"]["fromTop"] = json!(["L4", "T13"]));
    assert_refused(
        &error,
        ErrorCode::CaseInvalidValue,
        "marks[1].naming.fromTop",
    );
}

#[test]
fn a_confirmed_l5_is_a_vertebra_named_l5_on_that_film() {
    let mut value = four_films();
    value["marks"][1]["naming"]["l5ConfirmedOn"] = json!(4);
    assert!(read(&value).is_ok(), "4 is the side film's L5");
    // 3 is the side film's L4; 5 is L5 on another film.
    for id in [3, 5] {
        let error = refused(|case| case["marks"][1]["naming"]["l5ConfirmedOn"] = json!(id));
        assert_refused(
            &error,
            ErrorCode::CaseInvalidValue,
            "marks[1].naming.l5ConfirmedOn",
        );
    }
}

#[test]
fn a_blanked_box_stays_on_its_film() {
    let error = refused(|case| case["films"][0]["blanked"][0]["left"] = json!(-0.6));
    assert_refused(&error, ErrorCode::CaseInvalidValue, "films[0].blanked[0]");
    let mut value = four_films();
    value["films"][0]["blanked"][0]["left"] = json!(-0.5);
    assert!(read(&value).is_ok());
}
