//! Distribution checks on made-up geometry. The bounds below are worked
//! from the normal distribution, independently of the width summariser.

use implant_layout::evidence::{PublishedValue, Table, Usage, tables};
use implant_layout::pedicles::{CurvePosition, MIN_OUTER_MM, drawn_outer_width};
use implant_layout::readings::{NamedSampler, PedicleDraw, Version};
use shapes::implant_layout::CurveSide;
use shapes::levels::Level;
use shapes::made_up::VertebraPose;
use shapes::spine_fit::{
    FitAngles, FitRmsMm, FitStandardDeviations, FitStopReason, FittedVertebra, InferredFit,
    SpineFit,
};
use shapes::spine_readings::BodySide;

fn synthetic_fit(level: Level) -> SpineFit {
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

fn coverage(seeds: u64, count: u32, child_share: f64, concave: bool) -> f64 {
    // Choose a nonrounded usable Gao control cell so the independent
    // answer is exactly normal, without a numerical convolution oracle.
    let cell = tables()
        .cells(Table::PedicleWidths)
        .iter()
        .find(|cell| {
            cell.coordinate.group.as_deref() == Some("gao-controls")
                && !cell.whole_mm
                && cell.usage == Usage::Usable
        })
        .expect("an unrounded control cell");
    let (mean, sd) = match cell.value {
        PublishedValue::PlusMinus { value, spread } => (value, spread),
        PublishedValue::MeanSd { mean, sd } => (mean, sd),
        _ => panic!("the selected cell has a published spread"),
    };
    let (mean, sd) = if concave {
        let ratio = tables()
            .cells(Table::ConcaveRatios)
            .iter()
            .find(|cell| {
                cell.coordinate.key == "concave-convex-ratio"
                    && cell.coordinate.group.as_deref() == Some("apex-one")
                    && cell.usage == Usage::Usable
            })
            .expect("the apical ratio");
        let (ratio_mean, ratio_sd) = match ratio.value {
            PublishedValue::MeanSd { mean, sd } => (mean, sd),
            PublishedValue::PlusMinus { value, spread } => (value, spread),
            _ => panic!("a ratio with a published spread"),
        };
        // The independent side-factor normal multiplies a constant base.
        // Its variance adds to the child/pedicle noise variance; the
        // shared split does not change that latter marginal variance.
        (
            mean * ratio_mean,
            libm::sqrt(mean * mean * ratio_sd * ratio_sd + sd * sd),
        )
    } else {
        (mean, sd)
    };
    // Standard-normal 90th percentile, a mathematical constant rather
    // than a paper value. Truncation carries a lower atom at MIN_OUTER_MM.
    let q90 = 1.281_551_565_544_600_4;
    let low = (mean - q90 * sd).max(MIN_OUTER_MM);
    let high = (mean + q90 * sd).max(MIN_OUTER_MM);
    let level = cell.coordinate.level.expect("a level");
    let fit = synthetic_fit(level);
    let mut inside = 0;
    for seed in 0..seeds {
        let sampler = NamedSampler::new(seed, count).expect("an ensemble");
        for index in 0..count {
            let version = Version {
                index,
                fit: fit.clone(),
                z_child: sampler
                    .normal("versions-pedicles/child", index)
                    .expect("a child draw"),
                // Supply both independent sampler slots. A concave
                // curve uses the canonical left slot even on the right
                // physical side; the distribution oracle stays unchanged.
                pedicles: [(BodySide::Left, "left"), (BodySide::Right, "right")]
                    .into_iter()
                    .map(|(side, name)| PedicleDraw {
                        level,
                        side,
                        z_pedicle: sampler
                            .normal(&format!("versions-pedicles/0/{name}"), index)
                            .expect("a pedicle draw"),
                    })
                    .collect(),
            };
            let width = drawn_outer_width(
                level,
                BodySide::Right,
                None,
                CurvePosition {
                    curve_side: if concave {
                        CurveSide::Concave
                    } else {
                        CurveSide::Convex
                    },
                    apex_distance: 0.0,
                    neutral_distance: None,
                },
                &version,
                &sampler,
                Some(child_share),
            )
            .expect("a width draw")
            .expect("a covered width");
            inside += u64::from((low..=high).contains(&width));
        }
    }
    inside as f64 / (seeds * u64::from(count)) as f64
}

#[test]
fn the_drawn_normal_widths_hold_eighty_percent_within_four_percentage_points() {
    // Twenty-four independent seeds, 32 versions each, for each variance
    // split. The tolerance is this test's Monte Carlo allowance, not a
    // statement about population-table accuracy.
    for share in [0.0, 0.5, 1.0] {
        let fraction = coverage(24, 32, share, false);
        assert!(
            (fraction - 0.8).abs() <= 0.04,
            "child share {share}: coverage {fraction}"
        );
    }
}

#[test]
#[ignore = "a larger deterministic distribution sweep"]
fn a_larger_seed_sweep_holds_eighty_percent_within_one_percentage_point() {
    for share in [0.0, 0.5, 1.0] {
        let fraction = coverage(256, 128, share, false);
        assert!(
            (fraction - 0.8).abs() <= 0.01,
            "child share {share}: coverage {fraction}"
        );
    }
}

#[test]
fn the_concave_apical_band_includes_the_independent_side_factor_spread() {
    let fraction = coverage(32, 32, 0.5, true);
    assert!(
        (fraction - 0.8).abs() <= 0.04,
        "concave apical coverage {fraction}"
    );
}
