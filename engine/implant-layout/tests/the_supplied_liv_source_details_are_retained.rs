//! Source metadata remains complete in the catalogue.

use implant_layout::evidence::{ReadStatus, tables};

#[test]
fn the_liv_meta_analysis_retains_its_supplied_pmid() {
    // Liu CW et al., Neurospine 2020, PMID 33401869.
    let source = tables()
        .source("liu-2020")
        .expect("the Liu source is present");
    assert_eq!(source.pmid.as_deref(), Some("33401869"));
}

#[test]
fn the_liv_meta_analysis_retains_its_supplied_authors_and_journal() {
    // The same passage explicitly supplies both the authors and journal.
    let source = tables()
        .source("liu-2020")
        .expect("the Liu source is present");
    // Liu 2020 has a complete bibliographic record.
    assert_eq!(
        source.citation.authors.as_deref(),
        Some("Liu CW, Lenke LG, Tan LA, Oh T, Chao KH, Lin SD, Pan RY")
    );
    assert_eq!(source.citation.journal.as_deref(), Some("Neurospine"));
    assert!(!source.notes.contains("journal, PMID and DOI not supplied"));
}

#[test]
fn joining_abstract_metadata_keeps_the_full_text_read_status() {
    // The meta-analysis was read in full; its abstract also supplies metadata.
    let source = tables()
        .source("liu-2020")
        .expect("the Liu source is present");
    assert_eq!(source.read, ReadStatus::FullText);
}
