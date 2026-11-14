//! Published observations retain their coordinate, statistic and provenance.

use implant_layout::evidence::{Cell, Coordinate, PublishedValue, ReadStatus, Table, Unit, tables};

fn expect_observation(
    table: Table,
    key: &str,
    group: &str,
    source: &str,
    unit: Unit,
    value: PublishedValue,
    qualifications: &[&str],
) -> &'static Cell {
    let coordinate = Coordinate {
        key: key.to_string(),
        group: Some(group.to_string()),
        ..Coordinate::default()
    };
    let cell = tables()
        .lookup(table, &coordinate)
        .unwrap_or_else(|| panic!("missing {table:?}: {coordinate:?}"));
    assert_eq!(cell.source, source, "{coordinate:?}");
    assert_eq!(cell.unit, unit, "{coordinate:?}");
    assert_eq!(cell.value, value, "{coordinate:?}");
    if matches!(cell.value, PublishedValue::Median { .. }) {
        assert!(cell.section.contains("citation-audit-placement-tables.md"));
    }
    let qualifier = cell.qualifier.as_deref().expect("observation conditions");
    for qualification in qualifications {
        assert!(
            qualifier.contains(qualification),
            "{coordinate:?}: {qualification}: {qualifier}"
        );
    }
    cell
}

// The expected values below are copied by hand from implant-mechanics.md
// section 4, Contour loss after insertion table, rather than recomputed.
#[test]
fn cidambis_rod_deflection_is_retained() {
    expect_observation(
        Table::RodChoices,
        "deflection",
        "cidambi-concave",
        "cidambi-2012",
        Unit::Mm,
        PublishedValue::Scalar { value: -13.0 },
        &["Concave", "−13 mm", "n = 27"],
    );
}

#[test]
fn cidambis_before_kyphosis_is_retained() {
    expect_observation(
        Table::RodChoices,
        "before-kyphosis",
        "cidambi-t5-t12",
        "cidambi-2012",
        Unit::Degrees,
        PublishedValue::PlusMinus {
            value: 19.0,
            spread: 14.0,
        },
        &["Before", "T5–T12", "neither statistic", "n = 27"],
    );
}

#[test]
fn cidambis_after_kyphosis_is_retained() {
    expect_observation(
        Table::RodChoices,
        "after-kyphosis",
        "cidambi-t5-t12",
        "cidambi-2012",
        Unit::Degrees,
        PublishedValue::PlusMinus {
            value: 22.0,
            spread: 6.0,
        },
        &["After", "T5–T12", "neither statistic", "n = 27"],
    );
}

#[test]
fn salmingos_sagittal_correlation_is_retained() {
    expect_observation(
        Table::RodChoices,
        "curvature-correction-correlation",
        "salmingo-sagittal",
        "salmingo-2014",
        Unit::Correlation,
        PublishedValue::Scalar { value: 0.60 },
        &[
            "Correlation r",
            "rod curvature",
            "sagittal correction",
            "n = 20",
        ],
    );
}

#[test]
fn sias_rod_diameter_is_retained() {
    expect_observation(
        Table::RodChoices,
        "diameter",
        "sia-all",
        "sia-2019",
        Unit::Mm,
        PublishedValue::Scalar { value: 5.5 },
        &["n = 21", "titanium n = 10", "cobalt-chrome n = 11"],
    );
}

#[test]
fn sias_reported_p_value_does_not_invent_a_statistical_test() {
    // implant-mechanics.md section 4, Sia contour-loss row: p = 0.173.
    expect_observation(
        Table::RodChoices,
        "contour-loss-p-value",
        "sia-material-comparison",
        "sia-2019",
        Unit::Ratio,
        PublishedValue::Scalar { value: 0.173 },
        &["does not name the statistical test", "n = 21"],
    );
}

#[test]
fn sias_titanium_before_kyphosis_is_retained() {
    // citation-audit-placement-tables.md §A Sia 2019, Tables II–III; §B.9:
    // the audit names Median (IQR), but does not transcribe these four IQRs.
    expect_observation(
        Table::RodChoices,
        "before-kyphosis",
        "sia-titanium",
        "sia-2019",
        Unit::Degrees,
        PublishedValue::Median {
            value: 20.0,
            iqr: None,
        },
        &["Titanium before", "median", "subgroup n = 10"],
    );
}

#[test]
fn sias_titanium_after_kyphosis_is_retained() {
    // citation-audit-placement-tables.md §A Sia 2019, Tables II–III; §B.9.
    expect_observation(
        Table::RodChoices,
        "after-kyphosis",
        "sia-titanium",
        "sia-2019",
        Unit::Degrees,
        PublishedValue::Median {
            value: 26.0,
            iqr: None,
        },
        &["Titanium after", "median", "subgroup n = 10"],
    );
}

#[test]
fn sias_cobalt_chrome_before_kyphosis_is_retained() {
    // citation-audit-placement-tables.md §A Sia 2019, Tables II–III; §B.9.
    expect_observation(
        Table::RodChoices,
        "before-kyphosis",
        "sia-cobalt-chrome",
        "sia-2019",
        Unit::Degrees,
        PublishedValue::Median {
            value: 25.0,
            iqr: None,
        },
        &["Cobalt-chrome before", "median", "subgroup n = 11"],
    );
}

#[test]
fn sias_cobalt_chrome_after_kyphosis_is_retained() {
    // citation-audit-placement-tables.md §A Sia 2019, Tables II–III; §B.9.
    expect_observation(
        Table::RodChoices,
        "after-kyphosis",
        "sia-cobalt-chrome",
        "sia-2019",
        Unit::Degrees,
        PublishedValue::Median {
            value: 23.0,
            iqr: None,
        },
        &["Cobalt-chrome after", "median", "subgroup n = 11"],
    );
}

#[test]
fn boissieres_spine_to_rod_disparity_is_retained() {
    expect_observation(
        Table::RodChoices,
        "spine-rod-kyphosis-disparity",
        "boissiere-concave",
        "boissiere-2023",
        Unit::Degrees,
        PublishedValue::Scalar { value: 5.5 },
        &["Spine-to-rod kyphosis disparity", "n = 99"],
    );
    let source = tables().source("boissiere-2023").expect("Boissiere source");
    assert_eq!(source.pmid.as_deref(), Some("38255344"));
    assert_eq!(source.read, ReadStatus::Abstract);
}

#[test]
fn boissieres_less_kyphotic_rod_share_is_retained() {
    expect_observation(
        Table::RodChoices,
        "rod-less-kyphotic-share",
        "boissiere-concave",
        "boissiere-2023",
        Unit::Percent,
        PublishedValue::Scalar { value: 84.0 },
        &["Concave rod less kyphotic than the spine", "n = 99"],
    );
}

#[test]
fn the_medial_shoulder_imbalance_share_is_retained() {
    // fusion-levels.md section 3, UIV tilt drives T1 tilt row.
    expect_observation(
        Table::PlacementChecks,
        "medial-shoulder-imbalance",
        "uiv-tilt-cohort",
        "uiv-tilt-40020691",
        Unit::Percent,
        PublishedValue::Scalar { value: 27.8 },
        &["Medial shoulder", "n = 151", "Lenke 1–2"],
    );
}

#[test]
fn the_neck_imbalance_share_is_retained() {
    // Same UIV-tilt row: this is a distinct outcome, not shoulder imbalance.
    expect_observation(
        Table::PlacementChecks,
        "neck-imbalance",
        "uiv-tilt-cohort",
        "uiv-tilt-40020691",
        Unit::Percent,
        PublishedValue::Scalar { value: 23.2 },
        &["Neck imbalance", "n = 151", "Lenke 1–2"],
    );
}

#[test]
fn the_lateral_shoulder_imbalance_share_is_retained() {
    // Same UIV-tilt row: this is distinct from medial shoulder imbalance.
    expect_observation(
        Table::PlacementChecks,
        "lateral-shoulder-imbalance",
        "uiv-tilt-cohort",
        "uiv-tilt-40020691",
        Unit::Percent,
        PublishedValue::Scalar { value: 9.9 },
        &["Lateral shoulder", "n = 151", "Lenke 1–2"],
    );
}

#[test]
fn chens_younger_group_specificity_is_retained() {
    // fusion-levels.md section 2b, LIV-index row: subgroup n is absent.
    expect_observation(
        Table::PlacementChecks,
        "specificity",
        "liv-index-under-15",
        "chen-2025",
        Unit::Percent,
        PublishedValue::Scalar { value: 94.0 },
        &[
            "Under 15 years",
            "Subgroup n not supplied",
            "strictly less than",
        ],
    );
    let full = tables()
        .lookup(
            Table::PlacementChecks,
            &Coordinate {
                key: "specificity".to_string(),
                group: Some("liv-index".to_string()),
                ..Coordinate::default()
            },
        )
        .expect("full cohort specificity");
    assert_eq!(full.value, PublishedValue::Scalar { value: 92.0 });
}

#[test]
fn chens_younger_group_positive_value_is_retained() {
    // Same row, under 15 years PPV; full-cohort PPV remains 55.5%.
    expect_observation(
        Table::PlacementChecks,
        "positive-predictive-value",
        "liv-index-under-15",
        "chen-2025",
        Unit::Percent,
        PublishedValue::Scalar { value: 77.0 },
        &[
            "Under 15 years",
            "Subgroup n not supplied",
            "strictly less than",
        ],
    );
    let full = tables()
        .lookup(
            Table::PlacementChecks,
            &Coordinate {
                key: "positive-predictive-value".to_string(),
                group: Some("liv-index".to_string()),
                ..Coordinate::default()
            },
        )
        .expect("full cohort PPV");
    assert_eq!(full.value, PublishedValue::Scalar { value: 55.5 });
}

#[test]
fn chens_younger_group_age_boundary_is_retained() {
    // Same row: an exclusive upper age bound applies to both observations.
    expect_observation(
        Table::PlacementChecks,
        "age-upper-bound",
        "liv-index-under-15",
        "chen-2025",
        Unit::Years,
        PublishedValue::Scalar { value: 15.0 },
        &[
            "strictly less than 15 years",
            "specificity 94% and PPV 77%",
            "Subgroup n not supplied",
        ],
    );
}

#[test]
fn the_robot_channel_failure_range_is_retained() {
    // screw-sizing-and-placement.md P1, Hard floor: a mixed cited summary,
    // not the separate Akazawa 31.5% grade-4 observation.
    expect_observation(
        Table::DiameterRules,
        "hard-floor-failure",
        "p1-inner-under-one",
        "screw-p1-hard-floor",
        Unit::Percent,
        PublishedValue::Range {
            lower: 22.0,
            upper: 31.0,
        },
        &[
            "Approximate",
            "inner channel < 1 mm",
            "even with robot",
            "not a pooled cohort",
        ],
    );
}

#[test]
fn the_anterior_thread_limit_is_retained() {
    // screw-sizing-and-placement.md section 4 and P4 Rule: thread count
    // cannot be converted into mm without an unpublished thread pitch.
    expect_observation(
        Table::LengthRules,
        "anterior-protrusion-limit",
        "balikci-cortex",
        "balikci-2019",
        Unit::Count,
        PublishedValue::Scalar { value: 1.0 },
        &[
            "about 1 thread",
            "anterior cortical surface",
            "not millimetres",
        ],
    );
}

#[test]
fn the_upper_thoracic_measured_chord_range_is_retained() {
    // screw-sizing-and-placement.md P4, measured chord/trajectory column.
    expect_observation(
        Table::LengthRules,
        "measured-chord-trajectory",
        "p4-t1-t5",
        "screw-p4-summary",
        Unit::Mm,
        PublishedValue::Range {
            lower: 28.0,
            upper: 37.0,
        },
        &["T1–T5 regional", "Mixed cited series", "no pooled n"],
    );
}

#[test]
fn the_lower_thoracic_measured_chord_range_is_retained() {
    // Same P4 column: retain the region instead of assigning each level.
    expect_observation(
        Table::LengthRules,
        "measured-chord-trajectory",
        "p4-t6-t12",
        "screw-p4-summary",
        Unit::Mm,
        PublishedValue::Range {
            lower: 37.0,
            upper: 46.0,
        },
        &["T6–T12 regional", "Mixed cited series", "no pooled n"],
    );
}

#[test]
fn the_lumbar_measured_chord_range_is_retained() {
    // Same P4 range, also stated for Pluemvitayaporn after Table 3.
    expect_observation(
        Table::Chords,
        "trajectory-length",
        "pluemvitayaporn-l1-l5",
        "pluemvitayaporn-2025",
        Unit::Mm,
        PublishedValue::Range {
            lower: 46.0,
            upper: 51.0,
        },
        &["L1–L5"],
    );
}

#[test]
fn cuis_medial_wall_danger_depth_is_retained() {
    // screw-sizing-and-placement.md section 5 and P6: Cui 2012, n=30.
    expect_observation(
        Table::PlacementChecks,
        "medial-wall-danger-depth",
        "cui-probe",
        "cui-2012",
        Unit::Mm,
        PublishedValue::Range {
            lower: 8.0,
            upper: 10.0,
        },
        &["Probe-path depth", "along the probe path", "30 AIS CTs"],
    );
    let source = tables().source("cui-2012").expect("Cui source");
    assert_eq!(source.pmid.as_deref(), Some("21739245"));
    assert_eq!(source.read, ReadStatus::Abstract);
}

#[test]
fn the_published_upper_thoracic_entry_offset_is_retained() {
    // screw-sizing-and-placement.md section 5 and P6: Kim technique via
    // Etemadifar, about 2 mm lateral, with a specific landmark reference.
    expect_observation(
        Table::PlacementChecks,
        "entry-lateral-offset",
        "kim-t1-t10",
        "etemadifar-2017",
        Unit::Mm,
        PublishedValue::Scalar { value: 2.0 },
        &[
            "About 2 mm lateral",
            "midpoint of the superior facet",
            "superior border of the transverse process",
            "T1–T10",
        ],
    );
    let source = tables()
        .source("etemadifar-2017")
        .expect("Etemadifar source");
    assert_eq!(source.pmid.as_deref(), Some("28349022"));
    assert_eq!(source.read, ReadStatus::FullText);
}

#[test]
fn sias_rod_angles_and_patient_changes_are_medians_with_iqrs() {
    // Hand-typed from citation-audit-placement-tables.md §A Sia 2019,
    // Tables II and III; §B.9. Changes are per-patient median reductions.
    for (key, group, value, iqr) in [
        ("before-angle", "sia-titanium", 60.5, 10.4),
        ("after-angle", "sia-titanium", 37.0, 11.6),
        ("contour-loss", "sia-titanium", 22.5, 9.9),
        ("before-angle", "sia-cobalt-chrome", 51.0, 26.0),
        ("after-angle", "sia-cobalt-chrome", 28.0, 22.0),
        ("contour-loss", "sia-cobalt-chrome", 17.0, 7.7),
    ] {
        let cell = expect_observation(
            Table::RodChoices,
            key,
            group,
            "sia-2019",
            Unit::Degrees,
            PublishedValue::Median {
                value,
                iqr: Some(iqr),
            },
            &[
                "Median (IQR)",
                "concave rod",
                "medians of per-patient changes",
            ],
        );
        assert!(!cell.qualifier.as_deref().unwrap().contains("inconsistent"));
    }
}

#[test]
fn balikcis_standard_and_mid_length_catalogues_remain_separate() {
    // Hand-typed from full-tables-from-five-papers.md §A Balikci Table1.
    // The two printed lists are not combined or generated.
    for (group, value, category) in [
        ("balikci-standard-25", 25.0, "Standard length screws"),
        ("balikci-standard-30", 30.0, "Standard length screws"),
        ("balikci-standard-35", 35.0, "Standard length screws"),
        ("balikci-standard-40", 40.0, "Standard length screws"),
        ("balikci-standard-45", 45.0, "Standard length screws"),
        ("balikci-standard-50", 50.0, "Standard length screws"),
        ("balikci-standard-55", 55.0, "Standard length screws"),
        ("balikci-mid-27.5", 27.5, "Mid-length screws"),
        ("balikci-mid-32.5", 32.5, "Mid-length screws"),
        ("balikci-mid-37.5", 37.5, "Mid-length screws"),
        ("balikci-mid-42.5", 42.5, "Mid-length screws"),
        ("balikci-mid-47.5", 47.5, "Mid-length screws"),
        ("balikci-mid-52.5", 52.5, "Mid-length screws"),
    ] {
        let cell = expect_observation(
            Table::ScrewGrids,
            "length",
            group,
            "balikci-2019",
            Unit::Mm,
            PublishedValue::Scalar { value },
            &[category, "13–54", "AIS 86"],
        );
        assert!(
            cell.section.contains("citation-audit-placement-tables.md")
                || cell.section.contains("full-tables-from-five-papers.md")
        );
    }
}

#[test]
fn balikcis_availability_and_cited_lengths_keep_their_population_and_origin() {
    // Hand-typed from citation-audit-placement-tables.md §A Balikci 2019,
    // available screws, cited Krag range and regional lengths; §B.8.
    for (table, key, group, unit, value) in [
        (
            Table::ScrewGrids,
            "diameter",
            "balikci",
            Unit::Mm,
            PublishedValue::Range {
                lower: 5.0,
                upper: 8.0,
            },
        ),
        (
            Table::ScrewGrids,
            "diameter-step",
            "balikci",
            Unit::Mm,
            PublishedValue::Scalar { value: 1.0 },
        ),
        (
            Table::ScrewGrids,
            "length-step",
            "balikci",
            Unit::Mm,
            PublishedValue::Grid {
                values: vec![2.5, 5.0],
            },
        ),
        (
            Table::LengthRules,
            "chord-penetration",
            "krag-cited",
            Unit::Percent,
            PublishedValue::Range {
                lower: 80.0,
                upper: 100.0,
            },
        ),
        (
            Table::LengthRules,
            "anterior-protrusion-limit",
            "balikci-cortex",
            Unit::Count,
            PublishedValue::Scalar { value: 1.0 },
        ),
    ] {
        let cell = expect_observation(table, key, group, "balikci-2019", unit, value, &[]);
        assert!(
            cell.section.contains("citation-audit-placement-tables.md")
                || cell.section.contains("full-tables-from-five-papers.md")
        );
        // Cohort metadata belongs to the source, with cell-specific conditions
        // retained in the qualifier. Check the facts rather than an old phrase.
        let source = tables().source("balikci-2019").unwrap();
        let provenance = format!(
            "{} {} {} {}",
            cell.qualifier.as_deref().unwrap(),
            source.n.as_deref().unwrap(),
            source.population,
            source.notes
        );
        for fact in [
            "120",
            "13–54",
            "19.2",
            "AIS 86",
            "congenital 17",
            "adult 10",
            "neuromuscular 7",
        ] {
            assert!(provenance.contains(fact), "{fact}");
        }
        assert!(!cell.qualifier.as_deref().unwrap().contains("no 25–45"));
    }
    for (group, lower, upper) in [
        ("balikci-2019-t1-t5", 25.0, 30.0),
        ("balikci-2019-t6-t12", 30.0, 35.0),
        ("balikci-2019-l1-l5", 35.0, 40.0),
    ] {
        let cell = expect_observation(
            Table::LengthRules,
            "usual-length",
            group,
            "balikci-2019",
            Unit::Mm,
            PublishedValue::Range { lower, upper },
            &[
                "Secondary",
                "Kuraishi",
                "reference 25",
                "duplicate",
                "not an independent observation",
                "ages 13–54",
            ],
        );
        assert!(
            cell.section.contains("citation-audit-placement-tables.md")
                || cell.section.contains("full-tables-from-five-papers.md")
        );
    }
}

#[test]
fn grabalas_printed_expansions_retain_the_mixed_cohort_and_table_discrepancies() {
    // Hand-typed from citation-audit-placement-tables.md §A Grabala 2023,
    // Tables 1, 3 and 6 and discussion; §B.10. Printed figures are not repaired.
    for (key, group, value) in [
        (
            "expansion-by-level",
            "grabala",
            PublishedValue::Range {
                lower: 47.0,
                upper: 71.0,
            },
        ),
        (
            "expansion",
            "grabala-table-5.0",
            PublishedValue::Scalar { value: 32.0 },
        ),
        (
            "expansion",
            "grabala-table-5.5",
            PublishedValue::Scalar { value: 39.0 },
        ),
        (
            "expansion",
            "grabala-table-6.0",
            PublishedValue::Scalar { value: 49.0 },
        ),
        (
            "expansion",
            "grabala-table-6.5",
            PublishedValue::Scalar { value: 67.0 },
        ),
        (
            "expansion",
            "grabala-table-7.0",
            PublishedValue::Scalar { value: 78.0 },
        ),
        (
            "expansion",
            "grabala-text-5.0",
            PublishedValue::Scalar { value: 39.0 },
        ),
        (
            "expansion",
            "grabala-age-2-5",
            PublishedValue::Scalar { value: 49.0 },
        ),
        (
            "expansion",
            "grabala-age-6-10",
            PublishedValue::Scalar { value: 52.0 },
        ),
        (
            "expansion",
            "grabala-age-11-18",
            PublishedValue::Scalar { value: 62.0 },
        ),
    ] {
        let cell = expect_observation(
            Table::DiameterRules,
            key,
            group,
            "grabala-2023",
            Unit::Percent,
            value,
            &[
                "idiopathic 164",
                "congenital 25",
                "neuromuscular 23",
                "syndromic 18",
                "T5",
                "2.28",
                "2.55",
                "L3",
                "1.74",
                "2.89",
                "L5",
                "1.34",
                "2.68",
                "N = 7954",
                "N = 5080",
                "without repair",
            ],
        );
        assert!(cell.section.contains("citation-audit-placement-tables.md"));
    }
    let grid = expect_observation(
        Table::ScrewGrids,
        "diameter",
        "grabala",
        "grabala-2023",
        Unit::Mm,
        PublishedValue::Grid {
            values: vec![5.0, 5.5, 6.0, 6.5, 7.0],
        },
        &[
            "idiopathic 164",
            "congenital 25",
            "neuromuscular 23",
            "syndromic 18",
        ],
    );
    assert!(grid.section.contains("citation-audit-placement-tables.md"));
}

#[test]
fn abe_and_salmingo_are_not_independent_twenty_patient_observations() {
    // Hand-typed from citation-audit-placement-tables.md §A Abe 2015 and
    // Salmingo 2014 records; §B.10. The audit calls the overlap very probable.
    for (source, key, group, unit, value) in [
        ("abe-2015", "diameter", "ti6al7nb", Unit::Mm, 6.0),
        (
            "abe-2015",
            "before-angle",
            "abe-concave",
            Unit::Degrees,
            33.6,
        ),
        (
            "abe-2015",
            "after-angle",
            "abe-concave",
            Unit::Degrees,
            17.8,
        ),
        (
            "abe-2015",
            "before-angle",
            "abe-convex",
            Unit::Degrees,
            25.9,
        ),
        ("abe-2015", "after-angle", "abe-convex", Unit::Degrees, 23.8),
        (
            "salmingo-2014",
            "before-angle",
            "salmingo-concave",
            Unit::Degrees,
            33.6,
        ),
        (
            "salmingo-2014",
            "after-angle",
            "salmingo-concave",
            Unit::Degrees,
            17.8,
        ),
        (
            "salmingo-2014",
            "contour-loss",
            "salmingo-concave",
            Unit::Degrees,
            15.8,
        ),
        (
            "salmingo-2014",
            "before-angle",
            "salmingo-convex",
            Unit::Degrees,
            25.5,
        ),
        (
            "salmingo-2014",
            "after-angle",
            "salmingo-convex",
            Unit::Degrees,
            23.9,
        ),
        (
            "salmingo-2014",
            "contour-loss",
            "salmingo-convex",
            Unit::Degrees,
            1.6,
        ),
        (
            "salmingo-2014",
            "curvature-correction-correlation",
            "salmingo-sagittal",
            Unit::Correlation,
            0.60,
        ),
    ] {
        let cell = expect_observation(
            Table::RodChoices,
            key,
            group,
            source,
            unit,
            PublishedValue::Scalar { value },
            &["Very probably", "same 20-patient cohort", "not independent"],
        );
        assert!(cell.section.contains("citation-audit-placement-tables.md"));
    }
}

#[test]
fn rod_simulation_values_keep_their_model_conditions() {
    // citation-audit-placement-tables.md §A Wang 2016 and Borrelli 2025.
    let wang = expect_observation(
        Table::RodChoices,
        "diameter",
        "cobalt-chrome",
        "wang-2016",
        Unit::Mm,
        PublishedValue::Grid {
            values: vec![5.5, 6.0],
        },
        &[
            "5.5/5.5",
            "6.0/5.5",
            "10 AIS models",
            "only on the concave side",
        ],
    );
    assert!(wang.section.contains("citation-audit-placement-tables.md"));
    let borrelli = expect_observation(
        Table::RodChoices,
        "screw-cantilever-length",
        "lumbar",
        "borrelli-2025",
        Unit::Mm,
        PublishedValue::Scalar { value: 15.0 },
        &[
            "complete osseointegration",
            "CFR-PEEK",
            "6.5 mm",
            "model offset",
            "not a measured adolescent head offset",
        ],
    );
    assert!(
        borrelli
            .section
            .contains("citation-audit-placement-tables.md")
    );
}

#[test]
fn sarwahis_anterior_breach_values_keep_both_populations() {
    // Hand-typed from citation-audit-placement-tables.md §A Sarwahi 2017,
    // abstract values and recordDiffers population correction.
    for (key, value) in [
        (
            "anterior-breach-threshold",
            PublishedValue::Scalar { value: 4.0 },
        ),
        (
            "aorta-contact-depth",
            PublishedValue::PlusMinus {
                value: 5.7,
                spread: 0.6,
            },
        ),
        (
            "non-contact-depth",
            PublishedValue::PlusMinus {
                value: 3.9,
                spread: 0.2,
            },
        ),
    ] {
        let cell = expect_observation(
            Table::LengthRules,
            key,
            "aorta",
            "sarwahi-2017",
            Unit::Mm,
            value,
            &[
                "165 spine-deformity patients",
                "2800 screws",
                "8 cadavers",
                "285 screws",
                "not an exclusively AIS cohort",
            ],
        );
        assert!(cell.section.contains("citation-audit-placement-tables.md"));
    }
}
