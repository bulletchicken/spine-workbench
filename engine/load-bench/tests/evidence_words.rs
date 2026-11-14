use serde_json::Value;
use std::{fs, path::Path};

fn check_string(text: &str, path: &Path, needles: &[&str]) {
    let lower = text.to_lowercase();
    for needle in needles {
        assert!(
            !lower.contains(needle),
            "{}: {needle:?} in {text:?}",
            path.display()
        );
    }
    for needle in [
        concat!("DECI", "SIONS"),
        concat!("BRI", "EF"),
        concat!("BU", "ILD"),
        concat!("stand", "In"),
        concat!("Stand", "In"),
    ] {
        assert!(
            !text.contains(needle),
            "{}: {needle:?} in {text:?}",
            path.display()
        );
    }
}

fn check_value(value: &Value, path: &Path, needles: &[&str]) {
    match value {
        Value::String(text) => check_string(text, path, needles),
        Value::Array(values) => {
            for value in values {
                check_value(value, path, needles);
            }
        }
        Value::Object(fields) => {
            for (key, value) in fields {
                check_string(key, path, needles);
                check_value(value, path, needles);
            }
        }
        _ => {}
    }
}

#[test]
#[allow(
    clippy::disallowed_methods,
    reason = "This test reads every bundled evidence file from disk."
)]
fn evidence_strings_keep_publication_words() {
    let needles = [
        concat!(".", "md"),
        "\u{a7}",
        concat!("research", " note"),
        concat!("research", "-note"),
        concat!("citation", " audit"),
        concat!("citation", "-audit"),
        concat!("project", " notes"),
        concat!("original", " note"),
        concat!("the", " note"),
        concat!("note", "-supplied"),
        concat!("aud", "it"),
        concat!("synth", "esis"),
        concat!("citations-physics", "-extra"),
        concat!("citations-placement", "-extra"),
        concat!("full", "-tables-from-five", "-papers"),
        concat!("screw-sizing", "-and-placement"),
        concat!("implant", "-mechanics"),
        concat!("growth", "-and-loads"),
        concat!("published", "-simulators"),
        concat!("fast", "-physics"),
        concat!("fast", "-physics-methods"),
        concat!("outcomes", "-over-time"),
        concat!("fusion", "-levels"),
        concat!("existing-tools", "-and-gaps"),
        concat!("stand", "-in"),
    ];
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("evidence");
    let mut count = 0;
    for entry in fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            let value: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
            check_value(&value, &path, &needles);
            count += 1;
        }
    }
    assert!(
        count >= 6,
        "expected at least 6 evidence JSON files, read {count}"
    );
}
