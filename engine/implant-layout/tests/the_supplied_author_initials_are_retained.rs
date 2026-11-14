//! Citation details remain complete in the catalogue.

use implant_layout::evidence::{ReadStatus, tables};

#[test]
fn cidambis_supplied_author_initials_are_retained() {
    // Cidambi KR et al., Spine 2012, PMID 22426445.
    let source = tables()
        .source("cidambi-2012")
        .expect("the Cidambi source is present");
    assert_eq!(source.pmid.as_deref(), Some("22426445"));
    assert_eq!(source.read, ReadStatus::Abstract);
    // Cidambi 2012 has a complete author list.
    assert_eq!(
        source.citation.authors.as_deref(),
        Some("Cidambi KR, Glaser DA, Bastrom TP, Nunn TN, Ono T, Newton PO")
    );
    assert_eq!(
        source.citation.journal.as_deref(),
        Some("Spine (Phila Pa 1976)")
    );
    assert!(
        !source
            .notes
            .contains("Author initials and article title not supplied")
    );
}
