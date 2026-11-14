use implant_layout::evidence::{Cell, Coordinate, PublishedValue, Table, Unit, tables};
use shapes::levels::Level;

fn summary(
    group: &str,
    level: Option<Level>,
    source: &str,
    value: PublishedValue,
) -> &'static Cell {
    let cell = tables()
        .lookup(
            Table::MedialAngles,
            &Coordinate {
                key: "transverse-angle-summary".to_string(),
                group: Some(group.to_string()),
                level,
                ..Coordinate::default()
            },
        )
        .expect("the exact summary coordinate is present");
    assert_eq!(cell.source, source);
    assert_eq!(cell.unit, Unit::Degrees);
    assert_eq!(cell.value, value);
    assert_eq!(
        cell.section,
        if source == "zheng-2009" {
            "not recorded (summary of the anatomic transverse angle, children)"
        } else {
            "not recorded (summary of the anatomic transverse angle, AIS)"
        }
    );
    cell
}

#[test]
fn the_child_t1_summary_remains_a_published_approximation() {
    // T1 ~31°, Zheng 2009, n=120. The exact male/female 30.7/31.4 values
    // are separate records, not inputs to this answer.
    let cell = summary(
        "zheng-p6-t1",
        Some(Level::T1),
        "zheng-2009",
        PublishedValue::Scalar { value: 31.0 },
    );
    assert!(
        cell.qualifier
            .as_deref()
            .expect("approximation")
            .contains("Approximately")
    );
}

#[test]
fn the_child_t2_summary_remains_a_published_approximation() {
    // Summary: T2 ~20°, without a sex or specific age-band allocation.
    let cell = summary(
        "zheng-p6-t2",
        Some(Level::T2),
        "zheng-2009",
        PublishedValue::Scalar { value: 20.0 },
    );
    assert!(
        cell.qualifier
            .as_deref()
            .expect("approximation")
            .contains("Approximately")
    );
}

#[test]
fn the_child_t3_summary_remains_a_published_approximation() {
    // Summary: T3 ~13°, separate from exact 13.2/12.6 columns.
    let cell = summary(
        "zheng-p6-t3",
        Some(Level::T3),
        "zheng-2009",
        PublishedValue::Scalar { value: 13.0 },
    );
    assert!(
        cell.qualifier
            .as_deref()
            .expect("approximation")
            .contains("Approximately")
    );
}

#[test]
fn the_child_mid_thoracic_summary_remains_a_pooled_range() {
    // Summary: T4–T10 ~4–8°, one regional range, not seven level cells.
    let cell = summary(
        "zheng-p6-t4-t10",
        None,
        "zheng-2009",
        PublishedValue::Range {
            lower: 4.0,
            upper: 8.0,
        },
    );
    let qualifier = cell.qualifier.as_deref().expect("regional approximation");
    assert!(qualifier.contains("Approximately"));
    assert!(qualifier.contains("T4–T10"));
    assert!(
        tables()
            .lookup(
                Table::MedialAngles,
                &Coordinate {
                    key: "transverse-angle-summary".to_string(),
                    group: Some("zheng-p6-t4-t10".to_string()),
                    level: Some(Level::T4),
                    ..Coordinate::default()
                }
            )
            .is_none()
    );
}

#[test]
fn the_child_divergent_summary_keeps_the_reported_endpoint_order() {
    // The summary gives T11–T12 ~−4 to −6°. The interval has ordered
    // mathematical bounds, while its qualifier retains the source's printed order.
    let cell = summary(
        "zheng-p6-t11-t12",
        None,
        "zheng-2009",
        PublishedValue::Range {
            lower: -6.0,
            upper: -4.0,
        },
    );
    let qualifier = cell.qualifier.as_deref().expect("signed approximation");
    assert!(qualifier.contains("approximately −4 to −6°"));
    assert!(qualifier.contains("T11–T12"));
}

#[test]
fn the_ais_summary_remains_the_reported_range() {
    // AIS summary: 12–26°, concave larger than convex, Upendra n=24.
    // The exact 12.4° value and the distinct 12° summary endpoint stay separate.
    let cell = summary(
        "upendra-p6-ais",
        None,
        "upendra-2010",
        PublishedValue::Range {
            lower: 12.0,
            upper: 26.0,
        },
    );
    assert!(
        cell.qualifier
            .as_deref()
            .expect("side condition")
            .contains("concave larger than convex")
    );
}
