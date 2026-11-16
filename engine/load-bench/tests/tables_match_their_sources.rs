//! Expected cells are typed by hand from the publications, independently of
//! the JSON reader. No fixture is generated from the bundled evidence.

use load_bench::evidence::{
    Basis, Citation, Evidence, Read, Source, Table, TableKind, Unit, Usage,
};
use serde_json::{Value as Json, json};
use std::collections::BTreeSet;

fn check(
    evidence: &Evidence,
    table: TableKind,
    row: &str,
    source: &str,
    unit: Unit,
    expected: &[(&str, f64)],
) {
    for &(term, expected) in expected {
        let cell = evidence
            .value(table, row, term)
            .expect("a transcribed cell");
        assert_eq!(cell.value, expected, "{table:?}/{row}/{term}");
        assert_eq!(cell.unit, unit, "{row}/{term}");
        assert_eq!(cell.source, source, "{row}/{term}");
    }
}

#[test]
fn joint_springs_keep_the_published_units_and_the_disagreeing_tables() {
    let e = Evidence::read().unwrap();
    // Every stiffness cell, including the facetectomy row.
    for (key, translations, moments) in [
        (
            "t1-2-to-l4-5-intact",
            [1392.0, 294.0, 341.0],
            [448.0, 644.0, 738.0],
        ),
        (
            "t1-2-to-l4-5-facetectomy",
            [696.0, 188.0, 188.0],
            [224.0, 406.0, 406.0],
        ),
        ("l5-s1", [700.0, 190.0, 190.0], [222.0, 410.0, 410.0]),
    ] {
        check(
            &e,
            TableKind::JointSprings,
            key,
            "tachi-2021",
            Unit::NPerMm,
            &[
                ("ap", translations[0]),
                ("ml", translations[1]),
                ("cc", translations[2]),
            ],
        );
        check(
            &e,
            TableKind::JointSprings,
            key,
            "tachi-2021",
            Unit::NmmPerDegree,
            &[
                ("moment-1", moments[0]),
                ("moment-2", moments[1]),
                ("moment-3", moments[2]),
            ],
        );
    }
    // Release: signed percentage reductions.
    check(
        &e,
        TableKind::JointSprings,
        "release",
        "wang-2017",
        Unit::Percent,
        &[
            ("axial-rotation", -17.0),
            ("flexion", -15.0),
            ("coronal", -3.8),
            ("compression", -14.0),
        ],
    );
    // Two discrete intact-segment preload results.
    check(
        &e,
        TableKind::JointSprings,
        "preload",
        "gardner-morse-2004",
        Unit::Ratio,
        &[("lower", 1.71), ("upper", 2.11)],
    );
    check(
        &e,
        TableKind::JointSprings,
        "preload",
        "gardner-morse-2004",
        Unit::N,
        &[("lower-preload", 250.0), ("upper-preload", 500.0)],
    );
    // Mannen raw EZS pairs: units are not printed in the paper.
    for (key, lb, fe, ar) in [
        ("rib-mannen-t1-t4", [6.85, 4.70], [2.29, 2.12], [3.28, 1.35]),
        ("rib-mannen-t4-t8", [8.30, 4.48], [4.74, 3.36], [4.51, 1.08]),
        (
            "rib-mannen-t8-t12",
            [2.87, 1.50],
            [2.97, 1.39],
            [2.31, 1.02],
        ),
    ] {
        check(
            &e,
            TableKind::JointSprings,
            key,
            "mannen-2018",
            Unit::Unstated,
            &[
                ("lb-with-ribs", lb[0]),
                ("lb-without-ribs", lb[1]),
                ("fe-with-ribs", fe[0]),
                ("fe-without-ribs", fe[1]),
                ("ar-with-ribs", ar[0]),
                ("ar-without-ribs", ar[1]),
            ],
        );
    }
    // Watkins percentages and the project sampling bounds.
    check(
        &e,
        TableKind::JointSprings,
        "rib-watkins",
        "watkins-2005",
        Unit::Percent,
        &[
            ("flexion-extension", 40.0),
            ("lateral-bending", 35.0),
            ("axial-rotation", 31.0),
        ],
    );
    check(
        &e,
        TableKind::JointSprings,
        "rib-lateral-bending",
        "project-assumptions",
        Unit::Ratio,
        &[("lower", 1.35), ("upper", 1.9)],
    );
    check(
        &e,
        TableKind::JointSprings,
        "rib-axial-rotation",
        "project-assumptions",
        Unit::Ratio,
        &[("lower", 1.3), ("upper", 4.0)],
    );
    // Physiological 642 N: comparison only, not Tachi's units.
    check(
        &e,
        TableKind::JointSprings,
        "lumbar-disc-642",
        "stokes-gardner-morse-2016",
        Unit::NmPerDegree,
        &[
            ("mx-rx", 2.8),
            ("mx-rx-sd", 2.2),
            ("mz-rz", 2.4),
            ("mz-rz-sd", 1.0),
        ],
    );
    // Wilke secants are inferred, with ROM kept beside them.
    check(
        &e,
        TableKind::JointSprings,
        "thoracic-secants",
        "project-assumptions",
        Unit::NmPerDegree,
        &[
            ("t1-t2-lb", 1.27),
            ("t1-t2-ar", 1.21),
            ("t8-t12-ar-upper", 2.27),
        ],
    );
    check(
        &e,
        TableKind::JointSprings,
        "thoracic-rom",
        "wilke-2017",
        Unit::Degrees,
        &[("t1-t2-lb", 5.9), ("t1-t2-ar", 6.2), ("t11-t12-ar", 3.3)],
    );
    // The stiffness scale is a calibration of this solver against published
    // head forces worked back from flexed rods, not a measurement of a
    // child's stiffness. The reference forces precede springback.
    check(
        &e,
        TableKind::JointSprings,
        "stiffness-scale",
        "project-assumptions",
        Unit::Ratio,
        &[("lower", 0.76), ("upper", 1.73)],
    );
    for term in ["lower", "upper"] {
        assert_eq!(
            e.value(TableKind::JointSprings, "stiffness-scale", term)
                .unwrap()
                .basis,
            Basis::OwnInference
        );
    }
    // Fitted factors have another baseline.
    check(
        &e,
        TableKind::JointSprings,
        "stiffness-scale",
        "petit-2004",
        Unit::Ratio,
        &[
            ("lumbar-fitted-lower", 1.38),
            ("lumbar-fitted-upper", 10.0),
            ("right-thoracic-fitted-lower", 1.74),
            ("right-thoracic-fitted-upper", 5.18),
        ],
    );
}

#[test]
fn rod_materials_keep_the_table_body_text_and_other_material_sets_separate() {
    let e = Evidence::read().unwrap();
    // Prasad's material table, all15cells.
    for (key, young, yl, yu, ul, uu) in [
        ("titanium-alloy", 114.0, 827.0, 1103.0, 860.0, 965.0),
        ("cobalt-chrome", 240.0, 500.0, 1500.0, 900.0, 1540.0),
        ("stainless-steel", 193.0, 170.0, 310.0, 540.0, 1000.0),
    ] {
        check(
            &e,
            TableKind::RodMaterials,
            key,
            "prasad-2017",
            Unit::Gpa,
            &[("young-modulus", young)],
        );
        check(
            &e,
            TableKind::RodMaterials,
            key,
            "prasad-2017",
            Unit::Mpa,
            &[
                ("yield-lower", yl),
                ("yield-upper", yu),
                ("uts-lower", ul),
                ("uts-upper", uu),
            ],
        );
        assert_eq!(
            e.row(TableKind::RodMaterials, key).unwrap().usage,
            if key == "titanium-alloy" {
                Usage::Beside
            } else {
                Usage::Bench
            }
        );
    }
    // Prasad also prints conflicting body-text values.
    check(
        &e,
        TableKind::RodMaterials,
        "prasad-body-text",
        "prasad-2017",
        Unit::Gpa,
        &[("ti-young-modulus", 115.0)],
    );
    check(
        &e,
        TableKind::RodMaterials,
        "prasad-body-text",
        "prasad-2017",
        Unit::Mpa,
        &[("cocr-yield-lower", 448.0), ("cocr-yield-upper", 1606.0)],
    );
    // Conflicting simulator material sets.
    check(
        &e,
        TableKind::RodMaterials,
        "tachi-cocr",
        "tachi-2021",
        Unit::Gpa,
        &[("young-modulus", 420.0)],
    );
    check(
        &e,
        TableKind::RodMaterials,
        "little-ti",
        "little-2013",
        Unit::Mpa,
        &[("yield", 390.0)],
    );
    // These Poisson ratios belong to these material sets only.
    check(
        &e,
        TableKind::RodMaterials,
        "jeong-ti-eli",
        "jeong-2025",
        Unit::Gpa,
        &[("young-modulus", 108.0)],
    );
    check(
        &e,
        TableKind::RodMaterials,
        "jeong-ti-eli",
        "jeong-2025",
        Unit::Ratio,
        &[("poisson-ratio", 0.342)],
    );
    check(
        &e,
        TableKind::RodMaterials,
        "song-ti",
        "song-2021",
        Unit::Gpa,
        &[("young-modulus", 113.0)],
    );
    check(
        &e,
        TableKind::RodMaterials,
        "song-ti",
        "song-2021",
        Unit::Ratio,
        &[("poisson-ratio", 0.30)],
    );
    // Textbook comparison, not the bench modulus.
    check(
        &e,
        TableKind::RodMaterials,
        "textbook-cocr",
        "ohrt-nissen-2018",
        Unit::Gpa,
        &[
            ("young-modulus-lower", 200.0),
            ("young-modulus-upper", 300.0),
        ],
    );
}

#[test]
fn anchor_strengths_preserve_se_population_and_bone_prior_inputs() {
    let e = Evidence::read().unwrap();
    // Trajectory: SE, not SD.
    check(
        &e,
        TableKind::AnchorStrength,
        "thoracic-anatomic",
        "lehman-2003",
        Unit::N,
        &[("mean", 481.0), ("se", 54.0)],
    );
    check(
        &e,
        TableKind::AnchorStrength,
        "thoracic-straight-forward",
        "lehman-2003",
        Unit::N,
        &[("mean", 611.0), ("se", 50.0)],
    );
    // Thoracic screws against hooks.
    for (row, expected) in [
        ("t4-t8-screw", 531.0),
        ("t9-t12-screw", 807.0),
        ("t4-t8-pedicle-hook", 321.0),
        ("laminar-hook", 600.0),
    ] {
        check(
            &e,
            TableKind::AnchorStrength,
            row,
            "liljenqvist-2001",
            Unit::N,
            &[("mean", expected)],
        );
    }
    // Soshi abstract supplies exact force precision and screw diameter.
    check(
        &e,
        TableKind::AnchorStrength,
        "lumbar-screw",
        "soshi-1991",
        Unit::N,
        &[
            ("normal", 1056.4),
            ("osteoporosis-grade-i", 495.6),
            ("osteoporosis-grade-ii", 269.5),
        ],
    );
    check(
        &e,
        TableKind::AnchorStrength,
        "lumbar-screw",
        "soshi-1991",
        Unit::Mm,
        &[("diameter", 7.0)],
    );
    check(
        &e,
        TableKind::AnchorStrength,
        "bone-density-multipliers",
        "project-assumptions",
        Unit::Ratio,
        &[("grade-i", 0.47), ("grade-ii", 0.26)],
    );
    // Exact prevalence behind the project mixture, not pull-out calibration.
    check(
        &e,
        TableKind::AnchorStrength,
        "ais-bone-prevalence",
        "cheng-2000",
        Unit::Percent,
        &[("low-abmd", 38.0), ("low-vbmd", 36.0)],
    );
    check(
        &e,
        TableKind::AnchorStrength,
        "ais-bone-prevalence",
        "yip-2016",
        Unit::Percent,
        &[("osteopenia", 32.9)],
    );
    // Young bone: separate populations, no combined animal range.
    check(
        &e,
        TableKind::AnchorStrength,
        "immature-pig",
        "le-cann-2015",
        Unit::N,
        &[("lower", 420.0), ("upper", 1341.0)],
    );
    check(
        &e,
        TableKind::AnchorStrength,
        "six-week-pig",
        "yilmaz-2009",
        Unit::N,
        &[
            ("mean", 397.0),
            ("sd", 80.0),
            ("dilated-mean", 379.0),
            ("dilated-sd", 73.0),
        ],
    );
}

#[test]
fn loads_preserve_literal_activity_edges_and_both_walking_sources() {
    let e = Evidence::read().unwrap();
    // Fractions confirmed as printed in Keenan; Winter itself unopened.
    check(
        &e,
        TableKind::DailyLoads,
        "head-neck",
        "winter-via-keenan-2015",
        Unit::BodyWeightFraction,
        &[("fraction", 0.081)],
    );
    check(
        &e,
        TableKind::DailyLoads,
        "each-arm-hand",
        "winter-via-keenan-2015",
        Unit::BodyWeightFraction,
        &[("fraction", 0.056)],
    );
    check(
        &e,
        TableKind::DailyLoads,
        "pearsall-slices",
        "pearsall-1996",
        Unit::Percent,
        &[("t1", 1.1), ("l5", 2.6)],
    );
    check(
        &e,
        TableKind::DailyLoads,
        "keenan-slices",
        "keenan-2014",
        Unit::Kg,
        &[("t1", 0.6), ("l5", 1.5), ("body-mean", 58.2)],
    );
    check(
        &e,
        TableKind::DailyLoads,
        "slice-offset",
        "project-assumptions",
        Unit::Mm,
        &[("mean", 28.0), ("sd", 5.6)],
    );
    // Different implant populations at each telemetry endpoint.
    check(
        &e,
        TableKind::DailyLoads,
        "walking",
        "rohlmann-2002",
        Unit::Ratio,
        &[("lower", 1.28)],
    );
    check(
        &e,
        TableKind::DailyLoads,
        "walking",
        "rohlmann-2014-gait",
        Unit::Ratio,
        &[("upper", 1.71)],
    );
    // One adult's pressure ratio, inferred load multiple.
    check(
        &e,
        TableKind::DailyLoads,
        "bending-forward",
        "wilke-1999",
        Unit::Mpa,
        &[("standing-flexed", 1.1), ("standing-relaxed", 0.5)],
    );
    check(
        &e,
        TableKind::DailyLoads,
        "bending-forward",
        "project-assumptions",
        Unit::Ratio,
        &[("multiple", 2.2)],
    );
    check(
        &e,
        TableKind::DailyLoads,
        "weight-if-absent",
        "schmid-2020-ais",
        Unit::Kg,
        &[("lower", 29.0), ("upper", 63.0)],
    );
    // Adult activity bounds: use 7499/9999/12499, not the rounded project bands.
    for (key, low, high) in [
        ("steps-low", 5000.0, 7499.0),
        ("steps-typical", 7500.0, 9999.0),
        ("steps-active", 10000.0, 12499.0),
    ] {
        check(
            &e,
            TableKind::DailyLoads,
            key,
            "tudor-locke-2008",
            Unit::StepsPerDay,
            &[("lower-inclusive", low), ("upper-inclusive", high)],
        );
    }
    check(
        &e,
        TableKind::DailyLoads,
        "steps-very-low",
        "tudor-locke-2008",
        Unit::StepsPerDay,
        &[("upper-exclusive", 5000.0)],
    );
    check(
        &e,
        TableKind::DailyLoads,
        "steps-very-active",
        "tudor-locke-2008",
        Unit::StepsPerDay,
        &[("lower-inclusive", 12500.0)],
    );
    check(
        &e,
        TableKind::DailyLoads,
        "rounded-steps-low",
        "project-assumptions",
        Unit::StepsPerDay,
        &[("upper", 7500.0)],
    );
    check(
        &e,
        TableKind::DailyLoads,
        "steps-if-absent",
        "project-assumptions",
        Unit::StepsPerDay,
        &[("lower", 8000.0), ("upper", 16000.0)],
    );
}

#[test]
fn fatigue_keeps_the_published_inputs_beside_the_project_calculation() {
    let e = Evidence::read().unwrap();
    // Construct tests and derived calibration.
    for (key, load, max, amplitude, mean, equivalent, uts) in [
        ("titanium-alloy", 250.0, 306.0, 138.0, 168.0, 169.0, 900.0),
        ("cobalt-chrome", 290.0, 355.0, 160.0, 195.0, 191.0, 1200.0),
    ] {
        check(
            &e,
            TableKind::Fatigue,
            key,
            "vieweg-2023",
            Unit::N,
            &[("run-out-load", load)],
        );
        check(
            &e,
            TableKind::Fatigue,
            key,
            "vieweg-2023",
            Unit::Mm,
            &[
                ("rod-diameter", 5.5),
                ("active-length", 76.0),
                ("screw-diameter", 4.5),
                ("screw-length", 45.0),
            ],
        );
        check(
            &e,
            TableKind::Fatigue,
            key,
            "vieweg-2023",
            Unit::Cycles,
            &[("run-out-cycles", 5_000_000.0)],
        );
        check(
            &e,
            TableKind::Fatigue,
            key,
            "vieweg-2023",
            Unit::Ratio,
            &[("printed-r", 10.0)],
        );
        check(
            &e,
            TableKind::Fatigue,
            key,
            "project-assumptions",
            Unit::Mm,
            &[("assumed-arm", 40.0)],
        );
        check(
            &e,
            TableKind::Fatigue,
            key,
            "project-assumptions",
            Unit::Mpa,
            &[
                ("maximum-stress", max),
                ("amplitude", amplitude),
                ("mean", mean),
                ("goodman-equivalent", equivalent),
                ("goodman-uts", uts),
            ],
        );
        assert_eq!(
            e.value(TableKind::Fatigue, key, "goodman-equivalent")
                .unwrap()
                .basis,
            Basis::OwnInference
        );
        assert_eq!(
            e.value(TableKind::Fatigue, key, "run-out-load")
                .unwrap()
                .basis,
            Basis::Published
        );
    }
    // Slivka's 20–40% drop and the converted 0.6–0.8 factor.
    check(
        &e,
        TableKind::Fatigue,
        "contour-knock-down",
        "slivka-2013",
        Unit::Percent,
        &[("reduction-lower", 20.0), ("reduction-upper", 40.0)],
    );
    check(
        &e,
        TableKind::Fatigue,
        "contour-knock-down",
        "project-assumptions",
        Unit::Ratio,
        &[("lower", 0.6), ("upper", 0.8)],
    );
    // Textbook bounds are not a construct level.
    check(
        &e,
        TableKind::Fatigue,
        "textbook-titanium",
        "ohrt-nissen-2018",
        Unit::Mpa,
        &[("lower", 300.0), ("upper", 389.0)],
    );
    check(
        &e,
        TableKind::Fatigue,
        "smooth-ti-coupon",
        "carrion-2016",
        Unit::Reversals,
        &[("reversals-more-than", 2_713_156.0)],
    );
}

#[test]
fn missing_values_return_none_without_fallback_or_interpolation() {
    let e = Evidence::read().unwrap();
    for (table, row, term) in [
        (TableKind::Fatigue, "stainless-steel", "goodman-equivalent"),
        (TableKind::RodMaterials, "titanium-alloy", "poisson-ratio"),
        (TableKind::RodMaterials, "cobalt-chrome", "poisson-ratio"),
        (TableKind::RodMaterials, "stainless-steel", "poisson-ratio"),
        (TableKind::RodMaterials, "titanium-alloy", "shear-modulus"),
        (TableKind::RodMaterials, "cobalt-chrome", "shear-modulus"),
        (TableKind::RodMaterials, "stainless-steel", "shear-modulus"),
        (TableKind::JointSprings, "rib-flexion-range", "lower"),
        (TableKind::DailyLoads, "trunk-slice-endpoints", "t2"),
        (TableKind::DailyLoads, "steps-very-low", "lower-inclusive"),
        (
            TableKind::DailyLoads,
            "steps-very-active",
            "upper-inclusive",
        ),
        (TableKind::AnchorStrength, "lumbar-screw", "length"),
        (TableKind::AnchorStrength, "thoracic-anatomic", "sd"),
    ] {
        assert!(e.value(table, row, term).is_none(), "{row}/{term}");
    }
    assert!(e.row(TableKind::RodMaterials, "Titanium-alloy").is_none());
    assert!(e.source("unrecorded").is_none());
}

#[test]
fn every_key_resolves_and_every_source_keeps_its_access_and_sample() {
    let e = Evidence::read().unwrap();
    let mut source_keys = BTreeSet::new();
    for source in e.sources() {
        assert!(source_keys.insert(&source.key), "duplicate source");
        assert!(!source.citation.authors.is_empty());
        assert!(!source.population.is_empty());
        if let Some(pmid) = &source.pmid {
            assert!(!pmid.is_empty() && pmid.bytes().all(|b| b.is_ascii_digit()));
        }
    }
    let mut used = BTreeSet::new();
    for kind in TableKind::ALL {
        let mut rows = BTreeSet::new();
        for row in &e.table(kind).rows {
            assert!(rows.insert(&row.key), "duplicate row");
            assert!(!row.values.is_empty(), "empty row: {}", row.key);
            let mut terms = BTreeSet::new();
            for cell in &row.values {
                assert!(terms.insert(&cell.term), "duplicate term");
                assert!(cell.value.is_finite());
                assert!(!cell.section.is_empty());
                assert!(e.source(&cell.source).is_some(), "{}", cell.source);
                used.insert(&cell.source);
            }
        }
    }
    assert_eq!(used, source_keys);
    // Recorded reading status is not implied by a PMC link.
    let tachi = e.source("tachi-2021").unwrap();
    assert_eq!(tachi.read, Read::FullText);
    assert_eq!(tachi.n.as_deref(), Some("47"));
    assert_eq!(tachi.pmid.as_deref(), Some("34712654"));
    assert_eq!(tachi.doi.as_deref(), Some("10.3389/fbioe.2021.746902"));
    assert_eq!(e.source("rohlmann-2014-gait").unwrap().read, Read::Abstract);
    assert_eq!(
        e.source("winter-via-keenan-2015").unwrap().read,
        Read::FullText
    );
    assert_eq!(
        e.source("lehman-2003").unwrap().pmid.as_deref(),
        Some("14501914")
    );
    assert_eq!(
        e.source("petit-2004").unwrap().doi.as_deref(),
        Some("10.1007/bf02351011")
    );
    assert!(e.source("gardner-morse-2004").unwrap().n.is_none());
    assert_eq!(
        e.source("prasad-2017").unwrap().citation.title.as_deref(),
        Some("Metallic Biomaterials: Current Challenges and Opportunities.")
    );
}

fn add_unknown(value: &mut Json, path: &str) {
    value
        .pointer_mut(path)
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert("extra".into(), json!(1));
}

#[test]
fn unknown_fields_are_refused_at_every_evidence_object() {
    let table: Json = serde_json::from_str(include_str!("../evidence/joint-springs.json")).unwrap();
    for path in ["", "/rows/0", "/rows/0/values/0", "/missing/0"] {
        let mut changed = table.clone();
        add_unknown(&mut changed, path);
        assert!(serde_json::from_value::<Table>(changed).is_err(), "{path}");
    }
    let sources: Json = serde_json::from_str(include_str!("../evidence/sources.json")).unwrap();
    for path in ["", "/citation"] {
        let mut changed = sources["sources"][1].clone();
        add_unknown(&mut changed, path);
        assert!(serde_json::from_value::<Source>(changed).is_err(), "{path}");
    }
    let mut citation = sources["sources"][1]["citation"].clone();
    add_unknown(&mut citation, "");
    assert!(serde_json::from_value::<Citation>(citation).is_err());
    let mut wrong_unit = table.clone();
    wrong_unit["rows"][0]["values"][0]["unit"] = json!("invented-unit");
    assert!(serde_json::from_value::<Table>(wrong_unit).is_err());
    let mut wrong_basis = table;
    wrong_basis["rows"][0]["values"][0]["basis"] = json!("invented-basis");
    assert!(serde_json::from_value::<Table>(wrong_basis).is_err());
}

#[test]
fn published_paper_records_have_titles_identifiers_and_complete_locations() {
    let e = Evidence::read().unwrap();
    // Publication identifiers.
    // Liljenqvist has no DOI in the supplied record; all records have a PMID.
    for (key, pmid, doi) in [
        ("tachi-2021", "34712654", Some("10.3389/fbioe.2021.746902")),
        ("wang-2017", "28428982", Some("10.1186/s13013-017-0120-4")),
        (
            "gardner-morse-2004",
            "14706323",
            Some("10.1016/j.jbiomech.2003.10.003"),
        ),
        (
            "stokes-gardner-morse-2016",
            "26900035",
            Some("10.1016/j.jbiomech.2016.01.035"),
        ),
        (
            "wilke-2017",
            "28520819",
            Some("10.1371/journal.pone.0177823"),
        ),
        (
            "watkins-2005",
            "15928553",
            Some("10.1097/01.brs.0000164257.69354.bb"),
        ),
        (
            "mannen-2018",
            "29852313",
            Some("10.1016/j.jmbbm.2018.05.019"),
        ),
        ("liebsch-2020", "32117927", Some("10.3389/fbioe.2020.00046")),
        ("petit-2004", "14977223", Some("10.1007/bf02351011")),
        ("prasad-2017", "28773240", Some("10.3390/ma10080884")),
        ("little-2013", "23680391", Some("10.1186/1748-7161-8-9")),
        (
            "ohrt-nissen-2018",
            "29991241",
            Some("10.14245/ns.1836050.025"),
        ),
        ("jeong-2025", "41383715", Some("10.3389/fbioe.2025.1673061")),
        ("song-2021", "34155224", Some("10.1038/s41598-021-90686-6")),
        (
            "lehman-2003",
            "14501914",
            Some("10.1097/01.brs.0000087743.57439.4f"),
        ),
        ("liljenqvist-2001", "11383294", None),
        (
            "soshi-1991",
            "1750009",
            Some("10.1097/00007632-199111000-00015"),
        ),
        (
            "le-cann-2015",
            "26451947",
            Some("10.1371/journal.pone.0127463"),
        ),
        (
            "yilmaz-2009",
            "19829250",
            Some("10.1097/brs.0b013e3181b0bb9b"),
        ),
        (
            "cheng-2000",
            "10934658",
            Some("10.1359/jbmr.2000.15.8.1587"),
        ),
        ("yip-2016", "27991528", Some("10.1038/srep39220")),
        (
            "winter-via-keenan-2015",
            "26681978",
            Some("10.1186/s13013-015-0060-9"),
        ),
        ("pearsall-1996", "8678352", Some("10.1007/bf02667349")),
        (
            "keenan-2014",
            "24973826",
            Some("10.1016/j.clinbiomech.2014.06.002"),
        ),
        (
            "gangnet-2003",
            "13680185",
            Some("10.1007/s00276-003-0154-6"),
        ),
        ("rohlmann-2002", "11784277", Some("10.1093/ptj/82.1.44")),
        (
            "rohlmann-2014-gait",
            "24211089",
            Some("10.1016/j.gaitpost.2013.10.010"),
        ),
        (
            "wilke-1999",
            "10222525",
            Some("10.1097/00007632-199904150-00005"),
        ),
        (
            "schmid-2020-ais",
            "32195239",
            Some("10.3389/fbioe.2020.00159"),
        ),
        (
            "tudor-locke-2008",
            "18562971",
            Some("10.1249/mss.0b013e31817c7133"),
        ),
        (
            "tudor-locke-2011",
            "21798014",
            Some("10.1186/1479-5868-8-78"),
        ),
        ("vieweg-2023", "36685708", Some("10.1016/j.bas.2022.101708")),
        (
            "slivka-2013",
            "27927364",
            Some("10.1016/j.jspd.2013.08.004"),
        ),
        (
            "carrion-2016",
            "26952022",
            Some("10.1016/j.dib.2016.02.014"),
        ),
    ] {
        let s = e.source(key).unwrap();
        assert_eq!(s.pmid.as_deref(), Some(pmid), "{key}");
        assert_eq!(s.doi.as_deref(), doi, "{key}");
        assert_eq!(
            s.record_note.as_deref(),
            Some("bibliographic record and read status")
        );
        assert!(
            s.citation.title.as_deref().is_some_and(|v| !v.is_empty()),
            "{key}"
        );
        assert!(
            s.citation.journal.is_some()
                && s.citation.volume.is_some()
                && s.citation.pages.is_some(),
            "{key}"
        );
    }
    // Alias is the same identified Keenan paper, not another primary source.
    assert_eq!(
        e.source("keenan-2015").unwrap().citation,
        e.source("winter-via-keenan-2015").unwrap().citation
    );
    for s in e.sources().iter().filter(|s| s.record_note.is_some()) {
        assert!(s.citation.title.is_some(), "identified paper: {}", s.key);
        assert!(s.pmid.is_some() || s.doi.is_some(), "{}", s.key);
    }
    // Record locations (volume, issue, pages) and corrected first-author-only entries.
    let liljenqvist = e.source("liljenqvist-2001").unwrap();
    assert_eq!(
        liljenqvist.citation.authors,
        "Liljenqvist U, Hackenberg L, Link T, Halm H"
    );
    assert_eq!(liljenqvist.citation.volume.as_deref(), Some("67"));
    assert_eq!(liljenqvist.citation.issue.as_deref(), Some("2"));
    assert_eq!(liljenqvist.citation.pages.as_deref(), Some("157-163"));
    assert_eq!(
        e.source("cheng-2000").unwrap().citation.authors,
        "Cheng JC, Qin L, Cheung CS, Sher AH, Lee KM, Ng SW, Guo X"
    );
    assert_eq!(
        e.source("gangnet-2003").unwrap().citation.authors,
        "Gangnet N, Pomero V, Dumas R, Skalli W, Vital JM"
    );
    assert_eq!(
        e.source("tudor-locke-2008").unwrap().citation.authors,
        "Tudor-Locke C, Hatano Y, Pangrazi RP, Kang M"
    );
    assert_eq!(
        e.source("stokes-gardner-morse-2016")
            .unwrap()
            .citation
            .authors,
        "Stokes IAF, Gardner-Morse M"
    );
    assert_eq!(e.source("le-cann-2015").unwrap().read, Read::FullText);
    assert_eq!(e.source("tudor-locke-2011").unwrap().read, Read::FullText);
    assert_eq!(
        e.source("liebsch-2020").unwrap().pmcid.as_deref(),
        Some("PMC7018667")
    );
}

#[test]
fn the_evidence_separates_contradictory_material_sets_and_selects_paired_strengths() {
    use shapes::implant_layout::RodMaterial;
    let e = Evidence::read().unwrap();
    // Prasad's body-text strengths, kept apart from the Table 1 sets.
    check(
        &e,
        TableKind::RodMaterials,
        "prasad-body-text",
        "prasad-2017",
        Unit::Mpa,
        &[
            ("ti-yield-lower", 896.0),
            ("ti-yield-upper", 1034.0),
            ("ti-uts-lower", 965.0),
            ("ti-uts-upper", 1103.0),
            ("cocr-uts-lower", 655.0),
            ("cocr-uts-upper", 1896.0),
        ],
    );
    check(
        &e,
        TableKind::RodMaterials,
        "titanium-alloy-body-text",
        "prasad-2017",
        Unit::Mpa,
        &[
            ("yield-lower", 896.0),
            ("yield-upper", 1034.0),
            ("uts-lower", 965.0),
            ("uts-upper", 1103.0),
        ],
    );
    // The internally inconsistent Table 1 pair stays beside the chosen pair.
    let raw = e.row(TableKind::RodMaterials, "titanium-alloy").unwrap();
    assert_eq!(raw.usage, Usage::Beside);
    assert!(
        e.value(TableKind::RodMaterials, "titanium-alloy", "yield-upper")
            .unwrap()
            .value
            > e.value(TableKind::RodMaterials, "titanium-alloy", "uts-upper")
                .unwrap()
                .value
    );
    assert!(
        !e.row(TableKind::RodMaterials, "stainless-steel")
            .unwrap()
            .description
            .to_lowercase()
            .contains("anneal")
    );
    // Endpoint differences bound the affine UTS(q)-yield(q) for every
    // shared q in[0,1]. Independent q is forbidden.
    for (material, y0, y1, u0, u1, margin0, margin1) in [
        (
            RodMaterial::TitaniumAlloy,
            896.0,
            1034.0,
            965.0,
            1103.0,
            69.0,
            69.0,
        ),
        (
            RodMaterial::CobaltChrome,
            500.0,
            1500.0,
            900.0,
            1540.0,
            400.0,
            40.0,
        ),
        (
            RodMaterial::StainlessSteel,
            170.0,
            310.0,
            540.0,
            1000.0,
            370.0,
            690.0,
        ),
    ] {
        let pair = e.bench_strengths(material).unwrap();
        for (cell, expected) in [
            (pair.yield_lower, y0),
            (pair.yield_upper, y1),
            (pair.ultimate_lower, u0),
            (pair.ultimate_upper, u1),
        ] {
            assert_eq!(cell.value, expected);
            assert_eq!(cell.unit, Unit::Mpa);
            assert_eq!(cell.source, "prasad-2017");
            assert_eq!(
                cell.section,
                if material == RodMaterial::TitaniumAlloy {
                    "body text"
                } else {
                    "Table 1"
                }
            );
        }
        assert_eq!(pair.ultimate_lower.value - pair.yield_lower.value, margin0);
        assert_eq!(pair.ultimate_upper.value - pair.yield_upper.value, margin1);
        assert!(margin0 > 0.0 && margin1 > 0.0);
    }
}

#[test]
fn the_evidence_corrects_the_thoracic_span_and_its_secant_arithmetic() {
    let e = Evidence::read().unwrap();
    // Narrow the stated span, retaining T6-T7 separately.
    // No5.2–5.7 range may claim to cover it.
    check(
        &e,
        TableKind::JointSprings,
        "thoracic-rom",
        "wilke-2017",
        Unit::Degrees,
        &[
            ("t2-t6-ar-lower", 5.2),
            ("t2-t6-ar-upper", 5.7),
            ("t6-t7-ar", 5.9),
        ],
    );
    for row in ["thoracic-rom", "thoracic-secants"] {
        assert!(
            e.value(TableKind::JointSprings, row, "t2-t7-ar-lower")
                .is_none()
        );
        assert!(
            e.value(TableKind::JointSprings, row, "t2-t7-ar-upper")
                .is_none()
        );
    }
    // Own secants:7.5N·m /5.7deg and7.5N·m /5.2deg, recomputed
    // from the published inputs, without copying the old rounded results.
    for (term, expected) in [("t2-t6-ar-lower", 7.5 / 5.7), ("t2-t6-ar-upper", 7.5 / 5.2)] {
        let c = e
            .value(TableKind::JointSprings, "thoracic-secants", term)
            .unwrap();
        assert_eq!(c.value, expected);
        assert_eq!(c.unit, Unit::NmPerDegree);
        assert_eq!(c.basis, Basis::OwnInference);
        assert_eq!(
            c.section,
            "worked out from the Wilke ROM endpoints (own inference: applied moment over ROM)"
        );
    }
}

#[test]
fn the_evidence_keeps_exact_soshi_forces_beside_own_rounded_ratios() {
    let e = Evidence::read().unwrap();
    // Soshi abstract precision.
    check(
        &e,
        TableKind::AnchorStrength,
        "lumbar-screw",
        "soshi-1991",
        Unit::N,
        &[
            ("normal", 1056.4),
            ("osteoporosis-grade-i", 495.6),
            ("osteoporosis-grade-ii", 269.5),
        ],
    );
    check(
        &e,
        TableKind::AnchorStrength,
        "bone-density-multipliers",
        "soshi-1991",
        Unit::N,
        &[
            ("normal-force", 1056.4),
            ("grade-i-force", 495.6),
            ("grade-ii-force", 269.5),
        ],
    );
    for (term, rounded, numerator) in [("grade-i", 0.47, 495.6), ("grade-ii", 0.26, 269.5)] {
        let c = e
            .value(TableKind::AnchorStrength, "bone-density-multipliers", term)
            .unwrap();
        assert_eq!(c.value, rounded);
        assert_eq!(c.basis, Basis::OwnInference);
        assert_eq!(c.source, "project-assumptions");
        assert!(c.approximate);
        assert!((c.value - numerator / 1056.4).abs() < 0.005);
    }
    assert!(
        e.source("soshi-1991")
            .unwrap()
            .population
            .contains("Adulthood not stated")
    );
}

#[test]
fn the_evidence_resolves_axes_and_distinguishes_the_specimen_sets() {
    let e = Evidence::read().unwrap();
    // Joint-axis mappings retain the published headings.
    for (key, moments) in [
        ("t1-2-to-l4-5-intact", [448.0, 644.0, 738.0]),
        ("t1-2-to-l4-5-facetectomy", [224.0, 406.0, 406.0]),
        ("l5-s1", [222.0, 410.0, 410.0]),
    ] {
        check(
            &e,
            TableKind::JointSprings,
            key,
            "tachi-2021",
            Unit::NmmPerDegree,
            &[
                ("moment-1", moments[0]),
                ("moment-2", moments[1]),
                ("moment-3", moments[2]),
            ],
        );
        let r = e.row(TableKind::JointSprings, key).unwrap();
        for mapping in [
            "AP (x; lateral bending)",
            "ML (y; flexion-extension)",
            "CC (z; axial rotation)",
            "Argoubi",
        ] {
            assert!(r.notes.contains(mapping), "{key}/{mapping}");
        }
    }
    assert!(
        !e.table(TableKind::JointSprings)
            .missing
            .iter()
            .any(|m| m.key == "moment-axis-mapping")
    );
    for key in [
        "lumbar-disc-0",
        "lumbar-disc-250",
        "lumbar-disc-500",
        "lumbar-disc-642",
    ] {
        let notes = &e.row(TableKind::JointSprings, key).unwrap().notes;
        assert!(notes.contains("x = posterior-anterior shear/lateral bending"));
        assert!(notes.contains("y = lateral shear/flexion-extension"));
        assert!(notes.contains("z = compression-tension/axial rotation"));
        assert!(!notes.contains("ambiguous") && !notes.contains("unresolved"));
    }
    check(
        &e,
        TableKind::JointSprings,
        "lumbar-disc-642",
        "stokes-gardner-morse-2016",
        Unit::Hz,
        &[("test-frequency", 0.01)],
    );
    check(
        &e,
        TableKind::JointSprings,
        "lumbar-disc-642",
        "stokes-gardner-morse-2016",
        Unit::N,
        &[("preload-sd", 132.0)],
    );
    let frequency = &e
        .row(TableKind::JointSprings, "lumbar-disc-642")
        .unwrap()
        .notes;
    for condition in [
        "eight segments",
        "seven spines",
        "five male/two female",
        "mean age41",
    ] {
        assert!(frequency.contains(condition));
    }
    for key in ["rib-mannen-t1-t4", "rib-mannen-t4-t8", "rib-mannen-t8-t12"] {
        let r = e.row(TableKind::JointSprings, key).unwrap();
        assert!(r.notes.contains("no-ribs EZS means:n7"));
        assert!(r.values.iter().all(|c| c.unit == Unit::Unstated));
    }
}

#[test]
fn the_evidence_distinguishes_gangnet_measurements_from_the_sagittal_prior() {
    let e = Evidence::read().unwrap();
    // Unsigned3D and left lateral, not two measurements
    // of an anterior centroid coordinate.
    check(
        &e,
        TableKind::DailyLoads,
        "slice-offset",
        "gangnet-2003",
        Unit::Mm,
        &[
            ("distance-3d-mean", 28.0),
            ("distance-3d-sd", 5.6),
            ("left-lateral-mean", 7.6),
            ("left-lateral-sd", 1.6),
        ],
    );
    for (term, expected) in [("mean", 28.0), ("sd", 5.6)] {
        let c = e
            .value(TableKind::DailyLoads, "slice-offset", term)
            .unwrap();
        assert_eq!(c.value, expected);
        assert_eq!(c.basis, Basis::OwnInference);
        assert_eq!(c.source, "project-assumptions");
    }
    assert_eq!(e.source("gangnet-2003").unwrap().n.as_deref(), Some("34"));
    assert!(
        e.source("gangnet-2003")
            .unwrap()
            .population
            .contains("healthy upright standing")
    );
}

#[test]
fn the_evidence_preserves_population_and_indirect_source_qualifications() {
    let e = Evidence::read().unwrap();
    // Retain the stated populations and their separate qualifications.
    let pig = e.source("yilmaz-2009").unwrap();
    assert_eq!(
        pig.n.as_deref(),
        Some("2 animals with data;3 operated,1 lost")
    );
    assert!(pig.population.contains("six weeks old at surgery"));
    assert!(pig.population.contains("three months later"));
    assert!(
        e.row(TableKind::AnchorStrength, "laminar-hook")
            .unwrap()
            .description
            .contains("supralaminar")
    );
    check(
        &e,
        TableKind::AnchorStrength,
        "laminar-hook",
        "liljenqvist-2001",
        Unit::N,
        &[("mean", 600.0)],
    );
    check(
        &e,
        TableKind::JointSprings,
        "release",
        "wang-2017",
        Unit::Percent,
        &[
            ("axial-rotation", -17.0),
            ("flexion", -15.0),
            ("coronal", -3.8),
            ("compression", -14.0),
        ],
    );
    assert!(
        e.row(TableKind::JointSprings, "release")
            .unwrap()
            .notes
            .contains("calibration targets cited by Wang")
    );
    check(
        &e,
        TableKind::DailyLoads,
        "walking",
        "rohlmann-2002",
        Unit::Ratio,
        &[("lower", 1.28)],
    );
    check(
        &e,
        TableKind::DailyLoads,
        "walking",
        "rohlmann-2014-gait",
        Unit::Ratio,
        &[("upper", 1.71)],
    );
    let walking = &e.row(TableKind::DailyLoads, "walking").unwrap().notes;
    assert!(
        walking.contains("whole load on a short posterior internal fixator")
            && walking.contains("resultant force on a vertebral body replacement")
    );
    check(
        &e,
        TableKind::DailyLoads,
        "head-neck",
        "winter-via-keenan-2015",
        Unit::BodyWeightFraction,
        &[("fraction", 0.081)],
    );
    check(
        &e,
        TableKind::DailyLoads,
        "each-arm-hand",
        "winter-via-keenan-2015",
        Unit::BodyWeightFraction,
        &[("fraction", 0.056)],
    );
    let keenan = e.source("winter-via-keenan-2015").unwrap();
    assert_eq!(keenan.read, Read::FullText);
    assert_eq!(
        keenan.citation.title.as_deref(),
        Some("Gravity-induced coronal plane joint moments in adolescent idiopathic scoliosis.")
    );
    assert!(
        keenan
            .notes
            .contains("Biomechanics and Motor Control of Human Movement")
    );
    assert!(keenan.notes.contains("textbook itself was not opened"));
}

#[test]
fn additional_precisions_remain_distinct_from_the_rounded_values() {
    let e = Evidence::read().unwrap();
    // Separate material sets and exact source precisions
    // remain distinct from the rounded comparison cells.
    check(
        &e,
        TableKind::RodMaterials,
        "tachi-cocr",
        "tachi-2021",
        Unit::Ratio,
        &[("poisson-ratio", 0.3)],
    );
    check(
        &e,
        TableKind::RodMaterials,
        "little-ti",
        "little-2013",
        Unit::Ratio,
        &[("poisson-ratio", 0.3)],
    );
    check(
        &e,
        TableKind::AnchorStrength,
        "immature-pig",
        "le-cann-2015",
        Unit::N,
        &[("full-text-lower", 419.9), ("full-text-upper", 1341.2)],
    );
    check(
        &e,
        TableKind::AnchorStrength,
        "six-week-pig",
        "yilmaz-2009",
        Unit::N,
        &[("dilated-mean-abstract", 378.7)],
    );
    assert!(
        e.value(TableKind::AnchorStrength, "six-week-pig", "dilated-mean")
            .unwrap()
            .approximate
    );
}

#[test]
fn supplementary_records_close_the_remaining_citations_without_raising_read_status() {
    let e = Evidence::read().unwrap();
    // Bibliographic confirmation does not change how much of each paper was read.
    for (key, pmid, doi, read) in [
        (
            "abe-2015",
            "25810754",
            "10.1186/1748-7161-10-s2-s2",
            Read::FullText,
        ),
        (
            "cidambi-2012",
            "22426445",
            "10.1097/brs.0b013e318252ccbe",
            Read::Abstract,
        ),
        (
            "karami-2015",
            "25310387",
            "10.1097/bsd.0000000000000178",
            Read::Abstract,
        ),
        (
            "sairyo-1998",
            "9588471",
            "10.1097/00002517-199804000-00009",
            Read::Abstract,
        ),
        (
            "kang-2014",
            "24590340",
            "10.1097/bpo.0000000000000166",
            Read::Abstract,
        ),
        (
            "tsai-2009",
            "19402917",
            "10.1186/1471-2474-10-44",
            Read::FullText,
        ),
        (
            "kueny-2014",
            "25082759",
            "10.1007/s00586-014-3476-7",
            Read::Abstract,
        ),
        (
            "sariyilmaz-2021",
            "32453125",
            "10.1097/bpb.0000000000000751",
            Read::Abstract,
        ),
        (
            "weiser-2017",
            "28391382",
            "10.1007/s00586-017-5091-x",
            Read::Abstract,
        ),
        (
            "santoni-2009",
            "18790684",
            "10.1016/j.spinee.2008.07.008",
            Read::Abstract,
        ),
        (
            "keenan-2015",
            "26681978",
            "10.1186/s13013-015-0060-9",
            Read::FullText,
        ),
        (
            "rohlmann-2008-standing",
            "18469689",
            "10.1097/brs.0b013e3181722d52",
            Read::Abstract,
        ),
        (
            "rohlmann-2013-lifting",
            "23141957",
            "10.1016/j.jbiomech.2012.10.022",
            Read::Abstract,
        ),
        (
            "rohlmann-2014-activities",
            "24866883",
            "10.1371/journal.pone.0098510",
            Read::FullText,
        ),
        (
            "rohlmann-2008-month-one",
            "17983694",
            "10.1016/j.clinbiomech.2007.09.011",
            Read::Abstract,
        ),
        (
            "damm-2017",
            "27914627",
            "10.1016/j.jbiomech.2016.11.060",
            Read::Abstract,
        ),
        (
            "rohlmann-1997-walker",
            "8970923",
            "10.1016/s0021-9290(96)00103-0",
            Read::Abstract,
        ),
        (
            "rohlmann-2015-flexion",
            "25640900",
            "10.1016/j.jbiomech.2015.01.007",
            Read::Abstract,
        ),
        (
            "rohlmann-2014-carrying",
            "24559120",
            "10.1080/00140139.2014.887789",
            Read::Abstract,
        ),
        (
            "rohlmann-2014-cycling",
            "24743577",
            "10.1371/journal.pone.0095497",
            Read::Abstract,
        ),
        (
            "rohlmann-2014-vibration",
            "24201510",
            "10.1007/s00586-013-3087-8",
            Read::Abstract,
        ),
        (
            "rohlmann-2013-orthosis",
            "23623494",
            "10.1016/j.clinbiomech.2013.03.010",
            Read::Abstract,
        ),
    ] {
        let source = e.source(key).unwrap();
        assert_eq!(source.pmid.as_deref(), Some(pmid), "{key}");
        assert_eq!(source.doi.as_deref(), Some(doi), "{key}");
        assert_eq!(
            source.read, read,
            "metadata lookup must not raise read status: {key}"
        );
        assert!(
            source
                .notes
                .contains("Bibliographic record confirmed; numbers not independently checked")
        );
    }
    let papers: Vec<_> = e
        .sources()
        .iter()
        .filter(|s| s.key != "project-assumptions")
        .collect();
    assert_eq!(papers.len(), 58);
    for paper in papers {
        // The earlier report supplies the rod assumed for the six-patient
        // comparison; missing bibliographic fields stay absent.
        if paper.key == "salmingo-2012" {
            assert_eq!(paper.citation.authors, "Salmingo RA et al.");
            assert_eq!(paper.citation.year, 2012);
            assert_eq!(paper.pmid.as_deref(), Some("22321374"));
            assert_eq!(paper.read, Read::Secondary);
            assert_eq!(paper.n.as_deref(), Some("3"));
            assert!(paper.record_note.is_none());
            assert!(paper.citation.title.is_none());
            assert!(paper.citation.journal.is_none());
            assert!(paper.citation.volume.is_none());
            assert!(paper.citation.issue.is_none());
            assert!(paper.citation.pages.is_none());
            assert!(paper.doi.is_none());
            assert!(paper.pmcid.is_none());
            assert!(paper.url.is_none());
            assert!(
                paper
                    .notes
                    .contains("assumed for the six-patient comparison")
            );
            continue;
        }
        // The force calibration has a supplied manuscript reading and PMID,
        // but no complete bibliographic record. Missing fields stay absent.
        if paper.key == "salmingo-2013" {
            assert_eq!(paper.pmid.as_deref(), Some("23273729"));
            assert_eq!(paper.read, Read::FullText);
            assert_eq!(paper.n.as_deref(), Some("6"));
            assert!(paper.record_note.is_none());
            assert!(paper.citation.title.is_none());
            assert!(paper.doi.is_none());
            continue;
        }
        assert!(paper.record_note.is_some(), "{}", paper.key);
        assert!(
            paper
                .citation
                .title
                .as_deref()
                .is_some_and(|title| !title.is_empty()),
            "{}",
            paper.key
        );
        assert!(
            paper.citation.journal.is_some()
                && paper.citation.volume.is_some()
                && paper.citation.pages.is_some()
        );
        assert!(paper.pmid.is_some());
        if paper.key == "liljenqvist-2001" {
            assert!(paper.doi.is_none()); // The bibliographic record has no DOI.
        } else {
            assert!(paper.doi.is_some(), "{}", paper.key);
        }
    }
    assert_eq!(
        e.source("abe-2015").unwrap().pmcid.as_deref(),
        Some("PMC4331738")
    );
    assert_eq!(
        e.source("tsai-2009").unwrap().pmcid.as_deref(),
        Some("PMC2694760")
    );
    assert_eq!(
        e.source("weiser-2017").unwrap().citation.journal.as_deref(),
        Some("Eur Spine J")
    );
    assert_eq!(
        e.source("rohlmann-2015-flexion").unwrap().citation.authors,
        "Dreischarf M, Albiol L, Zander T, Arshad R, Graichen F, Bergmann G, Schmidt H, Rohlmann A"
    );
    assert!(
        e.source("rohlmann-2015-flexion")
            .unwrap()
            .notes
            .contains("Source key retained")
    );
}
