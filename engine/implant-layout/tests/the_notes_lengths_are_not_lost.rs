use implant_layout::evidence::{Cell, Coordinate, PublishedValue, Table, Unit, tables};
use shapes::levels::Level;

fn trajectory(
    group: &str,
    key: &str,
    level: Option<Level>,
    value: PublishedValue,
) -> &'static Cell {
    let cell = tables()
        .lookup(
            Table::Chords,
            &Coordinate {
                key: key.to_string(),
                group: Some(group.to_string()),
                level,
                ..Coordinate::default()
            },
        )
        .expect("the exact Lenke 5 trajectory coordinate is present");
    assert_eq!(cell.source, "pluemvitayaporn-2025");
    assert_eq!(cell.unit, Unit::Mm);
    assert_eq!(cell.value, value);
    assert!(cell.section.contains("Table 3"));
    cell
}

#[test]
fn the_lenke_five_t1_trajectory_range_is_preserved() {
    // screw-sizing-and-placement.md §1, paragraph after Table 3:
    // T1 28.4–28.6 mm, n=112; no allocation of the endpoints to sides.
    let cell = trajectory(
        "pluemvitayaporn-t1",
        "trajectory-length",
        Some(Level::T1),
        PublishedValue::Range {
            lower: 28.4,
            upper: 28.6,
        },
    );
    assert!(
        cell.qualifier
            .as_deref()
            .expect("side uncertainty")
            .contains("side allocation")
    );
}

#[test]
fn the_lenke_five_t12_trajectory_is_preserved() {
    // Same paragraph: about 45.5 mm at T12, not a side-specific value.
    let cell = trajectory(
        "pluemvitayaporn-t12",
        "trajectory-length",
        Some(Level::T12),
        PublishedValue::Scalar { value: 45.5 },
    );
    let qualifier = cell.qualifier.as_deref().expect("approximation");
    assert!(qualifier.contains("About 45.5"));
    assert!(qualifier.contains("side allocation"));
}

#[test]
fn the_lenke_five_lumbar_trajectory_range_is_preserved() {
    // Same paragraph and P4: one pooled L1–L5 46–51 mm range.
    let cell = trajectory(
        "pluemvitayaporn-l1-l5",
        "trajectory-length",
        None,
        PublishedValue::Range {
            lower: 46.0,
            upper: 51.0,
        },
    );
    assert!(cell.section.contains("P4"));
    assert!(
        cell.qualifier
            .as_deref()
            .expect("region")
            .contains("pooled L1–L5")
    );
    assert!(
        tables()
            .lookup(
                Table::Chords,
                &Coordinate {
                    key: "trajectory-length".to_string(),
                    group: Some("pluemvitayaporn-l1-l5".to_string()),
                    level: Some(Level::L1),
                    ..Coordinate::default()
                }
            )
            .is_none()
    );
}

#[test]
fn the_lenke_five_trajectory_side_difference_is_preserved() {
    // Same paragraph: concave is 2.1–3.2 mm longer than convex at L2–L4.
    // This is a regional difference, not an absolute trajectory length.
    let cell = trajectory(
        "pluemvitayaporn-l2-l4",
        "trajectory-side-difference",
        None,
        PublishedValue::Range {
            lower: 2.1,
            upper: 3.2,
        },
    );
    let qualifier = cell.qualifier.as_deref().expect("difference direction");
    assert!(qualifier.contains("L2–L4"));
    assert!(qualifier.contains("concave-minus-convex"));
}
