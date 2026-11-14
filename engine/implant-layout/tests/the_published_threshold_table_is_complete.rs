//! Published criteria retain their measurements, conditions and provenance.

use implant_layout::evidence::{Cell, Coordinate, PublishedValue, Table, Unit, tables};
use shapes::levels::Level;

fn threshold(key: &str, group: &str) -> &'static Cell {
    tables()
        .lookup(
            Table::LevelRuleThresholds,
            &Coordinate {
                key: key.into(),
                group: Some(group.into()),
                ..Coordinate::default()
            },
        )
        .unwrap_or_else(|| panic!("missing published threshold {key}/{group}"))
}

fn scalar(key: &str, group: &str, value: f64, unit: Unit, source: &str) -> &'static Cell {
    let cell = threshold(key, group);
    assert_eq!(cell.value, PublishedValue::Scalar { value });
    assert_eq!(cell.unit, unit);
    assert_eq!(cell.source, source);
    assert!(!cell.section.is_empty());
    cell
}

#[test]
fn the_kaya_criteria_keep_percent_and_grade_distinct() {
    // Lumbar flexibility >70%, AVR ≤grade2.
    let flexibility = scalar(
        "lumbar-flexibility-min",
        "kaya-selective-thoracic",
        70.0,
        Unit::Percent,
        "bmc-level-review",
    );
    assert!(
        flexibility
            .qualifier
            .as_deref()
            .unwrap()
            .contains("Strictly greater")
    );
    assert!(
        flexibility
            .qualifier
            .as_deref()
            .unwrap()
            .contains("primary was not opened")
    );
    let rotation = scalar(
        "lumbar-rotation-max",
        "kaya-selective-thoracic",
        2.0,
        Unit::NashMoeGrade,
        "bmc-level-review",
    );
    assert!(
        rotation
            .qualifier
            .as_deref()
            .unwrap()
            .contains("Less than or equal")
    );
}

#[test]
fn the_s1_liv_angle_does_not_invent_its_missing_geometry() {
    // PMID42811138: >9.5°, n177/289.
    let cell = scalar(
        "s1-liv-angle-min",
        "adding-on",
        9.5,
        Unit::Degrees,
        "s1-liv-42811138",
    );
    let qualifier = cell.qualifier.as_deref().unwrap();
    assert!(qualifier.contains("Strictly greater"));
    assert!(qualifier.contains("does not supply the angle geometry"));
    let source = tables().source(&cell.source).unwrap();
    assert_eq!(source.pmid.as_deref(), Some("42811138"));
}

#[test]
fn the_ssv_offset_and_postoperative_regions_remain_separate() {
    // PMID40103372: no more proximal thanSSV−1.
    let offset = scalar(
        "liv-above-ssv-max",
        "djk",
        1.0,
        Unit::Levels,
        "ssv-40103372",
    );
    assert!(
        offset
            .qualifier
            .as_deref()
            .unwrap()
            .contains("no more proximal than SSV−1")
    );
    // PMID34096362: postoperative TK≥25° OR TLK≥10° if SSV excluded.
    for (key, value, region) in [
        ("postoperative-kyphosis-t5-t12-min", 25.0, "T5–T12"),
        ("postoperative-kyphosis-t11-l2-min", 10.0, "T11–L2"),
    ] {
        let cell = scalar(
            key,
            "ssv-excluded-djk",
            value,
            Unit::Degrees,
            "djk-34096362",
        );
        let qualifier = cell.qualifier.as_deref().unwrap();
        for condition in [
            "Greater than or equal",
            "postoperative",
            region,
            "SSV excluded",
            "OR",
        ] {
            assert!(qualifier.contains(condition), "{condition}");
        }
    }
}

#[test]
fn l3_translation_retains_the_concave_bend_and_curve_eligibility() {
    // PMID30739071: translation<10mm when lumbarCobb>60°.
    let cell = scalar(
        "l3-bending-translation-max",
        "l3-on-concave-bend",
        10.0,
        Unit::Mm,
        "l3-bending-30739071",
    );
    let qualifier = cell.qualifier.as_deref().unwrap();
    assert!(qualifier.contains("Strictly less than"));
    assert!(qualifier.contains("preoperative concave side-bending"));
    let eligibility = scalar(
        "lumbar-cobb-min",
        "l3-on-concave-bend",
        60.0,
        Unit::Degrees,
        "l3-bending-30739071",
    );
    assert!(
        eligibility
            .qualifier
            .as_deref()
            .unwrap()
            .contains("Strictly greater")
    );
}

#[test]
fn the_contested_shoulder_rule_keeps_all_named_options() {
    // Lenke1 left-highT2, levelT3/T4, right-highT4/T5.
    for (group, first, last) in [
        ("lenke-1-left-shoulder-high", Level::T2, Level::T2),
        ("lenke-1-level-shoulders", Level::T3, Level::T4),
        ("lenke-1-right-shoulder-high", Level::T4, Level::T5),
    ] {
        let cell = threshold("uiv-options", group);
        assert_eq!(cell.value, PublishedValue::LevelRange { first, last });
        assert_eq!(cell.unit, Unit::Levels);
        assert_eq!(cell.source, "bmc-level-review");
        let qualifier = cell.qualifier.as_deref().unwrap();
        assert!(qualifier.contains("Contested by Brooks"));
        assert!(qualifier.contains("outside this spine model"));
    }
}

#[test]
fn the_proximal_thoracic_criterion_keeps_its_film_condition() {
    // Chan2016 through review: PTbend≥15°.
    let cell = scalar(
        "pt-bending-cobb-min",
        "pt-behaves-structural",
        15.0,
        Unit::Degrees,
        "bmc-level-review",
    );
    let qualifier = cell.qualifier.as_deref().unwrap();
    for condition in [
        "Greater than or equal",
        "cervical supine side-bending",
        "primary not opened",
    ] {
        assert!(qualifier.contains(condition));
    }
}

#[test]
fn residual_t1_tilt_keeps_trigger_outcome_and_follow_up_distinct() {
    // PMID39658742: preoperativePT>26.5°, residualT1>5° at2years.
    for (key, value) in [
        ("preoperative-pt-cobb-min", 26.5),
        ("residual-t1-tilt-min", 5.0),
    ] {
        let cell = scalar(
            key,
            "residual-t1-tilt",
            value,
            Unit::Degrees,
            "residual-t1-39658742",
        );
        assert!(
            cell.qualifier
                .as_deref()
                .unwrap()
                .contains("Strictly greater")
        );
    }
    scalar(
        "follow-up",
        "residual-t1-tilt",
        2.0,
        Unit::Years,
        "residual-t1-39658742",
    );
}

#[test]
fn shoulder_height_keeps_its_negative_sign_and_correction_keeps_percent_units() {
    // PMID36138576: RSH>−6.5mm; MTcorrection>76.9%.
    for (key, value, unit) in [
        ("preoperative-rsh-min", -6.5, Unit::Mm),
        ("mt-correction-min", 76.9, Unit::Percent),
    ] {
        let cell = scalar(
            key,
            "lenke-1-shoulder-imbalance",
            value,
            unit,
            "shoulder-36138576",
        );
        assert!(
            cell.qualifier
                .as_deref()
                .unwrap()
                .contains("Strictly greater")
        );
    }
}

#[test]
fn absolute_and_increased_proximal_junction_angles_are_two_criteria() {
    // PJK definition: both≥10°, joined byAND.
    for key in ["absolute-pja-min", "pja-increase-min"] {
        let cell = scalar(key, "pjk-definition", 10.0, Unit::Degrees, "zhao-2024");
        let qualifier = cell.qualifier.as_deref().unwrap();
        assert!(qualifier.contains("Greater than or equal"));
        assert!(qualifier.contains("AND"));
    }
    assert!(
        threshold("absolute-pja-min", "pjk-definition")
            .qualifier
            .as_deref()
            .unwrap()
            .contains("absolute PJA")
    );
    assert!(
        threshold("pja-increase-min", "pjk-definition")
            .qualifier
            .as_deref()
            .unwrap()
            .contains("increase over preoperative")
    );
}

#[test]
fn distal_junction_and_coronal_definitions_keep_their_attribution_limits() {
    // DJA≥10°, primary attribution not unique.
    let cell = scalar(
        "distal-junction-angle-min",
        "djk-definition",
        10.0,
        Unit::Degrees,
        "fusion-level-definitions",
    );
    let qualifier = cell.qualifier.as_deref().unwrap();
    assert!(qualifier.contains("Greater than or equal"));
    assert!(qualifier.contains("does not uniquely attribute"));
    // Coronal decompensation: C7PL−CSVL >20 mm, PMC4417731 read in part.
    let cell = scalar(
        "c7pl-csvl-min",
        "coronal-decompensation",
        20.0,
        Unit::Mm,
        "coronal-decompensation-4417731",
    );
    assert!(
        cell.qualifier
            .as_deref()
            .unwrap()
            .contains("Strictly greater")
    );
    assert!(cell.qualifier.as_deref().unwrap().contains("read in part"));
}

#[test]
fn the_suk_summary_does_not_restrict_the_detailed_first_branch() {
    // The 0–1 summary must not invent a lower bound for the NV≤EV+1 rule.
    let summary = threshold("nv-ev-summary-interval", "suk-neutral-summary");
    assert_eq!(
        summary.value,
        PublishedValue::Range {
            lower: 0.0,
            upper: 1.0
        }
    );
    assert_eq!(summary.unit, Unit::Levels);
    assert_eq!(summary.source, "liu-2020");
    let qualifier = summary.qualifier.as_deref().unwrap();
    assert!(qualifier.contains("no separate lower bound"));
    assert!(qualifier.contains("must not exclude a more proximal NV"));
    scalar("nv-ev-max", "suk-neutral", 1.0, Unit::Levels, "liu-2020");
    scalar("nv-ev-min", "suk-neutral", 2.0, Unit::Levels, "liu-2020");
}
