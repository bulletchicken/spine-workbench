//! The same independently written archives and verdicts run in native Rust
//! and through the built WebAssembly engine in proving-ground/parity.

// Fixture I/O belongs to this test only; the engine never touches disk.
#![allow(clippy::disallowed_methods)]

use case_file::{pack::pack_case, unpack::unpack_case};
use shapes::reading::write_case_json;
use std::collections::BTreeSet;
use std::path::Path;

#[test]
fn every_archive_has_its_independently_recorded_verdict() {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../proving-ground/known-answers/case-files");
    let verdicts = std::fs::read_to_string(root.join("archive-verdicts.tsv")).unwrap();
    let mut listed = BTreeSet::new();
    for line in verdicts.lines() {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 3);
        let (file, code, detail) = (fields[0], fields[1], fields[2]);
        assert!(listed.insert(file.to_owned()), "duplicate verdict: {file}");
        let bytes = std::fs::read(root.join(file)).unwrap();
        let opened = unpack_case(&bytes);
        if code == "accept" {
            let opened = opened.unwrap_or_else(|error| panic!("{file}: {error}"));
            let stem = Path::new(file).file_stem().unwrap().to_str().unwrap();
            let json = std::fs::read_to_string(root.join(format!("valid/{stem}.json"))).unwrap();
            assert_eq!(write_case_json(&opened.case), json, "{file}");
            // Independent PNG compression need not match ours. Once written
            // by the engine, both record and archive are byte-exact on resave.
            let packed = pack_case(&opened.case, &opened.films).unwrap();
            let saved = unpack_case(&packed).unwrap();
            assert_eq!(saved.case, opened.case);
            assert_eq!(pack_case(&saved.case, &saved.films).unwrap(), packed);
        } else {
            let error = opened.expect_err(file);
            assert_eq!(error.code.name(), code, "{file}: {error}");
            let expected = if detail.is_empty() {
                None
            } else {
                Some(detail)
            };
            assert_eq!(error.detail.as_deref(), expected, "{file}: {error}");
        }
    }
    let mut on_disk = BTreeSet::new();
    for folder in ["archives/valid", "archives/invalid"] {
        for entry in std::fs::read_dir(root.join(folder)).unwrap() {
            let name = entry.unwrap().file_name();
            on_disk.insert(format!("{folder}/{}", name.to_str().unwrap()));
        }
    }
    assert_eq!(on_disk, listed);
}
