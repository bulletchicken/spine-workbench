//! Hand-entered cells from the complete referenced comparison tables.
//! Unit, term and source checks keep similarly valued measurements distinct.

use load_bench::evidence::{Basis, Evidence, TableKind, Unit};

fn cells(
    e: &Evidence,
    table: TableKind,
    row: &str,
    unit: Unit,
    source: &str,
    expected: &[(&str, f64)],
) {
    for &(term, value) in expected {
        let actual = e
            .value(table, row, term)
            .unwrap_or_else(|| panic!("missing {table:?}/{row}/{term}"));
        assert_eq!(actual.value, value, "{row}/{term}");
        assert_eq!(actual.unit, unit, "{row}/{term}");
        assert_eq!(actual.source, source, "{row}/{term}");
        assert_eq!(
            actual.basis,
            if source == "project-assumptions" {
                Basis::OwnInference
            } else {
                Basis::Published
            },
            "{row}/{term}"
        );
    }
}

#[test]
fn lumbar_comparisons_keep_the_translation_means_and_standard_deviations() {
    let e = Evidence::read().unwrap();
    // Fx/Dx, Fy/Dy, Fz/Dz, mean/SD in kN/mm.
    // Named axes matter; matching a multiset would allow swapped axes/SDs.
    for (key, v) in [
        ("lumbar-disc-0", [0.06, 0.01, 0.08, 0.01, 0.41, 0.08]),
        ("lumbar-disc-250", [0.11, 0.01, 0.14, 0.02, 1.7, 0.1]),
        ("lumbar-disc-500", [0.15, 0.02, 0.19, 0.02, 2.5, 0.2]),
        ("lumbar-disc-642", [0.17, 0.07, 0.21, 0.07, 3.3, 0.7]),
    ] {
        cells(
            &e,
            TableKind::JointSprings,
            key,
            Unit::KnPerMm,
            "stokes-gardner-morse-2016",
            &[
                ("fx-dx", v[0]),
                ("fx-dx-sd", v[1]),
                ("fy-dy", v[2]),
                ("fy-dy-sd", v[3]),
                ("fz-dz", v[4]),
                ("fz-dz-sd", v[5]),
            ],
        );
    }
}

#[test]
fn the_rib_comparison_keeps_the_reported_uncertainty_beside_the_mean() {
    let e = Evidence::read().unwrap();
    // AR EZS T4–T8 with ribs 4.51 ± 6.13.
    cells(
        &e,
        TableKind::JointSprings,
        "rib-mannen-t4-t8",
        Unit::Unstated,
        "mannen-2018",
        &[("ar-with-ribs", 4.51), ("ar-with-ribs-sd", 6.13)],
    );
    // Wilke ROM values, full FE column.
    cells(
        &e,
        TableKind::JointSprings,
        "thoracic-rom",
        Unit::Degrees,
        "wilke-2017",
        &[
            ("t2-t7-flexion-extension-lower", 3.1),
            ("t2-t7-flexion-extension-upper", 3.8),
            ("t2-t3-flexion-extension", 4.1),
            ("t7-t8-flexion", 2.8),
            ("t7-t8-extension", 2.8),
            ("t8-t12-flexion-extension-lower", 3.1),
            ("t8-t12-flexion-extension-upper", 3.8),
        ],
    );
}

#[test]
fn rod_sections_and_the_additional_alloys_keep_their_original_units() {
    let e = Evidence::read().unwrap();
    // Computed section values are retained without recomputing them.
    for (key, v) in [
        (
            "section-at-4.75mm",
            [4.75, 25.0, 10.5, 2.85, 6.00, 4.82, 9.5],
        ),
        (
            "section-at-5.0mm",
            [5.0, 30.7, 12.3, 3.50, 7.36, 5.92, 11.0],
        ),
        (
            "section-at-5.5mm",
            [5.5, 44.9, 16.3, 5.12, 10.78, 8.67, 14.7],
        ),
        (
            "section-at-6.0mm",
            [6.0, 63.6, 21.2, 7.25, 15.27, 12.28, 19.1],
        ),
        (
            "section-at-6.35mm",
            [6.35, 79.8, 25.1, 9.10, 19.15, 15.40, 22.6],
        ),
    ] {
        cells(
            &e,
            TableKind::RodMaterials,
            key,
            Unit::Mm,
            "project-assumptions",
            &[("rod-diameter", v[0])],
        );
        cells(
            &e,
            TableKind::RodMaterials,
            key,
            Unit::MmToFourth,
            "project-assumptions",
            &[("second-moment", v[1])],
        );
        cells(
            &e,
            TableKind::RodMaterials,
            key,
            Unit::MmCubed,
            "project-assumptions",
            &[("section-modulus", v[2])],
        );
        cells(
            &e,
            TableKind::RodMaterials,
            key,
            Unit::NmSquared,
            "project-assumptions",
            &[
                ("ei-titanium", v[3]),
                ("ei-cobalt-chrome", v[4]),
                ("ei-stainless-steel", v[5]),
            ],
        );
        cells(
            &e,
            TableKind::RodMaterials,
            key,
            Unit::Nm,
            "project-assumptions",
            &[("yield-moment-titanium-at-900MPa", v[6])],
        );
    }
    // Reference inputs behind the section-property inferences.
    cells(
        &e,
        TableKind::RodMaterials,
        "section-table-reference",
        Unit::Mpa,
        "project-assumptions",
        &[("titanium-yield", 900.0)],
    );
    cells(
        &e,
        TableKind::RodMaterials,
        "section-table-reference",
        Unit::Gpa,
        "prasad-2017",
        &[
            ("young-modulus-titanium", 114.0),
            ("young-modulus-cobalt-chrome", 240.0),
            ("young-modulus-stainless-steel", 193.0),
        ],
    );
    // Distinct Ti6Al7Nb and UHSS material sets.
    cells(
        &e,
        TableKind::RodMaterials,
        "ti6al7nb",
        Unit::Gpa,
        "abe-2015",
        &[("young-modulus", 105.0), ("hardening-modulus", 2.41)],
    );
    cells(
        &e,
        TableKind::RodMaterials,
        "ti6al7nb",
        Unit::Mpa,
        "abe-2015",
        &[("yield", 900.0)],
    );
    cells(
        &e,
        TableKind::RodMaterials,
        "ti6al7nb",
        Unit::Ratio,
        "abe-2015",
        &[("yield-strain", 8.57e-3)],
    );
    cells(
        &e,
        TableKind::RodMaterials,
        "ultra-high-strength-steel",
        Unit::Ksi,
        "cidambi-2012",
        &[("uts", 200.0)],
    );
    cells(
        &e,
        TableKind::RodMaterials,
        "ultra-high-strength-steel",
        Unit::Mpa,
        "project-assumptions",
        &[("approximate-uts", 1380.0)],
    );
    assert!(
        e.value(
            TableKind::RodMaterials,
            "ultra-high-strength-steel",
            "approximate-uts"
        )
        .unwrap()
        .approximate
    );
}

#[test]
fn anchor_comparisons_keep_every_reference_multiplier_and_precision() {
    let e = Evidence::read().unwrap();
    // Pull-out comparisons with their qualifying conditions.
    cells(
        &e,
        TableKind::AnchorStrength,
        "osteoporotic-lumbar",
        Unit::N,
        "karami-2015",
        &[
            ("mid-body", 583.0),
            ("mid-body-sd", 306.0),
            ("pericortical", 713.0),
            ("pericortical-sd", 321.0),
            ("bicortical", 797.0),
            ("bicortical-sd", 285.0),
        ],
    );
    cells(
        &e,
        TableKind::AnchorStrength,
        "immature-calf",
        Unit::N,
        "sairyo-1998",
        &[
            ("six-mm-table", 853.0),
            ("six-mm-body-text", 852.5),
            ("seven-mm", 1015.0),
        ],
    );
    cells(
        &e,
        TableKind::AnchorStrength,
        "immature-calf",
        Unit::Mm,
        "sairyo-1998",
        &[("smaller-diameter", 6.0), ("larger-diameter", 7.0)],
    );
    cells(
        &e,
        TableKind::AnchorStrength,
        "immature-calf-hubbing",
        Unit::N,
        "kang-2014",
        &[
            ("non-hubbed", 922.0),
            ("non-hubbed-sd", 112.0),
            ("hubbed", 747.0),
            ("hubbed-sd", 197.0),
        ],
    );
    cells(
        &e,
        TableKind::AnchorStrength,
        "foam-reference",
        Unit::N,
        "tsai-2009",
        &[("lower", 1568.0), ("upper", 1904.0)],
    );
    cells(
        &e,
        TableKind::AnchorStrength,
        "foam-reference",
        Unit::Mm,
        "tsai-2009",
        &[("diameter", 6.0), ("engaged-length", 30.0)],
    );
    // Each pull-out effect retains its qualifying inputs.
    for (key, term, source, value) in [
        (
            "trajectory-multiplier",
            "straight-forward-over-anatomic",
            "lehman-2003",
            1.27,
        ),
        (
            "depth-multiplier",
            "bicortical-over-mid-body",
            "karami-2015",
            1.37,
        ),
        ("diameter-multiplier", "lower", "sairyo-1998", 1.19),
        ("diameter-multiplier", "upper", "kueny-2014", 1.24),
        ("hubbing-multiplier", "factor", "kang-2014", 0.81),
        (
            "direct-rotation-multiplier",
            "factor",
            "sariyilmaz-2021",
            0.65,
        ),
        ("low-bmd-fatigue", "fatigue-load", "weiser-2017", 0.60),
        ("low-bmd-fatigue", "cycles", "weiser-2017", 0.45),
        (
            "cortical-trajectory",
            "yield-pull-out",
            "santoni-2009",
            1.30,
        ),
        ("cortical-trajectory", "p", "santoni-2009", 0.08),
    ] {
        cells(
            &e,
            TableKind::AnchorStrength,
            key,
            Unit::Ratio,
            source,
            &[(term, value)],
        );
    }
    cells(
        &e,
        TableKind::AnchorStrength,
        "diameter-multiplier",
        Unit::Mm,
        "kueny-2014",
        &[("diameter-increment", 1.0)],
    );
    cells(
        &e,
        TableKind::AnchorStrength,
        "low-bmd-fatigue",
        Unit::MgPerCubicCentimetre,
        "weiser-2017",
        &[
            ("low-bmd-upper-exclusive", 80.0),
            ("high-bmd-lower-exclusive", 120.0),
        ],
    );
    // The hook comparison retains the reported region.
    let hook = e.row(TableKind::AnchorStrength, "laminar-hook").unwrap();
    assert!(hook.description.contains("T9–T12"));
    assert_eq!(hook.values[0].section, "abstract");
    cells(
        &e,
        TableKind::AnchorStrength,
        "laminar-hook",
        Unit::N,
        "liljenqvist-2001",
        &[("mean", 600.0)],
    );
}

#[test]
fn load_tables_keep_millions_pressure_inputs_and_activity_populations() {
    let e = Evidence::read().unwrap();
    // Mass fractions and model tissue density.
    cells(
        &e,
        TableKind::DailyLoads,
        "pearsall-slices",
        Unit::BodyWeightFraction,
        "pearsall-1996",
        &[
            ("t1-fraction", 0.011),
            ("trunk-total-rounded-fraction", 0.42),
        ],
    );
    cells(
        &e,
        TableKind::DailyLoads,
        "keenan-slices",
        Unit::BodyWeightFraction,
        "keenan-2014",
        &[("trunk-total-fraction", 0.278)],
    );
    cells(
        &e,
        TableKind::DailyLoads,
        "tissue-density",
        Unit::KgPerCubicMetre,
        "keenan-2015",
        &[("density", 1040.0)],
    );
    // Rounded million cycles/year, not converted back from steps.
    for (key, terms) in [
        ("annual-cycles-very-low", vec![("upper-exclusive", 1.8)]),
        ("annual-cycles-low", vec![("lower", 1.8), ("upper", 2.7)]),
        (
            "annual-cycles-typical",
            vec![("lower", 2.7), ("upper", 3.7)],
        ),
        ("annual-cycles-active", vec![("lower", 3.7), ("upper", 4.6)]),
        ("annual-cycles-very-active", vec![("lower-exclusive", 4.6)]),
    ] {
        cells(
            &e,
            TableKind::DailyLoads,
            key,
            Unit::MillionCyclesPerYear,
            "project-assumptions",
            &terms,
        );
    }
    cells(
        &e,
        TableKind::DailyLoads,
        "lying-relaxed",
        Unit::Ratio,
        "rohlmann-2008-standing",
        &[("multiple-upper-exclusive", 0.3)],
    );
    cells(
        &e,
        TableKind::DailyLoads,
        "lying-relaxed",
        Unit::Ratio,
        "project-assumptions",
        &[("pressure-ratio", 0.2)],
    );
    cells(
        &e,
        TableKind::DailyLoads,
        "lying-relaxed",
        Unit::Mpa,
        "wilke-1999",
        &[("lying-pressure", 0.1), ("standing-pressure", 0.5)],
    );
    cells(
        &e,
        TableKind::DailyLoads,
        "sitting-unsupported",
        Unit::Ratio,
        "project-assumptions",
        &[("multiple", 0.9)],
    );
    cells(
        &e,
        TableKind::DailyLoads,
        "sitting-unsupported",
        Unit::Mpa,
        "wilke-1999",
        &[("sitting-pressure", 0.46), ("standing-pressure", 0.5)],
    );
    cells(
        &e,
        TableKind::DailyLoads,
        "stairs",
        Unit::Ratio,
        "rohlmann-2014-gait",
        &[("down", 2.25), ("up", 2.65)],
    );
    cells(
        &e,
        TableKind::DailyLoads,
        "lifting-round-back",
        Unit::Ratio,
        "project-assumptions",
        &[("multiple", 4.6)],
    );
    cells(
        &e,
        TableKind::DailyLoads,
        "lifting-round-back",
        Unit::Mpa,
        "wilke-1999",
        &[("lifting-pressure", 2.3), ("standing-pressure", 0.5)],
    );
    cells(
        &e,
        TableKind::DailyLoads,
        "lifting-round-back",
        Unit::Kg,
        "wilke-1999",
        &[("weight", 20.0)],
    );
    cells(
        &e,
        TableKind::DailyLoads,
        "lifting-small-weights",
        Unit::Ratio,
        "rohlmann-2013-lifting",
        &[("multiple-upper-inclusive", 5.0)],
    );
    for (key, weight, lower, upper) in [
        ("backpack-10-percent", 10.0, 1.50, 1.62),
        ("backpack-15-percent", 15.0, 1.77, 1.94),
        ("backpack-20-percent", 20.0, 2.03, 2.28),
    ] {
        cells(
            &e,
            TableKind::DailyLoads,
            key,
            Unit::Percent,
            "schmid-2020-ais",
            &[("body-weight", weight)],
        );
        cells(
            &e,
            TableKind::DailyLoads,
            key,
            Unit::Ratio,
            "schmid-2020-ais",
            &[("multiple-lower", lower), ("multiple-upper", upper)],
        );
    }
    cells(
        &e,
        TableKind::DailyLoads,
        "scoliosis-vs-straight",
        Unit::Ratio,
        "schmid-2020-ais",
        &[("at-apex", 1.10), ("below-apex", 1.18)],
    );
    cells(
        &e,
        TableKind::DailyLoads,
        "adolescent-mvpa",
        Unit::StepsPerDay,
        "tudor-locke-2011",
        &[("lower", 10000.0), ("upper", 11700.0)],
    );
    cells(
        &e,
        TableKind::DailyLoads,
        "adolescent-mvpa",
        Unit::Minutes,
        "tudor-locke-2011",
        &[("duration", 60.0)],
    );
}

#[test]
fn adult_telemetry_keeps_absolute_loads_distinct_from_multiples() {
    let e = Evidence::read().unwrap();
    // Telemetry comparisons retain every numerical cell.
    for (key, source, expected) in [
        (
            "vbr-supine",
            "rohlmann-2014-activities",
            vec![("lower", 42.0), ("upper", 96.0)],
        ),
        (
            "vbr-month-one",
            "rohlmann-2008-month-one",
            vec![("lower", 150.0), ("upper", 450.0)],
        ),
        (
            "stand-sit-bend-twist",
            "rohlmann-2014-activities",
            vec![("vbr-threshold-exclusive", 1200.0)],
        ),
        (
            "vbr-upper-body-flexion",
            "rohlmann-2015-flexion",
            vec![("lower", 450.0), ("upper", 565.0)],
        ),
        (
            "vbr-lifting-peak",
            "rohlmann-2014-activities",
            vec![("peak", 1650.0)],
        ),
        (
            "backpack-absolute",
            "rohlmann-2014-carrying",
            vec![("force-increment", 35.0)],
        ),
        (
            "cycling",
            "rohlmann-2014-cycling",
            vec![
                ("force-increment-lower", 73.0),
                ("force-increment-upper", 225.0),
            ],
        ),
    ] {
        cells(&e, TableKind::DailyLoads, key, Unit::N, source, &expected);
    }
    cells(
        &e,
        TableKind::DailyLoads,
        "walking-body-weight",
        Unit::BodyWeightFraction,
        "damm-2017",
        &[("fraction", 0.39)],
    );
    cells(
        &e,
        TableKind::DailyLoads,
        "walking-with-walker",
        Unit::Ratio,
        "rohlmann-2014-gait",
        &[("vbr-multiple", 0.62)],
    );
    cells(
        &e,
        TableKind::DailyLoads,
        "walking-with-walker",
        Unit::Percent,
        "rohlmann-1997-walker",
        &[("fixator-change", -25.0)],
    );
    cells(
        &e,
        TableKind::DailyLoads,
        "stand-sit-bend-twist",
        Unit::Ratio,
        "rohlmann-2002",
        &[("fixator-lower", 1.11), ("fixator-upper", 1.20)],
    );
    cells(
        &e,
        TableKind::DailyLoads,
        "backpack-absolute",
        Unit::Kg,
        "rohlmann-2014-carrying",
        &[("weight", 10.0)],
    );
    cells(
        &e,
        TableKind::DailyLoads,
        "cycling",
        Unit::Watts,
        "rohlmann-2014-cycling",
        &[("power-lower", 25.0), ("power-upper", 85.0)],
    );
    cells(
        &e,
        TableKind::DailyLoads,
        "whole-body-vibration",
        Unit::Percent,
        "rohlmann-2014-vibration",
        &[("force-change", 24.0)],
    );
    cells(
        &e,
        TableKind::DailyLoads,
        "orthosis",
        Unit::Percent,
        "rohlmann-2013-orthosis",
        &[
            ("average-force-change", -9.0),
            ("hyperextension-force-change", -19.0),
        ],
    );
}

#[test]
fn coupon_conditions_keep_run_out_inequalities_and_reversal_units() {
    let e = Evidence::read().unwrap();
    // Carrion coupon data: all strain conditions.
    for (key, strain, ratio) in [
        ("r-minus-one-strain-0.006", 0.006, -1.0),
        ("r-minus-one-strain-0.004", 0.004, -1.0),
        ("r-zero-strain-0.004", 0.004, 0.0),
    ] {
        cells(
            &e,
            TableKind::Fatigue,
            key,
            Unit::Ratio,
            "carrion-2016",
            &[("strain-amplitude", strain), ("strain-ratio", ratio)],
        );
    }
    cells(
        &e,
        TableKind::Fatigue,
        "r-minus-one-strain-0.006",
        Unit::Reversals,
        "carrion-2016",
        &[("failure-lower", 124952.0), ("failure-upper", 207610.0)],
    );
    cells(
        &e,
        TableKind::Fatigue,
        "r-minus-one-strain-0.004",
        Unit::MillionReversals,
        "carrion-2016",
        &[
            ("run-out-lower-more-than", 2.3),
            ("run-out-upper-more-than", 4.4),
        ],
    );
    cells(
        &e,
        TableKind::Fatigue,
        "r-zero-strain-0.004",
        Unit::Reversals,
        "carrion-2016",
        &[("reversals-more-than", 2506668.0)],
    );
    cells(
        &e,
        TableKind::Fatigue,
        "run-out-definition",
        Unit::Cycles,
        "carrion-2016",
        &[("cycles-more-than", 1000000.0)],
    );
    assert!(
        e.value(TableKind::Fatigue, "r-minus-one-strain-0.004", "cycles")
            .is_none()
    );
}

#[test]
fn citation_authors_do_not_gain_detail_absent_from_the_records() {
    let e = Evidence::read().unwrap();
    // Full author records are retained.
    for (key, authors) in [
        ("stokes-gardner-morse-2016", "Stokes IAF, Gardner-Morse M"),
        (
            "watkins-2005",
            "Watkins R, Watkins R, Williams L, Ahlbrand S, Garcia R, Karamanian A, Sharp L, Vo C, Hedman T",
        ),
        (
            "prasad-2017",
            "Prasad K, Bazaka O, Chua M, Rochford M, Fedrick L, Spoor J, Symes R, Tieppo M, Collins C, Cao A, Markwell D, Ostrikov KK, Bazaka K",
        ),
        (
            "vieweg-2023",
            "Vieweg U, Keck J, Krüger S, Arabmotlagh M, Rauschmann M, Schilling C",
        ),
    ] {
        assert_eq!(e.source(key).unwrap().citation.authors, authors);
    }
}
