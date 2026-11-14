//! An assembled anchor is checked against a deliberately imposed population.
//! SYNTHESIS 2.10: five widths of 4.2 mm and five of 8.4 mm, k=1.25,
//! alpha=0.10 give 5 mm: 5.5 mm fails in half the versions. The ratios
//! are five copies each of 25/21 and 25/42, with median 25/28.

use implant_layout::curves;
use implant_layout::pedicles::width_prior;
use implant_layout::readings::{NamedSampler, PedicleDraw, Version};
use implant_layout::screws::{anatomy, anchor};
use shapes::implant_layout::{ChildFacts, CurveSide, LayoutInput, NamingDoubt};
use shapes::levels::Level;
use shapes::made_up::VertebraPose;
use shapes::spine_fit::{
    FitAngles, FitRmsMm, FitStandardDeviations, FitStopReason, FittedVertebra, InferredFit,
    SpineFit,
};
use shapes::spine_readings::BodySide;

fn input() -> LayoutInput {
    let vertebrae = [
        (Level::T1, -0.3, 0.0, 300.0),
        (Level::T7, 0.0, -20.0, 200.0),
        (Level::T12, 0.3, 0.0, 100.0),
    ]
    .into_iter()
    .map(|(level, tilt, y, z)| FittedVertebra {
        level,
        pose: VertebraPose {
            level,
            centre: [0.0, y, z],
            rotation: [tilt, 0.0, 0.0],
            height: 20.0,
            wedge_front: 0.0,
            wedge_side: 0.0,
            top_half_width: 15.0,
            top_half_depth: 12.0,
            bottom_half_width: 15.0,
            bottom_half_depth: 12.0,
        },
        angles: FitAngles {
            side_tilt_rad: tilt,
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
    })
    .collect();
    LayoutInput {
        fit: SpineFit {
            vertebrae,
            side_scale: 1.0,
            side_offset_mm: 0.0,
            origin_level: Level::T12,
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
        },
        front: None,
        bend_left: None,
        bend_right: None,
        child: ChildFacts {
            age_years: Some(14.0),
            weight_kg: None,
            activity: None,
        },
        naming: NamingDoubt::AsRead,
        versions: 10,
        seed: 59,
        pedicle_child_share: Some(0.5),
    }
}

#[test]
fn a_half_and_half_width_population_reports_the_inverse_width_band() {
    let input = input();
    let reading = curves::measure(&input, &input.fit).unwrap();
    assert_eq!(reading.curves.len(), 1);
    assert_eq!(reading.curves[0].convex, BodySide::Right);
    assert_eq!(reading.curves[0].apex_index, 1);
    let prior = width_prior(Level::T7, input.child.age_years, CurveSide::Convex).unwrap();
    let chord = anatomy::chord(Level::T7, CurveSide::Convex, input.child.age_years).unwrap();
    let sampler = NamedSampler::new(u64::from(input.seed), input.versions).unwrap();
    let root_half = libm::sqrt(0.5);
    let versions: Vec<_> = (0..input.versions)
        .map(|index| {
            // Solve the documented normal-combination formula backwards,
            // imposing 40 mm chords and the two stated width atoms. T7's
            // control cell is not whole-mm, so it has no rounding term.
            let chord_z = sampler
                .normal("versions-pedicles/1/Right/chord", index)
                .unwrap();
            let z_child = (40.0 - chord.mean) / (chord.sd.unwrap() * root_half) - chord_z;
            let width = if index < 5 { 4.2 } else { 8.4 };
            let z_pedicle = (width - prior.mean_mm) / (prior.sd_mm * root_half) - z_child;
            Version {
                index,
                fit: input.fit.clone(),
                z_child,
                pedicles: vec![PedicleDraw {
                    level: Level::T7,
                    side: BodySide::Right,
                    z_pedicle,
                }],
            }
        })
        .collect();
    let readings = vec![reading; input.versions as usize];
    let proposal = anchor(&input, &versions, &readings, Level::T7, BodySide::Right).unwrap();
    assert_eq!(proposal.diameter_mm, Some(5.0));
    // SYNTHESIS 2.11: min(0.8*40, 40-4)=32, rounded down to 30 mm.
    assert_eq!(proposal.length_mm, Some(30.0));
    let ratio = proposal.screw_to_pedicle.unwrap();
    for (actual, expected) in [
        (ratio.p10, 25.0 / 42.0),
        (ratio.p50, 25.0 / 28.0),
        (ratio.p90, 25.0 / 21.0),
    ] {
        assert!((actual - expected).abs() < 1e-12, "{actual} != {expected}");
    }
    let repeated = anchor(&input, &versions, &readings, Level::T7, BodySide::Right).unwrap();
    assert_eq!(
        serde_json::to_vec(&proposal).unwrap(),
        serde_json::to_vec(&repeated).unwrap(),
        "the same assembled input keeps identical serialised bits"
    );
    let reversed_versions: Vec<_> = versions.into_iter().rev().collect();
    let reversed_readings: Vec<_> = readings.into_iter().rev().collect();
    let reordered = anchor(
        &input,
        &reversed_versions,
        &reversed_readings,
        Level::T7,
        BodySide::Right,
    )
    .unwrap();
    assert_eq!(
        proposal, reordered,
        "global version identity survives reordering"
    );
}

#[test]
fn a_younger_child_uses_the_middle_share_and_keeps_the_shown_longer_choice() {
    let mut input = input();
    input.child.age_years = Some(10.0);
    let reading = curves::measure(&input, &input.fit).unwrap();
    let prior = width_prior(Level::T7, input.child.age_years, CurveSide::Convex).unwrap();
    let chord = anatomy::chord(Level::T7, CurveSide::Convex, input.child.age_years).unwrap();
    let sampler = NamedSampler::new(u64::from(input.seed), input.versions).unwrap();
    let root_half = libm::sqrt(0.5);
    let versions: Vec<_> = (0..input.versions)
        .map(|index| {
            let chord_z = sampler
                .normal("versions-pedicles/1/Right/chord", index)
                .unwrap();
            let z_child = (40.0 - chord.mean) / (chord.sd.unwrap() * root_half) - chord_z;
            let z_pedicle = (6.0 - prior.mean_mm) / (prior.sd_mm * root_half) - z_child;
            Version {
                index,
                fit: input.fit.clone(),
                z_child,
                pedicles: vec![PedicleDraw {
                    level: Level::T7,
                    side: BodySide::Right,
                    z_pedicle,
                }],
            }
        })
        .collect();
    let readings = vec![reading; input.versions as usize];
    let proposal = anchor(&input, &versions, &readings, Level::T7, BodySide::Right).unwrap();
    // min(.65*40,40-4)=26 ->25. Of the published .5--.8 range,
    // .5 gives20 (below the grid) and .8 gives32 ->30.
    assert_eq!(proposal.chord_share, Some(0.65));
    assert_eq!(proposal.length_mm, Some(25.0));
    assert_eq!(proposal.length_also_seen, vec![30.0]);
    assert_eq!(
        proposal.length_status,
        shapes::implant_layout::ValueStatus::FromTables
    );
}
