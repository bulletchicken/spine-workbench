//! Published-row edges and the centred whole-millimetre term.

use implant_layout::evidence::{AgeBand, Coordinate, PublishedValue, Sex, Table, tables};
use implant_layout::pedicles::{CurvePosition, draw_width, width_prior};
use implant_layout::readings::{NamedSampler, PedicleDraw, Version};
use implant_layout::screws::anatomy::{chord, medial_angle, medial_angles_beside};
use shapes::implant_layout::CurveSide;
use shapes::levels::Level;
use shapes::made_up::VertebraPose;
use shapes::spine_fit::{
    FitAngles, FitRmsMm, FitStandardDeviations, FitStopReason, FittedVertebra, InferredFit,
    SpineFit,
};
use shapes::spine_readings::BodySide;

fn fit(level: Level) -> SpineFit {
    SpineFit {
        vertebrae: vec![FittedVertebra {
            level,
            pose: VertebraPose {
                level,
                centre: [0.0, 0.0, 100.0],
                rotation: [0.0; 3],
                height: 20.0,
                wedge_front: 0.0,
                wedge_side: 0.0,
                top_half_width: 15.0,
                top_half_depth: 10.0,
                bottom_half_width: 15.0,
                bottom_half_depth: 10.0,
            },
            angles: FitAngles {
                side_tilt_rad: 0.0,
                forward_lean_rad: 0.0,
                twist_rad: 0.0,
            },
            sd: FitStandardDeviations {
                centre_mm: [0.0; 3],
                angles_rad: [0.0; 3],
            },
            inferred: InferredFit {
                depth: false,
                width: false,
                twist: false,
            },
            rms_mm: FitRmsMm {
                front: Some(0.0),
                side: Some(0.0),
            },
            corner_tilt_rad: None,
        }],
        side_scale: 1.0,
        side_offset_mm: 0.0,
        origin_level: level,
        iterations: 0,
        converged: true,
        stop_reason: FitStopReason::SmallStep,
        rms_mm: 0.0,
        lengths: shapes::spine_fit::FitLengths::Measured,
        typical_curve_deg: None,
        front_film: shapes::spine_fit::FilmPlacement {
            mm_per_px: [1.0; 2],
            origin_px: [0.0; 2],
            facing_assumed: false,
        },
        side_film: Some(shapes::spine_fit::FilmPlacement {
            mm_per_px: [1.0; 2],
            origin_px: [0.0; 2],
            facing_assumed: false,
        }),
    }
}

fn centre_spread(value: &PublishedValue) -> (f64, f64) {
    match *value {
        PublishedValue::PlusMinus { value, spread } => (value, spread),
        PublishedValue::MeanSd { mean, sd } => (mean, sd),
        _ => panic!("a width with a spread"),
    }
}

#[test]
fn fractional_age_edges_choose_the_stated_populations_and_both_sex_columns() {
    // Zheng's table is used below 12 years, Gao's from 12 or when age is absent.
    // Whole-year child band labels are interpreted as completed years.
    for (age, band) in [
        (Some(4.9), None),
        (Some(5.0), Some(AgeBand::FiveToEight)),
        (Some(8.9), Some(AgeBand::FiveToEight)),
        (Some(9.0), Some(AgeBand::NineToEleven)),
        (Some(11.9), Some(AgeBand::NineToEleven)),
    ] {
        let actual = width_prior(Level::T4, age, CurveSide::Convex);
        let Some(band) = band else {
            assert!(actual.is_none());
            assert!(chord(Level::T4, CurveSide::Convex, age).is_none());
            continue;
        };
        let read = |sex| {
            centre_spread(
                &tables()
                    .lookup(
                        Table::PedicleWidths,
                        &Coordinate {
                            key: "outer-width".into(),
                            level: Some(Level::T4),
                            age_band: Some(band),
                            sex: Some(sex),
                            group: Some("zheng-children".into()),
                            ..Coordinate::default()
                        },
                    )
                    .unwrap()
                    .value,
            )
        };
        let m = read(Sex::Male);
        let f = read(Sex::Female);
        let actual = actual.unwrap();
        assert_eq!(actual.mean_mm, (m.0 + f.0) / 2.0);
        assert_eq!(actual.sd_mm, m.1.max(f.1));
        assert_eq!(actual.sources, vec!["zheng-2009"]);
        assert!(width_prior(Level::L1, age, CurveSide::Concave).is_none());
        assert!(chord(Level::L1, CurveSide::Concave, age).is_none());
    }
    let adolescent = width_prior(Level::T4, None, CurveSide::Outside).unwrap();
    for age in [Some(12.0), Some(14.0), Some(14.1), Some(18.0), None] {
        assert_eq!(
            width_prior(Level::T4, age, CurveSide::Outside).unwrap(),
            adolescent
        );
        assert_eq!(adolescent.sources, vec!["gao-2017"]);
        assert_eq!(
            chord(Level::T4, CurveSide::Convex, age).unwrap().sources,
            vec!["upendra-2010"]
        );
    }
    assert!(width_prior(Level::T13, None, CurveSide::Convex).is_none());
    assert!(chord(Level::T13, CurveSide::Convex, None).is_none());
    assert!(medial_angle(Level::T13, CurveSide::Convex).is_none());
    assert!(width_prior(Level::L1, None, CurveSide::Outside).is_some());
    assert!(chord(Level::L1, CurveSide::Outside, None).is_some());
    // The female 5-8 T10 width is printed-only because its spread is
    // malformed. Both sex columns are required, so the usable male
    // column cannot supply the whole answer.
    assert!(width_prior(Level::T10, Some(8.9), CurveSide::Convex).is_none());
    assert!(
        medial_angles_beside(Level::T4, Some(18.0))
            .iter()
            .all(|a| a.source != "zheng-2009")
    );
}

#[test]
fn only_whole_millimetre_cells_gain_a_centred_band_once_per_version() {
    // One uniform(-0.5,+0.5) per cell/version. A complete
    // 32-stratum uniform draw must have 16 residuals of each sign, span
    // above 30/32 mm, and mean magnitude below half a stratum (1/64 mm).
    // Zero normal draws make those uniform consequences independent of
    // the width normal; no expected random number is copied from code.
    let sampler = NamedSampler::new(7, 32).unwrap();
    for cell in tables()
        .cells(Table::PedicleWidths)
        .iter()
        .filter(|c| c.coordinate.group.as_deref() == Some("gao-controls"))
    {
        let level = cell.coordinate.level.unwrap();
        let mean = centre_spread(&cell.value).0;
        let mut residuals = Vec::new();
        for index in 0..32 {
            let version = Version {
                index,
                fit: fit(level),
                z_child: 0.0,
                pedicles: [BodySide::Left, BodySide::Right]
                    .into_iter()
                    .map(|side| PedicleDraw {
                        level,
                        side,
                        z_pedicle: 0.0,
                    })
                    .collect(),
            };
            let width = |side| {
                draw_width(
                    level,
                    side,
                    None,
                    CurvePosition::OUTSIDE,
                    &version,
                    &sampler,
                    None,
                )
                .unwrap()
                .unwrap()
                .outer_mm
            };
            let left = width(BodySide::Left);
            assert_eq!(left, width(BodySide::Right), "same published cell");
            residuals.push(left - mean);
        }
        if cell.whole_mm {
            assert_eq!(residuals.iter().filter(|&&r| r < 0.0).count(), 16);
            assert_eq!(residuals.iter().filter(|&&r| r > 0.0).count(), 16);
            assert!(residuals.iter().all(|r| r.abs() < 0.5));
            let low = residuals.iter().copied().fold(f64::INFINITY, f64::min);
            let high = residuals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            assert!(high - low > 30.0 / 32.0);
            assert!((residuals.iter().sum::<f64>() / 32.0).abs() < 1.0 / 64.0);
        } else {
            assert!(residuals.iter().all(|&r| r == 0.0));
        }
    }
}
