use implant_layout::evidence::{
    AgeBand, Coordinate, PublishedValue, ReadStatus, Sex, Table, Unit, Usage, tables,
};
use shapes::implant_layout::CurveSide;
use shapes::levels::Level;

fn coordinate(key: &str, group: &str) -> Coordinate {
    Coordinate {
        key: key.to_string(),
        group: Some(group.to_string()),
        ..Coordinate::default()
    }
}

fn expect_cell(
    table: Table,
    coordinate: Coordinate,
    value: PublishedValue,
    unit: Unit,
    source: &str,
) {
    let cell = tables()
        .lookup(table, &coordinate)
        .unwrap_or_else(|| panic!("missing {table:?}: {coordinate:?}"));
    assert_eq!(cell.value, value, "{table:?}: {coordinate:?}");
    assert_eq!(cell.unit, unit, "{table:?}: {coordinate:?}");
    assert_eq!(cell.source, source, "{table:?}: {coordinate:?}");
    assert!(!cell.section.trim().is_empty());
}

#[test]
fn each_population_column_keeps_its_published_width_and_spread() {
    // These columns are independent cohorts.
    for (group, side, level, mean, sd, source) in [
        (
            "gao-ais",
            Some(CurveSide::Concave),
            Level::T1,
            7.0,
            1.25,
            "gao-2017",
        ),
        (
            "gao-ais",
            Some(CurveSide::Convex),
            Level::T4,
            4.44,
            0.88,
            "gao-2017",
        ),
        ("gao-controls", None, Level::T5, 4.44, 0.75, "gao-2017"),
        (
            "upendra-ais",
            Some(CurveSide::Concave),
            Level::L2,
            6.16,
            1.06,
            "upendra-2010",
        ),
        (
            "upendra-ais",
            Some(CurveSide::Convex),
            Level::L5,
            12.45,
            0.96,
            "upendra-2010",
        ),
        (
            "pluemvitayaporn-lenke-5",
            Some(CurveSide::Concave),
            Level::T5,
            2.71,
            0.91,
            "pluemvitayaporn-2025",
        ),
        (
            "pluemvitayaporn-lenke-5",
            Some(CurveSide::Convex),
            Level::L5,
            9.07,
            1.89,
            "pluemvitayaporn-2025",
        ),
    ] {
        expect_cell(
            Table::PedicleWidths,
            Coordinate {
                level: Some(level),
                curve_side: side,
                ..coordinate("outer-width", group)
            },
            // Gao leaves ± unlabelled; the other papers name SD.
            if source == "gao-2017" {
                PublishedValue::PlusMinus {
                    value: mean,
                    spread: sd,
                }
            } else {
                PublishedValue::MeanSd { mean, sd }
            },
            Unit::Mm,
            source,
        );
    }
}

#[test]
fn childrens_widths_keep_the_age_bands_and_sexes_separate() {
    // Zheng Table 1 leaves ± unlabelled; SD is only the project reading.
    for (level, age_band, sex, mean, sd) in [
        (Level::T1, AgeBand::FiveToEight, Sex::Male, 6.06, 0.92),
        (Level::T1, AgeBand::FiveToEight, Sex::Female, 5.73, 0.94),
        (Level::T5, AgeBand::NineToEleven, Sex::Male, 3.97, 1.00),
        (Level::T5, AgeBand::NineToEleven, Sex::Female, 3.91, 0.46),
        (Level::T12, AgeBand::TwelveToFourteen, Sex::Male, 7.83, 1.58),
        (
            Level::T12,
            AgeBand::TwelveToFourteen,
            Sex::Female,
            7.39,
            1.64,
        ),
    ] {
        expect_cell(
            Table::PedicleWidths,
            Coordinate {
                level: Some(level),
                age_band: Some(age_band),
                sex: Some(sex),
                ..coordinate("outer-width", "zheng-children")
            },
            PublishedValue::PlusMinus {
                value: mean,
                spread: sd,
            },
            Unit::Mm,
            "zheng-2009",
        );
    }
}

#[test]
fn an_age_band_or_population_without_a_width_has_no_value() {
    // Gao has no lumbar rows; Zheng gives thoracic widths only,
    // with no combined-sex column.
    for level in [Level::L1, Level::L2, Level::L3, Level::L4, Level::L5] {
        assert!(
            tables()
                .lookup(
                    Table::PedicleWidths,
                    &Coordinate {
                        level: Some(level),
                        ..coordinate("outer-width", "gao-controls")
                    }
                )
                .is_none()
        );
        for age_band in [AgeBand::FiveToEight, AgeBand::NineToEleven] {
            for sex in [Sex::Male, Sex::Female] {
                assert!(
                    tables()
                        .lookup(
                            Table::PedicleWidths,
                            &Coordinate {
                                level: Some(level),
                                age_band: Some(age_band),
                                sex: Some(sex),
                                ..coordinate("outer-width", "zheng-children")
                            }
                        )
                        .is_none()
                );
            }
        }
    }
    assert!(
        tables()
            .lookup(
                Table::PedicleWidths,
                &Coordinate {
                    level: Some(Level::T1),
                    age_band: Some(AgeBand::FiveToEight),
                    ..coordinate("outer-width", "zheng-children")
                }
            )
            .is_none()
    );
}

#[test]
fn inner_widths_and_cortical_walls_remain_distinct_measurements() {
    // Inner transverse width.
    for (sex, value, spread) in [(Sex::Male, 2.25, 0.66), (Sex::Female, 2.06, 0.53)] {
        expect_cell(
            Table::InnerWidths,
            Coordinate {
                level: Some(Level::T4),
                age_band: Some(AgeBand::TwelveToFourteen),
                sex: Some(sex),
                ..coordinate("inner-width", "zheng-children")
            },
            PublishedValue::PlusMinus { value, spread },
            Unit::Mm,
            "zheng-2009",
        );
    }
    // Chiu: this is the right-side pair and its ± statistics are not named.
    for (key, value, spread) in [
        ("medial-wall-thickness", 0.85, 0.28),
        ("lateral-wall-thickness", 0.64, 0.26),
    ] {
        expect_cell(
            Table::InnerWalls,
            coordinate(key, "chiu-right"),
            PublishedValue::PlusMinus { value, spread },
            Unit::Mm,
            "chiu-2024",
        );
    }
    expect_cell(
        Table::InnerWalls,
        coordinate("fully-cortical-pedicles", "chiu-all"),
        PublishedValue::Scalar { value: 264.0 },
        Unit::Count,
        "chiu-2024",
    );
}

#[test]
fn published_chiu_walls_keep_physical_sides_and_unresolved_curve_sides_separate() {
    // Chiu abstract: the left pair is independent of the right pair.
    for (key, value, spread) in [
        ("medial-wall-thickness", 0.86, 0.26),
        ("lateral-wall-thickness", 0.64, 0.26),
    ] {
        expect_cell(
            Table::InnerWalls,
            coordinate(key, "chiu-left"),
            PublishedValue::PlusMinus { value, spread },
            Unit::Mm,
            "chiu-2024",
        );
    }
    // Same passage: the abstract labels these medial-wall values "right
    // concave", contradicting its earlier "left concave"/"right convex".
    for (level, sd) in [(Level::T7, 0.24), (Level::T8, 0.23)] {
        let coord = Coordinate {
            level: Some(level),
            ..coordinate("medial-wall-thickness", "chiu-side-unresolved")
        };
        expect_cell(
            Table::InnerWalls,
            coord.clone(),
            PublishedValue::MeanSd { mean: 0.73, sd },
            Unit::Mm,
            "chiu-2024",
        );
        let cell = tables().lookup(Table::InnerWalls, &coord).unwrap();
        assert_eq!(cell.coordinate.curve_side, None);
        let qualifier = cell.qualifier.as_deref().unwrap();
        assert!(qualifier.contains("Side unresolved"));
        assert!(qualifier.contains("right concave"));
        assert!(qualifier.contains("left concave"));
        assert!(qualifier.contains("right convex"));
    }
    // Same abstract: these fully corticalized pedicles were excluded.
    for (group, value) in [("chiu-right", 183.0), ("chiu-left", 81.0)] {
        expect_cell(
            Table::InnerWalls,
            coordinate("fully-cortical-pedicles", group),
            PublishedValue::Scalar { value },
            Unit::Count,
            "chiu-2024",
        );
    }
}

#[test]
fn published_sato_neutral_widths_are_kept_as_printed_but_not_used() {
    // Sato Table 2: the repeated right NV block makes the two neutral widths unusable.
    for (side, spread) in [(CurveSide::Convex, 0.9), (CurveSide::Concave, 0.7)] {
        let coord = Coordinate {
            curve_side: Some(side),
            ..coordinate("outer-width", "sato-neutral-one")
        };
        assert!(tables().lookup(Table::PedicleWidths, &coord).is_none());
        let cell = tables()
            .cells(Table::PedicleWidths)
            .iter()
            .find(|cell| cell.coordinate == coord)
            .expect("printed neutral width remains inspectable");
        assert_eq!(cell.usage, Usage::PrintedOnly);
        assert_eq!(cell.value, PublishedValue::PlusMinus { value: 3.6, spread });
        assert_eq!(cell.unit, Unit::Mm);
        assert_eq!(cell.source, "sato-2022");
        assert_eq!(cell.section, "Table 2");
        assert!(cell.not_for_use_reason().is_some());
    }
}

#[test]
fn complete_chords_and_angles_keep_exact_published_statistics() {
    // Upendra Tables2–3: the complete tables take precedence over rounded samples.
    for (table, key, level, side, mean, sd, unit) in [
        (
            Table::Chords,
            "chord",
            Level::T1,
            CurveSide::Concave,
            27.67,
            2.89,
            Unit::Mm,
        ),
        (
            Table::Chords,
            "chord",
            Level::T10,
            CurveSide::Convex,
            33.35,
            4.22,
            Unit::Mm,
        ),
        (
            Table::Chords,
            "chord",
            Level::L3,
            CurveSide::Concave,
            51.2,
            3.21,
            Unit::Mm,
        ),
        (
            Table::Chords,
            "chord",
            Level::L5,
            CurveSide::Convex,
            47.0,
            2.08,
            Unit::Mm,
        ),
        (
            Table::MedialAngles,
            "transverse-angle",
            Level::T7,
            CurveSide::Concave,
            26.0,
            1.14,
            Unit::Degrees,
        ),
        (
            Table::MedialAngles,
            "transverse-angle",
            Level::T7,
            CurveSide::Convex,
            17.0,
            2.33,
            Unit::Degrees,
        ),
    ] {
        expect_cell(
            table,
            Coordinate {
                level: Some(level),
                curve_side: Some(side),
                ..coordinate(key, "upendra-ais")
            },
            PublishedValue::MeanSd { mean, sd },
            unit,
            "upendra-2010",
        );
    }
    // Zheng Tables3–4, 12–14-year columns. The ± is literal;
    // reading it as SD and its sign as convergence is inferred.
    for (sex, value, spread) in [(Sex::Male, 39.42, 4.12), (Sex::Female, 37.71, 5.65)] {
        expect_cell(
            Table::Chords,
            Coordinate {
                level: Some(Level::T12),
                age_band: Some(AgeBand::TwelveToFourteen),
                sex: Some(sex),
                ..coordinate("pedicle-length", "zheng-children")
            },
            PublishedValue::PlusMinus { value, spread },
            Unit::Mm,
            "zheng-2009",
        );
    }
    for (level, sex, value, spread) in [
        (Level::T11, Sex::Male, -3.80, 3.00),
        (Level::T11, Sex::Female, -4.72, 2.21),
        (Level::T12, Sex::Male, -4.48, 3.00),
        (Level::T12, Sex::Female, -4.26, 2.93),
    ] {
        expect_cell(
            Table::MedialAngles,
            Coordinate {
                level: Some(level),
                age_band: Some(AgeBand::TwelveToFourteen),
                sex: Some(sex),
                ..coordinate("transverse-angle", "zheng-children")
            },
            PublishedValue::PlusMinus { value, spread },
            Unit::Degrees,
            "zheng-2009",
        );
    }
    // The published prose summaries stay separate from per-level rows.
    for (sex, lower, upper) in [(Sex::Male, 28.11, 32.26), (Sex::Female, 27.28, 32.29)] {
        expect_cell(
            Table::Chords,
            Coordinate {
                age_band: Some(AgeBand::FiveToEight),
                sex: Some(sex),
                ..coordinate("pedicle-length", "zheng-children")
            },
            PublishedValue::Range { lower, upper },
            Unit::Mm,
            "zheng-2009",
        );
    }
}

#[test]
fn concave_ratios_keep_apical_and_neutral_groups_separate() {
    // Only the apex ratio spread is explicitly read as SD.
    // The neutral pair retains bare ± notation.
    for (group, value) in [
        (
            "apex-one",
            PublishedValue::MeanSd {
                mean: 0.73,
                sd: 0.13,
            },
        ),
        (
            "neutral-one",
            PublishedValue::PlusMinus {
                value: 1.08,
                spread: 0.17,
            },
        ),
    ] {
        expect_cell(
            Table::ConcaveRatios,
            coordinate("concave-convex-ratio", group),
            value,
            Unit::Ratio,
            "sato-2022",
        );
    }
    expect_cell(
        Table::ConcaveRatios,
        coordinate("concave-convex-ratio-range", "apex-one"),
        PublishedValue::Range {
            lower: 0.28,
            upper: 1.00,
        },
        Unit::Ratio,
        "sato-2022",
    );
    assert!(
        tables()
            .lookup(
                Table::ConcaveRatios,
                &Coordinate {
                    level: Some(Level::T7),
                    ..coordinate("concave-convex-ratio", "apex-one")
                }
            )
            .is_none()
    );
}

#[test]
fn each_channel_grade_keeps_its_published_failure_percentage() {
    // Akazawa 2015 channel grades.
    for (group, value) in [
        ("grade-1", 0.5),
        ("grade-2", 2.9),
        ("grade-3", 12.0),
        ("grade-4", 31.5),
    ] {
        expect_cell(
            Table::ChannelGrades,
            coordinate("failure-rate", group),
            PublishedValue::Scalar { value },
            Unit::Percent,
            "akazawa-2015",
        );
    }
    // Grade 4 is only <1 mm; it has no published lower bound.
    assert!(
        tables()
            .lookup(
                Table::ChannelGrades,
                &coordinate("inner-diameter-min", "grade-4")
            )
            .is_none()
    );
    // Grade 1 is only ≥4 mm; it has no published upper bound.
    assert!(
        tables()
            .lookup(
                Table::ChannelGrades,
                &coordinate("inner-diameter-max", "grade-1")
            )
            .is_none()
    );
}

#[test]
fn screw_sizes_keep_the_reported_sizes_and_steps() {
    // Grabala's five screw sizes.
    expect_cell(
        Table::ScrewGrids,
        coordinate("diameter", "grabala"),
        PublishedValue::Grid {
            values: vec![5.0, 5.5, 6.0, 6.5, 7.0],
        },
        Unit::Mm,
        "grabala-2023",
    );
    // Balikci Table 1 gives 2.5 or 5 mm steps, without a 25–45 mm grid.
    expect_cell(
        Table::ScrewGrids,
        coordinate("length-step", "balikci"),
        PublishedValue::Grid {
            values: vec![2.5, 5.0],
        },
        Unit::Mm,
        "balikci-2019",
    );
    // The mid-thoracic Jiang cell is a single 5.5 mm size.
    expect_cell(
        Table::ScrewGrids,
        coordinate("diameter", "jiang-t5-t8"),
        PublishedValue::Scalar { value: 5.5 },
        Unit::Mm,
        "jiang-2022",
    );
}

#[test]
fn diameter_rules_preserve_ratios_and_conflicting_expansion_reports() {
    // Three published diameter rules.
    for (group, value, source) in [
        ("adult-cadaver", 0.80, "misenhimer-1989"),
        ("expansion-25", 1.25, "sato-2022"),
    ] {
        expect_cell(
            Table::DiameterRules,
            coordinate("outer-width-ratio-max", group),
            PublishedValue::Scalar { value },
            Unit::Ratio,
            source,
        );
    }
    expect_cell(
        Table::DiameterRules,
        coordinate("outer-width-ratio", "upendra-cited"),
        PublishedValue::Range {
            lower: 0.80,
            upper: 1.15,
        },
        Unit::Ratio,
        "upendra-2010",
    );
    // Grabala's 5.0 mm value differs between the table and the text.
    for (group, value) in [("grabala-table-5.0", 32.0), ("grabala-text-5.0", 39.0)] {
        expect_cell(
            Table::DiameterRules,
            coordinate("expansion", group),
            PublishedValue::Scalar { value },
            Unit::Percent,
            "grabala-2023",
        );
    }
    // Katsumi's odds increment was not supplied by the abstract.
    expect_cell(
        Table::DiameterRules,
        coordinate("deviation-odds", "screw-pedicle-ratio"),
        PublishedValue::Scalar { value: 1.607 },
        Unit::Ratio,
        "katsumi-2025",
    );
}

#[test]
fn length_rules_keep_percentages_and_the_reported_anterior_threshold() {
    // Length rules and their distinct units.
    for (key, group, lower, upper, unit, source) in [
        (
            "chord-penetration",
            "krag-cited",
            80.0,
            100.0,
            Unit::Percent,
            "balikci-2019",
        ),
        (
            "ap-penetration",
            "children",
            50.0,
            80.0,
            Unit::Percent,
            "zheng-2009",
        ),
        (
            "usual-length",
            "balikci-2019-t1-t5",
            25.0,
            30.0,
            Unit::Mm,
            "balikci-2019",
        ),
        (
            "usual-length",
            "kuraishi-2013-t6-t12",
            30.0,
            35.0,
            Unit::Mm,
            "kuraishi-2013",
        ),
        (
            "usual-length",
            "balikci-2019-l1-l5",
            35.0,
            40.0,
            Unit::Mm,
            "balikci-2019",
        ),
    ] {
        expect_cell(
            Table::LengthRules,
            coordinate(key, group),
            PublishedValue::Range { lower, upper },
            unit,
            source,
        );
    }
    // Sarwahi's anterior-breach threshold is 4 mm or less.
    expect_cell(
        Table::LengthRules,
        coordinate("anterior-breach-threshold", "aorta"),
        PublishedValue::Scalar { value: 4.0 },
        Unit::Mm,
        "sarwahi-2017",
    );
}

#[test]
fn level_thresholds_keep_structural_and_selective_rules_separate() {
    // Structural and sagittal modifiers.
    for (key, group, value, source) in [
        ("bending-cobb-min", "structural-minor", 25.0, "ao-lenke"),
        ("kyphosis-t2-t5-min", "structural-minor", 20.0, "ao-lenke"),
        ("kyphosis-t10-l2-min", "structural-minor", 20.0, "ao-lenke"),
        (
            "normal-kyphosis-min",
            "sagittal-modifier",
            10.0,
            "lenke-2001",
        ),
        (
            "normal-kyphosis-max",
            "sagittal-modifier",
            40.0,
            "lenke-2001",
        ),
    ] {
        expect_cell(
            Table::LevelRuleThresholds,
            coordinate(key, group),
            PublishedValue::Scalar { value },
            Unit::Degrees,
            source,
        );
    }
    // The three ratios use distinct thoracic/lumbar cutoffs.
    for key in [
        "cobb-ratio-min",
        "translation-ratio-min",
        "rotation-ratio-min",
    ] {
        for (group, value) in [("selective-thoracic", 1.2), ("selective-lumbar", 1.25)] {
            expect_cell(
                Table::LevelRuleThresholds,
                coordinate(key, group),
                PublishedValue::Scalar { value },
                Unit::Ratio,
                "korean-level-review",
            );
        }
    }
    // A published zero-degree target is retained as a value.
    expect_cell(
        Table::LevelRuleThresholds,
        coordinate("bending-disc-angle", "lev-plus-one-5c"),
        PublishedValue::Scalar { value: 0.0 },
        Unit::Degrees,
        "bmc-level-review",
    );
}

#[test]
fn stop_short_odds_match_the_full_text_ladder_and_its_denominators() {
    // Liu/Lenke 2020 pooled table.
    for (group, proximal, distal, patients) in [
        ("sv-minus-one", 2.60, 0.44, 369.0),
        ("ev-plus-one", 2.43, 0.56, 371.0),
        ("nv", 3.05, 0.69, 472.0),
        ("ltv", 3.40, 0.66, 398.0),
        ("stv-or-nstv-plus-one", 4.52, 0.50, 426.0),
    ] {
        for (key, value, unit) in [
            ("proximal-odds", proximal, Unit::Ratio),
            ("distal-odds", distal, Unit::Ratio),
            ("patients", patients, Unit::Count),
        ] {
            expect_cell(
                Table::StopShortOdds,
                coordinate(key, group),
                PublishedValue::Scalar { value },
                unit,
                "liu-2020",
            );
        }
    }
    expect_cell(
        Table::StopShortOdds,
        coordinate("proximal-confidence-interval", "sv-minus-one"),
        PublishedValue::Range {
            lower: 1.44,
            upper: 4.69,
        },
        Unit::Ratio,
        "liu-2020",
    );
    expect_cell(
        Table::StopShortOdds,
        coordinate("i-squared", "stv-or-nstv-plus-one"),
        PublishedValue::Scalar { value: 80.0 },
        Unit::Percent,
        "liu-2020",
    );
}

#[test]
fn rod_evidence_keeps_the_survey_and_borrowed_cantilever_length() {
    // Wilke 2024 survey.
    expect_cell(
        Table::RodChoices,
        coordinate("diameter", "surgeon-survey"),
        PublishedValue::Scalar { value: 5.5 },
        Unit::Mm,
        "wilke-2024",
    );
    for (group, value) in [("cobalt-chrome", 53.0), ("titanium-alloy", 28.0)] {
        expect_cell(
            Table::RodChoices,
            coordinate("material-choice-share", group),
            PublishedValue::Scalar { value },
            Unit::Percent,
            "wilke-2024",
        );
    }
    // Cidambi 2012 abstract contour-loss values.
    expect_cell(
        Table::RodChoices,
        coordinate("overcontour", "concave"),
        PublishedValue::Scalar { value: 20.0 },
        Unit::Degrees,
        "cidambi-2012",
    );
    // Lumbar-model screw cantilever length.
    expect_cell(
        Table::RodChoices,
        coordinate("screw-cantilever-length", "lumbar"),
        PublishedValue::Scalar { value: 15.0 },
        Unit::Mm,
        "borrelli-2025",
    );
}

#[test]
fn anchor_pattern_evidence_keeps_trial_thresholds_and_hook_associations() {
    // MIMO trial low-density arm threshold.
    expect_cell(
        Table::AnchorPatterns,
        coordinate("density", "mimo-low-max"),
        PublishedValue::Scalar { value: 1.4 },
        Unit::ImplantsPerLevel,
        "mimo-2024",
    );
    // The table CI takes precedence over the prose misprint.
    expect_cell(
        Table::AnchorPatterns,
        coordinate("hook-pjk-odds", "hooks-at-uiv"),
        PublishedValue::Scalar { value: 0.48 },
        Unit::Ratio,
        "zhao-2024",
    );
    expect_cell(
        Table::AnchorPatterns,
        coordinate("hook-pjk-confidence-interval", "hooks-at-uiv"),
        PublishedValue::Range {
            lower: 0.31,
            upper: 0.74,
        },
        Unit::Ratio,
        "zhao-2024",
    );
    // Cammarata 2014: about 26% reduction.
    expect_cell(
        Table::AnchorPatterns,
        coordinate("junction-index-reduction", "hooks-at-uiv"),
        PublishedValue::Scalar { value: 26.0 },
        Unit::Percent,
        "cammarata-2014",
    );
}

#[test]
fn placement_checks_keep_the_unrounded_ppv_and_the_uniform_technique_angles() {
    // Full-cohort PPV is 55.5%; the rounded 55% is not used.
    expect_cell(
        Table::PlacementChecks,
        coordinate("positive-predictive-value", "liv-index"),
        PublishedValue::Scalar { value: 55.5 },
        Unit::Percent,
        "chen-2025",
    );
    // T1/UIV tilt correlation is 0.708.
    expect_cell(
        Table::PlacementChecks,
        coordinate("t1-uiv-tilt-correlation", "t1-tilt-proxy"),
        PublishedValue::Scalar { value: 0.708 },
        Unit::Correlation,
        "uiv-tilt-40020691",
    );
    // Fennell 2014 entry observations.
    for (group, value) in [("t1-t2", 30.0), ("t3-t12", 20.0)] {
        expect_cell(
            Table::PlacementChecks,
            coordinate("uniform-medial-angle", group),
            PublishedValue::Scalar { value },
            Unit::Degrees,
            "fennell-2014",
        );
    }
    assert_eq!(
        tables()
            .source("lehman-2003")
            .expect("Lehman citation")
            .read,
        ReadStatus::Abstract
    );
    assert_eq!(
        tables()
            .source("upendra-2010")
            .expect("Upendra citation")
            .read,
        ReadStatus::FullText
    );
}

#[test]
fn concave_apex_percentages_keep_their_different_width_conditions() {
    // Sato 2022: these are not a single range.
    expect_cell(
        Table::PlacementChecks,
        coordinate("patients-too-narrow", "apex-one-4mm-expansion"),
        PublishedValue::Scalar { value: 26.3 },
        Unit::Percent,
        "sato-2022",
    );
    expect_cell(
        Table::PlacementChecks,
        coordinate("patients-under-outer-width", "apex-one-under-4mm"),
        PublishedValue::Scalar { value: 71.9 },
        Unit::Percent,
        "sato-2022",
    );
    // Takeshita 2010: warning levels are for right thoracic curves.
    expect_cell(
        Table::PlacementChecks,
        coordinate("aorta-warning-levels", "right-thoracic-left-upper"),
        PublishedValue::LevelRange {
            first: Level::T4,
            last: Level::T5,
        },
        Unit::Levels,
        "takeshita-2010",
    );
    assert!(
        tables()
            .lookup(
                Table::PlacementChecks,
                &coordinate("aorta-warning-levels", "left-thoracic-right-upper")
            )
            .is_none()
    );
    // The threshold is the named L1–L2 disc.
    expect_cell(
        Table::LevelRuleThresholds,
        coordinate("apex-at-or-above", "lenke-l3"),
        PublishedValue::Disc {
            upper: Level::L1,
            lower: Level::L2,
        },
        Unit::Levels,
        "bmc-level-review",
    );
}

#[test]
fn every_table_value_resolves_to_a_readable_source_record() {
    let evidence = tables();
    let readme = include_str!("../evidence/README.md");
    for table in Table::ALL {
        assert!(!evidence.cells(table).is_empty(), "{table:?}");
        for cell in evidence.cells(table) {
            let source = evidence
                .source(&cell.source)
                .unwrap_or_else(|| panic!("{table:?}: {}", cell.source));
            assert!(
                source
                    .citation
                    .title
                    .as_deref()
                    .is_some_and(|title| !title.trim().is_empty()),
                "{}",
                cell.source
            );
            assert!(
                matches!(
                    source.read,
                    ReadStatus::FullText | ReadStatus::Abstract | ReadStatus::Secondary
                ),
                "{}",
                cell.source
            );
        }
    }
    for (key, source) in evidence.sources() {
        assert!(
            source
                .citation
                .title
                .as_deref()
                .is_some_and(|title| !title.trim().is_empty()),
            "{key}"
        );
        assert!(
            source
                .n
                .as_deref()
                .map(|n| !n.trim().is_empty())
                .unwrap_or(true),
            "{key}"
        );
        assert!(!source.population.trim().is_empty(), "{key}");
        assert!(
            readme.contains(key),
            "{key} is absent from the human source list"
        );
        if let Some(pmid) = &source.pmid {
            assert!(
                !pmid.is_empty() && pmid.bytes().all(|byte| byte.is_ascii_digit()),
                "{key}"
            );
        }
    }
}

fn assert_no_null(value: &serde_json::Value) {
    match value {
        serde_json::Value::Null => panic!("an absent evidence cell must be omitted"),
        serde_json::Value::Array(values) => values.iter().for_each(assert_no_null),
        serde_json::Value::Object(fields) => fields.values().for_each(assert_no_null),
        _ => {}
    }
}

#[test]
fn absent_evidence_cells_have_no_null_stand_ins() {
    for text in [
        include_str!("../evidence/pedicle-widths.json"),
        include_str!("../evidence/inner-widths.json"),
        include_str!("../evidence/inner-walls.json"),
        include_str!("../evidence/chords.json"),
        include_str!("../evidence/medial-angles.json"),
        include_str!("../evidence/concave-ratios.json"),
        include_str!("../evidence/channel-grades.json"),
        include_str!("../evidence/screw-grids.json"),
        include_str!("../evidence/diameter-rules.json"),
        include_str!("../evidence/length-rules.json"),
        include_str!("../evidence/level-rule-thresholds.json"),
        include_str!("../evidence/stop-short-odds.json"),
        include_str!("../evidence/rod-choices.json"),
        include_str!("../evidence/anchor-patterns.json"),
        include_str!("../evidence/placement-checks.json"),
        include_str!("../evidence/pedicle-lengths.json"),
        include_str!("../evidence/pedicle-heights.json"),
        include_str!("../evidence/screw-use.json"),
    ] {
        assert_no_null(&serde_json::from_str::<serde_json::Value>(text).expect("valid table JSON"));
    }
}

#[test]
fn gaos_complete_row_keeps_short_lengths_distinct_from_chords() {
    // Gao Table3, complete T4 row: the length is pedicle-only, not a full chord.
    for (table, key, group, side, value, spread, printed) in [
        (
            Table::PedicleWidths,
            "outer-width",
            "gao-ais",
            Some(CurveSide::Concave),
            3.41,
            0.98,
            "3.41 ± 0.98*,†",
        ),
        (
            Table::PedicleWidths,
            "outer-width",
            "gao-ais",
            Some(CurveSide::Convex),
            4.44,
            0.88,
            "4.44 ± 0.88",
        ),
        (
            Table::PedicleWidths,
            "outer-width",
            "gao-controls",
            None,
            4.46,
            0.89,
            "4.46 ± 0.89",
        ),
        (
            Table::PedicleLengths,
            "pedicle-only-length",
            "gao-ais",
            Some(CurveSide::Concave),
            15.0,
            2.43,
            "15 ± 2.43",
        ),
        (
            Table::PedicleLengths,
            "pedicle-only-length",
            "gao-ais",
            Some(CurveSide::Convex),
            14.0,
            2.18,
            "14 ± 2.18",
        ),
        (
            Table::PedicleLengths,
            "pedicle-only-length",
            "gao-controls",
            None,
            14.0,
            1.38,
            "14 ± 1.38",
        ),
    ] {
        let coord = Coordinate {
            level: Some(Level::T4),
            curve_side: side,
            ..coordinate(key, group)
        };
        expect_cell(
            table,
            coord.clone(),
            PublishedValue::PlusMinus { value, spread },
            Unit::Mm,
            "gao-2017",
        );
        let cell = tables().lookup(table, &coord).unwrap();
        assert_eq!(cell.printed.as_deref(), Some(printed));
        assert_eq!(cell.n, None);
        assert_eq!(cell.range, None);
    }
    let mut whole = 0;
    let mut total = 0;
    for table in [Table::PedicleWidths, Table::PedicleLengths] {
        for cell in tables()
            .cells(table)
            .iter()
            .filter(|cell| cell.source == "gao-2017")
        {
            total += 1;
            assert_eq!(cell.whole_mm, cell.decimals == Some(0));
            assert!(cell.printed.is_some());
            whole += usize::from(cell.whole_mm);
        }
    }
    assert_eq!((total, whole), (72, 55));
    let no_chord = Coordinate {
        level: Some(Level::T4),
        curve_side: Some(CurveSide::Concave),
        ..coordinate("chord", "gao-ais")
    };
    assert!(tables().lookup(Table::Chords, &no_chord).is_none());
    assert!(
        tables()
            .definitions(Table::PedicleLengths)
            .iter()
            .any(|definition| definition
                .text
                .contains("short pedicle-only length, not a chord length"))
    );
}

#[test]
fn every_marked_observation_reports_its_reason_and_lookup_status() {
    // Marks preserve the printed source problem,
    // while usage distinguishes exclusion from doubt.
    for table in Table::ALL {
        for cell in tables().cells(table) {
            if let Some(mark) = &cell.mark {
                assert!(!mark.reason.trim().is_empty(), "{:?}", cell.coordinate);
                assert_eq!(
                    cell.section,
                    match (
                        cell.source.as_str(),
                        table,
                        cell.coordinate.group.as_deref()
                    ) {
                        ("zheng-2009", Table::MedialAngles, _) => "Table 4",
                        ("zheng-2009", _, _) => "Table 1",
                        ("sato-2022", Table::ConcaveRatios, _) => "Table 3",
                        ("sato-2022", _, _) => "Table 2",
                        ("upendra-2010", Table::PedicleWidths, Some("upendra-table-4")) =>
                            "Table 4(a) (full text)",
                        ("upendra-2010", Table::MedialAngles, Some("upendra-table-4")) =>
                            "Table 4(b) (full text)",
                        ("upendra-2010", Table::PedicleWidths, _) => "Table 1 (full text)",
                        ("upendra-2010", Table::MedialAngles, _) => "Table 2 (full text)",
                        ("upendra-2010", Table::Chords, _) => "Table 3 (full text)",
                        _ => panic!("unexpected marked cell: {:?}", cell.coordinate),
                    }
                );
                if cell.usage == Usage::PrintedOnly {
                    assert_eq!(cell.not_for_use_reason(), Some(mark.reason.as_str()));
                    assert!(tables().lookup(table, &cell.coordinate).is_none());
                } else {
                    assert_eq!(cell.not_for_use_reason(), None);
                    assert!(tables().lookup(table, &cell.coordinate).is_some());
                }
            }
            if cell.usage == Usage::PrintedOnly {
                assert!(cell.mark.is_some(), "{:?}", cell.coordinate);
                assert!(cell.not_for_use_reason().is_some());
            }
        }
        for definition in tables().definitions(table) {
            assert!(tables().source(&definition.source).is_some());
            assert!(!definition.text.is_empty());
        }
    }
}

#[test]
fn the_published_gao_columns_keep_all_literal_pairs_and_printed_precision() {
    // Gao Table 3: the three independent columns leave ± unlabelled.
    // Whole-mm precision is a transcription marker,
    // never an instruction to round another cell.
    let levels = [
        Level::T1,
        Level::T2,
        Level::T3,
        Level::T4,
        Level::T5,
        Level::T6,
        Level::T7,
        Level::T8,
        Level::T9,
        Level::T10,
        Level::T11,
        Level::T12,
    ];
    for (group, side, pairs, whole_levels) in [
        (
            "gao-ais",
            Some(CurveSide::Concave),
            [
                (7.0, 1.25),
                (5.0, 1.24),
                (3.76, 1.37),
                (3.41, 0.98),
                (3.95, 1.05),
                (4.05, 1.05),
                (3.97, 1.10),
                (4.08, 1.11),
                (4.92, 1.45),
                (6.0, 1.62),
                (7.0, 2.04),
                (7.0, 1.61),
            ],
            vec![Level::T1, Level::T2, Level::T10, Level::T11, Level::T12],
        ),
        (
            "gao-ais",
            Some(CurveSide::Convex),
            [
                (7.0, 1.09),
                (6.0, 0.91),
                (4.98, 1.03),
                (4.44, 0.88),
                (4.71, 1.01),
                (4.77, 1.01),
                (5.0, 0.88),
                (5.0, 1.04),
                (5.0, 1.05),
                (6.0, 1.43),
                (7.0, 1.81),
                (7.0, 1.83),
            ],
            vec![
                Level::T1,
                Level::T2,
                Level::T7,
                Level::T8,
                Level::T9,
                Level::T10,
                Level::T11,
                Level::T12,
            ],
        ),
        (
            "gao-controls",
            None,
            [
                (7.0, 1.01),
                (6.0, 1.02),
                (4.89, 0.83),
                (4.46, 0.89),
                (4.44, 0.75),
                (4.61, 0.79),
                (4.68, 0.80),
                (4.99, 0.91),
                (5.0, 0.90),
                (6.0, 1.00),
                (7.0, 1.73),
                (7.0, 1.57),
            ],
            vec![
                Level::T1,
                Level::T2,
                Level::T9,
                Level::T10,
                Level::T11,
                Level::T12,
            ],
        ),
    ] {
        for (level, (value, spread)) in levels.into_iter().zip(pairs) {
            let coord = Coordinate {
                level: Some(level),
                curve_side: side,
                ..coordinate("outer-width", group)
            };
            expect_cell(
                Table::PedicleWidths,
                coord.clone(),
                PublishedValue::PlusMinus { value, spread },
                Unit::Mm,
                "gao-2017",
            );
            let cell = tables().lookup(Table::PedicleWidths, &coord).unwrap();
            assert_eq!(cell.whole_mm, whole_levels.contains(&level));
            let qualifier = cell.qualifier.as_deref().unwrap();
            assert!(!qualifier.contains("Concave is left"));
            assert!(qualifier.contains("outer"));
        }
    }
    let source = tables().source("gao-2017").unwrap();
    assert!(
        source
            .population
            .contains("matched for age, sex and height")
    );
    assert!(source.notes.contains("28875444"));
    assert!(source.notes.contains("Table 3"));
}

#[test]
fn transcribed_zheng_width_statistics_remain_literal() {
    // Zheng Table1: all numeric pairs leave ± unlabelled; the malformed
    // 072 row is retained without supplying an inferred decimal point.
    let cells: Vec<_> = tables()
        .cells(Table::PedicleWidths)
        .iter()
        .filter(|cell| cell.source == "zheng-2009")
        .collect();
    assert_eq!(cells.len(), 72);
    for cell in cells {
        if cell.usage == Usage::Usable {
            assert!(matches!(cell.value, PublishedValue::PlusMinus { .. }));
        } else {
            assert_eq!(cell.printed.as_deref(), Some("4.83 ± 072"));
            assert_eq!(cell.value, PublishedValue::Scalar { value: 4.83 });
            assert!(cell.not_for_use_reason().is_some());
        }
    }
}

#[test]
fn published_lower_end_and_selective_lumbar_coordinates_keep_the_printed_rules() {
    // Liu 2020: Wang's first cephalad >10-mm vertebra is LIV itself (n=45), not LIV+1.
    let coord = coordinate("first-cephalad-deviation-min", "deviated-vertebra");
    expect_cell(
        Table::LevelRuleThresholds,
        coord.clone(),
        PublishedValue::Scalar { value: 10.0 },
        Unit::Mm,
        "liu-2020",
    );
    let cell = tables().lookup(Table::LevelRuleThresholds, &coord).unwrap();
    for phrase in [
        "Strictly greater",
        "cephalad",
        "sacrum",
        "n = 45",
        "LIV itself",
    ] {
        assert!(
            cell.qualifier.as_deref().unwrap().contains(phrase),
            "{phrase}"
        );
    }
    assert!(
        tables()
            .lookup(
                Table::LevelRuleThresholds,
                &coordinate("liv-plus-one-deviation-min", "wang-2011")
            )
            .is_none()
    );
    // Kim 2021 Table 2 prints TL/L wording;
    // the thoracic interpretation remains a project rule.
    expect_cell(
        Table::LevelRuleThresholds,
        coordinate("tll-bending-cobb-max", "selective-lumbar"),
        PublishedValue::Scalar { value: 25.0 },
        Unit::Degrees,
        "korean-level-review",
    );
    assert!(
        tables()
            .lookup(
                Table::LevelRuleThresholds,
                &coordinate("thoracic-bending-cobb-max", "selective-lumbar")
            )
            .is_none()
    );
    let ratio = coordinate("cobb-ratio-min", "sanders-selective-lumbar");
    expect_cell(
        Table::LevelRuleThresholds,
        ratio.clone(),
        PublishedValue::Scalar { value: 1.25 },
        Unit::Ratio,
        "korean-level-review",
    );
    assert!(
        tables()
            .lookup(Table::LevelRuleThresholds, &ratio)
            .unwrap()
            .qualifier
            .as_deref()
            .unwrap()
            .contains("inclusive")
    );
}

#[test]
fn the_published_liu_comparison_counts_do_not_share_denominators() {
    // Liu 2020, Tables 2–3.
    for (group, proximal_studies, proximal_patients, distal_studies, distal_patients) in [
        ("sv-minus-one", 4.0, 369.0, 4.0, 233.0),
        ("ev-plus-one", 4.0, 371.0, 4.0, 250.0),
        ("nv", 5.0, 472.0, 5.0, 286.0),
        ("ltv", 4.0, 398.0, 4.0, 441.0),
        ("stv-or-nstv-plus-one", 3.0, 426.0, 2.0, 158.0),
    ] {
        for (key, value) in [
            ("studies", proximal_studies),
            ("patients", proximal_patients),
            ("distal-studies", distal_studies),
            ("distal-patients", distal_patients),
        ] {
            expect_cell(
                Table::StopShortOdds,
                coordinate(key, group),
                PublishedValue::Scalar { value },
                Unit::Count,
                "liu-2020",
            );
        }
    }
    for cell in tables().cells(Table::StopShortOdds) {
        let qualifier = cell.qualifier.as_deref().unwrap();
        assert!(qualifier.contains("distal comparison") || qualifier.contains("Distal comparison"));
    }
}

#[test]
fn akazawas_published_grade_boundaries_are_assigned() {
    // Akazawa 2015 abstract.
    for (group, key, value, condition) in [
        ("grade-1", "inner-diameter-min", 4.0, "≥4 mm"),
        ("grade-2", "inner-diameter-min", 2.0, "≥2 mm and <4 mm"),
        ("grade-2", "inner-diameter-max", 4.0, "≥2 mm and <4 mm"),
        ("grade-3", "inner-diameter-min", 1.0, "≥1 mm and <2 mm"),
        ("grade-3", "inner-diameter-max", 2.0, "≥1 mm and <2 mm"),
        ("grade-4", "inner-diameter-max", 1.0, "<1 mm"),
    ] {
        let coord = coordinate(key, group);
        expect_cell(
            Table::ChannelGrades,
            coord.clone(),
            PublishedValue::Scalar { value },
            Unit::Mm,
            "akazawa-2015",
        );
        assert!(
            tables()
                .lookup(Table::ChannelGrades, &coord)
                .unwrap()
                .qualifier
                .as_deref()
                .unwrap()
                .contains(condition)
        );
    }
}

#[test]
fn uniform_technique_cells_keep_the_published_non_ais_populations() {
    // Peng 2023 and Fennell 2014.
    for (source, groups, patient_count) in [
        ("fennell-2014", ["t1-t2", "t3-t12"], "33"),
        ("peng-2023", ["peng-t1-t2", "peng-t3-t12"], "118"),
    ] {
        for (group, value) in groups.into_iter().zip([30.0, 20.0]) {
            let coord = coordinate("uniform-medial-angle", group);
            expect_cell(
                Table::PlacementChecks,
                coord.clone(),
                PublishedValue::Scalar { value },
                Unit::Degrees,
                source,
            );
            let cell = tables().lookup(Table::PlacementChecks, &coord).unwrap();
            let qualifier = cell.qualifier.as_deref().unwrap();
            assert!(qualifier.contains("not an AIS cohort"));
            assert!(qualifier.contains(patient_count));
            if source == "fennell-2014" {
                assert!(qualifier.contains("not tested in severe"));
            }
        }
    }
}

#[test]
fn every_identified_paper_has_its_complete_citation_and_a_recorded_identifier() {
    // There are 39 publications in the main set, eleven additional papers,
    // and the Kuraishi full paper separately from its meeting abstract.
    let papers: Vec<_> = tables()
        .sources()
        .values()
        .filter(|s| matches!(s.label.as_str(), "peer-reviewed" | "conference-abstract"))
        .collect();
    assert_eq!(papers.len(), 51);
    for source in papers {
        for field in [
            &source.citation.authors,
            &source.citation.title,
            &source.citation.journal,
        ] {
            assert!(
                field.as_deref().is_some_and(|text| !text.is_empty()),
                "{}",
                source.key
            );
        }
        assert!(source.citation.year.is_some(), "{}", source.key);
        assert!(
            source.pmid.is_some() || source.doi.is_some(),
            "{}",
            source.key
        );
    }
    // Chen 2025 key (2026 issue) and Kuraishi 2013.
    let chen = tables().source("chen-2025").unwrap();
    assert_eq!(chen.citation.year, Some(2026));
    assert!(chen.notes.contains("online-first"));
    let kuraishi = tables().source("kuraishi-2013").unwrap();
    assert_eq!(kuraishi.label, "conference-abstract");
    assert_eq!(kuraishi.doi.as_deref(), Some("10.1186/1748-7161-8-S1-O27"));
    assert!(kuraishi.notes.contains("21959835"));
    assert!(kuraishi.notes.contains("26(1):22"));
    // Bibliographic confirmation does not establish a new numerical reading.
    for (key, read, notes) in [
        (
            "boissiere-2023",
            ReadStatus::Abstract,
            "Recorded values are 5.5° spine-to-rod kyphosis disparity and the concave rod less kyphotic than the spine in 84%; no rod diameter, material or spread is recorded.",
        ),
        (
            "cui-2012",
            ReadStatus::Abstract,
            "The 8–10 mm danger-depth range is along the probe path, not pedicle width or a cortex margin. Further ± pairs reported for this series have no identified spread statistic.",
        ),
        (
            "etemadifar-2017",
            ReadStatus::FullText,
            "The Kim/Lenke entry technique is described through this opened series: about 2 mm lateral to the midpoint of the superior facet, at the superior border of the transverse process for T1–T10. It is not a landmark measured on the fitted template.",
        ),
        (
            "ueno-2025",
            ReadStatus::Abstract,
            "The 20.4–22.2% deviation is reported jointly with Akazawa 2023, without allocating the endpoints here. A broader approximate 22–31% hard-floor summary with Akazawa 2015 is kept separately; neither interval is repaired or treated as a pooled estimate.",
        ),
        (
            "s1-liv-42811138",
            ReadStatus::Abstract,
            "Exact S1–LIV angle geometry is not given in the abstract; the 9.5° threshold does not establish a computable angle definition.",
        ),
        (
            "ssv-40103372",
            ReadStatus::Abstract,
            "Rule is no more proximal than SSV−1 and below preoperative distal junctional kyphosis; other risk factors are also reported. This is distinct from the 122-patient postoperative-kyphosis cohort.",
        ),
        (
            "djk-34096362",
            ReadStatus::Abstract,
            "TK T5–T12 ≥25° or TLK T11–L2 ≥10° applies with SSV excluded. The 53.8% result is 7/13, not a full-cohort estimate.",
        ),
        (
            "l3-bending-30739071",
            ReadStatus::Abstract,
            "Translation threshold is strictly below 10 mm on the concave bending film, under the greater-than-60° curve condition; no standing-film substitution is implied.",
        ),
        (
            "residual-t1-39658742",
            ReadStatus::Abstract,
            "Preoperative PT Cobb >26.5° is the risk-factor threshold; T1 tilt >5° at two years is the distinct outcome definition.",
        ),
        (
            "shoulder-36138576",
            ReadStatus::Abstract,
            "Preoperative RSH >−6.5 mm and planned MT correction >76.9% are separate strict thresholds. The source’s imbalance definition is RSH ≥2 cm; no conversion is made in the tables.",
        ),
        (
            "coronal-decompensation-4417731",
            ReadStatus::FullText,
            "Full text read in part.",
        ),
    ] {
        let source = tables().source(key).expect(key);
        assert_eq!(source.read, read, "{key}");
        assert_eq!(
            source.notes,
            format!("{notes} Bibliographic record confirmed; numbers not independently checked."),
            "{key}"
        );
    }
}

mod upendra_complete {
    // Upendra Tables 1–3. Each row lists convex level/n/mean/range/SD, then concave.
    use implant_layout::evidence::{
        Cell, Coordinate, PublishedValue, ReadStatus, Table, Unit, Usage, tables,
    };
    use shapes::implant_layout::CurveSide;
    use shapes::levels::Level;

    const UPENDRA_WIDTH_ROWS: &str = r#"
T1|12|5.1|3.8-7.1|1.76|T1|12|5.2|3.6-6.8|1.01
T2|10|5.35|4.1-7.2|1.25|T2|11|5.2|3.7-7.2|1.36
T3|13|5.12|3.6-6.8|1.11|T3|13|4.1|3.4-5.1|0.31
T4|14|4.92|3.0-5.8|0.86|T4|15|4.42|3.6-5.0|0.62
T5|9|5.87|5.1-7.2|0.61|T5|9|5.2|4.1-6.2|0.41
T6|11|6.6|5.4-8.1|1.14|T6|13|4.95|4.0-6.1|0.56
T7|9|6.60|5.1-9.2|1.32|T7|11|5.50|4.6-6.3|0.21
T8|14|5.35|4.5-6.3|0.68|T8|14|5.33|4.2-6.4|0.46
T9|12|5.08|4.3-6.2|0.89|T9|12|4.96|3.8-6.1|0.65
T10|15|5.36|4.2-7.7|1.12|T10|15|5.21|4.1-7.1|1.01
T11|16|6.27|4.8-8.2|1.36|T11|16|5.60|3.6-8.1|1.65
T12|19|6.8|5.0-9.7|1.33|T12|22|5.22|4.1-7.2|0.89
L1|18|6.48|4.6-10.1|1.78|L1|19|6.04|4.0-9.2|1.63
L2|13|5.8|4.5-9.0|1.14|L2|13|6.16|4.6-9.1|1.06
L3|11|7.77|6.1-10.3|1.37|L3|11|7.46|5.1-10.3|1.83
L4|9|8.9|7.2-12.1|1.22|L4|9|8.16|6.1-10.4|1.71
L5|8|12.45|11.4-14.8|0.96|L5|8|9.85|8.1-12.0|1.44
"#;

    const UPENDRA_ANGLE_ROWS: &str = r#"
T1|9|20.67|16-28|4.66|T1|9|21.33|15-30|5.63
T2|11|17.67|14-25|4.96|T2|13|20.7|16-27|4.36
T3|13|12.67|7-21|6.63|T3|11|16|9-20|4.23
T4|16|14.6|0-20|8.14|T4|15|17.75|5-25|5.32
T5|9|18.25|10-26|5.1|T5|9|19.67|10-29|8.65
T6|13|14.5|4-24|5.96|T6|13|20.75|13-34|9.11
T7|11|17|15-20|2.33|T7|11|26.0|20-31|1.14
T8|14|15|5-26|4.69|T8|17|15.67|3-28|9.68
T9|12|15.33|10-18|2.89|T9|12|19.5|14-22.5|3.34
T10|15|12.4|7-19|3.66|T10|15|12.8|4-24|8.24
T11|16|11|3-19|6.93|T11|16|12.0|4-18|6.52
T12|19|13.56|0-30|9.98|T12|16|14.8|5-24|6.59
L1|19|16.38|0-26|8.74|L1|19|16.6|4-26|6.56
L2|13|16.75|5-35|9.66|L2|13|17.6|4-23|5.18
L3|11|12.8|5-21|5.73|L3|11|16|8-23|6.21
L4|9|17.8|18-24|2.66|L4|9|18.7|12-24|4.43
L5|7|18|15-22|2.12|L5|7|24|19-28|4.98
"#;

    const UPENDRA_CHORD_ROWS: &str = r#"
T1|12|29.75|25.1-32.3|2.31|T1|12|27.67|25.1-32.6|2.89
T2|10|30.5|27.2-32.3|2.38|T2|10|32.83|28.1-37.3|3.55
T3|13|32.8|18.1-41.3|6.59|T3|13|37.33|32.1-40.5|3.89
T4|14|34.83|30.1-37.5|2.11|T4|14|35.75|30.4-40.5|4.12
T5|9|35.33|32.2-38.6|1.96|T5|9|37.0|32.1-43.1|4.22
T6|11|38.5|30.8-43.2|4.66|T6|11|39.1|37.1-43.2|3.11
T7|9|41.5|38.2-43.1|1.95|T7|9|41.8|39.2-43.8|2.12
T8|14|40.2|38.5-41.6|1.86|T8|14|42.1|40.0-44.8|1.68
T9|12|43.5|37.1-53.2|4.96|T9|12|44.2|35.4-46.5|4.97
T10|15|33.35|28.4-41.2|4.22|T10|15|41.5|38.1-46.8|3.12
T11|16|37.5|35.1-42.6|1.69|T11|16|39.5|35.1-46.1|4.2
T12|19|40.27|32.5-47.1|5.86|T12|19|42.2|28.9-50.4|5.66
L1|18|41.46|35.4-52.6|4.39|L1|18|46.8|36.1-55.7|5.76
L2|13|47.37|32.1-54.8|4.96|L2|13|49.86|43.2-54.8|4.11
L3|11|48.56|42.5-52.9|3.64|L3|11|51.2|45.1-55.4|3.21
L4|9|46.2|44.5-47.6|2.14|L4|9|47.65|45.1-50.9|2.11
L5|8|47.0|45.2-50.8|2.08|L5|8|48.2|45.1-50.8|2.89
"#;

    fn upendra_coordinate(key: &str, group: &str, level: Level, side: CurveSide) -> Coordinate {
        Coordinate {
            key: key.to_string(),
            group: Some(group.to_string()),
            level: Some(level),
            curve_side: Some(side),
            ..Coordinate::default()
        }
    }

    fn upendra_raw_cell(table: Table, coordinate: &Coordinate) -> &'static Cell {
        tables()
            .cells(table)
            .iter()
            .find(|cell| &cell.coordinate == coordinate)
            .expect("printed Upendra cell")
    }

    #[test]
    fn complete_upendra_values_counts_ranges_and_literal_means_match_the_paper() {
        // Upendra Tables 1–3 explicitly label mean/range/SD/n.
        let levels = [
            Level::T1,
            Level::T2,
            Level::T3,
            Level::T4,
            Level::T5,
            Level::T6,
            Level::T7,
            Level::T8,
            Level::T9,
            Level::T10,
            Level::T11,
            Level::T12,
            Level::L1,
            Level::L2,
            Level::L3,
            Level::L4,
            Level::L5,
        ];
        for (table, key, unit, rows, expected_counts) in [
            (
                Table::PedicleWidths,
                "outer-width",
                Unit::Mm,
                UPENDRA_WIDTH_ROWS,
                [213, 223],
            ),
            (
                Table::MedialAngles,
                "transverse-angle",
                Unit::Degrees,
                UPENDRA_ANGLE_ROWS,
                [217, 216],
            ),
            (
                Table::Chords,
                "chord",
                Unit::Mm,
                UPENDRA_CHORD_ROWS,
                [213, 213],
            ),
        ] {
            assert_eq!(rows.trim().lines().count(), 17);
            let mut counts = [0, 0];
            for (line, level) in rows.trim().lines().zip(levels) {
                let columns: Vec<_> = line.split('|').collect();
                assert_eq!(columns.len(), 10);
                for (side_index, side, offset) in
                    [(0, CurveSide::Convex, 0), (1, CurveSide::Concave, 5)]
                {
                    assert_eq!(columns[offset], format!("{level:?}"));
                    let n: u32 = columns[offset + 1].parse().unwrap();
                    let printed = columns[offset + 2];
                    let mean: f64 = printed.parse().unwrap();
                    let (lower, upper) = columns[offset + 3].split_once('-').unwrap();
                    let lower: f64 = lower.parse().unwrap();
                    let upper: f64 = upper.parse().unwrap();
                    let sd: f64 = columns[offset + 4].parse().unwrap();
                    let coordinate = upendra_coordinate(key, "upendra-ais", level, side);
                    let cell = upendra_raw_cell(table, &coordinate);
                    assert_eq!(
                        cell.value,
                        PublishedValue::MeanSd { mean, sd },
                        "{coordinate:?}"
                    );
                    assert_eq!(cell.n, Some(n), "{coordinate:?}");
                    assert_eq!(
                        cell.range.as_ref().map(|range| (range.lower, range.upper)),
                        Some((lower, upper)),
                        "{coordinate:?}"
                    );
                    assert_eq!(cell.printed.as_deref(), Some(printed), "{coordinate:?}");
                    assert_eq!(
                        cell.decimals,
                        Some(
                            printed
                                .split_once('.')
                                .map_or(0, |(_, decimal)| decimal.len() as u8)
                        ),
                        "{coordinate:?}"
                    );
                    assert_eq!(cell.unit, unit);
                    assert_eq!(cell.source, "upendra-2010");
                    assert_eq!(
                        cell.section,
                        match table {
                            Table::PedicleWidths => "Table 1 (full text)",
                            Table::MedialAngles => "Table 2 (full text)",
                            Table::Chords => "Table 3 (full text)",
                            _ => unreachable!(),
                        }
                    );
                    assert!(
                        cell.qualifier
                            .as_deref()
                            .unwrap()
                            .contains("[own inference]")
                    );
                    counts[side_index] += n;
                    if table == Table::MedialAngles
                        && level == Level::L4
                        && side == CurveSide::Convex
                    {
                        assert_eq!(cell.usage, Usage::PrintedOnly);
                        assert!(tables().lookup(table, &coordinate).is_none());
                    } else {
                        assert_eq!(cell.usage, Usage::Usable);
                        assert!(tables().lookup(table, &coordinate).is_some());
                    }
                }
            }
            assert_eq!(counts, expected_counts);
        }
        assert_eq!(
            tables().source("upendra-2010").unwrap().read,
            ReadStatus::FullText
        );
    }

    #[test]
    fn upendra_doubtful_figures_are_marked_and_the_impossible_angle_is_not_repaired() {
        // These four cells are doubtful, rather than impossible.
        for (table, key, level, side) in [
            (
                Table::PedicleWidths,
                "outer-width",
                Level::T3,
                CurveSide::Concave,
            ),
            (
                Table::PedicleWidths,
                "outer-width",
                Level::T7,
                CurveSide::Concave,
            ),
            (
                Table::MedialAngles,
                "transverse-angle",
                Level::T7,
                CurveSide::Concave,
            ),
            (Table::Chords, "chord", Level::T10, CurveSide::Convex),
        ] {
            let coordinate = upendra_coordinate(key, "upendra-ais", level, side);
            let cell = tables().lookup(table, &coordinate).unwrap();
            let mark = cell.mark.as_ref().expect("doubtful mark");
            assert_eq!(format!("{:?}", mark.kind), "Doubtful");
            assert!(!mark.reason.is_empty());
            assert_eq!(cell.usage, Usage::Usable);
        }
        let coordinate = upendra_coordinate(
            "transverse-angle",
            "upendra-ais",
            Level::L4,
            CurveSide::Convex,
        );
        let cell = upendra_raw_cell(Table::MedialAngles, &coordinate);
        assert_eq!(
            cell.value,
            PublishedValue::MeanSd {
                mean: 17.8,
                sd: 2.66
            }
        );
        assert_eq!(
            cell.range.as_ref().map(|range| (range.lower, range.upper)),
            Some((18.0, 24.0))
        );
        assert_eq!(
            format!("{:?}", cell.mark.as_ref().unwrap().kind),
            "Impossible"
        );
        assert_eq!(cell.usage, Usage::PrintedOnly);
        assert!(tables().lookup(Table::MedialAngles, &coordinate).is_none());
    }

    #[test]
    fn all_five_conflicting_upendra_table_four_figures_remain_inspectable_only() {
        // Upendra Table 4(a)/(b): all five differing cells.
        for (table, key, level, side, value, printed) in [
            (
                Table::PedicleWidths,
                "outer-width",
                Level::T10,
                CurveSide::Concave,
                5.51,
                "5.51",
            ),
            (
                Table::MedialAngles,
                "transverse-angle",
                Level::T10,
                CurveSide::Concave,
                12.0,
                "12.0",
            ),
            (
                Table::MedialAngles,
                "transverse-angle",
                Level::L1,
                CurveSide::Concave,
                16.0,
                "16.0",
            ),
            (
                Table::MedialAngles,
                "transverse-angle",
                Level::L4,
                CurveSide::Convex,
                21.0,
                "21",
            ),
            (
                Table::MedialAngles,
                "transverse-angle",
                Level::L4,
                CurveSide::Concave,
                16.7,
                "16.7",
            ),
        ] {
            let coordinate = upendra_coordinate(key, "upendra-table-4", level, side);
            let cell = upendra_raw_cell(table, &coordinate);
            assert_eq!(cell.value, PublishedValue::Scalar { value });
            assert_eq!(cell.printed.as_deref(), Some(printed));
            assert_eq!(cell.source, "upendra-2010");
            assert_eq!(
                cell.section,
                if table == Table::PedicleWidths {
                    "Table 4(a) (full text)"
                } else {
                    "Table 4(b) (full text)"
                }
            );
            assert_eq!(cell.usage, Usage::PrintedOnly);
            assert_eq!(
                format!("{:?}", cell.mark.as_ref().unwrap().kind),
                "AmbiguousPrint"
            );
            assert!(tables().lookup(table, &coordinate).is_none());
        }
    }
}

mod zheng_complete {
    use implant_layout::evidence::{
        AgeBand, Cell, Coordinate, MarkKind, PublishedValue, Sex, Table, Unit, Usage, tables,
    };
    use shapes::levels::Level;

    fn zheng_coordinate(key: &str, level: Level, age_band: AgeBand, sex: Sex) -> Coordinate {
        Coordinate {
            key: key.into(),
            level: Some(level),
            age_band: Some(age_band),
            sex: Some(sex),
            group: Some("zheng-children".into()),
            ..Coordinate::default()
        }
    }

    fn zheng_observation(
        table: Table,
        key: &str,
        level: Level,
        age_band: AgeBand,
        sex: Sex,
    ) -> &'static Cell {
        tables()
            .lookup(table, &zheng_coordinate(key, level, age_band, sex))
            .expect("the published usable cell is present")
    }

    #[test]
    fn zhengs_complete_first_rows_keep_each_age_and_sex_column() {
        let dimensions = [
            (AgeBand::FiveToEight, Sex::Male),
            (AgeBand::FiveToEight, Sex::Female),
            (AgeBand::NineToEleven, Sex::Male),
            (AgeBand::NineToEleven, Sex::Female),
            (AgeBand::TwelveToFourteen, Sex::Male),
            (AgeBand::TwelveToFourteen, Sex::Female),
        ];
        // Zheng Table 1, T1 first and second rows.
        // Row-to-inner/outer assignment is the project's inference, not a table label.
        let inner = [
            (3.19, 0.63),
            (2.95, 0.53),
            (4.15, 0.15),
            (3.26, 0.52),
            (4.41, 0.93),
            (3.76, 0.70),
        ];
        let outer = [
            (6.06, 0.92),
            (5.73, 0.94),
            (6.84, 0.67),
            (6.3, 0.68),
            (7.44, 0.82),
            (6.7, 0.89),
        ];
        // Zheng Table 2, complete T1 row.
        let sagittal = [
            (3.40, 0.52),
            (3.43, 0.62),
            (3.84, 0.44),
            (4.04, 0.41),
            (4.14, 0.54),
            (4.12, 0.79),
        ];
        // Zheng Table 3, complete T1 row.
        let length = [
            (28.57, 2.18),
            (27.28, 1.86),
            (29.99, 3.71),
            (29.06, 1.43),
            (31.94, 3.10),
            (31.03, 2.61),
        ];
        // Zheng Table 4, complete T1 row.
        let angle = [
            (30.94, 5.25),
            (29.42, 5.77),
            (30.83, 4.55),
            (31.16, 3.71),
            (30.69, 4.59),
            (31.40, 3.92),
        ];
        for (table, key, unit, row) in [
            (Table::InnerWidths, "inner-width", Unit::Mm, inner),
            (Table::PedicleWidths, "outer-width", Unit::Mm, outer),
            (
                Table::PedicleHeights,
                "sagittal-diameter",
                Unit::Mm,
                sagittal,
            ),
            (Table::Chords, "pedicle-length", Unit::Mm, length),
            (
                Table::MedialAngles,
                "transverse-angle",
                Unit::Degrees,
                angle,
            ),
        ] {
            for ((age_band, sex), (value, spread)) in dimensions.into_iter().zip(row) {
                let cell = zheng_observation(table, key, Level::T1, age_band, sex);
                assert_eq!(cell.value, PublishedValue::PlusMinus { value, spread });
                assert_eq!(cell.unit, unit);
                assert_eq!(cell.source, "zheng-2009");
                assert_eq!(
                    cell.section,
                    match table {
                        Table::InnerWidths | Table::PedicleWidths => "Table 1",
                        Table::PedicleHeights => "Table 2",
                        Table::Chords => "Table 3",
                        Table::MedialAngles => "Table 4",
                        _ => unreachable!(),
                    }
                );
                assert_eq!(cell.n, None);
                assert_eq!(cell.range, None);
            }
        }
        // Table 1 prints these central values to one decimal; zeros are not fabricated.
        let cell = zheng_observation(
            Table::PedicleWidths,
            "outer-width",
            Level::T1,
            AgeBand::NineToEleven,
            Sex::Female,
        );
        assert_eq!(cell.printed.as_deref(), Some("6.3 ± 0.68"));
        assert_eq!(cell.decimals, Some(1));
    }

    #[test]
    fn zhengs_three_printing_qualifications_keep_the_literal_numbers() {
        // Zheng Table 1: this spread token is malformed, so no numerical spread is inferred.
        let coordinate =
            zheng_coordinate("outer-width", Level::T10, AgeBand::FiveToEight, Sex::Female);
        assert!(tables().lookup(Table::PedicleWidths, &coordinate).is_none());
        let printed = tables()
            .cells(Table::PedicleWidths)
            .iter()
            .find(|cell| cell.coordinate == coordinate)
            .unwrap();
        assert_eq!(printed.value, PublishedValue::Scalar { value: 4.83 });
        assert_eq!(printed.printed.as_deref(), Some("4.83 ± 072"));
        assert_eq!(printed.decimals, Some(2));
        assert_eq!(printed.usage, Usage::PrintedOnly);
        let mark = printed.mark.as_ref().unwrap();
        assert_eq!(mark.kind, MarkKind::ProbableMisprint);
        assert!(mark.reason.contains("decimal point missing"));
        assert!(mark.reason.contains("072 is not parsed or repaired"));
        assert_eq!(printed.not_for_use_reason(), Some(mark.reason.as_str()));

        // Zheng Table 1, T1 first row, 9–11 M.
        let doubtful = zheng_observation(
            Table::InnerWidths,
            "inner-width",
            Level::T1,
            AgeBand::NineToEleven,
            Sex::Male,
        );
        assert_eq!(
            doubtful.value,
            PublishedValue::PlusMinus {
                value: 4.15,
                spread: 0.15
            }
        );
        assert_eq!(doubtful.usage, Usage::Usable);
        assert_eq!(doubtful.mark.as_ref().unwrap().kind, MarkKind::Doubtful);
        assert!(
            doubtful
                .mark
                .as_ref()
                .unwrap()
                .reason
                .contains("unusually small")
        );
        assert_eq!(doubtful.not_for_use_reason(), None);

        // Zheng Table 4, T10 5–8 F: spacing only.
        let spacing = zheng_observation(
            Table::MedialAngles,
            "transverse-angle",
            Level::T10,
            AgeBand::FiveToEight,
            Sex::Female,
        );
        assert_eq!(
            spacing.value,
            PublishedValue::PlusMinus {
                value: 6.45,
                spread: 3.35
            }
        );
        assert_eq!(spacing.printed.as_deref(), Some("6.45±3.35"));
        assert_eq!(spacing.usage, Usage::Usable);
        assert_eq!(
            spacing.mark.as_ref().unwrap().kind,
            MarkKind::ProbableMisprint
        );
        assert!(
            spacing
                .mark
                .as_ref()
                .unwrap()
                .reason
                .contains("Spacing only")
        );
    }

    #[test]
    fn zhengs_negative_angles_and_conflicting_length_definitions_remain_visible() {
        // Zheng Table 4, T11 9–11 F.
        let negative = zheng_observation(
            Table::MedialAngles,
            "transverse-angle",
            Level::T11,
            AgeBand::NineToEleven,
            Sex::Female,
        );
        assert_eq!(
            negative.value,
            PublishedValue::PlusMinus {
                value: -3.95,
                spread: 2.49
            }
        );
        assert_eq!(negative.printed.as_deref(), Some("−3.95 ± 2.49"));
        assert!(
            negative
                .qualifier
                .as_ref()
                .unwrap()
                .contains("own inference")
        );
        // Zheng length endpoints disagree between Table 3 and Figure 1.
        let definition = tables()
            .definitions(Table::Chords)
            .iter()
            .find(|definition| definition.key == "zheng-length-endpoints")
            .unwrap();
        assert_eq!(definition.source, "zheng-2009");
        assert!(definition.text.contains("anterior cortex of the vertebra"));
        assert!(definition.text.contains("posterior longitudinal ligament"));
        assert!(definition.text.contains("contradiction is unresolved"));
        // Zheng's full transcription is thoracic. No lumbar or side allocation is supplied.
        let missing = zheng_coordinate("outer-width", Level::L1, AgeBand::FiveToEight, Sex::Male);
        assert!(tables().lookup(Table::PedicleWidths, &missing).is_none());
    }

    #[test]
    fn zhengs_five_measurements_cover_all_twelve_levels_and_six_columns() {
        // Table 1 has 12 pairs of six-column rows; Tables 2–4 each have
        // 12 six-column rows. This includes the printed-only cell.
        for (table, key) in [
            (Table::InnerWidths, "inner-width"),
            (Table::PedicleWidths, "outer-width"),
            (Table::PedicleHeights, "sagittal-diameter"),
            (Table::Chords, "pedicle-length"),
            (Table::MedialAngles, "transverse-angle"),
        ] {
            assert_eq!(
                tables()
                    .cells(table)
                    .iter()
                    .filter(
                        |cell| cell.coordinate.group.as_deref() == Some("zheng-children")
                            && cell.coordinate.key == key
                            && cell.coordinate.level.is_some()
                    )
                    .count(),
                72
            );
        }
    }
}

mod sato_balikci_chiu_kuraishi_complete {
    //! Independent checks of the printed values.

    use implant_layout::evidence::{
        Cell, Coordinate, MarkKind, ObservedRange, PublishedValue, ReadStatus, Table, Unit, Usage,
        tables,
    };
    use shapes::implant_layout::CurveSide;
    use shapes::levels::Level;

    fn clinical_coordinate(key: &str, group: &str, curve_side: Option<CurveSide>) -> Coordinate {
        Coordinate {
            key: key.to_owned(),
            group: Some(group.to_owned()),
            curve_side,
            ..Coordinate::default()
        }
    }

    fn clinical_printed(
        table: Table,
        key: &str,
        group: &str,
        side: Option<CurveSide>,
    ) -> &'static Cell {
        let coordinate = clinical_coordinate(key, group, side);
        let cell = tables()
            .cells(table)
            .iter()
            .find(|cell| cell.coordinate == coordinate)
            .unwrap_or_else(|| panic!("missing printed cell {table:?}: {coordinate:?}"));
        assert_eq!(
            cell.section,
            if table == Table::ConcaveRatios {
                "Table 3"
            } else {
                "Table 2"
            }
        );
        cell
    }

    fn clinical_usable(
        table: Table,
        key: &str,
        group: &str,
        side: Option<CurveSide>,
        source: &str,
        unit: Unit,
        value: PublishedValue,
    ) -> &'static Cell {
        let coordinate = clinical_coordinate(key, group, side);
        let cell = tables()
            .lookup(table, &coordinate)
            .expect("published usable cell");
        assert_eq!(cell.source, source);
        assert_eq!(cell.unit, unit);
        assert_eq!(cell.value, value);
        assert_eq!(
            cell.section,
            match (source, table) {
                ("sato-2022", Table::ConcaveRatios) => "Table 3",
                ("sato-2022", _) => "Table 2",
                ("balikci-2019", Table::ScrewGrids) if matches!(key, "length" | "length-step") =>
                    "Table 1",
                ("balikci-2019", Table::ScrewGrids) => "not recorded",
                ("balikci-2019", Table::ScrewUse) if group.starts_with("balikci-table-2-") =>
                    "Table 2",
                ("balikci-2019", Table::ScrewUse) => "Table 4",
                ("chiu-2024", _) => "abstract",
                ("kuraishi-2013-full-paper", _) => "abstract",
                _ => panic!("unexpected source: {source}"),
            }
        );
        cell
    }

    #[test]
    fn sato_table_2_apex_height_row_retains_both_sides_and_ranges() {
        // Sato Table 2, H row.
        for (side, value, spread, lower, upper, printed) in [
            (
                CurveSide::Convex,
                12.4,
                1.5,
                9.1,
                16.3,
                "12.4 ± 1.5 (9.1–16.3)",
            ),
            (
                CurveSide::Concave,
                10.8,
                1.5,
                7.3,
                15.6,
                "10.8 ± 1.5 (7.3–15.6)",
            ),
        ] {
            let cell = clinical_usable(
                Table::PedicleHeights,
                "outer-height",
                "sato-apex-one",
                Some(side),
                "sato-2022",
                Unit::Mm,
                PublishedValue::PlusMinus { value, spread },
            );
            assert_eq!(cell.range, Some(ObservedRange { lower, upper }));
            assert_eq!(cell.printed.as_deref(), Some(printed));
            assert_eq!(cell.decimals, Some(1));
            assert_eq!(cell.n, None); // The 684 pedicles are cohort-wide, not this cell's n.
        }
        clinical_usable(
            Table::PedicleHeights,
            "p-value-max",
            "sato-apex-one-outer-height",
            None,
            "sato-2022",
            Unit::Ratio,
            PublishedValue::Scalar { value: 0.001 },
        );
    }

    #[test]
    fn sato_table_2_neutral_width_p_values_are_exact_not_bounds() {
        // Sato Table 2, neutral W and w rows.
        clinical_usable(
            Table::PedicleWidths,
            "p-value",
            "sato-neutral-one-outer-width",
            None,
            "sato-2022",
            Unit::Ratio,
            PublishedValue::Scalar { value: 0.929 },
        );
        clinical_usable(
            Table::InnerWidths,
            "p-value",
            "sato-neutral-one-inner-width",
            None,
            "sato-2022",
            Unit::Ratio,
            PublishedValue::Scalar { value: 0.903 },
        );
    }

    #[test]
    fn sato_table_3_height_ratios_retain_the_print_and_the_probable_misprint() {
        // Sato Table 3, Lt.H/Rt.H.
        let apex = clinical_printed(Table::ConcaveRatios, "outer-height-ratio", "apex-one", None);
        assert_eq!(
            apex.value,
            PublishedValue::PlusMinus {
                value: 0.86,
                spread: 0.52
            }
        );
        assert_eq!(
            apex.range,
            Some(ObservedRange {
                lower: 0.75,
                upper: 0.97
            })
        );
        assert_eq!(apex.printed.as_deref(), Some("0.86 ± 0.52 (0.75–0.97)"));
        assert_eq!(apex.mark.as_ref().unwrap().kind, MarkKind::ProbableMisprint);
        assert_eq!(apex.usage, Usage::PrintedOnly);
        assert!(apex.not_for_use_reason().unwrap().contains("0.52"));
        assert!(
            tables()
                .lookup(Table::ConcaveRatios, &apex.coordinate)
                .is_none()
        );
        let neutral = clinical_usable(
            Table::ConcaveRatios,
            "outer-height-ratio",
            "neutral-one",
            None,
            "sato-2022",
            Unit::Ratio,
            PublishedValue::PlusMinus {
                value: 0.97,
                spread: 0.06,
            },
        );
        assert_eq!(
            neutral.range,
            Some(ObservedRange {
                lower: 0.82,
                upper: 1.10
            })
        );
        // The apex width ratio uses 0.13 as SD.
        clinical_usable(
            Table::ConcaveRatios,
            "concave-convex-ratio",
            "apex-one",
            None,
            "sato-2022",
            Unit::Ratio,
            PublishedValue::MeanSd {
                mean: 0.73,
                sd: 0.13,
            },
        );
    }

    #[test]
    fn every_sato_neutral_block_exclusion_keeps_its_reason_and_value() {
        // Sato Table 2 has a duplicated block. The left neutral outer width
        // is also excluded under the project rule.
        for (table, key, side, value, spread, lower, upper) in [
            (
                Table::PedicleHeights,
                "outer-height",
                CurveSide::Convex,
                10.8,
                1.5,
                7.3,
                15.6,
            ),
            (
                Table::PedicleWidths,
                "outer-width",
                CurveSide::Convex,
                3.6,
                0.9,
                0.9,
                5.6,
            ),
            (
                Table::PedicleHeights,
                "inner-height",
                CurveSide::Convex,
                5.9,
                1.5,
                1.9,
                10.1,
            ),
            (
                Table::InnerWidths,
                "inner-width",
                CurveSide::Convex,
                1.7,
                0.7,
                0.2,
                3.9,
            ),
            (
                Table::PedicleWidths,
                "outer-width",
                CurveSide::Concave,
                3.6,
                0.7,
                2.4,
                5.7,
            ),
        ] {
            let cell = clinical_printed(table, key, "sato-neutral-one", Some(side));
            assert_eq!(cell.value, PublishedValue::PlusMinus { value, spread });
            assert_eq!(cell.range, Some(ObservedRange { lower, upper }));
            assert_eq!(cell.mark.as_ref().unwrap().kind, MarkKind::ProbableCopy);
            assert_eq!(cell.usage, Usage::PrintedOnly);
            assert!(cell.not_for_use_reason().unwrap().contains("11 of 12"));
            assert!(tables().lookup(table, &cell.coordinate).is_none());
        }
    }

    #[test]
    fn balikci_table_1_catalogue_and_diameter_increment_are_transcribed_separately() {
        // Balikci Table 1, available diameters.
        for (group, value) in [("balikci-mid-27.5", 27.5), ("balikci-standard-55", 55.0)] {
            clinical_usable(
                Table::ScrewGrids,
                "length",
                group,
                None,
                "balikci-2019",
                Unit::Mm,
                PublishedValue::Scalar { value },
            );
        }
        clinical_usable(
            Table::ScrewGrids,
            "diameter",
            "balikci",
            None,
            "balikci-2019",
            Unit::Mm,
            PublishedValue::Range {
                lower: 5.0,
                upper: 8.0,
            },
        );
        clinical_usable(
            Table::ScrewGrids,
            "diameter-step",
            "balikci",
            None,
            "balikci-2019",
            Unit::Mm,
            PublishedValue::Scalar { value: 1.0 },
        );
        assert!(
            tables()
                .definitions(Table::LengthRules)
                .iter()
                .any(|definition| {
                    definition.key == "balikci-length-choice-method"
                        && definition
                            .text
                            .contains("probe touched but not perforated the anterior cortex")
                        && definition
                            .text
                            .contains("The nearest screw length to the measured length was chosen")
                })
        );
    }

    #[test]
    fn balikci_table_2_mid_length_row_retains_region_counts_percentages_ci_and_p() {
        // Balikci Table 2.
        for (group, count, percent) in [
            ("balikci-table-2-mid-length-thoracal", 1007.0, 46.5),
            ("balikci-table-2-mid-length-lumbar", 264.0, 38.7),
        ] {
            clinical_usable(
                Table::ScrewUse,
                "count",
                group,
                None,
                "balikci-2019",
                Unit::Count,
                PublishedValue::Scalar { value: count },
            );
            clinical_usable(
                Table::ScrewUse,
                "percent",
                group,
                None,
                "balikci-2019",
                Unit::Percent,
                PublishedValue::Scalar { value: percent },
            );
        }
        clinical_usable(
            Table::ScrewUse,
            "confidence-interval",
            "balikci-table-2-mid-length",
            None,
            "balikci-2019",
            Unit::Ratio,
            PublishedValue::Range {
                lower: 0.765,
                upper: 0.813,
            },
        );
        clinical_usable(
            Table::ScrewUse,
            "p-value-max",
            "balikci-table-2-mid-length",
            None,
            "balikci-2019",
            Unit::Ratio,
            PublishedValue::Scalar { value: 0.05 },
        );
    }

    #[test]
    fn balikci_table_4_mid_length_row_keeps_the_concave_percentage_misprint() {
        // Balikci Table 4.
        for (side, count, percent) in [
            (CurveSide::Convex, 651.0, 45.7),
            (CurveSide::Concave, 620.0, 43.5),
        ] {
            clinical_usable(
                Table::ScrewUse,
                "count",
                "balikci-table-4-mid-length",
                Some(side),
                "balikci-2019",
                Unit::Count,
                PublishedValue::Scalar { value: count },
            );
            let cell = clinical_usable(
                Table::ScrewUse,
                "percent",
                "balikci-table-4-mid-length",
                Some(side),
                "balikci-2019",
                Unit::Percent,
                PublishedValue::Scalar { value: percent },
            );
            if side == CurveSide::Concave {
                assert!(
                    cell.qualifier
                        .as_deref()
                        .unwrap()
                        .contains("no percentage is recomputed")
                );
            }
        }
        clinical_usable(
            Table::ScrewUse,
            "confidence-interval",
            "balikci-table-4-mid-length",
            None,
            "balikci-2019",
            Unit::Ratio,
            PublishedValue::Range {
                lower: 0.474,
                upper: 0.681,
            },
        );
        clinical_usable(
            Table::ScrewUse,
            "p-value-max",
            "balikci-table-4-mid-length",
            None,
            "balikci-2019",
            Unit::Ratio,
            PublishedValue::Scalar { value: 0.05 },
        );
    }

    #[test]
    fn chiu_abstract_medial_wall_regions_preserve_each_physical_side() {
        // Chiu abstract quotations.
        for (group, value, spread) in [
            ("chiu-right-t1-t3", 0.89, 0.28),
            ("chiu-right-t4-t10", 0.75, 0.23),
            ("chiu-right-t11-l5", 0.92, 0.30),
            ("chiu-left-t1-t3", 0.88, 0.26),
            ("chiu-left-t4-t7", 0.76, 0.24),
            ("chiu-left-t8-l5", 0.90, 0.27),
        ] {
            let cell = clinical_usable(
                Table::InnerWalls,
                "medial-wall-thickness",
                group,
                None,
                "chiu-2024",
                Unit::Mm,
                PublishedValue::PlusMinus { value, spread },
            );
            assert_eq!(cell.coordinate.curve_side, None);
            assert_eq!(cell.n, None);
            assert!(
                cell.qualifier
                    .as_deref()
                    .unwrap()
                    .contains("curve-side attribution is unresolved")
            );
        }
        for group in ["chiu-right", "chiu-left"] {
            clinical_usable(
                Table::InnerWalls,
                "lateral-wall-thickness",
                group,
                None,
                "chiu-2024",
                Unit::Mm,
                PublishedValue::PlusMinus {
                    value: 0.64,
                    spread: 0.26,
                },
            );
        }
        let conflict = tables()
            .definitions(Table::InnerWalls)
            .iter()
            .find(|definition| definition.key == "chiu-side-conflict")
            .unwrap();
        assert!(conflict.text.contains("left concave"));
        assert!(conflict.text.contains("right convex"));
        assert!(conflict.text.contains("right concave"));
    }

    #[test]
    fn kuraishi_full_paper_abstract_retains_the_two_lengths_per_region() {
        // Kuraishi full-paper record and abstract.
        for (group, values) in [
            ("kuraishi-2013-full-paper-t1-t5", vec![25.0, 30.0]),
            ("kuraishi-2013-full-paper-t6-t12", vec![30.0, 35.0]),
            ("kuraishi-2013-full-paper-l1-l5", vec![35.0, 40.0]),
        ] {
            clinical_usable(
                Table::LengthRules,
                "usual-length",
                group,
                None,
                "kuraishi-2013-full-paper",
                Unit::Mm,
                PublishedValue::Grid { values },
            );
        }
        let source = tables().source("kuraishi-2013-full-paper").unwrap();
        assert_eq!(source.read, ReadStatus::Abstract);
        assert_eq!(source.n.as_deref(), Some("15"));
        assert_eq!(source.pmid.as_deref(), Some("21959835"));
        assert_eq!(source.doi.as_deref(), Some("10.1097/BSD.0b013e31823162ef"));
        assert!(source.notes.contains("Same work"));
        assert!(tables().source("kuraishi-2013").is_some());
        let no_per_level_diameter = Coordinate {
            key: "outer-width".to_owned(),
            group: Some("kuraishi-2013-full-paper".to_owned()),
            level: Some(Level::T1),
            curve_side: Some(CurveSide::Convex),
            ..Coordinate::default()
        };
        assert!(
            tables()
                .lookup(Table::PedicleWidths, &no_per_level_diameter)
                .is_none()
        );
    }
}
