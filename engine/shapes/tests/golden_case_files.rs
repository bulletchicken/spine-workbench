//! The hand-checked case files in proving-ground/known-answers/case-files
//! give the verdict `verdicts.json` records for each: valid files read and
//! are written back byte for byte unchanged; invalid files are refused
//! with the recorded error code and JSON path.

// Reading the fixture files is the point of this test; the engine itself
// never touches files.
#![allow(clippy::disallowed_methods)]

use serde::Deserialize;
use shapes::errors::ErrorCode;
use shapes::reading::{read_case_json, write_case_json};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Verdict {
    file: String,
    verdict: Outcome,
    code: Option<ErrorCode>,
    detail: Option<String>,
}

#[derive(Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "kebab-case")]
enum Outcome {
    Accept,
    Refuse,
}

fn case_files() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../proving-ground/known-answers/case-files")
}

fn verdicts() -> Vec<Verdict> {
    let text = std::fs::read_to_string(case_files().join("verdicts.json"))
        .expect("verdicts.json is readable");
    serde_json::from_str(&text).expect("verdicts.json is well formed")
}

#[test]
fn every_case_file_has_exactly_one_verdict() {
    let mut on_disk = BTreeSet::new();
    for folder in ["valid", "invalid"] {
        let entries = std::fs::read_dir(case_files().join(folder)).expect("the folder exists");
        for entry in entries {
            let name = entry.expect("a readable entry").file_name();
            on_disk.insert(format!("{folder}/{}", name.to_string_lossy()));
        }
    }
    let mut listed = BTreeSet::new();
    for verdict in verdicts() {
        assert!(
            listed.insert(verdict.file.clone()),
            "{} listed twice",
            verdict.file
        );
    }
    assert_eq!(listed, on_disk);
}

#[test]
fn valid_files_read_and_rewrite_unchanged() {
    for verdict in verdicts().iter().filter(|v| v.verdict == Outcome::Accept) {
        assert!(
            verdict.file.starts_with("valid/"),
            "{} is accepted",
            verdict.file
        );
        let text = std::fs::read_to_string(case_files().join(&verdict.file))
            .expect("the case file is readable");
        let case = read_case_json(&text)
            .unwrap_or_else(|error| panic!("{} was refused: {error}", verdict.file));
        assert_eq!(
            write_case_json(&case),
            text,
            "{} changed on rewrite",
            verdict.file
        );
    }
}

#[test]
fn invalid_files_are_refused_with_their_code_and_path() {
    for verdict in verdicts().iter().filter(|v| v.verdict == Outcome::Refuse) {
        assert!(
            verdict.file.starts_with("invalid/"),
            "{} is refused",
            verdict.file
        );
        let text = std::fs::read_to_string(case_files().join(&verdict.file))
            .expect("the case file is readable");
        let error =
            read_case_json(&text).expect_err(&format!("{} should have been refused", verdict.file));
        assert_eq!(Some(error.code), verdict.code, "{}: {error}", verdict.file);
        assert_eq!(error.detail, verdict.detail, "{}: {error}", verdict.file);
    }
}
