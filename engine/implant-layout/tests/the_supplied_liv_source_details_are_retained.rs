//! Source metadata supplied elsewhere in the research notes is not missing.

use implant_layout::evidence::{ReadStatus, tables};

#[test]
fn the_liv_meta_analysis_retains_its_supplied_pmid() {
    // outcomes-over-time.md §2, LIV choice meta-analysis (line 65):
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
    // citation-audit-placement-tables.md §A liu-2020 gives the complete record.
    assert_eq!(
        source.citation.authors.as_deref(),
        Some("Liu CW, Lenke LG, Tan LA, Oh T, Chao KH, Lin SD, Pan RY")
    );
    assert_eq!(source.citation.journal.as_deref(), Some("Neurospine"));
    assert!(!source.notes.contains("journal, PMID and DOI not supplied"));
}

#[test]
fn joining_abstract_metadata_keeps_the_full_text_read_status() {
    // fusion-levels.md §2c reports the full text of this meta-analysis;
    // outcomes-over-time.md §2 supplies extra metadata from its abstract.
    let source = tables()
        .source("liu-2020")
        .expect("the Liu source is present");
    assert_eq!(source.read, ReadStatus::FullText);
}
